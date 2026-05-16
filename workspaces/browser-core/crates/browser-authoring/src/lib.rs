#![no_std]

extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};

use edgerun_crypto::{sha256, Ed25519SigningKey as SigningKey, Ed25519VerifyingKey};
use edgerun_wire::{
    app_run_prompt_decision_id, package_cache_id, sdk_wire_bytes, AppArtifactRecord,
    AppGraphRecord, AppHttpRouteRecord, AppManifestRecord, AppRunPromptDecisionRecord,
    AppStoreSubmissionRecord, ArtifactSignature, BrowserAppFirstRunInputRecord,
    BrowserAppFirstRunProjectionRecord, BrowserPackageRetrievalRecord, PackageCacheRecord,
    RuntimeAppInstall, RuntimeCapabilityDeclaration, RuntimeEvent, RuntimeHttpRoute,
    SdkWireRecord, WireError, APP_RUN_DECISION_VERIFY_AND_CACHE, PACKAGE_CACHE_STATE_VERIFIED,
    RUNTIME_EVENT_APP_RUN_PROMPT_DECIDED, RUNTIME_EVENT_PACKAGE_CACHE_UPDATED,
    RUNTIME_EVENT_PACKAGE_RETRIEVED, RUNTIME_EVENT_PACKAGE_VERIFIED, SDK_WIRE_ABI_VERSION,
};

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
pub struct VerifiedBrowserAppPackage {
    pub app_id: [u8; 32],
    pub release_id: [u8; 32],
    pub developer_public_key: [u8; 32],
    pub app_slug: Vec<u8>,
    pub manifest_sha256: [u8; 32],
    pub package_sha256: [u8; 32],
    pub code_sha256: [u8; 32],
    pub runtime_projection: RuntimeAppInstall,
    pub app_graph: AppGraphRecord,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserAppFirstRunInput<'a> {
    pub profile_id: [u8; 32],
    pub runtime_id: [u8; 32],
    pub previous_event_sha256: [u8; 32],
    pub first_event_seq: u64,
    pub event_time: u64,
    pub retrieval_cost: u64,
    pub source_admission_hash: [u8; 32],
    pub decision: u16,
    pub user_signature: &'a [u8],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserAppFirstRunProjection {
    pub verified: VerifiedBrowserAppPackage,
    pub decision: AppRunPromptDecisionRecord,
    pub cache: Option<PackageCacheRecord>,
    pub events: Vec<RuntimeEvent>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserPackageRetrievalInput<'a> {
    pub package_key: &'a [u8],
    pub retrieval_cost: u64,
    pub retrieved_at: u64,
    pub source_admission_hash: [u8; 32],
    pub retrieval_evidence_bytes: &'a [u8],
    pub proof_bytes: &'a [u8],
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
    let runtime_projection =
        runtime_projection_for_manifest(&manifest, app_manifest_sha256, release_id);
    let graph = AppGraphRecord {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        app_id,
        developer_public_key,
        app_manifest_sha256,
        app_slug: spec.slug.as_bytes().to_vec(),
        runtime_install: runtime_projection,
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

pub fn verify_browser_app_package(
    app_manifest_bytes: &[u8],
    app_graph_bytes: &[u8],
    developer_signature_bytes: &[u8],
) -> Result<VerifiedBrowserAppPackage, String> {
    let manifest = decode_app_manifest(app_manifest_bytes)?;
    let graph = decode_app_graph(app_graph_bytes)?;
    let developer_signature = decode_artifact_signature(developer_signature_bytes)?;
    let manifest_sha256 = sha256(app_manifest_bytes);
    let package_sha256 = sha256(app_graph_bytes);

    if graph.app_manifest_sha256 != manifest_sha256 {
        return Err("app graph manifest hash does not match manifest bytes".into());
    }
    let slug = core::str::from_utf8(&graph.app_slug)
        .map_err(|_| "app graph slug is not utf-8".to_string())?;
    if manifest.app_slug != graph.app_slug {
        return Err("app graph slug does not match manifest".into());
    }
    if graph.developer_public_key != manifest.developer_id {
        return Err("app graph developer does not match manifest".into());
    }
    if graph.app_id != manifest.app_id || graph.app_id != app_id_for(slug, &manifest.developer_id) {
        return Err("app graph app id does not match manifest and developer".into());
    }
    if graph
        .artifacts
        .iter()
        .find(|artifact| artifact.path == b"app.edapp")
        .is_none_or(|artifact| artifact.sha256 != manifest_sha256)
    {
        return Err("app graph does not bind app.edapp manifest artifact".into());
    }
    let release_id = app_release_id_for(&graph.app_id, manifest_sha256, &graph.artifacts);
    let runtime_projection =
        runtime_projection_for_manifest(&manifest, manifest_sha256, release_id);
    if graph.runtime_install != runtime_projection {
        return Err("app graph runtime projection does not match manifest".into());
    }
    verify_developer_signature(&developer_signature, package_sha256, app_graph_bytes)?;

    Ok(VerifiedBrowserAppPackage {
        app_id: graph.app_id,
        release_id,
        developer_public_key: graph.developer_public_key,
        app_slug: graph.app_slug.clone(),
        manifest_sha256,
        package_sha256,
        code_sha256: manifest.code_sha256,
        runtime_projection,
        app_graph: graph,
    })
}

pub fn first_run_projection_for_verified_app(
    verified: VerifiedBrowserAppPackage,
    input: BrowserAppFirstRunInput<'_>,
) -> BrowserAppFirstRunProjection {
    let decision = AppRunPromptDecisionRecord {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        decision_id: app_run_prompt_decision_id(
            input.profile_id,
            verified.app_id,
            verified.release_id,
            verified.package_sha256,
            verified.manifest_sha256,
            input.decision,
            input.event_time,
        ),
        profile_id: input.profile_id,
        app_id: verified.app_id,
        release_id: verified.release_id,
        package_sha256: verified.package_sha256,
        manifest_sha256: verified.manifest_sha256,
        retrieval_cost: input.retrieval_cost,
        decision: input.decision,
        decided_at: input.event_time,
        user_signature: input.user_signature.to_vec(),
    };
    let cache = (input.decision == APP_RUN_DECISION_VERIFY_AND_CACHE).then(|| PackageCacheRecord {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        cache_id: package_cache_id(
            input.profile_id,
            verified.app_id,
            verified.release_id,
            verified.package_sha256,
            verified.manifest_sha256,
        ),
        profile_id: input.profile_id,
        app_id: verified.app_id,
        release_id: verified.release_id,
        package_sha256: verified.package_sha256,
        manifest_sha256: verified.manifest_sha256,
        code_sha256: verified.code_sha256,
        cached_bytes: 0,
        state: PACKAGE_CACHE_STATE_VERIFIED,
        verified_at: input.event_time,
        source_admission_hash: input.source_admission_hash,
    });

    let mut events = Vec::new();
    let mut previous = input.previous_event_sha256;
    push_runtime_event(
        &mut events,
        &mut previous,
        input.first_event_seq,
        input.event_time,
        RUNTIME_EVENT_PACKAGE_RETRIEVED,
        input.runtime_id,
        sdk_wire_bytes(&SdkWireRecord::AppGraph(verified.app_graph.clone())),
    );
    push_runtime_event(
        &mut events,
        &mut previous,
        input.first_event_seq + 1,
        input.event_time,
        RUNTIME_EVENT_PACKAGE_VERIFIED,
        input.runtime_id,
        sdk_wire_bytes(&SdkWireRecord::RuntimeAppInstall(
            verified.runtime_projection.clone(),
        )),
    );
    push_runtime_event(
        &mut events,
        &mut previous,
        input.first_event_seq + 2,
        input.event_time,
        RUNTIME_EVENT_APP_RUN_PROMPT_DECIDED,
        input.runtime_id,
        sdk_wire_bytes(&SdkWireRecord::AppRunPromptDecision(decision.clone())),
    );
    if let Some(cache) = cache.as_ref() {
        push_runtime_event(
            &mut events,
            &mut previous,
            input.first_event_seq + 3,
            input.event_time,
            RUNTIME_EVENT_PACKAGE_CACHE_UPDATED,
            input.runtime_id,
            sdk_wire_bytes(&SdkWireRecord::PackageCache(cache.clone())),
        );
    }

    BrowserAppFirstRunProjection {
        verified,
        decision,
        cache,
        events,
    }
}

pub fn first_run_projection_for_package(
    app_manifest_bytes: &[u8],
    app_graph_bytes: &[u8],
    developer_signature_bytes: &[u8],
    input: BrowserAppFirstRunInput<'_>,
) -> Result<BrowserAppFirstRunProjection, String> {
    let verified = verify_browser_app_package(
        app_manifest_bytes,
        app_graph_bytes,
        developer_signature_bytes,
    )?;
    Ok(first_run_projection_for_verified_app(verified, input))
}

pub fn first_run_projection_record_for_package(
    app_manifest_bytes: &[u8],
    app_graph_bytes: &[u8],
    developer_signature_bytes: &[u8],
    input_bytes: &[u8],
) -> Result<BrowserAppFirstRunProjectionRecord, String> {
    let input = decode_first_run_input(input_bytes)?;
    let projection = first_run_projection_for_package(
        app_manifest_bytes,
        app_graph_bytes,
        developer_signature_bytes,
        BrowserAppFirstRunInput {
            profile_id: input.profile_id,
            runtime_id: input.runtime_id,
            previous_event_sha256: input.previous_event_sha256,
            first_event_seq: input.first_event_seq,
            event_time: input.event_time,
            retrieval_cost: input.retrieval_cost,
            source_admission_hash: input.source_admission_hash,
            decision: input.decision,
            user_signature: &input.user_signature,
        },
    )?;
    Ok(first_run_projection_record(projection))
}

pub fn first_run_projection_wire_bytes_for_package(
    app_manifest_bytes: &[u8],
    app_graph_bytes: &[u8],
    developer_signature_bytes: &[u8],
    input_bytes: &[u8],
) -> Result<Vec<u8>, String> {
    Ok(sdk_wire_bytes(&SdkWireRecord::BrowserAppFirstRunProjection(
        first_run_projection_record_for_package(
            app_manifest_bytes,
            app_graph_bytes,
            developer_signature_bytes,
            input_bytes,
        )?,
    )))
}

pub fn package_retrieval_record_for_package(
    app_manifest_bytes: &[u8],
    app_graph_bytes: &[u8],
    developer_signature_bytes: &[u8],
    input: BrowserPackageRetrievalInput<'_>,
) -> Result<BrowserPackageRetrievalRecord, String> {
    let verified = verify_browser_app_package(
        app_manifest_bytes,
        app_graph_bytes,
        developer_signature_bytes,
    )?;
    Ok(BrowserPackageRetrievalRecord {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        package_key: input.package_key.to_vec(),
        app_id: verified.app_id,
        release_id: verified.release_id,
        developer_id: verified.developer_public_key,
        package_sha256: verified.package_sha256,
        manifest_sha256: verified.manifest_sha256,
        developer_signature_sha256: sha256(developer_signature_bytes),
        manifest_bytes: app_manifest_bytes.len() as u64,
        graph_bytes: app_graph_bytes.len() as u64,
        developer_signature_bytes: developer_signature_bytes.len() as u64,
        retrieval_cost: input.retrieval_cost,
        retrieved_at: input.retrieved_at,
        source_admission_hash: input.source_admission_hash,
        retrieval_evidence_sha256: sha256(input.retrieval_evidence_bytes),
        proof_sha256: sha256(input.proof_bytes),
    })
}

pub fn package_retrieval_wire_bytes_for_package(
    app_manifest_bytes: &[u8],
    app_graph_bytes: &[u8],
    developer_signature_bytes: &[u8],
    input: BrowserPackageRetrievalInput<'_>,
) -> Result<Vec<u8>, String> {
    Ok(sdk_wire_bytes(&SdkWireRecord::BrowserPackageRetrieval(
        package_retrieval_record_for_package(
            app_manifest_bytes,
            app_graph_bytes,
            developer_signature_bytes,
            input,
        )?,
    )))
}

pub fn first_run_input_wire_bytes(input: BrowserAppFirstRunInput<'_>) -> Vec<u8> {
    sdk_wire_bytes(&SdkWireRecord::BrowserAppFirstRunInput(
        BrowserAppFirstRunInputRecord {
            abi_version: SDK_WIRE_ABI_VERSION,
            flags: 1,
            profile_id: input.profile_id,
            runtime_id: input.runtime_id,
            previous_event_sha256: input.previous_event_sha256,
            first_event_seq: input.first_event_seq,
            event_time: input.event_time,
            retrieval_cost: input.retrieval_cost,
            source_admission_hash: input.source_admission_hash,
            decision: input.decision,
            user_signature: input.user_signature.to_vec(),
        },
    ))
}

pub fn first_run_projection_record(
    projection: BrowserAppFirstRunProjection,
) -> BrowserAppFirstRunProjectionRecord {
    BrowserAppFirstRunProjectionRecord {
        abi_version: SDK_WIRE_ABI_VERSION,
        flags: 1,
        runtime_projection: projection.verified.runtime_projection,
        decision: projection.decision,
        cache: projection.cache,
        events: projection.events,
    }
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

fn runtime_projection_for_manifest(
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

fn decode_app_manifest(bytes: &[u8]) -> Result<AppManifestRecord, String> {
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(bytes)
        .map_err(|_| "invalid app manifest wire record".to_string())?
    {
        SdkWireRecord::AppManifest(manifest)
            if manifest.abi_version == SDK_WIRE_ABI_VERSION && manifest.flags & 1 == 1 =>
        {
            Ok(manifest)
        }
        _ => Err("bytes are not an app manifest record".into()),
    }
}

fn decode_first_run_input(bytes: &[u8]) -> Result<BrowserAppFirstRunInputRecord, String> {
    match edgerun_wire::from_bytes::<SdkWireRecord, WireError>(bytes)
        .map_err(|_| "first-run input is not a valid SDK wire record".to_string())?
    {
        SdkWireRecord::BrowserAppFirstRunInput(input) => {
            if input.abi_version != SDK_WIRE_ABI_VERSION {
                Err("first-run input ABI version is not supported".into())
            } else {
                Ok(input)
            }
        }
        _ => Err("first-run input wire record has unexpected kind".into()),
    }
}

fn decode_app_graph(bytes: &[u8]) -> Result<AppGraphRecord, String> {
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(bytes)
        .map_err(|_| "invalid app graph wire record".to_string())?
    {
        SdkWireRecord::AppGraph(graph)
            if graph.abi_version == SDK_WIRE_ABI_VERSION && graph.flags & 1 == 1 =>
        {
            Ok(graph)
        }
        _ => Err("bytes are not an app graph record".into()),
    }
}

fn decode_artifact_signature(bytes: &[u8]) -> Result<ArtifactSignature, String> {
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(bytes)
        .map_err(|_| "invalid artifact signature wire record".to_string())?
    {
        SdkWireRecord::ArtifactSignature(signature)
            if signature.abi_version == SDK_WIRE_ABI_VERSION && signature.flags & 1 == 1 =>
        {
            Ok(signature)
        }
        _ => Err("bytes are not an artifact signature record".into()),
    }
}

fn verify_developer_signature(
    signature: &ArtifactSignature,
    package_sha256: [u8; 32],
    app_graph_bytes: &[u8],
) -> Result<(), String> {
    if signature.algorithm != SIGN_ALGORITHM_ED25519 {
        return Err("developer signature uses unsupported algorithm".into());
    }
    if signature.artifact_sha256 != package_sha256 {
        return Err("developer signature does not bind app graph hash".into());
    }
    let public_key: [u8; 32] = signature
        .public_key
        .as_slice()
        .try_into()
        .map_err(|_| "developer signature public key is not 32 bytes".to_string())?;
    let key = Ed25519VerifyingKey::from_bytes(&public_key)
        .map_err(|_| "developer signature public key is invalid".to_string())?;
    key.verify(
        &signature_payload_for_domain(DEVELOPER_SIGNATURE_DOMAIN, &sha256(app_graph_bytes)),
        &signature.signature,
    )
    .map_err(|_| "developer signature verification failed".to_string())
}

fn push_runtime_event(
    events: &mut Vec<RuntimeEvent>,
    previous: &mut [u8; 32],
    seq: u64,
    time: u64,
    event_kind: u16,
    runtime_id: [u8; 32],
    payload: Vec<u8>,
) {
    let payload_sha256 = sha256(&payload);
    let event = RuntimeEvent::unsigned_payload(
        seq,
        time,
        event_kind,
        0,
        runtime_id,
        *previous,
        payload_sha256,
        payload,
    );
    *previous = sha256(&sdk_wire_bytes(&SdkWireRecord::RuntimeEvent(event.clone())));
    events.push(event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use edgerun_wire::{ArchivedSdkWireRecord, APP_RUN_DECISION_CANCEL, APP_RUN_DECISION_RUN_ONCE};

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

    #[test]
    fn verifies_publishable_app_and_builds_run_once_events() {
        let package = build_publishable_app(BrowserAppSpec {
            slug: "browser-notes",
            name: "Browser Notes",
            version: "0.1.0",
            summary: "Notes from network storage",
            developer_seed: [9; 32],
            code_sha256: None,
            routes: Vec::new(),
            storage_namespaces: vec![b"notes/state".as_slice()],
            provided_capabilities: Vec::new(),
            required_capabilities: Vec::new(),
            artifacts: vec![BrowserAppArtifact {
                path: "apps/notes.wasm",
                bytes: b"notes wasm",
            }],
        })
        .expect("package");

        let projection = first_run_projection_for_package(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &package.developer_signature,
            BrowserAppFirstRunInput {
                profile_id: [1; 32],
                runtime_id: [2; 32],
                previous_event_sha256: [0; 32],
                first_event_seq: 10,
                event_time: 42,
                retrieval_cost: 5,
                source_admission_hash: [3; 32],
                decision: APP_RUN_DECISION_RUN_ONCE,
                user_signature: b"user-sig",
            },
        )
        .expect("first run projection");

        assert_eq!(projection.verified.app_id, package.app_id);
        assert_eq!(projection.verified.release_id, package.release_id);
        assert_eq!(projection.decision.decision, APP_RUN_DECISION_RUN_ONCE);
        assert!(projection.cache.is_none());
        assert_eq!(projection.events.len(), 3);
        assert_eq!(
            projection.events[0].event_kind,
            RUNTIME_EVENT_PACKAGE_RETRIEVED
        );
        assert_eq!(
            projection.events[1].previous_event_sha256,
            sha256(&sdk_wire_bytes(&SdkWireRecord::RuntimeEvent(
                projection.events[0].clone()
            )))
        );
    }

    #[test]
    fn verify_and_cache_decision_emits_cache_record() {
        let package = build_publishable_app(BrowserAppSpec {
            slug: "browser-cache",
            name: "Browser Cache",
            version: "0.1.0",
            summary: "Cache test",
            developer_seed: [10; 32],
            code_sha256: None,
            routes: Vec::new(),
            storage_namespaces: Vec::new(),
            provided_capabilities: Vec::new(),
            required_capabilities: Vec::new(),
            artifacts: vec![BrowserAppArtifact {
                path: "apps/cache.wasm",
                bytes: b"cache wasm",
            }],
        })
        .expect("package");
        let projection = first_run_projection_for_package(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &package.developer_signature,
            BrowserAppFirstRunInput {
                profile_id: [1; 32],
                runtime_id: [2; 32],
                previous_event_sha256: [0; 32],
                first_event_seq: 1,
                event_time: 99,
                retrieval_cost: 7,
                source_admission_hash: [8; 32],
                decision: APP_RUN_DECISION_VERIFY_AND_CACHE,
                user_signature: b"user-sig",
            },
        )
        .expect("first run projection");

        let cache = projection.cache.expect("cache record");
        assert_eq!(cache.package_sha256, projection.verified.package_sha256);
        assert_eq!(cache.manifest_sha256, projection.verified.manifest_sha256);
        assert_eq!(projection.events.len(), 4);
        assert_eq!(
            projection.events[3].event_kind,
            RUNTIME_EVENT_PACKAGE_CACHE_UPDATED
        );
    }

    #[test]
    fn package_retrieval_record_verifies_package_and_binds_evidence() {
        let package = build_publishable_app(BrowserAppSpec {
            slug: "browser-retrieval",
            name: "Browser Retrieval",
            version: "0.1.0",
            summary: "Retrieval test",
            developer_seed: [13; 32],
            code_sha256: None,
            routes: Vec::new(),
            storage_namespaces: Vec::new(),
            provided_capabilities: Vec::new(),
            required_capabilities: Vec::new(),
            artifacts: vec![BrowserAppArtifact {
                path: "apps/retrieval.wasm",
                bytes: b"retrieval wasm",
            }],
        })
        .expect("package");

        let record = package_retrieval_record_for_package(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &package.developer_signature,
            BrowserPackageRetrievalInput {
                package_key: b"apps/retrieval",
                retrieval_cost: 55,
                retrieved_at: 123,
                source_admission_hash: [9; 32],
                retrieval_evidence_bytes: b"retrieval-evidence",
                proof_bytes: b"proof",
            },
        )
        .expect("retrieval record");

        assert_eq!(record.package_key, b"apps/retrieval".to_vec());
        assert_eq!(record.app_id, package.app_id);
        assert_eq!(record.release_id, package.release_id);
        assert_eq!(record.package_sha256, sha256(&package.app_graph_bytes));
        assert_eq!(record.manifest_sha256, sha256(&package.app_manifest_bytes));
        assert_eq!(record.retrieval_evidence_sha256, sha256(b"retrieval-evidence"));
        assert_eq!(record.proof_sha256, sha256(b"proof"));
        assert!(!package_retrieval_wire_bytes_for_package(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &package.developer_signature,
            BrowserPackageRetrievalInput {
                package_key: b"apps/retrieval",
                retrieval_cost: 55,
                retrieved_at: 123,
                source_admission_hash: [9; 32],
                retrieval_evidence_bytes: b"retrieval-evidence",
                proof_bytes: b"proof",
            },
        )
        .expect("retrieval wire bytes")
        .is_empty());
    }

    #[test]
    fn rejects_tampered_package_graph_and_bad_signature() {
        let package = build_publishable_app(BrowserAppSpec {
            slug: "browser-tamper",
            name: "Browser Tamper",
            version: "0.1.0",
            summary: "Tamper test",
            developer_seed: [11; 32],
            code_sha256: None,
            routes: Vec::new(),
            storage_namespaces: Vec::new(),
            provided_capabilities: Vec::new(),
            required_capabilities: Vec::new(),
            artifacts: vec![BrowserAppArtifact {
                path: "apps/tamper.wasm",
                bytes: b"tamper wasm",
            }],
        })
        .expect("package");
        let mut graph = package.app_graph_bytes.clone();
        let last = graph.len() - 1;
        graph[last] ^= 0x01;
        assert!(verify_browser_app_package(
            &package.app_manifest_bytes,
            &graph,
            &package.developer_signature
        )
        .is_err());

        let mut signature_record =
            decode_artifact_signature(&package.developer_signature).expect("signature record");
        signature_record.signature[0] ^= 0x01;
        let signature = sdk_wire_bytes(&SdkWireRecord::ArtifactSignature(signature_record));
        assert!(verify_browser_app_package(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &signature,
        )
        .is_err());
    }

    #[test]
    fn cancel_decision_does_not_cache_verified_package() {
        let package = build_publishable_app(BrowserAppSpec {
            slug: "browser-cancel",
            name: "Browser Cancel",
            version: "0.1.0",
            summary: "Cancel test",
            developer_seed: [12; 32],
            code_sha256: None,
            routes: Vec::new(),
            storage_namespaces: Vec::new(),
            provided_capabilities: Vec::new(),
            required_capabilities: Vec::new(),
            artifacts: vec![BrowserAppArtifact {
                path: "apps/cancel.wasm",
                bytes: b"cancel wasm",
            }],
        })
        .expect("package");
        let projection = first_run_projection_for_package(
            &package.app_manifest_bytes,
            &package.app_graph_bytes,
            &package.developer_signature,
            BrowserAppFirstRunInput {
                profile_id: [1; 32],
                runtime_id: [2; 32],
                previous_event_sha256: [0; 32],
                first_event_seq: 1,
                event_time: 100,
                retrieval_cost: 7,
                source_admission_hash: [8; 32],
                decision: APP_RUN_DECISION_CANCEL,
                user_signature: b"user-sig",
            },
        )
        .expect("first run projection");
        assert!(projection.cache.is_none());
        assert_eq!(projection.events.len(), 3);
    }
}
