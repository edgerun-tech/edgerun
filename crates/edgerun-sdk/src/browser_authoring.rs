use alloc::{string::String, vec, vec::Vec};

use edgerun_crypto::Ed25519SigningKey as SigningKey;
use edgerun_wire::{
    AppArtifactRecord, AppGraphRecord, AppHttpRouteRecord, AppManifestRecord,
    AppStoreSubmissionRecord, ArtifactSignature, RuntimeAppInstall, RuntimeCapabilityDeclaration,
    RuntimeHttpRoute, SdkWireRecord, sdk_wire_bytes,
};

use crate::sha256;

pub const APP_ID_DOMAIN: &[u8] = b"edgerun-sdk.eapp.v1.app-id";
pub const DEVELOPER_SIGNATURE_DOMAIN: &[u8] = b"edgerun-sdk.esig.v1.app.developer";
pub const STORE_SIGNATURE_DOMAIN: &[u8] = b"edgerun-sdk.esig.v1.app.store";
pub const SIGN_ALGORITHM_ED25519: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrowserAppRoute<'a> {
    pub scheme: u16,
    pub host: &'a [u8],
    pub path_prefix: &'a [u8],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrowserAppArtifact<'a> {
    pub path: &'a str,
    pub bytes: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserAppSpec<'a> {
    pub slug: &'a str,
    pub name: &'a str,
    pub version: &'a str,
    pub summary: &'a str,
    pub developer_seed: [u8; 32],
    pub code_sha256: Option<[u8; 32]>,
    pub routes: Vec<BrowserAppRoute<'a>>,
    pub storage_namespaces: Vec<&'a [u8]>,
    pub provided_capabilities: Vec<RuntimeCapabilityDeclaration>,
    pub required_capabilities: Vec<RuntimeCapabilityDeclaration>,
    pub artifacts: Vec<BrowserAppArtifact<'a>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserAppPackage {
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub developer_public_key: [u8; 32],
    pub app_slug: Vec<u8>,
    pub app_manifest_sha256: [u8; 32],
    pub app_graph_sha256: [u8; 32],
    pub developer_signature: Vec<u8>,
    pub app_manifest_bytes: Vec<u8>,
    pub app_graph_bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserSubmissionSpec<'a> {
    pub submitted_at: u64,
    pub package_ref: &'a [u8],
    pub manifest_ref: &'a [u8],
    pub notes: &'a [u8],
}

pub fn app_id_for(slug: &str, developer_public_key: &[u8; 32]) -> [u8; 32] {
    let mut bytes =
        Vec::with_capacity(APP_ID_DOMAIN.len() + developer_public_key.len() + slug.len());
    bytes.extend_from_slice(APP_ID_DOMAIN);
    bytes.extend_from_slice(developer_public_key);
    bytes.extend_from_slice(slug.as_bytes());
    sha256(&bytes)
}

