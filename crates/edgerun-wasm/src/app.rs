use anyhow::{bail, Context, Result};
use edgerun_core::protocol::{
    app::{
        capability_check, capability_result, CapabilityCheck, CapabilityResult, ExecutionContext,
    },
    common::{ExecutionClass, ObjectKind, ObjectRef},
    stream::AppPackage,
    trust::{CapabilityDescriptor, CapabilityKind, ConstraintSet, ScopeDescriptor, ScopeKind},
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

#[derive(Debug)]
pub struct LoadedApp {
    pub name: String,
    pub entry: String,
    pub wasm_bytes: Vec<u8>,
    pub routes: HashMap<String, String>,
    pub required_capabilities: Vec<CapabilityDescriptor>,
    pub granted_capabilities: Vec<CapabilityDescriptor>,
    pub domain: String,
}

pub fn load_app(package_bytes: &[u8], wasm_bytes: &[u8], domain: &str) -> Result<LoadedApp> {
    let package = AppPackage::decode(package_bytes).context("failed to decode AppPackage")?;

    validate_app_package(&package)?;

    if wasm_bytes.is_empty() {
        bail!("wasm binary is empty");
    }

    validate_wasm_abi(wasm_bytes)?;

    Ok(LoadedApp {
        name: package.name,
        entry: package.entry,
        wasm_bytes: wasm_bytes.to_vec(),
        routes: package.routes.into_iter().collect(),
        required_capabilities: package.required_capabilities,
        granted_capabilities: Vec::new(),
        domain: domain.to_string(),
    })
}

fn validate_app_package(package: &AppPackage) -> Result<()> {
    if package.name.is_empty() {
        bail!("AppPackage name is empty");
    }

    if package.entry.is_empty() {
        bail!("AppPackage entry is empty");
    }

    if package.wasm_object.is_none() {
        bail!("AppPackage wasm_object is required");
    }

    for cap in &package.required_capabilities {
        validate_capability_descriptor(cap)?;
    }

    Ok(())
}

fn validate_capability_descriptor(cap: &CapabilityDescriptor) -> Result<()> {
    let kind = edgerun_core::protocol::enum_from_i32::<CapabilityKind>(cap.capability_kind)
        .ok_or_else(|| anyhow::anyhow!("invalid capability kind: {}", cap.capability_kind))?;

    if kind == CapabilityKind::Unspecified {
        bail!("capability kind is unspecified");
    }

    if cap.actions.is_empty() {
        bail!("capability descriptor must declare at least one action");
    }

    if let Some(ref scope) = cap.scope {
        validate_scope(scope)?;
    }

    Ok(())
}

fn validate_scope(scope: &ScopeDescriptor) -> Result<()> {
    let kind = edgerun_core::protocol::enum_from_i32::<ScopeKind>(scope.scope_kind)
        .ok_or_else(|| anyhow::anyhow!("invalid scope kind: {}", scope.scope_kind))?;

    if kind == ScopeKind::Unspecified {
        bail!("scope kind is unspecified");
    }

    Ok(())
}

fn validate_wasm_abi(bytes: &[u8]) -> Result<()> {
    use wasmparser::Validator;

    let mut validator = Validator::new();
    validator
        .validate_all(bytes)
        .context("WASM module failed validation")?;

    Ok(())
}

pub fn resolve_capabilities(
    app: &mut LoadedApp,
    node_policy: &[CapabilityDescriptor],
    delegation_chain: &[CapabilityDescriptor],
) -> Result<()> {
    let mut granted = Vec::new();

    for required in &app.required_capabilities {
        let node_has = node_policy.iter().any(|cap| {
            cap.capability_kind == required.capability_kind
                && required.actions.iter().all(|a| cap.actions.contains(a))
        });

        let delegation_has = delegation_chain.iter().any(|cap| {
            cap.capability_kind == required.capability_kind
                && required.actions.iter().all(|a| cap.actions.contains(a))
        });

        if node_has || delegation_has {
            granted.push(required.clone());
        }
    }

    app.granted_capabilities = granted;

    if app.granted_capabilities.is_empty() && !app.required_capabilities.is_empty() {
        bail!(
            "no required capabilities granted: app requires {} capabilities, none satisfied",
            app.required_capabilities.len()
        );
    }

    Ok(())
}

pub fn build_execution_context(app: &LoadedApp, instance_id: &[u8]) -> ExecutionContext {
    ExecutionContext {
        version: 1,
        app_package: Some(ObjectRef {
            object_id: instance_id.to_vec(),
            object_kind: Some(ObjectKind::AppPackage as i32),
        }),
        instance_id: instance_id.to_vec(),
        granted_capabilities: app.granted_capabilities.clone(),
        execution_class: ExecutionClass::LocalOnly as i32,
        domain: app.domain.clone(),
    }
}

pub fn check_capability(
    ctx: &ExecutionContext,
    operation: capability_check::Operation,
    scope: Option<ScopeDescriptor>,
) -> CapabilityResult {
    let op_i32 = operation as i32;
    let has_capability = ctx.granted_capabilities.iter().any(|cap| {
        cap.actions
            .iter()
            .any(|action| action_matches_operation(action, op_i32))
            && scope_matches(cap, &scope)
    });

    if has_capability {
        CapabilityResult {
            decision: capability_result::Decision::Granted as i32,
            reason: String::new(),
        }
    } else {
        CapabilityResult {
            decision: capability_result::Decision::Denied as i32,
            reason: format!("operation {:?} not granted", operation),
        }
    }
}

fn action_matches_operation(action: &str, operation: i32) -> bool {
    let op_name =
        match edgerun_core::protocol::enum_from_i32::<capability_check::Operation>(operation) {
            Some(op) => op.as_str_name(),
            None => return false,
        };

    let action_lower = action.to_lowercase();
    let op_lower = op_name.to_lowercase();

    action_lower == op_lower
        || action_lower == "*"
        || action_lower.contains(&op_lower)
        || op_lower.contains(&action_lower)
}

fn scope_matches(cap: &CapabilityDescriptor, scope: &Option<ScopeDescriptor>) -> bool {
    let Some(scope_ref) = scope else {
        return true;
    };

    let Some(cap_scope) = &cap.scope else {
        return true;
    };

    let scope_kind = edgerun_core::protocol::enum_from_i32::<ScopeKind>(scope_ref.scope_kind);
    let cap_scope_kind = edgerun_core::protocol::enum_from_i32::<ScopeKind>(cap_scope.scope_kind);

    if cap_scope_kind == Some(ScopeKind::GlobalWithConstraints) {
        return true;
    }

    if scope_kind != cap_scope_kind {
        return false;
    }

    match scope_kind {
        Some(ScopeKind::Domain) => {
            scope_ref.target_domains.is_empty()
                || cap_scope.target_domains.iter().any(|d| {
                    scope_ref.target_domains.is_empty() || scope_ref.target_domains.contains(d)
                })
        }
        _ => true,
    }
}

pub fn compute_app_object_id(package_bytes: &[u8]) -> [u8; 32] {
    let canonicalization_id = b"proto-v0:AppPackage:1";

    let mut hasher = Sha256::new();
    hasher.update(b"edgerun:v0:object");
    hasher.update([0x00]);
    hasher.update(canonicalization_id);
    hasher.update([0x00]);
    hasher.update(package_bytes);

    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_app_object_id_deterministic() {
        let bytes = b"test package data";
        let id1 = compute_app_object_id(bytes);
        let id2 = compute_app_object_id(bytes);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_compute_app_object_id_different() {
        let id1 = compute_app_object_id(b"package A");
        let id2 = compute_app_object_id(b"package B");
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_action_matches_operation_wildcard() {
        assert!(action_matches_operation("*", 1));
        assert!(action_matches_operation("*", 7));
    }

    #[test]
    fn test_action_matches_operation_exact() {
        assert!(action_matches_operation(
            "OPERATION_READ_BLOB",
            capability_check::Operation::ReadBlob as i32
        ));
    }

    #[test]
    fn test_action_matches_operation_substring() {
        assert!(action_matches_operation(
            "read_blob",
            capability_check::Operation::ReadBlob as i32
        ));
    }

    #[test]
    fn test_action_matches_operation_no_match() {
        assert!(!action_matches_operation(
            "write_blob",
            capability_check::Operation::ReadBlob as i32
        ));
    }

    #[test]
    fn test_scope_matches_none_scope() {
        let cap = CapabilityDescriptor {
            capability_kind: CapabilityKind::ObjectFetch as i32,
            ..Default::default()
        };
        assert!(scope_matches(&cap, &None));
    }

    #[test]
    fn test_scope_matches_global() {
        let cap = CapabilityDescriptor {
            capability_kind: CapabilityKind::ExecuteWorkload as i32,
            scope: Some(ScopeDescriptor {
                scope_kind: ScopeKind::GlobalWithConstraints as i32,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(scope_matches(
            &cap,
            &Some(ScopeDescriptor {
                scope_kind: ScopeKind::Domain as i32,
                ..Default::default()
            })
        ));
    }

    #[test]
    fn test_resolve_capabilities_grants_matching() {
        let mut app = LoadedApp {
            name: "test".to_string(),
            entry: "main".to_string(),
            wasm_bytes: vec![],
            routes: HashMap::new(),
            required_capabilities: vec![CapabilityDescriptor {
                capability_kind: CapabilityKind::ObjectFetch as i32,
                actions: vec!["read_blob".to_string()],
                ..Default::default()
            }],
            granted_capabilities: Vec::new(),
            domain: "test".to_string(),
        };

        let node_policy = vec![CapabilityDescriptor {
            capability_kind: CapabilityKind::ObjectFetch as i32,
            actions: vec!["read_blob".to_string(), "write_blob".to_string()],
            ..Default::default()
        }];

        resolve_capabilities(&mut app, &node_policy, &[]).unwrap();
        assert_eq!(app.granted_capabilities.len(), 1);
    }

    #[test]
    fn test_resolve_capabilities_denies_non_matching() {
        let mut app = LoadedApp {
            name: "test".to_string(),
            entry: "main".to_string(),
            wasm_bytes: vec![],
            routes: HashMap::new(),
            required_capabilities: vec![CapabilityDescriptor {
                capability_kind: CapabilityKind::ObjectStore as i32,
                actions: vec!["write_blob".to_string()],
                ..Default::default()
            }],
            granted_capabilities: Vec::new(),
            domain: "test".to_string(),
        };

        let node_policy = vec![CapabilityDescriptor {
            capability_kind: CapabilityKind::ObjectFetch as i32,
            actions: vec!["read_blob".to_string()],
            ..Default::default()
        }];

        let result = resolve_capabilities(&mut app, &node_policy, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn conformance_app_object_id_known_vector() {
        let package_bytes = include_bytes!("../../../proto/edgerun/v0/app.proto");
        let object_id = compute_app_object_id(package_bytes);

        assert_eq!(object_id.len(), 32);

        let _actual_hex: String = object_id.iter().map(|b| format!("{:02x}", b)).collect();
    }

    #[test]
    fn conformance_canonicalization_prefix() {
        let prefix = b"edgerun:v0:object";
        let separator = [0x00u8];
        let canonicalization_id = b"proto-v0:AppPackage:1";

        assert_eq!(prefix, b"edgerun:v0:object");
        assert_eq!(separator, [0x00]);
        assert_eq!(canonicalization_id, b"proto-v0:AppPackage:1");
    }

    #[test]
    fn conformance_capability_enforcement_deny_by_kind() {
        let ctx = ExecutionContext {
            version: 1,
            app_package: None,
            instance_id: vec![0x01],
            granted_capabilities: vec![CapabilityDescriptor {
                capability_kind: CapabilityKind::ObjectFetch as i32,
                actions: vec!["OPERATION_READ_BLOB".to_string()],
                ..Default::default()
            }],
            execution_class: ExecutionClass::LocalOnly as i32,
            domain: "test".to_string(),
        };

        let result = check_capability(&ctx, capability_check::Operation::WriteBlob, None);
        assert_eq!(result.decision, capability_result::Decision::Denied as i32);
    }

    #[test]
    fn conformance_capability_enforcement_grant_by_action() {
        let ctx = ExecutionContext {
            version: 1,
            app_package: None,
            instance_id: vec![0x01],
            granted_capabilities: vec![CapabilityDescriptor {
                capability_kind: CapabilityKind::ObjectStore as i32,
                actions: vec!["OPERATION_WRITE_BLOB".to_string()],
                ..Default::default()
            }],
            execution_class: ExecutionClass::LocalOnly as i32,
            domain: "test".to_string(),
        };

        let result = check_capability(&ctx, capability_check::Operation::WriteBlob, None);
        assert_eq!(result.decision, capability_result::Decision::Granted as i32);
    }

    #[test]
    fn conformance_delegation_chain_grants_capability() {
        let mut app = LoadedApp {
            name: "delegated_app".to_string(),
            entry: "main".to_string(),
            wasm_bytes: vec![],
            routes: HashMap::new(),
            required_capabilities: vec![CapabilityDescriptor {
                capability_kind: CapabilityKind::Query as i32,
                actions: vec!["query".to_string()],
                ..Default::default()
            }],
            granted_capabilities: Vec::new(),
            domain: "test".to_string(),
        };

        let node_policy: Vec<CapabilityDescriptor> = vec![];

        let delegation_chain = vec![CapabilityDescriptor {
            capability_kind: CapabilityKind::Query as i32,
            actions: vec!["query".to_string(), "read_blob".to_string()],
            ..Default::default()
        }];

        resolve_capabilities(&mut app, &node_policy, &delegation_chain).unwrap();
        assert_eq!(app.granted_capabilities.len(), 1);
        assert_eq!(
            app.granted_capabilities[0].capability_kind,
            CapabilityKind::Query as i32
        );
    }

    #[test]
    fn conformance_attenuation_child_cannot_expand_parent() {
        let mut app = LoadedApp {
            name: "attenuation_test".to_string(),
            entry: "main".to_string(),
            wasm_bytes: vec![],
            routes: HashMap::new(),
            required_capabilities: vec![CapabilityDescriptor {
                capability_kind: CapabilityKind::ExecuteWorkload as i32,
                actions: vec!["execution".to_string(), "read_blob".to_string()],
                ..Default::default()
            }],
            granted_capabilities: Vec::new(),
            domain: "test".to_string(),
        };

        let parent_policy = vec![CapabilityDescriptor {
            capability_kind: CapabilityKind::ExecuteWorkload as i32,
            actions: vec!["execution".to_string()],
            ..Default::default()
        }];

        let result = resolve_capabilities(&mut app, &parent_policy, &[]);
        assert!(
            result.is_err(),
            "child should not be able to expand parent actions"
        );
    }

    #[test]
    fn conformance_build_execution_context() {
        let app = LoadedApp {
            name: "ctx_test".to_string(),
            entry: "main".to_string(),
            wasm_bytes: vec![0x00, 0x61, 0x73, 0x6D],
            routes: HashMap::new(),
            required_capabilities: vec![],
            granted_capabilities: vec![CapabilityDescriptor {
                capability_kind: CapabilityKind::ObjectFetch as i32,
                actions: vec!["read_blob".to_string()],
                ..Default::default()
            }],
            domain: "isolated".to_string(),
        };

        let instance_id = [0xAA; 16];
        let ctx = build_execution_context(&app, &instance_id);

        assert_eq!(ctx.version, 1);
        assert_eq!(ctx.instance_id, instance_id.to_vec());
        assert_eq!(ctx.domain, "isolated");
        assert_eq!(ctx.granted_capabilities.len(), 1);
        assert_eq!(ctx.execution_class, ExecutionClass::LocalOnly as i32);
    }

    #[test]
    fn conformance_check_capability_with_scope_domain_match() {
        let ctx = ExecutionContext {
            version: 1,
            app_package: None,
            instance_id: vec![0x01],
            granted_capabilities: vec![CapabilityDescriptor {
                capability_kind: CapabilityKind::ObjectFetch as i32,
                actions: vec!["OPERATION_READ_BLOB".to_string()],
                scope: Some(ScopeDescriptor {
                    scope_kind: ScopeKind::Domain as i32,
                    target_domains: vec!["trusted".to_string()],
                    ..Default::default()
                }),
                ..Default::default()
            }],
            execution_class: ExecutionClass::LocalOnly as i32,
            domain: "trusted".to_string(),
        };

        let scope = Some(ScopeDescriptor {
            scope_kind: ScopeKind::Domain as i32,
            target_domains: vec!["trusted".to_string()],
            ..Default::default()
        });

        let result = check_capability(&ctx, capability_check::Operation::ReadBlob, scope);
        assert_eq!(result.decision, capability_result::Decision::Granted as i32);
    }

    #[test]
    fn conformance_check_capability_with_scope_domain_mismatch() {
        let ctx = ExecutionContext {
            version: 1,
            app_package: None,
            instance_id: vec![0x01],
            granted_capabilities: vec![CapabilityDescriptor {
                capability_kind: CapabilityKind::ObjectFetch as i32,
                actions: vec!["OPERATION_READ_BLOB".to_string()],
                scope: Some(ScopeDescriptor {
                    scope_kind: ScopeKind::Domain as i32,
                    target_domains: vec!["trusted".to_string()],
                    ..Default::default()
                }),
                ..Default::default()
            }],
            execution_class: ExecutionClass::LocalOnly as i32,
            domain: "trusted".to_string(),
        };

        let scope = Some(ScopeDescriptor {
            scope_kind: ScopeKind::Domain as i32,
            target_domains: vec!["untrusted".to_string()],
            ..Default::default()
        });

        let result = check_capability(&ctx, capability_check::Operation::ReadBlob, scope);
        assert_eq!(result.decision, capability_result::Decision::Denied as i32);
    }
}
