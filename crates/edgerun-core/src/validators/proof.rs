//! Proof structural validation per spec §8.1.
//!
//! Validates the structural integrity of individual proof types.
//! ProofBundle itself carries a payload_type + payload_object reference;
//! the actual proof objects are resolved separately.
//!
//! Rules from spec §8.1:
//! - SnapshotSetProof with empty asserted set → structurally invalid
//! - EventSetProof with empty asserted set → structurally invalid
//! - ObjectAssertionProof without object_ref → structurally invalid
//! - AggregateSummaryProof with overlapping included/excluded responders → structurally invalid
//! - TrustPolicyProof MUST carry at least one of policy_object or assignments_object

use crate::prelude::v1::*;

use crate::protocol::{
    AggregateSummaryProof, EventSetProof, FederatedAggregateDescriptor, ObjectAssertionProof,
    ProofBundle, ProofPayloadType, ResultFragmentProof, SnapshotSetProof, StreamHeadsProof,
    TrustPolicyProof,
};
use crate::result::{accept, defer, empty_map, reject, ReasonCode, ValidationResult};
use crate::value::{mapping, ystr, Value};

/// Structural validation result for a proof object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofStructuralResult {
    Valid,
    Invalid { reason: &'static str },
}

fn validate_object_ref(
    object: &crate::protocol::ObjectRef,
    reason: &'static str,
) -> Option<ProofStructuralResult> {
    if object.object_id.is_empty() {
        return Some(ProofStructuralResult::Invalid { reason });
    }
    if object.object_kind.is_some_and(|object_kind| {
        crate::protocol::ObjectKind::from_i32(object_kind)
            .is_none_or(|kind| kind == crate::protocol::ObjectKind::Unspecified)
    }) {
        return Some(ProofStructuralResult::Invalid {
            reason: "ObjectRef object_kind is invalid",
        });
    }
    None
}

fn validate_identity_ref(
    identity: &crate::protocol::IdentityRef,
    empty_reason: &'static str,
    invalid_kind_reason: &'static str,
) -> Option<ProofStructuralResult> {
    if identity.identity_id.is_empty() {
        return Some(ProofStructuralResult::Invalid {
            reason: empty_reason,
        });
    }
    if identity.identity_kind.is_some_and(|identity_kind| {
        crate::protocol::IdentityKind::from_i32(identity_kind)
            .is_none_or(|kind| kind == crate::protocol::IdentityKind::Unspecified)
    }) {
        return Some(ProofStructuralResult::Invalid {
            reason: invalid_kind_reason,
        });
    }
    None
}

fn validate_event_ref(event: &crate::protocol::EventRef) -> Option<ProofStructuralResult> {
    if event.stream_id.is_empty() {
        return Some(ProofStructuralResult::Invalid {
            reason: "EventSetProof event_ref missing stream_id",
        });
    }
    let Some(event_hash) = &event.event_hash else {
        return Some(ProofStructuralResult::Invalid {
            reason: "EventSetProof event_ref missing event_hash",
        });
    };
    if event_hash.algorithm != crate::protocol::digest::Algorithm::DigestAlgorithmSha256 as i32 {
        return Some(ProofStructuralResult::Invalid {
            reason: "EventSetProof event_ref event_hash algorithm is not SHA-256",
        });
    }
    if event_hash.value.len() != 32 {
        return Some(ProofStructuralResult::Invalid {
            reason: "EventSetProof event_ref event_hash must be 32 bytes",
        });
    }
    None
}

fn validate_head_ref(head: &crate::protocol::HeadRef) -> Option<ProofStructuralResult> {
    if head.stream_id.is_empty() {
        return Some(ProofStructuralResult::Invalid {
            reason: "StreamHeadsProof head_ref missing stream_id",
        });
    }
    let Some(event_hash) = &head.event_hash else {
        return Some(ProofStructuralResult::Invalid {
            reason: "StreamHeadsProof head_ref missing event_hash",
        });
    };
    if event_hash.algorithm != crate::protocol::digest::Algorithm::DigestAlgorithmSha256 as i32 {
        return Some(ProofStructuralResult::Invalid {
            reason: "StreamHeadsProof head_ref event_hash algorithm is not SHA-256",
        });
    }
    if event_hash.value.len() != 32 {
        return Some(ProofStructuralResult::Invalid {
            reason: "StreamHeadsProof head_ref event_hash must be 32 bytes",
        });
    }
    None
}

fn validate_source_query_id(
    source_query_id: &[u8],
    reason: &'static str,
) -> Option<ProofStructuralResult> {
    if source_query_id.is_empty() {
        return Some(ProofStructuralResult::Invalid { reason });
    }
    None
}

fn validate_timestamp_shape(
    timestamp: &prost_types::Timestamp,
    reason: &'static str,
) -> Option<ValidationResult> {
    if !(0..1_000_000_000).contains(&timestamp.nanos) {
        return Some(reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr(reason))]),
            empty_map(),
        ));
    }
    None
}

