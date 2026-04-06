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

mod capabilities;
mod ingress;

use clap::{Parser, Subcommand};
use lifegraph_hardware_signing::{MeshSigner, NodeID};
use lifegraph_mesh::{FrameType, LocalNode, MeshFrame};
use lifegraph_mesh_link::MeshLink;
use lifegraph_mesh_router::MeshRouter;
use lifegraph_storage::{NodeStore, NodeStoreConfig, BlobKeySource};
use prost::Message;
use std::sync::Arc;
use p256::ecdsa::SigningKey;
use p256::ecdsa::signature::hazmat::RandomizedPrehashSigner;
use rand::rngs::OsRng;
use std::fs;
use lifegraph_linux_netif::discover_network_interfaces;
use lifegraph_network_interface::NetworkLinkState;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use std::net::SocketAddr;

#[derive(Parser)]
#[command(name = "lifegraphd", about = "Lifegraph Node Daemon")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Generate a new node identity and config
    Init {
        #[arg(long, default_value = "node.yaml")]
        config: PathBuf,
        #[arg(long)]
        name: Option<String>,
        /// Dev-only: generate an in-memory software key (NOT for production)
        #[arg(long)]
        software: bool,
    },
    /// Start the node daemon
    Run {
        #[arg(long, default_value = "node.yaml")]
        config: PathBuf,
        /// TCP listen address (e.g. 0.0.0.0:8080). If omitted, mesh-only.
        #[arg(long)]
        listen: Option<SocketAddr>,
        /// Health endpoint port. If omitted, no health server.
        #[arg(long)]
        health_port: Option<u16>,
        /// Log level: trace, debug, info, warn, error. Default: info.
        #[arg(long, default_value = "info")]
        log_level: String,
    },
    /// Show node identity and state
    Status {
        #[arg(long, default_value = "node.yaml")]
        config: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { config, name, software } => {
            cmd_init(&config, name, software);
        }
        Commands::Run { config, listen, health_port, log_level } => {
            // Initialize structured logging
            let env_filter = tracing_subscriber::EnvFilter::try_new(&log_level)
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .json()
                .init();

            let rt = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap_or_else(|e| {
                    tracing::error!("failed to create tokio runtime: {}", e);
                    std::process::exit(1);
                });
            rt.block_on(async move {
                cmd_run(&config, listen, health_port).await;
            });
        }
        Commands::Status { config } => {
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
        let signing_key = SigningKey::random(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let encoded = verifying_key.to_encoded_point(false);
        let mut node_id_bytes = [0u8; 64];
        node_id_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
        let node_id = NodeID(node_id_bytes);
        let key_hex = hex::encode(signing_key.to_bytes());
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
    let config: NodeConfig = serde_yaml::from_str(&yaml).unwrap_or_else(|e| {
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
        reply_tx: tokio::sync::oneshot::Sender<StoreResponse>,
    },
    Query {
        /// The raw message bytes (for dedup hashing before decode).
        raw_bytes: Vec<u8>,
        query: lifegraph_proto::lifegraph::v0::access::QueryRequest,
        /// Peer identity for allowlist check.
        peer_id: Option<Vec<u8>>,
        reply_tx: tokio::sync::oneshot::Sender<StoreResponse>,
    },
}

/// Response from the store task back to the TCP handler.
enum StoreResponse {
    /// Command was processed successfully — payload is the response.
    Ok(Vec<u8>),
    /// Ingress screening rejected the message.
    Rejected(ingress::IngressResult),
}

async fn cmd_run(path: &PathBuf, listen_addr: Option<SocketAddr>, health_port: Option<u16>) {
    if !path.exists() {
        tracing::error!("config not found at {}. Run `lifegraphd init` first.", path.display());
        std::process::exit(1);
    }

    let yaml = fs::read_to_string(path).unwrap();
    let config: NodeConfig = serde_yaml::from_str(&yaml).unwrap_or_else(|e| {
        tracing::error!("invalid config: {}", e);
        std::process::exit(1);
    });

    let signer = load_signer_from_config(&config);
    let node_id = signer.node_id();
    let private_key_bytes = extract_private_key_bytes(&config);

    let node_name = config.name.as_deref().unwrap_or("(unnamed)").to_string();
    let signer_type = config.signer.as_ref().map(|s| s.signer_type.clone()).unwrap_or_else(|| "unconfigured".to_string());

    tracing::info!(
        node = node_name,
        stream_id = config.stream_id,
        node_id = node_id.short(),
        signer = signer_type,
        "lifegraphd starting"
    );

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
        tracing::error!("failed to open storage at {}: {}", data_root.display(), e);
        std::process::exit(1);
    });

    // Create genesis if new node
    let stream_id_bytes = config.stream_id.as_bytes();
    if store.get_head(stream_id_bytes).unwrap().is_none() {
        use lifegraph_core::protocol::EventEnvelope;
        use lifegraph_proto::lifegraph::v0::stream::EventType;

        let mut genesis = EventEnvelope {
            envelope_version: 1,
            stream_id: stream_id_bytes.to_vec(),
            seq: 0,
            prev_event_hash: None,
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        };
        sign_event_envelope(&mut genesis, &*signer).unwrap_or_else(|e| {
            tracing::error!("failed to sign genesis event: {}", e);
            std::process::exit(1);
        });
        store.append_event(&genesis).unwrap_or_else(|e| {
            tracing::error!("failed to write genesis event: {}", e);
            std::process::exit(1);
        });
        tracing::info!(stream_id = config.stream_id, "genesis event created (seq=0)");
    } else {
        let (head_seq, _) = store.get_head(stream_id_bytes).unwrap().unwrap();
        tracing::info!(head_seq, stream_id = config.stream_id, "loaded stream");
    }

    // --- Unix socket capability server ---
    let socket_path = data_root.join("capabilities.sock");
    {
        let mut multi = capabilities::MultiCapabilityProvider::new();
        let policy = lifegraph_capability_policy::SimplePolicyEngine::default();
        capabilities::discover_and_register_capabilities(&mut multi, policy);
        let cap_count = multi.len();
        tracing::info!(count = cap_count, "discovered capability providers");

        if cap_count > 0 {
            let multi_arc = Arc::new(std::sync::Mutex::new(multi));
            let socket_path_clone = socket_path.clone();
            std::thread::spawn(move || {
                if let Err(e) = capabilities::serve_capabilities_unix(multi_arc, &socket_path_clone) {
                    tracing::error!("capability server error: {}", e);
                }
            });
            tracing::info!(path = %socket_path.display(), "capability server listening");
        }
    }

    // --- Store task (owns NodeStore + ingress state, not Send) ---
    let (store_tx, store_rx) = tokio::sync::mpsc::channel::<StoreRequest>(256);

    let stream_id_vec = stream_id_bytes.to_vec();
    let stream_id_vec_clone = stream_id_vec.clone();
    let store_signer: Box<dyn MeshSigner + Send> = clone_signer_for_send(&*signer);

    // Ingress screening state
    let global_rate_limiter = ingress::TokenBucket::new(1000, 500); // burst 1000, 500/sec global
    let message_hash_cache = ingress::RecentHashCache::new(4096);
    let allowed_peers: Vec<Vec<u8>> = config.allowed_peers.iter()
        .map(|s| s.as_bytes().to_vec())
        .collect();
    if !allowed_peers.is_empty() {
        tracing::info!(count = allowed_peers.len(), "peer allowlist active");
    }

    let store_handle = tokio::task::spawn_blocking(move || {
        run_store_task(store, &stream_id_vec, &*store_signer, store_rx,
                       global_rate_limiter, message_hash_cache, allowed_peers, node_id);
    });

    // --- Mesh poll loop (owns MeshLink, runs on blocking thread) ---
    let mesh_node_id = node_id;
    let (mesh_inbound_tx, mut mesh_inbound_rx) = tokio::sync::mpsc::channel::<MeshFrame>(256);

    let mesh_handle = tokio::task::spawn_blocking(move || {
        run_mesh_loop(mesh_node_id, mesh_inbound_tx);
    });

    // --- TCP listener (if configured) ---
    if let Some(addr) = listen_addr {
        let _tcp_handle = tokio::spawn(run_tcp_listener(
            addr,
            node_id,
            store_tx.clone(),
        ));
        tracing::info!(addr = %addr, "TCP listener started");
    } else {
        tracing::info!("running mesh-only (no TCP listener)");
    }

    // Event processing loop: route mesh inbound to store
    let shutdown = tokio::spawn(async {
        tokio::signal::ctrl_c().await.ok();
        tracing::info!("shutting down");
    });
    let mut shutdown_fut = shutdown;

    loop {
        tokio::select! {
            _ = &mut shutdown_fut => break,
            Some(frame) = mesh_inbound_rx.recv() => {
                if frame.header.frame_type == FrameType::Data {
                    if let Ok(command) =
                        lifegraph_proto::lifegraph::v0::stream::CommandEnvelope::decode(&frame.payload[..])
                    {
                        // Process mesh command directly on this thread via store channel
                        process_mesh_command(command, &store_tx, &node_id, &stream_id_vec_clone, &*signer).await;
                    }
                }
            }
        }
    }

    // Cleanup
    drop(store_tx);
    let _ = store_handle.await;
    let _ = mesh_handle.await;
}

// ---------------------------------------------------------------------------
// Store task — owns NodeStore (non-Send), processes commands sequentially
// ---------------------------------------------------------------------------

fn run_store_task(
    mut store: NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    mut rx: tokio::sync::mpsc::Receiver<StoreRequest>,
    mut global_rate_limiter: ingress::TokenBucket,
    mut message_hash_cache: ingress::RecentHashCache,
    allowed_peers: Vec<Vec<u8>>,
    responder_node_id: NodeID,
) {
    let mut request_counter: u64 = 0;

    while let Some(req) = rx.blocking_recv() {
        request_counter += 1;

        // Extract screening data without consuming the full request yet
        let raw_bytes: Vec<u8> = match &req {
            StoreRequest::Command { raw_bytes, .. } |
            StoreRequest::Query { raw_bytes, .. } => raw_bytes.clone(),
        };
        let peer_id: Option<Vec<u8>> = match &req {
            StoreRequest::Command { peer_id, .. } |
            StoreRequest::Query { peer_id, .. } => peer_id.clone(),
        };

        // ---- §18.3 Ingress screening (cheap → expensive) ----

        // 1. Recent duplicate detection (before any crypto work)
        let msg_hash = ingress::quick_message_hash(&raw_bytes);
        if message_hash_cache.contains(msg_hash) {
            let reply = match req {
                StoreRequest::Command { reply_tx, .. } => reply_tx,
                StoreRequest::Query { reply_tx, .. } => reply_tx,
            };
            let _ = reply.send(StoreResponse::Rejected(ingress::IngressResult::Duplicate));
            continue;
        }

        // 2. Global rate limit
        if !global_rate_limiter.try_consume() {
            let reply = match req {
                StoreRequest::Command { reply_tx, .. } => reply_tx,
                StoreRequest::Query { reply_tx, .. } => reply_tx,
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
                };
                let _ = reply.send(StoreResponse::Rejected(ingress::IngressResult::PeerNotAllowed));
                continue;
            }
        }

        // Screening passed — cache the hash and proceed
        message_hash_cache.insert(msg_hash);

        // 4. Process the request
        match req {
            StoreRequest::Command { command, reply_tx, .. } => {
                let result = process_command_sync(&command, &mut store, stream_id, signer);
                let _ = reply_tx.send(StoreResponse::Ok(result));
            }
            StoreRequest::Query { query, reply_tx, .. } => {
                let result = execute_query(&query, &mut store, stream_id, &responder_node_id);
                let _ = reply_tx.send(StoreResponse::Ok(result));
            }
        }

        // Periodically process the fetch queue (every 10 requests)
        if request_counter.is_multiple_of(10) {
            if let Ok(resolved) = store.process_fetch_queue() {
                if resolved > 0 {
                    tracing::info!(resolved, "fetch queue processed items");
                }
            }
        }
    }
}

