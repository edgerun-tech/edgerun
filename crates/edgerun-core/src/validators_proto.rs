//! Protobuf-native protocol validators.
//!
//! This module provides direct validation functions over protobuf-generated types,
//! bridging the gap between the conformance suite (validators.rs, YAML/JSON-based)
//! and the runtime protocol path (command.rs, stream crate).
//!
//! Each validator takes and returns protobuf-native types with the unified
//! ACCEPT / REJECT / DEFER / DUPLICATE outcome model.

use crate::prelude::v1::*;

use crate::protocol::{
    canonical_bytes, AssuranceClaim, ChunkManifest, CommandEnvelope, CommandType, DelegationRecord,
    EventEnvelope, LogicalObjectDescriptor, ProtocolRecord, QueryRequest, QueryResultFragment,
    RevocationRecord, SessionAccept, SessionHello, SnapshotDescriptor, StoredRepresentationHeader,
};
use crate::result::{accept, defer, duplicate, empty_map, reject, ReasonCode, ValidationResult};
use crate::value::Value;

// ---------------------------------------------------------------------------
// Stream append validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates a candidate event for appending to a stream.
///
/// Checks:
/// - Genesis event: seq=0, no prev_hash
/// - Non-genesis: seq = prev_seq + 1, prev_hash matches
/// - Writer identity consistency
/// - Signature verification
/// - Duplicate detection
///
/// This is the protobuf-native counterpart to
/// `validators.rs::validate_stream_append_case`.
pub fn validate_stream_append(
    candidate: &EventEnvelope,
    current_head: Option<&EventEnvelope>,
    writer_key_hint: Option<&[u8; 64]>,
) -> ValidationResult {
    // Structural: required fields
    if candidate.stream_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("empty stream_id".into()),
            empty_map(),
        );
    }

    let is_genesis = candidate.seq == 0;

    if is_genesis {
        if candidate.prev_event_hash.is_some() {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("genesis event must not have prev_hash".into()),
                empty_map(),
            );
        }
        if current_head.is_some() {
            return duplicate(
                ReasonCode::ForkConflict,
                Value::String("genesis already exists".into()),
            );
        }
    } else {
        let Some(head) = current_head else {
            return defer(
                ReasonCode::MissingDependency,
                Value::String("no genesis event found".into()),
            );
        };

        if candidate.seq != head.seq + 1 {
            if candidate.seq <= head.seq {
                let head_hash = compute_event_hash(head);
                let cand_hash = compute_event_hash(candidate);
                if head_hash.value == cand_hash.value {
                    return duplicate(
                        ReasonCode::ReplayDetected,
                        Value::String("exact duplicate event".into()),
                    );
                }
                return reject(
                    ReasonCode::ForkConflict,
                    Value::String(format!(
                        "seq {} already exists with different event",
                        candidate.seq
                    )),
                    empty_map(),
                );
            }
            return defer(
                ReasonCode::MissingDependency,
                Value::String(format!(
                    "missing predecessor: expected seq {}, got {}",
                    head.seq + 1,
                    candidate.seq
                )),
            );
        }

        let expected_hash = compute_event_hash(head);
        if candidate.prev_event_hash.as_ref() != Some(&expected_hash) {
            return reject(
                ReasonCode::CryptoInvalid,
                Value::String("prev_hash mismatch".into()),
                empty_map(),
            );
        }
    }

    // Crypto: signature presence
    if candidate.signature.is_none() {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("missing signature".into()),
            empty_map(),
        );
    }

    // Signature verification (if key is available)
    if let Some(key_bytes) = writer_key_hint {
        if !verify_event_signature(candidate, key_bytes) {
            return reject(
                ReasonCode::CryptoInvalid,
                Value::String("signature verification failed".into()),
                empty_map(),
            );
        }
    }

    // Accept
    let mut derived = std::collections::BTreeMap::new();
    derived.insert("seq".into(), Value::Int(candidate.seq as i64));
    derived.insert(
        "stream_id".into(),
        Value::String(crate::util::bytes_to_hex(&candidate.stream_id)),
    );
    accept(Value::Map(derived), empty_map())
}

fn compute_event_hash(event: &EventEnvelope) -> crate::protocol::Digest {
    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, true);
    let hash = crate::crypto::record_hash(crate::crypto::HASH_DOMAIN_EVENT_ENVELOPE, &canonical);
    crate::protocol::Digest {
        algorithm: 1,
        value: hash.to_vec(),
    }
}

fn verify_event_signature(event: &EventEnvelope, key: &[u8; 64]) -> bool {
    let Some(sig) = &event.signature else {
        return false;
    };

    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return false;
    }

    if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
        return false;
    }

    let vk = match crate::crypto::node_id_to_verifying_key(key) {
        Some(vk) => vk,
        None => return false,
    };

    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, true);
    let record_hash =
        crate::crypto::record_hash(crate::crypto::HASH_DOMAIN_EVENT_ENVELOPE, &canonical);

    let sig_input =
        crate::crypto::signature_input(crate::crypto::SIG_DOMAIN_EVENT_ENVELOPE, &record_hash);

    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;
    let Ok(r): Result<[u8; 32], _> = sig.value[..32].try_into() else {
        return false;
    };
    let Ok(s): Result<[u8; 32], _> = sig.value[32..].try_into() else {
        return false;
    };
    let Ok(ecdsa_sig) = edgerun_crypto::p256::ecdsa::Signature::from_scalars(r, s) else {
        return false;
    };

    vk.verify_prehash(&sig_input, &ecdsa_sig).is_ok()
}

// ---------------------------------------------------------------------------
// Delegation chain validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates a delegation chain without an associated command.
///
/// This is the protobuf-native counterpart to
/// `validators.rs::validate_delegation_case`.
pub fn validate_delegation_chain(
    chain: &[DelegationRecord],
    now_ms: i64,
    revoked_ids: &std::collections::HashSet<Vec<u8>>,
) -> ValidationResult {
    if chain.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("empty delegation chain".into()),
            empty_map(),
        );
    }

    // Check continuity first: each delegation's recipient matches next issuer
    for i in 1..chain.len() {
        let prev_recipient = chain[i - 1]
            .recipient
            .as_ref()
            .map(|r| r.identity_id.clone());
        let curr_issuer = chain[i].issuer.as_ref().map(|r| r.identity_id.clone());
        if prev_recipient != curr_issuer {
            return reject(
                ReasonCode::AuthorityDenied,
                Value::String("delegation chain broken: recipient != next issuer".into()),
                empty_map(),
            );
        }
    }

    for (i, delegation) in chain.iter().enumerate() {
        // Check revocation
        if revoked_ids.contains(&delegation.delegation_id) {
            return reject(
                ReasonCode::RevocationActive,
                Value::String(format!(
                    "delegation {} is revoked",
                    crate::util::bytes_to_hex(&delegation.delegation_id)
                )),
                empty_map(),
            );
        }

        // Check timing: expires_at
        if let Some(ref expires_at) = delegation.expires_at {
            let expires_ms = expires_at.seconds * 1000 + (expires_at.nanos as i64) / 1_000_000;
            if now_ms > expires_ms {
                return reject(
                    ReasonCode::TimeInvalid,
                    Value::String("delegation has expired".into()),
                    empty_map(),
                );
            }
        }

        // Check timing: not_before
        if let Some(ref not_before) = delegation.not_before {
            let not_before_ms = not_before.seconds * 1000 + (not_before.nanos as i64) / 1_000_000;
            if now_ms < not_before_ms {
                return defer(
                    ReasonCode::TimeInvalid,
                    Value::String("delegation not yet valid".into()),
                );
            }
        }

        if !verify_delegation_signature(delegation) {
            return reject(
                ReasonCode::CryptoInvalid,
                Value::String("delegation signature verification failed".into()),
                empty_map(),
            );
        }

        // Attenuation: child must not expand parent's actions
        if i > 0 {
            let parent = &chain[i - 1];
            let Some(parent_cap) = &parent.capability else {
                return reject(
                    ReasonCode::StructuralInvalid,
                    Value::String("parent has no capability".into()),
                    empty_map(),
                );
            };
            let Some(child_cap) = &delegation.capability else {
                return reject(
                    ReasonCode::StructuralInvalid,
                    Value::String("child has no capability".into()),
                    empty_map(),
                );
            };
            let parent_actions: std::collections::HashSet<&String> =
                parent_cap.actions.iter().collect();
            for action in &child_cap.actions {
                if !parent_actions.contains(action) {
                    return reject(
                        ReasonCode::AuthorityDenied,
                        Value::String(format!(
                            "attenuation violated: child adds action '{}'",
                            action
                        )),
                        empty_map(),
                    );
                }
            }
        }
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert("chain_length".into(), Value::Int(chain.len() as i64));
    derived.insert(
        "root_issuer".into(),
        Value::String(crate::util::bytes_to_hex(
            &chain[0]
                .issuer
                .as_ref()
                .map(|i| i.identity_id.clone())
                .unwrap_or_default(),
        )),
    );
    derived.insert(
        "leaf_recipient".into(),
        Value::String(crate::util::bytes_to_hex(
            &chain
                .last()
                .unwrap()
                .recipient
                .as_ref()
                .map(|r| r.identity_id.clone())
                .unwrap_or_default(),
        )),
    );
    accept(Value::Map(derived), empty_map())
}

