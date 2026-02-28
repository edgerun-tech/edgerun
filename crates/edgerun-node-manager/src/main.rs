// SPDX-License-Identifier: Apache-2.0
use std::ffi::CString;
use std::fs;
use std::io::Write;
use std::net::SocketAddr;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, Context, Result};
use axum::extract::ws::{Message as WsMessage, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::header::{
    ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS, ACCESS_CONTROL_ALLOW_ORIGIN,
    CACHE_CONTROL, CONTENT_TYPE as AXUM_CONTENT_TYPE,
};
use axum::http::{HeaderValue, StatusCode as AxumStatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, options};
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use clap::{Parser, Subcommand};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use edgerun_hwvault_primitives::hardware::{
    load_or_create_device_signer, random_token_b64url, tpm_nv_available, tpm_nv_read_blob,
    tpm_nv_write_blob, DeviceSigner, HardwareBackend, HardwareSecurityMode,
};
use edgerun_runtime_proto::local::v1::{
    LocalEventEnvelopeV1, LocalNodeInfoResponseV1,
};
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
use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::time::sleep;

const SECURITY_MODE: HardwareSecurityMode = HardwareSecurityMode::TpmRequired;
const CONFIG_TPM_NV_INDEX: u32 = 0x0150_0026;
const CONFIG_TPM_NV_SIZE: usize = 4096;
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
    started_unix_ms: u64,
    tx: broadcast::Sender<LocalEventEnvelopeV1>,
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

fn start_local_bridge(local_bridge_listen: &str, device_pubkey_b64url: &str) -> Result<()> {
    let addr: SocketAddr = local_bridge_listen
        .parse()
        .with_context(|| format!("invalid local bridge listen addr: {local_bridge_listen}"))?;
    if !addr.ip().is_loopback() {
        return Err(anyhow!(
            "local bridge must bind to loopback only, got {local_bridge_listen}"
        ));
    }
    let state = LocalBridgeState {
        node_id: derive_default_node_id(device_pubkey_b64url),
        device_pubkey_b64url: device_pubkey_b64url.to_string(),
        started_unix_ms: now_unix_ms(),
        tx: broadcast::channel(512).0,
    };
    let app = Router::new()
        .route("/v1/local/node/info.pb", get(handle_local_node_info))
        .route("/v1/local/node/info.pb", options(handle_local_options))
        .route(LOCAL_BRIDGE_EVENTBUS_PATH, get(handle_local_eventbus_ws))
        .with_state(Arc::new(state));
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
            (ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, OPTIONS")),
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
            (ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, OPTIONS")),
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
                    let _ = ws.send(WsMessage::Binary(pong.encode_to_vec().into())).await;
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
    bootstrap_api_state(&client, &signer, &mut cfg, boot_policy.as_ref()).await?;
    ensure_runtime_image_ready(&client, &mut cfg, &signer).await?;
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
    if let Some(image_ref) = cfg.runtime_image_ref.as_deref() {
        println!("runtime_image_ref={image_ref}");
    }

    loop {
        if !cfg.runtime_image_pulled {
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
        if let Err(err) = send_heartbeat(&client, &cfg, &signer.public_key_b64url).await {
            eprintln!("heartbeat_error={err}");
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
