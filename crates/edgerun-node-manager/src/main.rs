// SPDX-License-Identifier: Apache-2.0
use std::ffi::CString;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use edgerun_hwvault_primitives::hardware::{
    load_or_create_device_signer, tpm_nv_available, tpm_nv_read_blob, tpm_nv_write_blob,
    DeviceSigner, HardwareBackend, HardwareSecurityMode,
};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

const SECURITY_MODE: HardwareSecurityMode = HardwareSecurityMode::TpmRequired;
const CONFIG_TPM_NV_INDEX: u32 = 0x0150_0026;
const CONFIG_TPM_NV_SIZE: usize = 4096;
const DEFAULT_API_BASE: &str = "https://api.edgerun.tech";
const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8899";
const DEFAULT_VALIDATOR_WS_URL: &str = "ws://127.0.0.1:8900";
const DEFAULT_VALIDATOR_LEDGER_DIR: &str = "/run/edgerun/solana-ledger";
const DEFAULT_VALIDATOR_BIN: &str = "agave-validator";
const DEFAULT_VALIDATOR_KEYGEN_BIN: &str = "solana-keygen";
const DEFAULT_VALIDATOR_IDENTITY_PATH: &str = "/run/edgerun/validator-identity.json";
const VALIDATOR_PID_FILE: &str = "/run/edgerun/agave-validator.pid";
const REQUIRED_CMDLINE_LOCK_TOKEN: &str = "edgerun.locked_cmdline=1";

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
    Run,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ManagerConfig {
    api_base: String,
    rpc_url: String,
    validator_ws_url: String,
    validator_ledger_dir: String,
    heartbeat_secs: u64,
    bonded: bool,
    stake_initialized: bool,
    node_initialized: bool,
    owner_pubkey: Option<String>,
}

#[derive(Debug)]
struct BootPolicy {
    owner_pubkey: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    bootstrap_pid1_runtime();
    let cli = Cli::parse();
    match cli.command.unwrap_or(Commands::Run) {
        Commands::Configure {
            rpc_url,
            heartbeat_secs,
        } => cmd_configure(&rpc_url, heartbeat_secs),
        Commands::Bond { owner_pubkey } => cmd_bond(&owner_pubkey),
        Commands::Identity => cmd_identity(),
        Commands::Register { owner_pubkey } => cmd_register(&owner_pubkey).await,
        Commands::Run => cmd_run().await,
    }
}

fn bootstrap_pid1_runtime() {
    if std::process::id() != 1 {
        return;
    }
    for dir in ["/proc", "/sys", "/dev", "/run"] {
        let _ = fs::create_dir_all(dir);
    }
    let _ = mount_fs("proc", "/proc", "proc");
    let _ = mount_fs("sysfs", "/sys", "sysfs");
    let _ = mount_fs("devtmpfs", "/dev", "devtmpfs");
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
    serde_json::from_slice(raw).context("failed to parse manager config json from TPM")
}

fn save_config(cfg: &ManagerConfig) -> Result<()> {
    let payload = serde_json::to_vec(cfg).context("failed to encode manager config")?;
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
        validator_ws_url: DEFAULT_VALIDATOR_WS_URL.to_string(),
        validator_ledger_dir: DEFAULT_VALIDATOR_LEDGER_DIR.to_string(),
        heartbeat_secs,
        bonded: false,
        stake_initialized: false,
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

async fn cmd_run() -> Result<()> {
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
        validator_ws_url: DEFAULT_VALIDATOR_WS_URL.to_string(),
        validator_ledger_dir: DEFAULT_VALIDATOR_LEDGER_DIR.to_string(),
        heartbeat_secs: 15,
        bonded: false,
        stake_initialized: false,
        node_initialized: false,
        owner_pubkey: None,
    });

    if cfg.heartbeat_secs == 0 {
        cfg.heartbeat_secs = 15;
    }
    cfg.rpc_url = DEFAULT_RPC_URL.to_string();
    if cfg.validator_ws_url.is_empty() {
        cfg.validator_ws_url = DEFAULT_VALIDATOR_WS_URL.to_string();
    }
    if cfg.validator_ledger_dir.is_empty() {
        cfg.validator_ledger_dir = DEFAULT_VALIDATOR_LEDGER_DIR.to_string();
    }

    let client = Client::new();
    ensure_local_validator(&client, &mut cfg).await?;
    bootstrap_api_state(&client, &signer, &mut cfg, boot_policy.as_ref()).await?;
    save_config(&cfg)?;

    println!("manager=starting");
    println!("backend=tpm");
    println!("device_pubkey_b64url={}", signer.public_key_b64url);
    println!("api_base={}", cfg.api_base);
    println!("rpc_url={}", cfg.rpc_url);
    println!("validator_ws_url={}", cfg.validator_ws_url);

    loop {
        if let Err(err) = ensure_local_validator(&client, &mut cfg).await {
            eprintln!("validator_error={err}");
        }
        if let Err(err) = send_heartbeat(&client, &cfg, &signer.public_key_b64url).await {
            eprintln!("heartbeat_error={err}");
        }
        sleep(Duration::from_secs(cfg.heartbeat_secs)).await;
    }
}

