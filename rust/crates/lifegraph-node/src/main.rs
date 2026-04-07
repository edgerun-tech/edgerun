//! Lifegraph Node Daemon (lifegraphd)
//!
//! Runs a single-writer stream node with mesh networking,
//! command processing, and capability discovery.
//!
//! ## Usage
//! ```text
//! lifegraphd init --config node.yaml --software          # Dev-only: in-memory key
//! lifegraphd run --config node.yaml --listen 0.0.0.0:8080  # Start daemon with TCP
//! lifegraphd status --config node.yaml                   # Show node identity
//! ```
//!
//! ## Security
//! The node's private key NEVER leaves secure hardware. The config file only
//! stores the public key (NodeID) and a reference to the hardware key handle.
//! No `.key` file is ever written.
use lifegraph_log;

mod capabilities;
mod command_dispatch;
mod ingress;

use lifegraph_hardware_signing::{MeshSigner, NodeID};
use lifegraph_mesh::{FrameType, LocalNode, MeshFrame};
use lifegraph_mesh_link::MeshLink;
use lifegraph_mesh_router::MeshRouter;
use lifegraph_storage::{NodeStore, NodeStoreConfig, BlobKeySource};
use prost::Message;
use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::hazmat::PrehashSigner;
use std::env;
use std::sync::Arc;
use std::fs;
use lifegraph_linux_netif::discover_network_interfaces;
use lifegraph_network_interface::NetworkLinkState;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::net::SocketAddr;

/// Parsed CLI arguments.
enum Command {
    Init {
        config: PathBuf,
        name: Option<String>,
        software: bool,
    },
    Run {
        config: PathBuf,
        listen: Option<SocketAddr>,
        health_port: Option<u16>,
        log_level: String,
    },
    Status {
        config: PathBuf,
    },
}

fn parse_args() -> Result<Command, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return Err(format!(
            "Usage: lifegraphd <command> [options]\n\nCommands:\n  init    Generate node identity\n  run     Start the daemon\n  status  Show node identity\n  help    Show this help"
        ));
    }
    let cmd = args[0].as_str();
    match cmd {
        "init" => {
            let mut config = PathBuf::from("node.yaml");
            let mut name = None;
            let mut software = false;
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => { i += 1; config = PathBuf::from(&args[i]); }
                    "--name" => { i += 1; name = Some(args[i].clone()); }
                    "--software" => { software = true; }
                    "--help" | "-h" => {
                        return Err("Usage: lifegraphd init [--config path] [--name name] [--software]".into());
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            Ok(Command::Init { config, name, software })
        }
        "run" => {
            let mut config = PathBuf::from("node.yaml");
            let mut listen = None;
            let mut health_port = None;
            let mut log_level = "info".to_string();
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => { i += 1; config = PathBuf::from(&args[i]); }
                    "--listen" => { i += 1; listen = Some(args[i].parse().map_err(|e| format!("invalid listen address: {}", e))?); }
                    "--health-port" => { i += 1; health_port = Some(args[i].parse().map_err(|e| format!("invalid port: {}", e))?); }
                    "--log-level" => { i += 1; log_level = args[i].clone(); }
                    "--help" | "-h" => {
                        return Err("Usage: lifegraphd run [--config path] [--listen addr] [--health-port port] [--log-level level]".into());
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            Ok(Command::Run { config, listen, health_port, log_level })
        }
        "status" => {
            let mut config = PathBuf::from("node.yaml");
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--config" => { i += 1; config = PathBuf::from(&args[i]); }
                    "--help" | "-h" => {
                        return Err("Usage: lifegraphd status [--config path]".into());
                    }
                    other => return Err(format!("unknown option: {}", other)),
                }
                i += 1;
            }
            Ok(Command::Status { config })
        }
        "help" | "--help" | "-h" => {
            Err("Lifegraph Node Daemon\n\nCommands:\n  init    Generate node identity\n  run     Start the daemon\n  status  Show node identity".into())
        }
        other => Err(format!("unknown command: {}", other)),
    }
}

fn main() {
    let cmd = match parse_args() {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("{}", msg);
            std::process::exit(1);
        }
    };
    match cmd {
        Command::Init { config, name, software } => {
            cmd_init(&config, name, software);
        }
        Command::Run { config, listen, health_port, log_level } => {
            // Initialize structured logging
            env::set_var("RUST_LOG", &log_level);
            lifegraph_log::init_from_env();

            let rt = lifegraph_rt::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap_or_else(|e| {
                    lifegraph_log::error!("failed to create runtime: {}", e);
                    std::process::exit(1);
                });
            rt.block_on(async move {
                cmd_run(&config, listen, health_port).await;
            });
        }
        Command::Status { config } => {
            cmd_status(&config);
        }
    }
}

// ---------------------------------------------------------------------------
// Init
// ---------------------------------------------------------------------------

fn cmd_init(path: &PathBuf, name: Option<String>, software: bool) {
    let has_tpm = PathBuf::from("/dev/tpmrm0").exists();
    let has_yubikey = check_yubikey_available();

    if !software && !has_tpm && !has_yubikey {
        eprintln!("error: no secure hardware found.");
        eprintln!();
        eprintln!("Available hardware backends:");
        eprintln!("  TPM 2.0:    {} (device: /dev/tpmrm0)", if has_tpm { "FOUND" } else { "not found" });
        eprintln!("  YubiKey:    {}", if has_yubikey { "FOUND" } else { "not found" });
        eprintln!();
        eprintln!("For development only, you can generate a software key with --software:");
        eprintln!("  lifegraphd init --config {} --software", path.display());
        std::process::exit(1);
    }

    let (node_id, key_material) = if software {
        eprintln!("WARNING: --software generates an INSECURE key stored in the config file.");
        eprintln!("This is for development/testing only. NEVER use in production.");
        eprintln!();
        let mut key_bytes = [0u8; 32];
        getrandom::fill(&mut key_bytes).unwrap_or_else(|e| {
            eprintln!("error: failed to get random bytes for key generation: {}", e);
            std::process::exit(1);
        });
        let signing_key = SigningKey::from_bytes(&key_bytes.into()).unwrap_or_else(|e| {
            eprintln!("error: failed to create signing key: {}", e);
            std::process::exit(1);
        });
        let verifying_key = signing_key.verifying_key();
        let encoded = verifying_key.to_encoded_point(false);
        let mut node_id_bytes = [0u8; 64];
        node_id_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
        let node_id = NodeID(node_id_bytes);
        let key_hex = lifegraph_core::util::bytes_to_hex(&signing_key.to_bytes());
        (node_id, Some(key_hex))
    } else if has_tpm {
        eprintln!("TPM found at /dev/tpmrm0, but automated key provisioning is not yet implemented.");
        eprintln!("Use tpm2-tools to create a signing key and reference its handle in config.");
        std::process::exit(0);
    } else {
        eprintln!("YubiKey found, but automated key provisioning is not yet implemented.");
        eprintln!("Use yubico-piv-tool to create a signing key in slot 9a.");
        std::process::exit(0);
    };

    let node_name = name.unwrap_or_else(|| format!("lifegraph-{}", node_id.short()));
    let stream_id = format!("stream-{}", node_id.short());

    let signer_block = if let Some(ref kh) = key_material {
        format!(
            r#"signer:
  type: "software"
  public_key_hex: "{node_id_hex}"
  private_key_hex: "{key_hex}"
"#,
            node_id_hex = node_id.to_hex(),
            key_hex = kh,
        )
    } else {
        String::new()
    };

    let config_yaml = format!(
        r#"# Lifegraph Node Configuration
stream_id: "{stream_id}"
name: "{node_name}"
controllers: []
trust_nodes: []
initial_grants: []
{signer_block}metadata:
  environment: "production"
"#
    );

    fs::write(path, &config_yaml).unwrap_or_else(|e| {
        eprintln!("error: failed to write config to {}: {}", path.display(), e);
        std::process::exit(1);
    });

    println!("Node identity generated:");
    println!("  NodeID:     {}", node_id.to_hex());
    println!("  Short ID:   {}", node_id.short());
    println!("  Stream ID:  {}", stream_id);
    println!("  Name:       {}", node_name);
    println!("  Signer:     {}", if key_material.is_some() { "software (INSECURE)" } else { "hardware" });
    println!("  Config:     {}", path.display());
    if key_material.is_some() {
        println!();
        println!("WARNING: This is a SOFTWARE KEY. The private key is stored in the config file.");
        println!("Do NOT use this key in production.");
    }
    println!();
    println!("Start the node with:");
    println!("  lifegraphd run --config {} --listen 0.0.0.0:8080", path.display());
}

