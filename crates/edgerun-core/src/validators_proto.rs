//! Protobuf-native protocol validators.
//!
//! This module provides direct validation functions over protobuf-generated types,
//! bridging the gap between the conformance suite (validators.rs, YAML/JSON-based)
//! and the runtime protocol path (command.rs, stream crate).
//!
//! Each validator takes and returns protobuf-native types with the unified
//! ACCEPT / REJECT / DEFER / DUPLICATE outcome model.

use crate::protocol::{
    canonical_bytes, DelegationRecord, EventEnvelope, ProtocolRecord, SnapshotDescriptor,
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
    let canonical = canonical_bytes(&record, false);
    let hash = crate::crypto::sha256(&canonical);
    crate::protocol::Digest {
        algorithm: 1,
        value: hash.to_vec(),
    }
}

fn verify_event_signature(event: &EventEnvelope, key: &[u8; 64]) -> bool {
    let Some(sig) = &event.signature else {
        return false;
    };

    if sig.algorithm != 1 {
        return false;
    }

    if sig.value.len() != 64 {
        return false;
    }

    let vk = match crate::crypto::node_id_to_verifying_key(key) {
        Some(vk) => vk,
        None => return false,
    };

    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, true);
    let record_hash = crate::crypto::sha256(&canonical);

    let sig_input = crate::crypto::signature_input(
        crate::crypto::HASH_DOMAIN_EVENT_ENVELOPE,
        &record_hash,
    );

    use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashVerifier;
    let Ok(r): Result<[u8; 32], _> = sig.value[..32].try_into() else { return false };
    let Ok(s): Result<[u8; 32], _> = sig.value[32..].try_into() else { return false };
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
        let curr_issuer = chain[i]
            .issuer
            .as_ref()
            .map(|r| r.identity_id.clone());
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
            let not_before_ms =
                not_before.seconds * 1000 + (not_before.nanos as i64) / 1_000_000;
            if now_ms < not_before_ms {
                return defer(
                    ReasonCode::TimeInvalid,
                    Value::String("delegation not yet valid".into()),
                );
            }
        }

        // Structural: signature must be present
        if delegation.signature.is_none() {
            return reject(
                ReasonCode::CryptoInvalid,
                Value::String("delegation missing signature".into()),
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
    use crate::protocol::{EventEnvelope, EventType, NodeRef, IdentityRef};

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
        let event = make_event(1, Some(crate::protocol::Digest {
            algorithm: 1,
            value: vec![1; 32],
        }));
        let result = validate_stream_append(&event, None, None);
        assert_eq!(result.verdict, crate::result::Verdict::Defer);
    }

    #[test]
    fn stream_append_defers_missing_predecessor() {
        let genesis = make_genesis_event();
        let event = make_event(5, Some(crate::protocol::Digest {
            algorithm: 1,
            value: vec![1; 32],
        }));
        let result = validate_stream_append(&event, Some(&genesis), None);
        assert_eq!(result.verdict, crate::result::Verdict::Defer);
    }

    #[test]
    fn stream_append_rejects_prev_hash_mismatch() {
        let genesis = make_genesis_event();
        let event = make_event(1, Some(crate::protocol::Digest {
            algorithm: 1,
            value: vec![0xFF; 32],
        }));
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
        let deleg = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef { identity_id: b"root".to_vec(), identity_kind: None, key_hint: None }),
            recipient: Some(IdentityRef { identity_id: b"user".to_vec(), identity_kind: None, key_hint: None }),
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(crate::protocol::Signature { algorithm: 1, value: vec![1; 64] }),
        };

        let result = validate_delegation_chain(&[deleg], 1_700_000_000_000, &std::collections::HashSet::new());
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn delegation_chain_revoked() {
        let deleg = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-1".to_vec(),
            issuer: Some(IdentityRef { identity_id: b"root".to_vec(), identity_kind: None, key_hint: None }),
            recipient: Some(IdentityRef { identity_id: b"user".to_vec(), identity_kind: None, key_hint: None }),
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(crate::protocol::Signature { algorithm: 1, value: vec![1; 64] }),
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
            issuer: Some(IdentityRef { identity_id: b"root".to_vec(), identity_kind: None, key_hint: None }),
            recipient: Some(IdentityRef { identity_id: b"mid".to_vec(), identity_kind: None, key_hint: None }),
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(crate::protocol::Signature { algorithm: 1, value: vec![1; 64] }),
        };

        let child = DelegationRecord {
            record_version: 1,
            delegation_id: b"deleg-2".to_vec(),
            issuer: Some(IdentityRef { identity_id: b"OTHER".to_vec(), identity_kind: None, key_hint: None }), // != "mid"
            recipient: Some(IdentityRef { identity_id: b"user".to_vec(), identity_kind: None, key_hint: None }),
            issued_at: None,
            not_before: None,
            expires_at: None,
            capability: None,
            parent_delegation: None,
            revocation_authorities: vec![],
            delegation_metadata: None,
            signature: Some(crate::protocol::Signature { algorithm: 1, value: vec![1; 64] }),
        };

        let result = validate_delegation_chain(&[parent, child], 1_700_000_000_000, &std::collections::HashSet::new());
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    #[test]
    fn snapshot_valid_with_trusted_producer() {
        let snapshot = SnapshotDescriptor {
            descriptor_version: 1,
            snapshot_id: b"snap-1".to_vec(),
            view_type: "timeline".into(),
            view_version: 1,
            producer: Some(IdentityRef { identity_id: b"trusted".to_vec(), identity_kind: None, key_hint: None }),
            produced_at: None,
            base_heads: vec![crate::protocol::HeadRef {
                stream_id: b"stream-1".to_vec(),
                seq: 10,
                event_hash: None,
            }],
            base_checkpoints: vec![],
            scope: None,
            completeness: 0,
            payload_object: Some(crate::protocol::ObjectRef { object_id: b"obj-1".to_vec(), object_kind: None }),
            supersedes: None,
            snapshot_metadata: None,
            signature: None,
        };

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn snapshot_untrusted_producer_rejected() {
        let snapshot = SnapshotDescriptor {
            descriptor_version: 1,
            snapshot_id: b"snap-1".to_vec(),
            view_type: "timeline".into(),
            view_version: 1,
            producer: Some(IdentityRef { identity_id: b"untrusted".to_vec(), identity_kind: None, key_hint: None }),
            produced_at: None,
            base_heads: vec![crate::protocol::HeadRef {
                stream_id: b"stream-1".to_vec(),
                seq: 10,
                event_hash: None,
            }],
            base_checkpoints: vec![],
            scope: None,
            completeness: 0,
            payload_object: Some(crate::protocol::ObjectRef { object_id: b"obj-1".to_vec(), object_kind: None }),
            supersedes: None,
            snapshot_metadata: None,
            signature: None,
        };

        let result = validate_snapshot(&snapshot, &[b"trusted".to_vec()]);
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::AuthorityDenied));
    }

    // ------------------------------------------------------------------
    // Event signature verification — regression tests to catch no-op
    // verify_event_signature implementations.
    // ------------------------------------------------------------------

    /// Signs an event with a real ECDSA P-256 key and attaches the signature.
    fn sign_event_envelope(event: &EventEnvelope, signing_key: &edgerun_crypto::p256::ecdsa::SigningKey) -> EventEnvelope {
        let record = ProtocolRecord::EventEnvelope(event.clone());
        let canonical = canonical_bytes(&record, true);
        let record_hash = crate::crypto::sha256(&canonical);

        let sig = crate::crypto::sign_record(
            signing_key,
            crate::crypto::HASH_DOMAIN_EVENT_ENVELOPE,
            &record_hash,
        ).expect("signing failed");

        let mut event = event.clone();
        event.signature = Some(crate::protocol::Signature {
            algorithm: 1,
            value: sig,
        });
        event
    }

    #[test]
    fn event_signature_valid_is_accepted() {
        let signing_key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let verifying_key = signing_key.verifying_key();
        let node_id = crate::crypto::verifying_key_to_node_id(verifying_key);

        let genesis = make_genesis_event();
        let signed = sign_event_envelope(&genesis, &signing_key);

        let result = validate_stream_append(&signed, None, Some(&node_id));
        assert_eq!(result.verdict, crate::result::Verdict::Accept);
    }

    #[test]
    fn event_signature_invalid_is_rejected() {
        let signing_key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
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
        let signing_key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
        let verifying_key = signing_key.verifying_key();
        let node_id = crate::crypto::verifying_key_to_node_id(verifying_key);

        let other_key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[99u8; 32].into()).unwrap();

        let genesis = make_genesis_event();
        let signed = sign_event_envelope(&genesis, &other_key);

        // Verify with the WRONG key
        let result = validate_stream_append(&signed, None, Some(&node_id));
        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::CryptoInvalid));
    }

    #[test]
    fn event_signature_bogus_bytes_are_rejected() {
        let signing_key = edgerun_crypto::p256::ecdsa::SigningKey::from_bytes(&[42u8; 32].into()).unwrap();
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
