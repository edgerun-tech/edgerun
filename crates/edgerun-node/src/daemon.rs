use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use edgerun_hardware_signing::{MeshSigner, NodeID};
use edgerun_mesh_daemon::MeshDaemon;
use edgerun_storage::{NodeStore, NodeStoreConfig, BlobKeySource};
use edgerun_core::util::system_time_to_prost;
use prost::Message;

use crate::capabilities;
use crate::capacity;
use crate::command_dispatch;
use crate::config::{self, NodeConfig, BootstrapPeer};
use crate::config::{parse_config, parse_bootstrap_peers, extract_private_key_bytes};
use crate::signer::load_signer_from_config;
use crate::health::{HealthState, run_health_server};
use crate::ingress;
use crate::mesh_store_provider::{make_mesh_command_handler, MeshCommandBridge};
use crate::peer_reconnect::run_peer_reconnection;
use crate::session;
use crate::store_task::{run_store_task};
use crate::tcp_server::{SessionContext, run_tcp_listener, handle_tcp_connection, encode_tcp_frame, perform_session_handshake_as_initiator};
use crate::types::{StoreRequest, StoreResponse};
use crate::workload_policy;

async fn run_fetch_queue_consumer(
    index_path: std::path::PathBuf,
    peers: Vec<BootstrapPeer>,
    local_node_id: NodeID,
) {
    use edgerun_proto::edgerun::v0::access::{QueryClass, QueryRequest};
    use edgerun_proto::edgerun::v0::common::IdentityRef;
    use edgerun_proto::edgerun::v0::trust::{ScopeDescriptor, ScopeKind};
    use edgerun_storage::FileIndex;

    if peers.is_empty() {
        edgerun_log::info!("no bootstrap peers configured, fetch queue consumer disabled");
        return;
    }

    // Open a separate SQLite connection for the fetch queue
    let parent_path = match index_path.parent() {
        Some(p) => p.to_path_buf(),
        None => {
            edgerun_log::error!("index path has no parent directory: {:?}", index_path);
            return;
        }
    };
    let index = match FileIndex::open(&parent_path) {
        Ok(idx) => idx,
        Err(e) => {
            edgerun_log::error!("failed to open SQLite index for fetch queue: {}", e);
            return;
        }
    };

    let mut interval = edgerun_rt::interval(std::time::Duration::from_secs(30));
    interval.set_missed_tick_behavior(edgerun_rt::MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        // Dequeue one pending fetch at a time
        let fetch_entry = match index.dequeue_fetch() {
            Ok(Some(entry)) => entry,
            Ok(None) => continue, // Queue is empty
            Err(e) => {
                edgerun_log::warn!("failed to dequeue fetch: {}", e);
                continue;
            }
        };

        let fetch_type = fetch_entry.target_type.clone();
        let fetch_id = fetch_entry.target_id.clone();
        let fetch_priority = fetch_entry.priority;

        edgerun_log::debug!("processing fetch queue entry");

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
                Ok(fragment_bytes) => {
                    edgerun_log::info!("fetch query succeeded");
                    fetched = true;

                    // Try to decode the response and store fetched objects
                    if let Ok(fragment) =
                        edgerun_proto::edgerun::v0::access::QueryResultFragment::decode(&fragment_bytes[..])
                    {
                        // Log any object refs returned
                        for obj_ref in &fragment.object_refs {
                            if !obj_ref.object_id.is_empty() {
                                let obj_id_hex = edgerun_core::util::bytes_to_hex(&obj_ref.object_id);
                                edgerun_log::info!("fetched object reference: {}", obj_id_hex);
                            }
                        }

                        // Log event refs returned
                        for event_ref in &fragment.event_refs {
                            edgerun_log::info!("fetched event reference");
                            let _ = event_ref;
                        }

                        // Decode bundled_result_object if present and store it
                        if let Some(ref bundled) = fragment.bundled_result_object {
                            let obj_id = edgerun_core::util::bytes_to_hex(&bundled.object_id);
                            edgerun_log::info!("bundled result object: {}", obj_id);
                        }

                        // Decode bundled_result_object and store fetched objects
                        // The bundled_result_object is an ObjectRef pointing to an encrypted
                        // blob in the peer's store. In a full mesh implementation, we'd fetch
                        // the actual object data via the object protocol. For now, we record
                        // the fetch completion and log the object refs for downstream processing.
                        if !fragment.object_refs.is_empty() || fragment.bundled_result_object.is_some() {
                            edgerun_log::info!("fetch completed: {} object refs, bundled={}",
                                fragment.object_refs.len(),
                                fragment.bundled_result_object.is_some());
                        }
                    }

                    // Mark as done
                    let _ = index.mark_fetch_done(fetch_entry.id);
                    break;
                }
                Err(_e) => {
                    edgerun_log::debug!("peer query failed, trying next");
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
/// Called from the store task's blocking thread — does direct blocking TCP I/O.
pub fn send_command_to_peer(
    peer_addr: &str,
    command: &edgerun_proto::edgerun::v0::stream::CommandEnvelope,
    store: &mut NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // Record CommandSent event on our own stream (double-entry bookkeeping)
    command_dispatch::record_command_sent_event(store, stream_id, signer, command);

    // Do blocking TCP I/O directly since we're already on a blocking thread
    send_command_to_peer_blocking(peer_addr, command)
}

/// Direct blocking TCP connection to send a command to a peer.
fn send_command_to_peer_blocking(
    peer_addr: &str,
    command: &edgerun_proto::edgerun::v0::stream::CommandEnvelope,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let mut stream = TcpStream::connect_timeout(
        &peer_addr.parse()?,
        Duration::from_secs(10),
    )?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;

    let cmd_bytes = prost::Message::encode_to_vec(command);
    let frame = encode_tcp_frame(&cmd_bytes);
    stream.write_all(&frame)?;

    let mut header = [0u8; 8];
    stream.read_exact(&mut header)?;

    let payload_len = u64::from_be_bytes(header) as usize;
    let mut payload = vec![0u8; payload_len];
    stream.read_exact(&mut payload)?;

    Ok(payload)
}

/// Async version of send_command_to_peer -- no store mutation, just network I/O.
pub async fn send_command_to_peer_async(
    peer_addr: &str,
    command: &edgerun_proto::edgerun::v0::stream::CommandEnvelope,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use edgerun_rt::{AsyncReadExt, AsyncWriteExt};

    let mut stream = edgerun_rt::timeout(
        std::time::Duration::from_secs(10),
        edgerun_rt::ConnectFuture::new(peer_addr),
    ).await??;

    let cmd_bytes = prost::Message::encode_to_vec(command);
    let frame = encode_tcp_frame(&cmd_bytes);
    edgerun_rt::timeout(
        std::time::Duration::from_secs(10),
        stream.write_all(&frame),
    ).await??;

    let mut header = [0u8; 8];
    edgerun_rt::timeout(
        std::time::Duration::from_secs(10),
        stream.read_exact(&mut header),
    ).await??;

    let payload_len = u64::from_be_bytes(header) as usize;
    let mut payload = vec![0u8; payload_len];
    edgerun_rt::timeout(
        std::time::Duration::from_secs(30),
        stream.read_exact(&mut payload),
    ).await??;

    Ok(payload)
}

/// Sends a QueryRequest to a peer over TCP and returns the QueryResultFragment bytes.
pub async fn send_query_to_peer(
    peer_addr: &str,
    query: &edgerun_proto::edgerun::v0::access::QueryRequest,
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    use edgerun_rt::{AsyncReadExt, AsyncWriteExt};
    let mut stream = edgerun_rt::timeout(
        std::time::Duration::from_secs(10),
        edgerun_rt::ConnectFuture::new(peer_addr),
    ).await??;

    let query_bytes = prost::Message::encode_to_vec(query);
    let frame = encode_tcp_frame(&query_bytes);

    edgerun_rt::timeout(
        std::time::Duration::from_secs(10),
        stream.write_all(&frame),
    ).await??;

    // Read response (8-byte length prefix + payload)
    let mut header = [0u8; 8];
    edgerun_rt::timeout(
        std::time::Duration::from_secs(10),
        stream.read_exact(&mut header),
    ).await??;

    let payload_len = u64::from_be_bytes(header) as usize;
    let mut payload = vec![0u8; payload_len];
    edgerun_rt::timeout(
        std::time::Duration::from_secs(30),
        stream.read_exact(&mut payload),
    ).await??;

    Ok(payload)
}

/// Extracts the initial controller identities.
/// The node's own identity is always the initial controller.
fn config_controllers_from_signer(signer: &dyn MeshSigner) -> Vec<Vec<u8>> {
    vec![signer.node_id().0.to_vec()]
}

pub async fn cmd_run(path: &PathBuf, listen_addr: Option<SocketAddr>, health_port: Option<u16>, is_init: bool) {
    let yaml = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            edgerun_log::error!("config not found at {}: {}. Run `edgerund init` first.", path.display(), e);
            std::process::exit(1);
        }
    };
    let config: NodeConfig = parse_config(&yaml).unwrap_or_else(|e| {
        edgerun_log::error!("invalid config: {}", e);
        std::process::exit(1);
    });

    let signer = load_signer_from_config(&config);
    let node_id = signer.node_id();
    let private_key_bytes = extract_private_key_bytes(&config);

    let _node_name = config.name.as_deref().unwrap_or("(unnamed)").to_string();
    let _signer_type = config.signer.as_ref().map(|s| s.signer_type.clone()).unwrap_or_else(|| "unconfigured".to_string());

    // Determine local assurance class from signer type
    let local_assurance_class: i32 = match config.signer.as_ref().map(|s| s.signer_type.as_str()) {
        Some("tpm") | Some("yubikey") | Some("android-keystore") => 2, // HARDWARE_BACKED
        Some("software") => 1, // SOFTWARE
        _ => 0, // Unknown/unconfigured
    };

    edgerun_log::info!("edgerund starting");

    // --- Health endpoint ---
    if let Some(hp) = health_port {
        let health_state = HealthState {
            node_id: node_id.short(),
            stream_id: config.stream_id.clone(),
            started_at: std::time::Instant::now(),
        };
        edgerun_rt::spawn(run_health_server(hp, health_state));
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
        edgerun_log::error!("failed to open storage at {}: {}", data_root.display(), e);
        std::process::exit(1);
    });

    // Initialize resource capacity tracker (reserve 1 core + 512MB for system)
    let capacity = capacity::NodeCapacity::discover();
    let tracker = std::sync::Arc::new(capacity::ResourceTracker::new(
        &capacity,
        1,                              // reserve 1 core for system
        512 * 1024 * 1024,             // reserve 512MB for system
    ));
    edgerun_log::info!("capacity: {} cores, {} memory -- available: {} cores, {} memory",
        capacity.total_cores,
        capacity::format_bytes(capacity.total_memory_bytes),
        tracker.available_cores(),
        capacity::format_bytes(tracker.available_memory()),
    );

    // Load workload content policy (optional file next to config)
    let policy_path = path.parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("workload_policy.txt");
    let workload_policy = match workload_policy::load_policy_file(&policy_path) {
        Ok(p) => {
            if !p.allowed_registries.is_empty() || !p.blocked_images.is_empty() || !p.pinned_digests.is_empty() {
                edgerun_log::info!("workload policy loaded: {} registries, {} blocked, {} pinned",
                    p.allowed_registries.len(), p.blocked_images.len(), p.pinned_digests.len());
            }
            p
        }
        Err(e) => {
            edgerun_log::warn!("failed to load workload policy: {}", e);
            workload_policy::WorkloadPolicy::permissive()
        }
    };

    // Create genesis if new node
    let stream_id_bytes = config.stream_id.as_bytes();
    let head_result = store.get_head(stream_id_bytes).unwrap_or_else(|e| {
        edgerun_log::error!("failed to read stream head: {}", e);
        std::process::exit(1);
    });
    if head_result.is_none() {
        use edgerun_core::protocol::EventEnvelope;
        use edgerun_proto::edgerun::v0::stream::EventType;

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
            recorded_at: Some(system_time_to_prost(SystemTime::now())),
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
        command_dispatch::sign_event_envelope(&mut genesis, &*signer).unwrap_or_else(|e| {
            edgerun_log::error!("failed to sign genesis event: {}", e);
            std::process::exit(1);
        });
        store.append_event(&genesis).unwrap_or_else(|e| {
            edgerun_log::error!("failed to write genesis event: {}", e);
            std::process::exit(1);
        });
        edgerun_log::info!("genesis event created (seq=0)");
    } else {
        let head = match store.get_head(stream_id_bytes) {
            Ok(Some(head)) => head,
            Ok(None) => {
                edgerun_log::error!("stream head should exist after genesis check");
                std::process::exit(1);
            }
            Err(e) => {
                edgerun_log::error!("failed to read stream head: {}", e);
                std::process::exit(1);
            }
        };
        let (_head_seq, _) = head;
        edgerun_log::info!("loaded stream");
    }

    // --- Unix socket capability server ---
    let socket_path = data_root.join("capabilities.sock");
    {
        // Replay grants from event stream into policy engine
        let policy = if let Ok(Some((head_seq, _))) = store.get_head(stream_id_bytes) {
            capabilities::project_capability_grants(&store, stream_id_bytes, head_seq as u64)
        } else {
            edgerun_capability_policy::SimplePolicyEngine::default()
        };

        let mut multi = capabilities::MultiCapabilityProvider::new();
        capabilities::discover_and_register_capabilities(&mut multi, policy);
        let cap_count = multi.len();
        edgerun_log::info!("discovered {} capability providers", cap_count);

        if cap_count > 0 {
            let multi_arc = Arc::new(std::sync::Mutex::new(multi));
            let socket_path_clone = socket_path.clone();
            edgerun_rt::spawn_blocking(move || {
                if let Err(e) = capabilities::serve_capabilities_unix(multi_arc, &socket_path_clone) {
                    edgerun_log::error!("capability server error: {}", e);
                }
            });
            edgerun_log::info!("capability server listening on {}", socket_path.display());
        }
    }

    // --- Store task (owns NodeStore + ingress state, not Send) ---
    let (store_tx, store_rx) = edgerun_rt::mpsc::channel::<StoreRequest>(256);

    let stream_id_vec = stream_id_bytes.to_vec();
    let store_signer: Arc<dyn MeshSigner + Send + Sync> = Arc::clone(&signer);

    // Ingress screening state
    let global_rate_limiter = ingress::TokenBucket::new(1000, 500); // burst 1000, 500/sec global
    let message_hash_cache = ingress::RecentHashCache::new(4096);
    let allowed_peers: Vec<Vec<u8>> = config.allowed_peers.iter()
        .map(|s| s.as_bytes().to_vec())
        .collect();
    if !allowed_peers.is_empty() {
        edgerun_log::info!("peer allowlist active");
    }

    // --- Peer bootstrap (before store is moved) ---
    let bootstrap_peers = parse_bootstrap_peers(&config.bootstrap_peers);
    let unreachable_peers = store.list_unreachable_peers_with_addr().unwrap_or_default();
    for peer in &bootstrap_peers {
        if let Err(_e) = store.upsert_peer(&peer.node_id_hex, Some(&peer.addr), "unknown", true) {
            edgerun_log::warn!("failed to record bootstrap peer");
        }
    }

    // --- Mesh daemon (blocking thread) — handles session encryption, frame signing,
    //     and capability dispatch. Decrypted frames that aren't capability envelopes
    //     are forwarded to the store task via the command handler callback.
    let mesh_command_bridge = Arc::new(std::sync::Mutex::new(None::<MeshCommandBridge>));
    let bridge_for_daemon = mesh_command_bridge.clone();
    let mesh_handler = make_mesh_command_handler(store_tx.clone());

    let store_handle = edgerun_rt::spawn_blocking(move || {
        run_store_task(store, &stream_id_vec, &*store_signer, store_rx,
                       global_rate_limiter, message_hash_cache, allowed_peers, node_id,
                       tracker, workload_policy, local_assurance_class);
    });

    // Create the MeshDaemon on a separate blocking thread
    let mesh_daemon_node_id = node_id;
    let mesh_handle = edgerun_rt::spawn_blocking(move || {
        use edgerun_capabilities::capability_descriptor;
        use edgerun_capabilities::{CapabilityModality, CapabilityOperation, CapabilityRole};
        use edgerun_remote_capability::RemoteCapabilityProvider;

        // Minimal capability provider — mesh peers can discover this node
        // but actual command dispatch goes through the command_handler callback
        struct MeshNodeProvider;
        impl RemoteCapabilityProvider for MeshNodeProvider {
            fn descriptor(&self) -> edgerun_capabilities::CapabilityDescriptor {
                capability_descriptor(
                    "edgerun-mesh-node",
                    "edgerund",
                    CapabilityRole::Communication,
                    &[CapabilityModality::Text],
                    &[edgerun_capabilities::CapabilityEventKind::Text],
                    &[CapabilityOperation::Query],
                    Vec::new(),
                )
            }
            fn open_session(
                &mut self,
                open: &edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionOpen,
            ) -> Result<edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionAccept, edgerun_capabilities::CapabilityError> {
                Ok(edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionAccept {
                    version: open.version,
                    session_id: open.session_id.clone(),
                    accepted: true,
                    granted_operations: vec![CapabilityOperation::Query as i32],
                    granted_access_class: open.requested_access_class,
                    error_reason: String::new(),
                    grant_id: open.session_id.clone(),
                })
            }
            fn invoke(
                &mut self,
                _session_id: &[u8],
                _invocation: &edgerun_proto::edgerun::v0::capability::CapabilityInvocation,
                _inline_parameters: Option<&[u8]>,
            ) -> Result<edgerun_remote_capability::RemoteInvocationResult, edgerun_capabilities::CapabilityError> {
                Err(edgerun_capabilities::CapabilityError::Unsupported("use command_handler for mesh commands".into()))
            }
            fn close_session(&mut self, _close: &edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionClose) -> Result<(), edgerun_capabilities::CapabilityError> {
                Ok(())
            }
        }

        let mut daemon = MeshDaemon::new(
            mesh_daemon_node_id,
            edgerun_mesh_daemon::MeshDaemonConfig::default(),
            MeshNodeProvider,
        );

        // Register all UP network interfaces
        match daemon.discover_and_open_interfaces() {
            Ok(ifaces) => edgerun_log::info!("mesh daemon opened {} interfaces", ifaces.len()),
            Err(e) => edgerun_log::warn!("mesh daemon failed to open interfaces: {}", e),
        }

        // Enable UDP broadcast for local network peer discovery
        if let Err(e) = daemon.enable_udp_broadcast() {
            edgerun_log::warn!("mesh daemon UDP broadcast failed: {}", e);
        }

        // Wire up the command handler: decrypted non-capability frames → store task
        daemon = daemon.with_command_handler(mesh_handler);

        // Expose the outbound queue for the store task to send commands over mesh
        let outbound = daemon.outbound_queue();
        *bridge_for_daemon.lock().unwrap() = Some(MeshCommandBridge::new(outbound));

        edgerun_log::info!("mesh daemon running with encrypted sessions and command dispatch");

        // Run the daemon's event loop (blocks until shutdown)
        if let Err(e) = daemon.run() {
            edgerun_log::error!("mesh daemon error: {}", e);
        }
    });

    // --- TCP listener (if configured) ---
    if let Some(addr) = listen_addr {
        let tcp_signer: Arc<dyn edgerun_hardware_signing::MeshSigner + Send + Sync> = Arc::clone(&signer);
        let _tcp_handle = edgerun_rt::spawn(run_tcp_listener(
            addr,
            node_id,
            store_tx.clone(),
            tcp_signer,
        ));
        edgerun_log::info!("TCP listener started");
    } else {
        edgerun_log::info!("running mesh-only (no TCP listener)");
    }

    // --- Bootstrap peer connections ---
    if !bootstrap_peers.is_empty() {
        edgerun_log::info!("connecting to bootstrap peers");
        let bootstrap_signer: Arc<dyn edgerun_hardware_signing::MeshSigner + Send + Sync> = Arc::clone(&signer);
        let bootstrap_node_id = node_id;
        for peer in &bootstrap_peers {
            // Attempt TCP connection
            let peer_addr = peer.addr.clone();
            let peer_id_hex = peer.node_id_hex.clone();
            let conn_store_tx = store_tx.clone();
            let ctx = SessionContext {
                node_id: bootstrap_node_id,
                signer: Arc::clone(&bootstrap_signer),
            };
            edgerun_rt::spawn(async move {
                match edgerun_rt::ConnectFuture::new(&peer_addr).await {
                    Ok(stream) => {
                        edgerun_log::info!("connected to bootstrap peer");
                        // As initiator, generate nonce and send SessionHello first
                        let nonce = session::generate_nonce();
                        handle_tcp_connection(stream, conn_store_tx, &ctx, Some(nonce)).await;
                    }
                    Err(_e) => {
                        edgerun_log::warn!("failed to connect to bootstrap peer");
                    }
                }
            });
        }
    }

    // --- Fetch queue consumer: processes pending fetch requests by querying peers ---
    //
    // NOTE: This opens a second FileIndex to the same data directory as the store
    // task. Both instances read/write the same binary index files (fetch_queue.bin).
    // This is a known data race. The fix is to route fetch queue operations through
    // the store task via the existing mpsc channel (add FetchDequeue/FetchMarkDone
    // variants to StoreRequest).
    let fetch_store_path = data_root.join("index.sqlite3");
    let fetch_peers = bootstrap_peers.clone();
    let fetch_node_id = node_id;
    let _fetch_handle = edgerun_rt::spawn(async move {
        run_fetch_queue_consumer(fetch_store_path, fetch_peers, fetch_node_id).await;
    });

    // --- Peer reconnection task ---
    let recon_store_tx = store_tx.clone();
    let recon_peers = unreachable_peers;
    edgerun_rt::spawn(async move {
        run_peer_reconnection(recon_peers, recon_store_tx).await;
    });

    // Wait for shutdown signal
    let shutdown = edgerun_rt::spawn(async move {
        if is_init {
            // In init mode, poll the shutdown flag (set by signal handler)
            loop {
                if crate::init::SHUTDOWN_REQUESTED.load(std::sync::atomic::Ordering::Relaxed) {
                    edgerun_log::info!("shutdown requested (init mode)");
                    break;
                }
                edgerun_rt::sleep(std::time::Duration::from_millis(100)).await;
            }
        } else {
            // Normal mode: wait for ctrl_c
            edgerun_rt::ctrl_c().await.ok();
            edgerun_log::info!("shutdown requested (normal mode)");
        }
    });
    let _ = shutdown.await;

    // Cleanup
    edgerun_log::info!("shutting down");
    drop(store_tx);
    let _ = store_handle.await;
    let _ = mesh_handle.await;
    edgerun_log::info!("shutdown complete");
}