fn verify_delegation_signature(delegation: &DelegationRecord) -> bool {
    let Some(sig) = &delegation.signature else {
        return false;
    };
    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return false;
    }
    if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
        return false;
    }

    let Some(issuer) = &delegation.issuer else {
        return false;
    };
    let Some(key_hint) = &issuer.key_hint else {
        return false;
    };
    if key_hint.len() != crate::crypto::ECDSA_P256_PUBLIC_KEY_LEN {
        return false;
    }
    let Ok(key_hint): Result<[u8; 64], _> = key_hint.as_slice().try_into() else {
        return false;
    };
    let Some(vk) = crate::crypto::node_id_to_verifying_key(&key_hint) else {
        return false;
    };

    let record = ProtocolRecord::DelegationRecord(delegation.clone());
    let canonical = canonical_bytes(&record, true);
    crate::crypto::verify_canonical_record(
        &vk,
        crate::crypto::SIG_DOMAIN_DELEGATION_RECORD,
        &canonical,
        &sig.value,
    )
}

// ---------------------------------------------------------------------------
// Snapshot validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates a snapshot descriptor.
///
/// This is the protobuf-native counterpart to
/// `validators.rs::validate_snapshot_case`.
pub fn validate_snapshot(
    snapshot: &SnapshotDescriptor,
    trusted_producers: &[Vec<u8>],
) -> ValidationResult {
    // Structural: required fields
    if snapshot.snapshot_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("empty snapshot_id".into()),
            empty_map(),
        );
    }

    if snapshot.producer.is_none() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("missing producer".into()),
            empty_map(),
        );
    }

    if snapshot.payload_object.is_none() {
        return defer(
            ReasonCode::MissingDependency,
            Value::String("snapshot payload object not available".into()),
        );
    }

    // Producer trust
    if !trusted_producers.is_empty() {
        let producer_id = snapshot
            .producer
            .as_ref()
            .map(|p| p.identity_id.clone())
            .unwrap_or_default();
        if !trusted_producers.contains(&producer_id) {
            return reject(
                ReasonCode::AuthorityDenied,
                Value::String("snapshot producer not in trusted set".into()),
                empty_map(),
            );
        }
    }

    if !verify_snapshot_signature(snapshot) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("snapshot signature verification failed".into()),
            empty_map(),
        );
    }

    // Base heads or checkpoints must be present (at least one)
    if snapshot.base_heads.is_empty() && snapshot.base_checkpoints.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("snapshot has no base heads or checkpoints".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "snapshot_id".into(),
        Value::String(crate::util::bytes_to_hex(&snapshot.snapshot_id)),
    );
    derived.insert(
        "producer".into(),
        Value::String(crate::util::bytes_to_hex(
            &snapshot
                .producer
                .as_ref()
                .map(|p| p.identity_id.clone())
                .unwrap_or_default(),
        )),
    );
    derived.insert(
        "view_type".into(),
        Value::String(snapshot.view_type.clone()),
    );
    accept(Value::Map(derived), empty_map())
}

fn verify_snapshot_signature(snapshot: &SnapshotDescriptor) -> bool {
    let Some(sig) = &snapshot.signature else {
        return false;
    };
    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return false;
    }
    if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
        return false;
    }

    let Some(producer) = &snapshot.producer else {
        return false;
    };
    let Some(key_hint) = &producer.key_hint else {
        return false;
    };
    if key_hint.len() != crate::crypto::ECDSA_P256_PUBLIC_KEY_LEN {
        return false;
    }
    let Ok(key_hint): Result<[u8; 64], _> = key_hint.as_slice().try_into() else {
        return false;
    };
    let Some(vk) = crate::crypto::node_id_to_verifying_key(&key_hint) else {
        return false;
    };

    let record = ProtocolRecord::SnapshotDescriptor(snapshot.clone());
    let canonical = canonical_bytes(&record, true);
    crate::crypto::verify_canonical_record(
        &vk,
        crate::crypto::SIG_DOMAIN_SNAPSHOT_DESCRIPTOR,
        &canonical,
        &sig.value,
    )
}