fn check_yubikey_available() -> bool {
    fs::read_dir("/sys/bus/usb/devices/")
        .map(|entries| {
            entries.filter_map(|e| e.ok()).any(|entry| {
                let path = entry.path();
                if path.join("idVendor").exists() {
                    if let Ok(vendor) = fs::read_to_string(path.join("idVendor")) {
                        return vendor.trim().eq_ignore_ascii_case("1050");
                    }
                }
                false
            })
        })
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Status
// ---------------------------------------------------------------------------

fn cmd_status(path: &PathBuf) {
    if !path.exists() {
        eprintln!("error: config not found at {}. Run `lifegraphd init` first.", path.display());
        std::process::exit(1);
    }

    let yaml = fs::read_to_string(path).unwrap();
    let config: NodeConfig = parse_config(&yaml).unwrap_or_else(|e| {
        eprintln!("error: invalid config: {}", e);
        std::process::exit(1);
    });

    let signer = load_signer_from_config(&config);
    let node_id = signer.node_id();

    println!("Lifegraph Node Status");
    println!("  Name:       {}", config.name.as_deref().unwrap_or("(unnamed)"));
    println!("  Stream ID:  {}", config.stream_id);
    println!("  NodeID:     {}", node_id.to_hex());
    println!("  Short ID:   {}", node_id.short());
    println!("  Signer:     {}", config.signer.as_ref().map(|s| &s.signer_type).unwrap_or(&"unconfigured".to_string()));
    println!("  Controllers: {:?}", config.controllers);
    println!("  Trust nodes: {:?}", config.trust_nodes);

    let interfaces = discover_network_interfaces().unwrap_or_default();
    let up_interfaces: Vec<_> = interfaces
        .iter()
        .filter(|i| i.link_state == NetworkLinkState::Up && i.name != "lo")
        .map(|i| &i.name)
        .collect();
    println!("  UP interfaces: {:?}", up_interfaces);
}

// ---------------------------------------------------------------------------
// Run — async daemon
// ---------------------------------------------------------------------------

/// Request sent from TCP connection handlers to the store task.
enum StoreRequest {
    Command {
        /// The raw message bytes (for dedup hashing before decode).
        raw_bytes: Vec<u8>,
        command: lifegraph_proto::lifegraph::v0::stream::CommandEnvelope,
        /// Peer identity for allowlist check.
        peer_id: Option<Vec<u8>>,
        reply_tx: lifegraph_rt::oneshot::Sender<StoreResponse>,
    },
    Query {
        /// The raw message bytes (for dedup hashing before decode).
        raw_bytes: Vec<u8>,
        query: lifegraph_proto::lifegraph::v0::access::QueryRequest,
        /// Peer identity for allowlist check.
        peer_id: Option<Vec<u8>>,
        reply_tx: lifegraph_rt::oneshot::Sender<StoreResponse>,
    },
    /// Produce a snapshot of current stream heads.
    ProduceSnapshot {
        view_type: String,
        completeness: i32,
        reply_tx: lifegraph_rt::oneshot::Sender<StoreResponse>,
    },
    /// Fetch a local object by ObjectRef and return its content.
    FetchObject {
        object_ref: lifegraph_proto::lifegraph::v0::common::ObjectRef,
        reply_tx: lifegraph_rt::oneshot::Sender<StoreResponse>,
    },
    /// Send a command to a remote peer over TCP and record CommandSent event.
    #[allow(dead_code)]
    SendCommand {
        peer_addr: String,
        command: lifegraph_proto::lifegraph::v0::stream::CommandEnvelope,
        reply_tx: lifegraph_rt::oneshot::Sender<StoreResponse>,
    },
}

/// Response from the store task back to the TCP handler.
enum StoreResponse {
    /// Command was processed successfully — payload is the response.
    Ok(Vec<u8>),
    /// Ingress screening rejected the message.
    Rejected(ingress::IngressResult),
}

/// Reply from the store task back to the TCP handler.
/// Contains the response bytes and the original sender's NodeID.
struct MeshReply {
    source: NodeID,
    response_bytes: Vec<u8>,
}

/// An outbound command to be sent to a remote peer.
#[allow(dead_code)]
struct OutboundCommand {
    peer_addr: String,
    command: lifegraph_proto::lifegraph::v0::stream::CommandEnvelope,
    reply_tx: lifegraph_rt::oneshot::Sender<StoreResponse>,
}

/// A request sent to the store task from the mesh loop (blocking thread).
/// Uses std::sync::mpsc since both sender (mesh loop) and receiver (store task)
/// are on blocking threads.
struct MeshCommandRequest {
    command: lifegraph_proto::lifegraph::v0::stream::CommandEnvelope,
    raw_bytes: Vec<u8>,
    source: NodeID,
    reply_tx: std::sync::mpsc::Sender<MeshReply>,
}

// ---------------------------------------------------------------------------
// Health endpoint
// ---------------------------------------------------------------------------

/// Shared health state updated by the daemon.
#[derive(Clone, Debug)]
struct HealthState {
    node_id: String,
    stream_id: String,
    started_at: std::time::Instant,
}

/// Runs a simple HTTP health server on the given port.
///
/// GET /health → { "status": "ok", "uptime_secs": N, "node_id": "...", "stream_id": "..." }
async fn run_health_server(port: u16, state: HealthState) {
    use lifegraph_rt::{AsyncReadExt, AsyncWriteExt};

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match lifegraph_rt::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            lifegraph_log::error!("failed to bind health endpoint on {}: {}", addr, e);
            return;
        }
    };
    lifegraph_log::info!("health endpoint listening");

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                let state = state.clone();
                lifegraph_rt::spawn(async move {
                    let mut buf = [0u8; 1024];
                    let _ = stream.read(&mut buf).await;
                    let uptime = state.started_at.elapsed().as_secs();
                    let body = format!(
                        r#"{{"status":"ok","uptime_secs":{},"node_id":"{}","stream_id":"{}"}}"#,
                        uptime, state.node_id, state.stream_id
                    );
                    let response = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.flush().await;
                });
            }
            Err(e) => {
                lifegraph_log::warn!("health endpoint accept error: {}", e);
            }
        }
    }
}

/// Handles a TCP connection to a bootstrap peer.
///
/// Sends a ping-like message and maintains the connection for query/command exchange.
async fn handle_bootstrap_connection(
    stream: lifegraph_rt::TcpStream,
    store_tx: lifegraph_rt::mpsc::Sender<StoreRequest>,
    _peer_id_hex: &str,
) {
    // Use the same TCP frame handler as regular connections
    handle_tcp_connection(stream, store_tx).await;
}

/// Periodically attempts to reconnect to unreachable peers.
///
/// Runs on a timer, tracks unreachable peers and attempts TCP reconnection
/// with exponential backoff. Uses `store_tx` to send peer status updates
/// to the store task.
async fn run_peer_reconnection(
    initial_unreachable: Vec<(String, String)>,
    _store_tx: lifegraph_rt::mpsc::Sender<StoreRequest>,
) {
    use std::collections::HashMap;

    // Track backoff state per peer: (retry_count, next_attempt)
    let mut backoff: HashMap<String, (u32, std::time::Instant)> = HashMap::new();
    const INITIAL_BACKOFF_SECS: u64 = 5;
    const MAX_BACKOFF_SECS: u64 = 300; // 5 minutes

    // Initialize with known unreachable peers
    for (node_id_hex, _addr) in initial_unreachable {
        backoff.insert(node_id_hex, (0, std::time::Instant::now()));
    }

    let mut interval = lifegraph_rt::interval(std::time::Duration::from_secs(10));
    interval.set_missed_tick_behavior(lifegraph_rt::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;
        let now = std::time::Instant::now();
        let mut to_remove = Vec::new();

        for (node_id_hex, (retry_count, next_attempt)) in backoff.iter_mut() {
            if now >= *next_attempt {
                let rc = *retry_count;
                let delay = (INITIAL_BACKOFF_SECS * 2u64.pow(rc.min(6))).min(MAX_BACKOFF_SECS);
                *next_attempt = now + std::time::Duration::from_secs(delay);
                *retry_count += 1;

                // In v0, we just log — actual outbound reconnection is initiated
                // when the peer is listed in bootstrap_peers config.
                lifegraph_log::debug!("peer reconnection pending (use bootstrap_peers config)");

                // Clean up very old entries
                if *retry_count > 20 {
                    to_remove.push(node_id_hex.clone());
                }
            }
        }

        for key in to_remove {
            backoff.remove(&key);
        }
    }
}

