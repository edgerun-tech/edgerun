use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use edgerun_core::util::now_prost_timestamp;
use edgerun_encoding::byteorder::read_u64_be;
use edgerun_hardware_signing::{MeshSigner, NodeID};
use edgerun_mesh_daemon::MeshDaemon;
use edgerun_rt::CancellationToken;
use edgerun_storage::{BlobKeySource, FetchEntry, NodeStore, NodeStoreConfig};
use prost::Message;

use crate::capabilities;
use crate::capacity;
use crate::command_dispatch;
use crate::config::{self, BootstrapPeer, NodeConfig};
use crate::config::{extract_private_key_bytes, parse_bootstrap_peers, parse_config};
use crate::health::{run_health_server, HealthState};
use crate::ingress;
use crate::mesh_store_provider::{make_mesh_command_handler, MeshCommandBridge};
use crate::peer_reconnect::run_peer_reconnection;
use crate::provisioning_listener::run_provisioning_listener;
use crate::session;
use crate::signer::load_signer_from_config;
use crate::store_task::run_store_task;
use crate::tcp_server::{
    encode_tcp_frame, handle_tcp_connection, perform_session_handshake_as_initiator,
    run_tcp_listener, SessionContext,
};
use crate::types::{StoreRequest, StoreResponse};
use crate::workload_policy;