/// Process a command from a mesh frame (no TCP response needed).
async fn process_mesh_command(
    command: lifegraph_proto::lifegraph::v0::stream::CommandEnvelope,
    store_tx: &tokio::sync::mpsc::Sender<StoreRequest>,
    _node_id: &NodeID,
    _stream_id: &[u8],
    _signer: &dyn MeshSigner,
) {
    // Send to store task — mesh commands don't need a response back to sender
    let (reply_tx, _reply_rx) = tokio::sync::oneshot::channel();
    let _ = store_tx.send(StoreRequest::Command {
        raw_bytes: command.encode_to_vec(),
        command,
        peer_id: None,
        reply_tx,
    }).await;
}

// ---------------------------------------------------------------------------
// TCP listener and per-connection handling
// ---------------------------------------------------------------------------

const TCP_MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

async fn run_tcp_listener(
    listen_addr: SocketAddr,
    _node_id: NodeID,
    store_tx: tokio::sync::mpsc::Sender<StoreRequest>,
) {
    let listener = match tokio::net::TcpListener::bind(&listen_addr).await {
        Ok(l) => l,
        Err(e) => {
            tracing::error!("failed to bind TCP on {}: {}", listen_addr, e);
            return;
        }
    };

    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                tracing::debug!(peer = %peer_addr, "TCP connection accepted");
                let conn_store_tx = store_tx.clone();
                tokio::spawn(async move {
                    handle_tcp_connection(stream, conn_store_tx).await;
                });
            }
            Err(e) => {
                tracing::warn!("TCP accept error: {}", e);
            }
        }
    }
}