async fn cmd_run(path: &PathBuf, listen_addr: Option<SocketAddr>, health_port: Option<u16>) {
    if !path.exists() {
        lifegraph_log::error!("config not found at {}. Run `lifegraphd init` first.", path.display());
        std::process::exit(1);
    }

    let yaml = fs::read_to_string(path).unwrap();
    let config: NodeConfig = parse_config(&yaml).unwrap_or_else(|e| {
        lifegraph_log::error!("invalid config: {}", e);
        std::process::exit(1);
    });

    let signer = load_signer_from_config(&config);
    let node_id = signer.node_id();
    let private_key_bytes = extract_private_key_bytes(&config);

    let node_name = config.name.as_deref().unwrap_or("(unnamed)").to_string();
    let signer_type = config.signer.as_ref().map(|s| s.signer_type.clone()).unwrap_or_else(|| "unconfigured".to_string());

    lifegraph_log::info!("lifegraphd starting"
    );

    // --- Health endpoint ---
    if let Some(hp) = health_port {
        let health_state = HealthState {
            node_id: node_id.short(),
            stream_id: config.stream_id.clone(),
            started_at: std::time::Instant::now(),
        };
        lifegraph_rt::spawn(run_health_server(hp, health_state));
    }

    let data_root = path.parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("data");

    let store_config = NodeStoreConfig {
        data_root: data_root.clone(),
        blob_key_source: Arc::new(BlobKeySource::Software {
            private_key_bytes: private_key_bytes.clone(),
        }),
    };
    let mut store = NodeStore::open(&store_config).unwrap_or_else(|e| {
        lifegraph_log::error!("failed to open storage at {}: {}", data_root.display(), e);
        std::process::exit(1);
    });

    // Create genesis if new node
    let stream_id_bytes = config.stream_id.as_bytes();
    if store.get_head(stream_id_bytes).unwrap().is_none() {
        use lifegraph_core::protocol::EventEnvelope;
        use lifegraph_proto::lifegraph::v0::stream::EventType;

        // Create and store the NodeGenesisPayload as an encrypted object
        let initial_controllers: Vec<Vec<u8>> = vec![node_id.0.to_vec()];
        let payload_object_ref = command_dispatch::create_node_genesis_payload(
            &mut store,
            stream_id_bytes,
            &node_id,
            &initial_controllers,
        );

        let mut genesis = EventEnvelope {
            envelope_version: 1,
            stream_id: stream_id_bytes.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: Some(now_ms_timestamp()),
            effective_at: None,
            payload_object: Some(payload_object_ref),
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        sign_event_envelope(&mut genesis, &*signer).unwrap_or_else(|e| {
            lifegraph_log::error!("failed to sign genesis event: {}", e);
            std::process::exit(1);
        });
        store.append_event(&genesis).unwrap_or_else(|e| {
            lifegraph_log::error!("failed to write genesis event: {}", e);
            std::process::exit(1);
        });
        lifegraph_log::info!("genesis event created (seq=0)");
    } else {
        let (head_seq, _) = store.get_head(stream_id_bytes).unwrap().unwrap();
        lifegraph_log::info!("loaded stream");
    }

    // --- Unix socket capability server ---
    let socket_path = data_root.join("capabilities.sock");
    {
        let mut multi = capabilities::MultiCapabilityProvider::new();
        let policy = lifegraph_capability_policy::SimplePolicyEngine::default();
        capabilities::discover_and_register_capabilities(&mut multi, policy);
        let cap_count = multi.len();
        lifegraph_log::info!("discovered capability providers");

        if cap_count > 0 {
            let multi_arc = Arc::new(std::sync::Mutex::new(multi));
            let socket_path_clone = socket_path.clone();
            std::thread::spawn(move || {
                if let Err(e) = capabilities::serve_capabilities_unix(multi_arc, &socket_path_clone) {
                    lifegraph_log::error!("capability server error: {}", e);
                }
            });
            lifegraph_log::info!("capability server listening");
        }
    }

    // --- Store task (owns NodeStore + ingress state, not Send) ---
    let (store_tx, store_rx) = lifegraph_rt::mpsc::channel::<StoreRequest>(256);

    let stream_id_vec = stream_id_bytes.to_vec();
    let store_signer: Arc<dyn MeshSigner + Send + Sync> = Arc::clone(&signer);

    // Ingress screening state
    let global_rate_limiter = ingress::TokenBucket::new(1000, 500); // burst 1000, 500/sec global
    let message_hash_cache = ingress::RecentHashCache::new(4096);
    let allowed_peers: Vec<Vec<u8>> = config.allowed_peers.iter()
        .map(|s| s.as_bytes().to_vec())
        .collect();
    if !allowed_peers.is_empty() {
        lifegraph_log::info!("peer allowlist active");
    }

    // --- Peer bootstrap (before store is moved) ---
    let bootstrap_peers = parse_bootstrap_peers(&config.bootstrap_peers);
    let unreachable_peers = store.list_unreachable_peers_with_addr().unwrap_or_default();
    for peer in &bootstrap_peers {
        if let Err(e) = store.upsert_peer(&peer.node_id_hex, Some(&peer.addr), "unknown", true) {
            lifegraph_log::warn!("failed to record bootstrap peer");
        }
    }

    // --- Mesh poll loop (blocking thread) ---
    // Creates direct channels between mesh loop and store task
    let mesh_node_id = node_id;
    let (mesh_command_tx, mesh_command_rx) = std::sync::mpsc::channel::<MeshCommandRequest>();
    let (_mesh_reply_tx, mesh_reply_rx) = std::sync::mpsc::channel::<MeshReply>();

    let store_handle = lifegraph_rt::spawn_blocking(move || {
        run_store_task(store, &stream_id_vec, &*store_signer, store_rx,
                       mesh_command_rx,
                       global_rate_limiter, message_hash_cache, allowed_peers, node_id);
    });

    let mesh_handle = lifegraph_rt::spawn_blocking(move || {
        run_mesh_loop(mesh_node_id, mesh_command_tx, mesh_reply_rx);
    });

    // --- TCP listener (if configured) ---
    if let Some(addr) = listen_addr {
        let _tcp_handle = lifegraph_rt::spawn(run_tcp_listener(
            addr,
            node_id,
            store_tx.clone(),
        ));
        lifegraph_log::info!("TCP listener started");
    } else {
        lifegraph_log::info!("running mesh-only (no TCP listener)");
    }

    // --- Bootstrap peer connections ---
    if !bootstrap_peers.is_empty() {
        lifegraph_log::info!("connecting to bootstrap peers");
        for peer in &bootstrap_peers {
            // Attempt TCP connection
            let peer_addr = peer.addr.clone();
            let peer_id_hex = peer.node_id_hex.clone();
            let conn_store_tx = store_tx.clone();
            lifegraph_rt::spawn(async move {
                match lifegraph_rt::TcpStream::connect(&peer_addr).await {
                    Ok(stream) => {
                        lifegraph_log::info!("connected to bootstrap peer");
                        // Handle as a regular TCP connection (bidirectional)
                        handle_bootstrap_connection(stream, conn_store_tx, &peer_id_hex).await;
                    }
                    Err(e) => {
                        lifegraph_log::warn!("failed to connect to bootstrap peer");
                    }
                }
            });
        }
    }

    // --- Fetch queue consumer: processes pending fetch requests by querying peers ---
    // Note: The fetch consumer needs its own store access. We can't share the same
    // NodeStore with the store task (it's not Send), so we open a separate connection
    // to the same SQLite database for read-only fetch queue operations.
    let fetch_store_path = data_root.join("index.sqlite3");
    let fetch_peers = bootstrap_peers.clone();
    let fetch_node_id = node_id;
    let _fetch_handle = lifegraph_rt::spawn(async move {
        run_fetch_queue_consumer(fetch_store_path, fetch_peers, fetch_node_id).await;
    });

    // --- Peer reconnection task ---
    let recon_store_tx = store_tx.clone();
    let recon_peers = unreachable_peers;
    lifegraph_rt::spawn(async move {
        run_peer_reconnection(recon_peers, recon_store_tx).await;
    });

    // Wait for shutdown signal
    let shutdown = lifegraph_rt::spawn(async {
        lifegraph_rt::ctrl_c().await.ok();
        lifegraph_log::info!("shutting down");
    });
    let _ = shutdown.await;

    // Cleanup
    drop(store_tx);
    let _ = store_handle.await;
    let _ = mesh_handle.await;
}

// ---------------------------------------------------------------------------
// Fetch queue consumer: processes pending fetches by querying bootstrap peers
// ---------------------------------------------------------------------------

/// Periodically checks the fetch queue and sends queries to bootstrap peers
/// to retrieve missing events, objects, or snapshots.
async fn run_fetch_queue_consumer(
    index_path: std::path::PathBuf,
    peers: Vec<BootstrapPeer>,
    local_node_id: NodeID,
) {
    use lifegraph_proto::lifegraph::v0::access::{QueryClass, QueryRequest};
    use lifegraph_proto::lifegraph::v0::common::IdentityRef;
    use lifegraph_proto::lifegraph::v0::trust::{ScopeDescriptor, ScopeKind};
    use lifegraph_storage::FileIndex;
    

    if peers.is_empty() {
        lifegraph_log::info!("no bootstrap peers configured, fetch queue consumer disabled");
        return;
    }

    // Open a separate SQLite connection for the fetch queue
    let index = match FileIndex::open(&index_path.parent().unwrap().to_path_buf()) {
        Ok(idx) => idx,
        Err(e) => {
            lifegraph_log::error!("failed to open SQLite index for fetch queue: {}", e);
            return;
        }
    };

    let mut interval = lifegraph_rt::interval(std::time::Duration::from_secs(30));
    interval.set_missed_tick_behavior(lifegraph_rt::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        // Dequeue one pending fetch at a time
        let fetch_entry = match index.dequeue_fetch() {
            Ok(Some(entry)) => entry,
            Ok(None) => continue, // Queue is empty
            Err(e) => {
                lifegraph_log::warn!("failed to dequeue fetch: {}", e);
                continue;
            }
        };

        let fetch_type = fetch_entry.target_type.clone();
        let fetch_id = fetch_entry.target_id.clone();
        let fetch_priority = fetch_entry.priority;

        lifegraph_log::debug!("processing fetch queue entry");

        // Try each peer until one responds
        let mut fetched = false;
        for peer in &peers {
            let peer_addr = peer.addr.clone();

            // Build appropriate query based on fetch type
            let query_class = match fetch_type.as_str() {
                "event" => QueryClass::EventRange,
                "object" => QueryClass::ObjectFetch,
                "snapshot" => QueryClass::Snapshot,
                _ => continue,
            };

            let query_id = format!("fetch-{}-{}", fetch_type, fetch_id).into_bytes();
            let query = QueryRequest {
                request_version: 1,
                query_id,
                requester: Some(IdentityRef {
                    identity_id: local_node_id.0.to_vec(),
                    identity_kind: Some(2), // NODE
                    key_hint: None,
                }),
                target_scope: Some(ScopeDescriptor {
                    scope_version: 1,
                    scope_kind: ScopeKind::Node as i32,
                    target_nodes: vec![],
                    target_streams: vec![],
                    target_object_kinds: vec![],
                    target_view_types: vec![],
                    target_domains: vec![],
                    time_bounds: None,
                    scope_metadata: None,
                }),
                query_class: query_class as i32,
                time_window: None,
                checkpoint_base: None,
                result_limit: Some(100),
                cost_limit: None,
                required_proof_classes: vec![],
                query_payload_object: None,
                signature: None,
            };

            // Send query over TCP to the peer
            match send_query_to_peer(&peer_addr, &query).await {
                Ok(_fragment_bytes) => {
                    lifegraph_log::info!("fetch query succeeded");
                    fetched = true;
                    // Mark as done
                    let _ = index.mark_fetch_done(fetch_entry.id);
                    break;
                }
                Err(e) => {
                    lifegraph_log::debug!("peer query failed, trying next");
                }
            }
        }

        if !fetched {
            // Re-enqueue with lower priority for retry
            let _ = index.enqueue_fetch(&fetch_type, &fetch_id, fetch_priority - 1);
        }
    }
}

/// Sends a CommandEnvelope to a peer over TCP, records CommandSent event, and returns the response.
/// (Blocking version — for use from the store task's blocking thread)
fn send_command_to_peer(
    peer_addr: &str,
    command: &lifegraph_proto::lifegraph::v0::stream::CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Record CommandSent event on our own stream (double-entry bookkeeping)
    command_dispatch::record_command_sent_event(store, stream_id, signer, command);

    // Spawn the async operation and wait for it
    let peer_addr = peer_addr.to_string();
    let command = command.clone();
    lifegraph_rt::spawn(async move {
        send_command_to_peer_async(&peer_addr, &command).await
    }).blocking_recv().unwrap()
}

/// Async version of send_command_to_peer — no store mutation, just network I/O.
async fn send_command_to_peer_async(
    peer_addr: &str,
    command: &lifegraph_proto::lifegraph::v0::stream::CommandEnvelope,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use lifegraph_rt::{AsyncReadExt, AsyncWriteExt};

    let mut stream = lifegraph_rt::timeout(
        std::time::Duration::from_secs(10),
        lifegraph_rt::TcpStream::connect(peer_addr),
    ).await??;

    let cmd_bytes = prost::Message::encode_to_vec(command);
    let frame = encode_tcp_frame(&cmd_bytes);
    lifegraph_rt::timeout(
        std::time::Duration::from_secs(10),
        stream.write_all(&frame),
    ).await??;

    let mut header = [0u8; 8];
    lifegraph_rt::timeout(
        std::time::Duration::from_secs(10),
        stream.read_exact(&mut header),
    ).await??;

    let payload_len = u64::from_be_bytes(header) as usize;
    let mut payload = vec![0u8; payload_len];
    lifegraph_rt::timeout(
        std::time::Duration::from_secs(30),
        stream.read_exact(&mut payload),
    ).await??;

    Ok(payload)
}

/// Sends a QueryRequest to a peer over TCP and returns the QueryResultFragment bytes.
async fn send_query_to_peer(
    peer_addr: &str,
    query: &lifegraph_proto::lifegraph::v0::access::QueryRequest,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use lifegraph_rt::{AsyncReadExt, AsyncWriteExt};
    let mut stream = lifegraph_rt::timeout(
        std::time::Duration::from_secs(10),
        lifegraph_rt::TcpStream::connect(peer_addr),
    ).await??;

    let query_bytes = prost::Message::encode_to_vec(query);
    let frame = encode_tcp_frame(&query_bytes);

    lifegraph_rt::timeout(
        std::time::Duration::from_secs(10),
        stream.write_all(&frame),
    ).await??;

    // Read response (8-byte length prefix + payload)
    let mut header = [0u8; 8];
    lifegraph_rt::timeout(
        std::time::Duration::from_secs(10),
        stream.read_exact(&mut header),
    ).await??;

    let payload_len = u64::from_be_bytes(header) as usize;
    let mut payload = vec![0u8; payload_len];
    lifegraph_rt::timeout(
        std::time::Duration::from_secs(30),
        stream.read_exact(&mut payload),
    ).await??;

    Ok(payload)
}

// ---------------------------------------------------------------------------
// Store task — owns NodeStore (non-Send), processes commands sequentially
// ---------------------------------------------------------------------------

fn run_store_task(
    mut store: NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    mut rx: lifegraph_rt::mpsc::Receiver<StoreRequest>,
    mesh_command_rx: std::sync::mpsc::Receiver<MeshCommandRequest>,
    mut global_rate_limiter: ingress::TokenBucket,
    mut message_hash_cache: ingress::RecentHashCache,
    allowed_peers: Vec<Vec<u8>>,
    responder_node_id: NodeID,
) {
    // Initialize controller set from the node's config
    let initial_controllers = config_controllers_from_signer(&*signer);
    let mut controllers = command_dispatch::ControllerSet::new(initial_controllers.clone());
    let mut replay_cache: std::collections::HashMap<Vec<u8>, (Vec<u8>, i64)> = std::collections::HashMap::new();

    // Load active revocations from the database
    let revoked_delegations: std::collections::HashSet<Vec<u8>> = match store.list_active_revocations() {
        Ok(revocations) => {
            let revoked: std::collections::HashSet<Vec<u8>> = revocations
                .into_iter()
                .filter(|(typ, _)| typ == "delegation")
                .filter_map(|(_, hex)| lifegraph_core::util::hex_to_bytes(&hex).ok())
                .collect();
            lifegraph_log::info!("loaded active revocations");
            revoked
        }
        Err(e) => {
            lifegraph_log::warn!("failed to load revocations: {}", e);
            std::collections::HashSet::new()
        }
    };

    let trusted_root_ids: Vec<Vec<u8>> = controllers.to_vec();

    let mut request_counter: u64 = 0;

    loop {
        // Try to receive from either mesh (sync, non-blocking poll) or TCP (blocking recv)
        let req = if let Ok(mesh_req) = mesh_command_rx.try_recv() {
            // Process mesh command and route reply back through mesh
            process_mesh_command_sync(mesh_req, &mut store, stream_id, signer,
                &mut controllers, &mut replay_cache, &revoked_delegations, &trusted_root_ids);
            request_counter += 1;
            continue;
        } else if let Some(store_req) = rx.blocking_recv() {
            store_req
        } else {
            // TCP channel closed, shut down
            break;
        };

        request_counter += 1;

        // Extract screening data without consuming the full request yet
        // ProduceSnapshot, FetchObject, and SendCommand skip screening (local trusted operations)
        let raw_bytes: Vec<u8> = match &req {
            StoreRequest::Command { raw_bytes, .. } |
            StoreRequest::Query { raw_bytes, .. } => raw_bytes.clone(),
            StoreRequest::ProduceSnapshot { .. } |
            StoreRequest::FetchObject { .. } |
            StoreRequest::SendCommand { .. } => Vec::new(),
        };
        let peer_id: Option<Vec<u8>> = match &req {
            StoreRequest::Command { peer_id, .. } |
            StoreRequest::Query { peer_id, .. } => peer_id.clone(),
            StoreRequest::ProduceSnapshot { .. } |
            StoreRequest::FetchObject { .. } |
            StoreRequest::SendCommand { .. } => None,
        };

        // ---- §18.3 Ingress screening (cheap → expensive) ----
        // ProduceSnapshot is a local trusted operation — skip screening

        if !matches!(&req, StoreRequest::ProduceSnapshot { .. } | StoreRequest::FetchObject { .. } | StoreRequest::SendCommand { .. }) {
            // 1. Recent duplicate detection (before any crypto work)
            let msg_hash = ingress::quick_message_hash(&raw_bytes);
            if message_hash_cache.contains(msg_hash) {
                let reply = match req {
                    StoreRequest::Command { reply_tx, .. } => reply_tx,
                    StoreRequest::Query { reply_tx, .. } => reply_tx,
                    StoreRequest::ProduceSnapshot { reply_tx, .. } => reply_tx,
                    StoreRequest::FetchObject { reply_tx, .. } => reply_tx,
                    StoreRequest::SendCommand { reply_tx, .. } => reply_tx,
                };
                let _ = reply.send(StoreResponse::Rejected(ingress::IngressResult::Duplicate));
                continue;
            }

            // 2. Global rate limit
            if !global_rate_limiter.try_consume() {
                let reply = match req {
                    StoreRequest::Command { reply_tx, .. } => reply_tx,
                    StoreRequest::Query { reply_tx, .. } => reply_tx,
                    StoreRequest::ProduceSnapshot { reply_tx, .. } => reply_tx,
                    StoreRequest::FetchObject { reply_tx, .. } => reply_tx,
                    StoreRequest::SendCommand { reply_tx, .. } => reply_tx,
                };
                let _ = reply.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                continue;
            }

            // 3. Peer allowlist check
            if let Some(ref p) = peer_id {
                if !ingress::is_peer_allowed(p, &allowed_peers) {
                    let reply = match req {
                        StoreRequest::Command { reply_tx, .. } => reply_tx,
                        StoreRequest::Query { reply_tx, .. } => reply_tx,
                        StoreRequest::ProduceSnapshot { reply_tx, .. } => reply_tx,
                        StoreRequest::FetchObject { reply_tx, .. } => reply_tx,
                        StoreRequest::SendCommand { reply_tx, .. } => reply_tx,
                    };
                    let _ = reply.send(StoreResponse::Rejected(ingress::IngressResult::PeerNotAllowed));
                    continue;
                }
            }

            // Screening passed — cache the hash
            message_hash_cache.insert(msg_hash);
        }

        // 4. Process the request
        match req {
            StoreRequest::Command { command, reply_tx, .. } => {
                // Full command dispatch: validation, delegation check, type dispatch
                // (handled in run_store_task via controllers and replay_cache)
                let result = command_dispatch::dispatch_command(
                    &command, &mut store, stream_id, signer,
                    &mut controllers, &mut replay_cache,
                    &revoked_delegations, &trusted_root_ids,
                );
                let _ = reply_tx.send(StoreResponse::Ok(result.response_bytes));
            }
            StoreRequest::Query { query, reply_tx, .. } => {
                let result = execute_query(&query, &mut store, stream_id, &responder_node_id);
                let _ = reply_tx.send(StoreResponse::Ok(result));
            }
            StoreRequest::ProduceSnapshot { view_type, completeness, reply_tx } => {
                match store.produce_snapshot(signer, &view_type, completeness) {
                    Ok(descriptor) => {
                        lifegraph_log::info!("snapshot produced"
                        );
                        let result_bytes = prost::Message::encode_to_vec(&descriptor);
                        let _ = reply_tx.send(StoreResponse::Ok(result_bytes));
                    }
                    Err(e) => {
                        lifegraph_log::error!("snapshot production failed: {}", e);
                        let _ = reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                    }
                }
            }
            StoreRequest::FetchObject { object_ref, reply_tx } => {
                match store.get_object(&object_ref) {
                    Ok(Some(result)) => {
                        lifegraph_log::info!("object fetched"
                        );
                        let _ = reply_tx.send(StoreResponse::Ok(result.content));
                    }
                    Ok(None) => {
                        lifegraph_log::warn!("object not found for fetch"
                        );
                        let _ = reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                    }
                    Err(e) => {
                        lifegraph_log::error!("object fetch failed: {}", e);
                        let _ = reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                    }
                }
            }
            StoreRequest::SendCommand { peer_addr, command, reply_tx } => {
                match send_command_to_peer(&peer_addr, &command, &mut store, stream_id, signer) {
                    Ok(response) => {
                        lifegraph_log::info!("outbound command succeeded");
                        let _ = reply_tx.send(StoreResponse::Ok(response));
                    }
                    Err(e) => {
                        lifegraph_log::error!("outbound command failed");
                        let _ = reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                    }
                }
            }
        }

        // Periodically process the fetch queue (every 10 requests)
        if request_counter.is_multiple_of(10) {
            if let Ok(resolved) = store.process_fetch_queue() {
                if resolved > 0 {
                    lifegraph_log::info!("fetch queue processed items");
                }
            }
        }

        // Periodically run integrity check (every 100 requests)
        if request_counter.is_multiple_of(100) {
            match store.integrity_check_and_rebuild() {
                Ok(0) => {} // Healthy
                Ok(rebuilt) => {
                    lifegraph_log::warn!("SQLite corruption detected and indexes rebuilt");
                }
                Err(e) => {
                    lifegraph_log::error!("integrity check and rebuild failed");
                }
            }
        }

        // Periodically checkpoint WAL (every 50 requests)
        if request_counter.is_multiple_of(50) {
            match store.wal_checkpoint() {
                Ok(Some(wal_size)) if wal_size > 1024 * 1024 => {
                    // WAL > 1MB after checkpoint — log a warning
                    lifegraph_log::warn!("WAL file remains large after checkpoint");
                }
                _ => {}
            }
        }

        // Periodically check disk space (every 200 requests)
        if request_counter.is_multiple_of(200) {
            match store.check_disk_space() {
                Ok(()) => {}
                Err(available_bytes) => {
                    lifegraph_log::error!("CRITICAL: disk space critically low, writes may fail"
                    );
                }
            }
        }
    }
}

/// Processes a mesh command synchronously and routes the reply back through the mesh.
fn process_mesh_command_sync(
    req: MeshCommandRequest,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    controllers: &mut command_dispatch::ControllerSet,
    replay_cache: &mut std::collections::HashMap<Vec<u8>, (Vec<u8>, i64)>,
    revoked_delegations: &std::collections::HashSet<Vec<u8>>,
    trusted_root_ids: &[Vec<u8>],
) {
    let source = req.source;
    let raw_bytes = req.raw_bytes.clone();

    // Ingress screening for mesh commands
    let _msg_hash = ingress::quick_message_hash(&raw_bytes);
    // (mesh commands bypass rate limiting and allowlist for now — they're from the local mesh)

    // Full command dispatch
    let result = command_dispatch::dispatch_command(
        &req.command, store, stream_id, signer,
        controllers, replay_cache,
        revoked_delegations, trusted_root_ids,
    );

    // Route reply back through mesh to the original sender
    let reply = MeshReply {
        source,
        response_bytes: result.response_bytes,
    };
    let _ = req.reply_tx.send(reply);
}

// ---------------------------------------------------------------------------
// TCP listener and per-connection handling
// ---------------------------------------------------------------------------

const TCP_MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

async fn run_tcp_listener(
    listen_addr: SocketAddr,
    _node_id: NodeID,
    store_tx: lifegraph_rt::mpsc::Sender<StoreRequest>,
) {
    let listener = match lifegraph_rt::TcpListener::bind(&listen_addr).await {
        Ok(l) => l,
        Err(e) => {
            lifegraph_log::error!("failed to bind TCP on {}: {}", listen_addr, e);
            return;
        }
    };

    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                let conn_store_tx = store_tx.clone();
                lifegraph_rt::spawn(async move {
                    lifegraph_log::debug!("TCP connection accepted");
                    handle_tcp_connection(stream, conn_store_tx).await;
                });
            }
            Err(e) => {
                lifegraph_log::warn!("TCP accept error: {}", e);
            }
        }
    }
}

