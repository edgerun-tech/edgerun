use std::sync::Arc;

use edgerun_hardware_signing::{MeshSigner, NodeID};
use edgerun_storage::NodeStore;

use crate::capacity;
use crate::command_dispatch;
use crate::daemon::send_command_to_peer;
use crate::ingress;
use crate::query_engine::{execute_federated_query, execute_query};
use crate::types::{MeshReply, StoreRequest, StoreResponse};
use crate::workload_policy;

pub fn run_store_task(
    mut store: NodeStore,
    stream_id: &[u8],
    signer: &dyn MeshSigner,
    mut rx: edgerun_rt::mpsc::Receiver<StoreRequest>,
    mut global_rate_limiter: ingress::TokenBucket,
    mut message_hash_cache: ingress::RecentHashCache,
    mut allowed_peers: Vec<Vec<u8>>,
    responder_node_id: NodeID,
    capacity_tracker: std::sync::Arc<capacity::ResourceTracker>,
    mut workload_policy: workload_policy::WorkloadPolicy,
    local_assurance_class: i32,
) {
    let rate_limiter = workload_policy::RateLimiter::new(
        100,        // max 100 workloads per requester
        60_000_000, // within a 60-second window
    );
    let running_workloads = std::sync::Arc::new(crate::running_workloads::RunningWorkloads::new());
    // Initialize controller set from the node's config, then replay from
    // the persistent change log so controller state survives restarts.
    let initial_controllers = config_controllers_from_signer(signer);
    let mut controllers =
        command_dispatch::project_controller_set(&store, stream_id, initial_controllers);
    edgerun_log::info!(
        "controller set projected: {} controllers",
        controllers.to_vec().len()
    );
    let mut replay_cache: edgerun_core::collections::HashMap<Vec<u8>, (Vec<u8>, i64)> =
        edgerun_core::collections::HashMap::new();

    // Load active revocations from the database
    let revoked_delegations: edgerun_core::collections::HashSet<Vec<u8>> =
        match store.list_active_revocations() {
            Ok(revocations) => {
                let revoked: edgerun_core::collections::HashSet<Vec<u8>> = revocations
                    .into_iter()
                    .filter(|(typ, _)| typ == "delegation")
                    .filter_map(|(_, hex)| edgerun_core::util::hex_to_bytes(&hex).ok())
                    .collect();
                edgerun_log::info!("loaded active revocations");
                revoked
            }
            Err(e) => {
                edgerun_log::warn!("failed to load revocations: {}", e);
                edgerun_core::collections::HashSet::new()
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
            StoreRequest::Command { raw_bytes, .. } | StoreRequest::Query { raw_bytes, .. } => {
                raw_bytes.clone()
            }
            StoreRequest::ProduceSnapshot { .. }
            | StoreRequest::FetchObject { .. }
            | StoreRequest::SendCommand { .. }
            | StoreRequest::FederatedQuery { .. }
            | StoreRequest::FetchDequeue { .. }
            | StoreRequest::FetchMarkDone { .. }
            | StoreRequest::FetchRequeue { .. }
            | StoreRequest::MaintenanceTick
            | StoreRequest::PeerStatusUpdate { .. }
            | StoreRequest::PeerLookup { .. }
            | StoreRequest::Shutdown
            | StoreRequest::ConfigReload { .. } => Vec::new(),
        };
        let peer_id: Option<Vec<u8>> = match &req {
            StoreRequest::Command { peer_id, .. } | StoreRequest::Query { peer_id, .. } => {
                peer_id.clone()
            }
            StoreRequest::ProduceSnapshot { .. }
            | StoreRequest::FetchObject { .. }
            | StoreRequest::SendCommand { .. }
            | StoreRequest::FederatedQuery { .. }
            | StoreRequest::FetchDequeue { .. }
            | StoreRequest::FetchMarkDone { .. }
            | StoreRequest::FetchRequeue { .. }
            | StoreRequest::MaintenanceTick
            | StoreRequest::PeerStatusUpdate { .. }
            | StoreRequest::PeerLookup { .. }
            | StoreRequest::Shutdown
            | StoreRequest::ConfigReload { .. } => None,
        };

        // ---- Ingress screening (cheap -> expensive) ----
        // ProduceSnapshot is a local trusted operation -- skip screening

        if !matches!(
            &req,
            StoreRequest::ProduceSnapshot { .. }
                | StoreRequest::FetchObject { .. }
                | StoreRequest::SendCommand { .. }
                | StoreRequest::FederatedQuery { .. }
        ) {
            // 1. Recent duplicate detection (before any crypto work)
            let msg_hash = ingress::quick_message_hash(&raw_bytes);
            if message_hash_cache.contains(msg_hash) {
                // For fire-and-forget (mesh) commands with no reply_tx, just skip.
                let rejected = StoreResponse::Rejected(ingress::IngressResult::Duplicate);
                match req {
                    StoreRequest::Command { reply_tx, .. } => {
                        if let Some(tx) = reply_tx {
                            let _ = tx.send(rejected);
                        }
                    }
                    StoreRequest::Query { reply_tx, .. } => {
                        if let Some(tx) = reply_tx {
                            let _ = tx.send(rejected);
                        }
                    }
                    StoreRequest::ProduceSnapshot { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::FetchObject { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::SendCommand { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::FederatedQuery { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::FetchDequeue { reply_tx } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::FetchMarkDone { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::FetchRequeue { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::PeerLookup { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::ConfigReload { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::MaintenanceTick
                    | StoreRequest::PeerStatusUpdate { .. }
                    | StoreRequest::Shutdown => {}
                };
                continue;
            }

            // 2. Global rate limit
            if !global_rate_limiter.try_consume() {
                let rejected = StoreResponse::Rejected(ingress::IngressResult::RateLimited);
                match req {
                    StoreRequest::Command { reply_tx, .. } => {
                        if let Some(tx) = reply_tx {
                            let _ = tx.send(rejected);
                        }
                    }
                    StoreRequest::Query { reply_tx, .. } => {
                        if let Some(tx) = reply_tx {
                            let _ = tx.send(rejected);
                        }
                    }
                    StoreRequest::ProduceSnapshot { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::FetchObject { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::SendCommand { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::FederatedQuery { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::FetchDequeue { reply_tx } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::FetchMarkDone { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::FetchRequeue { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::PeerLookup { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::ConfigReload { reply_tx, .. } => {
                        let _ = reply_tx.send(rejected);
                    }
                    StoreRequest::MaintenanceTick
                    | StoreRequest::PeerStatusUpdate { .. }
                    | StoreRequest::Shutdown => {}
                };
                continue;
            }

            // 3. Peer allowlist check
            if let Some(ref p) = peer_id {
                if !ingress::is_peer_allowed(p, &allowed_peers) {
                    let rejected = StoreResponse::Rejected(ingress::IngressResult::PeerNotAllowed);
                    match req {
                        StoreRequest::Command { reply_tx, .. } => {
                            if let Some(tx) = reply_tx {
                                let _ = tx.send(rejected);
                            }
                        }
                        StoreRequest::Query { reply_tx, .. } => {
                            if let Some(tx) = reply_tx {
                                let _ = tx.send(rejected);
                            }
                        }
                        StoreRequest::ProduceSnapshot { reply_tx, .. } => {
                            let _ = reply_tx.send(rejected);
                        }
                        StoreRequest::FetchObject { reply_tx, .. } => {
                            let _ = reply_tx.send(rejected);
                        }
                        StoreRequest::SendCommand { reply_tx, .. } => {
                            let _ = reply_tx.send(rejected);
                        }
                        StoreRequest::FederatedQuery { reply_tx, .. } => {
                            let _ = reply_tx.send(rejected);
                        }
                        StoreRequest::FetchDequeue { reply_tx } => {
                            let _ = reply_tx.send(rejected);
                        }
                        StoreRequest::FetchMarkDone { reply_tx, .. } => {
                            let _ = reply_tx.send(rejected);
                        }
                        StoreRequest::FetchRequeue { reply_tx, .. } => {
                            let _ = reply_tx.send(rejected);
                        }
                        StoreRequest::PeerLookup { reply_tx, .. } => {
                            let _ = reply_tx.send(rejected);
                        }
                        StoreRequest::ConfigReload { reply_tx, .. } => {
                            let _ = reply_tx.send(rejected);
                        }
                        StoreRequest::MaintenanceTick
                        | StoreRequest::PeerStatusUpdate { .. }
                        | StoreRequest::Shutdown => {}
                    };
                    continue;
                }
            }

            // Screening passed -- cache the hash
            message_hash_cache.insert(msg_hash);
        }

        // 4. Process the request
        match req {
            StoreRequest::Command {
                command, reply_tx, ..
            } => {
                let result = command_dispatch::dispatch_command(
                    &command,
                    &mut store,
                    stream_id,
                    signer,
                    &mut controllers,
                    &mut replay_cache,
                    &revoked_delegations,
                    &trusted_root_ids,
                    local_assurance_class,
                    &capacity_tracker,
                    &workload_policy,
                    &rate_limiter,
                    &running_workloads,
                );
                if let Some(tx) = reply_tx {
                    let _ = tx.send(StoreResponse::Ok(result.response_bytes));
                }
            }
            StoreRequest::Query {
                query,
                raw_bytes,
                reply_tx,
                peer_id,
            } => {
                let query_validation =
                    edgerun_core::validators_proto::validate_query_request_signature(&query);
                if query_validation.verdict != edgerun_core::result::Verdict::Accept {
                    edgerun_log::warn!(
                        "query signature verification failed: {:?}",
                        query_validation.reason_code
                    );
                    if let Some(tx) = reply_tx {
                        let _ =
                            tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                    }
                    continue;
                }
                let result =
                    execute_query(&query, &mut store, stream_id, &responder_node_id, signer);
                if let Some(tx) = reply_tx {
                    let _ = tx.send(StoreResponse::Ok(result));
                }
            }
            StoreRequest::FederatedQuery {
                query,
                remote_fragments,
                trusted_responders,
                reply_tx,
            } => {
                let query_validation =
                    edgerun_core::validators_proto::validate_query_request_signature(&query);
                if query_validation.verdict != edgerun_core::result::Verdict::Accept {
                    edgerun_log::warn!(
                        "federated query signature verification failed: {:?}",
                        query_validation.reason_code
                    );
                    let _ =
                        reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                    continue;
                }
                let result = execute_federated_query(
                    &query,
                    &mut store,
                    stream_id,
                    &responder_node_id,
                    signer,
                    &remote_fragments,
                    &trusted_responders,
                );
                let _ = reply_tx.send(StoreResponse::Ok(result));
            }
            StoreRequest::ProduceSnapshot {
                view_type,
                completeness,
                reply_tx,
            } => match store.produce_snapshot(signer, &view_type, completeness) {
                Ok(descriptor) => {
                    edgerun_log::info!("snapshot produced");
                    let result_bytes = prost::Message::encode_to_vec(&descriptor);
                    let _ = reply_tx.send(StoreResponse::Ok(result_bytes));
                }
                Err(e) => {
                    edgerun_log::error!("snapshot production failed: {}", e);
                    let _ =
                        reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                }
            },
            StoreRequest::FetchObject {
                object_ref,
                reply_tx,
            } => match store.get_object(&object_ref) {
                Ok(Some(result)) => {
                    edgerun_log::info!("object fetched");
                    let _ = reply_tx.send(StoreResponse::Ok(result.content));
                }
                Ok(None) => {
                    edgerun_log::warn!("object not found for fetch");
                    let _ =
                        reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                }
                Err(e) => {
                    edgerun_log::error!("object fetch failed: {}", e);
                    let _ =
                        reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                }
            },
            StoreRequest::SendCommand {
                peer_addr,
                command,
                reply_tx,
            } => match send_command_to_peer(&peer_addr, &command, &mut store, stream_id, signer) {
                Ok(response) => {
                    edgerun_log::info!("outbound command succeeded");
                    let _ = reply_tx.send(StoreResponse::Ok(response));
                }
                Err(_e) => {
                    edgerun_log::error!("outbound command failed");
                    let _ =
                        reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                }
            },
            StoreRequest::FetchDequeue { reply_tx } => match store.dequeue_fetch_for_remote() {
                Ok(Some(entry)) => {
                    let resp = format!(
                        "{}:{}:{}",
                        entry.target_type, entry.target_id, entry.priority
                    );
                    let _ = reply_tx.send(StoreResponse::Ok(resp.into_bytes()));
                }
                Ok(None) => {
                    let _ = reply_tx.send(StoreResponse::Ok(vec![]));
                }
                Err(e) => {
                    edgerun_log::error!("fetch dequeue failed: {}", e);
                    let _ =
                        reply_tx.send(StoreResponse::Rejected(ingress::IngressResult::RateLimited));
                }
            },
            StoreRequest::FetchMarkDone { fetch_id, reply_tx } => {
                if let Err(e) = store.mark_fetch_done(fetch_id) {
                    edgerun_log::error!("fetch mark done failed: {}", e);
                }
                let _ = reply_tx.send(StoreResponse::Ok(vec![]));
            }
            StoreRequest::FetchRequeue {
                target_type,
                target_id,
                priority,
                reply_tx,
            } => {
                if let Err(e) = store.requeue_fetch(&target_type, &target_id, priority) {
                    edgerun_log::error!("fetch requeue failed: {}", e);
                }
                let _ = reply_tx.send(StoreResponse::Ok(vec![]));
            }
            StoreRequest::MaintenanceTick => {
                run_periodic_maintenance(&mut store);
            }
            StoreRequest::PeerStatusUpdate {
                node_id_hex,
                status,
            } => {
                if let Err(e) = store.update_peer_status(&node_id_hex, &status) {
                    edgerun_log::warn!("failed to update peer status: {}", e);
                }
            }
            StoreRequest::PeerLookup {
                node_id_hex,
                reply_tx,
            } => {
                let addr = store.list_peers().ok().and_then(|peers| {
                    peers
                        .iter()
                        .find(|(nid, _, _, _, _)| nid == &node_id_hex)
                        .and_then(|(_, addr, _, _, _)| addr.clone())
                });
                let _ = reply_tx.send(StoreResponse::Ok(addr.unwrap_or_default().into_bytes()));
            }
            StoreRequest::Shutdown => {
                // Graceful shutdown: terminate all running workloads
                let terminated = running_workloads.terminate_all();
                if !terminated.is_empty() {
                    edgerun_log::info!("signaled {} workloads to terminate", terminated.len());
                }
                // Break out of the loop — the task will exit naturally
                break;
            }
            StoreRequest::ConfigReload { config, reply_tx } => {
                let projected_config = match command_dispatch::project_config_from_base(
                    &store,
                    stream_id,
                    config.clone(),
                ) {
                    Ok(projected) => projected,
                    Err(e) => {
                        edgerun_log::warn!("config reload projection failed: {}", e);
                        config
                    }
                };

                // Reload allowed_peers
                let new_allowed_peers: Vec<Vec<u8>> = projected_config
                    .allowed_peers
                    .iter()
                    .map(|s| s.as_bytes().to_vec())
                    .collect();
                let old_count = allowed_peers.len();
                allowed_peers = new_allowed_peers;
                edgerun_log::info!(
                    "config reload: allowed_peers updated ({} -> {})",
                    old_count,
                    allowed_peers.len()
                );

                // Reload workload policy
                let policy_path = std::path::Path::new("workload_policy.txt");
                if let Ok(p) = workload_policy::load_policy_file(policy_path) {
                    if !p.allowed_registries.is_empty() || !p.blocked_images.is_empty() {
                        workload_policy = p;
                        edgerun_log::info!("config reload: workload policy updated");
                    }
                }

                // Reload controllers from the event log (replay from persistent change log)
                let initial_controllers = config_controllers_from_signer(signer);
                if let Ok(head) = store.get_head(stream_id) {
                    let up_to_seq = head.map(|(seq, _)| seq).unwrap_or(0);
                    let new_controllers = command_dispatch::project_controller_set(
                        &store,
                        stream_id,
                        initial_controllers,
                    );
                    let old_count = controllers.to_vec().len();
                    controllers = new_controllers;
                    edgerun_log::info!(
                        "config reload: controllers updated ({} -> {})",
                        old_count,
                        controllers.to_vec().len()
                    );
                }

                let _ = reply_tx.send(StoreResponse::Ok(b"ok".to_vec()));
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
                    edgerun_log::warn!("index corruption detected and indexes rebuilt");
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
            edgerun_log::warn!("index corruption detected and indexes rebuilt");
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

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_core::protocol::{
        canonical_bytes, IdentityRef, NodeRef, ProtocolRecord, QueryRequest, ScopeDescriptor,
        Signature,
    };
    use edgerun_crypto::p256::ecdsa::SigningKey;
    use edgerun_proto::edgerun::v0::access::QueryClass;
    use edgerun_proto::edgerun::v0::trust::ScopeKind;

    fn test_signing_key() -> SigningKey {
        SigningKey::from_bytes(&[42u8; 32].into()).unwrap()
    }

    fn key_hint(key: &SigningKey) -> Vec<u8> {
        let encoded = key.verifying_key().to_encoded_point(false);
        encoded.as_bytes()[1..65].to_vec()
    }

    fn unsigned_query(key: &SigningKey) -> QueryRequest {
        QueryRequest {
            request_version: 1,
            query_id: b"query-1".to_vec(),
            requester: Some(IdentityRef {
                identity_id: b"requester-1".to_vec(),
                identity_kind: Some(1),
                key_hint: Some(key_hint(key)),
            }),
            target_scope: Some(ScopeDescriptor {
                scope_version: 1,
                scope_kind: ScopeKind::Node as i32,
                target_nodes: vec![NodeRef {
                    node_id: b"target-node".to_vec(),
                }],
                target_streams: vec![],
                target_object_kinds: vec![],
                target_view_types: vec![],
                target_domains: vec![],
                time_bounds: None,
                scope_metadata: None,
            }),
            query_class: QueryClass::Head as i32,
            time_window: None,
            checkpoint_base: None,
            result_limit: Some(10),
            cost_limit: None,
            required_proof_classes: vec![],
            query_payload_object: None,
            signature: None,
        }
    }

    fn signed_query(key: &SigningKey) -> QueryRequest {
        let mut query = unsigned_query(key);
        let canonical = canonical_bytes(&ProtocolRecord::QueryRequest(query.clone()), true);
        let sig = edgerun_core::crypto::sign_canonical_record(
            key,
            edgerun_core::crypto::SIG_DOMAIN_QUERY_REQUEST,
            &canonical,
        )
        .unwrap();
        query.signature = Some(Signature {
            algorithm: 1,
            value: sig,
        });
        query
    }

    #[test]
    fn query_without_signature_is_rejected() {
        let key = test_signing_key();
        let query = unsigned_query(&key);

        let result = edgerun_core::validators_proto::validate_query_request_signature(&query);
        assert_eq!(result.verdict, edgerun_core::result::Verdict::Reject);
        assert_eq!(
            result.reason_code,
            Some(edgerun_core::result::ReasonCode::CryptoInvalid)
        );
    }

    #[test]
    fn query_with_valid_signature_is_accepted() {
        let key = test_signing_key();
        let query = signed_query(&key);

        let result = edgerun_core::validators_proto::validate_query_request_signature(&query);
        assert_eq!(result.verdict, edgerun_core::result::Verdict::Accept);
    }
}
