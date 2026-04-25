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

use edgerun_proto::edgerun::v0::access::{
    AggregateSummaryProof, EventSetProof, ObjectAssertionProof, SnapshotSetProof, TrustPolicyProof,
};

/// Structural validation result for a proof object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProofStructuralResult {
    Valid,
    Invalid { reason: &'static str },
}

/// Validates the structural integrity of a SnapshotSetProof.
///
/// §8.1: SnapshotSetProof with empty asserted set is structurally invalid.
pub fn validate_snapshot_set_proof(proof: &SnapshotSetProof) -> ProofStructuralResult {
    if proof.snapshots.is_empty() {
        ProofStructuralResult::Invalid {
            reason: "SnapshotSetProof has empty asserted set",
        }
    } else {
        ProofStructuralResult::Valid
    }
}

/// Validates the structural integrity of an EventSetProof.
///
/// §8.1: EventSetProof with empty asserted set is structurally invalid.
pub fn validate_event_set_proof(proof: &EventSetProof) -> ProofStructuralResult {
    if proof.events.is_empty() {
        ProofStructuralResult::Invalid {
            reason: "EventSetProof has empty asserted set",
        }
    } else {
        ProofStructuralResult::Valid
    }
}

/// Validates the structural integrity of an ObjectAssertionProof.
///
/// §8.1: ObjectAssertionProof without object_ref is structurally invalid.
pub fn validate_object_assertion_proof(proof: &ObjectAssertionProof) -> ProofStructuralResult {
    if proof.object_ref.is_none() {
        ProofStructuralResult::Invalid {
            reason: "ObjectAssertionProof missing object_ref",
        }
    } else {
        ProofStructuralResult::Valid
    }
}

/// Validates the structural integrity of an AggregateSummaryProof.
///
/// §8.1: AggregateSummaryProof with overlapping included and excluded
/// responders is structurally invalid.
pub fn validate_aggregate_summary_proof(proof: &AggregateSummaryProof) -> ProofStructuralResult {
    use std::collections::HashSet;
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
    ProofStructuralResult::Valid
}

/// Validates the structural integrity of a TrustPolicyProof.
///
/// §8.1: TrustPolicyProof MUST carry at least one of policy_object
/// or assignments_object.
pub fn validate_trust_policy_proof(proof: &TrustPolicyProof) -> ProofStructuralResult {
    if proof.policy_object.is_none() && proof.assignments_object.is_none() {
        ProofStructuralResult::Invalid {
            reason: "TrustPolicyProof missing both policy_object and assignments_object",
        }
    } else {
        ProofStructuralResult::Valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_proto::edgerun::v0::common::{EventRef, IdentityRef, ObjectRef, SnapshotRef};

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
    fn event_set_proof_nonempty_is_valid() {
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
            ProofStructuralResult::Valid
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
}
