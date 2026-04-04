pub use lifegraph_proto::lifegraph::v0::capability::{
    CapabilityAccessClass, CapabilityConstraint, CapabilityConstraintKind, CapabilityDescriptor,
    CapabilityEventKind, CapabilityGrant, CapabilityInvocation, CapabilityModality,
    CapabilityOperation, CapabilityRequest, CapabilityResult, CapabilityRevocation, CapabilityRole,
    CapabilitySelector,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CapabilityError {
    InvalidRequest(&'static str),
    PermissionDenied(&'static str),
    Unsupported(&'static str),
    Provider(String),
}

impl core::fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidRequest(msg) => write!(f, "invalid capability request: {msg}"),
            Self::PermissionDenied(msg) => write!(f, "capability permission denied: {msg}"),
            Self::Unsupported(msg) => write!(f, "unsupported capability operation: {msg}"),
            Self::Provider(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for CapabilityError {}

pub trait CapabilityProvider {
    fn descriptor(&self) -> CapabilityDescriptor;
}

pub fn constraint(kind: CapabilityConstraintKind) -> CapabilityConstraint {
    CapabilityConstraint {
        kind: kind as i32,
        uint_value: None,
        string_value: String::new(),
        duration_value: None,
        rate_limit: None,
    }
}

pub fn constraint_with_uint(
    kind: CapabilityConstraintKind,
    uint_value: u64,
) -> CapabilityConstraint {
    CapabilityConstraint {
        kind: kind as i32,
        uint_value: Some(uint_value),
        string_value: String::new(),
        duration_value: None,
        rate_limit: None,
    }
}

pub fn constraint_with_scope(scope: impl Into<String>) -> CapabilityConstraint {
    CapabilityConstraint {
        kind: CapabilityConstraintKind::Scope as i32,
        uint_value: None,
        string_value: scope.into(),
        duration_value: None,
        rate_limit: None,
    }
}

pub fn capability_descriptor(
    provider_name: impl Into<String>,
    provider_instance_id: impl Into<String>,
    role: CapabilityRole,
    modalities: &[CapabilityModality],
    event_kinds: &[CapabilityEventKind],
    operations: &[CapabilityOperation],
    default_constraints: Vec<CapabilityConstraint>,
) -> CapabilityDescriptor {
    CapabilityDescriptor {
        descriptor_version: 1,
        capability_id: Vec::new(),
        provider_identity: None,
        provider_node: None,
        role: role as i32,
        modalities: modalities.iter().map(|v| *v as i32).collect(),
        event_kinds: event_kinds.iter().map(|v| *v as i32).collect(),
        operations: operations.iter().map(|v| *v as i32).collect(),
        default_constraints,
        provider_name: provider_name.into(),
        provider_instance_id: provider_instance_id.into(),
        signature: None,
    }
}

pub fn validate_descriptor(descriptor: &CapabilityDescriptor) -> Result<(), CapabilityError> {
    if descriptor.provider_name.trim().is_empty() {
        return Err(CapabilityError::InvalidRequest(
            "capability provider name must not be empty",
        ));
    }
    if descriptor.provider_instance_id.trim().is_empty() {
        return Err(CapabilityError::InvalidRequest(
            "capability provider instance id must not be empty",
        ));
    }
    if descriptor.modalities.is_empty() {
        return Err(CapabilityError::InvalidRequest(
            "capability must declare at least one modality",
        ));
    }
    if descriptor.event_kinds.is_empty() {
        return Err(CapabilityError::InvalidRequest(
            "capability must declare at least one event kind",
        ));
    }
    if descriptor.operations.is_empty() {
        return Err(CapabilityError::InvalidRequest(
            "capability must declare at least one operation",
        ));
    }
    Ok(())
}

pub fn validate_grant(grant: &CapabilityGrant) -> Result<(), CapabilityError> {
    if grant.grantee.is_none() {
        return Err(CapabilityError::InvalidRequest(
            "capability grant must name a grantee",
        ));
    }
    if grant.selector.is_none() {
        return Err(CapabilityError::InvalidRequest(
            "capability grant must include a selector",
        ));
    }
    if grant.granted_operations.is_empty() {
        return Err(CapabilityError::InvalidRequest(
            "capability grant must grant at least one operation",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_requires_provider_and_modalities() {
        let err = validate_descriptor(&CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: Vec::new(),
            provider_identity: None,
            provider_node: None,
            role: CapabilityRole::Input as i32,
            modalities: Vec::new(),
            event_kinds: Vec::new(),
            operations: Vec::new(),
            default_constraints: Vec::new(),
            provider_name: String::new(),
            provider_instance_id: String::new(),
            signature: None,
        })
        .unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability provider name must not be empty")
        );
    }

    #[test]
    fn helper_builds_proto_descriptor() {
        let descriptor = capability_descriptor(
            "alsa",
            "hw:0,0",
            CapabilityRole::Input,
            &[CapabilityModality::Auditory],
            &[CapabilityEventKind::Auditory],
            &[CapabilityOperation::Query, CapabilityOperation::Capture],
            vec![constraint(CapabilityConstraintKind::RequireUserPresence)],
        );
        assert_eq!(descriptor.role, CapabilityRole::Input as i32);
        assert_eq!(
            descriptor.modalities,
            vec![CapabilityModality::Auditory as i32]
        );
        assert_eq!(
            descriptor.event_kinds,
            vec![CapabilityEventKind::Auditory as i32]
        );
    }
}