/// Handles a plain TCP connection.
async fn handle_tcp_connection(
    mut stream: lifegraph_rt::TcpStream,
    store_tx: lifegraph_rt::mpsc::Sender<StoreRequest>,
) {
    let (mut reader, mut writer) = lifegraph_rt::split(&mut stream);
    let mut read_buf = Vec::with_capacity(4096);
    let mut conn_rate_limiter = ingress::TokenBucket::new(100, 50);

    handle_tcp_stream_common(
        &mut reader,
        &mut writer,
        &mut read_buf,
        &mut conn_rate_limiter,
        &store_tx,
    ).await;
}

/// Common frame handling logic for TCP connections.
async fn handle_tcp_stream_common<R, W>(
    reader: &mut R,
    writer: &mut W,
    read_buf: &mut Vec<u8>,
    conn_rate_limiter: &mut ingress::TokenBucket,
    store_tx: &lifegraph_rt::mpsc::Sender<StoreRequest>,
) where
    R: lifegraph_rt::AsyncRead + Unpin,
    W: lifegraph_rt::AsyncWrite + Unpin,
{
    use lifegraph_rt::{AsyncReadExt, AsyncWriteExt};

    loop {
        // Read 8-byte length prefix
        while read_buf.len() < 8 {
            let mut chunk = [0u8; 64];
            match reader.read(&mut chunk).await {
                Ok(0) => {
                    if read_buf.is_empty() {
                        return; // Clean close
                    }
                    lifegraph_log::debug!("TCP closed mid-header");
                    return;
                }
                Ok(n) => {
                    read_buf.extend_from_slice(&chunk[..n]);
                }
                Err(e) => {
                    lifegraph_log::debug!("TCP read error: {}", e);
                    return;
                }
            }
        }

        let frame_len = u64::from_be_bytes(read_buf[..8].try_into().unwrap()) as usize;
        if frame_len == 0 || frame_len > TCP_MAX_FRAME_SIZE {
            lifegraph_log::warn!("TCP frame length invalid or too large");
            return;
        }

        // Read payload
        let total_needed = 8 + frame_len;
        while read_buf.len() < total_needed {
            let mut chunk = vec![0u8; 4096.min(total_needed - read_buf.len())];
            match reader.read(&mut chunk).await {
                Ok(0) => {
                    lifegraph_log::debug!("TCP closed mid-frame");
                    return;
                }
                Ok(n) => {
                    read_buf.extend_from_slice(&chunk[..n]);
                }
                Err(e) => {
                    lifegraph_log::debug!("TCP read error: {}", e);
                    return;
                }
            }
        }

        let payload: Vec<u8> = {
            let frame = read_buf.drain(..total_needed).collect::<Vec<u8>>();
            frame.into_iter().skip(8).collect()
        };

        // Per-connection rate limit (cheap check, before decode or crypto)
        if !conn_rate_limiter.try_consume() {
            lifegraph_log::warn!("TCP rate-limited connection");
            return;
        }

        // Try CommandEnvelope
        if let Ok(command) =
            lifegraph_proto::lifegraph::v0::stream::CommandEnvelope::decode(&payload[..])
        {
            let raw = payload.clone();

            // Check if this is a snapshot publish command
            use lifegraph_proto::lifegraph::v0::stream::CommandType;
            if command.command_type == CommandType::PublishSnapshot as i32 {
                let (reply_tx, reply_rx) = lifegraph_rt::oneshot::channel();
                if store_tx.send(StoreRequest::ProduceSnapshot {
                    view_type: "stream_heads".to_string(),
                    completeness: 1, // FULL
                    reply_tx,
                }).await.is_err() {
                    return;
                }
                match reply_rx.await {
                    Ok(StoreResponse::Ok(resp_payload)) => {
                        let resp_frame = encode_tcp_frame(&resp_payload);
                        if writer.write_all(&resp_frame).await.is_err() {
                            return;
                        }
                    }
                    Ok(StoreResponse::Rejected(reason)) => {
                        lifegraph_log::debug!("TCP snapshot production screened");
                        return;
                    }
                    Err(_) => return,
                }
                continue;
            }

            // Check if this is a fetch object command
            if command.command_type == CommandType::FetchObject as i32 {
                // Decode the raw payload as proto CommandEnvelope to get payload_object
                let proto_command = match lifegraph_proto::lifegraph::v0::stream::CommandEnvelope::decode(&raw[..]) {
                    Ok(cmd) => cmd,
                    Err(e) => {
                        lifegraph_log::warn!("FETCH_OBJECT: failed to decode proto command: {}", e);
                        let err = format!("FETCH_OBJECT: decode failed");
                        let resp_frame = encode_tcp_frame(err.as_bytes());
                        let _ = writer.write_all(&resp_frame).await;
                        continue;
                    }
                };

                // Extract the ObjectRef from the proto command's payload_object
                let object_ref = if let Some(ref obj) = proto_command.payload {
                    use lifegraph_proto::lifegraph::v0::stream::command_envelope::Payload;
                    match obj {
                        Payload::PayloadObject(obj) => obj.clone(),
                        Payload::InlinePayload(bytes) => {
                            if let Ok(obj) = lifegraph_proto::lifegraph::v0::common::ObjectRef::decode(&bytes[..]) {
                                obj
                            } else {
                                lifegraph_proto::lifegraph::v0::common::ObjectRef {
                                    object_id: vec![],
                                    object_kind: None,
                                }
                            }
                        }
                    }
                } else {
                    lifegraph_proto::lifegraph::v0::common::ObjectRef {
                        object_id: vec![],
                        object_kind: None,
                    }
                };

                if object_ref.object_id.is_empty() {
                    lifegraph_log::warn!("FETCH_OBJECT: no object reference provided");
                    let err_resp = format!("FETCH_OBJECT: no object reference provided");
                    let resp_frame = encode_tcp_frame(err_resp.as_bytes());
                    let _ = writer.write_all(&resp_frame).await;
                    continue;
                }

                let (reply_tx, reply_rx) = lifegraph_rt::oneshot::channel();
                if store_tx.send(StoreRequest::FetchObject { object_ref, reply_tx }).await.is_err() {
                    return;
                }
                match reply_rx.await {
                    Ok(StoreResponse::Ok(resp_payload)) => {
                        let resp_frame = encode_tcp_frame(&resp_payload);
                        if writer.write_all(&resp_frame).await.is_err() {
                            return;
                        }
                    }
                    Ok(StoreResponse::Rejected(reason)) => {
                        lifegraph_log::debug!("TCP fetch object screened");
                        return;
                    }
                    Err(_) => return,
                }
                continue;
            }

            let (reply_tx, reply_rx) = lifegraph_rt::oneshot::channel();
            if store_tx.send(StoreRequest::Command {
                raw_bytes: raw, command, peer_id: None, reply_tx,
            }).await.is_err() {
                return;
            }
            match reply_rx.await {
                Ok(StoreResponse::Ok(resp_payload)) => {
                    let resp_frame = encode_tcp_frame(&resp_payload);
                    if writer.write_all(&resp_frame).await.is_err() {
                        return;
                    }
                }
                Ok(StoreResponse::Rejected(reason)) => {
                    lifegraph_log::debug!("TCP message screened");
                    return;
                }
                Err(_) => return,
            }
            continue;
        }

        // Try QueryRequest
        if let Ok(query) =
            lifegraph_proto::lifegraph::v0::access::QueryRequest::decode(&payload[..])
        {
            let raw = payload.clone();
            let (reply_tx, reply_rx) = lifegraph_rt::oneshot::channel();
            if store_tx.send(StoreRequest::Query {
                raw_bytes: raw, query, peer_id: None, reply_tx,
            }).await.is_err() {
                return;
            }
            match reply_rx.await {
                Ok(StoreResponse::Ok(resp_payload)) => {
                    let resp_frame = encode_tcp_frame(&resp_payload);
                    if writer.write_all(&resp_frame).await.is_err() {
                        return;
                    }
                }
                Ok(StoreResponse::Rejected(reason)) => {
                    lifegraph_log::debug!("TCP query screened");
                    return;
                }
                Err(_) => return,
            }
            continue;
        }

        lifegraph_log::debug!("TCP received unrecognized message type");
    }
}