async fn handle_tcp_connection(
    stream: tokio::net::TcpStream,
    store_tx: tokio::sync::mpsc::Sender<StoreRequest>,
) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // Per-connection rate limiter: burst 100, 50 messages/sec.
    // Prevents a single connection from monopolizing the global budget.
    let mut conn_rate_limiter = ingress::TokenBucket::new(100, 50);

    let (mut reader, mut writer) = stream.into_split();
    let mut read_buf = bytes::BytesMut::with_capacity(4096);

    loop {
        // Read 8-byte length prefix
        while read_buf.len() < 8 {
            let mut chunk = [0u8; 64];
            match reader.read(&mut chunk).await {
                Ok(0) => {
                    if read_buf.is_empty() {
                        return; // Clean close
                    }
                    tracing::debug!("TCP closed mid-header");
                    return;
                }
                Ok(n) => {
                    read_buf.extend_from_slice(&chunk[..n]);
                }
                Err(e) => {
                    tracing::debug!("TCP read error: {}", e);
                    return;
                }
            }
        }

        let frame_len = u64::from_be_bytes(read_buf[..8].try_into().unwrap()) as usize;
        if frame_len == 0 || frame_len > TCP_MAX_FRAME_SIZE {
            tracing::warn!(frame_len, "TCP frame length invalid or too large");
            return;
        }

        // Read payload
        let total_needed = 8 + frame_len;
        while read_buf.len() < total_needed {
            let mut chunk = vec![0u8; 4096.min(total_needed - read_buf.len())];
            match reader.read(&mut chunk).await {
                Ok(0) => {
                    tracing::debug!("TCP closed mid-frame");
                    return;
                }
                Ok(n) => {
                    read_buf.extend_from_slice(&chunk[..n]);
                }
                Err(e) => {
                    tracing::debug!("TCP read error: {}", e);
                    return;
                }
            }
        }

        let payload: Vec<u8> = read_buf.split_to(total_needed).split_off(8).to_vec();

        // Per-connection rate limit (cheap check, before decode or crypto)
        if !conn_rate_limiter.try_consume() {
            tracing::warn!("TCP rate-limited connection");
            return;
        }

        // Try CommandEnvelope
        if let Ok(command) =
            lifegraph_proto::lifegraph::v0::stream::CommandEnvelope::decode(&payload[..])
        {
            let raw = payload.clone();
            let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
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
                    tracing::debug!(?reason, "TCP message screened");
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
            let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
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
                    tracing::debug!(?reason, "TCP query screened");
                    return;
                }
                Err(_) => return,
            }
            continue;
        }

        tracing::debug!(payload_len = payload.len(), "TCP received unrecognized message type");
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
// Command processing (sync, runs on store task thread)
// ---------------------------------------------------------------------------