fn enforce_boot_policy() -> Result<BootPolicy> {
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
    let efivars = Path::new("/sys/firmware/efi/efivars");
    if !efivars.exists() {
        return Err(anyhow!("efivars path missing: {}", efivars.display()));
    }

    for entry in fs::read_dir(efivars).context("failed to list efivars")? {
        let entry = entry.context("failed to read efivars entry")?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("SecureBoot-") {
            continue;
        }
        let data = fs::read(entry.path()).context("failed to read SecureBoot efivar")?;
        if data.len() < 5 {
            return Err(anyhow!("invalid SecureBoot efivar payload"));
        }
        return Ok(data[4] == 1);
    }

    Err(anyhow!("SecureBoot efivar not found"))
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
    }

    let owner_pubkey = cfg
        .owner_pubkey
        .as_deref()
        .ok_or_else(|| anyhow!("owner_pubkey missing after bonding"))?;

    if !cfg.stake_initialized {
        call_ok_json(
            client,
            &format!("{}/v1/node/stake/init", cfg.api_base),
            &NodeInitRequest {
                owner_pubkey,
                device_pubkey_b64url: &signer.public_key_b64url,
                rpc_url: &cfg.rpc_url,
            },
        )
        .await?;
        cfg.stake_initialized = true;
        println!("status=stake-initialized");
    }

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

async fn ensure_local_validator(client: &Client, cfg: &mut ManagerConfig) -> Result<()> {
    cfg.rpc_url = DEFAULT_RPC_URL.to_string();

    if validator_healthy(client, &cfg.rpc_url).await {
        return Ok(());
    }

    if let Some(pid) = read_validator_pid() {
        if pid_alive(pid) {
            wait_for_validator(client, &cfg.rpc_url, Duration::from_secs(20)).await?;
            return Ok(());
        }
    }

    let rpc_port = reqwest::Url::parse(&cfg.rpc_url)
        .context("invalid rpc_url for local validator")?
        .port_or_known_default()
        .ok_or_else(|| anyhow!("rpc_url missing port: {}", cfg.rpc_url))?;

    let ws_port = reqwest::Url::parse(&cfg.validator_ws_url)
        .context("invalid validator_ws_url")?
        .port_or_known_default()
        .ok_or_else(|| anyhow!("validator_ws_url missing port: {}", cfg.validator_ws_url))?;

    fs::create_dir_all("/run/edgerun").context("failed to create /run/edgerun")?;
    fs::create_dir_all(&cfg.validator_ledger_dir)
        .with_context(|| format!("failed to create {}", cfg.validator_ledger_dir))?;
    ensure_validator_identity(DEFAULT_VALIDATOR_IDENTITY_PATH)?;
    ensure_validator_ledger_initialized(&cfg.validator_ledger_dir)?;

    let mut log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("/run/edgerun/agave-validator.log")
        .context("failed to open validator log file")?;
    writeln!(log_file, "=== starting agave-validator ===").ok();
    let log_file_err = log_file
        .try_clone()
        .context("failed to clone validator log fd")?;

    let mut child = Command::new(DEFAULT_VALIDATOR_BIN)
        .arg("--identity")
        .arg(DEFAULT_VALIDATOR_IDENTITY_PATH)
        .arg("--ledger")
        .arg(&cfg.validator_ledger_dir)
        .arg("--rpc-bind-address")
        .arg("127.0.0.1")
        .arg("--rpc-port")
        .arg(rpc_port.to_string())
        .arg("--dynamic-port-range")
        .arg(format!("{ws_port}-{}", ws_port + 40))
        .arg("--bind-address")
        .arg("127.0.0.1")
        .arg("--no-voting")
        .arg("--private-rpc")
        .arg("--full-rpc-api")
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err))
        .spawn()
        .context("failed to spawn agave-validator")?;

    let pid = child.id();
    fs::write(VALIDATOR_PID_FILE, pid.to_string()).context("failed to write validator pid file")?;

    // Detach child; manager keeps liveness through pid+health checks.
    let _ = child.stdout.take();
    let _ = child.stderr.take();

    wait_for_validator(client, &cfg.rpc_url, Duration::from_secs(45)).await?;
    println!("status=validator-started pid={pid} rpc_url={}", cfg.rpc_url);
    Ok(())
}