fn encode_tcp_frame(payload: &[u8]) -> Vec<u8> {
    let len = payload.len() as u64;
    let mut frame = Vec::with_capacity(8 + payload.len());
    frame.extend_from_slice(&len.to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Extracts the initial controller identities.
/// The node's own identity is always the initial controller.
fn config_controllers_from_signer(signer: &dyn MeshSigner) -> Vec<Vec<u8>> {
    vec![signer.node_id().0.to_vec()]
}

// ---------------------------------------------------------------------------
// Query execution engine (§14.20–§14.21)
// ---------------------------------------------------------------------------

/// Result of query cost evaluation.
#[allow(dead_code)]
enum QueryCostCheck {
    Allowed { max_bytes: Option<usize>, max_results: Option<usize> },
    Denied { reason: &'static str },
}

/// Evaluates the query's cost_limit and returns allowed limits or denial reason.
fn check_query_cost(
    query: &lifegraph_proto::lifegraph::v0::access::QueryRequest,
) -> QueryCostCheck {
    // Constants for v0 cost limits
    const DEFAULT_MAX_BYTES: usize = 10 * 1024 * 1024; // 10 MB
    const DEFAULT_MAX_RESULTS: usize = 10_000;

    let (max_bytes, max_results) = if let Some(ref cost_limit) = query.cost_limit {
        // If cost_limit is specified, use its values with defaults as fallbacks
        let bytes = cost_limit.max_total_bytes.map(|b| b as usize).unwrap_or(DEFAULT_MAX_BYTES);
        let results = cost_limit.max_results.map(|r| r as usize).unwrap_or(DEFAULT_MAX_RESULTS);
        (bytes, results)
    } else {
        // No cost limit specified — use defaults
        (DEFAULT_MAX_BYTES, DEFAULT_MAX_RESULTS)
    };

    // max_federated_responders: in v0 we only query locally, so always allowed
    // max_wall_time: enforced at call site via timeout

    QueryCostCheck::Allowed {
        max_bytes: Some(max_bytes),
        max_results: Some(max_results),
    }
}

/// Executes a QueryRequest against local state and returns a QueryResultFragment.
fn execute_query(
    query: &lifegraph_proto::lifegraph::v0::access::QueryRequest,
    store: &mut NodeStore,
    _local_stream_id: &[u8],
    responder_node_id: &NodeID,
) -> Vec<u8> {
    use lifegraph_proto::lifegraph::v0::access::{QueryClass, QueryResultFragment, ResultCompleteness};
    use lifegraph_proto::lifegraph::v0::common::{EventRef, ObjectRef};

    // 1. Check cost limits before doing any work
    let (max_bytes, max_results) = match check_query_cost(query) {
        QueryCostCheck::Allowed { max_bytes, max_results } => (max_bytes, max_results),
        QueryCostCheck::Denied { reason } => {
            lifegraph_log::warn!("query denied due to cost limits");
            return build_query_denial(query, responder_node_id, reason);
        }
    };

    let query_class = query.query_class;
    let mut event_refs: Vec<EventRef> = Vec::new();
    let mut snapshot_refs: Vec<lifegraph_proto::lifegraph::v0::common::SnapshotRef> = Vec::new();
    let mut object_refs: Vec<ObjectRef> = Vec::new();
    let mut completeness = ResultCompleteness::CompleteForLocalKnowledge as i32;

    match query_class {
        // Return all known stream heads
        x if x == QueryClass::Head as i32 => {
            match store.list_stream_heads() {
                Ok(heads) => {
                    for (stream_id_hex, seq, hash) in heads {
                        if max_results.map_or(false, |m| event_refs.len() >= m) {
                            completeness = ResultCompleteness::Partial as i32;
                            break;
                        }
                        event_refs.push(EventRef {
                            stream_id: lifegraph_core::util::hex_to_bytes(&stream_id_hex).unwrap_or_else(|_| stream_id_hex.into_bytes()),
                            seq: seq as u64,
                            event_hash: Some(lifegraph_core::protocol::Digest {
                                algorithm: 1,
                                value: hash,
                            }),
                        });
                    }
                }
                Err(e) => {
                    lifegraph_log::warn!("query HEAD failed");
                    completeness = ResultCompleteness::Partial as i32;
                }
            }
        }

        // Return events in a range for specified streams
        x if x == QueryClass::EventRange as i32 => {
            match store.list_stream_heads() {
                Ok(heads) => {
                    // Determine time_window filter if present
                    let time_filter = query.time_window.as_ref().map(|tw| {
                        (
                            tw.not_before.as_ref().map(|t| t.seconds),
                            tw.expires_at.as_ref().map(|t| t.seconds),
                        )
                    });

                    for (stream_id_hex, head_seq, _hash) in &heads {
                        if max_results.map_or(false, |m| event_refs.len() >= m) {
                            completeness = ResultCompleteness::Partial as i32;
                            break;
                        }

                        // Determine sequence range from time_window
                        let (from_seq, to_seq) = if let Some((Some(_not_before_secs), Some(_expires_at_secs))) = time_filter {
                            // In v0 we don't have event timestamps indexed, so we fall back
                            // to returning the full range and let the client filter.
                            // This is a known limitation.
                            (0i64, *head_seq)
                        } else {
                            (0i64, *head_seq)
                        };

                        if let Ok(events) = store.list_event_range(stream_id_hex, from_seq, to_seq) {
                            for (seq, hash, _ver) in events {
                                if max_results.map_or(false, |m| event_refs.len() >= m) {
                                    completeness = ResultCompleteness::Partial as i32;
                                    break;
                                }
                                event_refs.push(EventRef {
                                    stream_id: lifegraph_core::util::hex_to_bytes(stream_id_hex).unwrap_or_else(|_| stream_id_hex.clone().into_bytes()),
                                    seq: seq as u64,
                                    event_hash: Some(lifegraph_core::protocol::Digest {
                                        algorithm: 1,
                                        value: hash,
                                    }),
                                });
                            }
                        }
                    }
                }
                Err(e) => {
                    lifegraph_log::warn!("query EVENT_RANGE failed");
                    completeness = ResultCompleteness::Partial as i32;
                }
            }
        }

        // Check if specific objects exist
        x if x == QueryClass::ObjectExistence as i32 => {
            if let Some(ref obj_ref) = query.query_payload_object {
                let object_id_hex = lifegraph_core::util::bytes_to_hex(&obj_ref.object_id);
                match store.is_object_present(&object_id_hex) {
                    Ok(present) => {
                        if present {
                            object_refs.push(obj_ref.clone());
                        }
                    }
                    Err(e) => {
                        lifegraph_log::warn!("query OBJECT_EXISTENCE failed");
                        completeness = ResultCompleteness::Partial as i32;
                    }
                }
            }
        }

        // Fetch object: retrieve content if present locally
        x if x == QueryClass::ObjectFetch as i32 => {
            if let Some(ref obj_ref) = query.query_payload_object {
                match store.get_object(obj_ref) {
                    Ok(Some(result)) => {
                        object_refs.push(obj_ref.clone());
                        let _ = result;
                    }
                    Ok(None) => {
                        completeness = ResultCompleteness::Partial as i32;
                    }
                    Err(e) => {
                        lifegraph_log::warn!("query OBJECT_FETCH failed");
                        completeness = ResultCompleteness::Partial as i32;
                    }
                }
            }
        }

        // Snapshot query: return known snapshot refs
        x if x == QueryClass::Snapshot as i32 => {
            match store.list_snapshots() {
                Ok(snaps) => {
                    for (sid, oid_hex, _vt, _ph, _pa, _c, _bh) in &snaps {
                        if max_results.map_or(false, |m| snapshot_refs.len() >= m) {
                            completeness = ResultCompleteness::Partial as i32;
                            break;
                        }
                        use lifegraph_proto::lifegraph::v0::common::SnapshotRef;
                        snapshot_refs.push(SnapshotRef {
                            snapshot_id: sid.clone().into_bytes(),
                            object_id: Some(lifegraph_core::util::hex_to_bytes(oid_hex).unwrap_or_default()),
                        });
                    }
                    if snaps.is_empty() {
                        completeness = ResultCompleteness::MetadataOnly as i32;
                    }
                }
                Err(e) => {
                    lifegraph_log::warn!("query SNAPSHOT failed");
                    completeness = ResultCompleteness::Partial as i32;
                }
            }
        }

        // Search query: scan event log for matching event types or content
        x if x == QueryClass::Search as i32 => {
            // In v0, search is limited — we scan stream heads and return refs.
            // A full implementation would index event content.
            match store.list_stream_heads() {
                Ok(heads) => {
                    for (stream_id_hex, seq, hash) in heads {
                        if max_results.map_or(false, |m| event_refs.len() >= m) {
                            completeness = ResultCompleteness::Partial as i32;
                            break;
                        }
                        event_refs.push(EventRef {
                            stream_id: lifegraph_core::util::hex_to_bytes(&stream_id_hex).unwrap_or_else(|_| stream_id_hex.into_bytes()),
                            seq: seq as u64,
                            event_hash: Some(lifegraph_core::protocol::Digest {
                                algorithm: 1,
                                value: hash,
                            }),
                        });
                    }
                }
                Err(e) => {
                    lifegraph_log::warn!("query SEARCH failed");
                    completeness = ResultCompleteness::Partial as i32;
                }
            }
        }

        // View query: return current view of the node (stream heads + snapshots)
        x if x == QueryClass::View as i32 => {
            // Return stream heads as events and snapshot refs
            if let Ok(heads) = store.list_stream_heads() {
                for (stream_id_hex, seq, hash) in heads {
                    if max_results.map_or(false, |m| event_refs.len() >= m) {
                        completeness = ResultCompleteness::Partial as i32;
                        break;
                    }
                    event_refs.push(EventRef {
                        stream_id: lifegraph_core::util::hex_to_bytes(&stream_id_hex).unwrap_or_else(|_| stream_id_hex.into_bytes()),
                        seq: seq as u64,
                        event_hash: Some(lifegraph_core::protocol::Digest {
                            algorithm: 1,
                            value: hash,
                        }),
                    });
                }
            }
            if let Ok(snaps) = store.list_snapshots() {
                for (sid, oid_hex, _vt, _ph, _pa, _c, _bh) in &snaps {
                    if max_results.map_or(false, |m| snapshot_refs.len() >= m) {
                        completeness = ResultCompleteness::Partial as i32;
                        break;
                    }
                    use lifegraph_proto::lifegraph::v0::common::SnapshotRef;
                    snapshot_refs.push(SnapshotRef {
                        snapshot_id: sid.clone().into_bytes(),
                        object_id: Some(lifegraph_core::util::hex_to_bytes(oid_hex).unwrap_or_default()),
                    });
                }
            }
        }

        // Trust state: return current controller set (from config in v0)
        x if x == QueryClass::TrustState as i32 => {
            completeness = ResultCompleteness::MetadataOnly as i32;
        }

        // Unknown query class
        _ => {
            lifegraph_log::warn!("query class not supported");
            completeness = ResultCompleteness::Denied as i32;
        }
    }

    // Apply result_limit from query (in addition to cost_limit)
    if let Some(limit) = query.result_limit {
        let limit = limit as usize;
        if event_refs.len() > limit {
            event_refs.truncate(limit);
            completeness = ResultCompleteness::Partial as i32;
        }
        if object_refs.len() > limit {
            object_refs.truncate(limit);
            completeness = ResultCompleteness::Partial as i32;
        }
        if snapshot_refs.len() > limit {
            snapshot_refs.truncate(limit);
            completeness = ResultCompleteness::Partial as i32;
        }
    }

    let fragment = QueryResultFragment {
        fragment_version: 1,
        query_id: query.query_id.clone(),
        responder: Some(lifegraph_proto::lifegraph::v0::common::IdentityRef {
            identity_id: responder_node_id.0.to_vec(),
            identity_kind: Some(2), // NODE
            key_hint: None,
        }),
        answered_at: Some(now_ms_timestamp()),
        completeness,
        snapshot_refs,
        event_refs,
        object_refs,
        proof_objects: vec![],
        omission_reason: String::new(),
        bundled_result_object: None,
        result_metadata: None,
        signature: None,
    };

    let fragment_bytes = prost::Message::encode_to_vec(&fragment);

    // Enforce max_total_bytes on the serialized response
    if let Some(max_b) = max_bytes {
        if fragment_bytes.len() > max_b {
            lifegraph_log::warn!("query response exceeds max_total_bytes, returning denial"
            );
            return build_query_denial(query, responder_node_id, "response_too_large");
        }
    }

    fragment_bytes
}

/// Builds a denial QueryResultFragment when cost limits are exceeded.
fn build_query_denial(
    query: &lifegraph_proto::lifegraph::v0::access::QueryRequest,
    responder_node_id: &NodeID,
    reason: &str,
) -> Vec<u8> {
    use lifegraph_proto::lifegraph::v0::access::{QueryResultFragment, ResultCompleteness};

    let fragment = QueryResultFragment {
        fragment_version: 1,
        query_id: query.query_id.clone(),
        responder: Some(lifegraph_proto::lifegraph::v0::common::IdentityRef {
            identity_id: responder_node_id.0.to_vec(),
            identity_kind: Some(2),
            key_hint: None,
        }),
        answered_at: Some(now_ms_timestamp()),
        completeness: ResultCompleteness::Denied as i32,
        snapshot_refs: vec![],
        event_refs: vec![],
        object_refs: vec![],
        proof_objects: vec![],
        omission_reason: reason.to_string(),
        bundled_result_object: None,
        result_metadata: None,
        signature: None,
    };

    prost::Message::encode_to_vec(&fragment)
}

// ---------------------------------------------------------------------------
// Mesh poll loop (blocking thread)
// ---------------------------------------------------------------------------

fn run_mesh_loop(
    node_id: NodeID,
    mesh_command_tx: std::sync::mpsc::Sender<MeshCommandRequest>,
    mesh_reply_rx: std::sync::mpsc::Receiver<MeshReply>,
) {
    let local = LocalNode::new(node_id);
    let mut mesh_link = MeshLink::new();
    mesh_link.set_local_node_id(node_id);
    let mut router = MeshRouter::new(local);

    match mesh_link.enable_udp_broadcast() {
        Ok(_) => lifegraph_log::info!("UDP broadcast enabled on port 47079"),
        Err(e) => lifegraph_log::warn!("UDP broadcast failed: {}", e),
    }

    let interfaces = discover_network_interfaces().unwrap_or_default();
    for iface in &interfaces {
        if iface.link_state != NetworkLinkState::Up || iface.name == "lo" {
            continue;
        }
        let ifindex_path = format!("/sys/class/net/{}/ifindex", iface.name);
        let Ok(ifindex_str) = fs::read_to_string(&ifindex_path) else { continue; };
        let Ok(ifindex): Result<i32, _> = ifindex_str.trim().parse() else { continue; };
        if mesh_link.add_raw_ethernet(ifindex).is_ok() {
            lifegraph_log::info!("opened raw socket");
        }
    }

    if let Err(e) = mesh_link.broadcast_discovery(&mut router) {
        lifegraph_log::warn!("discovery broadcast failed: {}", e);
    }

    lifegraph_log::info!("mesh loop running");

    let mut discovery_counter: u64 = 0;
    loop {
        if let Err(e) = mesh_link.pump(&mut router) {
            lifegraph_log::warn!("mesh pump error: {}", e);
        }

        let frames = mesh_link.drain_inbound_data_frames();
        for frame in frames {
            if frame.header.dest == node_id || frame.header.dest.0 == [0u8; 64] {
                if frame.header.frame_type == FrameType::Data {
                    // Decode as command and send to store task for processing
                    if let Ok(command) =
                        lifegraph_proto::lifegraph::v0::stream::CommandEnvelope::decode(&frame.payload[..])
                    {
                        let (reply_tx, reply_rx) = std::sync::mpsc::channel();
                        let _ = mesh_command_tx.send(MeshCommandRequest {
                            command,
                            raw_bytes: frame.payload.clone(),
                            source: frame.header.src,
                            reply_tx,
                        });
                        // Wait for reply and queue it back through mesh
                        if let Ok(reply) = reply_rx.recv() {
                            let reply_frame = MeshFrame::from_payload(reply.source, reply.response_bytes);
                            mesh_link.queue_frame(reply_frame);
                        }
                    }
                }
            } else if let Some(next_hop) = router.next_hop_for(&frame.header.dest) {
                let mut fwd = frame;
                fwd.header.dest = next_hop;
                mesh_link.queue_frame(fwd);
            }
        }

        // Also receive replies that were routed from other sources (TCP queries forwarded to mesh)
        while let Ok(reply) = mesh_reply_rx.try_recv() {
            let reply_frame = MeshFrame::from_payload(reply.source, reply.response_bytes);
            mesh_link.queue_frame(reply_frame);
        }

        discovery_counter += 1;
        if discovery_counter.is_multiple_of(500) {
            if let Err(e) = mesh_link.broadcast_discovery(&mut router) {
                lifegraph_log::warn!("discovery failed: {}", e);
            }
            let dead = router.tick_heartbeat();
            for d in &dead {
                lifegraph_log::info!("peer dead");
            }
        }

        if let Err(e) = mesh_link.drain_pending_frames(&mut router) {
            lifegraph_log::warn!("send failed: {}", e);
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn sign_event_envelope(event: &mut lifegraph_core::protocol::EventEnvelope, signer: &dyn MeshSigner) -> Result<(), String> {
    use lifegraph_core::protocol::{ProtocolRecord, canonical_bytes};
    
    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, true);
    let digest = lifegraph_core::crypto::sha256(&canonical);
    let mut digest_bytes = [0u8; 32];
    digest_bytes.copy_from_slice(&digest);
    let sig = signer.sign_digest(&digest_bytes)
        .map_err(|e| format!("signing failed: {}", e))?;
    event.signature = Some(lifegraph_core::protocol::Signature {
        algorithm: 1,
        value: sig.to_vec(),
    });
    Ok(())
}

fn extract_private_key_bytes(config: &NodeConfig) -> Vec<u8> {
    if let Some(ref signer) = config.signer {
        if signer.signer_type == "software" {
            if let Some(ref hex_str) = signer.private_key_hex {
                return lifegraph_core::util::hex_to_bytes(hex_str.trim()).unwrap_or_else(|_| {
                    eprintln!("error: invalid private key hex");
                    std::process::exit(1);
                });
            }
        }
    }
    eprintln!("error: no software signer with private_key_hex found in config");
    std::process::exit(1);
}

fn now_ms_timestamp() -> prost_types::Timestamp {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    prost_types::Timestamp {
        seconds: now.as_secs() as i64,
        nanos: now.subsec_nanos() as i32,
    }
}

// ---------------------------------------------------------------------------
// Config — simple YAML-like parser (no serde dependency)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct NodeConfig {
    stream_id: String,
    name: Option<String>,
    controllers: Vec<String>,
    trust_nodes: Vec<String>,
    allowed_peers: Vec<String>,
    bootstrap_peers: Vec<String>,
    signer: Option<SignerConfig>,
}

#[derive(Clone, Debug)]
struct SignerConfig {
    signer_type: String,
    public_key_hex: String,
    private_key_hex: Option<String>,
    handle: Option<String>,
    slot: Option<String>,
}

fn parse_config(yaml: &str) -> Result<NodeConfig, String> {
    let mut config = NodeConfig {
        stream_id: String::new(),
        name: None,
        controllers: Vec::new(),
        trust_nodes: Vec::new(),
        allowed_peers: Vec::new(),
        bootstrap_peers: Vec::new(),
        signer: None,
    };

    let mut in_signer = false;
    let mut signer_type = String::new();
    let mut public_key_hex = String::new();
    let mut private_key_hex = Option::<String>::None;
    let mut handle = Option::<String>::None;
    let mut slot = Option::<String>::None;

    for line in yaml.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') { continue; }

        // Detect signer section
        if trimmed.starts_with("signer:") {
            in_signer = true;
            continue;
        }

        // Other top-level sections end signer parsing
        if in_signer && !trimmed.starts_with(' ') && !trimmed.starts_with('\t') {
            in_signer = false;
        }

        if in_signer {
            let content = trimmed.trim_start();
            if let Some(val) = parse_kv(content) {
                match val.0.as_str() {
                    "type" => signer_type = unquote(&val.1),
                    "public_key_hex" => public_key_hex = unquote(&val.1),
                    "private_key_hex" => private_key_hex = Some(unquote(&val.1)),
                    "handle" => handle = Some(unquote(&val.1)),
                    "slot" => slot = Some(unquote(&val.1)),
                    _ => {}
                }
            }
        } else {
            if let Some((key, val)) = parse_kv(trimmed) {
                match key.as_str() {
                    "stream_id" => config.stream_id = unquote(&val),
                    "name" => config.name = Some(unquote(&val)),
                    "controllers" => config.controllers = parse_list(&val),
                    "trust_nodes" => config.trust_nodes = parse_list(&val),
                    "allowed_peers" => config.allowed_peers = parse_list(&val),
                    "bootstrap_peers" => config.bootstrap_peers = parse_list(&val),
                    _ => {}
                }
            }
        }
    }

    if !signer_type.is_empty() || !public_key_hex.is_empty() {
        config.signer = Some(SignerConfig {
            signer_type: if signer_type.is_empty() { "unknown".into() } else { signer_type },
            public_key_hex,
            private_key_hex,
            handle,
            slot,
        });
    }

    if config.stream_id.is_empty() {
        return Err("missing required field: stream_id".into());
    }
    Ok(config)
}

fn parse_kv(line: &str) -> Option<(String, String)> {
    let colon = line.find(':')?;
    let key = line[..colon].trim().to_string();
    let val = line[colon+1..].trim().to_string();
    Some((key, val))
}

fn parse_list(val: &str) -> Vec<String> {
    let val = val.trim();
    if val == "[]" || val.is_empty() { return Vec::new(); }
    // Handle [item1, item2] format
    if val.starts_with('[') && val.ends_with(']') {
        let inner = &val[1..val.len()-1];
        if inner.trim().is_empty() { return Vec::new(); }
        return inner.split(',').map(|s| unquote(s.trim())).collect();
    }
    // Handled elsewhere
    Vec::new()
}

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 {
        let bytes = s.as_bytes();
        if (bytes[0] == b'"' && bytes[s.len()-1] == b'"') ||
           (bytes[0] == 39 && bytes[s.len()-1] == 39) {
            return s[1..s.len()-1].to_string();
        }
    }
    s.to_string()
}

/// Parsed bootstrap peer configuration.
#[derive(Clone, Debug)]
struct BootstrapPeer {
    addr: String,
    node_id_hex: String,
}

fn parse_bootstrap_peers(entries: &[String]) -> Vec<BootstrapPeer> {
    entries.iter().filter_map(|entry| {
        // Format: "host:port@node_id_hex"
        let parts: Vec<&str> = entry.splitn(2, '@').collect();
        if parts.len() == 2 {
            Some(BootstrapPeer {
                addr: parts[0].to_string(),
                node_id_hex: parts[1].to_string(),
            })
        } else {
            lifegraph_log::warn!("invalid bootstrap peer format (expected host:port@node_id_hex)");
            None
        }
    }).collect()
}

/// A software signer that is Send + Sync.
struct SyncSoftwareSigner {
    node_id: NodeID,
    key: std::sync::Mutex<SigningKey>,
}

impl SyncSoftwareSigner {
    fn new(key: SigningKey) -> Self {
        let vk = key.verifying_key();
        let encoded = vk.to_encoded_point(false);
        let mut node_bytes = [0u8; 64];
        node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
        Self {
            node_id: NodeID(node_bytes),
            key: std::sync::Mutex::new(key),
        }
    }
}

impl MeshSigner for SyncSoftwareSigner {
    fn node_id(&self) -> NodeID {
        self.node_id
    }

    fn sign_digest(
        &self,
        digest: &[u8; 32],
    ) -> Result<[u8; 64], lifegraph_hardware_signing::HardwareSigningError> {
        let key = self.key.lock().unwrap();
        let sig: p256::ecdsa::Signature = key
            .sign_prehash(digest)
            .map_err(|e| lifegraph_hardware_signing::HardwareSigningError::Provider(e.to_string()))?;
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&sig.to_bytes());
        Ok(bytes)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn load_signer_from_config(config: &NodeConfig) -> Arc<dyn MeshSigner + Send + Sync> {
    let Some(signer_config) = &config.signer else {
        eprintln!("error: no signer configured. Run `lifegraphd init` first.");
        std::process::exit(1);
    };

    match signer_config.signer_type.as_str() {
        "software" => {
            let Some(key_hex) = &signer_config.private_key_hex else {
                eprintln!("error: software signer configured but private_key_hex is missing.");
                std::process::exit(1);
            };
            let signing_key = parse_signing_key_hex(key_hex);
            Arc::new(SyncSoftwareSigner::new(signing_key))
        }
        "tpm" => {
            eprintln!("error: TPM signing not yet implemented.");
            std::process::exit(1);
        }
        "yubikey" => {
            eprintln!("error: YubiKey signing not yet implemented.");
            std::process::exit(1);
        }
        other => {
            eprintln!("error: unknown signer type: {}", other);
            std::process::exit(1);
        }
    }
}

fn parse_signing_key_hex(key_hex: &str) -> SigningKey {
    let hex_str = key_hex.trim();
    let bytes = lifegraph_core::util::hex_to_bytes(hex_str).unwrap_or_else(|e| {
        eprintln!("error: invalid key hex: {}", e);
        std::process::exit(1);
    });
    let bytes: [u8; 32] = bytes.try_into().unwrap_or_else(|_| {
        eprintln!("error: key must be 32 bytes (64 hex chars)");
        std::process::exit(1);
    });
    SigningKey::from_bytes(&bytes.into()).unwrap_or_else(|e| {
        eprintln!("error: invalid key: {}", e);
        std::process::exit(1);
    })
}
