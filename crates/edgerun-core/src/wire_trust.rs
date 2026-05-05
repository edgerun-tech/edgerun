//! Trust/identity canonical wire helpers.
//!
//! This is the migration seam away from protobuf canonicalization for signed
//! trust records. Field tags intentionally follow the v0 proto schema numbers,
//! while the bytes are encoded with edgerun-wire's deterministic custom format.

use alloc::vec::Vec;

use crate::protocol::*;
use crate::protocol::trust::assurance_claim::Subject as AssuranceSubject;
use crate::protocol::trust::revocation_record::Target as RevocationTarget;
use edgerun_wire::{bytes, boolv, canonical_bytes, field, i64v, struct_value, text, u64v, WireValue};

#[must_use]
pub fn identity_record_signable_bytes(value: &IdentityRecord) -> Vec<u8> {
    canonical_bytes(&identity_record_value(value, true))
}

#[must_use]
pub fn identity_record_full_bytes(value: &IdentityRecord) -> Vec<u8> {
    canonical_bytes(&identity_record_value(value, false))
}

#[must_use]
pub fn delegation_record_signable_bytes(value: &DelegationRecord) -> Vec<u8> {
    canonical_bytes(&delegation_record_value(value, true))
}

#[must_use]
pub fn delegation_record_full_bytes(value: &DelegationRecord) -> Vec<u8> {
    canonical_bytes(&delegation_record_value(value, false))
}

#[must_use]
pub fn revocation_record_signable_bytes(value: &RevocationRecord) -> Vec<u8> {
    canonical_bytes(&revocation_record_value(value, true))
}

#[must_use]
pub fn revocation_record_full_bytes(value: &RevocationRecord) -> Vec<u8> {
    canonical_bytes(&revocation_record_value(value, false))
}

#[must_use]
pub fn assurance_claim_signable_bytes(value: &AssuranceClaim) -> Vec<u8> {
    canonical_bytes(&assurance_claim_value(value, true))
}

#[must_use]
pub fn assurance_claim_full_bytes(value: &AssuranceClaim) -> Vec<u8> {
    canonical_bytes(&assurance_claim_value(value, false))
}

fn digest_value(value: &Digest) -> WireValue {
    struct_value(alloc::vec![
        field(1, u64v(value.algorithm as u64)),
        field(2, bytes(&value.value)),
    ])
}

fn signature_value(value: &Signature) -> WireValue {
    struct_value(alloc::vec![
        field(1, u64v(value.algorithm as u64)),
        field(2, bytes(&value.value)),
    ])
}

fn timestamp_value(value: &Timestamp) -> WireValue {
    struct_value(alloc::vec![
        field(1, i64v(value.seconds)),
        field(2, i64v(value.nanos as i64)),
    ])
}

fn duration_value(value: &Duration) -> WireValue {
    struct_value(alloc::vec![
        field(1, i64v(value.seconds)),
        field(2, i64v(value.nanos as i64)),
    ])
}

fn time_window_value(value: &TimeWindow) -> WireValue {
    let mut fields = Vec::new();
    if let Some(v) = &value.not_before {
        fields.push(field(1, timestamp_value(v)));
    }
    if let Some(v) = &value.expires_at {
        fields.push(field(2, timestamp_value(v)));
    }
    struct_value(fields)
}

fn rate_limit_value(value: &RateLimit) -> WireValue {
    let mut fields = alloc::vec![field(1, u64v(value.max_operations))];
    if let Some(v) = &value.per {
        fields.push(field(2, duration_value(v)));
    }
    struct_value(fields)
}

fn identity_ref_value(value: &IdentityRef) -> WireValue {
    let mut fields = alloc::vec![field(1, bytes(&value.identity_id))];
    if let Some(v) = value.identity_kind {
        fields.push(field(2, u64v(v as u64)));
    }
    if let Some(v) = &value.key_hint {
        fields.push(field(3, bytes(v)));
    }
    struct_value(fields)
}

fn node_ref_value(value: &NodeRef) -> WireValue {
    struct_value(alloc::vec![field(1, bytes(&value.node_id))])
}

