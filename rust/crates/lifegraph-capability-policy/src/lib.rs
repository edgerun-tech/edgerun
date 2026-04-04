use std::collections::{BTreeSet, HashMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use blake3::Hasher;
use lifegraph_capabilities::{
    validate_descriptor, validate_grant, CapabilityAccessClass, CapabilityConstraint,
    CapabilityConstraintKind, CapabilityDescriptor, CapabilityError, CapabilityGrant,
    CapabilityInvocation, CapabilityModality, CapabilityOperation, CapabilityRequest,
    CapabilityRevocation, CapabilityRole, CapabilitySelector,
};
use lifegraph_proto::lifegraph::v0::common::{IdentityRef, NodeRef};
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
        let mut hasher = Hasher::new();
        hasher.update(&request.request_id);
        hasher.update(descriptor.provider_name.as_bytes());
        hasher.update(descriptor.provider_instance_id.as_bytes());
        hasher.update(&self.nonce.to_le_bytes());
        let now = now.duration_since(UNIX_EPOCH).unwrap_or_default();
        hasher.update(&now.as_secs().to_le_bytes());
        hasher.update(&now.subsec_nanos().to_le_bytes());
        hasher.finalize().as_bytes().to_vec()
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

    let modalities = descriptor
        .modalities
        .iter()
        .filter_map(|value| CapabilityModality::try_from(*value).ok())
        .collect::<Vec<_>>();
    let operations = operations
        .iter()
        .filter_map(|value| CapabilityOperation::try_from(*value).ok())
        .collect::<Vec<_>>();
    let role = CapabilityRole::try_from(descriptor.role).unwrap_or(CapabilityRole::Unspecified);

    let sensitive_modality = modalities.iter().any(|modality| {
        matches!(
            modality,
            CapabilityModality::Auditory
                | CapabilityModality::Visual
                | CapabilityModality::Biometric
                | CapabilityModality::Touch
        )
    });
    let sensitive_operation = operations.iter().any(|operation| {
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
    let mut hasher = Hasher::new();
    hasher.update(grant_id);
    hasher.update(reason.as_str().as_bytes());
    hasher.finalize().as_bytes().to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use lifegraph_capabilities::{
        capability_descriptor, constraint, constraint_with_scope, CapabilityEventKind,
    };
    use lifegraph_proto::lifegraph::v0::common::RateLimit;

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
    fn authorizes_and_tracks_one_shot_grants() {
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

        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv-1".to_vec(),
            grant_id: grant.grant_id.clone(),
            invoker: None,
            operation: CapabilityOperation::Query as i32,
            requested_access_class: CapabilityAccessClass::Derived as i32,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
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
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv-1".to_vec(),
            grant_id: grant.grant_id.clone(),
            invoker: None,
            operation: CapabilityOperation::Query as i32,
            requested_access_class: CapabilityAccessClass::Derived as i32,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
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
            PolicyDecision::Allow { grant } => grant,
            other => panic!("unexpected decision: {other:?}"),
        };
        let revocation = engine
            .revoke(&grant.grant_id, RevocationReason::UserRevoked)
            .unwrap();
        assert_eq!(revocation.reason, "user_revoked");
        assert!(engine.grant_record(&grant.grant_id).unwrap().is_revoked());

        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"inv-1".to_vec(),
            grant_id: grant.grant_id.clone(),
            invoker: None,
            operation: CapabilityOperation::Query as i32,
            requested_access_class: CapabilityAccessClass::Derived as i32,
            parameter_object: None,
            correlation_id: Vec::new(),
            invoked_at: None,
            signature: None,
        };
        let err = engine
            .authorize_invocation(&invocation, &context)
            .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::PermissionDenied("capability grant has been revoked")
        );
    }

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
}