async fn run_fetch_queue_consumer(
    peers: Vec<BootstrapPeer>,
    local_node_id: NodeID,
    signer: Arc<dyn MeshSigner + Send + Sync>,
    store_tx: edgerun_rt::mpsc::Sender<StoreRequest>,
    cancel: CancellationToken,
) {
    use edgerun_core::protocol::{canonical_bytes, ProtocolRecord, Signature};
    use edgerun_core::result::Verdict;
    use edgerun_core::validators_proto::validate_query_result_fragment;
    use edgerun_proto::edgerun::v0::access::{QueryClass, QueryRequest};
    use edgerun_proto::edgerun::v0::common::IdentityRef;
    use edgerun_proto::edgerun::v0::trust::{ScopeDescriptor, ScopeKind};

    if peers.is_empty() {
        edgerun_log::info!("no bootstrap peers configured, fetch queue consumer disabled");
        return;
    }

    let mut interval = edgerun_rt::interval(std::time::Duration::from_secs(30));
    interval.set_missed_tick_behavior(edgerun_rt::MissedTickBehavior::Skip);

    loop {
        if cancel.is_cancelled() {
            edgerun_log::info!("fetch queue consumer shutting down");
            return;
        }
        interval.tick().await;

        // Dequeue one pending fetch via the store task (no direct FileIndex access)
        let (reply_tx, reply_rx) = edgerun_rt::oneshot::channel();
        if store_tx
            .send(StoreRequest::FetchDequeue { reply_tx })
            .await
            .is_err()
        {
            edgerun_log::info!("store task unavailable, fetch queue consumer exiting");
            return;
        }
        let fetch_entry = match reply_rx.await {
            Ok(StoreResponse::Ok(data)) if data.is_empty() => continue, // Queue empty
            Ok(StoreResponse::Ok(data)) => {
                // Parse "type:id:priority"
                let parts: Vec<&str> = std::str::from_utf8(&data)
                    .unwrap_or("")
                    .splitn(3, ':')
                    .collect();
                if parts.len() != 3 {
                    edgerun_log::warn!("invalid fetch entry format");
                    continue;
                }
                let priority: i64 = parts[2].parse().unwrap_or(0);
                FetchEntry {
                    id: 0,
                    target_type: parts[0].to_string(),
                    target_id: parts[1].to_string(),
                    priority,
                    created_at: 0,
                    status: String::new(),
                }
            }
            _ => continue,
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
            let mut query = QueryRequest {
                request_version: 1,
                query_id,
                requester: Some(IdentityRef {
                    identity_id: local_node_id.0.to_vec(),
                    identity_kind: Some(2), // NODE
                    key_hint: Some(local_node_id.0.to_vec()),
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
            let canonical = canonical_bytes(&ProtocolRecord::QueryRequest(query.clone()), true);
            match signer.sign_record(edgerun_core::crypto::SIG_DOMAIN_QUERY_REQUEST, &canonical) {
                Ok(sig) => {
                    query.signature = Some(Signature {
                        algorithm: 1,
                        value: sig.to_vec(),
                    });
                }
                Err(e) => {
                    edgerun_log::warn!("failed to sign fetch query: {}", e);
                    continue;
                }
            }

            // Send query over TCP to the peer
            match send_query_to_peer(&peer_addr, &query).await {
                Ok(fragment_bytes) => {
                    edgerun_log::info!("fetch query succeeded");
                    fetched = true;

                    // Try to decode the response and store fetched objects
                    if let Ok(fragment) =
                        edgerun_proto::edgerun::v0::access::QueryResultFragment::decode(
                            &fragment_bytes[..],
                        )
                    {
                        let validation =
                            validate_query_result_fragment(&fragment, Some(&query.query_id), &[]);
                        if validation.verdict != Verdict::Accept {
                            let reason = validation
                                .reason_code
                                .map(|r| r.as_str())
                                .unwrap_or("query_fragment_invalid");
                            edgerun_log::warn!(
                                "peer query returned invalid result fragment: {}",
                                reason
                            );
                            fetched = false;
                            continue;
                        }

                        // Log any object refs returned
                        for obj_ref in &fragment.object_refs {
                            if !obj_ref.object_id.is_empty() {
                                let obj_id_hex =
                                    edgerun_core::util::bytes_to_hex(&obj_ref.object_id);
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
                        if !fragment.object_refs.is_empty()
                            || fragment.bundled_result_object.is_some()
                        {
                            edgerun_log::info!(
                                "fetch completed: {} object refs, bundled={}",
                                fragment.object_refs.len(),
                                fragment.bundled_result_object.is_some()
                            );
                        }
                    }

                    // Mark as done via store task
                    let (done_tx, done_rx) = edgerun_rt::oneshot::channel();
                    let _ = store_tx
                        .send(StoreRequest::FetchMarkDone {
                            fetch_id: fetch_entry.id,
                            reply_tx: done_tx,
                        })
                        .await;
                    let _ = done_rx.await;
                    break;
                }
                Err(_e) => {
                    edgerun_log::debug!("peer query failed, trying next");
                }
            }
        }

        if !fetched {
            // Re-enqueue with lower priority for retry via store task
            let (requeue_tx, requeue_rx) = edgerun_rt::oneshot::channel();
            let _ = store_tx
                .send(StoreRequest::FetchRequeue {
                    target_type: fetch_type,
                    target_id: fetch_id,
                    priority: fetch_priority - 1,
                    reply_tx: requeue_tx,
                })
                .await;
            let _ = requeue_rx.await;
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

    let mut stream = TcpStream::connect_timeout(&peer_addr.parse()?, Duration::from_secs(10))?;
    stream.set_read_timeout(Some(Duration::from_secs(30)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;

    let cmd_bytes = prost::Message::encode_to_vec(command);
    let frame = encode_tcp_frame(&cmd_bytes);
    stream.write_all(&frame)?;

    let mut header = [0u8; 8];
    stream.read_exact(&mut header)?;

    let payload_len = read_u64_be(&header, 0) as usize;
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
    )
    .await??;

    let cmd_bytes = prost::Message::encode_to_vec(command);
    let frame = encode_tcp_frame(&cmd_bytes);
    edgerun_rt::timeout(std::time::Duration::from_secs(10), stream.write_all(&frame)).await??;

    let mut header = [0u8; 8];
    edgerun_rt::timeout(
        std::time::Duration::from_secs(10),
        stream.read_exact(&mut header),
    )
    .await??;

    let payload_len = read_u64_be(&header, 0) as usize;
    let mut payload = vec![0u8; payload_len];
    edgerun_rt::timeout(
        std::time::Duration::from_secs(30),
        stream.read_exact(&mut payload),
    )
    .await??;

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
    )
    .await??;

    let query_bytes = prost::Message::encode_to_vec(query);
    let frame = encode_tcp_frame(&query_bytes);

    edgerun_rt::timeout(std::time::Duration::from_secs(10), stream.write_all(&frame)).await??;

    // Read response (8-byte length prefix + payload)
    let mut header = [0u8; 8];
    edgerun_rt::timeout(
        std::time::Duration::from_secs(10),
        stream.read_exact(&mut header),
    )
    .await??;

    let payload_len = read_u64_be(&header, 0) as usize;
    let mut payload = vec![0u8; payload_len];
    edgerun_rt::timeout(
        std::time::Duration::from_secs(30),
        stream.read_exact(&mut payload),
    )
    .await??;

    Ok(payload)
}

/// Extracts the initial controller identities.
/// The node's own identity is always the initial controller.
fn config_controllers_from_signer(signer: &dyn MeshSigner) -> Vec<Vec<u8>> {
    vec![signer.node_id().0.to_vec()]
}

pub async fn cmd_run(
    path: &PathBuf,
    listen_addr: Option<SocketAddr>,
    health_port: Option<u16>,
    is_init: bool,
) {
    let yaml = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            edgerun_log::error!(
                "config not found at {}: {}. Run `edgerund init` first.",
                path.display(),
                e
            );
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
    let _signer_type = config
        .signer
        .as_ref()
        .map(|s| s.signer_type.clone())
        .unwrap_or_else(|| "unconfigured".to_string());

    // Determine local assurance class from signer type
    let local_assurance_class: i32 = match config.signer.as_ref().map(|s| s.signer_type.as_str()) {
        Some("tpm") | Some("yubikey") | Some("android-keystore") => 2, // HARDWARE_BACKED
        Some("software") => 1,                                         // SOFTWARE
        _ => 0,                                                        // Unknown/unconfigured
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

    let data_root = path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("data");

    let store_config = NodeStoreConfig {
        data_root: data_root.clone(),
        blob_key_source: Arc::new(BlobKeySource::Software {
            private_key_bytes: private_key_bytes.clone(),
        }),
        node_identity: node_id.0.to_vec(),
    };
    let mut store = NodeStore::open(&store_config).unwrap_or_else(|e| {
        edgerun_log::error!("failed to open storage at {}: {}", data_root.display(), e);
        std::process::exit(1);
    });

    // Initialize resource capacity tracker (reserve 1 core + 512MB for system)
    let capacity = capacity::NodeCapacity::discover();
    let tracker = std::sync::Arc::new(capacity::ResourceTracker::new(
        &capacity,
        1,                 // reserve 1 core for system
        512 * 1024 * 1024, // reserve 512MB for system
    ));
    edgerun_log::info!(
        "capacity: {} cores, {} memory -- available: {} cores, {} memory",
        capacity.total_cores,
        capacity::format_bytes(capacity.total_memory_bytes),
        tracker.available_cores(),
        capacity::format_bytes(tracker.available_memory()),
    );

    // Load workload content policy (optional file next to config)
    let policy_path = path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("workload_policy.txt");
    let workload_policy = match workload_policy::load_policy_file(&policy_path) {
        Ok(p) => {
            if !p.allowed_registries.is_empty()
                || !p.blocked_images.is_empty()
                || !p.pinned_digests.is_empty()
            {
                edgerun_log::info!(
                    "workload policy loaded: {} registries, {} blocked, {} pinned",
                    p.allowed_registries.len(),
                    p.blocked_images.len(),
                    p.pinned_digests.len()
                );
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
            recorded_at: Some(now_prost_timestamp()),
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
        if let Err(e) = command_dispatch::sign_and_append_event(&store, genesis, &*signer).await {
            edgerun_log::error!("failed to sign and write genesis event: {}", e);
            std::process::exit(1);
        }
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
        let (head_seq, _) = head;

        // Validate stream chain integrity on startup
        match store.validate_stream_chain_with_writer(stream_id_bytes, &node_id) {
            Ok(event_count) => {
                edgerun_log::info!("stream chain validated: {} events", event_count);
            }
            Err(e) => {
                edgerun_log::error!("stream chain integrity check failed: {}", e);
                edgerun_log::error!(
                    "DO NOT start this node until the stream is repaired out of band"
                );
                std::process::exit(1);
            }
        }
    }

    let runtime_config = match command_dispatch::project_config(&store, stream_id_bytes, &yaml) {
        Ok(projected) => {
            if projected.allowed_peers != config.allowed_peers
                || projected.bootstrap_peers != config.bootstrap_peers
                || projected.name != config.name
            {
                edgerun_log::info!("runtime config projected from event log");
            }
            projected
        }
        Err(e) => {
            edgerun_log::warn!("failed to project runtime config: {}", e);
            config.clone()
        }
    };

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
                if let Err(e) = capabilities::serve_capabilities_unix(multi_arc, &socket_path_clone)
                {
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
    let allowed_peers: Vec<Vec<u8>> = runtime_config
        .allowed_peers
        .iter()
        .map(|s| s.as_bytes().to_vec())
        .collect();
    if !allowed_peers.is_empty() {
        edgerun_log::info!("peer allowlist active");
    }

    // --- Peer bootstrap (before store is moved) ---
    let bootstrap_peers = parse_bootstrap_peers(&runtime_config.bootstrap_peers);
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

    // Shared stop signal so we can cleanly stop the mesh daemon from the async side
    let mesh_stop_signal = Arc::new(std::sync::atomic::AtomicBool::new(false));

    let store_handle = edgerun_rt::spawn_blocking(move || {
        run_store_task(
            store,
            &stream_id_vec,
            &*store_signer,
            store_rx,
            global_rate_limiter,
            message_hash_cache,
            allowed_peers,
            node_id,
            tracker,
            workload_policy,
            local_assurance_class,
        );
    });

    // Create the MeshDaemon on a separate blocking thread
    let mesh_daemon_node_id = node_id;
    let mesh_stop = mesh_stop_signal.clone();
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
            ) -> Result<
                edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionAccept,
                edgerun_capabilities::CapabilityError,
            > {
                Ok(
                    edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionAccept {
                        version: open.version,
                        session_id: open.session_id.clone(),
                        accepted: true,
                        granted_operations: vec![CapabilityOperation::Query as i32],
                        granted_access_class: open.requested_access_class,
                        error_reason: String::new(),
                        grant_id: open.session_id.clone(),
                    },
                )
            }
            fn invoke(
                &mut self,
                _session_id: &[u8],
                _invocation: &edgerun_proto::edgerun::v0::capability::CapabilityInvocation,
                _inline_parameters: Option<&[u8]>,
            ) -> Result<
                edgerun_remote_capability::RemoteInvocationResult,
                edgerun_capabilities::CapabilityError,
            > {
                Err(edgerun_capabilities::CapabilityError::Unsupported(
                    "use command_handler for mesh commands",
                ))
            }
            fn close_session(
                &mut self,
                _close: &edgerun_proto::edgerun::v0::capability_runtime::CapabilitySessionClose,
            ) -> Result<(), edgerun_capabilities::CapabilityError> {
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

        // Wire up the shared stop signal
        daemon = daemon.with_stop_signal(mesh_stop);

        // Expose the outbound queue for the store task to send commands over mesh
        let outbound = daemon.outbound_queue();
        *bridge_for_daemon.lock().unwrap() = Some(MeshCommandBridge::new(outbound));

        edgerun_log::info!("mesh daemon running with encrypted sessions and command dispatch");

        // Run the daemon's event loop (blocks until stop signal is set)
        if let Err(e) = daemon.run() {
            edgerun_log::error!("mesh daemon error: {}", e);
        }
    });

    // --- CancellationToken for coordinated shutdown ---
    let cancel = CancellationToken::new();

    // --- TCP listener (if configured) ---
    if let Some(addr) = listen_addr {
        let tcp_signer: Arc<dyn edgerun_hardware_signing::MeshSigner + Send + Sync> =
            Arc::clone(&signer);
        let tcp_cancel = cancel.child_token();
        let _tcp_handle = edgerun_rt::spawn(run_tcp_listener(
            addr,
            node_id,
            store_tx.clone(),
            tcp_signer,
            tcp_cancel,
        ));
        edgerun_log::info!("TCP listener started");
    } else {
        edgerun_log::info!("running mesh-only (no TCP listener)");
    }

    // --- Provisioning listener ---
    if let Some(ref signer_cfg) = config.signer {
        if signer_cfg.signer_type == "provisioned"
            && signer_cfg.state.as_ref() == Some(&config::SignerState::Provisioning)
        {
            let provision_cancel = cancel.child_token();
            let _provision_handle = edgerun_rt::spawn(run_provisioning_listener(
                node_id,
                signer_cfg.public_key_hex.clone(),
                signer_cfg.pairing_pin.clone(),
                provision_cancel,
            ));
            edgerun_log::info!("Provisioning listener started on :35630");
        }
    }

    // --- Bootstrap peer connections ---
    if !bootstrap_peers.is_empty() {
        edgerun_log::info!("connecting to bootstrap peers");
        let bootstrap_signer: Arc<dyn edgerun_hardware_signing::MeshSigner + Send + Sync> =
            Arc::clone(&signer);
        let bootstrap_node_id = node_id;
        for peer in &bootstrap_peers {
            let peer_addr = peer.addr.clone();
            let conn_store_tx = store_tx.clone();
            let ctx = SessionContext {
                node_id: bootstrap_node_id,
                signer: Arc::clone(&bootstrap_signer),
            };
            edgerun_rt::spawn(async move {
                match edgerun_rt::ConnectFuture::new(&peer_addr).await {
                    Ok(stream) => {
                        edgerun_log::info!("connected to bootstrap peer");
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
    // All fetch queue operations go through the store task's mpsc channel,
    // eliminating the previous data race from dual FileIndex access.
    let fetch_peers = bootstrap_peers.clone();
    let fetch_node_id = node_id;
    let fetch_signer: Arc<dyn edgerun_hardware_signing::MeshSigner + Send + Sync> =
        Arc::clone(&signer);
    let fetch_store_tx = store_tx.clone();
    let fetch_cancel = cancel.child_token();
    let _fetch_handle = edgerun_rt::spawn(async move {
        run_fetch_queue_consumer(
            fetch_peers,
            fetch_node_id,
            fetch_signer,
            fetch_store_tx,
            fetch_cancel,
        )
        .await;
    });

    // --- Peer reconnection task ---
    let recon_store_tx = store_tx.clone();
    let recon_peers = unreachable_peers;
    let recon_cancel = cancel.child_token();
    let recon_signer: Arc<dyn edgerun_hardware_signing::MeshSigner + Send + Sync> =
        Arc::clone(&signer);
    let recon_node_id = node_id;
    edgerun_rt::spawn(async move {
        run_peer_reconnection(
            recon_peers,
            recon_store_tx,
            recon_cancel,
            recon_signer,
            recon_node_id,
        )
        .await;
    });

    // --- Periodic maintenance timer ---
    // Sends MaintenanceTick to the store task every 60 seconds, ensuring
    // WAL checkpoints, integrity checks, and disk space checks run even
    // during quiet periods with no inbound requests.
    let maint_store_tx = store_tx.clone();
    let maint_cancel = cancel.child_token();
    edgerun_rt::spawn(async move {
        let mut interval = edgerun_rt::interval(std::time::Duration::from_secs(60));
        interval.set_missed_tick_behavior(edgerun_rt::MissedTickBehavior::Skip);
        loop {
            if maint_cancel.is_cancelled() {
                edgerun_log::info!("maintenance timer shutting down");
                return;
            }
            interval.tick().await;
            let _ = maint_store_tx.send(StoreRequest::MaintenanceTick).await;
        }
    });

    // --- Shutdown waiter: wait for signal (init mode) or ctrl_c (normal mode) ---
    let shutdown = edgerun_rt::spawn(async move {
        if is_init {
            // Bare runtime does not have hosted signalfd semantics yet. Keep init
            // mode alive and continue zombie reaping until real signal support lands.
            loop {
                crate::init::reap_zombies();
                edgerun_rt::sleep(std::time::Duration::from_millis(250)).await;
            }
        } else {
            let _ = edgerun_rt::ctrl_c().await;
            edgerun_log::info!("shutdown requested (normal mode)");
        }
    });
    let _ = shutdown.await;

    // --- Coordinated shutdown: cancel all tasks ---
    cancel.cancel();

    edgerun_log::info!("shutting down");

    // Signal the mesh daemon to exit its event loop
    mesh_stop_signal.store(true, std::sync::atomic::Ordering::Release);

    // Signal the store task to gracefully terminate all running workloads
    let _ = store_tx.send(StoreRequest::Shutdown).await;

    // Wait for store task to finish (workloads terminated, channel drained)
    let _ = store_handle.await;
    let _ = mesh_handle.await;
    edgerun_log::info!("shutdown complete");
}
