// SPDX-License-Identifier: Apache-2.0
use std::ffi::CString;
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::{Json, Query, State};
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    CACHE_CONTROL, CONTENT_TYPE as AXUM_CONTENT_TYPE,
};
use axum::http::{HeaderValue, StatusCode as AxumStatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, options, post};
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use clap::{Parser, Subcommand};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use edgerun_hwvault_primitives::hardware::{
    load_or_create_device_signer, random_token_b64url, tpm_nv_available, tpm_nv_read_blob,
    tpm_nv_write_blob, DeviceSigner, HardwareBackend, HardwareSecurityMode,
};
use edgerun_runtime_proto::local::v1::{LocalEventEnvelopeV1, LocalNodeInfoResponseV1};
use edgerun_runtime_proto::tunnel::v1::{
    CreatePairingCodeRequestV1, CreatePairingCodeResponseV1, RegisterEndpointRequestV1,
    RegisterEndpointResponseV1, RegisterWithPairingCodeRequestV1,
    RegisterWithPairingCodeResponseV1, ReserveDomainRequestV1, ReserveDomainResponseV1,
    TunnelHeartbeatRequestV1, TunnelHeartbeatResponseV1,
};
use futures_util::StreamExt;
use prost::Message;
use reqwest::{header::CONTENT_TYPE, Client, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;
use tokio::process::Command as TokioCommand;
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::time::sleep;

const SECURITY_MODE: HardwareSecurityMode = HardwareSecurityMode::TpmRequired;
const CONFIG_TPM_NV_INDEX: u32 = 0x0150_0026;
const CONFIG_TPM_NV_SIZE: usize = 1024;
const DEFAULT_API_BASE: &str = "https://api.edgerun.tech";
const DEFAULT_RPC_URL: &str = "local://edgerun";
const DEFAULT_WORKER_BIN: &str = "/usr/bin/edgerun-worker";
const DEFAULT_CRUN_BIN: &str = "/usr/bin/crun";
const WORKER_PID_FILE: &str = "/run/edgerun/edgerun-worker.pid";
const WORKER_RUNTIME_MARKER_FILE: &str = "/run/edgerun/worker-runtime.ready";
const DEFAULT_WORKER_MAX_CONCURRENCY: u32 = 2;
const DEFAULT_WORKER_MEM_BYTES: u64 = 2_147_483_648;
const REQUIRED_CMDLINE_LOCK_TOKEN: &str = "edgerun.locked_cmdline=1";
const EDGE_SECUREBOOT_CERT_DER_PATH: &str =
    "/etc/edgerun/secureboot/edgerun-secureboot-db-cert.der";
const EDGE_SECUREBOOT_CERT_PEM_PATH: &str =
    "/etc/edgerun/secureboot/edgerun-secureboot-db-cert.pem";
const EFI_UPDATEVAR_BIN: &str = "/usr/bin/efi-updatevar";
const BOOT_POLICY_VERIFY_KEY_B64URL_ENV: &str = "EDGERUN_BOOT_POLICY_VERIFY_KEY_B64URL";
const RUNTIME_IMAGE_POLICY_SIGNING_CONTEXT: &str = "edgerun-runtime-image-policy-v1";
const RUNTIME_IMAGE_REQUEST_SIGNING_CONTEXT: &str = "edgerun-runtime-image-request-v1";
const DEFAULT_TUNNEL_CONTROL_BASE: &str = "https://relay.edgerun.tech";
const DEFAULT_LOCAL_BRIDGE_LISTEN: &str = "127.0.0.1:7777";
const LOCAL_BRIDGE_EVENTBUS_PATH: &str = "/v1/local/eventbus/ws";
const LOCAL_DOCKER_SUMMARY_PATH: &str = "/v1/local/docker/summary";
const LOCAL_FS_ROOT_ENV: &str = "EDGERUN_LOCAL_FS_ROOT";
const LOCAL_FS_META_PATH: &str = "/v1/local/fs/meta";
const LOCAL_FS_LIST_PATH: &str = "/v1/local/fs/list";
const LOCAL_FS_READ_PATH: &str = "/v1/local/fs/read";
const LOCAL_FS_WRITE_PATH: &str = "/v1/local/fs/write";
const LOCAL_FS_MKDIR_PATH: &str = "/v1/local/fs/mkdir";
const LOCAL_FS_DELETE_PATH: &str = "/v1/local/fs/delete";
const LOCAL_FS_MOVE_PATH: &str = "/v1/local/fs/move";
const LOCAL_FS_COPY_PATH: &str = "/v1/local/fs/copy";
const LOCAL_FS_ARCHIVE_PATH: &str = "/v1/local/fs/archive";
const LOCAL_FS_EXTRACT_PATH: &str = "/v1/local/fs/extract";
const LOCAL_MCP_START_PATH: &str = "/v1/local/mcp/integration/start";
const LOCAL_MCP_STOP_PATH: &str = "/v1/local/mcp/integration/stop";
const LOCAL_MCP_STATUS_PATH: &str = "/v1/local/mcp/integration/status";
const EVENTBUS_NATS_URL_ENV: &str = "EDGERUN_EVENTBUS_NATS_URL";
const EVENTBUS_NATS_URL_DEFAULT: &str = "nats://127.0.0.1:4222";
const INTEGRATION_TOPIC_ROOT_ENV: &str = "EDGERUN_INTEGRATION_TOPIC_ROOT";
const INTEGRATION_TOPIC_ROOT_DEFAULT: &str = "edgerun.integrations";
const LOCAL_ASSISTANT_PATH: &str = "/v1/local/assistant";
const LOCAL_DOCKER_EVENTS_TOPIC: &str = "local.docker.events";
const CODEX_CONFIG_PATH: &str = "/workspace/edgerun/.codex/config.toml";
const LOCAL_BRIDGE_VERSION: &str = "v1";

#[derive(Parser, Debug)]
#[command(name = "edgerun-node-manager", about = "EdgeRun TPM node manager")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Configure {
        #[arg(long, default_value = DEFAULT_RPC_URL)]
        rpc_url: String,
        #[arg(long, default_value_t = 15)]
        heartbeat_secs: u64,
    },
    Bond {
        #[arg(long)]
        owner_pubkey: String,
    },
    Identity,
    Register {
        #[arg(long)]
        owner_pubkey: String,
    },
    TunnelReserve {
        #[arg(long, default_value = DEFAULT_TUNNEL_CONTROL_BASE)]
        relay_control_base: String,
        #[arg(long, default_value = "")]
        requested_label: String,
    },
    TunnelRegister {
        #[arg(long, default_value = DEFAULT_TUNNEL_CONTROL_BASE)]
        relay_control_base: String,
        #[arg(long)]
        domain: String,
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        registration_token: String,
    },
    TunnelCreateCode {
        #[arg(long, default_value = DEFAULT_TUNNEL_CONTROL_BASE)]
        relay_control_base: String,
        #[arg(long)]
        domain: String,
        #[arg(long)]
        registration_token: String,
        #[arg(long, default_value_t = 300)]
        ttl_seconds: u64,
    },
    TunnelConnect {
        #[arg(long, default_value = DEFAULT_TUNNEL_CONTROL_BASE)]
        relay_control_base: String,
        #[arg(long)]
        pairing_code: String,
        #[arg(long, default_value = "")]
        node_id: String,
    },
    TunnelHeartbeat {
        #[arg(long, default_value = DEFAULT_TUNNEL_CONTROL_BASE)]
        relay_control_base: String,
        #[arg(long)]
        domain: String,
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        session_id: String,
    },
    Run {
        #[arg(long, default_value = DEFAULT_LOCAL_BRIDGE_LISTEN)]
        local_bridge_listen: String,
    },
}

#[derive(Debug, Deserialize)]
struct DeviceChallengeResponse {
    nonce_b64url: String,
}

#[derive(Debug, Serialize)]
struct DeviceHandshakeRequest<'a> {
    owner_pubkey: &'a str,
    nonce_b64url: &'a str,
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RuntimeImageResponse {
    ok: bool,
    #[serde(default)]
    image_tag: Option<String>,
    #[serde(default)]
    image_ref: Option<String>,
    #[serde(default)]
    request_nonce_b64url: Option<String>,
    #[serde(default)]
    issued_at_unix_s: Option<u64>,
    #[serde(default)]
    valid_until_unix_s: Option<u64>,
    #[serde(default)]
    rollback_index: Option<u64>,
    #[serde(default)]
    signature_b64url: Option<String>,
    #[serde(default)]
    signing_key_id: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct BondRequest<'a> {
    owner_pubkey: &'a str,
    device_pubkey_b64url: &'a str,
}

#[derive(Debug, Serialize)]
struct NodeInitRequest<'a> {
    owner_pubkey: &'a str,
    device_pubkey_b64url: &'a str,
    rpc_url: &'a str,
}

#[derive(Debug, Serialize)]
struct NodeHeartbeatRequest<'a> {
    owner_pubkey: &'a str,
    device_pubkey_b64url: &'a str,
    rpc_url: &'a str,
    version: &'a str,
    pid1: bool,
}

#[derive(Debug, Serialize)]
struct RuntimeImageRequest<'a> {
    owner_pubkey: &'a str,
    device_pubkey_b64url: &'a str,
    rpc_url: &'a str,
    request_nonce_b64url: &'a str,
    request_issued_at_unix_s: u64,
    request_signature_b64url: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManagerConfig {
    api_base: String,
    rpc_url: String,
    worker_max_concurrency: u32,
    worker_mem_bytes: u64,
    #[serde(default)]
    runtime_image_ref: Option<String>,
    #[serde(default)]
    runtime_image_pulled: bool,
    #[serde(default)]
    runtime_policy_rollback_index: u64,
    heartbeat_secs: u64,
    bonded: bool,
    node_initialized: bool,
    owner_pubkey: Option<String>,
}

#[derive(Debug)]
struct BootPolicy {
    owner_pubkey: Option<String>,
}

#[derive(Clone)]
struct LocalBridgeState {
    node_id: String,
    device_pubkey_b64url: String,
    local_fs_root: PathBuf,
    started_unix_ms: u64,
    tx: broadcast::Sender<LocalEventEnvelopeV1>,
}

#[derive(Debug, Serialize)]
struct LocalDockerSummaryResponse {
    ok: bool,
    error: String,
    swarm_active: bool,
    swarm_node_id: String,
    services: Vec<LocalDockerService>,
    containers: Vec<LocalDockerContainer>,
}

#[derive(Debug, Serialize)]
struct LocalDockerService {
    id: String,
    name: String,
    mode: String,
    replicas: String,
    image: String,
    ports: String,
}

#[derive(Debug, Serialize)]
struct LocalDockerContainer {
    id: String,
    name: String,
    image: String,
    status: String,
    state: String,
    ports: String,
}

#[derive(Debug, Serialize)]
struct LocalDockerEventPayload {
    event_type: String,
    action: String,
    container_id: String,
    container_name: String,
    message: String,
    ts_unix_ms: u64,
}

#[derive(Debug, Serialize)]
struct LocalBridgeEventPayloadEnvelope {
    payload: sonic_rs::Value,
    meta: sonic_rs::Value,
}