// ---------------------------------------------------------------------------
// Object retrieval validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates the typed object retrieval surface for a descriptor, stored
/// representation header, optional chunk manifest, and available bytes.
pub fn validate_object_retrieval(
    descriptor: Option<&LogicalObjectDescriptor>,
    header: Option<&StoredRepresentationHeader>,
    manifest: Option<&ChunkManifest>,
    logical_bytes: Option<&[u8]>,
    stored_bytes: Option<&[u8]>,
) -> ValidationResult {
    if descriptor.is_none() && header.is_none() && logical_bytes.is_none() {
        return defer(
            ReasonCode::MissingDependency,
            Value::String("object descriptor or bytes not available".into()),
        );
    }

    if let Some(descriptor) = descriptor {
        if descriptor.object_id.is_empty() {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("descriptor object_id is empty".into()),
                empty_map(),
            );
        }

        if let Some(bytes) = logical_bytes {
            let derived =
                crate::crypto::derive_object_id(descriptor.canonicalization_id.as_bytes(), bytes);
            if descriptor.object_id != derived {
                return reject(
                    ReasonCode::ObjectIdMismatch,
                    Value::String("descriptor object_id does not match logical bytes".into()),
                    empty_map(),
                );
            }
            if let Some(canonical_digest) = &descriptor.canonical_digest {
                if canonical_digest.value != crate::crypto::sha256(bytes) {
                    return reject(
                        ReasonCode::ObjectIdMismatch,
                        Value::String(
                            "descriptor canonical_digest does not match logical bytes".into(),
                        ),
                        empty_map(),
                    );
                }
            }
            if descriptor.canonical_size != bytes.len() as u64 {
                return reject(
                    ReasonCode::ObjectIdMismatch,
                    Value::String("descriptor canonical_size does not match logical bytes".into()),
                    empty_map(),
                );
            }
        }
    }

    if let Some(header) = header {
        let Some(object_ref) = &header.object else {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("representation header missing object ref".into()),
                empty_map(),
            );
        };
        if let Some(descriptor) = descriptor {
            if object_ref.object_id != descriptor.object_id {
                return reject(
                    ReasonCode::ObjectIdMismatch,
                    Value::String("representation header object does not match descriptor".into()),
                    empty_map(),
                );
            }
        }
        if header.stored_size == 0 {
            return reject(
                ReasonCode::RepresentationInvalid,
                Value::String("representation stored_size is zero".into()),
                empty_map(),
            );
        }

        if let Some(bytes) = stored_bytes {
            if header.stored_size != bytes.len() as u64 {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String("representation stored_size does not match stored bytes".into()),
                    empty_map(),
                );
            }
            if let Some(digest) = &header.representation_digest {
                if digest.value != crate::crypto::derive_representation_digest(bytes) {
                    return reject(
                        ReasonCode::RepresentationInvalid,
                        Value::String("representation digest does not match stored bytes".into()),
                        empty_map(),
                    );
                }
            }
        }

        if header.chunking_mode == edgerun_proto::edgerun::v0::object::ChunkingMode::Manifest as i32
            && manifest.is_none()
        {
            return defer(
                ReasonCode::MissingDependency,
                Value::String("chunk manifest not available".into()),
            );
        }
    }

    if let Some(manifest) = manifest {
        let Some(manifest_object) = &manifest.object else {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("chunk manifest missing object ref".into()),
                empty_map(),
            );
        };
        if let Some(descriptor) = descriptor {
            if manifest_object.object_id != descriptor.object_id {
                return reject(
                    ReasonCode::ObjectIdMismatch,
                    Value::String("chunk manifest object does not match descriptor".into()),
                    empty_map(),
                );
            }
        }

        if manifest.chunk_count != manifest.chunk_entries.len() as u32 {
            return reject(
                ReasonCode::RepresentationInvalid,
                Value::String("chunk_count does not match entries".into()),
                empty_map(),
            );
        }
        let total_size: u64 = manifest
            .chunk_entries
            .iter()
            .map(|entry| entry.length)
            .sum();
        if manifest.total_stored_size != total_size {
            return reject(
                ReasonCode::RepresentationInvalid,
                Value::String("manifest total_stored_size does not match entries".into()),
                empty_map(),
            );
        }
        for entry in &manifest.chunk_entries {
            let Some(digest) = &entry.chunk_digest else {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String("chunk entry missing digest".into()),
                    empty_map(),
                );
            };
            if digest.value.len() != 32 {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String("chunk entry digest must be 32 bytes".into()),
                    empty_map(),
                );
            }
        }

        if let Some(header) = header {
            if let Some(representation) = &manifest.representation {
                if representation.representation_id != header.representation_id {
                    return reject(
                        ReasonCode::RepresentationInvalid,
                        Value::String(
                            "manifest representation does not match representation header".into(),
                        ),
                        empty_map(),
                    );
                }
            }
            if manifest.total_stored_size != header.stored_size {
                return reject(
                    ReasonCode::RepresentationInvalid,
                    Value::String(
                        "manifest total size does not match representation header".into(),
                    ),
                    empty_map(),
                );
            }
        }
    }

    let mut derived = std::collections::BTreeMap::new();
    if let Some(descriptor) = descriptor {
        derived.insert(
            "object_id".into(),
            Value::String(crate::util::bytes_to_hex(&descriptor.object_id)),
        );
    }
    derived.insert(
        "validation_level".into(),
        Value::String("proto_object_retrieval_valid".into()),
    );
    accept(Value::Map(derived), empty_map())
}

// ---------------------------------------------------------------------------
// Query request validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Verifies a signed `QueryRequest` before a responder executes it.
pub fn validate_query_request_signature(query: &QueryRequest) -> ValidationResult {
    if query.query_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("query_id is empty".into()),
            empty_map(),
        );
    }

    let Some(sig) = &query.signature else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query missing signature".into()),
            empty_map(),
        );
    };
    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("unsupported query signature algorithm".into()),
            empty_map(),
        );
    }
    if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query signature has invalid length".into()),
            empty_map(),
        );
    }

    let Some(requester) = &query.requester else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("query missing requester".into()),
            empty_map(),
        );
    };
    let Some(key_hint) = &requester.key_hint else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query requester missing key_hint".into()),
            empty_map(),
        );
    };
    if key_hint.len() != crate::crypto::ECDSA_P256_PUBLIC_KEY_LEN {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query requester key_hint has invalid length".into()),
            empty_map(),
        );
    }
    let Ok(key_hint): Result<[u8; 64], _> = key_hint.as_slice().try_into() else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query requester key_hint has invalid length".into()),
            empty_map(),
        );
    };
    let Some(vk) = crate::crypto::node_id_to_verifying_key(&key_hint) else {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query requester key_hint is not a valid public key".into()),
            empty_map(),
        );
    };

    let canonical = canonical_bytes(&ProtocolRecord::QueryRequest(query.clone()), true);
    if !crate::crypto::verify_canonical_record(
        &vk,
        crate::crypto::SIG_DOMAIN_QUERY_REQUEST,
        &canonical,
        &sig.value,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("query signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "query_id".into(),
        Value::String(crate::util::bytes_to_hex(&query.query_id)),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates a signed responder-local `QueryResultFragment`.
///
/// Accepted fragments remain advisory; callers must still verify referenced
/// objects, events, snapshots, and proof objects before treating their contents
/// as authoritative.
pub fn validate_query_result_fragment(
    fragment: &QueryResultFragment,
    expected_query_id: Option<&[u8]>,
    trusted_responders: &[Vec<u8>],
) -> ValidationResult {
    if fragment.fragment_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported QueryResultFragment version".into()),
            empty_map(),
        );
    }
    if fragment.query_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryResultFragment query_id is empty".into()),
            empty_map(),
        );
    }
    if let Some(expected_query_id) = expected_query_id {
        if fragment.query_id.as_slice() != expected_query_id {
            return reject(
                ReasonCode::TargetMismatch,
                Value::String("QueryResultFragment query_id mismatch".into()),
                empty_map(),
            );
        }
    }

    let Some(responder) = &fragment.responder else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryResultFragment missing responder".into()),
            empty_map(),
        );
    };
    if responder.identity_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryResultFragment responder identity is empty".into()),
            empty_map(),
        );
    }
    if !trusted_responders.is_empty()
        && !trusted_responders
            .iter()
            .any(|id| id.as_slice() == responder.identity_id.as_slice())
    {
        return reject(
            ReasonCode::AuthorityDenied,
            Value::String("QueryResultFragment responder is not trusted".into()),
            empty_map(),
        );
    }
    if fragment.answered_at.is_none() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryResultFragment missing answered_at".into()),
            empty_map(),
        );
    }

    let Some(completeness) =
        edgerun_proto::edgerun::v0::access::ResultCompleteness::from_i32(fragment.completeness)
    else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryResultFragment has invalid completeness".into()),
            empty_map(),
        );
    };
    if completeness == edgerun_proto::edgerun::v0::access::ResultCompleteness::Unspecified {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryResultFragment completeness is unspecified".into()),
            empty_map(),
        );
    }

    let has_backing = !fragment.snapshot_refs.is_empty()
        || !fragment.event_refs.is_empty()
        || !fragment.object_refs.is_empty()
        || !fragment.proof_objects.is_empty()
        || fragment.bundled_result_object.is_some()
        || fragment.result_metadata.is_some();
    if completeness == edgerun_proto::edgerun::v0::access::ResultCompleteness::Denied {
        if fragment.omission_reason.is_empty() {
            return reject(
                ReasonCode::StructuralInvalid,
                Value::String("denied QueryResultFragment missing omission_reason".into()),
                empty_map(),
            );
        }
    } else if !has_backing {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("QueryResultFragment has no backing references".into()),
            empty_map(),
        );
    }

    if !verify_record_signature(
        &ProtocolRecord::QueryResultFragment(fragment.clone()),
        &fragment.signature,
        responder,
        crate::crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("QueryResultFragment signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "query_id".into(),
        Value::String(crate::util::bytes_to_hex(&fragment.query_id)),
    );
    derived.insert("advisory_only".into(), Value::Bool(true));
    accept(Value::Map(derived), empty_map())
}

