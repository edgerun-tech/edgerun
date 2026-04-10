use std::sync::Arc;

use edgerun_hardware_signing::{MeshSigner, NodeID};
use edgerun_storage::NodeStore;

use crate::capacity;
use crate::command_dispatch;
use crate::daemon::send_command_to_peer;
use crate::ingress;
use crate::query_engine::execute_query;
use crate::types::{StoreRequest, StoreResponse, MeshReply};
use crate::workload_policy;

pub fn run_store_task(
    mut store: NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    mut rx: edgerun_rt::mpsc::Receiver<StoreRequest>,
    mut global_rate_limiter: ingress::TokenBucket,
    mut message_hash_cache: ingress::RecentHashCache,
    allowed_peers: Vec<Vec<u8>>,
    responder_node_id: NodeID,
    capacity_tracker: std::sync::Arc<capacity::ResourceTracker>,
    workload_policy: workload_policy::WorkloadPolicy,
    local_assurance_class: i32,
) {
    let rate_limiter = workload_policy::RateLimiter::new(
        100,    // max 100 workloads per requester
        60_000_000, // within a 60-second window
    );
    let running_workloads = std::sync::Arc::new(crate::running_workloads::RunningWorkloads::new());
    // Initialize controller set from the node's config, then replay from
    // the persistent change log so controller state survives restarts.
    let initial_controllers = config_controllers_from_signer(&*signer);
    let mut controllers = command_dispatch::project_controller_set(
        &store,
        stream_id,
        initial_controllers,
    );
    edgerun_log::info!("controller set projected: {} controllers", controllers.to_vec().len());
    let mut replay_cache: std::collections::HashMap<Vec<u8>, (Vec<u8>, i64)> = std::collections::HashMap::new();

    // Load active revocations from the database
    let revoked_delegations: std::collections::HashSet<Vec<u8>> = match store.list_active_revocations() {
        Ok(revocations) => {
            let revoked: std::collections::HashSet<Vec<u8>> = revocations
                .into_iter()
                .filter(|(typ, _)| typ == "delegation")
                .filter_map(|(_, hex)| edgerun_core::util::hex_to_bytes(&hex).ok())
                .collect();
            edgerun_log::info!("loaded active revocations");
            revoked
        }
        Err(e) => {
            edgerun_log::warn!("failed to load revocations: {}", e);
            std::collections::HashSet::new()
        }
    };

    let trusted_root_ids: Vec<Vec<u8>> = controllers.to_vec();

    let mut request_counter: u64 = 0;

    loop {
        // Receive commands from TCP or mesh (both use the same StoreRequest channel).
        // Nothing happens before the event is stored.
        let req = match rx.blocking_recv() {
            Some(store_req) => store_req,
            None => {
                // Channel closed, shut down
                break;
            }
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

        // ---- Ingress screening (cheap -> expensive) ----
        // ProduceSnapshot is a local trusted operation -- skip screening

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

            // Screening passed -- cache the hash
            message_hash_cache.insert(msg_hash);
        }

        // 4. Process the request
        match req {
            StoreRequest::Command { command, reply_tx, .. } => {
                let result = command_dispatch::dispatch_command(
                    &command, &mut store, stream_id, signer,
                    &mut controllers, &mut replay_cache,
                    &revoked_delegations, &trusted_root_ids,
                    local_assurance_class,
                    &capacity_tracker, &workload_policy, &rate_limiter,
                    &running_workloads,
                );
                let _ = reply_tx.send(StoreResponse::Ok(result.response_bytes));
            }
            StoreRequest::Query { query, reply_tx, .. } => {
                let result = execute_query(&query, &mut store, stream_id, &responder_node_id, &*signer);
                let _ = reply_tx.send(StoreResponse::Ok(result));
            }
            StoreRequest::ProduceSnapshot { view_type, completeness, reply_tx } => {
                match store.produce_snapshot(signer, &view_type, completeness) {
                    Ok(descriptor) => {
                        edgerun_log::info!("snapshot produced");
                        let result_bytes = prost::Message::encode_to_vec(&descriptor);
                        let _ = reply_tx.send(StoreResponse::Ok(result_bytes));
                    }
                    Err(e) => {
                        edgerun_log::error!("snapshot production failed: {}", e);
                        let _ = reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                    }
                }
            }
            StoreRequest::FetchObject { object_ref, reply_tx } => {
                match store.get_object(&object_ref) {
                    Ok(Some(result)) => {
                        edgerun_log::info!("object fetched");
                        let _ = reply_tx.send(StoreResponse::Ok(result.content));
                    }
                    Ok(None) => {
                        edgerun_log::warn!("object not found for fetch");
                        let _ = reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                    }
                    Err(e) => {
                        edgerun_log::error!("object fetch failed: {}", e);
                        let _ = reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                    }
                }
            }
            StoreRequest::SendCommand { peer_addr, command, reply_tx } => {
                match send_command_to_peer(&peer_addr, &command, &mut store, stream_id, signer) {
                    Ok(response) => {
                        edgerun_log::info!("outbound command succeeded");
                        let _ = reply_tx.send(StoreResponse::Ok(response));
                    }
                    Err(_e) => {
                        edgerun_log::error!("outbound command failed");
                        let _ = reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                    }
                }
            }
        }

        // Periodically process the fetch queue (every 10 requests)
        if request_counter.is_multiple_of(10) {
            if let Ok(resolved) = store.process_fetch_queue() {
                if resolved > 0 {
                    edgerun_log::info!("fetch queue processed items");
                }
            }
        }

        // Periodically run integrity check (every 100 requests)
        if request_counter.is_multiple_of(100) {
            match store.integrity_check_and_rebuild() {
                Ok(0) => {} // Healthy
                Ok(_rebuilt) => {
                    edgerun_log::warn!("SQLite corruption detected and indexes rebuilt");
                }
                Err(_e) => {
                    edgerun_log::error!("integrity check and rebuild failed");
                }
            }
        }

        // Periodically checkpoint WAL (every 50 requests)
        if request_counter.is_multiple_of(50) {
            match store.wal_checkpoint() {
                Ok(Some(wal_size)) if wal_size > 1024 * 1024 => {
                    // WAL > 1MB after checkpoint -- log a warning
                    edgerun_log::warn!("WAL file remains large after checkpoint");
                }
                _ => {}
            }
        }

        // Periodically check disk space (every 200 requests)
        if request_counter.is_multiple_of(200) {
            match store.check_disk_space() {
                Ok(()) => {}
                Err(_available_bytes) => {
                    edgerun_log::error!("CRITICAL: disk space critically low, writes may fail");
                }
            }
        }
    }
}

/// Extracts the initial controller identities.
/// The node's own identity is always the initial controller.
fn config_controllers_from_signer(signer: &dyn MeshSigner) -> Vec<Vec<u8>> {
    vec![signer.node_id().0.to_vec()]
}