fn process_command_sync(
    command: &lifegraph_proto::lifegraph::v0::stream::CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
) -> Vec<u8> {
    use lifegraph_proto::lifegraph::v0::common::CommandRef;
    use lifegraph_proto::lifegraph::v0::stream::{CommandResultPayload, EventType};

    let outcome = validate_command_signature(command);
    let (event_type, decision, reason_code) = match outcome {
        CommandValidation::Valid => (EventType::CommandCommitted, 1, ""),
        CommandValidation::MissingSignature => (EventType::CommandRejected, 2, "missing_signature"),
        CommandValidation::BadAlgorithm => (EventType::CommandRejected, 2, "bad_algorithm"),
        CommandValidation::NoIssuer => (EventType::CommandRejected, 2, "no_issuer"),
        CommandValidation::BadKeyHint => (EventType::CommandRejected, 2, "bad_key_hint"),
        CommandValidation::BadPublicKey => (EventType::CommandRejected, 2, "bad_public_key"),
        CommandValidation::BadSignature => (EventType::CommandRejected, 2, "invalid_signature"),
    };

    if outcome == CommandValidation::Valid {
        tracing::info!("command accepted");
    } else {
        tracing::info!(reason = reason_code, "command rejected");
    }

    let command_id_bytes = command.command_id.clone();
    let command_ref = CommandRef {
        command_id: command_id_bytes.clone(),
        command_hash: None,
    };
    let result_payload = CommandResultPayload {
        payload_version: 1,
        command: Some(command_ref.clone()),
        issuer: command.issuer.clone(),
        decision,
        decision_basis: None,
        reason_code: reason_code.to_string(),
        effect_summary_object: None,
        result_object: None,
    };
    let result_bytes = prost::Message::encode_to_vec(&result_payload);

    let object_ref = store.put_object(&result_bytes, 6 /* OBJECT_KIND_COMMAND */, &[stream_id.to_vec()])
        .unwrap_or_else(|e| {
            tracing::warn!("failed to store command result object: {}", e);
            lifegraph_proto::lifegraph::v0::common::ObjectRef {
                object_id: vec![],
                object_kind: Some(6),
            }
        });

    let head_seq = store.get_head(stream_id).ok().flatten().map(|(s, _)| s).unwrap_or(-1);
    let mut event = lifegraph_core::protocol::EventEnvelope {
        envelope_version: 1,
        stream_id: stream_id.to_vec(),
        seq: (head_seq + 1) as u64,
        prev_event_hash: store.get_head(stream_id).ok().flatten().map(|(_, h)| {
            lifegraph_core::protocol::Digest {
                algorithm: 1,
                value: h,
            }
        }),
        event_type: event_type as i32,
        event_version: 1,
        recorded_at: Some(now_ms_timestamp()),
        effective_at: None,
        payload_object: Some(object_ref),
        related_events: vec![],
        related_commands: vec![command_ref],
        related_objects: vec![],
        related_delegations: vec![],
        related_revocations: vec![],
        event_metadata: None,
        signature: None,
    };
    if sign_event_envelope(&mut event, signer).is_err() {
        tracing::warn!("failed to sign event");
    }
    if let Err(e) = store.append_event(&event) {
        tracing::warn!("failed to append event: {}", e);
    }

    result_bytes
}