/// Validates the structural integrity of a StreamHeadsProof.
pub fn validate_stream_heads_proof(proof: &StreamHeadsProof) -> ProofStructuralResult {
    if let Some(result) = validate_source_query_id(
        &proof.source_query_id,
        "StreamHeadsProof missing source_query_id",
    ) {
        return result;
    }
    if proof.heads.is_empty() {
        return ProofStructuralResult::Invalid {
            reason: "StreamHeadsProof has empty asserted set",
        };
    }
    for head in &proof.heads {
        if let Some(result) = validate_head_ref(head) {
            return result;
        }
    }
    ProofStructuralResult::Valid
}

/// Validates the structural integrity of a SnapshotSetProof.
///
/// §8.1: SnapshotSetProof with empty asserted set is structurally invalid.
pub fn validate_snapshot_set_proof(proof: &SnapshotSetProof) -> ProofStructuralResult {
    if let Some(result) = validate_source_query_id(
        &proof.source_query_id,
        "SnapshotSetProof missing source_query_id",
    ) {
        return result;
    }
    if proof.snapshots.is_empty() {
        return ProofStructuralResult::Invalid {
            reason: "SnapshotSetProof has empty asserted set",
        };
    }
    for snapshot in &proof.snapshots {
        if snapshot.snapshot_id.is_empty() {
            return ProofStructuralResult::Invalid {
                reason: "SnapshotSetProof snapshot_ref missing snapshot_id",
            };
        }
        if snapshot.object_id.as_ref().is_some_and(Vec::is_empty) {
            return ProofStructuralResult::Invalid {
                reason: "SnapshotSetProof snapshot_ref empty object_id",
            };
        }
    }
    ProofStructuralResult::Valid
}

/// Validates the structural integrity of an EventSetProof.
///
/// §8.1: EventSetProof with empty asserted set is structurally invalid.
pub fn validate_event_set_proof(proof: &EventSetProof) -> ProofStructuralResult {
    if let Some(result) = validate_source_query_id(
        &proof.source_query_id,
        "EventSetProof missing source_query_id",
    ) {
        return result;
    }
    if proof.events.is_empty() {
        return ProofStructuralResult::Invalid {
            reason: "EventSetProof has empty asserted set",
        };
    }
    for event in &proof.events {
        if let Some(result) = validate_event_ref(event) {
            return result;
        }
    }
    for object in &proof.related_objects {
        if let Some(result) =
            validate_object_ref(object, "EventSetProof related_object missing object_id")
        {
            return result;
        }
    }
    ProofStructuralResult::Valid
}

/// Validates the structural integrity of an ObjectAssertionProof.
///
/// §8.1: ObjectAssertionProof without object_ref is structurally invalid.
pub fn validate_object_assertion_proof(proof: &ObjectAssertionProof) -> ProofStructuralResult {
    if let Some(result) = validate_source_query_id(
        &proof.source_query_id,
        "ObjectAssertionProof missing source_query_id",
    ) {
        return result;
    }
    let Some(object_ref) = &proof.object_ref else {
        return ProofStructuralResult::Invalid {
            reason: "ObjectAssertionProof missing object_ref",
        };
    };
    if let Some(result) = validate_object_ref(
        object_ref,
        "ObjectAssertionProof object_ref missing object_id",
    ) {
        return result;
    }
    if let Some(bundled_result_object) = &proof.bundled_result_object {
        if let Some(result) = validate_object_ref(
            bundled_result_object,
            "ObjectAssertionProof bundled_result_object missing object_id",
        ) {
            return result;
        }
    }
    ProofStructuralResult::Valid
}

/// Validates the structural integrity of a ResultFragmentProof wrapper.
pub fn validate_result_fragment_proof(proof: &ResultFragmentProof) -> ProofStructuralResult {
    let Some(fragment) = &proof.fragment else {
        return ProofStructuralResult::Invalid {
            reason: "ResultFragmentProof missing fragment",
        };
    };
    if fragment.query_id.is_empty() {
        return ProofStructuralResult::Invalid {
            reason: "ResultFragmentProof fragment missing query_id",
        };
    }
    ProofStructuralResult::Valid
}