// ---------------------------------------------------------------------------
// Control change validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates a controller-set mutation command against current state.
///
/// The v0 `CommandEnvelope` does not define a typed control-change payload.
/// The runtime convention is that control commands carry the target controller
/// identity in `command_id`, falling back to the issuer identity for legacy
/// callers. This validator mirrors that convention and simulates post-state
/// before the node commits the control change to its stream.
pub fn validate_control_change_command(
    command: &CommandEnvelope,
    current_controllers: &[Vec<u8>],
    minimum_controllers: usize,
) -> ValidationResult {
    let Some(command_type) = CommandType::from_i32(command.command_type) else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("unknown command_type".into()),
            empty_map(),
        );
    };

    if !matches!(
        command_type,
        CommandType::AddController | CommandType::RemoveController | CommandType::TransferControl
    ) {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("not a control-change command".into()),
            empty_map(),
        );
    }

    let Some(issuer) = &command.issuer else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("control command missing issuer".into()),
            empty_map(),
        );
    };
    if issuer.identity_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("control command issuer identity is empty".into()),
            empty_map(),
        );
    }

    if !current_controllers.is_empty()
        && !current_controllers
            .iter()
            .any(|id| id.as_slice() == issuer.identity_id.as_slice())
    {
        return reject(
            ReasonCode::AuthorityDenied,
            Value::String("issuer is not a current controller".into()),
            empty_map(),
        );
    }

    let target_controller = control_target_identity(command);
    if target_controller.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("control command missing target controller identity".into()),
            empty_map(),
        );
    }

    let mut post_controllers = current_controllers.to_vec();
    match command_type {
        CommandType::AddController | CommandType::TransferControl => {
            if !post_controllers
                .iter()
                .any(|id| id.as_slice() == target_controller.as_slice())
            {
                post_controllers.push(target_controller.clone());
            }
        }
        CommandType::RemoveController => {
            let Some(index) = post_controllers
                .iter()
                .position(|id| id.as_slice() == target_controller.as_slice())
            else {
                return reject(
                    ReasonCode::ControlInvariantFailed,
                    Value::String("target controller is not installed".into()),
                    empty_map(),
                );
            };
            post_controllers.remove(index);
            if post_controllers.len() < minimum_controllers {
                return reject(
                    ReasonCode::ControlInvariantFailed,
                    Value::String("controller set would fall below minimum".into()),
                    empty_map(),
                );
            }
        }
        _ => unreachable!("control command types matched above"),
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "target_controller".into(),
        Value::String(crate::util::bytes_to_hex(&target_controller)),
    );
    derived.insert(
        "command_type".into(),
        Value::String(command_type.as_str_name().into()),
    );

    let controller_values = post_controllers
        .iter()
        .map(|id| Value::String(crate::util::bytes_to_hex(id)))
        .collect();
    let mut post_state = std::collections::BTreeMap::new();
    post_state.insert("controller_set".into(), Value::Seq(controller_values));

    accept(Value::Map(derived), Value::Map(post_state))
}

fn control_target_identity(command: &CommandEnvelope) -> Vec<u8> {
    if !command.command_id.is_empty() {
        return command.command_id.clone();
    }
    command
        .issuer
        .as_ref()
        .map(|issuer| issuer.identity_id.clone())
        .unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Session handshake validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates a signed `SessionHello` before accepting a peer handshake.
pub fn validate_session_hello(
    hello: &SessionHello,
    local_node_id: Option<&[u8]>,
) -> ValidationResult {
    if hello.message_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported SessionHello version".into()),
            empty_map(),
        );
    }
    let Some(initiator) = &hello.initiator else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SessionHello missing initiator".into()),
            empty_map(),
        );
    };
    if initiator.identity_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SessionHello initiator identity is empty".into()),
            empty_map(),
        );
    }
    if hello.supported_protocol_versions.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SessionHello missing supported protocol versions".into()),
            empty_map(),
        );
    }
    if hello.session_nonce.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SessionHello missing session nonce".into()),
            empty_map(),
        );
    }
    if let (Some(target), Some(local_node_id)) = (&hello.target_node, local_node_id) {
        if target.node_id.as_slice() != local_node_id {
            return reject(
                ReasonCode::TargetMismatch,
                Value::String("SessionHello targets a different node".into()),
                empty_map(),
            );
        }
    }

    if !verify_session_signature(
        &ProtocolRecord::SessionHello(hello.clone()),
        &hello.signature,
        initiator,
        crate::crypto::SIG_DOMAIN_SESSION_HELLO,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("SessionHello signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "initiator".into(),
        Value::String(crate::util::bytes_to_hex(&initiator.identity_id)),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates a signed `SessionAccept` for a previously sent hello nonce.
pub fn validate_session_accept(
    accept_msg: &SessionAccept,
    expected_nonce: &[u8],
    supported_protocol_versions: &[u32],
) -> ValidationResult {
    if accept_msg.message_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported SessionAccept version".into()),
            empty_map(),
        );
    }
    let Some(responder) = &accept_msg.responder else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SessionAccept missing responder".into()),
            empty_map(),
        );
    };
    if responder.identity_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("SessionAccept responder identity is empty".into()),
            empty_map(),
        );
    }
    if accept_msg.echoed_session_nonce != expected_nonce {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("SessionAccept nonce mismatch".into()),
            empty_map(),
        );
    }
    if !supported_protocol_versions.is_empty()
        && !supported_protocol_versions.contains(&accept_msg.selected_protocol_version)
    {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("SessionAccept selected unsupported protocol version".into()),
            empty_map(),
        );
    }

    if !verify_session_signature(
        &ProtocolRecord::SessionAccept(accept_msg.clone()),
        &accept_msg.signature,
        responder,
        crate::crypto::SIG_DOMAIN_SESSION_ACCEPT,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("SessionAccept signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "responder".into(),
        Value::String(crate::util::bytes_to_hex(&responder.identity_id)),
    );
    derived.insert(
        "selected_protocol_version".into(),
        Value::Int(accept_msg.selected_protocol_version as i64),
    );
    accept(Value::Map(derived), empty_map())
}

// ---------------------------------------------------------------------------
// Trust record validation (protobuf-native)
// ---------------------------------------------------------------------------

/// Validates a signed `AssuranceClaim` before using it as trust evidence.
pub fn validate_assurance_claim(
    claim: &AssuranceClaim,
    now_ms: i64,
    trusted_attesters: &[Vec<u8>],
) -> ValidationResult {
    if claim.claim_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported AssuranceClaim version".into()),
            empty_map(),
        );
    }
    if claim.subject.is_none() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("AssuranceClaim missing subject".into()),
            empty_map(),
        );
    }
    if edgerun_proto::edgerun::v0::common::AssuranceClass::from_i32(claim.assurance_class)
        .is_none_or(|class| {
            class == edgerun_proto::edgerun::v0::common::AssuranceClass::Unspecified
        })
    {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("AssuranceClaim has invalid assurance class".into()),
            empty_map(),
        );
    }

    let Some(attester) = &claim.attester else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("AssuranceClaim missing attester".into()),
            empty_map(),
        );
    };
    if attester.identity_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("AssuranceClaim attester identity is empty".into()),
            empty_map(),
        );
    }
    if !trusted_attesters.is_empty()
        && !trusted_attesters
            .iter()
            .any(|id| id.as_slice() == attester.identity_id.as_slice())
    {
        return reject(
            ReasonCode::AuthorityDenied,
            Value::String("AssuranceClaim attester is not trusted".into()),
            empty_map(),
        );
    }

    let Some(issued_at) = &claim.issued_at else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("AssuranceClaim missing issued_at".into()),
            empty_map(),
        );
    };
    let issued_ms = timestamp_ms(issued_at);
    if now_ms < issued_ms {
        return defer(
            ReasonCode::TimeInvalid,
            Value::String("AssuranceClaim issued_at is in the future".into()),
        );
    }
    if let Some(expires_at) = &claim.expires_at {
        if now_ms > timestamp_ms(expires_at) {
            return reject(
                ReasonCode::TimeInvalid,
                Value::String("AssuranceClaim has expired".into()),
                empty_map(),
            );
        }
    }

    if !verify_record_signature(
        &ProtocolRecord::AssuranceClaim(claim.clone()),
        &claim.signature,
        attester,
        crate::crypto::SIG_DOMAIN_ASSURANCE_CLAIM,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("AssuranceClaim signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "attester".into(),
        Value::String(crate::util::bytes_to_hex(&attester.identity_id)),
    );
    derived.insert(
        "assurance_class".into(),
        Value::Int(claim.assurance_class as i64),
    );
    accept(Value::Map(derived), empty_map())
}

