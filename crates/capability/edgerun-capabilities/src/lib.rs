#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

pub mod policy;

pub use edgerun_protocols::core_protocol::protocol::capability::{
    CapabilityAccessClass, CapabilityConstraint, CapabilityConstraintKind, CapabilityDescriptor,
    CapabilityEventKind, CapabilityGrant, CapabilityInvocation, CapabilityModality,
    CapabilityOperation, CapabilityRequest, CapabilityResult, CapabilityRevocation, CapabilityRole,
    CapabilitySelector,
};
pub use policy::{
    GrantRecord, PolicyContext, PolicyDecision, PolicyEngine, RevocationReason, SimplePolicyEngine,
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

impl core::error::Error for CapabilityError {}

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
    use alloc::boxed::Box;
    use alloc::format;
    use alloc::string::ToString;
    use alloc::vec;

    // ── CapabilityError Display & Error trait ─────────────────────────────

    #[test]
    fn error_display_invalid_request() {
        let err = CapabilityError::InvalidRequest("missing field");
        assert_eq!(err.to_string(), "invalid capability request: missing field");
    }

    #[test]
    fn error_display_permission_denied() {
        let err = CapabilityError::PermissionDenied("not authorized");
        assert_eq!(
            err.to_string(),
            "capability permission denied: not authorized"
        );
    }

    #[test]
    fn error_display_unsupported() {
        let err = CapabilityError::Unsupported("unknown op");
        assert_eq!(
            err.to_string(),
            "unsupported capability operation: unknown op"
        );
    }

    #[test]
    fn error_display_provider() {
        let err = CapabilityError::Provider("provider-specific error".to_string());
        assert_eq!(err.to_string(), "provider-specific error");
    }

    #[test]
    fn error_is_std_error() {
        let err: Box<dyn core::error::Error> = Box::new(CapabilityError::InvalidRequest("test"));
        assert!(err.source().is_none());
    }

    #[test]
    fn error_equality() {
        let a = CapabilityError::InvalidRequest("msg");
        let b = CapabilityError::InvalidRequest("msg");
        let c = CapabilityError::InvalidRequest("other");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, CapabilityError::PermissionDenied("msg"));
    }

    #[test]
    fn error_debug_contains_variant() {
        let err = CapabilityError::Unsupported("debug test");
        let debug_str = format!("{err:?}");
        assert!(debug_str.contains("Unsupported"));
    }

    #[test]
    fn error_clone_and_copy() {
        let err = CapabilityError::Provider("clone me".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    // ── Constraint helpers ────────────────────────────────────────────────

    #[test]
    fn constraint_basic() {
        let c = constraint(CapabilityConstraintKind::RequireUserPresence);
        assert_eq!(c.kind, CapabilityConstraintKind::RequireUserPresence as i32);
        assert!(c.uint_value.is_none());
        assert!(c.string_value.is_empty());
        assert!(c.duration_value.is_none());
        assert!(c.rate_limit.is_none());
    }

    #[test]
    fn constraint_with_uint_basic() {
        let c = constraint_with_uint(CapabilityConstraintKind::MaxBytes, 1024);
        assert_eq!(c.kind, CapabilityConstraintKind::MaxBytes as i32);
        assert_eq!(c.uint_value, Some(1024));
        assert!(c.string_value.is_empty());
    }

    #[test]
    fn constraint_with_uint_zero() {
        let c = constraint_with_uint(CapabilityConstraintKind::RateLimited, 0);
        assert_eq!(c.uint_value, Some(0));
    }

    #[test]
    fn constraint_with_uint_max() {
        let c = constraint_with_uint(CapabilityConstraintKind::MaxBytes, u64::MAX);
        assert_eq!(c.uint_value, Some(u64::MAX));
    }

    #[test]
    fn constraint_with_scope_basic() {
        let c = constraint_with_scope("file:/etc/passwd");
        assert_eq!(c.kind, CapabilityConstraintKind::Scope as i32);
        assert_eq!(c.string_value, "file:/etc/passwd");
        assert!(c.uint_value.is_none());
    }

    #[test]
    fn constraint_with_scope_empty_string() {
        let c = constraint_with_scope("");
        assert_eq!(c.string_value, "");
        assert_eq!(c.kind, CapabilityConstraintKind::Scope as i32);
    }

    #[test]
    fn constraint_with_scope_from_string() {
        let scope = String::from("dynamic-scope");
        let c = constraint_with_scope(scope);
        assert_eq!(c.string_value, "dynamic-scope");
    }

    #[test]
    fn constraint_unspecified_kind() {
        let c = constraint(CapabilityConstraintKind::Unspecified);
        assert_eq!(c.kind, CapabilityConstraintKind::Unspecified as i32);
    }

    #[test]
    fn constraint_all_kinds() {
        let kinds = [
            CapabilityConstraintKind::Unspecified,
            CapabilityConstraintKind::RequireUserPresence,
            CapabilityConstraintKind::RequireBiometric,
            CapabilityConstraintKind::RequireFreshness,
            CapabilityConstraintKind::RequireLocalOnly,
            CapabilityConstraintKind::RequireHardwareProtected,
            CapabilityConstraintKind::OneShot,
            CapabilityConstraintKind::RateLimited,
            CapabilityConstraintKind::MaxBytes,
            CapabilityConstraintKind::Scope,
        ];
        for kind in kinds {
            let c = constraint(kind);
            assert_eq!(c.kind, kind as i32);
        }
    }

    // ── capability_descriptor builder ─────────────────────────────────────

    #[test]
    fn descriptor_basic() {
        let d = capability_descriptor(
            "alsa",
            "hw:0,0",
            CapabilityRole::Input,
            &[CapabilityModality::Auditory],
            &[CapabilityEventKind::Auditory],
            &[CapabilityOperation::Query],
            vec![],
        );
        assert_eq!(d.descriptor_version, 1);
        assert_eq!(d.provider_name, "alsa");
        assert_eq!(d.provider_instance_id, "hw:0,0");
        assert_eq!(d.role, CapabilityRole::Input as i32);
        assert!(d.capability_id.is_empty());
        assert!(d.provider_identity.is_none());
        assert!(d.provider_node.is_none());
        assert!(d.signature.is_none());
    }

    #[test]
    fn descriptor_multiple_modalities() {
        let d = capability_descriptor(
            "sensor",
            "sensor-1",
            CapabilityRole::Input,
            &[CapabilityModality::Visual, CapabilityModality::Touch],
            &[CapabilityEventKind::Visual, CapabilityEventKind::Touch],
            &[CapabilityOperation::Capture],
            vec![],
        );
        assert_eq!(d.modalities.len(), 2);
        assert_eq!(
            d.modalities,
            vec![
                CapabilityModality::Visual as i32,
                CapabilityModality::Touch as i32,
            ]
        );
        assert_eq!(d.event_kinds.len(), 2);
    }

    #[test]
    fn descriptor_multiple_operations() {
        let d = capability_descriptor(
            "secure-element",
            "se-0",
            CapabilityRole::SecureElement,
            &[CapabilityModality::Cryptographic],
            &[CapabilityEventKind::Signing],
            &[
                CapabilityOperation::Sign,
                CapabilityOperation::Verify,
                CapabilityOperation::Attest,
            ],
            vec![],
        );
        assert_eq!(d.operations.len(), 3);
        assert_eq!(d.role, CapabilityRole::SecureElement as i32);
    }

    #[test]
    fn descriptor_with_constraints() {
        let constraints = vec![
            constraint(CapabilityConstraintKind::RequireUserPresence),
            constraint_with_uint(CapabilityConstraintKind::MaxBytes, 4096),
        ];
        let d = capability_descriptor(
            "mic",
            "mic-0",
            CapabilityRole::Input,
            &[CapabilityModality::Auditory],
            &[CapabilityEventKind::Auditory],
            &[CapabilityOperation::Capture],
            constraints.clone(),
        );
        assert_eq!(d.default_constraints.len(), 2);
        assert_eq!(d.default_constraints, constraints);
    }

    #[test]
    fn descriptor_from_string_params() {
        let name = String::from("dynamic-name");
        let instance = String::from("dynamic-instance");
        let d = capability_descriptor(
            name,
            instance,
            CapabilityRole::Output,
            &[CapabilityModality::Display],
            &[CapabilityEventKind::Display],
            &[CapabilityOperation::Render],
            vec![],
        );
        assert_eq!(d.provider_name, "dynamic-name");
        assert_eq!(d.provider_instance_id, "dynamic-instance");
    }

    #[test]
    fn descriptor_all_roles() {
        let roles = [
            CapabilityRole::Unspecified,
            CapabilityRole::Input,
            CapabilityRole::Output,
            CapabilityRole::SecureElement,
            CapabilityRole::Communication,
            CapabilityRole::Storage,
            CapabilityRole::Execution,
            CapabilityRole::Derived,
        ];
        for role in roles {
            let d = capability_descriptor(
                "provider",
                "inst",
                role,
                &[CapabilityModality::Other],
                &[CapabilityEventKind::State],
                &[CapabilityOperation::Query],
                vec![],
            );
            assert_eq!(d.role, role as i32);
        }
    }

    #[test]
    fn descriptor_empty_slices_produce_empty_vectors() {
        let d = capability_descriptor("p", "i", CapabilityRole::Input, &[], &[], &[], vec![]);
        assert!(d.modalities.is_empty());
        assert!(d.event_kinds.is_empty());
        assert!(d.operations.is_empty());
    }

    // ── validate_descriptor ───────────────────────────────────────────────

    #[test]
    fn validate_descriptor_valid_minimal() {
        let d = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: Vec::new(),
            provider_identity: None,
            provider_node: None,
            role: CapabilityRole::Input as i32,
            modalities: vec![CapabilityModality::Auditory as i32],
            event_kinds: vec![CapabilityEventKind::Auditory as i32],
            operations: vec![CapabilityOperation::Query as i32],
            default_constraints: Vec::new(),
            provider_name: "test-provider".to_string(),
            provider_instance_id: "test-instance".to_string(),
            signature: None,
        };
        assert!(validate_descriptor(&d).is_ok());
    }

    #[test]
    fn validate_descriptor_empty_provider_name() {
        let d = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: Vec::new(),
            provider_identity: None,
            provider_node: None,
            role: CapabilityRole::Input as i32,
            modalities: vec![CapabilityModality::Auditory as i32],
            event_kinds: vec![CapabilityEventKind::Auditory as i32],
            operations: vec![CapabilityOperation::Query as i32],
            default_constraints: Vec::new(),
            provider_name: String::new(),
            provider_instance_id: "instance".to_string(),
            signature: None,
        };
        let err = validate_descriptor(&d).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability provider name must not be empty")
        );
    }

    #[test]
    fn validate_descriptor_whitespace_only_provider_name() {
        let d = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: Vec::new(),
            provider_identity: None,
            provider_node: None,
            role: CapabilityRole::Input as i32,
            modalities: vec![CapabilityModality::Auditory as i32],
            event_kinds: vec![CapabilityEventKind::Auditory as i32],
            operations: vec![CapabilityOperation::Query as i32],
            default_constraints: Vec::new(),
            provider_name: "   ".to_string(),
            provider_instance_id: "instance".to_string(),
            signature: None,
        };
        assert!(validate_descriptor(&d).is_err());
    }

    #[test]
    fn validate_descriptor_empty_instance_id() {
        let d = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: Vec::new(),
            provider_identity: None,
            provider_node: None,
            role: CapabilityRole::Input as i32,
            modalities: vec![CapabilityModality::Auditory as i32],
            event_kinds: vec![CapabilityEventKind::Auditory as i32],
            operations: vec![CapabilityOperation::Query as i32],
            default_constraints: Vec::new(),
            provider_name: "provider".to_string(),
            provider_instance_id: String::new(),
            signature: None,
        };
        let err = validate_descriptor(&d).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability provider instance id must not be empty")
        );
    }

    #[test]
    fn validate_descriptor_whitespace_only_instance_id() {
        let d = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: Vec::new(),
            provider_identity: None,
            provider_node: None,
            role: CapabilityRole::Input as i32,
            modalities: vec![CapabilityModality::Auditory as i32],
            event_kinds: vec![CapabilityEventKind::Auditory as i32],
            operations: vec![CapabilityOperation::Query as i32],
            default_constraints: Vec::new(),
            provider_name: "provider".to_string(),
            provider_instance_id: " \t\n ".to_string(),
            signature: None,
        };
        assert!(validate_descriptor(&d).is_err());
    }

    #[test]
    fn validate_descriptor_empty_modalities() {
        let d = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: Vec::new(),
            provider_identity: None,
            provider_node: None,
            role: CapabilityRole::Input as i32,
            modalities: Vec::new(),
            event_kinds: vec![CapabilityEventKind::Auditory as i32],
            operations: vec![CapabilityOperation::Query as i32],
            default_constraints: Vec::new(),
            provider_name: "provider".to_string(),
            provider_instance_id: "instance".to_string(),
            signature: None,
        };
        let err = validate_descriptor(&d).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability must declare at least one modality")
        );
    }

    #[test]
    fn validate_descriptor_empty_event_kinds() {
        let d = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: Vec::new(),
            provider_identity: None,
            provider_node: None,
            role: CapabilityRole::Input as i32,
            modalities: vec![CapabilityModality::Auditory as i32],
            event_kinds: Vec::new(),
            operations: vec![CapabilityOperation::Query as i32],
            default_constraints: Vec::new(),
            provider_name: "provider".to_string(),
            provider_instance_id: "instance".to_string(),
            signature: None,
        };
        let err = validate_descriptor(&d).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability must declare at least one event kind")
        );
    }

    #[test]
    fn validate_descriptor_empty_operations() {
        let d = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: Vec::new(),
            provider_identity: None,
            provider_node: None,
            role: CapabilityRole::Input as i32,
            modalities: vec![CapabilityModality::Auditory as i32],
            event_kinds: vec![CapabilityEventKind::Auditory as i32],
            operations: Vec::new(),
            default_constraints: Vec::new(),
            provider_name: "provider".to_string(),
            provider_instance_id: "instance".to_string(),
            signature: None,
        };
        let err = validate_descriptor(&d).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability must declare at least one operation")
        );
    }

    #[test]
    fn validate_descriptor_all_fields_empty() {
        let d = CapabilityDescriptor {
            descriptor_version: 0,
            capability_id: Vec::new(),
            provider_identity: None,
            provider_node: None,
            role: 0,
            modalities: Vec::new(),
            event_kinds: Vec::new(),
            operations: Vec::new(),
            default_constraints: Vec::new(),
            provider_name: String::new(),
            provider_instance_id: String::new(),
            signature: None,
        };
        // Should fail on provider_name first
        let err = validate_descriptor(&d).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability provider name must not be empty")
        );
    }

    #[test]
    fn validate_descriptor_checks_order_name_first() {
        // provider_name empty should fail before checking instance_id
        let d = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: Vec::new(),
            provider_identity: None,
            provider_node: None,
            role: CapabilityRole::Input as i32,
            modalities: vec![CapabilityModality::Auditory as i32],
            event_kinds: vec![CapabilityEventKind::Auditory as i32],
            operations: vec![CapabilityOperation::Query as i32],
            default_constraints: Vec::new(),
            provider_name: String::new(),
            provider_instance_id: String::new(),
            signature: None,
        };
        let err = validate_descriptor(&d).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability provider name must not be empty")
        );
    }

    #[test]
    fn validate_descriptor_checks_order_instance_before_modalities() {
        // empty instance_id should fail before empty modalities
        let d = CapabilityDescriptor {
            descriptor_version: 1,
            capability_id: Vec::new(),
            provider_identity: None,
            provider_node: None,
            role: CapabilityRole::Input as i32,
            modalities: Vec::new(),
            event_kinds: vec![CapabilityEventKind::Auditory as i32],
            operations: vec![CapabilityOperation::Query as i32],
            default_constraints: Vec::new(),
            provider_name: "provider".to_string(),
            provider_instance_id: String::new(),
            signature: None,
        };
        let err = validate_descriptor(&d).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability provider instance id must not be empty")
        );
    }

    // ── validate_grant ────────────────────────────────────────────────────

    #[test]
    fn validate_grant_valid() {
        let grant = CapabilityGrant {
            grant_version: 1,
            grant_id: Vec::new(),
            issuer: None,
            grantee: Some(edgerun_protocols::core_protocol::protocol::IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: None,
                key_hint: None,
            }),
            grantee_node: None,
            selector: Some(CapabilitySelector {
                capability_id: Vec::new(),
                role: CapabilityRole::Input as i32,
                modalities: vec![CapabilityModality::Auditory as i32],
                event_kinds: vec![CapabilityEventKind::Auditory as i32],
                operations: vec![CapabilityOperation::Query as i32],
                access_class: CapabilityAccessClass::Raw as i32,
                provider_identity: None,
                provider_node: None,
                provider_instance_id: String::new(),
            }),
            granted_operations: vec![CapabilityOperation::Query as i32],
            enforced_constraints: Vec::new(),
            access_class: CapabilityAccessClass::Raw as i32,
            issued_at: None,
            expires_at: None,
            correlation_id: Vec::new(),
            supersedes_revocation: None,
            signature: None,
        };
        assert!(validate_grant(&grant).is_ok());
    }

    #[test]
    fn validate_grant_missing_grantee() {
        let grant = CapabilityGrant {
            grant_version: 1,
            grant_id: Vec::new(),
            issuer: None,
            grantee: None,
            grantee_node: None,
            selector: Some(CapabilitySelector {
                capability_id: Vec::new(),
                role: 0,
                modalities: vec![],
                event_kinds: vec![],
                operations: vec![],
                access_class: 0,
                provider_identity: None,
                provider_node: None,
                provider_instance_id: String::new(),
            }),
            granted_operations: vec![CapabilityOperation::Query as i32],
            enforced_constraints: Vec::new(),
            access_class: 0,
            issued_at: None,
            expires_at: None,
            correlation_id: Vec::new(),
            supersedes_revocation: None,
            signature: None,
        };
        let err = validate_grant(&grant).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability grant must name a grantee")
        );
    }

    #[test]
    fn validate_grant_missing_selector() {
        let grant = CapabilityGrant {
            grant_version: 1,
            grant_id: Vec::new(),
            issuer: None,
            grantee: Some(edgerun_protocols::core_protocol::protocol::IdentityRef {
                identity_id: vec![1],
                identity_kind: None,
                key_hint: None,
            }),
            grantee_node: None,
            selector: None,
            granted_operations: vec![CapabilityOperation::Query as i32],
            enforced_constraints: Vec::new(),
            access_class: 0,
            issued_at: None,
            expires_at: None,
            correlation_id: Vec::new(),
            supersedes_revocation: None,
            signature: None,
        };
        let err = validate_grant(&grant).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability grant must include a selector")
        );
    }

    #[test]
    fn validate_grant_empty_operations() {
        let grant = CapabilityGrant {
            grant_version: 1,
            grant_id: Vec::new(),
            issuer: None,
            grantee: Some(edgerun_protocols::core_protocol::protocol::IdentityRef {
                identity_id: vec![1],
                identity_kind: None,
                key_hint: None,
            }),
            grantee_node: None,
            selector: Some(CapabilitySelector {
                capability_id: Vec::new(),
                role: 0,
                modalities: vec![],
                event_kinds: vec![],
                operations: vec![],
                access_class: 0,
                provider_identity: None,
                provider_node: None,
                provider_instance_id: String::new(),
            }),
            granted_operations: Vec::new(),
            enforced_constraints: Vec::new(),
            access_class: 0,
            issued_at: None,
            expires_at: None,
            correlation_id: Vec::new(),
            supersedes_revocation: None,
            signature: None,
        };
        let err = validate_grant(&grant).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability grant must grant at least one operation")
        );
    }

    #[test]
    fn validate_grant_checks_order_grantee_first() {
        // Missing both grantee and selector -> should fail on grantee first
        let grant = CapabilityGrant {
            grant_version: 1,
            grant_id: Vec::new(),
            issuer: None,
            grantee: None,
            grantee_node: None,
            selector: None,
            granted_operations: Vec::new(),
            enforced_constraints: Vec::new(),
            access_class: 0,
            issued_at: None,
            expires_at: None,
            correlation_id: Vec::new(),
            supersedes_revocation: None,
            signature: None,
        };
        let err = validate_grant(&grant).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability grant must name a grantee")
        );
    }

    #[test]
    fn validate_grant_checks_order_selector_before_operations() {
        // Has grantee but no selector, empty operations -> should fail on selector first
        let grant = CapabilityGrant {
            grant_version: 1,
            grant_id: Vec::new(),
            issuer: None,
            grantee: Some(edgerun_protocols::core_protocol::protocol::IdentityRef {
                identity_id: vec![1],
                identity_kind: None,
                key_hint: None,
            }),
            grantee_node: None,
            selector: None,
            granted_operations: Vec::new(),
            enforced_constraints: Vec::new(),
            access_class: 0,
            issued_at: None,
            expires_at: None,
            correlation_id: Vec::new(),
            supersedes_revocation: None,
            signature: None,
        };
        let err = validate_grant(&grant).unwrap_err();
        assert_eq!(
            err,
            CapabilityError::InvalidRequest("capability grant must include a selector")
        );
    }

    // ── Existing tests (preserved) ────────────────────────────────────────

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

    // ── Integration: builder + validate roundtrip ─────────────────────────

    #[test]
    fn builder_produces_valid_descriptor() {
        let d = capability_descriptor(
            "camera",
            "cam-0",
            CapabilityRole::Input,
            &[CapabilityModality::Visual],
            &[CapabilityEventKind::Visual],
            &[CapabilityOperation::Capture, CapabilityOperation::Observe],
            vec![constraint(CapabilityConstraintKind::RequireUserPresence)],
        );
        assert!(validate_descriptor(&d).is_ok());
    }

    #[test]
    fn builder_empty_slices_fails_validation() {
        let d = capability_descriptor("p", "i", CapabilityRole::Input, &[], &[], &[], vec![]);
        assert!(validate_descriptor(&d).is_err());
    }

    // ── CapabilityProvider trait usage ────────────────────────────────────

    struct MockProvider {
        descriptor: CapabilityDescriptor,
    }

    impl CapabilityProvider for MockProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            self.descriptor.clone()
        }
    }

    #[test]
    fn provider_trait_returns_descriptor() {
        let desc = capability_descriptor(
            "mock",
            "mock-0",
            CapabilityRole::Execution,
            &[CapabilityModality::Computational],
            &[CapabilityEventKind::State],
            &[CapabilityOperation::Invoke],
            vec![],
        );
        let provider = MockProvider {
            descriptor: desc.clone(),
        };
        assert_eq!(provider.descriptor(), desc);
    }

    #[test]
    fn provider_trait_descriptor_is_valid() {
        let desc = capability_descriptor(
            "mock",
            "mock-1",
            CapabilityRole::Input,
            &[CapabilityModality::Auditory],
            &[CapabilityEventKind::Auditory],
            &[CapabilityOperation::Query],
            vec![],
        );
        let provider = MockProvider { descriptor: desc };
        assert!(validate_descriptor(&provider.descriptor()).is_ok());
    }

    // ── AccessClass enum coverage ─────────────────────────────────────────

    #[test]
    fn access_class_variants() {
        assert_eq!(CapabilityAccessClass::Unspecified as i32, 0);
        assert_eq!(CapabilityAccessClass::Raw as i32, 1);
        assert_eq!(CapabilityAccessClass::Derived as i32, 2);
    }
}
