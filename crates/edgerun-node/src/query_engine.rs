use std::time::SystemTime;

use edgerun_hardware_signing::{MeshSigner, NodeID};
use edgerun_storage::NodeStore;
use edgerun_core::util::system_time_to_prost;

/// Result of query cost evaluation.
pub enum QueryCostCheck {
    Allowed { max_bytes: Option<usize>, max_results: Option<usize> },
    Denied { reason: &'static str },
}

/// Evaluates the query's cost_limit and returns allowed limits or denial reason.
pub fn check_query_cost(
    query: &edgerun_proto::edgerun::v0::access::QueryRequest,
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
        // No cost limit specified -- use defaults
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
pub fn execute_query(
    query: &edgerun_proto::edgerun::v0::access::QueryRequest,
    store: &mut NodeStore,
    local_stream_id: &[u8],
    responder_node_id: &NodeID,
    signer: &dyn MeshSigner,
) -> Vec<u8> {
    use edgerun_proto::edgerun::v0::access::{QueryClass, QueryResultFragment, ResultCompleteness};
    use edgerun_proto::edgerun::v0::common::{EventRef, ObjectRef};

    // 1. Check cost limits before doing any work
    let (max_bytes, max_results) = match check_query_cost(query) {
        QueryCostCheck::Allowed { max_bytes, max_results } => (max_bytes, max_results),
        QueryCostCheck::Denied { reason } => {
            edgerun_log::warn!("query denied due to cost limits");
            return build_query_denial(query, responder_node_id, reason);
        }
    };

    let query_class = query.query_class;
    let mut event_refs: Vec<EventRef> = Vec::new();
    let mut snapshot_refs: Vec<edgerun_proto::edgerun::v0::common::SnapshotRef> = Vec::new();
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
                            stream_id: edgerun_core::util::hex_to_bytes(&stream_id_hex).unwrap_or_else(|_| stream_id_hex.into_bytes()),
                            seq: seq as u64,
                            event_hash: Some(edgerun_core::protocol::Digest {
                                algorithm: 1,
                                value: hash,
                            }),
                        });
                    }
                }
                Err(_e) => {
                    edgerun_log::warn!("query HEAD failed");
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
                                    stream_id: edgerun_core::util::hex_to_bytes(stream_id_hex).unwrap_or_else(|_| stream_id_hex.clone().into_bytes()),
                                    seq: seq as u64,
                                    event_hash: Some(edgerun_core::protocol::Digest {
                                        algorithm: 1,
                                        value: hash,
                                    }),
                                });
                            }
                        }
                    }
                }
                Err(_e) => {
                    edgerun_log::warn!("query EVENT_RANGE failed");
                    completeness = ResultCompleteness::Partial as i32;
                }
            }
        }

        // Check if specific objects exist
        x if x == QueryClass::ObjectExistence as i32 => {
            if let Some(ref obj_ref) = query.query_payload_object {
                let object_id_hex = edgerun_core::util::bytes_to_hex(&obj_ref.object_id);
                match store.is_object_present(&object_id_hex) {
                    Ok(present) => {
                        if present {
                            object_refs.push(obj_ref.clone());
                        }
                    }
                    Err(_e) => {
                        edgerun_log::warn!("query OBJECT_EXISTENCE failed");
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
                    Err(_e) => {
                        edgerun_log::warn!("query OBJECT_FETCH failed");
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
                        use edgerun_proto::edgerun::v0::common::SnapshotRef;
                        snapshot_refs.push(SnapshotRef {
                            snapshot_id: sid.clone().into_bytes(),
                            object_id: Some(edgerun_core::util::hex_to_bytes(oid_hex).unwrap_or_default()),
                        });
                    }
                    if snaps.is_empty() {
                        completeness = ResultCompleteness::MetadataOnly as i32;
                    }
                }
                Err(_e) => {
                    edgerun_log::warn!("query SNAPSHOT failed");
                    completeness = ResultCompleteness::Partial as i32;
                }
            }
        }

        // Search query: scan event log for matching event types or content
        x if x == QueryClass::Search as i32 => {
            // In v0, search is limited -- we scan stream heads and return refs.
            // A full implementation would index event content.
            match store.list_stream_heads() {
                Ok(heads) => {
                    for (stream_id_hex, seq, hash) in heads {
                        if max_results.map_or(false, |m| event_refs.len() >= m) {
                            completeness = ResultCompleteness::Partial as i32;
                            break;
                        }
                        event_refs.push(EventRef {
                            stream_id: edgerun_core::util::hex_to_bytes(&stream_id_hex).unwrap_or_else(|_| stream_id_hex.into_bytes()),
                            seq: seq as u64,
                            event_hash: Some(edgerun_core::protocol::Digest {
                                algorithm: 1,
                                value: hash,
                            }),
                        });
                    }
                }
                Err(_e) => {
                    edgerun_log::warn!("query SEARCH failed");
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
                        stream_id: edgerun_core::util::hex_to_bytes(&stream_id_hex).unwrap_or_else(|_| stream_id_hex.into_bytes()),
                        seq: seq as u64,
                        event_hash: Some(edgerun_core::protocol::Digest {
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
                    use edgerun_proto::edgerun::v0::common::SnapshotRef;
                    snapshot_refs.push(SnapshotRef {
                        snapshot_id: sid.clone().into_bytes(),
                        object_id: Some(edgerun_core::util::hex_to_bytes(oid_hex).unwrap_or_default()),
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
            edgerun_log::warn!("query class not supported");
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

    let mut fragment = QueryResultFragment {
        fragment_version: 1,
        query_id: query.query_id.clone(),
        responder: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
            identity_id: responder_node_id.0.to_vec(),
            identity_kind: Some(2), // NODE
            key_hint: None,
        }),
        answered_at: Some(system_time_to_prost(SystemTime::now())),
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

    // Sign the query response
    {
        use edgerun_core::protocol::{ProtocolRecord, canonical_bytes};
        let record = ProtocolRecord::QueryResultFragment(fragment.clone());
        let canonical = canonical_bytes(&record, true);
        let digest = edgerun_core::crypto::sha256(&canonical);
        let mut digest_bytes = [0u8; 32];
        digest_bytes.copy_from_slice(&digest);
        if let Ok(sig) = signer.sign_digest(&digest_bytes) {
            fragment.signature = Some(edgerun_core::protocol::Signature {
                algorithm: 1,
                value: sig.to_vec(),
            });
        }
    }

    let fragment_bytes = prost::Message::encode_to_vec(&fragment);

    // Enforce max_total_bytes on the serialized response
    if let Some(max_b) = max_bytes {
        if fragment_bytes.len() > max_b {
            edgerun_log::warn!("query response exceeds max_total_bytes, returning denial");
            return build_query_denial(query, responder_node_id, "response_too_large");
        }
    }

    fragment_bytes
}

/// Builds a denial QueryResultFragment when cost limits are exceeded.
pub fn build_query_denial(
    query: &edgerun_proto::edgerun::v0::access::QueryRequest,
    responder_node_id: &NodeID,
    reason: &str,
) -> Vec<u8> {
    use edgerun_proto::edgerun::v0::access::{QueryResultFragment, ResultCompleteness};

    let fragment = QueryResultFragment {
        fragment_version: 1,
        query_id: query.query_id.clone(),
        responder: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
            identity_id: responder_node_id.0.to_vec(),
            identity_kind: Some(2),
            key_hint: None,
        }),
        answered_at: Some(system_time_to_prost(SystemTime::now())),
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
