use crate::collections::{BTreeSet, HashMap, VecDeque};
use crate::prelude::v1::*;
use crate::time::{Duration, SystemTime, UNIX_EPOCH};

use super::types::*;
use edgerun_capabilities::{
    validate_descriptor, validate_grant, CapabilityAccessClass, CapabilityConstraint,
    CapabilityConstraintKind, CapabilityDescriptor, CapabilityError, CapabilityGrant,
    CapabilityInvocation, CapabilityModality, CapabilityOperation, CapabilityRequest,
    CapabilityRevocation, CapabilityRole, CapabilitySelector,
};
use edgerun_crypto::sha2::Digest;
use edgerun_proto::edgerun::v0::common::{IdentityRef, NodeRef};
use prost_types::{Duration as ProstDuration, Timestamp};

pub(crate) fn dedupe_i32(values: Vec<i32>) -> Vec<i32> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for value in values {
        if seen.insert(value) {
            out.push(value);
        }
    }
    out
}

pub(crate) fn constraint_identity(
    constraint: &CapabilityConstraint,
) -> (
    i32,
    Option<u64>,
    String,
    Option<i64>,
    Option<u64>,
    Option<i64>,
) {
    (
        constraint.kind,
        constraint.uint_value,
        constraint.string_value.clone(),
        constraint
            .duration_value
            .as_ref()
            .map(|value| value.seconds),
        constraint
            .rate_limit
            .as_ref()
            .map(|value| value.max_operations),
        constraint
            .rate_limit
            .as_ref()
            .and_then(|value| value.per.as_ref().map(|per| per.seconds)),
    )
}

pub(crate) fn dedupe_constraints(
    constraints: Vec<CapabilityConstraint>,
) -> Vec<CapabilityConstraint> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for constraint in constraints {
        if seen.insert(constraint_identity(&constraint)) {
            out.push(constraint);
        }
    }
    out
}

pub(crate) fn has_constraint_kind(
    constraints: &[CapabilityConstraint],
    kind: CapabilityConstraintKind,
) -> bool {
    constraints
        .iter()
        .any(|constraint| CapabilityConstraintKind::try_from(constraint.kind).ok() == Some(kind))
}

pub(crate) fn requires_user_presence(
    descriptor: &CapabilityDescriptor,
    operations: &[i32],
    constraints: &[CapabilityConstraint],
) -> bool {
    if has_constraint_kind(constraints, CapabilityConstraintKind::RequireUserPresence) {
        return true;
    }

    let role = CapabilityRole::try_from(descriptor.role).unwrap_or(CapabilityRole::Unspecified);

    let sensitive_modality = descriptor
        .modalities
        .iter()
        .filter_map(|value| CapabilityModality::try_from(*value).ok())
        .any(|modality| {
            matches!(
                modality,
                CapabilityModality::Auditory
                    | CapabilityModality::Visual
                    | CapabilityModality::Biometric
                    | CapabilityModality::Touch
            )
        });
    let sensitive_operation = operations
        .iter()
        .filter_map(|value| CapabilityOperation::try_from(*value).ok())
        .any(|operation| {
            matches!(
                operation,
                CapabilityOperation::Capture
                    | CapabilityOperation::Control
                    | CapabilityOperation::Render
                    | CapabilityOperation::Sign
            )
        });

    (sensitive_modality || matches!(role, CapabilityRole::Input | CapabilityRole::SecureElement))
        && sensitive_operation
}

pub(crate) fn timestamp_from_system_time(value: SystemTime) -> Timestamp {
    let duration = value.duration_since(UNIX_EPOCH).unwrap_or_default();
    Timestamp {
        seconds: duration.as_secs() as i64,
        nanos: duration.subsec_nanos() as i32,
    }
}

pub(crate) fn system_time_from_timestamp(value: &Timestamp) -> Option<SystemTime> {
    if value.seconds < 0 || value.nanos < 0 {
        return None;
    }
    Some(UNIX_EPOCH + Duration::new(value.seconds as u64, value.nanos as u32))
}

pub(crate) fn duration_from_prost(value: &ProstDuration) -> Option<Duration> {
    if value.seconds < 0 || value.nanos < 0 {
        return None;
    }
    Some(Duration::new(value.seconds as u64, value.nanos as u32))
}

pub(crate) fn revocation_id_for(grant_id: &[u8], reason: &RevocationReason) -> Vec<u8> {
    let mut h = edgerun_crypto::sha2::Sha256::new();
    h.update(grant_id);
    h.update(reason.as_str().as_bytes());
    h.finalize().to_vec()
}
