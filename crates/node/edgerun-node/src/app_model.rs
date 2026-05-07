//! App/capability installation model.
//!
//! Device and service crates describe capabilities. The node turns each
//! provider descriptor into an installed app identity and is then the only
//! authority that can route requests to that provider app.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityEventKind, CapabilityModality,
    CapabilityOperation, CapabilityRole,
};
use edgerun_protocols::wire::{
    RuntimeAppInstall, RuntimeCapabilityDeclaration, SDK_WIRE_ABI_VERSION,
};

use crate::runtime::{
    runtime_app_install_with_capabilities, runtime_capability_declaration, sha256,
};

const PROVIDER_APP_DOMAIN: &[u8] = b"edgerun-node.capability-provider-app.v1";
const PROVIDER_RELEASE_DOMAIN: &[u8] = b"edgerun-node.capability-provider-release.v1";
const PROVIDER_CODE_DOMAIN: &[u8] = b"edgerun-node.capability-provider-code.v1";
const PROVIDER_MANIFEST_DOMAIN: &[u8] = b"edgerun-node.capability-provider-manifest.v1";
const PROVIDER_SCOPE_DOMAIN: &[u8] = b"edgerun-node.capability-provider-scope.v1";

#[derive(Clone, Debug, PartialEq)]
pub struct CapabilityProviderApp {
    pub descriptor: CapabilityDescriptor,
    pub install: RuntimeAppInstall,
}

impl CapabilityProviderApp {
    pub fn app_id(&self) -> [u8; 32] {
        self.install.app_id
    }

    pub fn provided_capabilities(&self) -> &[RuntimeCapabilityDeclaration] {
        &self.install.provided_capabilities
    }
}

pub fn capability_provider_app(
    descriptor: CapabilityDescriptor,
    developer_id: [u8; 32],
) -> CapabilityProviderApp {
    let app_id = capability_provider_app_id(&descriptor);
    let release_id = capability_provider_release_id(&descriptor);
    let code_sha256 = capability_provider_code_sha256(&descriptor);
    let manifest_sha256 = capability_provider_manifest_sha256(&descriptor);
    let provided_capabilities = capability_declarations_from_descriptor(&descriptor);
    let install = runtime_app_install_with_capabilities(
        app_id,
        release_id,
        code_sha256,
        developer_id,
        manifest_sha256,
        Vec::new(),
        Vec::new(),
        provided_capabilities,
        Vec::new(),
    );
    CapabilityProviderApp {
        descriptor,
        install,
    }
}

pub fn capability_provider_app_id(descriptor: &CapabilityDescriptor) -> [u8; 32] {
    hash_descriptor_identity(PROVIDER_APP_DOMAIN, descriptor)
}

pub fn capability_provider_release_id(descriptor: &CapabilityDescriptor) -> [u8; 32] {
    hash_descriptor_identity(PROVIDER_RELEASE_DOMAIN, descriptor)
}

pub fn capability_provider_code_sha256(descriptor: &CapabilityDescriptor) -> [u8; 32] {
    hash_descriptor_identity(PROVIDER_CODE_DOMAIN, descriptor)
}

pub fn capability_provider_manifest_sha256(descriptor: &CapabilityDescriptor) -> [u8; 32] {
    hash_descriptor_identity(PROVIDER_MANIFEST_DOMAIN, descriptor)
}

pub fn capability_declarations_from_descriptor(
    descriptor: &CapabilityDescriptor,
) -> Vec<RuntimeCapabilityDeclaration> {
    descriptor
        .operations
        .iter()
        .filter_map(|operation| {
            let operation = u16::try_from(*operation).ok()?;
            if operation == 0 {
                return None;
            }
            Some(runtime_capability_declaration(
                descriptor.role.max(0) as u16,
                operation,
                0,
                capability_scope_sha256(descriptor, operation),
                capability_label(descriptor, operation),
                capability_context(descriptor),
            ))
        })
        .collect()
}

pub fn hardware_capability_descriptor(
    provider_name: impl Into<String>,
    provider_instance_id: impl Into<String>,
    modalities: &[CapabilityModality],
    event_kinds: &[CapabilityEventKind],
    operations: &[CapabilityOperation],
) -> CapabilityDescriptor {
    capability_descriptor(
        provider_name,
        provider_instance_id,
        CapabilityRole::Input,
        modalities,
        event_kinds,
        operations,
        Vec::new(),
    )
}

fn capability_label(descriptor: &CapabilityDescriptor, operation: u16) -> Vec<u8> {
    format!(
        "{}:{}:{}",
        descriptor.provider_name, descriptor.provider_instance_id, operation
    )
    .into_bytes()
}

fn capability_context(descriptor: &CapabilityDescriptor) -> Vec<u8> {
    let mut context = Vec::new();
    context.extend_from_slice(PROVIDER_APP_DOMAIN);
    context.push(0);
    context.extend_from_slice(descriptor.provider_name.as_bytes());
    context.push(0);
    context.extend_from_slice(descriptor.provider_instance_id.as_bytes());
    context
}

fn capability_scope_sha256(descriptor: &CapabilityDescriptor, operation: u16) -> [u8; 32] {
    let mut bytes = descriptor_identity_bytes(PROVIDER_SCOPE_DOMAIN, descriptor);
    bytes.extend_from_slice(&operation.to_le_bytes());
    sha256(&bytes)
}

fn hash_descriptor_identity(domain: &[u8], descriptor: &CapabilityDescriptor) -> [u8; 32] {
    sha256(&descriptor_identity_bytes(domain, descriptor))
}

fn descriptor_identity_bytes(domain: &[u8], descriptor: &CapabilityDescriptor) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(domain);
    bytes.push(0);
    bytes.extend_from_slice(descriptor.provider_name.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(descriptor.provider_instance_id.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(&descriptor.role.to_le_bytes());
    bytes.push(0);
    for modality in &descriptor.modalities {
        bytes.extend_from_slice(&modality.to_le_bytes());
    }
    bytes.push(0);
    for event_kind in &descriptor.event_kinds {
        bytes.extend_from_slice(&event_kind.to_le_bytes());
    }
    bytes.push(0);
    for operation in &descriptor.operations {
        bytes.extend_from_slice(&operation.to_le_bytes());
    }
    bytes
}

pub fn capability_provider_app_record(
    descriptor: CapabilityDescriptor,
    developer_id: [u8; 32],
) -> RuntimeAppInstall {
    capability_provider_app(descriptor, developer_id).install
}

pub fn empty_runtime_capability_declaration() -> RuntimeCapabilityDeclaration {
    RuntimeCapabilityDeclaration {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        capability_kind: 0,
        operation: 0,
        min_assurance: 0,
        scope_sha256: [0; 32],
        label: Vec::new(),
        context: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_descriptor_becomes_stable_provider_app() {
        let descriptor = hardware_capability_descriptor(
            "camera-access",
            "/dev/video0",
            &[CapabilityModality::Visual],
            &[CapabilityEventKind::Visual],
            &[CapabilityOperation::Capture, CapabilityOperation::Observe],
        );
        let first = capability_provider_app(descriptor.clone(), [7; 32]);
        let second = capability_provider_app(descriptor, [7; 32]);
        assert_eq!(first.install.app_id, second.install.app_id);
        assert_eq!(first.install.release_id, second.install.release_id);
        assert_eq!(first.install.provided_capabilities.len(), 2);
        assert!(first.install.required_capabilities.is_empty());
        assert!(first.install.declared_routes.is_empty());
    }
}