/// Validates a signed `RevocationRecord` before indexing or applying it.
pub fn validate_revocation_record(
    revocation: &RevocationRecord,
    now_ms: i64,
    trusted_issuers: &[Vec<u8>],
) -> ValidationResult {
    if revocation.record_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            Value::String("unsupported RevocationRecord version".into()),
            empty_map(),
        );
    }
    if revocation.revocation_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord revocation_id is empty".into()),
            empty_map(),
        );
    }

    let Some(issuer) = &revocation.issuer else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord missing issuer".into()),
            empty_map(),
        );
    };
    if issuer.identity_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord issuer identity is empty".into()),
            empty_map(),
        );
    }
    if !trusted_issuers.is_empty()
        && !trusted_issuers
            .iter()
            .any(|id| id.as_slice() == issuer.identity_id.as_slice())
    {
        return reject(
            ReasonCode::AuthorityDenied,
            Value::String("RevocationRecord issuer is not trusted".into()),
            empty_map(),
        );
    }

    let Some(issued_at) = &revocation.issued_at else {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord missing issued_at".into()),
            empty_map(),
        );
    };
    if now_ms < timestamp_ms(issued_at) {
        return defer(
            ReasonCode::TimeInvalid,
            Value::String("RevocationRecord issued_at is in the future".into()),
        );
    }
    if let Some(effective_at) = &revocation.effective_at {
        if now_ms < timestamp_ms(effective_at) {
            return defer(
                ReasonCode::TimeInvalid,
                Value::String("RevocationRecord is not yet effective".into()),
            );
        }
    }

    if edgerun_proto::edgerun::v0::trust::RevocationKind::from_i32(revocation.revocation_kind)
        .is_none_or(|kind| kind == edgerun_proto::edgerun::v0::trust::RevocationKind::Unspecified)
    {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord has invalid revocation kind".into()),
            empty_map(),
        );
    }
    if revocation.target.is_none() {
        return reject(
            ReasonCode::StructuralInvalid,
            Value::String("RevocationRecord missing target".into()),
            empty_map(),
        );
    }

    if !verify_record_signature(
        &ProtocolRecord::RevocationRecord(revocation.clone()),
        &revocation.signature,
        issuer,
        crate::crypto::SIG_DOMAIN_REVOCATION_RECORD,
    ) {
        return reject(
            ReasonCode::CryptoInvalid,
            Value::String("RevocationRecord signature verification failed".into()),
            empty_map(),
        );
    }

    let mut derived = std::collections::BTreeMap::new();
    derived.insert(
        "revocation_id".into(),
        Value::String(crate::util::bytes_to_hex(&revocation.revocation_id)),
    );
    derived.insert(
        "issuer".into(),
        Value::String(crate::util::bytes_to_hex(&issuer.identity_id)),
    );
    accept(Value::Map(derived), empty_map())
}

fn timestamp_ms(timestamp: &prost_types::Timestamp) -> i64 {
    timestamp.seconds * 1000 + (timestamp.nanos as i64) / 1_000_000
}

fn verify_session_signature(
    record: &ProtocolRecord,
    signature: &Option<crate::protocol::Signature>,
    identity: &crate::protocol::IdentityRef,
    signature_domain: &str,
) -> bool {
    verify_record_signature(record, signature, identity, signature_domain)
}

fn verify_record_signature(
    record: &ProtocolRecord,
    signature: &Option<crate::protocol::Signature>,
    identity: &crate::protocol::IdentityRef,
    signature_domain: &str,
) -> bool {
    let Some(sig) = signature else {
        return false;
    };
    if sig.algorithm != crate::crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32 {
        return false;
    }
    if sig.value.len() != crate::crypto::ECDSA_P256_SIGNATURE_LEN {
        return false;
    }

    let key_bytes = identity
        .key_hint
        .as_deref()
        .filter(|bytes| bytes.len() == crate::crypto::ECDSA_P256_PUBLIC_KEY_LEN)
        .unwrap_or(&identity.identity_id);
    if key_bytes.len() != crate::crypto::ECDSA_P256_PUBLIC_KEY_LEN {
        return false;
    }
    let Ok(key_hint): Result<[u8; 64], _> = key_bytes.try_into() else {
        return false;
    };
    let Some(vk) = crate::crypto::node_id_to_verifying_key(&key_hint) else {
        return false;
    };

    let canonical = canonical_bytes(record, true);
    crate::crypto::verify_canonical_record(&vk, signature_domain, &canonical, &sig.value)
        || crate::crypto::verify_canonical_record_hw(&vk, signature_domain, &canonical, &sig.value)
}

// ---------------------------------------------------------------------------
// Re-exports for convenience
// ---------------------------------------------------------------------------

