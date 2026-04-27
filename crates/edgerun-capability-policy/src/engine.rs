use crate::collections::{BTreeSet, HashMap, VecDeque};
use crate::prelude::v1::*;
use crate::time::{Duration, SystemTime, UNIX_EPOCH};

use super::helpers::*;
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

#[derive(Debug, Clone)]
pub struct SimplePolicyEngine {
    pub(crate) issuer: Option<IdentityRef>,
    default_grant_duration: Duration,
    max_grant_duration: Duration,
    pub(crate) grants: HashMap<Vec<u8>, GrantRecord>,
    nonce: u64,
}

impl Default for SimplePolicyEngine {
    fn default() -> Self {
        Self::new(None)
    }
}

impl SimplePolicyEngine {
    pub fn new(issuer: Option<IdentityRef>) -> Self {
        Self {
            issuer,
            default_grant_duration: Duration::from_secs(300),
            max_grant_duration: Duration::from_secs(3600),
            grants: HashMap::new(),
            nonce: 0,
        }
    }

    pub fn with_default_grant_duration(mut self, duration: Duration) -> Self {
        self.default_grant_duration = duration;
        self
    }

    pub fn with_max_grant_duration(mut self, duration: Duration) -> Self {
        self.max_grant_duration = duration;
        self
    }

    pub fn grants(&self) -> impl Iterator<Item = &GrantRecord> {
        self.grants.values()
    }

    pub(crate) fn next_grant_id(
        &mut self,
        descriptor: &CapabilityDescriptor,
        request: &CapabilityRequest,
        now: SystemTime,
    ) -> Vec<u8> {
        self.nonce = self.nonce.wrapping_add(1);
        let now = now.duration_since(UNIX_EPOCH).unwrap_or_default();
        let mut h = edgerun_crypto::sha2::Sha256::new();
        h.update(&request.request_id);
        h.update(descriptor.provider_name.as_bytes());
        h.update(descriptor.provider_instance_id.as_bytes());
        h.update(self.nonce.to_le_bytes());
        h.update(now.as_secs().to_le_bytes());
        h.update(now.subsec_nanos().to_le_bytes());
        h.finalize().to_vec()
    }

    pub(crate) fn effective_requested_operations(
        &self,
        descriptor: &CapabilityDescriptor,
        request: &CapabilityRequest,
    ) -> Vec<i32> {
        let requested = if !request.requested_operations.is_empty() {
            request.requested_operations.clone()
        } else if let Some(selector) = &request.selector {
            if !selector.operations.is_empty() {
                selector.operations.clone()
            } else {
                descriptor.operations.clone()
            }
        } else {
            descriptor.operations.clone()
        };
        dedupe_i32(requested)
    }

    pub(crate) fn effective_access_class(
        &self,
        request: &CapabilityRequest,
    ) -> CapabilityAccessClass {
        request
            .selector
            .as_ref()
            .and_then(|selector| CapabilityAccessClass::try_from(selector.access_class).ok())
            .filter(|value| *value != CapabilityAccessClass::Unspecified)
            .unwrap_or(CapabilityAccessClass::Derived)
    }

    pub(crate) fn selector_matches(
        &self,
        descriptor: &CapabilityDescriptor,
        selector: &CapabilitySelector,
    ) -> Result<(), PolicyDecision> {
        if !selector.provider_instance_id.is_empty()
            && selector.provider_instance_id != descriptor.provider_instance_id
        {
            return Err(PolicyDecision::Deny {
                reason: "selector provider instance does not match descriptor",
            });
        }
        if selector.role != 0 && selector.role != descriptor.role {
            return Err(PolicyDecision::Deny {
                reason: "selector role does not match descriptor",
            });
        }
        if !selector.capability_id.is_empty()
            && !descriptor.capability_id.is_empty()
            && selector.capability_id != descriptor.capability_id
        {
            return Err(PolicyDecision::Deny {
                reason: "selector capability id does not match descriptor",
            });
        }
        if !selector.modalities.is_empty()
            && !selector
                .modalities
                .iter()
                .all(|value| descriptor.modalities.contains(value))
        {
            return Err(PolicyDecision::Deny {
                reason: "selector modalities are not supported by descriptor",
            });
        }
        if !selector.event_kinds.is_empty()
            && !selector
                .event_kinds
                .iter()
                .all(|value| descriptor.event_kinds.contains(value))
        {
            return Err(PolicyDecision::Deny {
                reason: "selector event kinds are not supported by descriptor",
            });
        }
        Ok(())
    }