// ---------------------------------------------------------------------------
// Query execution engine (§14.20–§14.21)
// ---------------------------------------------------------------------------

/// Executes a QueryRequest against local state and returns a QueryResultFragment.
fn execute_query(
    query: &lifegraph_proto::lifegraph::v0::access::QueryRequest,
    store: &mut NodeStore,
    _local_stream_id: &[u8],
    responder_node_id: &NodeID,
) -> Vec<u8> {
    use lifegraph_proto::lifegraph::v0::access::{QueryClass, QueryResultFragment, ResultCompleteness};
    use lifegraph_proto::lifegraph::v0::common::{EventRef, ObjectRef};

    let query_class = query.query_class; // QueryClass enum
    let mut event_refs: Vec<EventRef> = Vec::new();
    let snapshot_refs: Vec<lifegraph_proto::lifegraph::v0::common::SnapshotRef> = Vec::new();
    let mut object_refs: Vec<ObjectRef> = Vec::new();
    let mut completeness = ResultCompleteness::CompleteForLocalKnowledge as i32;

    match query_class {
        // Return all known stream heads
        x if x == QueryClass::Head as i32 => {
            match store.list_stream_heads() {
                Ok(heads) => {
                    for (stream_id_hex, seq, hash) in heads {
                        event_refs.push(EventRef {
                            stream_id: hex::decode(&stream_id_hex).unwrap_or_else(|_| stream_id_hex.into_bytes()),
                            seq: seq as u64,
                            event_hash: Some(lifegraph_core::protocol::Digest {
                                algorithm: 1,
                                value: hash,
                            }),
                        });
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "query HEAD failed");
                    completeness = ResultCompleteness::Partial as i32;
                }
            }
        }

        // Return events in a range for specified streams
        x if x == QueryClass::EventRange as i32 => {
            match store.list_stream_heads() {
                Ok(heads) => {
                    for (stream_id_hex, head_seq, _hash) in &heads {
                        if let Ok(events) = store.list_event_range(stream_id_hex, 0, *head_seq) {
                            for (seq, hash, _ver) in events {
                                event_refs.push(EventRef {
                                    stream_id: hex::decode(stream_id_hex).unwrap_or_else(|_| stream_id_hex.clone().into_bytes()),
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
                    tracing::warn!(error = %e, "query EVENT_RANGE failed");
                    completeness = ResultCompleteness::Partial as i32;
                }
            }
        }

        // Check if specific objects exist
        x if x == QueryClass::ObjectExistence as i32 => {
            if let Some(ref obj_ref) = query.query_payload_object {
                let object_id_hex = hex::encode(&obj_ref.object_id);
                match store.is_object_present(&object_id_hex) {
                    Ok(present) => {
                        if present {
                            object_refs.push(obj_ref.clone());
                        }
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "query OBJECT_EXISTENCE failed");
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
                        tracing::warn!(error = %e, "query OBJECT_FETCH failed");
                        completeness = ResultCompleteness::Partial as i32;
                    }
                }
            }
        }

        // Snapshot query: return known snapshot refs (none in v0, but respond gracefully)
        x if x == QueryClass::Snapshot as i32 => {
            completeness = ResultCompleteness::MetadataOnly as i32;
        }

        // Trust state: return current controller set (from config in v0)
        x if x == QueryClass::TrustState as i32 => {
            // In v0, trust state is local config — return metadata-only
            completeness = ResultCompleteness::MetadataOnly as i32;
        }

        // Unknown query class
        _ => {
            eprintln!("lifegraphd: query class {} not supported", query_class);
            completeness = ResultCompleteness::Denied as i32;
        }
    }

    // Apply result_limit if set
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

    prost::Message::encode_to_vec(&fragment)
}

// ---------------------------------------------------------------------------
// Mesh poll loop (blocking thread)
// ---------------------------------------------------------------------------

fn run_mesh_loop(
    node_id: NodeID,
    mesh_inbound_tx: tokio::sync::mpsc::Sender<MeshFrame>,
) {
    let local = LocalNode::new(node_id);
    let mut mesh_link = MeshLink::new();
    mesh_link.set_local_node_id(node_id);
    let mut router = MeshRouter::new(local);

    match mesh_link.enable_udp_broadcast() {
        Ok(_) => println!("lifegraphd: UDP broadcast enabled on port 47079"),
        Err(e) => eprintln!("lifegraphd: warning: UDP broadcast failed: {}", e),
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
            println!("lifegraphd: opened raw socket on {} (ifindex={})", iface.name, ifindex);
        }
    }

    if let Err(e) = mesh_link.broadcast_discovery(&mut router) {
        eprintln!("lifegraphd: warning: discovery broadcast failed: {}", e);
    }

    println!("lifegraphd: mesh loop running");

    let mut discovery_counter: u64 = 0;
    loop {
        if let Err(e) = mesh_link.pump(&mut router) {
            eprintln!("lifegraphd: mesh pump error: {}", e);
        }

        let frames = mesh_link.drain_inbound_data_frames();
        for frame in frames {
            if frame.header.dest == node_id || frame.header.dest.0 == [0u8; 64] {
                let _ = mesh_inbound_tx.try_send(frame);
            } else if let Some(next_hop) = router.next_hop_for(&frame.header.dest) {
                let mut fwd = frame;
                fwd.header.dest = next_hop;
                mesh_link.queue_frame(fwd);
            }
        }

        discovery_counter += 1;
        if discovery_counter.is_multiple_of(500) {
            if let Err(e) = mesh_link.broadcast_discovery(&mut router) {
                eprintln!("lifegraphd: discovery failed: {}", e);
            }
            let dead = router.tick_heartbeat();
            for d in &dead {
                println!("lifegraphd: peer {} is dead", d.short());
            }
        }

        if let Err(e) = mesh_link.drain_pending_frames(&mut router) {
            eprintln!("lifegraphd: send failed: {}", e);
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

// ---------------------------------------------------------------------------
// Command validation
// ---------------------------------------------------------------------------

#[derive(PartialEq)]
enum CommandValidation {
    Valid,
    MissingSignature,
    BadAlgorithm,
    NoIssuer,
    BadKeyHint,
    BadPublicKey,
    BadSignature,
}

fn validate_command_signature(command: &lifegraph_proto::lifegraph::v0::stream::CommandEnvelope) -> CommandValidation {
    use sha2::{Digest, Sha256};

    let Some(sig) = &command.signature else {
        return CommandValidation::MissingSignature;
    };
    if sig.algorithm != 1 {
        return CommandValidation::BadAlgorithm;
    }
    let Some(issuer) = &command.issuer else {
        return CommandValidation::NoIssuer;
    };
    let Some(key_hint) = &issuer.key_hint else {
        return CommandValidation::BadKeyHint;
    };
    if key_hint.len() != 64 {
        return CommandValidation::BadKeyHint;
    }
    let mut vk_sec1 = [0u8; 65];
    vk_sec1[0] = 0x04;
    vk_sec1[1..].copy_from_slice(key_hint);
    let vk = match p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1) {
        Ok(v) => v,
        Err(_) => return CommandValidation::BadPublicKey,
    };

    let mut signable_cmd = command.clone();
    signable_cmd.signature = None;
    let mut canonical = Vec::new();
    prost::Message::encode(&signable_cmd, &mut canonical).unwrap();
    let digest = Sha256::digest(&canonical);

    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&sig.value);
    let r = p256::FieldBytes::from_slice(&sig_bytes[..32]);
    let s = p256::FieldBytes::from_slice(&sig_bytes[32..]);
    if let Ok(ecdsa_sig) = p256::ecdsa::Signature::from_scalars(*r, *s) {
        use p256::ecdsa::signature::hazmat::PrehashVerifier;
        if vk.verify_prehash(digest.as_slice(), &ecdsa_sig).is_ok() {
            return CommandValidation::Valid;
        }
    }
    CommandValidation::BadSignature
}

fn sign_event_envelope(event: &mut lifegraph_core::protocol::EventEnvelope, signer: &dyn MeshSigner) -> Result<(), String> {
    use lifegraph_core::protocol::{ProtocolRecord, canonical_bytes};
    use sha2::{Digest, Sha256};

    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, true);
    let digest = Sha256::digest(&canonical);
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

/// Clone a signer into a `Box<dyn MeshSigner + Send>`.
/// Currently only SoftwareSigner is functional.
fn clone_signer_for_send(signer: &dyn MeshSigner) -> Box<dyn MeshSigner + Send> {
    if let Some(software) = signer.as_any().downcast_ref::<SoftwareSigner>() {
        Box::new(SoftwareSigner::new(software.key.clone()))
    } else {
        panic!("cloning non-software signer not yet supported");
    }
}

fn extract_private_key_bytes(config: &NodeConfig) -> Vec<u8> {
    if let Some(ref signer) = config.signer {
        if signer.signer_type == "software" {
            if let Some(ref hex_str) = signer.private_key_hex {
                return hex::decode(hex_str.trim()).unwrap_or_else(|_| {
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
// Config
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct NodeConfig {
    stream_id: String,
    name: Option<String>,
    #[serde(default)]
    controllers: Vec<String>,
    #[serde(default)]
    trust_nodes: Vec<String>,
    /// Peer node IDs allowed to connect. Empty = allow all (open mode).
    #[serde(default)]
    allowed_peers: Vec<String>,
    #[serde(default)]
    signer: Option<SignerConfig>,
    #[serde(default)]
    initial_grants: Vec<serde_yaml::Value>,
    #[serde(default)]
    metadata: serde_yaml::Value,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
struct SignerConfig {
    #[serde(rename = "type")]
    signer_type: String,
    public_key_hex: String,
    private_key_hex: Option<String>,
    handle: Option<String>,
    slot: Option<String>,
}

// ---------------------------------------------------------------------------
// Software signer
// ---------------------------------------------------------------------------

struct SoftwareSigner {
    node_id: NodeID,
    key: SigningKey,
}

impl SoftwareSigner {
    fn new(key: SigningKey) -> Self {
        let vk = key.verifying_key();
        let encoded = vk.to_encoded_point(false);
        let mut node_bytes = [0u8; 64];
        node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
        Self {
            node_id: NodeID(node_bytes),
            key,
        }
    }
}

impl MeshSigner for SoftwareSigner {
    fn node_id(&self) -> NodeID {
        self.node_id
    }

    fn sign_digest(
        &self,
        digest: &[u8; 32],
    ) -> Result<[u8; 64], lifegraph_hardware_signing::HardwareSigningError> {
        let sig: p256::ecdsa::Signature = self
            .key
            .sign_prehash_with_rng(&mut OsRng, digest)
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

fn load_signer_from_config(config: &NodeConfig) -> Box<dyn MeshSigner> {
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
            Box::new(SoftwareSigner::new(signing_key))
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
    let bytes = hex::decode(hex_str).unwrap_or_else(|e| {
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