fn stream_ref_value(value: &StreamRef) -> WireValue {
    struct_value(alloc::vec![field(1, bytes(&value.stream_id))])
}

fn object_ref_value(value: &ObjectRef) -> WireValue {
    let mut fields = alloc::vec![field(1, bytes(&value.object_id))];
    if let Some(v) = value.object_kind {
        fields.push(field(2, u64v(v as u64)));
    }
    struct_value(fields)
}

fn delegation_ref_value(value: &DelegationRef) -> WireValue {
    let mut fields = alloc::vec![field(1, bytes(&value.delegation_id))];
    if let Some(v) = &value.delegation_hash {
        fields.push(field(2, digest_value(v)));
    }
    struct_value(fields)
}

fn list(values: Vec<WireValue>) -> WireValue {
    WireValue::List(values)
}

fn identity_ref_list(values: &[IdentityRef]) -> WireValue {
    list(values.iter().map(identity_ref_value).collect())
}

fn node_ref_list(values: &[NodeRef]) -> WireValue {
    list(values.iter().map(node_ref_value).collect())
}

fn stream_ref_list(values: &[StreamRef]) -> WireValue {
    list(values.iter().map(stream_ref_value).collect())
}

fn object_ref_list(values: &[ObjectRef]) -> WireValue {
    list(values.iter().map(object_ref_value).collect())
}

fn i32_list(values: &[i32]) -> WireValue {
    list(values.iter().map(|v| u64v(*v as u64)).collect())
}

fn string_list(values: &[alloc::string::String]) -> WireValue {
    list(values.iter().map(text).collect())
}

fn assurance_requirement_value(value: &AssuranceRequirement) -> WireValue {
    let mut fields = alloc::vec![
        field(1, u64v(value.assurance_version as u64)),
        field(2, u64v(value.required_class as u64)),
        field(3, identity_ref_list(&value.acceptable_attesters)),
    ];
    if let Some(v) = &value.max_evidence_age {
        fields.push(field(4, duration_value(v)));
    }
    if let Some(v) = &value.assurance_metadata {
        fields.push(field(5, object_ref_value(v)));
    }
    struct_value(fields)
}

fn scope_descriptor_value(value: &ScopeDescriptor) -> WireValue {
    let mut fields = alloc::vec![
        field(1, u64v(value.scope_version as u64)),
        field(2, u64v(value.scope_kind as u64)),
        field(3, node_ref_list(&value.target_nodes)),
        field(4, stream_ref_list(&value.target_streams)),
        field(5, i32_list(&value.target_object_kinds)),
        field(6, string_list(&value.target_view_types)),
        field(7, string_list(&value.target_domains)),
    ];
    if let Some(v) = &value.time_bounds {
        fields.push(field(8, time_window_value(v)));
    }
    if let Some(v) = &value.scope_metadata {
        fields.push(field(9, object_ref_value(v)));
    }
    struct_value(fields)
}

fn constraint_set_value(value: &ConstraintSet) -> WireValue {
    let mut fields = alloc::vec![
        field(1, u64v(value.constraint_version as u64)),
        field(8, i32_list(&value.requires_transport_classes)),
        field(9, string_list(&value.requires_location_classes)),
        field(10, u64v(value.export_policy as u64)),
        field(11, i32_list(&value.execution_class_limits)),
        field(12, i32_list(&value.storage_class_limits)),
    ];
    if let Some(v) = &value.not_before {
        fields.push(field(2, timestamp_value(v)));
    }
    if let Some(v) = &value.expires_at {
        fields.push(field(3, timestamp_value(v)));
    }
    if let Some(v) = value.max_uses {
        fields.push(field(4, u64v(v)));
    }
    if let Some(v) = &value.rate_limit {
        fields.push(field(5, rate_limit_value(v)));
    }
    if let Some(v) = value.requires_local_session {
        fields.push(field(6, boolv(v)));
    }
    if let Some(v) = value.requires_user_presence {
        fields.push(field(7, boolv(v)));
    }
    if let Some(v) = &value.constraint_metadata {
        fields.push(field(13, object_ref_value(v)));
    }
    struct_value(fields)
}