pub use crate::command::{
    command_hash, validate_command, validate_command_signature, CommandValidationContext,
};

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{Digest, EventEnvelope, EventType, IdentityRef, NodeRef, ObjectRef};

    fn make_event(seq: u64, prev_hash: Option<crate::protocol::Digest>) -> EventEnvelope {
        EventEnvelope {
            envelope_version: 1,
            stream_id: b"test-stream".to_vec(),
            seq,
            prev_event_hash: prev_hash,
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
            signature: Some(crate::protocol::Signature {
                algorithm: 1,
                value: vec![0; 64],
            }),
        }
    }

    fn make_genesis_event() -> EventEnvelope {
        make_event(0, None)
    }

    fn test_node_id(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> Vec<u8> {
        crate::crypto::verifying_key_to_node_id(key.verifying_key()).to_vec()
    }

    fn sign_delegation(
        delegation: &mut DelegationRecord,
        key: &edgerun_crypto::p256::ecdsa::SigningKey,
    ) {
        delegation.signature = None;
        if let Some(issuer) = &mut delegation.issuer {
            issuer.key_hint = Some(test_node_id(key));
        }
        let canonical =
            canonical_bytes(&ProtocolRecord::DelegationRecord(delegation.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_DELEGATION_RECORD,
            &canonical,
        )
        .unwrap();
        delegation.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
    }

    fn sign_snapshot(
        snapshot: &mut SnapshotDescriptor,
        key: &edgerun_crypto::p256::ecdsa::SigningKey,
    ) {
        snapshot.signature = None;
        if let Some(producer) = &mut snapshot.producer {
            producer.key_hint = Some(test_node_id(key));
        }
        let canonical =
            canonical_bytes(&ProtocolRecord::SnapshotDescriptor(snapshot.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_SNAPSHOT_DESCRIPTOR,
            &canonical,
        )
        .unwrap();
        snapshot.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
    }

    #[test]
    fn stream_append_accepts_genesis() {
        let genesis = make_genesis_event();
        let result = validate_stream_append(&genesis, None, None);
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn stream_append_rejects_genesis_with_prev_hash() {
        let mut genesis = make_genesis_event();
        genesis.prev_event_hash = Some(crate::protocol::Digest {
            algorithm: 1,
            value: vec![1; 32],
        });
        let result = validate_stream_append(&genesis, None, None);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn stream_append_defers_without_genesis() {
        let event = make_event(
            1,
            Some(crate::protocol::Digest {
                algorithm: 1,
                value: vec![1; 32],
            }),
        );
        let result = validate_stream_append(&event, None, None);
        assert_eq!(result.verdict, crate::result::Verdict::Defer);
    }

    #[test]
    fn stream_append_defers_missing_predecessor() {
        let genesis = make_genesis_event();
        let event = make_event(
            5,
            Some(crate::protocol::Digest {
                algorithm: 1,
                value: vec![1; 32],
            }),
        );
        let result = validate_stream_append(&event, Some(&genesis), None);
        assert_eq!(result.verdict, crate::result::Verdict::Defer);
    }

    #[test]
    fn stream_append_rejects_prev_hash_mismatch() {
        let genesis = make_genesis_event();
        let event = make_event(
            1,
            Some(crate::protocol::Digest {
                algorithm: 1,
                value: vec![0xFF; 32],
            }),
        );
        let result = validate_stream_append(&event, Some(&genesis), None);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn stream_append_detects_fork() {
        let genesis = make_genesis_event();
        let duplicate_genesis = make_genesis_event();
        let result = validate_stream_append(&duplicate_genesis, Some(&genesis), None);
        assert_eq!(result.verdict, crate::result::Verdict::Duplicate);
        assert_eq!(result.reason_code, Some(ReasonCode::ForkConflict));
    }

    #[test]
    fn delegation_chain_valid_single() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: b"user".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(crate::protocol::Signature {
                algorithm: 1,
                value: vec![1; 64],
            }),
        };
        sign_delegation(&mut deleg, &signing_key);

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn delegation_chain_bad_signature_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut deleg = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: b"user".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: None,
        };
        sign_delegation(&mut deleg, &signing_key);
        if let Some(sig) = &mut deleg.signature {
            sig.value[0] ^= 0xFF;
        }

        let result = validate_delegation_chain(
            &[deleg],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn delegation_chain_revoked() {
        let deleg = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: b"user".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(crate::protocol::Signature {
                algorithm: 1,
                value: vec![1; 64],
            }),
        };

        let mut revoked = std::collections::HashSet::new();
        revoked.insert(b"deleg-1".to_vec());

        let result = validate_delegation_chain(&[deleg], 1_700_000_000_000, &revoked);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RevocationActive));
    }

    #[test]
    fn delegation_chain_broken_continuity() {
        let parent = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"root".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            recipient: Some(IdentityRef {
                identity_id: b"mid".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(crate::protocol::Signature {
                algorithm: 1,
                value: vec![1; 64],
            }),
        };

        let child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"OTHER".to_vec(),
                identity_kind: None,
                key_hint: None,
            }), // != "mid"
            recipient: Some(IdentityRef {
                identity_id: b"user".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(crate::protocol::Signature {
                algorithm: 1,
                value: vec![1; 64],
            }),
        };

        let result = validate_delegation_chain(
            &[parent, child],
            1_700_000_000_000,
            &std::collections::HashSet::new(),
        );
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn snapshot_valid_with_trusted_producer() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut snapshot = SnapshotDescriptor {
            descriptor_version: 1,
            snapshot_id: b"snap-1".to_vec(),
            view_type: "timeline".into(),
            view_version: 1,
            producer: Some(IdentityRef {
                identity_id: b"trusted".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            produced_at: None,
            base_heads: vec![crate::protocol::HeadRef {
                stream_id: b"stream-1".to_vec(),
                seq: 10,
                event_hash: None,
            }],
            base_checkpoints: vec![],
            scope: None,
            completeness: 0,
            payload_object: Some(crate::protocol::ObjectRef {
                object_id: b"obj-1".to_vec(),
                object_kind: None,
            }),
            supersedes: None,
            snapshot_metadata: None,
            signature: None,
        };
        sign_snapshot(&mut snapshot, &signing_key);

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn snapshot_bad_signature_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut snapshot = SnapshotDescriptor {
            descriptor_version: 1,
            snapshot_id: b"snap-1".to_vec(),
            view_type: "timeline".into(),
            view_version: 1,
            producer: Some(IdentityRef {
                identity_id: b"trusted".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            produced_at: None,
            base_heads: vec![crate::protocol::HeadRef {
                stream_id: b"stream-1".to_vec(),
                seq: 10,
                event_hash: None,
            }],
            base_checkpoints: vec![],
            scope: None,
            completeness: 0,
            payload_object: Some(crate::protocol::ObjectRef {
                object_id: b"obj-1".to_vec(),
                object_kind: None,
            }),
            supersedes: None,
            snapshot_metadata: None,
            signature: None,
        };
        sign_snapshot(&mut snapshot, &signing_key);
        if let Some(sig) = &mut snapshot.signature {
            sig.value[0] ^= 0xFF;
        }

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn snapshot_untrusted_producer_rejected() {
        let snapshot = SnapshotDescriptor {
            descriptor_version: 1,
            snapshot_id: b"snap-1".to_vec(),
            view_type: "timeline".into(),
            view_version: 1,
            producer: Some(IdentityRef {
                identity_id: b"untrusted".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            produced_at: None,
            base_heads: vec![crate::protocol::HeadRef {
                stream_id: b"stream-1".to_vec(),
                seq: 10,
                event_hash: None,
            }],
            base_checkpoints: vec![],
            scope: None,
            completeness: 0,
            payload_object: Some(crate::protocol::ObjectRef {
                object_id: b"obj-1".to_vec(),
                object_kind: None,
            }),
            supersedes: None,
            snapshot_metadata: None,
            signature: None,
        };

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn object_retrieval_validates_descriptor_header_and_bytes() {
        let logical = b"hello object";
        let object_id = crate::crypto::derive_object_id(b"raw-bytes-v0", logical);
        let representation_id = crate::crypto::sha256(b"representation-id");
        let descriptor = LogicalObjectDescriptor {
            descriptor_version: 1,
            object_id: object_id.clone(),
            object_kind: 1,
            object_schema_version: 1,
            canonicalization_id: "raw-bytes-v0".into(),
            canonical_digest: Some(Digest {
                algorithm: 1,
                value: crate::crypto::sha256(logical),
            }),
            canonical_size: logical.len() as u64,
            created_at: None,
            producer: None,
            describes_object: None,
            object_metadata: None,
        };
        let header = StoredRepresentationHeader {
            header_version: 1,
            representation_id,
            object: Some(ObjectRef {
                object_id,
                object_kind: Some(1),
            }),
            representation_digest: Some(Digest {
                algorithm: 1,
                value: crate::crypto::derive_representation_digest(logical),
            }),
            plaintext_size: Some(logical.len() as u64),
            stored_size: logical.len() as u64,
            encryption_scheme: String::new(),
            compression_scheme: String::new(),
            chunking_mode: edgerun_proto::edgerun::v0::object::ChunkingMode::None as i32,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: None,
            representation_metadata: None,
        };

        let result = validate_object_retrieval(
            Some(&descriptor),
            Some(&header),
            None,
            Some(logical),
            Some(logical),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn object_retrieval_rejects_representation_digest_mismatch() {
        let logical = b"hello object";
        let object_id = crate::crypto::derive_object_id(b"raw-bytes-v0", logical);
        let descriptor = LogicalObjectDescriptor {
            descriptor_version: 1,
            object_id: object_id.clone(),
            object_kind: 1,
            object_schema_version: 1,
            canonicalization_id: "raw-bytes-v0".into(),
            canonical_digest: None,
            canonical_size: logical.len() as u64,
            created_at: None,
            producer: None,
            describes_object: None,
            object_metadata: None,
        };
        let header = StoredRepresentationHeader {
            header_version: 1,
            representation_id: vec![0x11; 32],
            object: Some(ObjectRef {
                object_id,
                object_kind: Some(1),
            }),
            representation_digest: Some(Digest {
                algorithm: 1,
                value: vec![0xFF; 32],
            }),
            plaintext_size: Some(logical.len() as u64),
            stored_size: logical.len() as u64,
            encryption_scheme: String::new(),
            compression_scheme: String::new(),
            chunking_mode: edgerun_proto::edgerun::v0::object::ChunkingMode::None as i32,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: None,
            representation_metadata: None,
        };

        let result = validate_object_retrieval(
            Some(&descriptor),
            Some(&header),
            None,
            Some(logical),
            Some(logical),
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::RepresentationInvalid));
    }

    #[test]
    fn object_retrieval_defers_missing_manifest() {
        let header = StoredRepresentationHeader {
            header_version: 1,
            representation_id: vec![0x11; 32],
            object: Some(ObjectRef {
                object_id: vec![0x22; 32],
                object_kind: Some(1),
            }),
            representation_digest: None,
            plaintext_size: None,
            stored_size: 10,
            encryption_scheme: String::new(),
            compression_scheme: String::new(),
            chunking_mode: edgerun_proto::edgerun::v0::object::ChunkingMode::Manifest as i32,
            chunk_manifest_object: None,
            access_package_object: None,
            created_at: None,
            representation_metadata: None,
        };

        let result = validate_object_retrieval(None, Some(&header), None, None, None);

        assert_eq!(result.verdict, crate::result::Verdict::Defer);
        assert_eq!(result.reason_code, Some(ReasonCode::MissingDependency));
    }

    fn control_command(
        command_type: CommandType,
        target_controller: Vec<u8>,
        issuer: Vec<u8>,
    ) -> CommandEnvelope {
        CommandEnvelope {
            envelope_version: 1,
            command_id: target_controller,
            target_node: Some(NodeRef {
                node_id: b"node".to_vec(),
            }),
            issuer: Some(IdentityRef {
                identity_id: issuer,
                identity_kind: Some(1),
                key_hint: None,
            }),
            command_type: command_type as i32,
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: vec![],
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signature: None,
        }
    }

    #[test]
    fn control_add_controller_accepts_and_simulates_post_state() {
        let issuer = b"controller-a".to_vec();
        let new_controller = b"controller-b".to_vec();
        let command = control_command(
            CommandType::AddController,
            new_controller.clone(),
            issuer.clone(),
        );

        let result = validate_control_change_command(&command, &[issuer], 1);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
        let post_state = result.post_state.as_map().unwrap();
        let controller_set = post_state.get("controller_set").unwrap().as_seq().unwrap();
        assert_eq!(controller_set.len(), 2);
        assert!(controller_set
            .iter()
            .any(|v| { v.as_str() == Some(crate::util::bytes_to_hex(&new_controller).as_str()) }));
    }

    #[test]
    fn control_remove_controller_rejects_below_minimum() {
        let issuer = b"controller-a".to_vec();
        let command = control_command(
            CommandType::RemoveController,
            issuer.clone(),
            issuer.clone(),
        );

        let result = validate_control_change_command(&command, &[issuer], 1);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::ControlInvariantFailed));
    }

    #[test]
    fn control_change_rejects_non_controller_issuer() {
        let command = control_command(
            CommandType::AddController,
            b"controller-b".to_vec(),
            b"intruder".to_vec(),
        );

        let result = validate_control_change_command(&command, &[b"controller-a".to_vec()], 1);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn control_transfer_adds_new_controller_for_safe_transfer_step() {
        let issuer = b"controller-a".to_vec();
        let new_controller = b"controller-b".to_vec();
        let command = control_command(CommandType::TransferControl, new_controller, issuer.clone());

        let result = validate_control_change_command(&command, &[issuer], 1);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
        let post_state = result.post_state.as_map().unwrap();
        let controller_set = post_state.get("controller_set").unwrap().as_seq().unwrap();
        assert_eq!(controller_set.len(), 2);
    }

    fn signed_query_request(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> QueryRequest {
        let key_hint = test_node_id(key);
        let mut query = QueryRequest {
            request_version: 1,
            query_id: b"query-1".to_vec(),
            requester: Some(IdentityRef {
                identity_id: b"requester".to_vec(),
                identity_kind: Some(1),
                key_hint: Some(key_hint),
            }),
            target_scope: None,
            query_class: edgerun_proto::edgerun::v0::access::QueryClass::Head as i32,
            time_window: None,
            checkpoint_base: None,
            result_limit: Some(10),
            cost_limit: None,
            required_proof_classes: vec![],
            query_payload_object: None,
            signature: None,
        };
        let canonical = canonical_bytes(&ProtocolRecord::QueryRequest(query.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_QUERY_REQUEST,
            &canonical,
        )
        .unwrap();
        query.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        query
    }

    #[test]
    fn query_request_signature_valid_is_accepted() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let query = signed_query_request(&key);

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn query_request_missing_signature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        query.signature = None;

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn query_request_bad_signature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut query = signed_query_request(&key);
        if let Some(sig) = &mut query.signature {
            sig.value[0] ^= 0xFF;
        }

        let result = validate_query_request_signature(&query);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    fn signed_query_result_fragment(
        key: &edgerun_crypto::p256::ecdsa::SigningKey,
    ) -> QueryResultFragment {
        let key_hint = test_node_id(key);
        let mut fragment = QueryResultFragment {
            fragment_version: 1,
            query_id: b"query-1".to_vec(),
            responder: Some(IdentityRef {
                identity_id: b"responder".to_vec(),
                identity_kind: Some(1),
                key_hint: Some(key_hint),
            }),
            answered_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            completeness:
                edgerun_proto::edgerun::v0::access::ResultCompleteness::CompleteForLocalKnowledge
                    as i32,
            snapshot_refs: vec![],
            event_refs: vec![],
            object_refs: vec![ObjectRef {
                object_id: b"object-1".to_vec(),
                object_kind: Some(1),
            }],
            proof_objects: vec![],
            omission_reason: String::new(),
            bundled_result_object: None,
            result_metadata: None,
            signature: None,
        };
        let canonical =
            canonical_bytes(&ProtocolRecord::QueryResultFragment(fragment.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT,
            &canonical,
        )
        .unwrap();
        fragment.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        fragment
    }

    #[test]
    fn query_result_fragment_valid_is_accepted_as_advisory() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let fragment = signed_query_result_fragment(&key);

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
        assert_eq!(
            result.derived.as_map().unwrap().get("advisory_only"),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn query_result_fragment_without_backing_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        fragment.object_refs.clear();
        let canonical =
            canonical_bytes(&ProtocolRecord::QueryResultFragment(fragment.clone()), true);
        fragment.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: crate::crypto::sign_canonical_record(
                &key,
                crate::crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT,
                &canonical,
            )
            .unwrap(),
        });

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn query_result_fragment_bad_signature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        if let Some(sig) = &mut fragment.signature {
            sig.value[0] ^= 0xFF;
        }

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn query_result_denial_without_backing_is_accepted() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[44u8; 32].into()).unwrap();
        let mut fragment = signed_query_result_fragment(&key);
        fragment.object_refs.clear();
        fragment.completeness =
            edgerun_proto::edgerun::v0::access::ResultCompleteness::Denied as i32;
        fragment.omission_reason = "policy_denied".into();
        let canonical =
            canonical_bytes(&ProtocolRecord::QueryResultFragment(fragment.clone()), true);
        fragment.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: crate::crypto::sign_canonical_record(
                &key,
                crate::crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT,
                &canonical,
            )
            .unwrap(),
        });

        let result = validate_query_result_fragment(&fragment, Some(b"query-1"), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    fn signed_session_hello(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> SessionHello {
        let node_id = test_node_id(key);
        let mut hello = SessionHello {
            message_version: 1,
            initiator: Some(IdentityRef {
                identity_id: node_id.clone(),
                identity_kind: Some(1),
                key_hint: Some(node_id),
            }),
            target_node: Some(NodeRef {
                node_id: b"target-node".to_vec(),
            }),
            supported_transport_features: vec!["tcp".into()],
            supported_protocol_versions: vec![1],
            session_nonce: b"nonce-1".to_vec(),
            initiator_locators: vec![],
            hello_metadata: None,
            signature: None,
        };
        let canonical = canonical_bytes(&ProtocolRecord::SessionHello(hello.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_SESSION_HELLO,
            &canonical,
        )
        .unwrap();
        hello.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        hello
    }

    fn signed_session_accept(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> SessionAccept {
        let node_id = test_node_id(key);
        let mut accept_msg = SessionAccept {
            message_version: 1,
            responder: Some(IdentityRef {
                identity_id: node_id.clone(),
                identity_kind: Some(1),
                key_hint: Some(node_id),
            }),
            echoed_session_nonce: b"nonce-1".to_vec(),
            selected_protocol_version: 1,
            selected_transport_features: vec!["tcp".into()],
            responder_locators: vec![],
            accept_metadata: None,
            signature: None,
        };
        let canonical = canonical_bytes(&ProtocolRecord::SessionAccept(accept_msg.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_SESSION_ACCEPT,
            &canonical,
        )
        .unwrap();
        accept_msg.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        accept_msg
    }

    #[test]
    fn session_hello_valid_is_accepted() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let hello = signed_session_hello(&key);

        let result = validate_session_hello(&hello, Some(b"target-node"));

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn session_hello_target_mismatch_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let hello = signed_session_hello(&key);

        let result = validate_session_hello(&hello, Some(b"other-node"));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TargetMismatch));
    }

    #[test]
    fn session_hello_bad_signature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut hello = signed_session_hello(&key);
        hello.session_nonce.push(0xFF);

        let result = validate_session_hello(&hello, Some(b"target-node"));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn session_accept_valid_is_accepted() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let accept_msg = signed_session_accept(&key);

        let result = validate_session_accept(&accept_msg, b"nonce-1", &[1]);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn session_accept_nonce_mismatch_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let accept_msg = signed_session_accept(&key);

        let result = validate_session_accept(&accept_msg, b"other-nonce", &[1]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn session_accept_unsupported_version_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let accept_msg = signed_session_accept(&key);

        let result = validate_session_accept(&accept_msg, b"nonce-1", &[2]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::VersionUnsupported));
    }

    fn signed_assurance_claim(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> AssuranceClaim {
        let node_id = test_node_id(key);
        let mut claim = AssuranceClaim {
            claim_version: 1,
            subject: Some(
                edgerun_proto::edgerun::v0::trust::assurance_claim::Subject::SubjectNode(NodeRef {
                    node_id: b"subject-node".to_vec(),
                }),
            ),
            assurance_class: edgerun_proto::edgerun::v0::common::AssuranceClass::HardwareBacked
                as i32,
            attester: Some(IdentityRef {
                identity_id: node_id.clone(),
                identity_kind: Some(1),
                key_hint: Some(node_id),
            }),
            issued_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            expires_at: Some(prost_types::Timestamp {
                seconds: 1_800_000_000,
                nanos: 0,
            }),
            evidence_object: None,
            claim_note: "test".into(),
            signature: None,
        };
        let canonical = canonical_bytes(&ProtocolRecord::AssuranceClaim(claim.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_ASSURANCE_CLAIM,
            &canonical,
        )
        .unwrap();
        claim.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        claim
    }

    fn signed_revocation_record(key: &edgerun_crypto::p256::ecdsa::SigningKey) -> RevocationRecord {
        let node_id = test_node_id(key);
        let mut revocation = RevocationRecord {
            record_version: 1,
            revocation_id: b"revocation-1".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: node_id.clone(),
                identity_kind: Some(1),
                key_hint: Some(node_id),
            }),
            issued_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            effective_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_001,
                nanos: 0,
            }),
            revocation_kind: edgerun_proto::edgerun::v0::trust::RevocationKind::Delegation as i32,
            target: Some(
                edgerun_proto::edgerun::v0::trust::revocation_record::Target::TargetDelegation(
                    crate::protocol::DelegationRef {
                        delegation_id: b"delegation-1".to_vec(),
                        delegation_hash: None,
                    },
                ),
            ),
            scope_override: None,
            reason_code: "compromised".into(),
            replacement_id: vec![],
            revocation_metadata: None,
            signature: None,
        };
        let canonical =
            canonical_bytes(&ProtocolRecord::RevocationRecord(revocation.clone()), true);
        let sig = crate::crypto::sign_canonical_record(
            key,
            crate::crypto::SIG_DOMAIN_REVOCATION_RECORD,
            &canonical,
        )
        .unwrap();
        revocation.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        revocation
    }

    #[test]
    fn assurance_claim_valid_is_accepted() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let claim = signed_assurance_claim(&key);
        let trusted = vec![claim.attester.as_ref().unwrap().identity_id.clone()];

        let result = validate_assurance_claim(&claim, 1_710_000_000_000, &trusted);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn assurance_claim_bad_signature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let mut claim = signed_assurance_claim(&key);
        claim.claim_note.push_str("-tampered");

        let result = validate_assurance_claim(&claim, 1_710_000_000_000, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn assurance_claim_untrusted_attester_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let claim = signed_assurance_claim(&key);

        let result = validate_assurance_claim(&claim, 1_710_000_000_000, &[b"other".to_vec()]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn revocation_record_valid_is_accepted() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let revocation = signed_revocation_record(&key);
        let trusted = vec![revocation.issuer.as_ref().unwrap().identity_id.clone()];

        let result = validate_revocation_record(&revocation, 1_710_000_000_000, &trusted);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn revocation_record_bad_signature_is_rejected() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let mut revocation = signed_revocation_record(&key);
        if let Some(sig) = &mut revocation.signature {
            sig.value[0] ^= 0xFF;
        }

        let result = validate_revocation_record(&revocation, 1_710_000_000_000, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn revocation_record_not_yet_effective_defers() {
        let key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[43u8; 32].into()).unwrap();
        let revocation = signed_revocation_record(&key);

        let result = validate_revocation_record(&revocation, 1_700_000_000_500, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Defer);
        assert_eq!(result.reason_code, Some(ReasonCode::TimeInvalid));
    }

    // ------------------------------------------------------------------
    // Event signature verification — regression tests to catch no-op
    // verify_event_signature implementations.
    // ------------------------------------------------------------------

    /// Signs an event with a real ECDSA P-256 key and attaches the signature.
    fn sign_event_envelope(
        event: &EventEnvelope,
        signing_key: &edgerun_crypto::p256::ecdsa::SigningKey,
    ) -> EventEnvelope {
        let record = ProtocolRecord::EventEnvelope(event.clone());
        let canonical = canonical_bytes(&record, true);
        let record_hash =
            crate::crypto::record_hash(crate::crypto::HASH_DOMAIN_EVENT_ENVELOPE, &canonical);

        let sig = crate::crypto::sign_record(
            signing_key,
            crate::crypto::SIG_DOMAIN_EVENT_ENVELOPE,
            &record_hash,
        )
        .expect("signing failed");

        let mut event = event.clone();
        event.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        event
    }

    #[test]
    fn event_signature_valid_is_accepted() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let verifying_key = signing_key.verifying_key();
        let node_id = crate::crypto::verifying_key_to_node_id(verifying_key);

        let genesis = make_genesis_event();
        let signed = sign_event_envelope(&genesis, &signing_key);

        let result = validate_stream_append(&signed, None, Some(&node_id));
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn event_signature_invalid_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let verifying_key = signing_key.verifying_key();
        let node_id = crate::crypto::verifying_key_to_node_id(verifying_key);

        let genesis = make_genesis_event();
        let mut signed = sign_event_envelope(&genesis, &signing_key);

        // Tamper with one byte of the signature
        if let Some(ref mut sig) = signed.signature {
            sig.value[0] ^= 0xFF;
        }

        let result = validate_stream_append(&signed, None, Some(&node_id));
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn event_signature_wrong_key_is_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let verifying_key = signing_key.verifying_key();
        let node_id = crate::crypto::verifying_key_to_node_id(verifying_key);

        let other_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[99u8; 32].into()).unwrap();

        let genesis = make_genesis_event();
        let signed = sign_event_envelope(&genesis, &other_key);

        // Verify with the WRONG key
        let result = validate_stream_append(&signed, None, Some(&node_id));
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn event_signature_bogus_bytes_are_rejected() {
        let signing_key =
            edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let verifying_key = signing_key.verifying_key();
        let node_id = crate::crypto::verifying_key_to_node_id(verifying_key);

        let mut event = make_genesis_event();
        // Put garbage in the signature value — this must NOT be accepted.
        event.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: vec![0xDE; 64],
        });

        let result = validate_stream_append(&event, None, Some(&node_id));
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }
}