pub fn build_publishable_app(spec: BrowserAppSpec<'_>) -> Result<BrowserAppPackage, String> {
    validate_artifact_paths(&spec.artifacts)?;

    let developer_key = SigningKey::from_bytes(&spec.developer_seed);
    let developer_public_key = *developer_key.verifying_key().as_bytes();
    let app_id = app_id_for(spec.slug, &developer_public_key);

    let manifest = AppManifestRecord {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        app_id,
        developer_id: developer_public_key,
        app_slug: spec.slug.as_bytes().to_vec(),
        name: spec.name.as_bytes().to_vec(),
        version: spec.version.as_bytes().to_vec(),
        summary: spec.summary.as_bytes().to_vec(),
        code_sha256: spec
            .code_sha256
            .unwrap_or_else(|| app_code_sha256_for(spec.slug, &spec.artifacts)),
        routes: spec
            .routes
            .iter()
            .map(|route| AppHttpRouteRecord {
                scheme: route.scheme,
                host: route.host.to_vec(),
                path_prefix: route.path_prefix.to_vec(),
            })
            .collect(),
        storage_namespaces: spec
            .storage_namespaces
            .iter()
            .map(|namespace| namespace.to_vec())
            .collect(),
        provided_capabilities: spec.provided_capabilities,
        required_capabilities: spec.required_capabilities,
    };
    let app_manifest_bytes = sdk_wire_bytes(&SdkWireRecord::AppManifest(manifest.clone()));
    let app_manifest_sha256 = sha256(&app_manifest_bytes);

    let mut artifacts = artifact_records(&spec.artifacts);
    artifacts.push(AppArtifactRecord {
        kind: app_artifact_kind("app.edapp"),
        path: b"app.edapp".to_vec(),
        sha256: app_manifest_sha256,
    });
    artifacts.sort_by(|left, right| left.path.cmp(&right.path));

    let release_id = app_release_id_for(&app_id, app_manifest_sha256, &artifacts);
    let runtime_install = runtime_install_for_manifest(&manifest, app_manifest_sha256, release_id);
    let graph = AppGraphRecord {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        app_id,
        developer_public_key,
        app_manifest_sha256,
        app_slug: spec.slug.as_bytes().to_vec(),
        runtime_install,
        artifacts,
    };
    let app_graph_bytes = sdk_wire_bytes(&SdkWireRecord::AppGraph(graph));
    let app_graph_sha256 = sha256(&app_graph_bytes);
    let developer_signature =
        artifact_signature_bytes(&app_graph_bytes, &developer_key, DEVELOPER_SIGNATURE_DOMAIN);

    Ok(BrowserAppPackage {
        app_id,
        release_id,
        developer_public_key,
        app_slug: spec.slug.as_bytes().to_vec(),
        app_manifest_sha256,
        app_graph_sha256,
        developer_signature,
        app_manifest_bytes,
        app_graph_bytes,
    })
}

pub fn build_app_store_submission(
    package: &BrowserAppPackage,
    spec: BrowserSubmissionSpec<'_>,
) -> Vec<u8> {
    sdk_wire_bytes(&SdkWireRecord::AppStoreSubmission(
        AppStoreSubmissionRecord {
            abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
            flags: 1,
            submitted_at: spec.submitted_at,
            app_id: package.app_id,
            release_id: package.release_id,
            developer_id: package.developer_public_key,
            app_graph_sha256: package.app_graph_sha256,
            manifest_sha256: package.app_manifest_sha256,
            package_sha256: package.app_graph_sha256,
            package_bytes: package.app_graph_bytes.len() as u64,
            app_slug: package.app_slug.clone(),
            package_ref: spec.package_ref.to_vec(),
            manifest_ref: spec.manifest_ref.to_vec(),
            notes: spec.notes.to_vec(),
            app_graph: app_graph_from_package(package),
            developer_signature: package.developer_signature.clone(),
        },
    ))
}

fn app_graph_from_package(package: &BrowserAppPackage) -> AppGraphRecord {
    edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&package.app_graph_bytes)
        .ok()
        .and_then(|record| match record {
            SdkWireRecord::AppGraph(graph) => Some(graph),
            _ => None,
        })
        .expect("browser package app_graph_bytes must contain an AppGraph record")
}

pub fn artifact_signature_bytes(
    artifact_bytes: &[u8],
    signing_key: &SigningKey,
    domain: &[u8],
) -> Vec<u8> {
    let artifact_hash = sha256(artifact_bytes);
    let signature = signing_key.sign_bytes(&signature_payload_for_domain(domain, &artifact_hash));
    sdk_wire_bytes(&SdkWireRecord::ArtifactSignature(ArtifactSignature {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        algorithm: SIGN_ALGORITHM_ED25519,
        artifact_sha256: artifact_hash,
        public_key: signing_key.verifying_key().as_bytes().to_vec(),
        signature: signature.to_vec(),
    }))
}