/// Validates the structural integrity of an AggregateSummaryProof.
///
/// §8.1: AggregateSummaryProof with overlapping included and excluded
/// responders is structurally invalid.
pub fn validate_aggregate_summary_proof(proof: &AggregateSummaryProof) -> ProofStructuralResult {
    use std::collections::HashSet;
    if let Some(result) = validate_source_query_id(
        &proof.source_query_id,
        "AggregateSummaryProof missing source_query_id",
    ) {
        return result;
    }
    for responder in proof
        .included_responders
        .iter()
        .chain(proof.excluded_responders.iter())
    {
        if let Some(result) = validate_identity_ref(
            responder,
            "AggregateSummaryProof responder identity_id is empty",
            "AggregateSummaryProof responder identity_kind is invalid",
        ) {
            return result;
        }
    }
    let included: HashSet<&[u8]> = proof
        .included_responders
        .iter()
        .map(|r| r.identity_id.as_slice())
        .collect();
    let excluded: HashSet<&[u8]> = proof
        .excluded_responders
        .iter()
        .map(|r| r.identity_id.as_slice())
        .collect();
    for id in &included {
        if excluded.contains(id) {
            return ProofStructuralResult::Invalid {
                reason: "AggregateSummaryProof has overlapping included/excluded responders",
            };
        }
    }
    if let Some(trust_policy_object) = &proof.trust_policy_object {
        if let Some(result) = validate_object_ref(
            trust_policy_object,
            "AggregateSummaryProof trust_policy_object missing object_id",
        ) {
            return result;
        }
    }
    ProofStructuralResult::Valid
}

/// Validates the structural integrity of a TrustPolicyProof.
///
/// §8.1: TrustPolicyProof MUST carry at least one of policy_object
/// or assignments_object.
pub fn validate_trust_policy_proof(proof: &TrustPolicyProof) -> ProofStructuralResult {
    if let Some(result) = validate_source_query_id(
        &proof.source_query_id,
        "TrustPolicyProof missing source_query_id",
    ) {
        return result;
    }
    if proof.policy_object.is_none() && proof.assignments_object.is_none() {
        return ProofStructuralResult::Invalid {
            reason: "TrustPolicyProof missing both policy_object and assignments_object",
        };
    }
    if let Some(policy_object) = &proof.policy_object {
        if let Some(result) = validate_object_ref(
            policy_object,
            "TrustPolicyProof policy_object missing object_id",
        ) {
            return result;
        }
    }
    if let Some(assignments_object) = &proof.assignments_object {
        if let Some(result) = validate_object_ref(
            assignments_object,
            "TrustPolicyProof assignments_object missing object_id",
        ) {
            return result;
        }
    }
    ProofStructuralResult::Valid
}

/// Validates the structural integrity of a `ProofBundle`.
///
/// The bundle is advisory in v0. This validator checks deterministic envelope
/// rules only: version, payload family, query binding, payload object presence,
/// optional allowed payload families, and optional local object availability.
pub fn validate_proof_bundle(
    bundle: &ProofBundle,
    expected_query_id: Option<&[u8]>,
    available_object_ids: Option<&std::collections::HashSet<Vec<u8>>>,
    allowed_payload_types: &[i32],
) -> ValidationResult {
    if bundle.bundle_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            mapping([("reason", ystr("unsupported_bundle_version"))]),
            empty_map(),
        );
    }

    let Some(payload_type) = ProofPayloadType::from_i32(bundle.payload_type) else {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("invalid_payload_type"))]),
            empty_map(),
        );
    };
    if payload_type == ProofPayloadType::Unspecified {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("unspecified_payload_type"))]),
            empty_map(),
        );
    }
    if !allowed_payload_types.is_empty() && !allowed_payload_types.contains(&bundle.payload_type) {
        return reject(
            ReasonCode::PolicyDenied,
            mapping([("reason", ystr("payload_type_not_allowed"))]),
            empty_map(),
        );
    }

    if bundle.source_query_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_source_query_id"))]),
            empty_map(),
        );
    }
    if let Some(expected_query_id) = expected_query_id {
        if bundle.source_query_id.as_slice() != expected_query_id {
            return reject(
                ReasonCode::TargetMismatch,
                mapping([("reason", ystr("source_query_id_mismatch"))]),
                empty_map(),
            );
        }
    }

    let Some(payload_object) = &bundle.payload_object else {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_payload_object"))]),
            empty_map(),
        );
    };
    if let Some(ProofStructuralResult::Invalid { reason }) =
        validate_object_ref(payload_object, "empty_payload_object_id")
    {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr(reason))]),
            empty_map(),
        );
    }

    for object in &bundle.supporting_objects {
        if let Some(ProofStructuralResult::Invalid { reason }) =
            validate_object_ref(object, "empty_supporting_object_id")
        {
            return reject(
                ReasonCode::StructuralInvalid,
                mapping([("reason", ystr(reason))]),
                empty_map(),
            );
        }
    }

    if let Some(available_object_ids) = available_object_ids {
        if !available_object_ids.contains(&payload_object.object_id) {
            return defer(
                ReasonCode::MissingDependency,
                mapping([("reason", ystr("payload_object_not_available"))]),
            );
        }
        for object in &bundle.supporting_objects {
            if !available_object_ids.contains(&object.object_id) {
                return defer(
                    ReasonCode::MissingDependency,
                    mapping([("reason", ystr("supporting_object_not_available"))]),
                );
            }
        }
    }

    accept(
        mapping([
            ("decision", ystr("proof_bundle_received")),
            ("advisory_only", Value::Bool(true)),
        ]),
        empty_map(),
    )
}

