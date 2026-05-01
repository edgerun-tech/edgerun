use edgerun_core::util::now_prost_timestamp;
use edgerun_hardware_signing::{MeshSigner, NodeID};
use edgerun_storage::NodeStore;
use prost::Message;

/// Result of query cost evaluation.
pub enum QueryCostCheck {
    Allowed {
        max_bytes: Option<usize>,
        max_results: Option<usize>,
    },
    Denied {
        reason: &'static str,
    },
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
        let bytes = cost_limit
            .max_total_bytes
            .map(|b| b as usize)
            .unwrap_or(DEFAULT_MAX_BYTES);
        let results = cost_limit
            .max_results
            .map(|r| r as usize)
            .unwrap_or(DEFAULT_MAX_RESULTS);
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
        QueryCostCheck::Allowed {
            max_bytes,
            max_results,
        } => (max_bytes, max_results),
        QueryCostCheck::Denied { reason } => {
            edgerun_log::warn!("query denied due to cost limits");
            return build_signed_query_denial(query, responder_node_id, reason, signer);
        }
    };

    let query_class = query.query_class;
    let mut event_refs: Vec<EventRef> = Vec::new();
    let mut snapshot_refs: Vec<edgerun_proto::edgerun::v0::common::SnapshotRef> = Vec::new();
    let mut object_refs: Vec<ObjectRef> = Vec::new();
    let mut completeness = ResultCompleteness::CompleteForLocalKnowledge as i32;

    match query_class {
        // Return all known stream heads
        x if x == QueryClass::Head as i32 => match store.list_stream_heads() {
            Ok(heads) => {
                for (stream_id_hex, seq, hash) in heads {
                    if max_results.is_some_and(|m| event_refs.len() >= m) {
                        completeness = ResultCompleteness::Partial as i32;
                        break;
                    }
                    event_refs.push(EventRef {
                        stream_id: edgerun_core::util::hex_to_bytes(&stream_id_hex)
                            .unwrap_or_else(|_| stream_id_hex.into_bytes()),
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
        },

        // Return events in a range for specified streams
        x if x == QueryClass::EventRange as i32 => {
            match store.list_stream_heads() {
                Ok(heads) => {
                    // Parse time_window filter
                    let time_filter = query.time_window.as_ref().map(|tw| {
                        (
                            tw.not_before.as_ref().map(|t| t.seconds),
                            tw.expires_at.as_ref().map(|t| t.seconds),
                        )
                    });

                    for (stream_id_hex, head_seq, _hash) in &heads {
                        if max_results.is_some_and(|m| event_refs.len() >= m) {
                            completeness = ResultCompleteness::Partial as i32;
                            break;
                        }

                        // Scan events and filter by time_window if present
                        if let Some((not_before_secs, expires_at_secs)) = time_filter {
                            // Filter events by timestamp — scan from recent events backward
                            // to find the range that matches the time window.
                            let filtered = filter_events_by_time(
                                store,
                                stream_id_hex,
                                *head_seq,
                                not_before_secs,
                                expires_at_secs,
                                max_results.map(|m| m - event_refs.len()),
                            );
                            event_refs.extend(filtered);
                            if max_results.is_some_and(|m| event_refs.len() >= m) {
                                completeness = ResultCompleteness::Partial as i32;
                            }
                        } else {
                            // No time filter — return full range
                            let from_seq = 0i64;
                            let to_seq = *head_seq;
                            if let Ok(events) =
                                store.list_event_range(stream_id_hex, from_seq, to_seq)
                            {
                                for (seq, hash, _ver) in events {
                                    if max_results.is_some_and(|m| event_refs.len() >= m) {
                                        completeness = ResultCompleteness::Partial as i32;
                                        break;
                                    }
                                    event_refs.push(EventRef {
                                        stream_id: edgerun_core::util::hex_to_bytes(stream_id_hex)
                                            .unwrap_or_else(|_| stream_id_hex.clone().into_bytes()),
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
        x if x == QueryClass::Snapshot as i32 => match store.list_snapshots() {
            Ok(snaps) => {
                for (sid, oid_hex, _vt, _ph, _pa, _c, _bh) in &snaps {
                    if max_results.is_some_and(|m| snapshot_refs.len() >= m) {
                        completeness = ResultCompleteness::Partial as i32;
                        break;
                    }
                    use edgerun_proto::edgerun::v0::common::SnapshotRef;
                    snapshot_refs.push(SnapshotRef {
                        snapshot_id: sid.clone().into_bytes(),
                        object_id: Some(
                            edgerun_core::util::hex_to_bytes(oid_hex).unwrap_or_default(),
                        ),
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
        },

        // Search query: scan event log for matching event types or content
        x if x == QueryClass::Search as i32 => {
            // In v0, search is limited -- we scan stream heads and return refs.
            // A full implementation would index event content.
            match store.list_stream_heads() {
                Ok(heads) => {
                    for (stream_id_hex, seq, hash) in heads {
                        if max_results.is_some_and(|m| event_refs.len() >= m) {
                            completeness = ResultCompleteness::Partial as i32;
                            break;
                        }
                        event_refs.push(EventRef {
                            stream_id: edgerun_core::util::hex_to_bytes(&stream_id_hex)
                                .unwrap_or_else(|_| stream_id_hex.into_bytes()),
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
                    if max_results.is_some_and(|m| event_refs.len() >= m) {
                        completeness = ResultCompleteness::Partial as i32;
                        break;
                    }
                    event_refs.push(EventRef {
                        stream_id: edgerun_core::util::hex_to_bytes(&stream_id_hex)
                            .unwrap_or_else(|_| stream_id_hex.into_bytes()),
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
                    if max_results.is_some_and(|m| snapshot_refs.len() >= m) {
                        completeness = ResultCompleteness::Partial as i32;
                        break;
                    }
                    use edgerun_proto::edgerun::v0::common::SnapshotRef;
                    snapshot_refs.push(SnapshotRef {
                        snapshot_id: sid.clone().into_bytes(),
                        object_id: Some(
                            edgerun_core::util::hex_to_bytes(oid_hex).unwrap_or_default(),
                        ),
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

    let proof_objects =
        build_query_proof_objects(query, store, &event_refs, &snapshot_refs, &object_refs, signer);

    let mut fragment = QueryResultFragment {
        fragment_version: 1,
        query_id: query.query_id.clone(),
        responder: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
            identity_id: responder_node_id.0.to_vec(),
            identity_kind: Some(2), // NODE
            key_hint: Some(responder_node_id.0.to_vec()),
        }),
        answered_at: Some(now_prost_timestamp()),
        completeness,
        snapshot_refs,
        event_refs,
        object_refs,
        proof_objects,
        omission_reason: String::new(),
        bundled_result_object: None,
        result_metadata: None,
        signature: None,
    };

    // Sign the query response with domain separation
    sign_query_result_fragment(&mut fragment, signer);

    let fragment_bytes = prost::Message::encode_to_vec(&fragment);

    // Enforce max_total_bytes on the serialized response
    if let Some(max_b) = max_bytes {
        if fragment_bytes.len() > max_b {
            edgerun_log::warn!("query response exceeds max_total_bytes, returning denial");
            return build_signed_query_denial(
                query,
                responder_node_id,
                "response_too_large",
                signer,
            );
        }
    }

    fragment_bytes
}

/// Executes a local query and folds validated remote `QueryResultFragment`s
/// into one advisory aggregate response.
///
/// Remote fragments are never treated as local authority. They must be signed,
/// match the query id, and come from `trusted_responders` when that list is
/// non-empty. Accepted fragments are stored as immutable objects and referenced
/// by a `FederatedAggregateDescriptor` object in the aggregate response.
pub fn execute_federated_query(
    query: &edgerun_proto::edgerun::v0::access::QueryRequest,
    store: &mut NodeStore,
    local_stream_id: &[u8],
    responder_node_id: &NodeID,
    signer: &dyn MeshSigner,
    remote_fragments: &[Vec<u8>],
    trusted_responders: &[Vec<u8>],
) -> Vec<u8> {
    use edgerun_core::result::Verdict;
    use edgerun_proto::edgerun::v0::access::{
        FederatedAggregateDescriptor, QueryResultFragment, ResultCompleteness,
    };
    use edgerun_proto::edgerun::v0::common::{IdentityRef, ObjectKind};

    let local_bytes = execute_query(query, store, local_stream_id, responder_node_id, signer);
    let Ok(mut aggregate) = QueryResultFragment::decode(&local_bytes[..]) else {
        return local_bytes;
    };

    let max_remote = query
        .cost_limit
        .as_ref()
        .and_then(|limit| limit.max_federated_responders)
        .map(|limit| limit as usize)
        .unwrap_or(remote_fragments.len());

    let mut input_fragments = Vec::new();
    let mut included_responders = Vec::new();
    let mut accepted = 0usize;
    let mut skipped = 0usize;

    for fragment_bytes in remote_fragments {
        if accepted >= max_remote {
            skipped += 1;
            continue;
        }
        let Ok(fragment) = QueryResultFragment::decode(&fragment_bytes[..]) else {
            skipped += 1;
            continue;
        };
        let validation = edgerun_core::validators_proto::validate_query_result_fragment(
            &fragment,
            Some(&query.query_id),
            trusted_responders,
        );
        if validation.verdict != Verdict::Accept {
            skipped += 1;
            continue;
        }
        if fragment.completeness == ResultCompleteness::Denied as i32 {
            skipped += 1;
            continue;
        }

        merge_fragment_refs(&mut aggregate, &fragment);
        if let Some(responder) = fragment.responder.clone() {
            included_responders.push(responder);
        }
        match store.put_object(
            fragment_bytes,
            ObjectKind::DerivedView as i32,
            &[responder_node_id.0.to_vec()],
        ) {
            Ok(object_ref) => input_fragments.push(object_ref),
            Err(e) => edgerun_log::warn!("failed to store remote query fragment: {}", e),
        }
        accepted += 1;
    }

    if accepted == 0 {
        return local_bytes;
    }

    let summary = build_federated_summary_payload(accepted, skipped, &included_responders);
    let payload_object = store
        .put_object(
            &summary,
            ObjectKind::DerivedView as i32,
            &[responder_node_id.0.to_vec()],
        )
        .ok();

    if let Some(payload_object) = payload_object {
        let descriptor = FederatedAggregateDescriptor {
            descriptor_version: 1,
            aggregate_id: edgerun_core::crypto::domain_hash("edgerun:v0:query-aggregate", &summary),
            source_query_id: query.query_id.clone(),
            aggregator: Some(IdentityRef {
                identity_id: responder_node_id.0.to_vec(),
                identity_kind: Some(2),
                key_hint: Some(responder_node_id.0.to_vec()),
            }),
            aggregated_at: Some(now_prost_timestamp()),
            input_fragments,
            aggregation_policy_object: None,
            payload_object: Some(payload_object),
            signature: None,
        };
        let descriptor_bytes = FederatedAggregateDescriptor::encode_to_vec(&descriptor);
        match store.put_object(
            &descriptor_bytes,
            ObjectKind::DerivedView as i32,
            &[responder_node_id.0.to_vec()],
        ) {
            Ok(mut descriptor_ref) => {
                descriptor_ref.object_kind = Some(ObjectKind::DerivedView as i32);
                aggregate.proof_objects.push(descriptor_ref);
            }
            Err(e) => edgerun_log::warn!("failed to store aggregate descriptor: {}", e),
        }
    }

    enforce_aggregate_result_limits(query, &mut aggregate);
    aggregate.completeness = ResultCompleteness::Partial as i32;
    aggregate.answered_at = Some(now_prost_timestamp());
    aggregate.signature = None;
    sign_query_result_fragment(&mut aggregate, signer);
    let aggregate_bytes = QueryResultFragment::encode_to_vec(&aggregate);
    if let Some(max_b) = query
        .cost_limit
        .as_ref()
        .and_then(|limit| limit.max_total_bytes)
    {
        if aggregate_bytes.len() > max_b as usize {
            edgerun_log::warn!("federated query aggregate exceeds max_total_bytes");
            return build_signed_query_denial(
                query,
                responder_node_id,
                "federated_response_too_large",
                signer,
            );
        }
    }
    aggregate_bytes
}

fn merge_fragment_refs(
    aggregate: &mut edgerun_proto::edgerun::v0::access::QueryResultFragment,
    fragment: &edgerun_proto::edgerun::v0::access::QueryResultFragment,
) {
    for snapshot_ref in &fragment.snapshot_refs {
        if !aggregate.snapshot_refs.iter().any(|existing| {
            existing.snapshot_id == snapshot_ref.snapshot_id
                && existing.object_id == snapshot_ref.object_id
        }) {
            aggregate.snapshot_refs.push(snapshot_ref.clone());
        }
    }
    for event_ref in &fragment.event_refs {
        if !aggregate.event_refs.iter().any(|existing| {
            existing.stream_id == event_ref.stream_id && existing.seq == event_ref.seq
        }) {
            aggregate.event_refs.push(event_ref.clone());
        }
    }
    for object_ref in &fragment.object_refs {
        if !aggregate
            .object_refs
            .iter()
            .any(|existing| existing.object_id == object_ref.object_id)
        {
            aggregate.object_refs.push(object_ref.clone());
        }
    }
    for proof_object in &fragment.proof_objects {
        if !aggregate
            .proof_objects
            .iter()
            .any(|existing| existing.object_id == proof_object.object_id)
        {
            aggregate.proof_objects.push(proof_object.clone());
        }
    }
}

fn enforce_aggregate_result_limits(
    query: &edgerun_proto::edgerun::v0::access::QueryRequest,
    aggregate: &mut edgerun_proto::edgerun::v0::access::QueryResultFragment,
) {
    let mut limit = query.result_limit.map(|value| value as usize);
    if let Some(cost_limit) = query
        .cost_limit
        .as_ref()
        .and_then(|cost_limit| cost_limit.max_results)
        .map(|value| value as usize)
    {
        limit = Some(limit.map_or(cost_limit, |current| current.min(cost_limit)));
    }

    let Some(limit) = limit else {
        return;
    };

    let mut truncated = false;
    if aggregate.event_refs.len() > limit {
        aggregate.event_refs.truncate(limit);
        truncated = true;
    }
    if aggregate.object_refs.len() > limit {
        aggregate.object_refs.truncate(limit);
        truncated = true;
    }
    if aggregate.snapshot_refs.len() > limit {
        aggregate.snapshot_refs.truncate(limit);
        truncated = true;
    }
    if truncated {
        aggregate.completeness =
            edgerun_proto::edgerun::v0::access::ResultCompleteness::Partial as i32;
    }
}

fn build_federated_summary_payload(
    accepted: usize,
    skipped: usize,
    responders: &[edgerun_proto::edgerun::v0::common::IdentityRef],
) -> Vec<u8> {
    let responder_ids = responders
        .iter()
        .map(|responder| {
            format!(
                "\"{}\"",
                edgerun_core::util::bytes_to_hex(&responder.identity_id)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"accepted_fragments\":{},\"skipped_fragments\":{},\"responders\":[{}]}}",
        accepted, skipped, responder_ids
    )
    .into_bytes()
}

fn sign_query_result_fragment(
    fragment: &mut edgerun_proto::edgerun::v0::access::QueryResultFragment,
    signer: &dyn MeshSigner,
) {
    use edgerun_core::crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT;
    use edgerun_core::protocol::{canonical_bytes, ProtocolRecord};

    let record = ProtocolRecord::QueryResultFragment(fragment.clone());
    let canonical = canonical_bytes(&record, true);
    if let Ok(sig) = signer.sign_record(SIG_DOMAIN_QUERY_RESULT_FRAGMENT, &canonical) {
        fragment.signature = Some(edgerun_core::protocol::Signature {
            algorithm: 1,
            value: sig.to_vec(),
        });
    }
}

fn build_query_proof_objects(
    query: &edgerun_proto::edgerun::v0::access::QueryRequest,
    store: &mut NodeStore,
    event_refs: &[edgerun_proto::edgerun::v0::common::EventRef],
    snapshot_refs: &[edgerun_proto::edgerun::v0::common::SnapshotRef],
    object_refs: &[edgerun_proto::edgerun::v0::common::ObjectRef],
    signer: &dyn MeshSigner,
) -> Vec<edgerun_proto::edgerun::v0::common::ObjectRef> {
    use edgerun_core::validators::{
        validate_event_set_proof, validate_object_assertion_proof, validate_snapshot_set_proof,
        ProofStructuralResult,
    };
    use edgerun_proto::edgerun::v0::access::{
        EventSetProof, ObjectAssertionProof, ProofClass, SnapshotSetProof, StreamHeadsProof,
    };
    use edgerun_proto::edgerun::v0::common::{HeadRef, ObjectKind};

    let mut proof_objects = Vec::new();

    if query
        .required_proof_classes
        .contains(&(ProofClass::StreamHead as i32))
    {
        let proof = StreamHeadsProof {
            source_query_id: query.query_id.clone(),
            heads: event_refs
                .iter()
                .map(|event_ref| HeadRef {
                    stream_id: event_ref.stream_id.clone(),
                    seq: event_ref.seq,
                    event_hash: event_ref.event_hash.clone(),
                })
                .collect(),
        };
        store_proof_object(
            store,
            signer,
            &prost::Message::encode_to_vec(&proof),
            &mut proof_objects,
        );
    }

    if query
        .required_proof_classes
        .contains(&(ProofClass::SnapshotBase as i32))
    {
        let proof = SnapshotSetProof {
            source_query_id: query.query_id.clone(),
            snapshots: snapshot_refs.to_vec(),
        };
        if validate_snapshot_set_proof(&proof) == ProofStructuralResult::Valid {
            store_proof_object(
                store,
                signer,
                &prost::Message::encode_to_vec(&proof),
                &mut proof_objects,
            );
        }
    }

    if query
        .required_proof_classes
        .contains(&(ProofClass::EventRef as i32))
    {
        let proof = EventSetProof {
            source_query_id: query.query_id.clone(),
            events: event_refs.to_vec(),
            related_objects: object_refs.to_vec(),
        };
        if validate_event_set_proof(&proof) == ProofStructuralResult::Valid {
            store_proof_object(
                store,
                signer,
                &prost::Message::encode_to_vec(&proof),
                &mut proof_objects,
            );
        }
    }

    if query
        .required_proof_classes
        .contains(&(ProofClass::ObjectRef as i32))
    {
        if let Some(object_ref) = &query.query_payload_object {
            let exists = object_refs
                .iter()
                .any(|candidate| candidate.object_id == object_ref.object_id);
            let proof = ObjectAssertionProof {
                source_query_id: query.query_id.clone(),
                object_ref: Some(object_ref.clone()),
                exists,
                bundled_result_object: None,
            };
            if validate_object_assertion_proof(&proof) == ProofStructuralResult::Valid {
                store_proof_object(
                    store,
                    signer,
                    &prost::Message::encode_to_vec(&proof),
                    &mut proof_objects,
                );
            }
        }
    }

    for proof_object in &mut proof_objects {
        proof_object.object_kind = Some(ObjectKind::Proof as i32);
    }

    proof_objects
}

fn store_proof_object(
    store: &mut NodeStore,
    signer: &dyn MeshSigner,
    proof_bytes: &[u8],
    proof_objects: &mut Vec<edgerun_proto::edgerun::v0::common::ObjectRef>,
) {
    use edgerun_proto::edgerun::v0::common::ObjectKind;

    match store.put_object(proof_bytes, ObjectKind::Proof as i32, &[signer.node_id().0.to_vec()]) {
        Ok(object_ref) => proof_objects.push(object_ref),
        Err(e) => edgerun_log::warn!("failed to store query proof object: {}", e),
    }
}

/// Scan events in a stream and return those whose `recorded_at` falls within
/// `[not_before, expires_at)`. Scans from the head backward for efficiency.
fn filter_events_by_time(
    store: &mut NodeStore,
    stream_id_hex: &str,
    head_seq: i64,
    not_before: Option<i64>,
    expires_at: Option<i64>,
    max_results: Option<usize>,
) -> Vec<edgerun_proto::edgerun::v0::common::EventRef> {
    use edgerun_core::protocol::EventEnvelope;
    use edgerun_proto::edgerun::v0::common::EventRef;
    use edgerun_proto::edgerun::v0::stream as proto_stream;
    use prost::Message;

    let mut result = Vec::new();

    // Get the full event list for this stream
    let Ok(events) = store.list_event_range(stream_id_hex, 0, head_seq) else {
        return result;
    };

    // Build stream_id bytes
    let stream_id_bytes = edgerun_core::util::hex_to_bytes(stream_id_hex)
        .unwrap_or_else(|_| stream_id_hex.as_bytes().to_vec());

    // Scan events in order (oldest to newest)
    for (seq, hash, _ver) in events {
        if let Some(max) = max_results {
            if result.len() >= max {
                break;
            }
        }

        // Fetch the event to read its timestamp
        let Ok(Some(event)) = store.get_event(&stream_id_bytes, seq as u64) else {
            continue;
        };

        let recorded_at_secs = event.recorded_at.as_ref().map(|t| t.seconds).unwrap_or(0);

        // Check not_before (inclusive)
        if let Some(nb) = not_before {
            if recorded_at_secs < nb {
                continue;
            }
        }

        // Check expires_at (exclusive)
        if let Some(ea) = expires_at {
            if recorded_at_secs >= ea {
                continue;
            }
        }

        result.push(EventRef {
            stream_id: stream_id_bytes.clone(),
            seq: seq as u64,
            event_hash: Some(edgerun_core::protocol::Digest {
                algorithm: 1,
                value: hash,
            }),
        });
    }

    result
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
            key_hint: Some(responder_node_id.0.to_vec()),
        }),
        answered_at: Some(now_prost_timestamp()),
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

fn build_signed_query_denial(
    query: &edgerun_proto::edgerun::v0::access::QueryRequest,
    responder_node_id: &NodeID,
    reason: &str,
    signer: &dyn MeshSigner,
) -> Vec<u8> {
    use edgerun_proto::edgerun::v0::access::{QueryResultFragment, ResultCompleteness};

    let mut fragment = QueryResultFragment {
        fragment_version: 1,
        query_id: query.query_id.clone(),
        responder: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
            identity_id: responder_node_id.0.to_vec(),
            identity_kind: Some(2),
            key_hint: Some(responder_node_id.0.to_vec()),
        }),
        answered_at: Some(now_prost_timestamp()),
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
    sign_query_result_fragment(&mut fragment, signer);

    prost::Message::encode_to_vec(&fragment)
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
    use edgerun_hardware_signing::{HardwareSigningError, MeshSigner};
    use edgerun_proto::edgerun::v0::access::{
        CostLimit, ProofClass, QueryClass, QueryRequest, QueryResultFragment, ResultCompleteness,
        StreamHeadsProof,
    };
    use edgerun_storage::{BlobKeySource, NodeStoreConfig};
    use prost::Message;
    use std::sync::Arc;

    struct TestSigner {
        node_id: NodeID,
        key: edgerun_crypto::p256::ecdsa::SigningKey,
    }

    impl TestSigner {
        fn new() -> Self {
            let key =
                edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
            let encoded = key.verifying_key().to_encoded_point(false);
            let mut node_id = [0u8; 64];
            node_id.copy_from_slice(&encoded.as_bytes()[1..65]);
            Self {
                node_id: NodeID(node_id),
                key,
            }
        }
    }

    impl MeshSigner for TestSigner {
        fn node_id(&self) -> NodeID {
            self.node_id
        }

        fn sign_digest(&self, digest: &[u8; 32]) -> Result<[u8; 64], HardwareSigningError> {
            let sig: edgerun_crypto::p256::ecdsa::Signature =
                self.key.sign_prehash(digest).map_err(|e| {
                    edgerun_hardware_signing::HardwareSigningError::Provider(e.to_string())
                })?;
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&sig.to_bytes());
            Ok(bytes)
        }
    }

    fn tmp_data_root() -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("query_engine_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn test_store(node_id: NodeID) -> NodeStore {
        let config = NodeStoreConfig {
            data_root: tmp_data_root(),
            blob_key_source: Arc::new(BlobKeySource::Software {
                private_key_bytes: vec![0x42; 32],
            }),
            node_identity: node_id.0.to_vec(),
        };
        NodeStore::open(&config).unwrap()
    }

    #[test]
    fn head_query_with_required_stream_head_proof_returns_proof_object() {
        let signer = TestSigner::new();
        let mut store = test_store(signer.node_id());
        let query = QueryRequest {
            request_version: 1,
            query_id: b"query-proof".to_vec(),
            requester: None,
            target_scope: None,
            query_class: QueryClass::Head as i32,
            time_window: None,
            checkpoint_base: None,
            result_limit: None,
            cost_limit: None,
            required_proof_classes: vec![ProofClass::StreamHead as i32],
            query_payload_object: None,
            signature: None,
        };

        let bytes = execute_query(&query, &mut store, b"stream-1", &signer.node_id(), &signer);
        let fragment = QueryResultFragment::decode(&bytes[..]).unwrap();

        assert_eq!(fragment.proof_objects.len(), 1);
        let proof_object = store
            .get_object(&fragment.proof_objects[0])
            .unwrap()
            .expect("stored proof object");
        let proof = StreamHeadsProof::decode(&proof_object.content[..]).unwrap();
        assert_eq!(proof.source_query_id, b"query-proof");
    }

    #[test]
    fn oversized_query_response_returns_signed_denial() {
        let signer = TestSigner::new();
        let mut store = test_store(signer.node_id());
        let query = QueryRequest {
            request_version: 1,
            query_id: b"query-denial".to_vec(),
            requester: None,
            target_scope: None,
            query_class: QueryClass::Head as i32,
            time_window: None,
            checkpoint_base: None,
            result_limit: None,
            cost_limit: Some(CostLimit {
                max_results: None,
                max_total_bytes: Some(1),
                max_wall_time: None,
                max_federated_responders: None,
            }),
            required_proof_classes: vec![],
            query_payload_object: None,
            signature: None,
        };

        let bytes = execute_query(&query, &mut store, b"stream-1", &signer.node_id(), &signer);
        let fragment = QueryResultFragment::decode(&bytes[..]).unwrap();

        assert_eq!(fragment.completeness, ResultCompleteness::Denied as i32);
        assert_eq!(fragment.omission_reason, "response_too_large");
        assert!(fragment.signature.is_some());
    }

    #[test]
    fn federated_query_merges_valid_remote_fragments() {
        let local_signer = TestSigner::new();
        let remote_signer = TestSigner::new();
        let mut store = test_store(local_signer.node_id());
        let query = QueryRequest {
            request_version: 1,
            query_id: b"federated-query".to_vec(),
            requester: None,
            target_scope: None,
            query_class: QueryClass::Head as i32,
            time_window: None,
            checkpoint_base: None,
            result_limit: None,
            cost_limit: Some(CostLimit {
                max_results: None,
                max_total_bytes: None,
                max_wall_time: None,
                max_federated_responders: Some(1),
            }),
            required_proof_classes: vec![],
            query_payload_object: None,
            signature: None,
        };

        let mut remote_fragment = QueryResultFragment {
            fragment_version: 1,
            query_id: b"federated-query".to_vec(),
            responder: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: remote_signer.node_id().0.to_vec(),
                identity_kind: Some(2),
                key_hint: Some(remote_signer.node_id().0.to_vec()),
            }),
            answered_at: Some(now_prost_timestamp()),
            completeness: ResultCompleteness::CompleteForLocalKnowledge as i32,
            snapshot_refs: vec![],
            event_refs: vec![edgerun_proto::edgerun::v0::common::EventRef {
                stream_id: b"remote-stream".to_vec(),
                seq: 7,
                event_hash: Some(edgerun_core::protocol::Digest {
                    algorithm: 1,
                    value: vec![9; 32],
                }),
            }],
            object_refs: vec![],
            proof_objects: vec![],
            omission_reason: String::new(),
            bundled_result_object: None,
            result_metadata: None,
            signature: None,
        };
        sign_query_result_fragment(&mut remote_fragment, &remote_signer);

        let aggregate_bytes = execute_federated_query(
            &query,
            &mut store,
            b"local-stream",
            &local_signer.node_id(),
            &local_signer,
            &[QueryResultFragment::encode_to_vec(&remote_fragment)],
            &[remote_signer.node_id().0.to_vec()],
        );
        let aggregate = QueryResultFragment::decode(&aggregate_bytes[..]).unwrap();

        assert!(aggregate
            .event_refs
            .iter()
            .any(|event| event.stream_id == b"remote-stream" && event.seq == 7));
        assert!(!aggregate.proof_objects.is_empty());
        assert!(aggregate.signature.is_some());
    }

    #[test]
    fn federated_query_enforces_result_and_byte_limits_after_merge() {
        let local_signer = TestSigner::new();
        let remote_signer = TestSigner::new();
        let mut store = test_store(local_signer.node_id());
        let mut query = QueryRequest {
            request_version: 1,
            query_id: b"federated-limits".to_vec(),
            requester: None,
            target_scope: None,
            query_class: QueryClass::Head as i32,
            time_window: None,
            checkpoint_base: None,
            result_limit: Some(1),
            cost_limit: Some(CostLimit {
                max_results: Some(1),
                max_total_bytes: None,
                max_wall_time: None,
                max_federated_responders: Some(1),
            }),
            required_proof_classes: vec![],
            query_payload_object: None,
            signature: None,
        };

        let mut remote_fragment = QueryResultFragment {
            fragment_version: 1,
            query_id: b"federated-limits".to_vec(),
            responder: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: remote_signer.node_id().0.to_vec(),
                identity_kind: Some(2),
                key_hint: Some(remote_signer.node_id().0.to_vec()),
            }),
            answered_at: Some(now_prost_timestamp()),
            completeness: ResultCompleteness::CompleteForLocalKnowledge as i32,
            snapshot_refs: vec![],
            event_refs: vec![
                edgerun_proto::edgerun::v0::common::EventRef {
                    stream_id: b"remote-stream-a".to_vec(),
                    seq: 1,
                    event_hash: Some(edgerun_core::protocol::Digest {
                        algorithm: 1,
                        value: vec![1; 32],
                    }),
                },
                edgerun_proto::edgerun::v0::common::EventRef {
                    stream_id: b"remote-stream-b".to_vec(),
                    seq: 2,
                    event_hash: Some(edgerun_core::protocol::Digest {
                        algorithm: 1,
                        value: vec![2; 32],
                    }),
                },
            ],
            object_refs: vec![],
            proof_objects: vec![],
            omission_reason: String::new(),
            bundled_result_object: None,
            result_metadata: None,
            signature: None,
        };
        sign_query_result_fragment(&mut remote_fragment, &remote_signer);

        let aggregate_bytes = execute_federated_query(
            &query,
            &mut store,
            b"local-stream",
            &local_signer.node_id(),
            &local_signer,
            &[QueryResultFragment::encode_to_vec(&remote_fragment)],
            &[remote_signer.node_id().0.to_vec()],
        );
        let aggregate = QueryResultFragment::decode(&aggregate_bytes[..]).unwrap();
        assert_eq!(aggregate.event_refs.len(), 1);

        query.cost_limit.as_mut().unwrap().max_total_bytes = Some(1);
        let denial_bytes = execute_federated_query(
            &query,
            &mut store,
            b"local-stream",
            &local_signer.node_id(),
            &local_signer,
            &[QueryResultFragment::encode_to_vec(&remote_fragment)],
            &[remote_signer.node_id().0.to_vec()],
        );
        let denial = QueryResultFragment::decode(&denial_bytes[..]).unwrap();
        assert_eq!(denial.completeness, ResultCompleteness::Denied as i32);
        assert_eq!(denial.omission_reason, "federated_response_too_large");
    }

    #[test]
    fn empty_snapshot_and_event_proofs_are_not_stored() {
        let signer = TestSigner::new();
        let mut store = test_store(signer.node_id());
        let query = QueryRequest {
            request_version: 1,
            query_id: b"query-empty-proofs".to_vec(),
            requester: None,
            target_scope: None,
            query_class: QueryClass::Head as i32,
            time_window: None,
            checkpoint_base: None,
            result_limit: None,
            cost_limit: None,
            required_proof_classes: vec![
                ProofClass::SnapshotBase as i32,
                ProofClass::EventRef as i32,
            ],
            query_payload_object: None,
            signature: None,
        };

        let proof_objects = build_query_proof_objects(&query, &mut store, &[], &[], &[], &signer);

        assert!(proof_objects.is_empty());
    }
}