#[derive(Debug, Deserialize)]
struct LocalFsQuery {
    #[serde(default)]
    path: Option<String>,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalFsWriteRequest {
    path: String,
    content: String,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalFsPathRequest {
    path: String,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalFsMoveRequest {
    from: String,
    to: String,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalFsArchiveRequest {
    path: String,
    #[serde(default)]
    format: Option<String>,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct LocalFsResponse<T>
where
    T: Serialize,
{
    ok: bool,
    error: String,
    #[serde(flatten)]
    data: T,
}

#[derive(Debug, Serialize)]
struct LocalFsMetaData {
    #[serde(rename = "localFsRoot")]
    local_fs_root: String,
}

#[derive(Debug, Serialize)]
struct LocalFsListData {
    entries: Vec<LocalFsEntry>,
}

#[derive(Debug, Serialize)]
struct LocalFsReadData {
    content: String,
}

#[derive(Debug, Serialize)]
struct LocalFsArchiveData {
    archive_path: String,
}

#[derive(Debug, Serialize)]
struct LocalFsEmptyData {}

#[derive(Debug, Serialize)]
struct LocalFsEntry {
    id: String,
    provider: String,
    path: String,
    name: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct LocalMcpStartRequest {
    integration_id: String,
    token: String,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalMcpStopRequest {
    integration_id: String,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LocalMcpStatusQuery {
    integration_id: String,
    #[serde(default, alias = "nodeId")]
    node_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct LocalMcpStatusData {
    integration_id: String,
    container_name: String,
    running: bool,
    status: String,
}

#[derive(Debug, Deserialize)]
struct LocalAssistantRequest {
    message: String,
    #[serde(default)]
    provider: Option<String>,
    #[serde(default, alias = "sessionId")]
    session_id: Option<String>,
    #[serde(default, alias = "threadId")]
    thread_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct LocalAssistantResponse {
    ok: bool,
    error: String,
    message: String,
    #[serde(default)]
    actions: Vec<String>,
    #[serde(default, rename = "statusEvents")]
    status_events: Vec<LocalAssistantStatusEvent>,
    #[serde(default, rename = "sessionId")]
    session_id: String,
    #[serde(default, rename = "threadId")]
    thread_id: String,
}

#[derive(Debug, Serialize)]
struct LocalAssistantStatusEvent {
    #[serde(rename = "type")]
    event_type: String,
    label: String,
    detail: String,
}

#[tokio::main]
async fn main() {
    if let Err(err) = run_main().await {
        eprintln!("Error: {err:#}");
        if std::process::id() == 1 {
            eprintln!("pid1 entering hold loop after fatal startup error");
            loop {
                std::thread::sleep(Duration::from_secs(60));
            }
        }
        std::process::exit(1);
    }
}

async fn run_main() -> Result<()> {
    bootstrap_pid1_runtime();
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Run {
        local_bridge_listen: DEFAULT_LOCAL_BRIDGE_LISTEN.to_string(),
    }) {
        Commands::Configure {
            rpc_url,
            heartbeat_secs,
        } => cmd_configure(&rpc_url, heartbeat_secs),
        Commands::Bond { owner_pubkey } => cmd_bond(&owner_pubkey),
        Commands::Identity => cmd_identity(),
        Commands::Register { owner_pubkey } => cmd_register(&owner_pubkey).await,
        Commands::TunnelReserve {
            relay_control_base,
            requested_label,
        } => cmd_tunnel_reserve(&relay_control_base, &requested_label).await,
        Commands::TunnelRegister {
            relay_control_base,
            domain,
            node_id,
            registration_token,
        } => cmd_tunnel_register(&relay_control_base, &domain, &node_id, &registration_token).await,
        Commands::TunnelCreateCode {
            relay_control_base,
            domain,
            registration_token,
            ttl_seconds,
        } => {
            cmd_tunnel_create_code(
                &relay_control_base,
                &domain,
                &registration_token,
                ttl_seconds,
            )
            .await
        }
        Commands::TunnelConnect {
            relay_control_base,
            pairing_code,
            node_id,
        } => cmd_tunnel_connect(&relay_control_base, &pairing_code, &node_id).await,
        Commands::TunnelHeartbeat {
            relay_control_base,
            domain,
            node_id,
            session_id,
        } => cmd_tunnel_heartbeat(&relay_control_base, &domain, &node_id, &session_id).await,
        Commands::Run {
            local_bridge_listen,
        } => cmd_run(&local_bridge_listen).await,
    }
}

fn bootstrap_pid1_runtime() {
    if std::process::id() != 1 {
        return;
    }
    std::env::set_var("PATH", "/usr/bin:/usr/sbin:/bin:/sbin");
    for dir in ["/proc", "/sys", "/dev", "/run"] {
        let _ = fs::create_dir_all(dir);
    }
    let _ = mount_fs("proc", "/proc", "proc");
    let _ = mount_fs("sysfs", "/sys", "sysfs");
    let _ = mount_fs("devtmpfs", "/dev", "devtmpfs");
    let _ = fs::create_dir_all("/sys/firmware/efi/efivars");
    let _ = mount_fs("efivarfs", "/sys/firmware/efi/efivars", "efivarfs");
    let _ = mount_fs("tmpfs", "/run", "tmpfs");
}

fn mount_fs(source: &str, target: &str, fstype: &str) -> Result<()> {
    let target_display = target.to_string();
    let source = CString::new(source).context("invalid source cstring")?;
    let target = CString::new(target).context("invalid target cstring")?;
    let fstype = CString::new(fstype).context("invalid fstype cstring")?;
    // SAFETY: pointers are valid for the duration of the call and nul-terminated.
    let rc = unsafe {
        libc::mount(
            source.as_ptr(),
            target.as_ptr(),
            fstype.as_ptr(),
            0,
            std::ptr::null(),
        )
    };
    if rc == 0 {
        return Ok(());
    }
    let err = std::io::Error::last_os_error();
    if err.raw_os_error() == Some(libc::EBUSY) {
        return Ok(());
    }
    Err(anyhow!("mount failed for {target_display}: {err}"))
}

fn load_tpm_signer() -> Result<DeviceSigner> {
    wait_for_tpm_ready();
    let signer = load_or_create_device_signer(SECURITY_MODE)
        .context("failed to initialize TPM-backed device signer")?;
    if signer.backend != HardwareBackend::Tpm {
        return Err(anyhow!(
            "TPM is required; refusing non-TPM backend: {:?}",
            signer.backend
        ));
    }
    Ok(signer)
}

fn wait_for_tpm_ready() {
    if tpm_nv_available() {
        return;
    }
    // Early PID1 startup can race TPM char device creation in initramfs boot.
    for _ in 0..120 {
        std::thread::sleep(Duration::from_millis(250));
        if tpm_nv_available() {
            return;
        }
    }
}

fn cmd_identity() -> Result<()> {
    let signer = load_tpm_signer()?;
    println!("backend=tpm");
    println!("device_pubkey_b64url={}", signer.public_key_b64url);
    Ok(())
}

fn load_config() -> Result<ManagerConfig> {
    let blob = tpm_nv_read_blob(CONFIG_TPM_NV_INDEX, CONFIG_TPM_NV_SIZE)
        .context("failed to read manager config from TPM NV")?;
    if blob.len() < 4 {
        return Err(anyhow!("invalid TPM config blob"));
    }
    let len = u32::from_le_bytes([blob[0], blob[1], blob[2], blob[3]]) as usize;
    if len == 0 || len > (CONFIG_TPM_NV_SIZE - 4) {
        return Err(anyhow!("invalid TPM config length: {len}"));
    }
    let raw = &blob[4..4 + len];
    sonic_rs::from_slice(raw).context("failed to parse manager config json from TPM")
}

fn save_config(cfg: &ManagerConfig) -> Result<()> {
    let payload = sonic_rs::to_vec(cfg).context("failed to encode manager config")?;
    if payload.len() > (CONFIG_TPM_NV_SIZE - 4) {
        return Err(anyhow!(
            "config too large for TPM NV ({} > {})",
            payload.len(),
            CONFIG_TPM_NV_SIZE - 4
        ));
    }
    let mut blob = vec![0_u8; CONFIG_TPM_NV_SIZE];
    let len = payload.len() as u32;
    blob[0..4].copy_from_slice(&len.to_le_bytes());
    blob[4..4 + payload.len()].copy_from_slice(&payload);
    tpm_nv_write_blob(CONFIG_TPM_NV_INDEX, &blob, CONFIG_TPM_NV_SIZE)
        .context("failed to store manager config in TPM NV")?;
    Ok(())
}

fn cmd_configure(_rpc_url: &str, heartbeat_secs: u64) -> Result<()> {
    if !tpm_nv_available() {
        return Err(anyhow!("TPM NV storage unavailable"));
    }
    if heartbeat_secs == 0 {
        return Err(anyhow!("heartbeat_secs must be greater than zero"));
    }
    if let Ok(existing) = load_config() {
        if existing.bonded {
            return Err(anyhow!(
                "configuration is immutable after bonding (node already bonded)"
            ));
        }
    }
    let cfg = ManagerConfig {
        api_base: DEFAULT_API_BASE.to_string(),
        rpc_url: DEFAULT_RPC_URL.to_string(),
        worker_max_concurrency: DEFAULT_WORKER_MAX_CONCURRENCY,
        worker_mem_bytes: DEFAULT_WORKER_MEM_BYTES,
        runtime_image_ref: None,
        runtime_image_pulled: false,
        runtime_policy_rollback_index: 0,
        heartbeat_secs,
        bonded: false,
        node_initialized: false,
        owner_pubkey: None,
    };
    save_config(&cfg)?;
    println!("status=configured");
    println!("config_tpm_nv_index=0x{CONFIG_TPM_NV_INDEX:08x}");
    Ok(())
}

fn cmd_bond(owner_pubkey: &str) -> Result<()> {
    let mut cfg = load_config()?;
    if cfg.bonded {
        println!("status=already-bonded");
        println!("owner_pubkey={}", cfg.owner_pubkey.unwrap_or_default());
        return Ok(());
    }
    if owner_pubkey.trim().is_empty() {
        return Err(anyhow!("owner_pubkey is required"));
    }
    cfg.bonded = true;
    cfg.owner_pubkey = Some(owner_pubkey.trim().to_string());
    save_config(&cfg)?;
    println!("status=bonded");
    println!("owner_pubkey={owner_pubkey}");
    Ok(())
}

async fn cmd_register(owner_pubkey: &str) -> Result<()> {
    let signer = load_tpm_signer()?;
    let client = Client::new();
    register_device(
        &client,
        DEFAULT_API_BASE,
        &signer.public_key_b64url,
        owner_pubkey,
    )
    .await?;

    let registration_link = format!(
        "{}/register?device={}&owner={}",
        DEFAULT_API_BASE, signer.public_key_b64url, owner_pubkey
    );
    println!("status=registered");
    println!("device_pubkey_b64url={}", signer.public_key_b64url);
    println!("owner_pubkey={owner_pubkey}");
    println!("registration_url={registration_link}");
    Ok(())
}

async fn cmd_tunnel_reserve(relay_control_base: &str, requested_label: &str) -> Result<()> {
    let signer = load_tpm_signer()?;
    let client = Client::new();
    let url = format!(
        "{}/v1/tunnel/reserve-domain",
        relay_control_base.trim_end_matches('/')
    );
    let payload = ReserveDomainRequestV1 {
        profile_public_key_b64url: signer.public_key_b64url.clone(),
        requested_label: requested_label.trim().to_string(),
    };
    let response: ReserveDomainResponseV1 = post_protobuf(&client, &url, &payload).await?;
    if !response.ok {
        return Err(anyhow!("tunnel reserve failed: {}", response.error.trim()));
    }
    println!("status=tunnel-domain-reserved");
    println!("user_id={}", response.user_id);
    println!("domain={}", response.domain);
    println!("reservation_status={}", response.status);
    println!("registration_token={}", response.registration_token);
    Ok(())
}

fn tunnel_registration_message(domain: &str, node_id: &str, nonce_b64url: &str) -> String {
    format!("edgerun-tunnel-register-v1\ndomain={domain}\nnode_id={node_id}\nnonce={nonce_b64url}")
}

async fn cmd_tunnel_register(
    relay_control_base: &str,
    domain: &str,
    node_id: &str,
    registration_token: &str,
) -> Result<()> {
    let signer = load_tpm_signer()?;
    if domain.trim().is_empty() {
        return Err(anyhow!("domain is required"));
    }
    if node_id.trim().is_empty() {
        return Err(anyhow!("node_id is required"));
    }
    if registration_token.trim().is_empty() {
        return Err(anyhow!("registration_token is required"));
    }
    let nonce = random_token_b64url(24);
    let signature = signer
        .sign_b64url(tunnel_registration_message(domain.trim(), node_id.trim(), &nonce).as_bytes());
    let payload = RegisterEndpointRequestV1 {
        domain: domain.trim().to_string(),
        node_id: node_id.trim().to_string(),
        endpoint_public_key_b64url: signer.public_key_b64url.clone(),
        tunnel_nonce_b64url: nonce,
        registration_signature_b64url: signature,
        capability_scopes: vec![],
        registration_token: registration_token.trim().to_string(),
    };
    let client = Client::new();
    let url = format!(
        "{}/v1/tunnel/register-endpoint",
        relay_control_base.trim_end_matches('/')
    );
    let response: RegisterEndpointResponseV1 = post_protobuf(&client, &url, &payload).await?;
    if !response.ok {
        return Err(anyhow!("tunnel register failed: {}", response.error.trim()));
    }
    println!("status=tunnel-endpoint-registered");
    println!("domain={}", domain.trim());
    println!("node_id={}", node_id.trim());
    println!("session_id={}", response.session_id);
    println!("relay_url={}", response.relay_url);
    println!("expires_unix_ms={}", response.expires_unix_ms);
    Ok(())
}

async fn cmd_tunnel_heartbeat(
    relay_control_base: &str,
    domain: &str,
    node_id: &str,
    session_id: &str,
) -> Result<()> {
    if domain.trim().is_empty() || node_id.trim().is_empty() || session_id.trim().is_empty() {
        return Err(anyhow!("domain, node_id, and session_id are required"));
    }
    let client = Client::new();
    let url = format!(
        "{}/v1/tunnel/heartbeat",
        relay_control_base.trim_end_matches('/')
    );
    let payload = TunnelHeartbeatRequestV1 {
        domain: domain.trim().to_string(),
        node_id: node_id.trim().to_string(),
        session_id: session_id.trim().to_string(),
    };
    let response: TunnelHeartbeatResponseV1 = post_protobuf(&client, &url, &payload).await?;
    if !response.ok {
        return Err(anyhow!(
            "tunnel heartbeat failed: {}",
            response.error.trim()
        ));
    }
    println!("status=tunnel-heartbeat-ok");
    println!("expires_unix_ms={}", response.expires_unix_ms);
    Ok(())
}

async fn cmd_tunnel_create_code(
    relay_control_base: &str,
    domain: &str,
    registration_token: &str,
    ttl_seconds: u64,
) -> Result<()> {
    if domain.trim().is_empty() {
        return Err(anyhow!("domain is required"));
    }
    if registration_token.trim().is_empty() {
        return Err(anyhow!("registration_token is required"));
    }
    let client = Client::new();
    let url = format!(
        "{}/v1/tunnel/create-pairing-code",
        relay_control_base.trim_end_matches('/')
    );
    let payload = CreatePairingCodeRequestV1 {
        domain: domain.trim().to_string(),
        registration_token: registration_token.trim().to_string(),
        ttl_seconds,
    };
    let response: CreatePairingCodeResponseV1 = post_protobuf(&client, &url, &payload).await?;
    if !response.ok {
        return Err(anyhow!(
            "tunnel create pairing code failed: {}",
            response.error.trim()
        ));
    }
    println!("status=tunnel-pairing-code-issued");
    println!("pairing_code={}", response.pairing_code);
    println!("expires_unix_ms={}", response.expires_unix_ms);
    println!("device_command={}", response.device_command);
    Ok(())
}

fn derive_default_node_id(device_pubkey_b64url: &str) -> String {
    let short = device_pubkey_b64url
        .chars()
        .take(12)
        .collect::<String>()
        .to_ascii_lowercase();
    format!("node-{short}")
}

fn tunnel_pairing_registration_message(
    pairing_code: &str,
    node_id: &str,
    nonce_b64url: &str,
) -> String {
    format!(
        "edgerun-tunnel-connect-v1\npairing_code={pairing_code}\nnode_id={node_id}\nnonce={nonce_b64url}"
    )
}

async fn cmd_tunnel_connect(
    relay_control_base: &str,
    pairing_code: &str,
    node_id: &str,
) -> Result<()> {
    let signer = load_tpm_signer()?;
    if pairing_code.trim().is_empty() {
        return Err(anyhow!("pairing_code is required"));
    }
    let effective_node_id = if node_id.trim().is_empty() {
        derive_default_node_id(&signer.public_key_b64url)
    } else {
        node_id.trim().to_string()
    };
    let nonce = random_token_b64url(24);
    let signature = signer.sign_b64url(
        tunnel_pairing_registration_message(pairing_code.trim(), &effective_node_id, &nonce)
            .as_bytes(),
    );
    let payload = RegisterWithPairingCodeRequestV1 {
        pairing_code: pairing_code.trim().to_string(),
        node_id: effective_node_id.clone(),
        endpoint_public_key_b64url: signer.public_key_b64url.clone(),
        tunnel_nonce_b64url: nonce,
        registration_signature_b64url: signature,
        capability_scopes: vec![],
    };
    let client = Client::new();
    let url = format!(
        "{}/v1/tunnel/register-with-code",
        relay_control_base.trim_end_matches('/')
    );
    let response: RegisterWithPairingCodeResponseV1 =
        post_protobuf(&client, &url, &payload).await?;
    if !response.ok {
        return Err(anyhow!("tunnel connect failed: {}", response.error.trim()));
    }
    println!("status=tunnel-connected");
    println!("domain={}", response.domain);
    println!("node_id={effective_node_id}");
    println!("session_id={}", response.session_id);
    println!("relay_url={}", response.relay_url);
    println!("expires_unix_ms={}", response.expires_unix_ms);
    Ok(())
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn local_bridge_envelope(topic: &str, source: &str) -> LocalEventEnvelopeV1 {
    LocalEventEnvelopeV1 {
        event_id: format!("local-{}", now_unix_ms()),
        topic: topic.to_string(),
        payload: vec![],
        source: source.to_string(),
        ts_unix_ms: now_unix_ms(),
    }
}

fn with_local_cors_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    headers.insert(
        ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("content-type"),
    );
    headers.insert(
        ACCESS_CONTROL_ALLOW_METHODS,
        HeaderValue::from_static("GET, POST, OPTIONS"),
    );
    response
}

fn local_json_ok<T>(data: T) -> Response
where
    T: Serialize,
{
    let payload = sonic_rs::to_string(&LocalFsResponse {
        ok: true,
        error: String::new(),
        data,
    })
    .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode failed\"}".to_string());
    with_local_cors_headers(
        (
            AxumStatusCode::OK,
            [(
                AXUM_CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            )],
            payload,
        )
            .into_response(),
    )
}

fn local_json_error(status: AxumStatusCode, message: &str) -> Response {
    let payload = sonic_rs::to_string(&LocalFsResponse {
        ok: false,
        error: message.to_string(),
        data: LocalFsEmptyData {},
    })
    .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode failed\"}".to_string());
    with_local_cors_headers(
        (
            status,
            [(
                AXUM_CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            )],
            payload,
        )
            .into_response(),
    )
}

fn normalized_relative_path(raw: &str) -> Result<PathBuf> {
    let mut rel = PathBuf::new();
    for component in Path::new(raw).components() {
        match component {
            Component::Prefix(_) | Component::ParentDir => {
                return Err(anyhow!("path traversal is not allowed"));
            }
            Component::CurDir => {}
            Component::RootDir => {}
            Component::Normal(part) => rel.push(part),
        }
    }
    Ok(rel)
}

fn local_fs_display_path(raw: &str) -> Result<String> {
    let rel = normalized_relative_path(raw)?;
    if rel.as_os_str().is_empty() {
        return Ok("/".to_string());
    }
    Ok(format!("/{}", rel.to_string_lossy()))
}

fn local_fs_abs_path(root: &Path, raw: &str) -> Result<PathBuf> {
    let rel = normalized_relative_path(raw)?;
    let candidate = root.join(rel);
    Ok(candidate)
}

fn assert_path_under_root(root: &Path, candidate: &Path) -> Result<()> {
    if candidate.starts_with(root) {
        return Ok(());
    }
    Err(anyhow!("path escapes local filesystem root"))
}

fn enforce_local_node(state: &LocalBridgeState, node_id: Option<&str>) -> Result<()> {
    let requested = node_id.unwrap_or("").trim();
    if requested.is_empty() || requested == state.node_id {
        return Ok(());
    }
    Err(anyhow!(
        "selected node is not local; local bridge filesystem is only available for node {}",
        state.node_id
    ))
}

fn local_fs_entry(root: &Path, path: &Path) -> Result<LocalFsEntry> {
    let metadata =
        fs::metadata(path).with_context(|| format!("failed to stat path {}", path.display()))?;
    let rel = path
        .strip_prefix(root)
        .with_context(|| format!("path {} is outside root {}", path.display(), root.display()))?;
    let display = if rel.as_os_str().is_empty() {
        "/".to_string()
    } else {
        format!("/{}", rel.to_string_lossy())
    };
    let name = if rel.as_os_str().is_empty() {
        "/".to_string()
    } else {
        path.file_name()
            .map(|v| v.to_string_lossy().to_string())
            .unwrap_or_else(|| rel.to_string_lossy().to_string())
    };
    let kind = if metadata.is_dir() { "dir" } else { "file" }.to_string();
    Ok(LocalFsEntry {
        id: format!("local:{display}"),
        provider: "local".to_string(),
        path: display,
        name,
        kind,
        size: if metadata.is_file() {
            Some(metadata.len())
        } else {
            None
        },
    })
}

fn local_fs_copy_recursive(from: &Path, to: &Path) -> Result<()> {
    let metadata =
        fs::metadata(from).with_context(|| format!("failed to stat source {}", from.display()))?;
    if metadata.is_dir() {
        fs::create_dir_all(to)
            .with_context(|| format!("failed to create destination dir {}", to.display()))?;
        for entry in
            fs::read_dir(from).with_context(|| format!("failed to read dir {}", from.display()))?
        {
            let entry =
                entry.with_context(|| format!("failed to read dir entry {}", from.display()))?;
            let source_child = entry.path();
            let target_child = to.join(entry.file_name());
            local_fs_copy_recursive(&source_child, &target_child)?;
        }
    } else {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create destination parent {}", parent.display())
            })?;
        }
        fs::copy(from, to).with_context(|| {
            format!(
                "failed to copy source {} to destination {}",
                from.display(),
                to.display()
            )
        })?;
    }
    Ok(())
}

fn local_fs_archive_path(path: &Path, format: Option<&str>) -> PathBuf {
    let selected = format.unwrap_or("tar.gz").trim();
    let suffix = if selected.eq_ignore_ascii_case("tar.gz") || selected.eq_ignore_ascii_case("tgz")
    {
        "tar.gz"
    } else {
        "tar.gz"
    };
    let name = path
        .file_name()
        .map(|v| v.to_string_lossy().to_string())
        .unwrap_or_else(|| "archive".to_string());
    path.with_file_name(format!("{name}.{suffix}"))
}

fn mcp_container_name(integration_id: &str) -> String {
    let normalized = integration_id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    format!("edgerun-mcp-{}", normalized.trim_matches('-'))
}

fn mcp_image_for(integration_id: &str) -> Option<String> {
    let key = integration_id.trim().to_ascii_uppercase().replace('-', "_");
    let specific_env = format!("EDGERUN_MCP_{}_IMAGE", key);
    if let Ok(value) = std::env::var(&specific_env) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    match integration_id.trim() {
        "github" => Some("ghcr.io/modelcontextprotocol/server-github:latest".to_string()),
        _ => None,
    }
}

fn mcp_token_env_for(integration_id: &str) -> &'static str {
    match integration_id.trim() {
        "github" => "GITHUB_PERSONAL_ACCESS_TOKEN",
        _ => "MCP_API_TOKEN",
    }
}

fn integration_topic_root() -> String {
    std::env::var(INTEGRATION_TOPIC_ROOT_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| INTEGRATION_TOPIC_ROOT_DEFAULT.to_string())
}

fn integration_topic_for(integration_id: &str, lane: &str) -> String {
    let lane = lane
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("{}.{}.{}", integration_topic_root(), integration_id, lane)
}

fn is_mcp_container_running(integration_id: &str) -> bool {
    let name = mcp_container_name(integration_id);
    let output = Command::new("docker")
        .args(["inspect", "-f", "{{.State.Running}}", &name])
        .output();
    let Ok(output) = output else {
        return false;
    };
    if !output.status.success() {
        return false;
    }
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .eq_ignore_ascii_case("true")
}

fn sync_codex_mcp_config() -> Result<()> {
    let config_path = Path::new(CODEX_CONFIG_PATH);
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "failed to create codex config directory {}",
                parent.display()
            )
        })?;
    }
    let existing = fs::read_to_string(config_path).unwrap_or_default();
    let begin = "# BEGIN EDGERUN MCP";
    let end = "# END EDGERUN MCP";
    let mut preserved = existing.clone();
    if let Some(start) = existing.find(begin) {
        if let Some(rel_end) = existing[start..].find(end) {
            let end_idx = start + rel_end + end.len();
            let mut next = String::new();
            next.push_str(&existing[..start]);
            let tail = &existing[end_idx..];
            if !tail.starts_with('\n') {
                next.push('\n');
            }
            next.push_str(tail.trim_start_matches('\n'));
            preserved = next;
        }
    }
    let github_running = is_mcp_container_running("github");
    let mut managed_block = String::new();
    managed_block.push_str(begin);
    managed_block.push('\n');
    if github_running {
        managed_block.push_str("[mcp_servers.github]\n");
        managed_block.push_str("command = \"docker\"\n");
        managed_block.push_str(
            "args = [\"exec\", \"-i\", \"edgerun-mcp-github\", \"node\", \"/app/dist/index.js\"]\n",
        );
    }
    managed_block.push_str(end);
    managed_block.push('\n');

    let mut next = preserved.trim_end().to_string();
    if !next.is_empty() {
        next.push_str("\n\n");
    }
    next.push_str(&managed_block);
    fs::write(config_path, next)
        .with_context(|| format!("failed to write codex config {}", config_path.display()))?;
    Ok(())
}