/// Validates the structural integrity of a `FederatedAggregateDescriptor`.
///
/// The descriptor is a derived summary in v0. Accepting it does not make its
/// input fragments authoritative; callers must validate linked fragments and
/// payload objects under explicit local policy before promotion.
pub fn validate_federated_aggregate_descriptor(
    descriptor: &FederatedAggregateDescriptor,
    expected_query_id: Option<&[u8]>,
) -> ValidationResult {
    if descriptor.descriptor_version != 1 {
        return reject(
            ReasonCode::VersionUnsupported,
            mapping([("reason", ystr("unsupported_aggregate_descriptor_version"))]),
            empty_map(),
        );
    }
    if descriptor.aggregate_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_aggregate_id"))]),
            empty_map(),
        );
    }
    if descriptor.source_query_id.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_source_query_id"))]),
            empty_map(),
        );
    }
    if let Some(expected_query_id) = expected_query_id {
        if descriptor.source_query_id.as_slice() != expected_query_id {
            return reject(
                ReasonCode::TargetMismatch,
                mapping([("reason", ystr("source_query_id_mismatch"))]),
                empty_map(),
            );
        }
    }

    let Some(aggregator) = &descriptor.aggregator else {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_aggregator"))]),
            empty_map(),
        );
    };
    if let Some(ProofStructuralResult::Invalid { reason }) = validate_identity_ref(
        aggregator,
        "empty_aggregator_identity",
        "invalid_aggregator_identity_kind",
    ) {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr(reason))]),
            empty_map(),
        );
    }
    let Some(aggregated_at) = &descriptor.aggregated_at else {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_aggregated_at"))]),
            empty_map(),
        );
    };
    if let Some(result) = validate_timestamp_shape(aggregated_at, "invalid_aggregated_at_timestamp")
    {
        return result;
    }
    if descriptor.input_fragments.is_empty() {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_input_fragments"))]),
            empty_map(),
        );
    }
    for fragment in &descriptor.input_fragments {
        if let Some(ProofStructuralResult::Invalid { reason }) =
            validate_object_ref(fragment, "empty_input_fragment_object_id")
        {
            return reject(
                ReasonCode::StructuralInvalid,
                mapping([("reason", ystr(reason))]),
                empty_map(),
            );
        }
    }
    if let Some(aggregation_policy_object) = &descriptor.aggregation_policy_object {
        if let Some(ProofStructuralResult::Invalid { reason }) = validate_object_ref(
            aggregation_policy_object,
            "empty_aggregation_policy_object_id",
        ) {
            return reject(
                ReasonCode::StructuralInvalid,
                mapping([("reason", ystr(reason))]),
                empty_map(),
            );
        }
    }

    let Some(payload_object) = &descriptor.payload_object else {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr("missing_payload_object"))]),
            empty_map(),
        );
    };
    if let Some(ProofStructuralResult::Invalid { reason }) =
        validate_object_ref(payload_object, "empty_payload_object_id")
    {
        return reject(
            ReasonCode::StructuralInvalid,
            mapping([("reason", ystr(reason))]),
            empty_map(),
        );
    }

    accept(
        mapping([
            ("decision", ystr("federated_aggregate_descriptor_received")),
            ("advisory_only", Value::Bool(true)),
        ]),
        empty_map(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        EventRef, HeadRef, IdentityKind, IdentityRef, ObjectKind, ObjectRef, SnapshotRef,
    };

    #[test]
    fn stream_heads_proof_nonempty_is_valid() {
        let proof = StreamHeadsProof {
            source_query_id: vec![1],
            heads: vec![HeadRef {
                stream_id: vec![2],
                seq: 3,
                event_hash: Some(crate::protocol::Digest {
                    algorithm: 1,
                    value: vec![4; 32],
                }),
            }],
        };
        assert_eq!(
            validate_stream_heads_proof(&proof),
            ProofStructuralResult::Valid
        );
    }

    #[test]
    fn stream_heads_proof_empty_source_query_id_is_invalid() {
        let proof = StreamHeadsProof {
            source_query_id: vec![],
            heads: vec![HeadRef {
                stream_id: vec![2],
                seq: 3,
                event_hash: Some(crate::protocol::Digest {
                    algorithm: 1,
                    value: vec![4; 32],
                }),
            }],
        };
        assert_eq!(
            validate_stream_heads_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "StreamHeadsProof missing source_query_id"
            }
        );
    }

    #[test]
    fn stream_heads_proof_empty_is_invalid() {
        let proof = StreamHeadsProof {
            source_query_id: vec![1],
            heads: vec![],
        };
        assert_eq!(
            validate_stream_heads_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "StreamHeadsProof has empty asserted set"
            }
        );
    }

    #[test]
    fn stream_heads_proof_missing_event_hash_is_invalid() {
        let proof = StreamHeadsProof {
            source_query_id: vec![1],
            heads: vec![HeadRef {
                stream_id: vec![2],
                seq: 3,
                event_hash: None,
            }],
        };
        assert_eq!(
            validate_stream_heads_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "StreamHeadsProof head_ref missing event_hash"
            }
        );
    }

    #[test]
    fn snapshot_set_proof_empty_is_invalid() {
        let proof = SnapshotSetProof {
            source_query_id: vec![1],
            snapshots: vec![],
        };
        assert_eq!(
            validate_snapshot_set_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "SnapshotSetProof has empty asserted set"
            }
        );
    }

    #[test]
    fn snapshot_set_proof_empty_source_query_id_is_invalid() {
        let proof = SnapshotSetProof {
            source_query_id: vec![],
            snapshots: vec![SnapshotRef {
                snapshot_id: vec![1],
                object_id: None,
            }],
        };
        assert_eq!(
            validate_snapshot_set_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "SnapshotSetProof missing source_query_id"
            }
        );
    }

    #[test]
    fn snapshot_set_proof_nonempty_is_valid() {
        let proof = SnapshotSetProof {
            source_query_id: vec![1],
            snapshots: vec![SnapshotRef {
                snapshot_id: vec![1],
                object_id: None,
            }],
        };
        assert_eq!(
            validate_snapshot_set_proof(&proof),
            ProofStructuralResult::Valid
        );
    }

    #[test]
    fn event_set_proof_empty_is_invalid() {
        let proof = EventSetProof {
            source_query_id: vec![1],
            events: vec![],
            related_objects: vec![],
        };
        assert_eq!(
            validate_event_set_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "EventSetProof has empty asserted set"
            }
        );
    }

    #[test]
    fn event_set_proof_empty_source_query_id_is_invalid() {
        let proof = EventSetProof {
            source_query_id: vec![],
            events: vec![EventRef {
                stream_id: vec![1],
                seq: 1,
                event_hash: Some(crate::protocol::Digest {
                    algorithm: 1,
                    value: vec![2; 32],
                }),
            }],
            related_objects: vec![],
        };
        assert_eq!(
            validate_event_set_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "EventSetProof missing source_query_id"
            }
        );
    }

    #[test]
    fn event_set_proof_nonempty_is_valid() {
        let proof = EventSetProof {
            source_query_id: vec![1],
            events: vec![EventRef {
                stream_id: vec![1],
                seq: 1,
                event_hash: Some(crate::protocol::Digest {
                    algorithm: 1,
                    value: vec![2; 32],
                }),
            }],
            related_objects: vec![],
        };
        assert_eq!(
            validate_event_set_proof(&proof),
            ProofStructuralResult::Valid
        );
    }

    #[test]
    fn snapshot_set_proof_empty_snapshot_id_is_invalid() {
        let proof = SnapshotSetProof {
            source_query_id: vec![1],
            snapshots: vec![SnapshotRef {
                snapshot_id: vec![],
                object_id: None,
            }],
        };
        assert_eq!(
            validate_snapshot_set_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "SnapshotSetProof snapshot_ref missing snapshot_id"
            }
        );
    }

    #[test]
    fn snapshot_set_proof_empty_object_id_is_invalid() {
        let proof = SnapshotSetProof {
            source_query_id: vec![1],
            snapshots: vec![SnapshotRef {
                snapshot_id: vec![1],
                object_id: Some(vec![]),
            }],
        };
        assert_eq!(
            validate_snapshot_set_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "SnapshotSetProof snapshot_ref empty object_id"
            }
        );
    }

    #[test]
    fn event_set_proof_missing_event_hash_is_invalid() {
        let proof = EventSetProof {
            source_query_id: vec![1],
            events: vec![EventRef {
                stream_id: vec![1],
                seq: 1,
                event_hash: None,
            }],
            related_objects: vec![],
        };
        assert_eq!(
            validate_event_set_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "EventSetProof event_ref missing event_hash"
            }
        );
    }

    #[test]
    fn event_set_proof_non_sha256_event_hash_is_invalid() {
        let proof = EventSetProof {
            source_query_id: vec![1],
            events: vec![EventRef {
                stream_id: vec![1],
                seq: 1,
                event_hash: Some(crate::protocol::Digest {
                    algorithm: 0,
                    value: vec![2; 32],
                }),
            }],
            related_objects: vec![],
        };
        assert_eq!(
            validate_event_set_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "EventSetProof event_ref event_hash algorithm is not SHA-256"
            }
        );
    }

    #[test]
    fn event_set_proof_empty_related_object_is_invalid() {
        let proof = EventSetProof {
            source_query_id: vec![1],
            events: vec![EventRef {
                stream_id: vec![1],
                seq: 1,
                event_hash: Some(crate::protocol::Digest {
                    algorithm: 1,
                    value: vec![2; 32],
                }),
            }],
            related_objects: vec![ObjectRef {
                object_id: vec![],
                object_kind: None,
            }],
        };
        assert_eq!(
            validate_event_set_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "EventSetProof related_object missing object_id"
            }
        );
    }

    #[test]
    fn event_set_proof_invalid_related_object_kind_is_invalid() {
        let proof = EventSetProof {
            source_query_id: vec![1],
            events: vec![EventRef {
                stream_id: vec![1],
                seq: 1,
                event_hash: Some(crate::protocol::Digest {
                    algorithm: 1,
                    value: vec![2; 32],
                }),
            }],
            related_objects: vec![ObjectRef {
                object_id: vec![1],
                object_kind: Some(ObjectKind::Unspecified as i32),
            }],
        };
        assert_eq!(
            validate_event_set_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "ObjectRef object_kind is invalid"
            }
        );
    }

    #[test]
    fn object_assertion_proof_missing_ref_is_invalid() {
        let proof = ObjectAssertionProof {
            source_query_id: vec![1],
            object_ref: None,
            exists: true,
            bundled_result_object: None,
        };
        assert_eq!(
            validate_object_assertion_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "ObjectAssertionProof missing object_ref"
            }
        );
    }

    #[test]
    fn object_assertion_proof_empty_source_query_id_is_invalid() {
        let proof = ObjectAssertionProof {
            source_query_id: vec![],
            object_ref: Some(ObjectRef {
                object_id: vec![1],
                object_kind: None,
            }),
            exists: true,
            bundled_result_object: None,
        };
        assert_eq!(
            validate_object_assertion_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "ObjectAssertionProof missing source_query_id"
            }
        );
    }

    #[test]
    fn object_assertion_proof_with_ref_is_valid() {
        let proof = ObjectAssertionProof {
            source_query_id: vec![1],
            object_ref: Some(ObjectRef {
                object_id: vec![1],
                object_kind: None,
            }),
            exists: true,
            bundled_result_object: None,
        };
        assert_eq!(
            validate_object_assertion_proof(&proof),
            ProofStructuralResult::Valid
        );
    }

    #[test]
    fn object_assertion_proof_empty_ref_is_invalid() {
        let proof = ObjectAssertionProof {
            source_query_id: vec![1],
            object_ref: Some(ObjectRef {
                object_id: vec![],
                object_kind: None,
            }),
            exists: true,
            bundled_result_object: None,
        };
        assert_eq!(
            validate_object_assertion_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "ObjectAssertionProof object_ref missing object_id"
            }
        );
    }

    #[test]
    fn aggregate_summary_overlapping_responders_is_invalid() {
        let responder = IdentityRef {
            identity_id: vec![1],
            identity_kind: None,
            key_hint: None,
        };
        let proof = AggregateSummaryProof {
            source_query_id: vec![1],
            included_responders: vec![responder.clone()],
            excluded_responders: vec![responder],
            total_trust_score: 0,
            trust_policy_object: None,
        };
        assert_eq!(
            validate_aggregate_summary_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "AggregateSummaryProof has overlapping included/excluded responders"
            }
        );
    }

    #[test]
    fn result_fragment_proof_missing_fragment_is_invalid() {
        let proof = ResultFragmentProof { fragment: None };
        assert_eq!(
            validate_result_fragment_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "ResultFragmentProof missing fragment"
            }
        );
    }

    #[test]
    fn result_fragment_proof_empty_query_id_is_invalid() {
        let proof = ResultFragmentProof {
            fragment: Some(crate::protocol::QueryResultFragment {
                fragment_version: 1,
                query_id: vec![],
                responder: None,
                answered_at: None,
                completeness: 0,
                snapshot_refs: vec![],
                event_refs: vec![],
                object_refs: vec![],
                proof_objects: vec![],
                omission_reason: String::new(),
                bundled_result_object: None,
                result_metadata: None,
                signature: None,
            }),
        };
        assert_eq!(
            validate_result_fragment_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "ResultFragmentProof fragment missing query_id"
            }
        );
    }

    #[test]
    fn result_fragment_proof_with_query_id_is_valid() {
        let proof = ResultFragmentProof {
            fragment: Some(crate::protocol::QueryResultFragment {
                fragment_version: 1,
                query_id: vec![1],
                responder: None,
                answered_at: None,
                completeness: 0,
                snapshot_refs: vec![],
                event_refs: vec![],
                object_refs: vec![],
                proof_objects: vec![],
                omission_reason: String::new(),
                bundled_result_object: None,
                result_metadata: None,
                signature: None,
            }),
        };
        assert_eq!(
            validate_result_fragment_proof(&proof),
            ProofStructuralResult::Valid
        );
    }

    #[test]
    fn aggregate_summary_empty_source_query_id_is_invalid() {
        let proof = AggregateSummaryProof {
            source_query_id: vec![],
            included_responders: vec![],
            excluded_responders: vec![],
            total_trust_score: 0,
            trust_policy_object: None,
        };
        assert_eq!(
            validate_aggregate_summary_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "AggregateSummaryProof missing source_query_id"
            }
        );
    }

    #[test]
    fn aggregate_summary_disjoint_responders_is_valid() {
        let proof = AggregateSummaryProof {
            source_query_id: vec![1],
            included_responders: vec![IdentityRef {
                identity_id: vec![1],
                identity_kind: None,
                key_hint: None,
            }],
            excluded_responders: vec![IdentityRef {
                identity_id: vec![2],
                identity_kind: None,
                key_hint: None,
            }],
            total_trust_score: 0,
            trust_policy_object: None,
        };
        assert_eq!(
            validate_aggregate_summary_proof(&proof),
            ProofStructuralResult::Valid
        );
    }

    #[test]
    fn aggregate_summary_empty_responder_is_invalid() {
        let proof = AggregateSummaryProof {
            source_query_id: vec![1],
            included_responders: vec![IdentityRef {
                identity_id: vec![],
                identity_kind: None,
                key_hint: None,
            }],
            excluded_responders: vec![],
            total_trust_score: 0,
            trust_policy_object: None,
        };
        assert_eq!(
            validate_aggregate_summary_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "AggregateSummaryProof responder identity_id is empty"
            }
        );
    }

    #[test]
    fn aggregate_summary_invalid_responder_kind_is_invalid() {
        let proof = AggregateSummaryProof {
            source_query_id: vec![1],
            included_responders: vec![IdentityRef {
                identity_id: vec![1],
                identity_kind: Some(999_999),
                key_hint: None,
            }],
            excluded_responders: vec![],
            total_trust_score: 0,
            trust_policy_object: None,
        };
        assert_eq!(
            validate_aggregate_summary_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "AggregateSummaryProof responder identity_kind is invalid"
            }
        );
    }

    #[test]
    fn trust_policy_proof_missing_both_objects_is_invalid() {
        let proof = TrustPolicyProof {
            source_query_id: vec![1],
            policy_object: None,
            assignments_object: None,
        };
        assert_eq!(
            validate_trust_policy_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "TrustPolicyProof missing both policy_object and assignments_object"
            }
        );
    }

    #[test]
    fn trust_policy_proof_empty_source_query_id_is_invalid() {
        let proof = TrustPolicyProof {
            source_query_id: vec![],
            policy_object: Some(ObjectRef {
                object_id: vec![1],
                object_kind: None,
            }),
            assignments_object: None,
        };
        assert_eq!(
            validate_trust_policy_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "TrustPolicyProof missing source_query_id"
            }
        );
    }

    #[test]
    fn trust_policy_proof_with_policy_object_is_valid() {
        let proof = TrustPolicyProof {
            source_query_id: vec![1],
            policy_object: Some(ObjectRef {
                object_id: vec![1],
                object_kind: None,
            }),
            assignments_object: None,
        };
        assert_eq!(
            validate_trust_policy_proof(&proof),
            ProofStructuralResult::Valid
        );
    }

    #[test]
    fn trust_policy_proof_empty_policy_object_is_invalid() {
        let proof = TrustPolicyProof {
            source_query_id: vec![1],
            policy_object: Some(ObjectRef {
                object_id: vec![],
                object_kind: None,
            }),
            assignments_object: None,
        };
        assert_eq!(
            validate_trust_policy_proof(&proof),
            ProofStructuralResult::Invalid {
                reason: "TrustPolicyProof policy_object missing object_id"
            }
        );
    }

    fn valid_proof_bundle() -> ProofBundle {
        ProofBundle {
            bundle_version: 1,
            payload_type: ProofPayloadType::StreamHeads as i32,
            source_query_id: vec![1, 2, 3],
            payload_object: Some(ObjectRef {
                object_id: vec![4, 5, 6],
                object_kind: Some(1),
            }),
            supporting_objects: vec![ObjectRef {
                object_id: vec![7, 8, 9],
                object_kind: Some(1),
            }],
            signature: None,
        }
    }

    #[test]
    fn proof_bundle_valid_is_advisory_accept() {
        let bundle = valid_proof_bundle();

        let result = validate_proof_bundle(&bundle, Some(&[1, 2, 3]), None, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
        assert_eq!(
            result.derived.as_map().unwrap().get("advisory_only"),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn proof_bundle_query_mismatch_rejected() {
        let bundle = valid_proof_bundle();

        let result = validate_proof_bundle(&bundle, Some(&[9, 9, 9]), None, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TargetMismatch));
    }

    #[test]
    fn proof_bundle_missing_payload_object_rejected() {
        let mut bundle = valid_proof_bundle();
        bundle.payload_object = None;

        let result = validate_proof_bundle(&bundle, Some(&[1, 2, 3]), None, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn proof_bundle_invalid_payload_object_kind_rejected() {
        let mut bundle = valid_proof_bundle();
        bundle.payload_object.as_mut().unwrap().object_kind = Some(ObjectKind::Unspecified as i32);

        let result = validate_proof_bundle(&bundle, Some(&[1, 2, 3]), None, &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn proof_bundle_unavailable_payload_defers() {
        let bundle = valid_proof_bundle();
        let available = std::collections::HashSet::new();

        let result = validate_proof_bundle(&bundle, Some(&[1, 2, 3]), Some(&available), &[]);

        assert_eq!(result.verdict, crate::result::Verdict::Defer);
        assert_eq!(result.reason_code, Some(ReasonCode::MissingDependency));
    }

    #[test]
    fn proof_bundle_disallowed_payload_rejected_by_policy() {
        let bundle = valid_proof_bundle();

        let result = validate_proof_bundle(
            &bundle,
            Some(&[1, 2, 3]),
            None,
            &[ProofPayloadType::TrustPolicy as i32],
        );

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::PolicyDenied));
    }

    fn valid_federated_aggregate_descriptor() -> FederatedAggregateDescriptor {
        FederatedAggregateDescriptor {
            descriptor_version: 1,
            aggregate_id: vec![1, 2, 3],
            source_query_id: vec![4, 5, 6],
            aggregator: Some(IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: None,
                key_hint: None,
            }),
            aggregated_at: Some(prost_types::Timestamp {
                seconds: 1_700_000_000,
                nanos: 0,
            }),
            input_fragments: vec![ObjectRef {
                object_id: vec![10, 11, 12],
                object_kind: Some(1),
            }],
            aggregation_policy_object: None,
            payload_object: Some(ObjectRef {
                object_id: vec![13, 14, 15],
                object_kind: Some(1),
            }),
            signature: None,
        }
    }

    #[test]
    fn federated_aggregate_descriptor_valid_is_advisory_accept() {
        let descriptor = valid_federated_aggregate_descriptor();

        let result = validate_federated_aggregate_descriptor(&descriptor, Some(&[4, 5, 6]));

        assert_eq!(result.verdict, crate::result::Verdict::Accept);
        assert_eq!(
            result.derived.as_map().unwrap().get("advisory_only"),
            Some(&Value::Bool(true))
        );
    }

    #[test]
    fn federated_aggregate_descriptor_query_mismatch_rejected() {
        let descriptor = valid_federated_aggregate_descriptor();

        let result = validate_federated_aggregate_descriptor(&descriptor, Some(&[9, 9, 9]));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::TargetMismatch));
    }

    #[test]
    fn federated_aggregate_descriptor_missing_input_rejected() {
        let mut descriptor = valid_federated_aggregate_descriptor();
        descriptor.input_fragments.clear();

        let result = validate_federated_aggregate_descriptor(&descriptor, Some(&[4, 5, 6]));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn federated_aggregate_descriptor_invalid_aggregator_kind_rejected() {
        let mut descriptor = valid_federated_aggregate_descriptor();
        descriptor.aggregator.as_mut().unwrap().identity_kind =
            Some(IdentityKind::Unspecified as i32);

        let result = validate_federated_aggregate_descriptor(&descriptor, Some(&[4, 5, 6]));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn federated_aggregate_descriptor_invalid_aggregated_at_rejected() {
        let mut descriptor = valid_federated_aggregate_descriptor();
        descriptor.aggregated_at = Some(prost_types::Timestamp {
            seconds: 1_700_000_000,
            nanos: 1_000_000_000,
        });

        let result = validate_federated_aggregate_descriptor(&descriptor, Some(&[4, 5, 6]));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn federated_aggregate_descriptor_invalid_input_fragment_kind_rejected() {
        let mut descriptor = valid_federated_aggregate_descriptor();
        descriptor.input_fragments[0].object_kind = Some(999_999);

        let result = validate_federated_aggregate_descriptor(&descriptor, Some(&[4, 5, 6]));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn federated_aggregate_descriptor_missing_payload_rejected() {
        let mut descriptor = valid_federated_aggregate_descriptor();
        descriptor.payload_object = None;

        let result = validate_federated_aggregate_descriptor(&descriptor, Some(&[4, 5, 6]));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }

    #[test]
    fn federated_aggregate_descriptor_empty_policy_object_rejected() {
        let mut descriptor = valid_federated_aggregate_descriptor();
        descriptor.aggregation_policy_object = Some(ObjectRef {
            object_id: vec![],
            object_kind: None,
        });

        let result = validate_federated_aggregate_descriptor(&descriptor, Some(&[4, 5, 6]));

        assert_eq!(result.verdict, crate::result::Verdict::Reject);
        assert_eq!(result.reason_code, Some(ReasonCode::StructuralInvalid));
    }
}
