//! App-facing helpers for declaring node runtime API use.
//!
//! These constructors build the existing rkyv runtime records. They do not
//! bind sockets, open storage, sign payloads, or grant capabilities; the node
//! validates and records those decisions as local runtime projections. Cross-node
//! authority still comes from admitted work.

use alloc::vec::Vec;

pub use edgerun_protocols::wire::{
    AppManifestRecord, ROUTE_SCHEME_HTTP, ROUTE_SCHEME_HTTPS, RuntimeAppInstall,
    RuntimeCapabilityDeclaration, RuntimeHttpRoute, RuntimeProtocolBinding, SDK_WIRE_ABI_VERSION,
};

use crate::sha256;

pub fn app_id(slug: &[u8], developer_id: &[u8; 32]) -> [u8; 32] {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"edgerun-sdk.runtime-app.v1.app-id");
    bytes.push(0);
    bytes.extend_from_slice(developer_id);
    bytes.push(0);
    bytes.extend_from_slice(slug);
    sha256(&bytes)
}

pub fn release_id(app_id: &[u8; 32], version: &[u8], manifest_sha256: &[u8; 32]) -> [u8; 32] {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"edgerun-sdk.runtime-app.v1.release-id");
    bytes.push(0);
    bytes.extend_from_slice(app_id);
    bytes.push(0);
    bytes.extend_from_slice(version);
    bytes.push(0);
    bytes.extend_from_slice(manifest_sha256);
    sha256(&bytes)
}

pub fn manifest_sha256(slug: &[u8], version: &[u8], route_specs: &[HttpRouteSpec]) -> [u8; 32] {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"edgerun-sdk.runtime-app.v1.manifest");
    bytes.push(0);
    bytes.extend_from_slice(slug);
    bytes.push(0);
    bytes.extend_from_slice(version);
    for route in route_specs {
        bytes.push(0);
        bytes.extend_from_slice(&route.scheme.to_le_bytes());
        bytes.push(0);
        bytes.extend_from_slice(route.host);
        bytes.push(0);
        bytes.extend_from_slice(route.path_prefix);
    }
    sha256(&bytes)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HttpRouteSpec {
    pub scheme: u16,
    pub host: &'static [u8],
    pub path_prefix: &'static [u8],
}

pub fn http_route(
    app_id: [u8; 32],
    release_id: [u8; 32],
    scheme: u16,
    host: impl Into<Vec<u8>>,
    path_prefix: impl Into<Vec<u8>>,
) -> RuntimeHttpRoute {
    RuntimeHttpRoute {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        app_id,
        release_id,
        scheme,
        host: host.into(),
        path_prefix: path_prefix.into(),
    }
}

pub fn runtime_app_projection(
    app_id: [u8; 32],
    release_id: [u8; 32],
    code_sha256: [u8; 32],
    developer_id: [u8; 32],
    manifest_sha256: [u8; 32],
    declared_routes: Vec<RuntimeHttpRoute>,
    storage_namespaces: Vec<Vec<u8>>,
) -> RuntimeAppInstall {
    runtime_app_projection_with_capabilities(
        app_id,
        release_id,
        code_sha256,
        developer_id,
        manifest_sha256,
        declared_routes,
        storage_namespaces,
        Vec::new(),
        Vec::new(),
    )
}

pub fn runtime_app_projection_with_capabilities(
    app_id: [u8; 32],
    release_id: [u8; 32],
    code_sha256: [u8; 32],
    developer_id: [u8; 32],
    manifest_sha256: [u8; 32],
    declared_routes: Vec<RuntimeHttpRoute>,
    storage_namespaces: Vec<Vec<u8>>,
    provided_capabilities: Vec<RuntimeCapabilityDeclaration>,
    required_capabilities: Vec<RuntimeCapabilityDeclaration>,
) -> RuntimeAppInstall {
    RuntimeAppInstall {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        app_id,
        release_id,
        code_sha256,
        developer_id,
        manifest_sha256,
        declared_routes,
        storage_namespaces,
        provided_capabilities,
        required_capabilities,
    }
}

pub fn runtime_app_projection_from_manifest(
    manifest: &AppManifestRecord,
    manifest_sha256: [u8; 32],
    release_id: [u8; 32],
) -> RuntimeAppInstall {
    runtime_app_projection_with_capabilities(
        manifest.app_id,
        release_id,
        manifest.code_sha256,
        manifest.developer_id,
        manifest_sha256,
        manifest
            .routes
            .iter()
            .map(|route| http_route(
                manifest.app_id,
                release_id,
                route.scheme,
                route.host.clone(),
                route.path_prefix.clone(),
            ))
            .collect(),
        manifest.storage_namespaces.clone(),
        manifest.provided_capabilities.clone(),
        manifest.required_capabilities.clone(),
    )
}

pub fn capability_declaration(
    capability_kind: u16,
    operation: u16,
    min_assurance: u16,
    scope_sha256: [u8; 32],
    label: impl Into<Vec<u8>>,
    context: impl Into<Vec<u8>>,
) -> RuntimeCapabilityDeclaration {
    RuntimeCapabilityDeclaration {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        capability_kind,
        operation,
        min_assurance,
        scope_sha256,
        label: label.into(),
        context: context.into(),
    }
}