fn capability_descriptor_value(value: &CapabilityDescriptor) -> WireValue {
    let mut fields = alloc::vec![
        field(1, u64v(value.capability_version as u64)),
        field(2, u64v(value.capability_kind as u64)),
        field(3, string_list(&value.actions)),
        field(6, u64v(value.delegation_policy as u64)),
    ];
    if let Some(v) = &value.scope {
        fields.push(field(4, scope_descriptor_value(v)));
    }
    if let Some(v) = &value.constraints {
        fields.push(field(5, constraint_set_value(v)));
    }
    if let Some(v) = &value.minimum_assurance {
        fields.push(field(7, assurance_requirement_value(v)));
    }
    if let Some(v) = &value.capability_metadata {
        fields.push(field(8, object_ref_value(v)));
    }
    struct_value(fields)
}

fn identity_record_value(value: &IdentityRecord, signable: bool) -> WireValue {
    let mut fields = alloc::vec![
        field(1, u64v(value.record_version as u64)),
        field(2, bytes(&value.identity_id)),
        field(3, u64v(value.identity_kind as u64)),
        field(4, u64v(value.key_algorithm as u64)),
        field(5, bytes(&value.public_key)),
        field(8, object_ref_list(&value.assurance_claim_objects)),
    ];
    if let Some(v) = &value.created_at {
        fields.push(field(6, timestamp_value(v)));
    }
    if let Some(v) = &value.supersedes_identity {
        fields.push(field(7, identity_ref_value(v)));
    }
    if let Some(v) = &value.metadata_object {
        fields.push(field(9, object_ref_value(v)));
    }
    if !signable {
        if let Some(v) = &value.signature {
            fields.push(field(10, signature_value(v)));
        }
    }
    struct_value(fields)
}

fn assurance_claim_value(value: &AssuranceClaim, signable: bool) -> WireValue {
    let mut fields = alloc::vec![
        field(1, u64v(value.claim_version as u64)),
        field(4, u64v(value.assurance_class as u64)),
        field(9, text(&value.claim_note)),
    ];
    if let Some(subject) = &value.subject {
        match subject {
            AssuranceSubject::SubjectIdentity(v) => fields.push(field(2, identity_ref_value(v))),
            AssuranceSubject::SubjectNode(v) => fields.push(field(3, node_ref_value(v))),
        }
    }
    if let Some(v) = &value.attester {
        fields.push(field(5, identity_ref_value(v)));
    }
    if let Some(v) = &value.issued_at {
        fields.push(field(6, timestamp_value(v)));
    }
    if let Some(v) = &value.expires_at {
        fields.push(field(7, timestamp_value(v)));
    }
    if let Some(v) = &value.evidence_object {
        fields.push(field(8, object_ref_value(v)));
    }
    if !signable {
        if let Some(v) = &value.signature {
            fields.push(field(10, signature_value(v)));
        }
    }
    struct_value(fields)
}

fn delegation_record_value(value: &DelegationRecord, signable: bool) -> WireValue {
    let mut fields = alloc::vec![
        field(1, u64v(value.record_version as u64)),
        field(2, bytes(&value.delegation_id)),
        field(10, identity_ref_list(&value.revocation_authorities)),
    ];
    if let Some(v) = &value.issuer {
        fields.push(field(3, identity_ref_value(v)));
    }
    if let Some(v) = &value.recipient {
        fields.push(field(4, identity_ref_value(v)));
    }
    if let Some(v) = &value.issued_at {
        fields.push(field(5, timestamp_value(v)));
    }
    if let Some(v) = &value.not_before {
        fields.push(field(6, timestamp_value(v)));
    }
    if let Some(v) = &value.expires_at {
        fields.push(field(7, timestamp_value(v)));
    }
    if let Some(v) = &value.capability {
        fields.push(field(8, capability_descriptor_value(v)));
    }
    if let Some(v) = &value.parent_delegation {
        fields.push(field(9, delegation_ref_value(v)));
    }
    if let Some(v) = &value.delegation_metadata {
        fields.push(field(11, object_ref_value(v)));
    }
    if !signable {
        if let Some(v) = &value.signature {
            fields.push(field(12, signature_value(v)));
        }
    }
    struct_value(fields)
}