fn ensure_validator_identity(identity_path: &str) -> Result<()> {
    if Path::new(identity_path).exists() {
        return Ok(());
    }
    let rc = Command::new(DEFAULT_VALIDATOR_KEYGEN_BIN)
        .arg("new")
        .arg("--no-bip39-passphrase")
        .arg("--silent")
        .arg("--force")
        .arg("--outfile")
        .arg(identity_path)
        .status()
        .context("failed to launch solana-keygen")?;
    if !rc.success() {
        return Err(anyhow!(
            "solana-keygen failed creating validator identity: {rc}"
        ));
    }
    Ok(())
}

fn ensure_validator_ledger_initialized(ledger_dir: &str) -> Result<()> {
    let genesis = Path::new(ledger_dir).join("genesis.bin");
    if genesis.exists() {
        return Ok(());
    }
    let rc = Command::new(DEFAULT_VALIDATOR_BIN)
        .arg("--ledger")
        .arg(ledger_dir)
        .arg("init")
        .status()
        .context("failed to launch agave-validator init")?;
    if !rc.success() {
        return Err(anyhow!("agave-validator init failed with status {rc}"));
    }
    Ok(())
}

fn read_validator_pid() -> Option<u32> {
    let raw = fs::read_to_string(VALIDATOR_PID_FILE).ok()?;
    raw.trim().parse::<u32>().ok()
}

fn pid_alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

async fn wait_for_validator(client: &Client, rpc_url: &str, timeout: Duration) -> Result<()> {
    let deadline = std::time::Instant::now() + timeout;
    while std::time::Instant::now() < deadline {
        if validator_healthy(client, rpc_url).await {
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
    }
    Err(anyhow!("local validator not healthy at {rpc_url}"))
}

async fn validator_healthy(client: &Client, rpc_url: &str) -> bool {
    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getVersion",
    });
    match client.post(rpc_url).json(&payload).send().await {
        Ok(resp) => resp.status() == StatusCode::OK,
        Err(_) => false,
    }
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
    let challenge: DeviceChallengeResponse = client
        .post(&challenge_url)
        .send()
        .await
        .with_context(|| format!("challenge request failed: {challenge_url}"))?
        .error_for_status()
        .context("challenge endpoint returned error status")?
        .json()
        .await
        .context("failed to decode challenge response")?;

    let handshake_url = format!("{api_base}/v1/device/handshake");
    let resp: ApiResponse = client
        .post(&handshake_url)
        .json(&DeviceHandshakeRequest {
            owner_pubkey,
            nonce_b64url: &challenge.nonce_b64url,
        })
        .send()
        .await
        .with_context(|| format!("handshake request failed: {handshake_url}"))?
        .error_for_status()
        .context("handshake endpoint returned error status")?
        .json()
        .await
        .context("failed to decode handshake response")?;

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
    let resp = client
        .post(&url)
        .json(&NodeHeartbeatRequest {
            owner_pubkey,
            device_pubkey_b64url,
            rpc_url: &cfg.rpc_url,
            version: env!("CARGO_PKG_VERSION"),
            pid1: std::process::id() == 1,
        })
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
    let resp: ApiResponse = client
        .post(url)
        .json(payload)
        .send()
        .await
        .with_context(|| format!("request failed: {url}"))?
        .error_for_status()
        .with_context(|| format!("endpoint returned error status: {url}"))?
        .json()
        .await
        .with_context(|| format!("failed to decode endpoint response: {url}"))?;

    if !resp.ok {
        return Err(anyhow!(
            "endpoint rejected request at {url}: {}",
            resp.error.unwrap_or_else(|| "unknown error".to_string())
        ));
    }
    Ok(())
}