fn start_local_bridge(local_bridge_listen: &str, device_pubkey_b64url: &str) -> Result<()> {
    let addr: SocketAddr = local_bridge_listen
        .parse()
        .with_context(|| format!("invalid local bridge listen addr: {local_bridge_listen}"))?;
    if !addr.ip().is_loopback() {
        return Err(anyhow!(
            "local bridge must bind to loopback only, got {local_bridge_listen}"
        ));
    }
    let local_fs_root = std::env::var(LOCAL_FS_ROOT_ENV).unwrap_or_else(|_| "/".to_string());
    let local_fs_root = fs::canonicalize(Path::new(local_fs_root.trim()))
        .with_context(|| format!("failed to resolve {}", LOCAL_FS_ROOT_ENV))?;
    if !local_fs_root.is_dir() {
        return Err(anyhow!(
            "{} must resolve to an existing directory",
            LOCAL_FS_ROOT_ENV
        ));
    }
    let state = LocalBridgeState {
        node_id: derive_default_node_id(device_pubkey_b64url),
        device_pubkey_b64url: device_pubkey_b64url.to_string(),
        local_fs_root,
        started_unix_ms: now_unix_ms(),
        tx: broadcast::channel(512).0,
    };
    let state = Arc::new(state);
    spawn_local_docker_events(Arc::clone(&state));
    let app = Router::new()
        .route("/v1/local/node/info.pb", get(handle_local_node_info))
        .route("/v1/local/node/info.pb", options(handle_local_options))
        .route(LOCAL_BRIDGE_EVENTBUS_PATH, get(handle_local_eventbus_ws))
        .route(LOCAL_DOCKER_SUMMARY_PATH, get(handle_local_docker_summary))
        .route(LOCAL_DOCKER_SUMMARY_PATH, options(handle_local_options))
        .route(LOCAL_FS_META_PATH, get(handle_local_fs_meta))
        .route(LOCAL_FS_META_PATH, options(handle_local_options))
        .route(LOCAL_FS_LIST_PATH, get(handle_local_fs_list))
        .route(LOCAL_FS_LIST_PATH, options(handle_local_options))
        .route(LOCAL_FS_READ_PATH, get(handle_local_fs_read))
        .route(LOCAL_FS_READ_PATH, options(handle_local_options))
        .route(LOCAL_FS_WRITE_PATH, post(handle_local_fs_write))
        .route(LOCAL_FS_WRITE_PATH, options(handle_local_options))
        .route(LOCAL_FS_MKDIR_PATH, post(handle_local_fs_mkdir))
        .route(LOCAL_FS_MKDIR_PATH, options(handle_local_options))
        .route(LOCAL_FS_DELETE_PATH, post(handle_local_fs_delete))
        .route(LOCAL_FS_DELETE_PATH, options(handle_local_options))
        .route(LOCAL_FS_MOVE_PATH, post(handle_local_fs_move))
        .route(LOCAL_FS_MOVE_PATH, options(handle_local_options))
        .route(LOCAL_FS_COPY_PATH, post(handle_local_fs_copy))
        .route(LOCAL_FS_COPY_PATH, options(handle_local_options))
        .route(LOCAL_FS_ARCHIVE_PATH, post(handle_local_fs_archive))
        .route(LOCAL_FS_ARCHIVE_PATH, options(handle_local_options))
        .route(LOCAL_FS_EXTRACT_PATH, post(handle_local_fs_extract))
        .route(LOCAL_FS_EXTRACT_PATH, options(handle_local_options))
        .route(LOCAL_MCP_START_PATH, post(handle_local_mcp_start))
        .route(LOCAL_MCP_START_PATH, options(handle_local_options))
        .route(LOCAL_MCP_STOP_PATH, post(handle_local_mcp_stop))
        .route(LOCAL_MCP_STOP_PATH, options(handle_local_options))
        .route(LOCAL_MCP_STATUS_PATH, get(handle_local_mcp_status))
        .route(LOCAL_MCP_STATUS_PATH, options(handle_local_options))
        .route(LOCAL_ASSISTANT_PATH, post(handle_local_assistant))
        .route(LOCAL_ASSISTANT_PATH, options(handle_local_options))
        .with_state(state);
    tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(addr).await {
            Ok(listener) => listener,
            Err(err) => {
                eprintln!("local_bridge_bind_error={err}");
                return;
            }
        };
        if let Err(err) = axum::serve(listener, app).await {
            eprintln!("local_bridge_serve_error={err}");
        }
    });
    Ok(())
}

