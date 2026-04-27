use crate::collections::{BTreeSet, HashMap, VecDeque};
use crate::prelude::v1::*;
use crate::time::{Duration, SystemTime, UNIX_EPOCH};

use super::engine::SimplePolicyEngine;
use super::helpers::*;
use super::trait_def::PolicyEngine;
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

impl PolicyEngine for SimplePolicyEngine {
    fn evaluate_request(
        &mut self,
        descriptor: &CapabilityDescriptor,
        request: &CapabilityRequest,
        context: &PolicyContext,
    ) -> Result<PolicyDecision, CapabilityError> {
        validate_descriptor(descriptor)?;
        let selector = request
            .selector
            .as_ref()
            .ok_or(CapabilityError::InvalidRequest(
                "capability request must include a selector",
            ))?;
        self.selector_matches(descriptor, selector)
            .map_err(|decision| match decision {
                PolicyDecision::Deny { reason } => CapabilityError::PermissionDenied(reason),
                PolicyDecision::RequireInteraction { reason } => {
                    CapabilityError::PermissionDenied(reason)
                }
                PolicyDecision::Allow { .. } => {
                    CapabilityError::Provider("unexpected allow decision".into())
                }
            })?;

        let requested_operations = self.effective_requested_operations(descriptor, request);
        let granted_operations: Vec<i32> = requested_operations
            .into_iter()
            .filter(|operation| descriptor.operations.contains(operation))
            .collect();
        let granted_operations = dedupe_i32(granted_operations);
        if granted_operations.is_empty() {
            return Ok(PolicyDecision::Deny {
                reason: "request did not ask for any operations supported by the descriptor",
            });
        }

        let constraints = self.merged_constraints(descriptor, request);
        let access_class = self.effective_access_class(request);
        if let Err(decision) = Self::evaluate_constraints(
            descriptor,
            &granted_operations,
            access_class,
            &constraints,
            context,
        ) {
            return Ok(decision);
        }

        let issued_at = context.now;
        let expires_at = self.requested_expiry(request, issued_at);
        let grant = CapabilityGrant {
            grant_version: 1,
            grant_id: self.next_grant_id(descriptor, request, issued_at),
            issuer: self.issuer.clone(),
            grantee: request
                .requester
                .clone()
                .or_else(|| context.requester.clone()),
            grantee_node: request
                .requester_node
                .clone()
                .or_else(|| context.requester_node.clone()),
            selector: Some(selector.clone()),
            granted_operations,
            enforced_constraints: constraints,
            access_class: access_class as i32,
            issued_at: Some(timestamp_from_system_time(issued_at)),
            expires_at: expires_at.map(timestamp_from_system_time),
            correlation_id: request.correlation_id.clone(),
            supersedes_revocation: None,
            signature: None,
        };
        validate_grant(&grant)?;
        self.store_grant(grant.clone(), issued_at, expires_at);
        Ok(PolicyDecision::Allow {
            grant: Box::new(grant),
        })
    }

    fn authorize_invocation(
        &mut self,
        invocation: &CapabilityInvocation,
        context: &PolicyContext,
    ) -> Result<(), CapabilityError> {
        let record =
            self.grants
                .get_mut(&invocation.grant_id)
                .ok_or(CapabilityError::PermissionDenied(
                    "capability invocation references an unknown grant",
                ))?;

        if record.revoked.is_some() {
            return Err(CapabilityError::PermissionDenied(
                "capability grant has been revoked",
            ));
        }

        if let Some(expires_at) = record.expires_at {
            if context.now >= expires_at {
                return Err(CapabilityError::PermissionDenied(
                    "capability grant has expired",
                ));
            }
        }

        let granted_access = CapabilityAccessClass::try_from(record.grant.access_class)
            .unwrap_or(CapabilityAccessClass::Derived);
        let requested_access = CapabilityAccessClass::try_from(invocation.requested_access_class)
            .unwrap_or(CapabilityAccessClass::Derived);
        if granted_access == CapabilityAccessClass::Derived
            && requested_access == CapabilityAccessClass::Raw
        {
            return Err(CapabilityError::PermissionDenied(
                "capability grant does not allow raw access",
            ));
        }

        if !record
            .grant
            .granted_operations
            .contains(&invocation.operation)
        {
            return Err(CapabilityError::PermissionDenied(
                "capability operation is not granted",
            ));
        }

        let selector = record
            .grant
            .selector
            .as_ref()
            .ok_or(CapabilityError::InvalidRequest(
                "stored capability grant is missing a selector",
            ))?;
        let granted_operations = record.grant.granted_operations.clone();
        let enforced_constraints = record.grant.enforced_constraints.clone();
        let descriptor = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: selector.capability_id.clone(),
            provider_identity: selector.provider_identity.clone(),
            provider_node: selector.provider_node.clone(),
            role: selector.role,
            modalities: selector.modalities.clone(),
            event_kinds: selector.event_kinds.clone(),
            operations: granted_operations.clone(),
            default_constraints: enforced_constraints.clone(),
            provider_name: String::new(),
            provider_instance_id: selector.provider_instance_id.clone(),
            signature: None,
        };

        Self::evaluate_constraints(
            &descriptor,
            &granted_operations,
            granted_access,
            &enforced_constraints,
            context,
        )
        .map_err(|decision| match decision {
            PolicyDecision::Deny { reason } | PolicyDecision::RequireInteraction { reason } => {
                CapabilityError::PermissionDenied(reason)
            }
            PolicyDecision::Allow { .. } => {
                CapabilityError::Provider("unexpected allow decision".into())
            }
        })?;

        if has_constraint_kind(
            &record.grant.enforced_constraints,
            CapabilityConstraintKind::OneShot,
        ) && record.invocation_count > 0
        {
            return Err(CapabilityError::PermissionDenied(
                "capability grant is one-shot and has already been used",
            ));
        }

        for constraint in &record.grant.enforced_constraints {
            if CapabilityConstraintKind::try_from(constraint.kind).ok()
                == Some(CapabilityConstraintKind::RateLimited)
            {
                let Some((max_operations, per)) = Self::constraint_rate_limit(constraint) else {
                    continue;
                };
                while let Some(front) = record.invocation_timestamps.front().copied() {
                    if context.now.duration_since(front).unwrap_or_default() >= per {
                        record.invocation_timestamps.pop_front();
                    } else {
                        break;
                    }
                }
                if record.invocation_timestamps.len() as u64 >= max_operations {
                    return Err(CapabilityError::PermissionDenied(
                        "capability grant is rate limited",
                    ));
                }
            }
        }

        record.invocation_count = record.invocation_count.saturating_add(1);
        record.invocation_timestamps.push_back(context.now);
        Ok(())
    }

    fn revoke(
        &mut self,
        grant_id: &[u8],
        reason: RevocationReason,
    ) -> Result<CapabilityRevocation, CapabilityError> {
        let record = self
            .grants
            .get_mut(grant_id)
            .ok_or(CapabilityError::InvalidRequest(
                "cannot revoke an unknown capability grant",
            ))?;
        let revocation = CapabilityRevocation {
            revocation_version: 1,
            revocation_id: revocation_id_for(grant_id, &reason),
            grant_id: grant_id.to_vec(),
            issuer: self.issuer.clone(),
            effective_at: Some(timestamp_from_system_time(SystemTime::now())),
            reason: reason.as_str().to_owned(),
            replacement_constraints: Vec::new(),
            signature: None,
        };
        record.revoked = Some(revocation.clone());
        Ok(revocation)
    }

    fn grant_record(&self, grant_id: &[u8]) -> Option<&GrantRecord> {
        self.grants.get(grant_id)
    }
}
