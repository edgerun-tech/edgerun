use std::sync::Arc;

use edgerun_hardware_signing::{MeshSigner, NodeID};
use edgerun_storage::NodeStore;
use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;

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
            StoreRequest::Query { query, raw_bytes, reply_tx, peer_id } => {
                // Verify query signature if present
                if let Some(ref sig) = query.signature {
                    if let Err(reason) = verify_query_signature(&query, sig) {
                        edgerun_log::warn!("query signature verification failed: {}", reason);
                        let _ = reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                        continue;
                    }
                }
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
            StoreRequest::FetchDequeue { reply_tx } => {
                match store.dequeue_fetch_for_remote() {
                    Ok(Some(entry)) => {
                        let resp = format!("{}:{}:{}", entry.target_type, entry.target_id, entry.priority);
                        let _ = reply_tx.send(StoreResponse::Ok(resp.into_bytes()));
                    }
                    Ok(None) => {
                        let _ = reply_tx.send(StoreResponse::Ok(vec![]));
                    }
                    Err(e) => {
                        edgerun_log::error!("fetch dequeue failed: {}", e);
                        let _ = reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                    }
                }
            }
            StoreRequest::FetchMarkDone { fetch_id, reply_tx } => {
                if let Err(e) = store.mark_fetch_done(fetch_id) {
                    edgerun_log::error!("fetch mark done failed: {}", e);
                }
                let _ = reply_tx.send(StoreResponse::Ok(vec![]));
            }
            StoreRequest::FetchRequeue { target_type, target_id, priority, reply_tx } => {
                if let Err(e) = store.requeue_fetch(&target_type, &target_id, priority) {
                    edgerun_log::error!("fetch requeue failed: {}", e);
                }
                let _ = reply_tx.send(StoreResponse::Ok(vec![]));
            }
            StoreRequest::MaintenanceTick => {
                run_periodic_maintenance(&mut store);
            }
            StoreRequest::PeerStatusUpdate { node_id_hex, status } => {
                if let Err(e) = store.update_peer_status(&node_id_hex, &status) {
                    edgerun_log::warn!("failed to update peer status: {}", e);
                }
            }
            StoreRequest::PeerLookup { node_id_hex, reply_tx } => {
                let addr = store.list_peers().ok()
                    .and_then(|peers| {
                        peers.iter()
                            .find(|(nid, _, _, _, _)| nid == &node_id_hex)
                            .and_then(|(_, addr, _, _, _)| addr.clone())
                    });
                let _ = reply_tx.send(StoreResponse::Ok(addr.unwrap_or_default().into_bytes()));
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

        // Periodic maintenance on request-volume thresholds (busy periods)
        if request_counter.is_multiple_of(50) {
            // WAL checkpoint every 50 requests
            match store.wal_checkpoint() {
                Ok(Some(wal_size)) if wal_size > 1024 * 1024 => {
                    edgerun_log::warn!("WAL file remains large after checkpoint");
                }
                _ => {}
            }
        }
        if request_counter.is_multiple_of(100) {
            // Integrity check every 100 requests
            match store.integrity_check_and_rebuild() {
                Ok(0) => {}
                Ok(_rebuilt) => {
                    edgerun_log::warn!("SQLite corruption detected and indexes rebuilt");
                }
                Err(_e) => {
                    edgerun_log::error!("integrity check and rebuild failed");
                }
            }
        }
        if request_counter.is_multiple_of(200) {
            // Disk space check every 200 requests
            match store.check_disk_space() {
                Ok(()) => {}
                Err(_available_bytes) => {
                    edgerun_log::error!("CRITICAL: disk space critically low, writes may fail");
                }
            }
        }
    }
}

/// Run all periodic maintenance tasks.
/// Called on maintenance ticks to ensure maintenance runs even during quiet periods.
fn run_periodic_maintenance(store: &mut NodeStore) {
    // WAL checkpoint
    match store.wal_checkpoint() {
        Ok(Some(wal_size)) if wal_size > 1024 * 1024 => {
            edgerun_log::warn!("WAL file remains large after checkpoint");
        }
        _ => {}
    }

    // Integrity check
    match store.integrity_check_and_rebuild() {
        Ok(0) => {}
        Ok(_rebuilt) => {
            edgerun_log::warn!("SQLite corruption detected and indexes rebuilt");
        }
        Err(_e) => {
            edgerun_log::error!("integrity check and rebuild failed");
        }
    }

    // Disk space check
    match store.check_disk_space() {
        Ok(()) => {}
        Err(_available_bytes) => {
            edgerun_log::error!("CRITICAL: disk space critically low, writes may fail");
        }
    }
}

/// Extracts the initial controller identities.
/// The node's own identity is always the initial controller.
fn config_controllers_from_signer(signer: &dyn MeshSigner) -> Vec<Vec<u8>> {
    vec![signer.node_id().0.to_vec()]
}

/// Verifies the ECDSA P-256 signature on a QueryRequest.
/// Returns `Ok(())` if the signature is valid, or `Err(reason)` if not.
fn verify_query_signature(
    query: &edgerun_proto::edgerun::v0::access::QueryRequest,
    sig: &edgerun_proto::edgerun::v0::common::Signature,
) -> Result<(), &'static str> {
    if sig.algorithm != 1 {
        return Err("bad_algorithm");
    }
    if sig.value.len() != 64 {
        return Err("bad_signature_length");
    }
    let Some(requester) = &query.requester else {
        return Err("no_requester");
    };
    let Some(key_hint) = &requester.key_hint else {
        return Err("no_key_hint");
    };
    if key_hint.len() != 64 {
        return Err("bad_key_hint");
    }

    // Reconstruct the public key from key_hint
    let mut vk_sec1 = [0u8; 65];
    vk_sec1[0] = 0x04;
    vk_sec1[1..].copy_from_slice(key_hint);
    let vk = match edgerun_crypto::p256::ecdsa::VerifyingKey::from_sec1_bytes(&vk_sec1) {
        Ok(v) => v,
        Err(_) => return Err("bad_public_key"),
    };

    // Canonical signable: clone query, clear signature, encode
    let mut signable = query.clone();
    signable.signature = None;
    let mut canonical = Vec::new();
    prost::Message::encode(&signable, &mut canonical).map_err(|_| "encode_failed")?;
    let digest = edgerun_core::crypto::sha256(&canonical);

    // Verify the ECDSA signature
    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&sig.value);
    let r = edgerun_crypto::p256::FieldBytes::from_slice(&sig_bytes[..32]);
    let s = edgerun_crypto::p256::FieldBytes::from_slice(&sig_bytes[32..]);
    let ecdsa_sig = match edgerun_crypto::p256::ecdsa::Signature::from_scalars(*r, *s) {
        Ok(sig) => sig,
        Err(_) => return Err("invalid_signature"),
    };

    if vk.verify_prehash(digest.as_slice(), &ecdsa_sig).is_err() {
        return Err("invalid_signature");
    }

    Ok(())
}