async fn handle_local_node_info(State(state): State<Arc<LocalBridgeState>>) -> Response {
    let payload = LocalNodeInfoResponseV1 {
        ok: true,
        error: String::new(),
        node_id: state.node_id.clone(),
        device_pubkey_b64url: state.device_pubkey_b64url.clone(),
        bridge_version: LOCAL_BRIDGE_VERSION.to_string(),
        started_unix_ms: state.started_unix_ms,
        eventbus_ws_path: LOCAL_BRIDGE_EVENTBUS_PATH.to_string(),
    }
    .encode_to_vec();
    (
        AxumStatusCode::OK,
        [
            (
                AXUM_CONTENT_TYPE,
                HeaderValue::from_static("application/x-protobuf"),
            ),
            (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*")),
            (
                ACCESS_CONTROL_ALLOW_HEADERS,
                HeaderValue::from_static("content-type"),
            ),
            (
                ACCESS_CONTROL_ALLOW_METHODS,
                HeaderValue::from_static("GET, OPTIONS"),
            ),
        ],
        payload,
    )
        .into_response()
}

async fn handle_local_options() -> Response {
    (
        AxumStatusCode::NO_CONTENT,
        [
            (ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*")),
            (
                ACCESS_CONTROL_ALLOW_HEADERS,
                HeaderValue::from_static("content-type"),
            ),
            (
                ACCESS_CONTROL_ALLOW_METHODS,
                HeaderValue::from_static("GET, POST, OPTIONS"),
            ),
        ],
    )
        .into_response()
}

async fn handle_local_eventbus_ws(
    ws: WebSocketUpgrade,
    State(state): State<Arc<LocalBridgeState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| local_eventbus_session(socket, state))
}

async fn handle_local_docker_summary() -> Response {
    let summary = match collect_local_docker_summary() {
        Ok(summary) => summary,
        Err(err) => LocalDockerSummaryResponse {
            ok: false,
            error: err.to_string(),
            swarm_active: false,
            swarm_node_id: String::new(),
            services: Vec::new(),
            containers: Vec::new(),
        },
    };
    let payload = sonic_rs::to_string(&summary)
        .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode failed\"}".to_string());
    (
        AxumStatusCode::OK,
        [
            (
                AXUM_CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            ),
            (CACHE_CONTROL, HeaderValue::from_static("no-store")),
            (ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*")),
            (
                ACCESS_CONTROL_ALLOW_HEADERS,
                HeaderValue::from_static("content-type"),
            ),
            (
                ACCESS_CONTROL_ALLOW_METHODS,
                HeaderValue::from_static("GET, OPTIONS"),
            ),
        ],
        payload,
    )
        .into_response()
}

async fn handle_local_fs_meta(
    State(state): State<Arc<LocalBridgeState>>,
    Query(query): Query<LocalFsQuery>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, query.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

async fn handle_local_fs_list(
    State(state): State<Arc<LocalBridgeState>>,
    Query(query): Query<LocalFsQuery>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, query.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let requested = query.path.unwrap_or_else(|| "/".to_string());
    let display = match local_fs_display_path(&requested) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let absolute = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    let metadata = match fs::metadata(&absolute) {
        Ok(meta) => meta,
        Err(err) => return local_json_error(AxumStatusCode::NOT_FOUND, &err.to_string()),
    };
    if !metadata.is_dir() {
        return local_json_error(
            AxumStatusCode::BAD_REQUEST,
            "list path must resolve to a directory",
        );
    }
    let mut entries = Vec::new();
    let iterator = match fs::read_dir(&absolute) {
        Ok(value) => value,
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    };
    for item in iterator {
        let item = match item {
            Ok(value) => value,
            Err(err) => {
                return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
            }
        };
        let path = item.path();
        match local_fs_entry(&state.local_fs_root, &path) {
            Ok(entry) => entries.push(entry),
            Err(err) => {
                return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
            }
        }
    }
    entries.sort_by(|a, b| {
        let a_kind = a.kind.as_str();
        let b_kind = b.kind.as_str();
        if a_kind != b_kind {
            return if a_kind == "dir" {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        a.name.to_lowercase().cmp(&b.name.to_lowercase())
    });
    local_json_ok(LocalFsListData { entries })
}

async fn handle_local_fs_read(
    State(state): State<Arc<LocalBridgeState>>,
    Query(query): Query<LocalFsQuery>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, query.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let requested = query.path.unwrap_or_else(|| "/".to_string());
    let display = match local_fs_display_path(&requested) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let absolute = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    let bytes = match fs::read(&absolute) {
        Ok(value) => value,
        Err(err) => return local_json_error(AxumStatusCode::NOT_FOUND, &err.to_string()),
    };
    let content = match String::from_utf8(bytes) {
        Ok(value) => value,
        Err(_) => {
            return local_json_error(AxumStatusCode::BAD_REQUEST, "file content is not utf-8")
        }
    };
    local_json_ok(LocalFsReadData { content })
}

async fn handle_local_fs_write(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsWriteRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let display = match local_fs_display_path(&body.path) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let absolute = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    if let Some(parent) = absolute.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
        }
    }
    if let Err(err) = fs::write(&absolute, body.content.as_bytes()) {
        return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

async fn handle_local_fs_mkdir(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsPathRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let display = match local_fs_display_path(&body.path) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let absolute = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    if let Err(err) = fs::create_dir_all(&absolute) {
        return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

async fn handle_local_fs_delete(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsPathRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let display = match local_fs_display_path(&body.path) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let absolute = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    let metadata = match fs::metadata(&absolute) {
        Ok(meta) => meta,
        Err(err) => return local_json_error(AxumStatusCode::NOT_FOUND, &err.to_string()),
    };
    let result = if metadata.is_dir() {
        fs::remove_dir_all(&absolute)
    } else {
        fs::remove_file(&absolute)
    };
    if let Err(err) = result {
        return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

async fn handle_local_fs_move(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsMoveRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let from_display = match local_fs_display_path(&body.from) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let to_display = match local_fs_display_path(&body.to) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let from_absolute = match local_fs_abs_path(&state.local_fs_root, &from_display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let to_absolute = match local_fs_abs_path(&state.local_fs_root, &to_display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &from_absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &to_absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    if let Some(parent) = to_absolute.parent() {
        if let Err(err) = fs::create_dir_all(parent) {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
        }
    }
    if let Err(err) = fs::rename(&from_absolute, &to_absolute) {
        return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

async fn handle_local_fs_copy(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsMoveRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let from_display = match local_fs_display_path(&body.from) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let to_display = match local_fs_display_path(&body.to) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let from_absolute = match local_fs_abs_path(&state.local_fs_root, &from_display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let to_absolute = match local_fs_abs_path(&state.local_fs_root, &to_display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &from_absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &to_absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    if let Err(err) = local_fs_copy_recursive(&from_absolute, &to_absolute) {
        return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string());
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

async fn handle_local_fs_archive(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsArchiveRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let display = match local_fs_display_path(&body.path) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let absolute = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &absolute) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    let archive = local_fs_archive_path(&absolute, body.format.as_deref());
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &archive) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    let parent = match absolute.parent() {
        Some(path) => path,
        None => {
            return local_json_error(AxumStatusCode::BAD_REQUEST, "invalid archive source path")
        }
    };
    let source_name = match absolute.file_name() {
        Some(name) => name.to_string_lossy().to_string(),
        None => {
            return local_json_error(AxumStatusCode::BAD_REQUEST, "invalid archive source path")
        }
    };
    let status = Command::new("tar")
        .arg("-czf")
        .arg(&archive)
        .arg("-C")
        .arg(parent)
        .arg(source_name)
        .status();
    match status {
        Ok(result) if result.success() => {}
        Ok(result) => {
            return local_json_error(
                AxumStatusCode::INTERNAL_SERVER_ERROR,
                &format!("tar failed with status {result}"),
            )
        }
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    }
    let archive_display = match archive.strip_prefix(&state.local_fs_root) {
        Ok(rel) if rel.as_os_str().is_empty() => "/".to_string(),
        Ok(rel) => format!("/{}", rel.to_string_lossy()),
        Err(_) => archive.to_string_lossy().to_string(),
    };
    local_json_ok(LocalFsArchiveData {
        archive_path: archive_display,
    })
}

async fn handle_local_fs_extract(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalFsPathRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let display = match local_fs_display_path(&body.path) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    let archive = match local_fs_abs_path(&state.local_fs_root, &display) {
        Ok(path) => path,
        Err(err) => return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string()),
    };
    if let Err(err) = assert_path_under_root(&state.local_fs_root, &archive) {
        return local_json_error(AxumStatusCode::BAD_REQUEST, &err.to_string());
    }
    let parent = match archive.parent() {
        Some(path) => path,
        None => return local_json_error(AxumStatusCode::BAD_REQUEST, "invalid archive path"),
    };
    let status = Command::new("tar")
        .arg("-xf")
        .arg(&archive)
        .arg("-C")
        .arg(parent)
        .status();
    match status {
        Ok(result) if result.success() => {}
        Ok(result) => {
            return local_json_error(
                AxumStatusCode::INTERNAL_SERVER_ERROR,
                &format!("extract failed with status {result}"),
            )
        }
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    }
    local_json_ok(LocalFsMetaData {
        local_fs_root: state.local_fs_root.to_string_lossy().to_string(),
    })
}

async fn handle_local_mcp_start(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalMcpStartRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let integration_id = body.integration_id.trim().to_ascii_lowercase();
    if integration_id.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "integration_id is required");
    }
    let token = body.token.trim().to_string();
    if token.len() < 8 {
        return local_json_error(
            AxumStatusCode::BAD_REQUEST,
            "integration token is missing or invalid",
        );
    }
    let image = match mcp_image_for(&integration_id) {
        Some(value) => value,
        None => {
            return local_json_error(
                AxumStatusCode::BAD_REQUEST,
                &format!("no MCP image mapping for integration {}", integration_id),
            )
        }
    };
    let container_name = mcp_container_name(&integration_id);
    let _ = Command::new("docker")
        .args(["rm", "-f", &container_name])
        .output();
    let token_env = mcp_token_env_for(&integration_id);
    let nats_url = std::env::var(EVENTBUS_NATS_URL_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| EVENTBUS_NATS_URL_DEFAULT.to_string());
    let mut args = vec![
        "run".to_string(),
        "-d".to_string(),
        "--name".to_string(),
        container_name.clone(),
        "--restart".to_string(),
        "unless-stopped".to_string(),
        "-e".to_string(),
        format!("{}={}", token_env, token),
        "-e".to_string(),
        format!("EDGERUN_INTEGRATION_ID={}", integration_id),
        "-e".to_string(),
        format!("EDGERUN_EVENTBUS_NATS_URL={}", nats_url),
        "-e".to_string(),
        format!(
            "EDGERUN_EVENTBUS_TOPIC_INBOUND={}",
            integration_topic_for(&integration_id, "inbound")
        ),
        "-e".to_string(),
        format!(
            "EDGERUN_EVENTBUS_TOPIC_OUTBOUND={}",
            integration_topic_for(&integration_id, "outbound")
        ),
        "-e".to_string(),
        format!(
            "EDGERUN_EVENTBUS_TOPIC_ERRORS={}",
            integration_topic_for(&integration_id, "errors")
        ),
        image.clone(),
    ];
    let output = match Command::new("docker").args(args.drain(..)).output() {
        Ok(value) => value,
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    };
    if !output.status.success() {
        return local_json_error(
            AxumStatusCode::INTERNAL_SERVER_ERROR,
            &format!(
                "failed to start MCP container {}: {}",
                container_name,
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        );
    }
    if let Err(err) = sync_codex_mcp_config() {
        return local_json_error(
            AxumStatusCode::INTERNAL_SERVER_ERROR,
            &format!("MCP started but codex config sync failed: {err}"),
        );
    }
    local_json_ok(LocalMcpStatusData {
        integration_id,
        container_name,
        running: true,
        status: "running".to_string(),
    })
}

async fn handle_local_mcp_stop(
    State(state): State<Arc<LocalBridgeState>>,
    Json(body): Json<LocalMcpStopRequest>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, body.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let integration_id = body.integration_id.trim().to_ascii_lowercase();
    if integration_id.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "integration_id is required");
    }
    let container_name = mcp_container_name(&integration_id);
    let output = match Command::new("docker")
        .args(["rm", "-f", &container_name])
        .output()
    {
        Ok(value) => value,
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    };
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !stderr.to_ascii_lowercase().contains("no such container") {
            return local_json_error(
                AxumStatusCode::INTERNAL_SERVER_ERROR,
                &format!(
                    "failed to stop MCP container {}: {}",
                    container_name,
                    stderr.trim()
                ),
            );
        }
    }
    if let Err(err) = sync_codex_mcp_config() {
        return local_json_error(
            AxumStatusCode::INTERNAL_SERVER_ERROR,
            &format!("MCP stopped but codex config sync failed: {err}"),
        );
    }
    local_json_ok(LocalMcpStatusData {
        integration_id,
        container_name,
        running: false,
        status: "stopped".to_string(),
    })
}

async fn handle_local_mcp_status(
    State(state): State<Arc<LocalBridgeState>>,
    Query(query): Query<LocalMcpStatusQuery>,
) -> Response {
    if let Err(err) = enforce_local_node(&state, query.node_id.as_deref()) {
        return local_json_error(AxumStatusCode::FORBIDDEN, &err.to_string());
    }
    let integration_id = query.integration_id.trim().to_ascii_lowercase();
    if integration_id.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "integration_id is required");
    }
    let container_name = mcp_container_name(&integration_id);
    let output = match Command::new("docker")
        .args([
            "inspect",
            "-f",
            "{{.State.Running}}\t{{.State.Status}}",
            &container_name,
        ])
        .output()
    {
        Ok(value) => value,
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    };
    if !output.status.success() {
        return local_json_ok(LocalMcpStatusData {
            integration_id,
            container_name,
            running: false,
            status: "not_found".to_string(),
        });
    }
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let cols: Vec<&str> = stdout.split('\t').collect();
    let running = cols
        .first()
        .map(|value| value.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let status = cols
        .get(1)
        .map(|value| value.trim().to_string())
        .unwrap_or_else(|| {
            if running {
                "running".to_string()
            } else {
                "stopped".to_string()
            }
        });
    local_json_ok(LocalMcpStatusData {
        integration_id,
        container_name,
        running,
        status,
    })
}

async fn handle_local_assistant(Json(body): Json<LocalAssistantRequest>) -> Response {
    let prompt = body.message.trim().to_string();
    if prompt.is_empty() {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "assistant message is required");
    }
    let provider = body
        .provider
        .as_deref()
        .unwrap_or("codex")
        .trim()
        .to_ascii_lowercase();
    if provider != "codex" && provider != "qwen" {
        return local_json_error(AxumStatusCode::BAD_REQUEST, "unsupported provider");
    }

    let output_file = format!("/tmp/edgerun-codex-last-{}.txt", now_unix_ms());
    let exec_output = Command::new("timeout")
        .args([
            "30s",
            "docker",
            "exec",
            "edgerun-codex-cli",
            "codex",
            "exec",
            "--skip-git-repo-check",
            "-C",
            "/workspace/edgerun",
            "--output-last-message",
            &output_file,
            &prompt,
        ])
        .output();

    let exec_output = match exec_output {
        Ok(value) => value,
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    };
    if !exec_output.status.success() {
        return local_json_error(
            AxumStatusCode::INTERNAL_SERVER_ERROR,
            &format!(
                "codex exec failed: {}",
                String::from_utf8_lossy(&exec_output.stderr).trim()
            ),
        );
    }

    let read_output = Command::new("docker")
        .args(["exec", "edgerun-codex-cli", "cat", &output_file])
        .output();
    let read_output = match read_output {
        Ok(value) => value,
        Err(err) => {
            return local_json_error(AxumStatusCode::INTERNAL_SERVER_ERROR, &err.to_string())
        }
    };
    if !read_output.status.success() {
        return local_json_error(
            AxumStatusCode::INTERNAL_SERVER_ERROR,
            &format!(
                "failed to read codex output: {}",
                String::from_utf8_lossy(&read_output.stderr).trim()
            ),
        );
    }

    let message = String::from_utf8_lossy(&read_output.stdout)
        .trim()
        .to_string();
    let response = LocalAssistantResponse {
        ok: true,
        error: String::new(),
        message: if message.is_empty() {
            "Codex returned no output.".to_string()
        } else {
            message
        },
        actions: Vec::new(),
        status_events: vec![LocalAssistantStatusEvent {
            event_type: "phase".to_string(),
            label: "done".to_string(),
            detail: "Response ready.".to_string(),
        }],
        session_id: body.session_id.unwrap_or_default(),
        thread_id: body.thread_id.unwrap_or_default(),
    };
    with_local_cors_headers(
        (
            AxumStatusCode::OK,
            [(
                AXUM_CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            )],
            sonic_rs::to_string(&response)
                .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode failed\"}".to_string()),
        )
            .into_response(),
    )
}

async fn local_eventbus_session(socket: WebSocket, state: Arc<LocalBridgeState>) {
    let mut rx = state.tx.subscribe();
    let socket = Arc::new(Mutex::new(socket));
    let sender_socket = Arc::clone(&socket);
    let outbound = tokio::spawn(async move {
        loop {
            let event = match rx.recv().await {
                Ok(event) => event,
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            };
            let bytes = event.encode_to_vec();
            let mut ws = sender_socket.lock().await;
            if ws.send(WsMessage::Binary(bytes.into())).await.is_err() {
                break;
            }
        }
    });

    let connected = local_bridge_envelope("local.bridge.connected", "node-manager");
    {
        let mut ws = socket.lock().await;
        let _ = ws
            .send(WsMessage::Binary(connected.encode_to_vec().into()))
            .await;
    }

    let mut inbound_open = true;
    while inbound_open {
        let next = {
            let mut ws = socket.lock().await;
            ws.next().await
        };
        let Some(Ok(message)) = next else {
            break;
        };
        match message {
            WsMessage::Binary(bytes) => {
                if let Ok(envelope) = LocalEventEnvelopeV1::decode(bytes.as_ref()) {
                    let _ = state.tx.send(envelope);
                }
            }
            WsMessage::Text(text) => {
                if text.trim() == "ping" {
                    let pong = local_bridge_envelope("local.bridge.pong", "node-manager");
                    let mut ws = socket.lock().await;
                    let _ = ws
                        .send(WsMessage::Binary(pong.encode_to_vec().into()))
                        .await;
                }
            }
            WsMessage::Close(_) => inbound_open = false,
            WsMessage::Ping(payload) => {
                let mut ws = socket.lock().await;
                if ws.send(WsMessage::Pong(payload)).await.is_err() {
                    break;
                }
            }
            WsMessage::Pong(_) => {}
        }
    }
    outbound.abort();
}

fn collect_local_docker_summary() -> Result<LocalDockerSummaryResponse> {
    let swarm_info = run_command_capture(
        "docker",
        &[
            "info",
            "--format",
            "{{.Swarm.LocalNodeState}}\t{{.Swarm.NodeID}}",
        ],
    )?;
    let mut swarm_state = String::new();
    let mut swarm_node_id = String::new();
    if let Some(line) = swarm_info.lines().next() {
        let cols: Vec<&str> = line.split('\t').collect();
        swarm_state = cols.first().copied().unwrap_or_default().trim().to_string();
        swarm_node_id = cols.get(1).copied().unwrap_or_default().trim().to_string();
    }
    let swarm_active = swarm_state == "active";

    let mut services = Vec::new();
    if swarm_active {
        let rows = run_command_capture(
            "docker",
            &[
                "service",
                "ls",
                "--format",
                "{{.ID}}\t{{.Name}}\t{{.Mode}}\t{{.Replicas}}\t{{.Image}}\t{{.Ports}}",
            ],
        )?;
        for line in rows.lines() {
            let cols: Vec<&str> = line.split('\t').collect();
            if cols.len() < 5 {
                continue;
            }
            services.push(LocalDockerService {
                id: cols.first().copied().unwrap_or_default().trim().to_string(),
                name: cols.get(1).copied().unwrap_or_default().trim().to_string(),
                mode: cols.get(2).copied().unwrap_or_default().trim().to_string(),
                replicas: cols.get(3).copied().unwrap_or_default().trim().to_string(),
                image: cols.get(4).copied().unwrap_or_default().trim().to_string(),
                ports: cols.get(5).copied().unwrap_or_default().trim().to_string(),
            });
        }
    }

    let rows = run_command_capture(
        "docker",
        &[
            "ps",
            "--format",
            "{{.ID}}\t{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.State}}\t{{.Ports}}",
        ],
    )?;
    let mut containers = Vec::new();
    for line in rows.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 5 {
            continue;
        }
        containers.push(LocalDockerContainer {
            id: cols.first().copied().unwrap_or_default().trim().to_string(),
            name: cols.get(1).copied().unwrap_or_default().trim().to_string(),
            image: cols.get(2).copied().unwrap_or_default().trim().to_string(),
            status: cols.get(3).copied().unwrap_or_default().trim().to_string(),
            state: cols.get(4).copied().unwrap_or_default().trim().to_string(),
            ports: cols.get(5).copied().unwrap_or_default().trim().to_string(),
        });
    }

    Ok(LocalDockerSummaryResponse {
        ok: true,
        error: String::new(),
        swarm_active,
        swarm_node_id,
        services,
        containers,
    })
}

fn spawn_local_docker_events(state: Arc<LocalBridgeState>) {
    tokio::spawn(async move {
        loop {
            if let Err(err) = run_local_docker_event_stream(Arc::clone(&state)).await {
                eprintln!("local_docker_event_stream_error={err}");
            }
            sleep(Duration::from_secs(2)).await;
        }
    });
}

async fn run_local_docker_event_stream(state: Arc<LocalBridgeState>) -> Result<()> {
    let mut child = TokioCommand::new("docker")
        .args([
            "events",
            "--format",
            "{{.TimeNano}}\t{{.Type}}\t{{.Action}}\t{{.Actor.ID}}\t{{.Actor.Attributes.name}}",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| "failed to start docker events stream".to_string())?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("docker events stream missing stdout"))?;
    let mut lines = BufReader::new(stdout).lines();
    while let Some(line) = lines
        .next_line()
        .await
        .with_context(|| "failed to read docker event line".to_string())?
    {
        let cols: Vec<&str> = line.split('\t').collect();
        let time_nano = cols.first().copied().unwrap_or_default().trim();
        let event_type = cols.get(1).copied().unwrap_or_default().trim().to_string();
        let action = cols.get(2).copied().unwrap_or_default().trim().to_string();
        let container_id = cols.get(3).copied().unwrap_or_default().trim().to_string();
        let container_name = cols.get(4).copied().unwrap_or_default().trim().to_string();
        let ts_unix_ms = time_nano
            .parse::<u64>()
            .map(|value| value / 1_000_000)
            .unwrap_or_else(|_| now_unix_ms());
        let message = format!(
            "{} {} {}",
            if event_type.is_empty() {
                "docker"
            } else {
                &event_type
            },
            if action.is_empty() { "event" } else { &action },
            if container_name.is_empty() {
                if container_id.is_empty() {
                    "unknown".to_string()
                } else {
                    container_id.clone()
                }
            } else {
                container_name.clone()
            }
        );
        let payload = LocalDockerEventPayload {
            event_type,
            action,
            container_id,
            container_name,
            message,
            ts_unix_ms,
        };
        let envelope_payload = LocalBridgeEventPayloadEnvelope {
            payload: sonic_rs::to_value(&payload)
                .with_context(|| "failed to convert docker event payload value".to_string())?,
            meta: sonic_rs::json!({ "source": "node-manager", "kind": "docker.event" }),
        };
        let mut envelope = local_bridge_envelope(LOCAL_DOCKER_EVENTS_TOPIC, "node-manager");
        envelope.payload = sonic_rs::to_vec(&envelope_payload)
            .with_context(|| "failed to encode local docker event payload".to_string())?;
        let _ = state.tx.send(envelope);
    }
    let status = child
        .wait()
        .await
        .with_context(|| "failed to wait docker event stream process".to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(anyhow!("docker events stream exited with status {status}"))
    }
}

fn run_command_capture(bin: &str, args: &[&str]) -> Result<String> {
    let output = Command::new(bin)
        .args(args)
        .output()
        .with_context(|| format!("failed to run command: {} {}", bin, args.join(" ")))?;
    if !output.status.success() {
        return Err(anyhow!(
            "command failed: {} {} ({})",
            bin,
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn cmd_run(local_bridge_listen: &str) -> Result<()> {
    let signer = load_tpm_signer()?;
    let pid1 = std::process::id() == 1;
    let boot_policy = if pid1 {
        Some(enforce_boot_policy()?)
    } else {
        None
    };

    let mut cfg = load_config().unwrap_or_else(|_| ManagerConfig {
        api_base: DEFAULT_API_BASE.to_string(),
        rpc_url: DEFAULT_RPC_URL.to_string(),
        worker_max_concurrency: DEFAULT_WORKER_MAX_CONCURRENCY,
        worker_mem_bytes: DEFAULT_WORKER_MEM_BYTES,
        runtime_image_ref: None,
        runtime_image_pulled: false,
        runtime_policy_rollback_index: 0,
        heartbeat_secs: 15,
        bonded: false,
        node_initialized: false,
        owner_pubkey: None,
    });

    if cfg.heartbeat_secs == 0 {
        cfg.heartbeat_secs = 15;
    }
    cfg.rpc_url = DEFAULT_RPC_URL.to_string();
    if cfg.worker_max_concurrency == 0 {
        cfg.worker_max_concurrency = DEFAULT_WORKER_MAX_CONCURRENCY;
    }
    if cfg.worker_mem_bytes == 0 {
        cfg.worker_mem_bytes = DEFAULT_WORKER_MEM_BYTES;
    }

    let client = Client::new();
    let cloud_enabled = owner_context_available(&cfg, boot_policy.as_ref());
    if cloud_enabled {
        bootstrap_api_state(&client, &signer, &mut cfg, boot_policy.as_ref()).await?;
        ensure_runtime_image_ready(&client, &mut cfg, &signer).await?;
    } else {
        println!("mode=local-unbonded");
        println!("status=cloud-bootstrap-skipped");
    }
    ensure_worker_running(&cfg, &signer.public_key_b64url)?;
    save_config(&cfg)?;
    start_local_bridge(local_bridge_listen, &signer.public_key_b64url)?;

    println!("manager=starting");
    println!("backend=tpm");
    println!("device_pubkey_b64url={}", signer.public_key_b64url);
    println!("api_base={}", cfg.api_base);
    println!("rpc_url={}", cfg.rpc_url);
    println!("worker_max_concurrency={}", cfg.worker_max_concurrency);
    println!("worker_mem_bytes={}", cfg.worker_mem_bytes);
    println!("local_bridge_listen={local_bridge_listen}");
    println!("local_bridge_eventbus_path={LOCAL_BRIDGE_EVENTBUS_PATH}");
    println!("cloud_enabled={cloud_enabled}");
    if let Some(image_ref) = cfg.runtime_image_ref.as_deref() {
        println!("runtime_image_ref={image_ref}");
    }

    loop {
        if cloud_enabled && !cfg.runtime_image_pulled {
            match ensure_runtime_image_ready(&client, &mut cfg, &signer).await {
                Ok(()) => {
                    let _ = save_config(&cfg);
                }
                Err(err) => {
                    eprintln!("runtime_image_error={err}");
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            }
        }
        if let Err(err) = ensure_worker_running(&cfg, &signer.public_key_b64url) {
            eprintln!("worker_error={err}");
        }
        if cloud_enabled {
            if let Err(err) = send_heartbeat(&client, &cfg, &signer.public_key_b64url).await {
                eprintln!("heartbeat_error={err}");
            }
        }
        sleep(Duration::from_secs(cfg.heartbeat_secs)).await;
    }
}

fn enforce_boot_policy() -> Result<BootPolicy> {
    if setup_mode_enabled()? {
        auto_import_secure_boot_keys()?;
        return Err(anyhow!(
            "UEFI SetupMode detected; EdgeRun Secure Boot keys were enrolled, reboot required"
        ));
    }

    if !secure_boot_enabled()? {
        return Err(anyhow!("secure boot is disabled; refusing PID 1 startup"));
    }

    let cmdline = fs::read_to_string("/proc/cmdline").context("failed to read /proc/cmdline")?;
    if !cmdline
        .split_ascii_whitespace()
        .any(|p| p == REQUIRED_CMDLINE_LOCK_TOKEN)
    {
        return Err(anyhow!(
            "missing required cmdline token `{REQUIRED_CMDLINE_LOCK_TOKEN}`"
        ));
    }

    for forbidden in ["init=", "rdinit=", "systemd.unit=", "edgerun.insecure="] {
        if cmdline
            .split_ascii_whitespace()
            .any(|p| p.starts_with(forbidden))
        {
            return Err(anyhow!(
                "forbidden cmdline token prefix detected: {forbidden}"
            ));
        }
    }

    let api = cmdline_arg(&cmdline, "api_base").unwrap_or_else(|| DEFAULT_API_BASE.to_string());
    if api.trim_end_matches('/') != DEFAULT_API_BASE {
        return Err(anyhow!(
            "api_base must be locked to {DEFAULT_API_BASE}, got {api}"
        ));
    }

    Ok(BootPolicy {
        owner_pubkey: cmdline_arg(&cmdline, "owner_pubkey"),
    })
}

fn secure_boot_enabled() -> Result<bool> {
    read_efi_bool_var("SecureBoot")
        .and_then(|v| v.ok_or_else(|| anyhow!("SecureBoot efivar not found")))
}

fn setup_mode_enabled() -> Result<bool> {
    Ok(read_efi_bool_var("SetupMode")?.unwrap_or(false))
}

fn read_efi_bool_var(var_prefix: &str) -> Result<Option<bool>> {
    let efivars = Path::new("/sys/firmware/efi/efivars");
    if !efivars.exists() {
        return Err(anyhow!("efivars path missing: {}", efivars.display()));
    }

    for entry in fs::read_dir(efivars).context("failed to list efivars")? {
        let entry = entry.context("failed to read efivars entry")?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(&format!("{var_prefix}-")) {
            continue;
        }
        let data = fs::read(entry.path())
            .with_context(|| format!("failed to read {var_prefix} efivar"))?;
        if data.len() < 5 {
            return Err(anyhow!("invalid {var_prefix} efivar payload"));
        }
        return Ok(Some(data[4] == 1));
    }
    Ok(None)
}

fn auto_import_secure_boot_keys() -> Result<()> {
    let cert_path = if Path::new(EDGE_SECUREBOOT_CERT_DER_PATH).exists() {
        EDGE_SECUREBOOT_CERT_DER_PATH
    } else {
        EDGE_SECUREBOOT_CERT_PEM_PATH
    };
    if !Path::new(EFI_UPDATEVAR_BIN).exists() {
        return Err(anyhow!(
            "SetupMode detected but efi-updatevar is unavailable at {EFI_UPDATEVAR_BIN}"
        ));
    }
    if !Path::new(cert_path).exists() {
        return Err(anyhow!(
            "SetupMode detected but EdgeRun cert is missing at {cert_path}"
        ));
    }

    for var in ["db", "KEK", "PK"] {
        let esl_path = format!("/etc/edgerun/secureboot/{var}.esl");
        let mut command = Command::new(EFI_UPDATEVAR_BIN);
        if Path::new(&esl_path).exists() {
            command.arg("-f").arg(&esl_path);
        } else {
            command.arg("-e").arg("-c").arg(cert_path);
        }
        let output = command
            .arg(var)
            .output()
            .with_context(|| format!("failed to execute efi-updatevar for {var}"))?;
        if !output.status.success() {
            return Err(anyhow!(
                "efi-updatevar failed for {var} with status {} stderr: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
    }
    Ok(())
}

fn cmdline_arg(cmdline: &str, key: &str) -> Option<String> {
    cmdline.split_ascii_whitespace().find_map(|part| {
        part.strip_prefix(&format!("{key}="))
            .map(ToString::to_string)
    })
}

fn owner_context_available(cfg: &ManagerConfig, policy: Option<&BootPolicy>) -> bool {
    cfg.owner_pubkey.is_some() || policy.and_then(|p| p.owner_pubkey.as_ref()).is_some()
}

async fn bootstrap_api_state(
    client: &Client,
    signer: &DeviceSigner,
    cfg: &mut ManagerConfig,
    policy: Option<&BootPolicy>,
) -> Result<()> {
    cfg.api_base = cfg.api_base.trim_end_matches('/').to_string();
    if cfg.api_base.is_empty() {
        cfg.api_base = DEFAULT_API_BASE.to_string();
    }

    if cfg.api_base != DEFAULT_API_BASE {
        return Err(anyhow!(
            "api_base must be {DEFAULT_API_BASE}, got {}",
            cfg.api_base
        ));
    }

    if !cfg.bonded {
        let owner_pubkey = cfg
            .owner_pubkey
            .clone()
            .or_else(|| policy.and_then(|p| p.owner_pubkey.clone()))
            .ok_or_else(|| anyhow!("owner_pubkey is required for first-boot bonding"))?;

        register_device(
            client,
            &cfg.api_base,
            &signer.public_key_b64url,
            &owner_pubkey,
        )
        .await?;
        call_ok_json(
            client,
            &format!("{}/v1/node/bond", cfg.api_base),
            &BondRequest {
                owner_pubkey: &owner_pubkey,
                device_pubkey_b64url: &signer.public_key_b64url,
            },
        )
        .await?;

        cfg.owner_pubkey = Some(owner_pubkey);
        cfg.bonded = true;
        println!("status=bonded");
        println!(
            "onboarding_url={}/register?device={}&owner={}",
            cfg.api_base,
            signer.public_key_b64url,
            cfg.owner_pubkey.as_deref().unwrap_or_default()
        );
    }

    let owner_pubkey = cfg
        .owner_pubkey
        .as_deref()
        .ok_or_else(|| anyhow!("owner_pubkey missing after bonding"))?;

    if !cfg.node_initialized {
        call_ok_json(
            client,
            &format!("{}/v1/node/init", cfg.api_base),
            &NodeInitRequest {
                owner_pubkey,
                device_pubkey_b64url: &signer.public_key_b64url,
                rpc_url: &cfg.rpc_url,
            },
        )
        .await?;
        cfg.node_initialized = true;
        println!("status=node-initialized");
    }

    Ok(())
}

fn read_worker_pid() -> Option<u32> {
    let raw = fs::read_to_string(WORKER_PID_FILE).ok()?;
    raw.trim().parse::<u32>().ok()
}

fn pid_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

fn ensure_worker_running(cfg: &ManagerConfig, worker_pubkey: &str) -> Result<()> {
    if !Path::new(DEFAULT_WORKER_BIN).exists() {
        if Path::new(DEFAULT_CRUN_BIN).exists() {
            if !Path::new(WORKER_RUNTIME_MARKER_FILE).exists() {
                fs::write(WORKER_RUNTIME_MARKER_FILE, "crun-ready\n")
                    .context("failed to write worker runtime marker")?;
                println!("status=worker-runtime=crun-ready");
            }
            return Ok(());
        }
        return Err(anyhow!(
            "worker runtime missing: expected {} or {}",
            DEFAULT_WORKER_BIN,
            DEFAULT_CRUN_BIN
        ));
    }

    if let Some(pid) = read_worker_pid() {
        if pid_alive(pid) {
            return Ok(());
        }
    }

    fs::create_dir_all("/run/edgerun").context("failed to create /run/edgerun")?;
    let mut log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/run/edgerun/edgerun-worker.log")
        .context("failed to open worker log file")?;
    writeln!(log_file, "=== starting edgerun-worker ===").ok();
    let log_file_err = log_file
        .try_clone()
        .context("failed to clone worker log fd")?;

    let child = Command::new(DEFAULT_WORKER_BIN)
        .env("EDGERUN_WORKER_PUBKEY", worker_pubkey)
        .env("EDGERUN_SCHEDULER_URL", &cfg.api_base)
        .env(
            "EDGERUN_WORKER_MAX_CONCURRENT",
            cfg.worker_max_concurrency.to_string(),
        )
        .env("EDGERUN_WORKER_MEM_BYTES", cfg.worker_mem_bytes.to_string())
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err))
        .spawn()
        .context("failed to spawn edgerun-worker")?;

    let pid = child.id();
    fs::write(WORKER_PID_FILE, pid.to_string()).context("failed to write worker pid file")?;
    println!("status=worker-started pid={pid} pubkey={worker_pubkey}");
    Ok(())
}

async fn register_device(
    client: &Client,
    api_base: &str,
    device_pubkey_b64url: &str,
    owner_pubkey: &str,
) -> Result<()> {
    if owner_pubkey.trim().is_empty() {
        return Err(anyhow!("owner_pubkey is required"));
    }

    let challenge_url = format!("{api_base}/v1/device/challenge");
    let challenge_resp = client
        .post(&challenge_url)
        .send()
        .await
        .with_context(|| format!("challenge request failed: {challenge_url}"))?
        .error_for_status()
        .context("challenge endpoint returned error status")?;
    let challenge: DeviceChallengeResponse =
        parse_json_response(challenge_resp, "failed to decode challenge response").await?;

    let handshake_url = format!("{api_base}/v1/device/handshake");
    let handshake_resp = post_json(
        client,
        &handshake_url,
        &DeviceHandshakeRequest {
            owner_pubkey,
            nonce_b64url: &challenge.nonce_b64url,
        },
    )?
    .send()
    .await
    .with_context(|| format!("handshake request failed: {handshake_url}"))?
    .error_for_status()
    .context("handshake endpoint returned error status")?;
    let resp: ApiResponse =
        parse_json_response(handshake_resp, "failed to decode handshake response").await?;

    if !resp.ok {
        return Err(anyhow!(
            "device handshake rejected: {}",
            resp.error.unwrap_or_else(|| "unknown error".to_string())
        ));
    }

    println!("status=registered");
    println!("device_pubkey_b64url={device_pubkey_b64url}");
    println!("owner_pubkey={owner_pubkey}");
    Ok(())
}

async fn ensure_runtime_image_ready(
    client: &Client,
    cfg: &mut ManagerConfig,
    signer: &DeviceSigner,
) -> Result<()> {
    if cfg.runtime_image_pulled {
        return Ok(());
    }
    let image_ref = request_runtime_image_ref(client, cfg, signer).await?;
    pull_runtime_image(&image_ref)?;
    cfg.runtime_image_ref = Some(image_ref.clone());
    cfg.runtime_image_pulled = true;
    println!("status=runtime-image-ready image_ref={image_ref}");
    Ok(())
}

async fn request_runtime_image_ref(
    client: &Client,
    cfg: &mut ManagerConfig,
    signer: &DeviceSigner,
) -> Result<String> {
    let owner_pubkey = cfg
        .owner_pubkey
        .as_deref()
        .ok_or_else(|| anyhow!("owner_pubkey missing for runtime image request"))?
        .to_string();
    let request_issued_at_unix_s = now_unix_s()?;
    let request_nonce_b64url = random_token_b64url(24);
    let request_canonical = runtime_image_request_canonical_message(
        &owner_pubkey,
        &signer.public_key_b64url,
        &cfg.rpc_url,
        &request_nonce_b64url,
        request_issued_at_unix_s,
    );
    let request_signature_b64url = signer.sign_b64url(request_canonical.as_bytes());
    let payload = RuntimeImageRequest {
        owner_pubkey: &owner_pubkey,
        device_pubkey_b64url: &signer.public_key_b64url,
        rpc_url: &cfg.rpc_url,
        request_nonce_b64url: &request_nonce_b64url,
        request_issued_at_unix_s,
        request_signature_b64url: &request_signature_b64url,
    };
    let candidate_paths = ["/v1/node/runtime/image-tag", "/v1/node/image-tag"];
    for path in candidate_paths {
        let url = format!("{}{}", cfg.api_base, path);
        let resp = post_json(client, &url, &payload)?
            .send()
            .await
            .with_context(|| format!("runtime image request failed: {url}"))?;
        if !resp.status().is_success() {
            continue;
        }
        let data: RuntimeImageResponse =
            parse_json_response(resp, &format!("invalid runtime image response from {url}"))
                .await?;
        if !data.ok {
            if let Some(err) = data.error {
                eprintln!("runtime_image_api_error={err}");
            }
            continue;
        }
        if let Some(image_ref) = data.image_ref.as_deref().filter(|s| !s.trim().is_empty()) {
            let image_ref = image_ref.to_string();
            verify_runtime_image_policy_response(
                cfg,
                &owner_pubkey,
                &signer.public_key_b64url,
                &request_nonce_b64url,
                &data,
                &image_ref,
            )?;
            println!("runtime_image_ref_requested={image_ref}");
            return Ok(image_ref);
        }
        if let Some(tag) = data.image_tag.as_deref().filter(|s| !s.trim().is_empty()) {
            let image_ref = format!("ghcr.io/edgerun/worker:{tag}");
            verify_runtime_image_policy_response(
                cfg,
                &owner_pubkey,
                &signer.public_key_b64url,
                &request_nonce_b64url,
                &data,
                &image_ref,
            )?;
            println!("runtime_image_tag_requested={tag}");
            return Ok(image_ref);
        }
    }
    Err(anyhow!(
        "runtime image tag endpoint unavailable or returned no image reference"
    ))
}

fn pull_runtime_image(image_ref: &str) -> Result<()> {
    let runners: &[(&str, &[&str])] = &[
        ("nerdctl", &["pull"]),
        ("ctr", &["images", "pull"]),
        ("podman", &["pull"]),
        ("docker", &["pull"]),
        ("skopeo", &["copy", "--insecure-policy"]),
    ];

    for (bin, args) in runners {
        if !command_in_path(bin) {
            continue;
        }
        let status = if *bin == "skopeo" {
            Command::new(bin)
                .args(*args)
                .arg(format!("docker://{image_ref}"))
                .arg(format!("containers-storage:{image_ref}"))
                .status()
        } else {
            Command::new(bin).args(*args).arg(image_ref).status()
        }
        .with_context(|| format!("failed launching image pull via {bin}"))?;
        if status.success() {
            println!("runtime_image_pull_via={bin}");
            return Ok(());
        }
    }

    Err(anyhow!(
        "no supported image pull tool succeeded (tried nerdctl/ctr/podman/docker/skopeo)"
    ))
}

fn now_unix_s() -> Result<u64> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock before unix epoch")?
        .as_secs())
}

fn runtime_image_request_canonical_message(
    owner_pubkey: &str,
    device_pubkey_b64url: &str,
    rpc_url: &str,
    request_nonce_b64url: &str,
    request_issued_at_unix_s: u64,
) -> String {
    format!(
        "{RUNTIME_IMAGE_REQUEST_SIGNING_CONTEXT}\nowner_pubkey={owner_pubkey}\ndevice_pubkey_b64url={device_pubkey_b64url}\nrpc_url={rpc_url}\nrequest_nonce_b64url={request_nonce_b64url}\nrequest_issued_at_unix_s={request_issued_at_unix_s}"
    )
}

struct RuntimeImagePolicyCanonicalInput<'a> {
    owner_pubkey: &'a str,
    device_pubkey_b64url: &'a str,
    rpc_url: &'a str,
    request_nonce_b64url: &'a str,
    image_ref: &'a str,
    issued_at_unix_s: u64,
    valid_until_unix_s: u64,
    rollback_index: u64,
}

fn runtime_image_policy_canonical_message(input: &RuntimeImagePolicyCanonicalInput<'_>) -> String {
    format!(
        "{RUNTIME_IMAGE_POLICY_SIGNING_CONTEXT}\nowner_pubkey={}\ndevice_pubkey_b64url={}\nrpc_url={}\nrequest_nonce_b64url={}\nimage_ref={}\nissued_at_unix_s={}\nvalid_until_unix_s={}\nrollback_index={}",
        input.owner_pubkey,
        input.device_pubkey_b64url,
        input.rpc_url,
        input.request_nonce_b64url,
        input.image_ref,
        input.issued_at_unix_s,
        input.valid_until_unix_s,
        input.rollback_index
    )
}

fn parse_ed25519_signature_b64url(signature_b64url: &str) -> Result<Signature> {
    let bytes = URL_SAFE_NO_PAD
        .decode(signature_b64url.trim().as_bytes())
        .context("invalid boot policy signature encoding")?;
    let arr: [u8; 64] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("boot policy signature must decode to 64 bytes"))?;
    Ok(Signature::from_bytes(&arr))
}

fn load_boot_policy_verifying_key() -> Result<VerifyingKey> {
    let raw = std::env::var(BOOT_POLICY_VERIFY_KEY_B64URL_ENV).with_context(|| {
        format!("missing {BOOT_POLICY_VERIFY_KEY_B64URL_ENV} for fail-closed boot policy verify")
    })?;
    let bytes = URL_SAFE_NO_PAD
        .decode(raw.trim().as_bytes())
        .with_context(|| format!("invalid base64url in {BOOT_POLICY_VERIFY_KEY_B64URL_ENV}"))?;
    let arr: [u8; 32] = bytes.as_slice().try_into().map_err(|_| {
        anyhow!("{BOOT_POLICY_VERIFY_KEY_B64URL_ENV} must decode to 32-byte ed25519 pubkey")
    })?;
    VerifyingKey::from_bytes(&arr).context("invalid ed25519 verifying key bytes")
}

fn verify_runtime_image_policy_response(
    cfg: &mut ManagerConfig,
    owner_pubkey: &str,
    device_pubkey_b64url: &str,
    request_nonce_b64url: &str,
    response: &RuntimeImageResponse,
    image_ref: &str,
) -> Result<()> {
    let response_nonce = response
        .request_nonce_b64url
        .as_deref()
        .ok_or_else(|| anyhow!("runtime policy response missing request_nonce_b64url"))?;
    if response_nonce != request_nonce_b64url {
        return Err(anyhow!(
            "runtime policy nonce mismatch (expected {request_nonce_b64url}, got {response_nonce})"
        ));
    }
    let issued_at_unix_s = response
        .issued_at_unix_s
        .ok_or_else(|| anyhow!("runtime policy response missing issued_at_unix_s"))?;
    let valid_until_unix_s = response
        .valid_until_unix_s
        .ok_or_else(|| anyhow!("runtime policy response missing valid_until_unix_s"))?;
    if valid_until_unix_s <= issued_at_unix_s {
        return Err(anyhow!(
            "runtime policy validity window is invalid ({issued_at_unix_s}..{valid_until_unix_s})"
        ));
    }
    let now = now_unix_s()?;
    if valid_until_unix_s <= now {
        return Err(anyhow!(
            "runtime policy expired at {valid_until_unix_s}, now={now}"
        ));
    }
    let rollback_index = response
        .rollback_index
        .ok_or_else(|| anyhow!("runtime policy response missing rollback_index"))?;
    if rollback_index < cfg.runtime_policy_rollback_index {
        return Err(anyhow!(
            "runtime policy rollback detected (stored={}, got={rollback_index})",
            cfg.runtime_policy_rollback_index
        ));
    }
    let canonical = runtime_image_policy_canonical_message(&RuntimeImagePolicyCanonicalInput {
        owner_pubkey,
        device_pubkey_b64url,
        rpc_url: &cfg.rpc_url,
        request_nonce_b64url,
        image_ref,
        issued_at_unix_s,
        valid_until_unix_s,
        rollback_index,
    });
    let signature_b64url = response
        .signature_b64url
        .as_deref()
        .ok_or_else(|| anyhow!("runtime policy response missing signature_b64url"))?;
    let signature = parse_ed25519_signature_b64url(signature_b64url)?;
    let verifying_key = load_boot_policy_verifying_key()?;
    verifying_key
        .verify(canonical.as_bytes(), &signature)
        .context("runtime policy signature verification failed")?;
    if let Some(signing_key_id) = response.signing_key_id.as_deref() {
        println!("runtime_policy_signing_key_id={signing_key_id}");
    }
    cfg.runtime_policy_rollback_index = rollback_index;
    Ok(())
}

fn command_in_path(cmd: &str) -> bool {
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {cmd} >/dev/null 2>&1"))
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn send_heartbeat(
    client: &Client,
    cfg: &ManagerConfig,
    device_pubkey_b64url: &str,
) -> Result<()> {
    let owner_pubkey = cfg
        .owner_pubkey
        .as_deref()
        .ok_or_else(|| anyhow!("owner_pubkey missing for heartbeat"))?;

    let url = format!("{}/v1/node/heartbeat", cfg.api_base);
    let resp = post_json(
        client,
        &url,
        &NodeHeartbeatRequest {
            owner_pubkey,
            device_pubkey_b64url,
            rpc_url: &cfg.rpc_url,
            version: env!("CARGO_PKG_VERSION"),
            pid1: std::process::id() == 1,
        },
    )?
    .send()
    .await
    .with_context(|| format!("heartbeat request failed: {url}"))?;

    if resp.status() != StatusCode::OK {
        return Err(anyhow!("heartbeat rejected with status {}", resp.status()));
    }

    println!("heartbeat_ok=true");
    Ok(())
}

async fn call_ok_json<T: Serialize>(client: &Client, url: &str, payload: &T) -> Result<()> {
    let resp = post_json(client, url, payload)?
        .send()
        .await
        .with_context(|| format!("request failed: {url}"))?
        .error_for_status()
        .with_context(|| format!("endpoint returned error status: {url}"))?;
    let resp: ApiResponse =
        parse_json_response(resp, &format!("failed to decode endpoint response: {url}")).await?;

    if !resp.ok {
        return Err(anyhow!(
            "endpoint rejected request at {url}: {}",
            resp.error.unwrap_or_else(|| "unknown error".to_string())
        ));
    }
    Ok(())
}

fn post_json<T: Serialize>(
    client: &Client,
    url: &str,
    payload: &T,
) -> Result<reqwest::RequestBuilder> {
    let body = sonic_rs::to_vec(payload).context("failed to encode json request body")?;
    Ok(client
        .post(url)
        .header(CONTENT_TYPE, "application/json")
        .body(body))
}

async fn parse_json_response<T: DeserializeOwned>(
    resp: reqwest::Response,
    context: &str,
) -> Result<T> {
    let payload = resp
        .bytes()
        .await
        .with_context(|| format!("{context}: failed to read response body"))?;
    sonic_rs::from_slice(&payload).with_context(|| context.to_string())
}

async fn post_protobuf<Req, Resp>(client: &Client, url: &str, payload: &Req) -> Result<Resp>
where
    Req: Message,
    Resp: Message + Default,
{
    let mut body = Vec::with_capacity(payload.encoded_len());
    payload
        .encode(&mut body)
        .context("failed to encode protobuf request body")?;
    let resp = client
        .post(url)
        .header(CONTENT_TYPE, "application/protobuf")
        .body(body)
        .send()
        .await
        .with_context(|| format!("protobuf request failed: {url}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!(
            "endpoint returned error status {}: {url}",
            resp.status()
        ));
    }
    let bytes = resp
        .bytes()
        .await
        .with_context(|| format!("failed to read protobuf response body: {url}"))?;
    Resp::decode(bytes.as_ref())
        .with_context(|| format!("failed to decode protobuf response: {url}"))
}
