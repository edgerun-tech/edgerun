use std::collections::{BTreeSet, HashMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use edgerun_capabilities::{
    validate_descriptor, validate_grant, CapabilityAccessClass, CapabilityConstraint,
    CapabilityConstraintKind, CapabilityDescriptor, CapabilityError, CapabilityGrant,
    CapabilityInvocation, CapabilityModality, CapabilityOperation, CapabilityRequest,
    CapabilityRevocation, CapabilityRole, CapabilitySelector,
};
use edgerun_proto::edgerun::v0::common::{IdentityRef, NodeRef};
use prost_types::{Duration as ProstDuration, Timestamp};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyContext {
    pub requester: Option<IdentityRef>,
    pub requester_node: Option<NodeRef>,
    pub is_local: bool,
    pub user_present: bool,
    pub biometric_present: bool,
    pub hardware_protected: bool,
    pub now: SystemTime,
}

impl Default for PolicyContext {
    fn default() -> Self {
        Self {
            requester: None,
            requester_node: None,
            is_local: true,
            user_present: false,
            biometric_present: false,
            hardware_protected: false,
            now: SystemTime::now(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PolicyDecision {
    Allow { grant: Box<CapabilityGrant> },
    Deny { reason: &'static str },
    RequireInteraction { reason: &'static str },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RevocationReason {
    PolicyChanged,
    UserRevoked,
    Expired,
    Superseded,
    Misuse,
    Other(String),
}

impl RevocationReason {
    pub fn as_str(&self) -> &str {
        match self {
            Self::PolicyChanged => "policy_changed",
            Self::UserRevoked => "user_revoked",
            Self::Expired => "expired",
            Self::Superseded => "superseded",
            Self::Misuse => "misuse",
            Self::Other(reason) => reason.as_str(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct GrantRecord {
    pub grant: CapabilityGrant,
    pub issued_at: SystemTime,
    pub expires_at: Option<SystemTime>,
    pub revoked: Option<CapabilityRevocation>,
    pub invocation_count: u64,
    invocation_timestamps: VecDeque<SystemTime>,
}

impl GrantRecord {
    pub fn is_revoked(&self) -> bool {
        self.revoked.is_some()
    }
}

pub trait PolicyEngine {
    fn evaluate_request(
        &mut self,
        descriptor: &CapabilityDescriptor,
        request: &CapabilityRequest,
        context: &PolicyContext,
    ) -> Result<PolicyDecision, CapabilityError>;

    fn authorize_invocation(
        &mut self,
        invocation: &CapabilityInvocation,
        context: &PolicyContext,
    ) -> Result<(), CapabilityError>;

    fn revoke(
        &mut self,
        grant_id: &[u8],
        reason: RevocationReason,
    ) -> Result<CapabilityRevocation, CapabilityError>;

    fn grant_record(&self, grant_id: &[u8]) -> Option<&GrantRecord>;
}

#[derive(Clone, Debug)]
pub struct SimplePolicyEngine {
    issuer: Option<IdentityRef>,
    default_grant_duration: Duration,
    max_grant_duration: Duration,
    grants: HashMap<Vec<u8>, GrantRecord>,
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

    fn next_grant_id(
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

    fn effective_requested_operations(
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

    fn effective_access_class(&self, request: &CapabilityRequest) -> CapabilityAccessClass {
        request
            .selector
            .as_ref()
            .and_then(|selector| CapabilityAccessClass::try_from(selector.access_class).ok())
            .filter(|value| *value != CapabilityAccessClass::Unspecified)
            .unwrap_or(CapabilityAccessClass::Derived)
    }

    fn selector_matches(
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

    fn merged_constraints(
        &self,
        descriptor: &CapabilityDescriptor,
        request: &CapabilityRequest,
    ) -> Vec<CapabilityConstraint> {
        let mut constraints = descriptor.default_constraints.clone();
        constraints.extend(request.requested_constraints.iter().cloned());
        dedupe_constraints(constraints)
    }

    fn evaluate_constraints(
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

    fn requested_expiry(&self, request: &CapabilityRequest, now: SystemTime) -> Option<SystemTime> {
        let duration = request
            .requested_duration
            .as_ref()
            .and_then(duration_from_prost)
            .unwrap_or(self.default_grant_duration)
            .min(self.max_grant_duration);
        Some(now + duration)
    }

    fn constraint_rate_limit(constraint: &CapabilityConstraint) -> Option<(u64, Duration)> {
        let rate_limit = constraint.rate_limit.as_ref()?;
        let per = duration_from_prost(rate_limit.per.as_ref()?)?;
        Some((rate_limit.max_operations, per))
    }

    fn store_grant(
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

fn dedupe_i32(values: Vec<i32>) -> Vec<i32> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for value in values {
        if seen.insert(value) {
            out.push(value);
        }
    }
    out
}

fn constraint_identity(
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

fn dedupe_constraints(constraints: Vec<CapabilityConstraint>) -> Vec<CapabilityConstraint> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    for constraint in constraints {
        if seen.insert(constraint_identity(&constraint)) {
            out.push(constraint);
        }
    }
    out
}

fn has_constraint_kind(
    constraints: &[CapabilityConstraint],
    kind: CapabilityConstraintKind,
) -> bool {
    constraints
        .iter()
        .any(|constraint| CapabilityConstraintKind::try_from(constraint.kind).ok() == Some(kind))
}

fn requires_user_presence(
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

fn timestamp_from_system_time(value: SystemTime) -> Timestamp {
    let duration = value.duration_since(UNIX_EPOCH).unwrap_or_default();
    Timestamp {
        seconds: duration.as_secs() as i64,
        nanos: duration.subsec_nanos() as i32,
    }
}

fn system_time_from_timestamp(value: &Timestamp) -> Option<SystemTime> {
    if value.seconds < 0 || value.nanos < 0 {
        return None;
    }
    Some(UNIX_EPOCH + Duration::new(value.seconds as u64, value.nanos as u32))
}

fn duration_from_prost(value: &ProstDuration) -> Option<Duration> {
    if value.seconds < 0 || value.nanos < 0 {
        return None;
    }
    Some(Duration::new(value.seconds as u64, value.nanos as u32))
}

fn revocation_id_for(grant_id: &[u8], reason: &RevocationReason) -> Vec<u8> {
    let mut h = edgerun_crypto::sha2::Sha256::new();
    h.update(grant_id);
    h.update(reason.as_str().as_bytes());
    h.finalize().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_capabilities::{
        capability_descriptor, constraint, constraint_with_scope, CapabilityEventKind,
    };
    use edgerun_proto::edgerun::v0::common::{IdentityRef, RateLimit};

    // -- Test fixture helpers --

    fn test_descriptor() -> CapabilityDescriptor {
        capability_descriptor(
            "test-provider",
            "provider-1",
            CapabilityRole::Input,
            &[CapabilityModality::Visual],
            &[CapabilityEventKind::Visual],
            &[CapabilityOperation::Query, CapabilityOperation::Capture],
            Vec::new(),
        )
    }

    fn selector_for(
        descriptor: &CapabilityDescriptor,
        access_class: CapabilityAccessClass,
    ) -> CapabilitySelector {
        CapabilitySelector {
            capability_id: descriptor.capability_id.clone(),
            role: descriptor.role,
            modalities: descriptor.modalities.clone(),
            event_kinds: descriptor.event_kinds.clone(),
            operations: descriptor.operations.clone(),
            access_class: access_class as i32,
            provider_identity: None,
            provider_node: None,
            provider_instance_id: descriptor.provider_instance_id.clone(),
        }
    }

    fn request_for(descriptor: &CapabilityDescriptor) -> CapabilityRequest {
        CapabilityRequest {
            request_version: 1,
            request_id: b"request-1".to_vec(),
            requester: Some(IdentityRef {
                identity_id: b"requester-1".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            requester_node: None,
            selector: Some(selector_for(descriptor, CapabilityAccessClass::Derived)),
            requested_operations: vec![CapabilityOperation::Query as i32],
            requested_constraints: Vec::new(),
            purpose: "test".into(),
            requested_duration: Some(ProstDuration {
                seconds: 60,
                nanos: 0,
            }),
            correlation_id: b"corr-1".to_vec(),
            signature: None,
        }
    }

    fn make_invocation(grant_id: &[u8], operation: i32) -> CapabilityInvocation {
        CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv-1".to_vec(),
            grant_id: grant_id.to_vec(),
            invoker: None,
            operation,
            requested_access_class: CapabilityAccessClass::Derived as i32,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        }
    }

    // -- PolicyContext --

    #[test]
    fn policy_context_default() {
        let ctx = PolicyContext::default();
        assert!(ctx.requester.is_none());
        assert!(ctx.requester_node.is_none());
        assert!(ctx.is_local);
        assert!(!ctx.user_present);
        assert!(!ctx.biometric_present);
        assert!(!ctx.hardware_protected);
    }

    #[test]
    fn policy_context_clone_and_debug() {
        let ctx = PolicyContext {
            user_present: true,
            biometric_present: true,
            hardware_protected: true,
            is_local: false,
            ..PolicyContext::default()
        };
        let cloned = ctx.clone();
        assert_eq!(ctx, cloned);
        let debug = format!("{ctx:?}");
        assert!(debug.contains("PolicyContext"));
    }

    #[test]
    fn policy_context_equality() {
        let now = SystemTime::now();
        let a = PolicyContext { now, ..PolicyContext::default() };
        let b = PolicyContext { now, ..PolicyContext::default() };
        assert_eq!(a, b);
        let c = PolicyContext {
            user_present: true,
            now,
            ..PolicyContext::default()
        };
        assert_ne!(a, c);
    }

    // -- PolicyDecision --

    #[test]
    fn policy_decision_allow_clone() {
        let grant = CapabilityGrant {
            grant_version: 1,
            grant_id: vec![1, 2, 3],
            issuer: None,
            grantee: Some(IdentityRef {
                identity_id: b"grantee".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            grantee_node: None,
            selector: None,
            granted_operations: vec![0],
            enforced_constraints: Vec::new(),
            access_class: 0,
            issued_at: None,
            expires_at: None,
            correlation_id: Vec::new(),
            supersedes_revocation: None,
            signature: None,
        };
        let d = PolicyDecision::Allow { grant: Box::new(grant) };
        let cloned = d.clone();
        assert_eq!(d, cloned);
    }

    #[test]
    fn policy_decision_deny_clone_equality() {
        let a = PolicyDecision::Deny { reason: "no" };
        let b = PolicyDecision::Deny { reason: "no" };
        let c = PolicyDecision::Deny { reason: "yes" };
        assert_eq!(a, b);
        assert_eq!(a.clone(), b);
        assert_ne!(a, c);
    }

    #[test]
    fn policy_decision_require_interaction_clone() {
        let a = PolicyDecision::RequireInteraction { reason: "confirm" };
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn policy_decision_deny_vs_require_interaction() {
        let deny = PolicyDecision::Deny { reason: "no" };
        let require = PolicyDecision::RequireInteraction { reason: "no" };
        assert_ne!(deny, require);
    }

    // -- RevocationReason --

    #[test]
    fn revocation_reason_as_str_all_variants() {
        assert_eq!(RevocationReason::PolicyChanged.as_str(), "policy_changed");
        assert_eq!(RevocationReason::UserRevoked.as_str(), "user_revoked");
        assert_eq!(RevocationReason::Expired.as_str(), "expired");
        assert_eq!(RevocationReason::Superseded.as_str(), "superseded");
        assert_eq!(RevocationReason::Misuse.as_str(), "misuse");
        assert_eq!(
            RevocationReason::Other("custom".to_string()).as_str(),
            "custom"
        );
    }

    #[test]
    fn revocation_reason_clone_debug_equality() {
        let a = RevocationReason::UserRevoked;
        let b = a.clone();
        assert_eq!(a, b);
        let debug = format!("{a:?}");
        assert!(debug.contains("UserRevoked"));
    }

    #[test]
    fn revocation_reason_other_equality() {
        let a = RevocationReason::Other("x".to_string());
        let b = RevocationReason::Other("x".to_string());
        let c = RevocationReason::Other("y".to_string());
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, RevocationReason::Expired);
    }

    // -- GrantRecord --

    fn minimal_grant_record() -> GrantRecord {
        GrantRecord {
            grant: CapabilityGrant {
                grant_version: 1,
                grant_id: vec![1],
                issuer: None,
                grantee: Some(IdentityRef {
                    identity_id: vec![1],
                    identity_kind: None,
                    key_hint: None,
                }),
                grantee_node: None,
                selector: None,
                granted_operations: vec![0],
                enforced_constraints: Vec::new(),
                access_class: 0,
                issued_at: None,
                expires_at: None,
                correlation_id: Vec::new(),
                supersedes_revocation: None,
                signature: None,
            },
            issued_at: UNIX_EPOCH,
            expires_at: None,
            revoked: None,
            invocation_count: 0,
            invocation_timestamps: VecDeque::new(),
        }
    }

    #[test]
    fn grant_record_is_revoked_false_initially() {
        let record = minimal_grant_record();
        assert!(!record.is_revoked());
    }

    #[test]
    fn grant_record_is_revoked_true_after_revocation() {
        let mut record = minimal_grant_record();
        record.revoked = Some(CapabilityRevocation {
            revocation_version: 1,
            revocation_id: vec![2],
            grant_id: vec![1],
            issuer: None,
            effective_at: None,
            reason: "test".into(),
            replacement_constraints: Vec::new(),
            signature: None,
        });
        assert!(record.is_revoked());
    }

    #[test]
    fn grant_record_debug_and_clone() {
        let record = minimal_grant_record();
        let debug = format!("{record:?}");
        assert!(debug.contains("GrantRecord"));
        let cloned = record.clone();
        assert_eq!(record.grant.grant_id, cloned.grant.grant_id);
    }

    // -- SimplePolicyEngine construction --

    #[test]
    fn engine_default_new_are_equivalent() {
        let a = SimplePolicyEngine::default();
        let b = SimplePolicyEngine::new(None);
        assert_eq!(a.grants().count(), b.grants().count());
    }

    #[test]
    fn engine_with_issuer() {
        let issuer = IdentityRef {
            identity_id: b"issuer-1".to_vec(),
            identity_kind: None,
            key_hint: None,
        };
        let engine = SimplePolicyEngine::new(Some(issuer.clone()));
        assert_eq!(engine.grants().count(), 0);
    }

    #[test]
    fn engine_with_custom_durations() {
        let engine = SimplePolicyEngine::new(None)
            .with_default_grant_duration(Duration::from_secs(120))
            .with_max_grant_duration(Duration::from_secs(600));
        assert_eq!(engine.grants().count(), 0);
    }

    #[test]
    fn engine_grants_iterator_empty() {
        let engine = SimplePolicyEngine::default();
        assert_eq!(engine.grants().count(), 0);
    }

    #[test]
    fn engine_grants_iterator_after_grant() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap();
        assert_eq!(engine.grants().count(), 1);
    }

    // -- Grant lifecycle: grant, authorize, revoke --

    #[test]
    fn allows_basic_compatible_request() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let decision = engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap();
        let PolicyDecision::Allow { grant } = decision else {
            panic!("expected allow");
        };
        assert_eq!(
            grant.granted_operations,
            vec![CapabilityOperation::Query as i32]
        );
        assert_eq!(grant.access_class, CapabilityAccessClass::Derived as i32);
        assert!(engine.grant_record(&grant.grant_id).is_some());
    }

    #[test]
    fn grant_record_returns_none_for_unknown_id() {
        let engine = SimplePolicyEngine::default();
        assert!(engine.grant_record(b"nonexistent").is_none());
    }

    #[test]
    fn grant_has_issued_and_expiry_timestamps() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let grant = match engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        assert!(grant.issued_at.is_some());
        assert!(grant.expires_at.is_some());
    }

    #[test]
    fn grant_has_correlation_id_from_request() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let grant = match engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        assert_eq!(grant.correlation_id, request.correlation_id);
    }

    #[test]
    fn grant_has_grantee_from_request() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let grant = match engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        assert!(grant.grantee.is_some());
        assert_eq!(
            grant.grantee.as_ref().unwrap().identity_id,
            request.requester.as_ref().unwrap().identity_id
        );
    }

    #[test]
    fn authorization_succeeds_for_valid_grant() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let grant = match engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        let invocation = make_invocation(&grant.grant_id, CapabilityOperation::Query as i32);
        engine
            .authorize_invocation(&invocation, &PolicyContext::default())
            .unwrap();
    }

    #[test]
    fn authorization_tracks_invocation_count() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let grant = match engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        let invocation = make_invocation(&grant.grant_id, CapabilityOperation::Query as i32);
        let ctx = PolicyContext::default();
        engine.authorize_invocation(&invocation, &ctx).unwrap();
        let record = engine.grant_record(&grant.grant_id).unwrap();
        assert_eq!(record.invocation_count, 1);
    }

    #[test]
    fn multiple_authorizations_increment_count() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let grant = match engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        let invocation = make_invocation(&grant.grant_id, CapabilityOperation::Query as i32);
        let ctx = PolicyContext::default();
        engine.authorize_invocation(&invocation, &ctx).unwrap();
        engine.authorize_invocation(&invocation, &ctx).unwrap();
        let record = engine.grant_record(&grant.grant_id).unwrap();
        assert_eq!(record.invocation_count, 2);
    }

    #[test]
    fn revoke_returns_revocation_with_correct_reason() {
        for (reason, expected) in [
            (RevocationReason::PolicyChanged, "policy_changed"),
            (RevocationReason::UserRevoked, "user_revoked"),
            (RevocationReason::Expired, "expired"),
            (RevocationReason::Superseded, "superseded"),
            (RevocationReason::Misuse, "misuse"),
        ] {
            let descriptor = test_descriptor();
            let request = request_for(&descriptor);
            let mut engine = SimplePolicyEngine::default();
            let grant = match engine
                .evaluate_request(&descriptor, &request, &PolicyContext::default())
                .unwrap()
            {
                PolicyDecision::Allow { grant } => *grant,
                other => panic!("unexpected: {other:?}"),
            };
            let rev = engine.revoke(&grant.grant_id, reason.clone()).unwrap();
            assert_eq!(rev.reason, expected);
            assert_eq!(rev.grant_id, grant.grant_id);
            assert!(rev.effective_at.is_some());
            assert_eq!(rev.revocation_version, 1);
        }
    }

    #[test]
    fn revoke_unknown_grant_returns_error() {
        let mut engine = SimplePolicyEngine::default();
        let err = engine
            .revoke(b"nonexistent", RevocationReason::UserRevoked)
            .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("cannot revoke an unknown capability grant")
        );
    }

    #[test]
    fn revoke_marks_grant_unusable() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            ..PolicyContext::default()
        };
        let grant = match engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        let revocation = engine
            .revoke(&grant.grant_id, RevocationReason::UserRevoked)
            .unwrap();
        assert_eq!(revocation.reason, "user_revoked");
        assert!(engine.grant_record(&grant.grant_id).unwrap().is_revoked());

        let invocation = make_invocation(&grant.grant_id, CapabilityOperation::Query as i32);
        let err = engine.authorize_invocation(&invocation, &context).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("capability grant has been revoked")
        );
    }

    // -- Constraint evaluation: user presence --

    #[test]
    fn requires_user_presence_for_sensitive_capture() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request.requested_operations = vec![CapabilityOperation::Capture as i32];
        let mut engine = SimplePolicyEngine::default();
        let decision = engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap();
        assert_eq!(
            decision,
            PolicyDecision::RequireInteraction {
                reason: "capability requires user presence"
            }
        );
    }

    #[test]
    fn user_presence_satisfied_allows_capture() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request.requested_operations = vec![CapabilityOperation::Capture as i32];
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            ..PolicyContext::default()
        };
        let decision = engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap();
        assert!(matches!(decision, PolicyDecision::Allow { .. }));
    }

    #[test]
    fn require_user_presence_constraint_explicit() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request
            .requested_constraints
            .push(constraint(CapabilityConstraintKind::RequireUserPresence));
        let mut engine = SimplePolicyEngine::default();
        let decision = engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap();
        assert_eq!(
            decision,
            PolicyDecision::RequireInteraction {
                reason: "capability requires user presence"
            }
        );
    }

    #[test]
    fn require_user_presence_constraint_satisfied() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request
            .requested_constraints
            .push(constraint(CapabilityConstraintKind::RequireUserPresence));
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            ..PolicyContext::default()
        };
        let decision = engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap();
        assert!(matches!(decision, PolicyDecision::Allow { .. }));
    }

    // -- Constraint evaluation: biometric --

    #[test]
    fn require_biometric_denied_without_biometric() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request
            .requested_constraints
            .push(constraint(CapabilityConstraintKind::RequireBiometric));
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            ..PolicyContext::default()
        };
        let decision = engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap();
        assert_eq!(
            decision,
            PolicyDecision::RequireInteraction {
                reason: "capability requires biometric confirmation"
            }
        );
    }

    #[test]
    fn require_biometric_satisfied_with_biometric() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request
            .requested_constraints
            .push(constraint(CapabilityConstraintKind::RequireBiometric));
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            biometric_present: true,
            ..PolicyContext::default()
        };
        let decision = engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap();
        assert!(matches!(decision, PolicyDecision::Allow { .. }));
    }

    // -- Constraint evaluation: hardware-protected --

    #[test]
    fn require_hardware_protected_denied_without_hardware() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request
            .requested_constraints
            .push(constraint(CapabilityConstraintKind::RequireHardwareProtected));
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            ..PolicyContext::default()
        };
        let decision = engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap();
        assert_eq!(
            decision,
            PolicyDecision::Deny {
                reason: "capability requires hardware-protected execution"
            }
        );
    }

    #[test]
    fn require_hardware_protected_satisfied_with_hardware() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request
            .requested_constraints
            .push(constraint(CapabilityConstraintKind::RequireHardwareProtected));
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            hardware_protected: true,
            ..PolicyContext::default()
        };
        let decision = engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap();
        assert!(matches!(decision, PolicyDecision::Allow { .. }));
    }

    // -- Constraint evaluation: local-only --

    #[test]
    fn require_local_only_denied_for_remote() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request
            .requested_constraints
            .push(constraint(CapabilityConstraintKind::RequireLocalOnly));
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            is_local: false,
            user_present: true,
            ..PolicyContext::default()
        };
        let decision = engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap();
        assert_eq!(
            decision,
            PolicyDecision::Deny {
                reason: "capability requires a local requester"
            }
        );
    }

    #[test]
    fn require_local_only_satisfied_for_local() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request
            .requested_constraints
            .push(constraint(CapabilityConstraintKind::RequireLocalOnly));
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            is_local: true,
            user_present: true,
            ..PolicyContext::default()
        };
        let decision = engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap();
        assert!(matches!(decision, PolicyDecision::Allow { .. }));
    }

    // -- Constraint evaluation: raw access --

    #[test]
    fn denies_remote_raw_access() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request.selector = Some(selector_for(&descriptor, CapabilityAccessClass::Raw));
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            is_local: false,
            ..PolicyContext::default()
        };
        let decision = engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap();
        assert_eq!(
            decision,
            PolicyDecision::Deny {
                reason: "raw capability access is only allowed locally"
            }
        );
    }

    #[test]
    fn allows_local_raw_access() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request.selector = Some(selector_for(&descriptor, CapabilityAccessClass::Raw));
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            is_local: true,
            ..PolicyContext::default()
        };
        let decision = engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap();
        assert!(matches!(decision, PolicyDecision::Allow { .. }));
    }

    // -- One-shot constraint --

    #[test]
    fn one_shot_from_descriptor_allows_single_use() {
        let mut descriptor = test_descriptor();
        descriptor
            .default_constraints
            .push(constraint(CapabilityConstraintKind::OneShot));
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            ..PolicyContext::default()
        };
        let grant = match engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap()
        {
            PolicyDecision::Allow { grant } => grant,
            other => panic!("unexpected decision: {other:?}"),
        };

        let invocation = make_invocation(&grant.grant_id, CapabilityOperation::Query as i32);
        engine.authorize_invocation(&invocation, &context).unwrap();
        let err = engine
            .authorize_invocation(&invocation, &context)
            .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied(
                "capability grant is one-shot and has already been used"
            )
        );
    }

    #[test]
    fn one_shot_from_request_allows_single_use() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request
            .requested_constraints
            .push(constraint(CapabilityConstraintKind::OneShot));
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            ..PolicyContext::default()
        };
        let grant = match engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        let invocation = make_invocation(&grant.grant_id, CapabilityOperation::Query as i32);
        engine.authorize_invocation(&invocation, &context).unwrap();
        let err = engine.authorize_invocation(&invocation, &context).unwrap_err();
        assert!(matches!(
            err,
            CapabilityError::PermissionDenied("capability grant is one-shot and has already been used")
        ));
    }

    // -- Rate-limited constraint --

    #[test]
    fn enforces_rate_limited_constraint() {
        let mut descriptor = test_descriptor();
        descriptor.default_constraints.push(CapabilityConstraint {
            kind: CapabilityConstraintKind::RateLimited as i32,
            uint_value: None,
            string_value: String::new(),
            duration_value: None,
            rate_limit: Some(RateLimit {
                max_operations: 1,
                per: Some(ProstDuration {
                    seconds: 60,
                    nanos: 0,
                }),
            }),
        });
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            ..PolicyContext::default()
        };
        let grant = match engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap()
        {
            PolicyDecision::Allow { grant } => grant,
            other => panic!("unexpected decision: {other:?}"),
        };
        let invocation = make_invocation(&grant.grant_id, CapabilityOperation::Query as i32);
        engine.authorize_invocation(&invocation, &context).unwrap();
        let err = engine
            .authorize_invocation(&invocation, &context)
            .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("capability grant is rate limited")
        );
    }

    #[test]
    fn rate_limit_resets_after_window() {
        let mut descriptor = test_descriptor();
        descriptor.default_constraints.push(CapabilityConstraint {
            kind: CapabilityConstraintKind::RateLimited as i32,
            uint_value: None,
            string_value: String::new(),
            duration_value: None,
            rate_limit: Some(RateLimit {
                max_operations: 1,
                per: Some(ProstDuration {
                    seconds: 10,
                    nanos: 0,
                }),
            }),
        });
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let now = SystemTime::now();
        let context = PolicyContext {
            user_present: true,
            now,
            ..PolicyContext::default()
        };
        let grant = match engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        let invocation = make_invocation(&grant.grant_id, CapabilityOperation::Query as i32);
        engine.authorize_invocation(&invocation, &context).unwrap();

        let later = now + Duration::from_secs(11);
        let later_ctx = PolicyContext {
            user_present: true,
            now: later,
            ..PolicyContext::default()
        };
        engine.authorize_invocation(&invocation, &later_ctx).unwrap();
        let record = engine.grant_record(&grant.grant_id).unwrap();
        assert_eq!(record.invocation_count, 2);
    }

    #[test]
    fn rate_limit_with_higher_max_allows_burst() {
        let mut descriptor = test_descriptor();
        descriptor.default_constraints.push(CapabilityConstraint {
            kind: CapabilityConstraintKind::RateLimited as i32,
            uint_value: None,
            string_value: String::new(),
            duration_value: None,
            rate_limit: Some(RateLimit {
                max_operations: 3,
                per: Some(ProstDuration {
                    seconds: 60,
                    nanos: 0,
                }),
            }),
        });
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            ..PolicyContext::default()
        };
        let grant = match engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        let invocation = make_invocation(&grant.grant_id, CapabilityOperation::Query as i32);
        engine.authorize_invocation(&invocation, &context).unwrap();
        engine.authorize_invocation(&invocation, &context).unwrap();
        engine.authorize_invocation(&invocation, &context).unwrap();
        let err = engine.authorize_invocation(&invocation, &context).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("capability grant is rate limited")
        );
    }

    // -- Authorization edge cases --

    #[test]
    fn authorize_invocation_unknown_grant() {
        let mut engine = SimplePolicyEngine::default();
        let invocation = make_invocation(b"nonexistent", CapabilityOperation::Query as i32);
        let err = engine
            .authorize_invocation(&invocation, &PolicyContext::default())
            .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("capability invocation references an unknown grant")
        );
    }

    #[test]
    fn authorize_invocation_expired_grant() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let grant = match engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        let expired_ctx = PolicyContext {
            now: SystemTime::now() + Duration::from_secs(99999),
            ..PolicyContext::default()
        };
        let invocation = make_invocation(&grant.grant_id, CapabilityOperation::Query as i32);
        let err = engine.authorize_invocation(&invocation, &expired_ctx).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("capability grant has expired")
        );
    }

    #[test]
    fn authorize_invocation_wrong_operation() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let grant = match engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        let invocation = make_invocation(&grant.grant_id, CapabilityOperation::Capture as i32);
        let err = engine
            .authorize_invocation(&invocation, &PolicyContext::default())
            .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("capability operation is not granted")
        );
    }

    #[test]
    fn authorize_invocation_raw_access_not_granted() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let grant = match engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        let mut invocation = make_invocation(&grant.grant_id, CapabilityOperation::Query as i32);
        invocation.requested_access_class = CapabilityAccessClass::Raw as i32;
        let err = engine
            .authorize_invocation(&invocation, &PolicyContext::default())
            .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("capability grant does not allow raw access")
        );
    }

    // -- Selector matching --

    #[test]
    fn selector_mismatch_provider_instance() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request.selector.as_mut().unwrap().provider_instance_id = "wrong-instance".to_string();
        let mut engine = SimplePolicyEngine::default();
        let err = engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("selector provider instance does not match descriptor")
        );
    }

    #[test]
    fn selector_mismatch_role() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request.selector.as_mut().unwrap().role = CapabilityRole::Output as i32;
        let mut engine = SimplePolicyEngine::default();
        let err = engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("selector role does not match descriptor")
        );
    }

    #[test]
    fn selector_mismatch_capability_id() {
        let mut descriptor = test_descriptor();
        descriptor.capability_id = b"cap-1".to_vec();
        let mut request = request_for(&descriptor);
        request.selector.as_mut().unwrap().capability_id = b"cap-2".to_vec();
        let mut engine = SimplePolicyEngine::default();
        let err = engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("selector capability id does not match descriptor")
        );
    }

    #[test]
    fn selector_mismatch_modalities() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request
            .selector
            .as_mut()
            .unwrap()
            .modalities
            .push(CapabilityModality::Auditory as i32);
        let mut engine = SimplePolicyEngine::default();
        let err = engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("selector modalities are not supported by descriptor")
        );
    }

    #[test]
    fn selector_mismatch_event_kinds() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request
            .selector
            .as_mut()
            .unwrap()
            .event_kinds
            .push(CapabilityEventKind::Auditory as i32);
        let mut engine = SimplePolicyEngine::default();
        let err = engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("selector event kinds are not supported by descriptor")
        );
    }

    // -- Request validation --

    #[test]
    fn request_without_selector_is_rejected() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request.selector = None;
        let mut engine = SimplePolicyEngine::default();
        let err = engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability request must include a selector")
        );
    }

    #[test]
    fn request_operations_not_in_descriptor_denied() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request.requested_operations = vec![999];
        let mut engine = SimplePolicyEngine::default();
        let decision = engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap();
        assert_eq!(
            decision,
            PolicyDecision::Deny {
                reason: "request did not ask for any operations supported by the descriptor"
            }
        );
    }

    #[test]
    fn request_operations_filtered_to_descriptor() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request.requested_operations = vec![
            CapabilityOperation::Query as i32,
            999,
            CapabilityOperation::Capture as i32,
        ];
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            ..PolicyContext::default()
        };
        let grant = match engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        assert!(grant
            .granted_operations
            .contains(&(CapabilityOperation::Query as i32)));
        assert!(!grant.granted_operations.contains(&999));
    }

    #[test]
    fn request_without_explicit_operations_uses_descriptor_ops() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request.requested_operations = Vec::new();
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            ..PolicyContext::default()
        };
        let grant = match engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        assert_eq!(
            grant.granted_operations,
            vec![
                CapabilityOperation::Query as i32,
                CapabilityOperation::Capture as i32,
            ]
        );
    }

    // -- Deduplication of constraints --

    #[test]
    fn dedupes_constraints_from_descriptor_and_request() {
        let mut descriptor = test_descriptor();
        descriptor
            .default_constraints
            .push(constraint_with_scope("inventory"));
        let mut request = request_for(&descriptor);
        request
            .requested_constraints
            .push(constraint_with_scope("inventory"));
        let mut engine = SimplePolicyEngine::default();
        let context = PolicyContext {
            user_present: true,
            ..PolicyContext::default()
        };
        let grant = match engine
            .evaluate_request(&descriptor, &request, &context)
            .unwrap()
        {
            PolicyDecision::Allow { grant } => grant,
            other => panic!("unexpected decision: {other:?}"),
        };
        assert_eq!(grant.enforced_constraints.len(), 1);
    }

    #[test]
    fn dedupes_operations_in_grant() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request.requested_operations = vec![
            CapabilityOperation::Query as i32,
            CapabilityOperation::Query as i32,
        ];
        let mut engine = SimplePolicyEngine::default();
        let grant = match engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        assert_eq!(
            grant.granted_operations,
            vec![CapabilityOperation::Query as i32]
        );
    }

    #[test]
    fn dedupe_i32_empty() {
        let result = dedupe_i32(Vec::new());
        assert!(result.is_empty());
    }

    #[test]
    fn dedupe_i32_all_same() {
        let result = dedupe_i32(vec![1, 1, 1]);
        assert_eq!(result, vec![1]);
    }

    #[test]
    fn dedupe_i32_preserves_order() {
        let result = dedupe_i32(vec![3, 1, 2, 1, 3]);
        assert_eq!(result, vec![3, 1, 2]);
    }

    #[test]
    fn dedupe_constraints_preserves_order() {
        let c1 = constraint(CapabilityConstraintKind::OneShot);
        let c2 = constraint_with_scope("scope-a");
        let c3 = constraint(CapabilityConstraintKind::RequireUserPresence);
        let constraints = vec![c1.clone(), c2.clone(), c1.clone(), c3.clone(), c2.clone()];
        let result = dedupe_constraints(constraints);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].kind, CapabilityConstraintKind::OneShot as i32);
        assert_eq!(result[1].string_value, "scope-a");
        assert_eq!(
            result[2].kind,
            CapabilityConstraintKind::RequireUserPresence as i32
        );
    }

    // -- External grant import and revocation --

    #[test]
    fn imports_external_grant_and_applies_revocation() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let grant = match engine
            .evaluate_request(
                &descriptor,
                &request,
                &PolicyContext {
                    user_present: true,
                    ..PolicyContext::default()
                },
            )
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected decision: {other:?}"),
        };

        let mut imported = SimplePolicyEngine::default();
        imported.import_grant(grant.clone()).unwrap();
        assert!(imported.grant_record(&grant.grant_id).is_some());

        let revocation = CapabilityRevocation {
            revocation_version: 1,
            revocation_id: b"rev-1".to_vec(),
            grant_id: grant.grant_id.clone(),
            issuer: None,
            effective_at: None,
            reason: "test".into(),
            replacement_constraints: Vec::new(),
            signature: None,
        };
        imported.apply_revocation(revocation).unwrap();
        assert!(imported.grant_record(&grant.grant_id).unwrap().is_revoked());
    }

    #[test]
    fn import_grant_validates_grant() {
        let mut engine = SimplePolicyEngine::default();
        let bad_grant = CapabilityGrant {
            grant_version: 1,
            grant_id: vec![1],
            issuer: None,
            grantee: None,
            grantee_node: None,
            selector: None,
            granted_operations: vec![0],
            enforced_constraints: Vec::new(),
            access_class: 0,
            issued_at: None,
            expires_at: None,
            correlation_id: Vec::new(),
            supersedes_revocation: None,
            signature: None,
        };
        let err = engine.import_grant(bad_grant).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability grant must name a grantee")
        );
    }

    #[test]
    fn apply_revocation_unknown_grant_errors() {
        let mut engine = SimplePolicyEngine::default();
        let revocation = CapabilityRevocation {
            revocation_version: 1,
            revocation_id: b"rev-1".to_vec(),
            grant_id: b"nonexistent".to_vec(),
            issuer: None,
            effective_at: None,
            reason: "test".into(),
            replacement_constraints: Vec::new(),
            signature: None,
        };
        let err = engine.apply_revocation(revocation).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("cannot revoke an unknown capability grant")
        );
    }

    #[test]
    fn import_grant_with_expiry() {
        let now = SystemTime::now();
        let issued = now - Duration::from_secs(60);
        let expires = now + Duration::from_secs(60);
        let grant = CapabilityGrant {
            grant_version: 1,
            grant_id: b"imported-grant".to_vec(),
            issuer: Some(IdentityRef {
                identity_id: b"issuer".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            grantee: Some(IdentityRef {
                identity_id: b"grantee".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            grantee_node: None,
            selector: Some(CapabilitySelector {
                capability_id: Vec::new(),
                role: CapabilityRole::Input as i32,
                modalities: vec![CapabilityModality::Visual as i32],
                event_kinds: vec![CapabilityEventKind::Visual as i32],
                operations: vec![CapabilityOperation::Query as i32],
                access_class: CapabilityAccessClass::Derived as i32,
                provider_identity: None,
                provider_node: None,
                provider_instance_id: "test-provider".to_string(),
            }),
            granted_operations: vec![CapabilityOperation::Query as i32],
            enforced_constraints: Vec::new(),
            access_class: CapabilityAccessClass::Derived as i32,
            issued_at: Some(timestamp_from_system_time(issued)),
            expires_at: Some(timestamp_from_system_time(expires)),
            correlation_id: Vec::new(),
            supersedes_revocation: None,
            signature: None,
        };
        let mut engine = SimplePolicyEngine::default();
        engine.import_grant(grant.clone()).unwrap();
        let record = engine.grant_record(&grant.grant_id).unwrap();
        assert_eq!(record.grant.grant_id, grant.grant_id);
        assert!(record.expires_at.is_some());
    }

    // -- Error types and display --

    #[test]
    fn capability_error_display_invalid_request() {
        let err = CapabilityError::InvalidRequest("missing field");
        assert_eq!(
            err.to_string(),
            "invalid capability request: missing field"
        );
    }

    #[test]
    fn capability_error_display_permission_denied() {
        let err = CapabilityError::PermissionDenied("not authorized");
        assert_eq!(
            err.to_string(),
            "capability permission denied: not authorized"
        );
    }

    #[test]
    fn capability_error_display_unsupported() {
        let err = CapabilityError::Unsupported("unknown op");
        assert_eq!(
            err.to_string(),
            "unsupported capability operation: unknown op"
        );
    }

    #[test]
    fn capability_error_display_provider() {
        let err = CapabilityError::Provider("provider error".to_string());
        assert_eq!(err.to_string(), "provider error");
    }

    #[test]
    fn capability_error_is_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(CapabilityError::InvalidRequest("test"));
        assert!(err.source().is_none());
    }

    #[test]
    fn capability_error_equality() {
        let a = CapabilityError::InvalidRequest("msg");
        let b = CapabilityError::InvalidRequest("msg");
        let c = CapabilityError::InvalidRequest("other");
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // -- Edge cases: empty policies, conflicting constraints --

    #[test]
    fn engine_starts_empty_and_handles_first_grant() {
        let engine = SimplePolicyEngine::default();
        assert_eq!(engine.grants().count(), 0);
        assert!(engine.grant_record(b"any").is_none());
    }

    #[test]
    fn engine_handles_multiple_distinct_grants() {
        let mut engine = SimplePolicyEngine::default();
        for i in 0..5 {
            let descriptor = test_descriptor();
            let mut request = request_for(&descriptor);
            request.request_id = format!("request-{i}").into_bytes();
            let ctx = PolicyContext::default();
            let grant = match engine
                .evaluate_request(&descriptor, &request, &ctx)
                .unwrap()
            {
                PolicyDecision::Allow { grant } => *grant,
                other => panic!("unexpected: {other:?}"),
            };
            assert!(engine.grant_record(&grant.grant_id).is_some());
        }
        assert_eq!(engine.grants().count(), 5);
    }

    #[test]
    fn combined_constraints_all_must_pass() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request
            .requested_constraints
            .push(constraint(CapabilityConstraintKind::RequireHardwareProtected));
        request
            .requested_constraints
            .push(constraint(CapabilityConstraintKind::RequireBiometric));
        let mut engine = SimplePolicyEngine::default();

        // Neither satisfied -> Deny (hardware check comes before biometric).
        let ctx_none = PolicyContext {
            user_present: true,
            ..PolicyContext::default()
        };
        let decision = engine
            .evaluate_request(&descriptor, &request, &ctx_none)
            .unwrap();
        assert_eq!(
            decision,
            PolicyDecision::Deny {
                reason: "capability requires hardware-protected execution"
            }
        );

        // Hardware present but not biometric -> RequireInteraction.
        let ctx_hw = PolicyContext {
            user_present: true,
            hardware_protected: true,
            ..PolicyContext::default()
        };
        let mut engine2 = SimplePolicyEngine::default();
        let decision = engine2
            .evaluate_request(&descriptor, &request, &ctx_hw)
            .unwrap();
        assert_eq!(
            decision,
            PolicyDecision::RequireInteraction {
                reason: "capability requires biometric confirmation"
            }
        );

        // Both satisfied -> Allow.
        let ctx_both = PolicyContext {
            user_present: true,
            biometric_present: true,
            hardware_protected: true,
            ..PolicyContext::default()
        };
        let mut engine3 = SimplePolicyEngine::default();
        let decision = engine3
            .evaluate_request(&descriptor, &request, &ctx_both)
            .unwrap();
        assert!(matches!(decision, PolicyDecision::Allow { .. }));
    }

    #[test]
    fn expired_grant_from_import_is_unusable() {
        let now = SystemTime::now();
        let past = now - Duration::from_secs(120);
        let grant = CapabilityGrant {
            grant_version: 1,
            grant_id: b"expired-grant".to_vec(),
            issuer: None,
            grantee: Some(IdentityRef {
                identity_id: b"grantee".to_vec(),
                identity_kind: None,
                key_hint: None,
            }),
            grantee_node: None,
            selector: Some(CapabilitySelector {
                capability_id: Vec::new(),
                role: CapabilityRole::Input as i32,
                modalities: vec![CapabilityModality::Visual as i32],
                event_kinds: vec![CapabilityEventKind::Visual as i32],
                operations: vec![CapabilityOperation::Query as i32],
                access_class: CapabilityAccessClass::Derived as i32,
                provider_identity: None,
                provider_node: None,
                provider_instance_id: "provider".to_string(),
            }),
            granted_operations: vec![CapabilityOperation::Query as i32],
            enforced_constraints: Vec::new(),
            access_class: CapabilityAccessClass::Derived as i32,
            issued_at: Some(timestamp_from_system_time(past)),
            expires_at: Some(timestamp_from_system_time(past + Duration::from_secs(60))),
            correlation_id: Vec::new(),
            supersedes_revocation: None,
            signature: None,
        };
        let mut engine = SimplePolicyEngine::default();
        engine.import_grant(grant.clone()).unwrap();
        let ctx = PolicyContext {
            now,
            ..PolicyContext::default()
        };
        let invocation = make_invocation(&grant.grant_id, CapabilityOperation::Query as i32);
        let err = engine.authorize_invocation(&invocation, &ctx).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("capability grant has expired")
        );
    }

    // -- Helper function coverage --

    #[test]
    fn timestamp_from_system_time_roundtrip() {
        let now = SystemTime::now();
        let ts = timestamp_from_system_time(now);
        let recovered = system_time_from_timestamp(&ts).unwrap();
        let diff = if recovered >= now {
            recovered.duration_since(now).unwrap()
        } else {
            now.duration_since(recovered).unwrap()
        };
        assert!(diff < Duration::from_secs(1));
    }

    #[test]
    fn system_time_from_timestamp_negative_seconds() {
        let ts = Timestamp {
            seconds: -1,
            nanos: 0,
        };
        assert!(system_time_from_timestamp(&ts).is_none());
    }

    #[test]
    fn system_time_from_timestamp_negative_nanos() {
        let ts = Timestamp {
            seconds: 0,
            nanos: -1,
        };
        assert!(system_time_from_timestamp(&ts).is_none());
    }

    #[test]
    fn duration_from_prost_valid() {
        let d = ProstDuration {
            seconds: 120,
            nanos: 500_000_000,
        };
        let result = duration_from_prost(&d).unwrap();
        assert_eq!(result, Duration::new(120, 500_000_000));
    }

    #[test]
    fn duration_from_prost_negative_seconds() {
        let d = ProstDuration {
            seconds: -1,
            nanos: 0,
        };
        assert!(duration_from_prost(&d).is_none());
    }

    #[test]
    fn duration_from_prost_negative_nanos() {
        let d = ProstDuration {
            seconds: 0,
            nanos: -1,
        };
        assert!(duration_from_prost(&d).is_none());
    }

    #[test]
    fn revocation_id_for_deterministic() {
        let grant_id = b"test-grant";
        let reason = RevocationReason::UserRevoked;
        let id1 = revocation_id_for(grant_id, &reason);
        let id2 = revocation_id_for(grant_id, &reason);
        assert_eq!(id1, id2);
        assert_eq!(id1.len(), 32);
    }

    #[test]
    fn revocation_id_for_different_reasons() {
        let grant_id = b"test-grant";
        let id1 = revocation_id_for(grant_id, &RevocationReason::UserRevoked);
        let id2 = revocation_id_for(grant_id, &RevocationReason::Expired);
        assert_ne!(id1, id2);
    }

    #[test]
    fn revocation_id_for_different_grants() {
        let reason = RevocationReason::UserRevoked;
        let id1 = revocation_id_for(b"grant-1", &reason);
        let id2 = revocation_id_for(b"grant-2", &reason);
        assert_ne!(id1, id2);
    }

    // -- has_constraint_kind and requires_user_presence --

    #[test]
    fn has_constraint_kind_finds_matching() {
        let constraints = vec![
            constraint(CapabilityConstraintKind::Scope),
            constraint(CapabilityConstraintKind::RequireUserPresence),
        ];
        assert!(has_constraint_kind(
            &constraints,
            CapabilityConstraintKind::RequireUserPresence
        ));
        assert!(has_constraint_kind(&constraints, CapabilityConstraintKind::Scope));
        assert!(!has_constraint_kind(
            &constraints,
            CapabilityConstraintKind::RequireBiometric
        ));
    }

    #[test]
    fn has_constraint_kind_empty() {
        let constraints: Vec<CapabilityConstraint> = Vec::new();
        assert!(!has_constraint_kind(
            &constraints,
            CapabilityConstraintKind::OneShot
        ));
    }

    #[test]
    fn requires_user_presence_explicit_constraint() {
        let descriptor = test_descriptor();
        let constraints = vec![constraint(CapabilityConstraintKind::RequireUserPresence)];
        assert!(requires_user_presence(
            &descriptor,
            &[CapabilityOperation::Query as i32],
            &constraints
        ));
    }

    #[test]
    fn requires_user_presence_sensitive_modality_and_operation() {
        let descriptor = test_descriptor();
        assert!(requires_user_presence(
            &descriptor,
            &[CapabilityOperation::Capture as i32],
            &[]
        ));
    }

    #[test]
    fn requires_user_presence_non_sensitive_operation() {
        let descriptor = test_descriptor();
        assert!(!requires_user_presence(
            &descriptor,
            &[CapabilityOperation::Query as i32],
            &[]
        ));
    }

    #[test]
    fn requires_user_presence_secure_element_role() {
        let descriptor = capability_descriptor(
            "secure-element",
            "se-0",
            CapabilityRole::SecureElement,
            &[CapabilityModality::Cryptographic],
            &[CapabilityEventKind::Signing],
            &[CapabilityOperation::Sign],
            Vec::new(),
        );
        assert!(requires_user_presence(
            &descriptor,
            &[CapabilityOperation::Sign as i32],
            &[]
        ));
    }

    #[test]
    fn requires_user_presence_non_sensitive_modality() {
        let descriptor = capability_descriptor(
            "logger",
            "log-0",
            CapabilityRole::Storage,
            &[CapabilityModality::Other],
            &[CapabilityEventKind::State],
            &[CapabilityOperation::Capture],
            Vec::new(),
        );
        assert!(!requires_user_presence(
            &descriptor,
            &[CapabilityOperation::Capture as i32],
            &[]
        ));
    }

    // -- constraint_identity --

    #[test]
    fn constraint_identity_basic() {
        let c = constraint(CapabilityConstraintKind::OneShot);
        let id = constraint_identity(&c);
        assert_eq!(id.0, CapabilityConstraintKind::OneShot as i32);
        assert!(id.1.is_none());
        assert!(id.2.is_empty());
        assert!(id.3.is_none());
        assert!(id.4.is_none());
        assert!(id.5.is_none());
    }

    #[test]
    fn constraint_identity_with_scope() {
        let c = constraint_with_scope("my-scope");
        let id = constraint_identity(&c);
        assert_eq!(id.0, CapabilityConstraintKind::Scope as i32);
        assert_eq!(id.2, "my-scope");
    }

    // -- Nonce increments across grants --

    #[test]
    fn engine_nonce_increments_per_grant() {
        let mut engine = SimplePolicyEngine::default();
        let mut grant_ids = Vec::new();
        for i in 0..3 {
            let descriptor = test_descriptor();
            let mut request = request_for(&descriptor);
            request.request_id = format!("req-{i}").into_bytes();
            let grant = match engine
                .evaluate_request(&descriptor, &request, &PolicyContext::default())
                .unwrap()
            {
                PolicyDecision::Allow { grant } => *grant,
                other => panic!("unexpected: {other:?}"),
            };
            grant_ids.push(grant.grant_id);
        }
        for i in 0..grant_ids.len() {
            for j in (i + 1)..grant_ids.len() {
                assert_ne!(grant_ids[i], grant_ids[j], "grant {i} and {j} have same id");
            }
        }
    }

    // -- AccessClass default behavior --

    #[test]
    fn effective_access_class_derived_when_unspecified() {
        let descriptor = test_descriptor();
        let mut request = request_for(&descriptor);
        request.selector.as_mut().unwrap().access_class =
            CapabilityAccessClass::Unspecified as i32;
        let mut engine = SimplePolicyEngine::default();
        let grant = match engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        assert_eq!(grant.access_class, CapabilityAccessClass::Derived as i32);
    }

    // -- Invocation timestamp tracking --

    #[test]
    fn invocation_timestamps_are_tracked() {
        let descriptor = test_descriptor();
        let request = request_for(&descriptor);
        let mut engine = SimplePolicyEngine::default();
        let grant = match engine
            .evaluate_request(&descriptor, &request, &PolicyContext::default())
            .unwrap()
        {
            PolicyDecision::Allow { grant } => *grant,
            other => panic!("unexpected: {other:?}"),
        };
        let invocation = make_invocation(&grant.grant_id, CapabilityOperation::Query as i32);
        let ctx = PolicyContext::default();
        engine.authorize_invocation(&invocation, &ctx).unwrap();
        let record = engine.grant_record(&grant.grant_id).unwrap();
        assert_eq!(record.invocation_timestamps.len(), 1);
    }
}