pub fn signature_payload_for_domain(domain: &[u8], artifact_hash: &[u8; 32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(domain.len() + artifact_hash.len());
    payload.extend_from_slice(domain);
    payload.extend_from_slice(artifact_hash);
    payload
}

fn artifact_records(artifacts: &[BrowserAppArtifact<'_>]) -> Vec<AppArtifactRecord> {
    artifacts
        .iter()
        .map(|artifact| AppArtifactRecord {
            kind: app_artifact_kind(artifact.path),
            path: artifact.path.as_bytes().to_vec(),
            sha256: sha256(artifact.bytes),
        })
        .collect()
}

fn runtime_install_for_manifest(
    manifest: &AppManifestRecord,
    manifest_sha256: [u8; 32],
    release_id: [u8; 32],
) -> RuntimeAppInstall {
    RuntimeAppInstall {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        app_id: manifest.app_id,
        release_id,
        code_sha256: manifest.code_sha256,
        developer_id: manifest.developer_id,
        manifest_sha256,
        declared_routes: manifest
            .routes
            .iter()
            .map(|route| RuntimeHttpRoute {
                abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
                flags: 1,
                app_id: manifest.app_id,
                release_id,
                scheme: route.scheme,
                host: route.host.clone(),
                path_prefix: route.path_prefix.clone(),
            })
            .collect(),
        storage_namespaces: manifest.storage_namespaces.clone(),
        provided_capabilities: manifest.provided_capabilities.clone(),
        required_capabilities: manifest.required_capabilities.clone(),
    }
}

fn app_release_id_for(
    app_id: &[u8; 32],
    manifest_sha256: [u8; 32],
    artifacts: &[AppArtifactRecord],
) -> [u8; 32] {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"edgerun-sdk.eapp.v1.release-id");
    bytes.extend_from_slice(app_id);
    bytes.extend_from_slice(&manifest_sha256);
    for artifact in artifacts {
        bytes.extend_from_slice(&artifact.path);
        bytes.push(0);
        bytes.extend_from_slice(&artifact.kind.to_le_bytes());
        bytes.extend_from_slice(&artifact.sha256);
    }
    sha256(&bytes)
}

fn app_code_sha256_for(slug: &str, artifacts: &[BrowserAppArtifact<'_>]) -> [u8; 32] {
    let mut records = artifact_records(artifacts);
    records.sort_by(|left, right| left.path.cmp(&right.path));
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"edgerun-sdk.eapp.v1.code");
    bytes.extend_from_slice(slug.as_bytes());
    for artifact in records
        .iter()
        .filter(|artifact| matches!(artifact.kind, 2 | 3 | 4 | 5))
    {
        bytes.extend_from_slice(&artifact.path);
        bytes.push(0);
        bytes.extend_from_slice(&artifact.sha256);
    }
    sha256(&bytes)
}

fn app_artifact_kind(path: &str) -> u16 {
    if path == "app.edapp" {
        1
    } else if path.ends_with(".edm") {
        2
    } else if path.ends_with(".wasm") {
        3
    } else if path.ends_with(".so") || path.ends_with(".dylib") || path.ends_with(".dll") {
        4
    } else if path.ends_with(".html") || path.ends_with(".js") {
        5
    } else {
        0
    }
}

fn validate_artifact_paths(artifacts: &[BrowserAppArtifact<'_>]) -> Result<(), String> {
    for artifact in artifacts {
        if artifact.path.is_empty()
            || artifact.path.starts_with('/')
            || artifact.path.contains('\\')
            || artifact
                .path
                .split('/')
                .any(|part| part == "." || part == ".." || part.is_empty())
        {
            return Err("app artifact paths must be normalized relative paths".into());
        }
        if matches!(
            artifact.path,
            "app.eapp" | "app.edapp" | "developer.esig" | "store.esig"
        ) {
            return Err("app artifact path is reserved".into());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_wire::ArchivedSdkWireRecord;

    #[test]
    fn builds_publishable_app_records_from_memory() {
        let package = build_publishable_app(BrowserAppSpec {
            slug: "browser-wallet",
            name: "Browser Wallet",
            version: "0.1.0",
            summary: "Wallet built in the browser",
            developer_seed: [7; 32],
            code_sha256: None,
            routes: Vec::new(),
            storage_namespaces: vec![b"wallet/state".as_slice()],
            provided_capabilities: Vec::new(),
            required_capabilities: Vec::new(),
            artifacts: vec![BrowserAppArtifact {
                path: "apps/wallet.wasm",
                bytes: b"fake wasm bytes",
            }],
        })
        .expect("package");

        assert!(!package.app_manifest_bytes.is_empty());
        assert!(!package.app_graph_bytes.is_empty());
        assert!(!package.developer_signature.is_empty());
        let archived = edgerun_wire::access::<ArchivedSdkWireRecord, edgerun_wire::WireError>(
            &package.app_graph_bytes,
        )
        .expect("archived graph");
        assert!(matches!(archived, ArchivedSdkWireRecord::AppGraph(_)));
    }
}