fn revocation_record_value(value: &RevocationRecord, signable: bool) -> WireValue {
    let mut fields = alloc::vec![
        field(1, u64v(value.record_version as u64)),
        field(2, bytes(&value.revocation_id)),
        field(6, u64v(value.revocation_kind as u64)),
        field(12, text(&value.reason_code)),
        field(13, bytes(&value.replacement_id)),
    ];
    if let Some(v) = &value.issuer {
        fields.push(field(3, identity_ref_value(v)));
    }
    if let Some(v) = &value.issued_at {
        fields.push(field(4, timestamp_value(v)));
    }
    if let Some(v) = &value.effective_at {
        fields.push(field(5, timestamp_value(v)));
    }
    if let Some(target) = &value.target {
        match target {
            RevocationTarget::TargetDelegation(v) => {
                fields.push(field(7, delegation_ref_value(v)));
            }
            RevocationTarget::TargetIdentity(v) => {
                fields.push(field(8, identity_ref_value(v)));
            }
            RevocationTarget::TargetNode(v) => {
                fields.push(field(9, node_ref_value(v)));
            }
            RevocationTarget::TargetObject(v) => {
                fields.push(field(10, object_ref_value(v)));
            }
        }
    }
    if let Some(v) = &value.scope_override {
        fields.push(field(11, scope_descriptor_value(v)));
    }
    if let Some(v) = &value.revocation_metadata {
        fields.push(field(14, object_ref_value(v)));
    }
    if !signable {
        if let Some(v) = &value.signature {
            fields.push(field(15, signature_value(v)));
        }
    }
    struct_value(fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_signable_bytes_drop_signature() {
        let mut record = IdentityRecord {
            record_version: 1,
            identity_id: b"id".to_vec(),
            identity_kind: 2,
            key_algorithm: 1,
            public_key: b"pub".to_vec(),
            ..IdentityRecord::default()
        };
        let without = identity_record_signable_bytes(&record);
        record.signature = Some(Signature {
            algorithm: 1,
            value: alloc::vec![0xab; 64],
        });
        let signable = identity_record_signable_bytes(&record);
        let full = identity_record_full_bytes(&record);
        assert_eq!(without, signable);
        assert_ne!(signable, full);
    }

    #[test]
    fn assurance_claim_signable_bytes_drop_signature() {
        let mut claim = AssuranceClaim {
            claim_version: 1,
            assurance_class: 1,
            claim_note: "ok".into(),
            ..AssuranceClaim::default()
        };
        let without = assurance_claim_signable_bytes(&claim);
        claim.signature = Some(Signature {
            algorithm: 1,
            value: alloc::vec![0xab; 64],
        });
        let signable = assurance_claim_signable_bytes(&claim);
        let full = assurance_claim_full_bytes(&claim);
        assert_eq!(without, signable);
        assert_ne!(signable, full);
    }

    #[test]
    fn delegation_signable_bytes_are_deterministic() {
        let record = DelegationRecord {
            record_version: 1,
            delegation_id: b"d".to_vec(),
            ..DelegationRecord::default()
        };
        assert_eq!(
            delegation_record_signable_bytes(&record),
            delegation_record_signable_bytes(&record)
        );
    }

    #[test]
    fn revocation_target_changes_bytes() {
        let mut record = RevocationRecord {
            record_version: 1,
            revocation_id: b"r".to_vec(),
            revocation_kind: 1,
            ..RevocationRecord::default()
        };
        let a = revocation_record_signable_bytes(&record);
        record.target = Some(RevocationTarget::TargetNode(NodeRef {
            node_id: b"node".to_vec(),
        }));
        let b = revocation_record_signable_bytes(&record);
        assert_ne!(a, b);
    }
}