    pub(crate) fn merged_constraints(
        &self,
        descriptor: &CapabilityDescriptor,
        request: &CapabilityRequest,
    ) -> Vec<CapabilityConstraint> {
        let mut constraints = descriptor.default_constraints.clone();
        constraints.extend(request.requested_constraints.iter().cloned());
        dedupe_constraints(constraints)
    }

    pub(crate) fn evaluate_constraints(
        descriptor: &CapabilityDescriptor,
        operations: &[i32],
        access_class: CapabilityAccessClass,
        constraints: &[CapabilityConstraint],
        context: &PolicyContext,
    ) -> Result<(), PolicyDecision> {
        if access_class == CapabilityAccessClass::Raw && !context.is_local {
            return Err(PolicyDecision::Deny {
                reason: "raw capability access is only allowed locally",
            });
        }

        if has_constraint_kind(constraints, CapabilityConstraintKind::RequireLocalOnly)
            && !context.is_local
        {
            return Err(PolicyDecision::Deny {
                reason: "capability requires a local requester",
            });
        }

        if has_constraint_kind(
            constraints,
            CapabilityConstraintKind::RequireHardwareProtected,
        ) && !context.hardware_protected
        {
            return Err(PolicyDecision::Deny {
                reason: "capability requires hardware-protected execution",
            });
        }

        if has_constraint_kind(constraints, CapabilityConstraintKind::RequireBiometric)
            && !context.biometric_present
        {
            return Err(PolicyDecision::RequireInteraction {
                reason: "capability requires biometric confirmation",
            });
        }

        if requires_user_presence(descriptor, operations, constraints) && !context.user_present {
            return Err(PolicyDecision::RequireInteraction {
                reason: "capability requires user presence",
            });
        }

        Ok(())
    }

    pub(crate) fn requested_expiry(
        &self,
        request: &CapabilityRequest,
        now: SystemTime,
    ) -> Option<SystemTime> {
        let duration = request
            .requested_duration
            .as_ref()
            .and_then(duration_from_prost)
            .unwrap_or(self.default_grant_duration)
            .min(self.max_grant_duration);
        Some(now + duration)
    }

    pub(crate) fn constraint_rate_limit(
        constraint: &CapabilityConstraint,
    ) -> Option<(u64, Duration)> {
        let rate_limit = constraint.rate_limit.as_ref()?;
        let per = duration_from_prost(rate_limit.per.as_ref()?)?;
        Some((rate_limit.max_operations, per))
    }

    pub(crate) fn store_grant(
        &mut self,
        grant: CapabilityGrant,
        issued_at: SystemTime,
        expires_at: Option<SystemTime>,
    ) {
        self.grants.insert(
            grant.grant_id.clone(),
            GrantRecord {
                grant,
                issued_at,
                expires_at,
                revoked: None,
                invocation_count: 0,
                invocation_timestamps: VecDeque::new(),
            },
        );
    }

    pub fn import_grant(&mut self, grant: CapabilityGrant) -> Result<(), CapabilityError> {
        validate_grant(&grant)?;
        let issued_at = grant
            .issued_at
            .as_ref()
            .and_then(system_time_from_timestamp)
            .unwrap_or_else(SystemTime::now);
        let expires_at = grant
            .expires_at
            .as_ref()
            .and_then(system_time_from_timestamp);
        self.store_grant(grant, issued_at, expires_at);
        Ok(())
    }

    pub fn apply_revocation(
        &mut self,
        revocation: CapabilityRevocation,
    ) -> Result<(), CapabilityError> {
        let record =
            self.grants
                .get_mut(&revocation.grant_id)
                .ok_or(CapabilityError::InvalidRequest(
                    "cannot revoke an unknown capability grant",
                ))?;
        record.revoked = Some(revocation);
        Ok(())
    }
}
