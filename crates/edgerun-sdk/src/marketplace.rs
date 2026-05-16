use super::*;
use edgerun_json::{JsonValue, parse_json};

fn verify_ed25519_signature(
    public_key: &[u8; 32],
    domain: &[u8],
    artifact_hash: &[u8; 32],
    signature: &[u8],
) -> bool {
    edgerun_crypto::verification::ed25519_verify(
        public_key,
        &signature_payload_for_domain(domain, artifact_hash),
        signature,
    )
    .is_ok()
}

pub(crate) fn cmd_sign_app(args: Vec<String>) -> i32 {
    let Some(app_dir) = args.first().map(PathBuf::from) else {
        eprintln!("sign-app requires app dir, app slug, and developer seed");
        return 1;
    };
    let Some(app_slug) = args.get(1) else {
        eprintln!("sign-app requires app dir, app slug, and developer seed");
        return 1;
    };
    let Some(developer_seed_hex) = args.get(2) else {
        eprintln!("sign-app requires app dir, app slug, and developer seed");
        return 1;
    };
    let Some(developer_seed) = parse_seed(developer_seed_hex) else {
        eprintln!("developer seed must be 32 hex bytes");
        return 1;
    };
    let developer_key = SigningKey::from_bytes(&developer_seed);
    let developer_public = *developer_key.verifying_key().as_bytes();
    let app_id = app_id_for(app_slug, &developer_public);
    if let Err(err) = rebind_app_manifest_developer(&app_dir, app_slug, &developer_public, &app_id)
    {
        eprintln!("sign-app failed: {err}");
        return 1;
    }
    let eapp = match packaged_app_graph_bytes(&app_dir, app_slug, &developer_public, &app_id) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("sign-app failed: {err}");
            return 1;
        }
    };
    let eapp_path = app_dir.join("app.eapp");
    if let Err(err) = fs::write(&eapp_path, &eapp) {
        eprintln!("cannot write {}: {err}", eapp_path.display());
        return 1;
    }
    let developer_signature =
        signature_bytes_for_domain(&eapp, &developer_key, EAPP_DEVELOPER_DOMAIN);
    if let Err(err) = fs::write(app_dir.join("developer.esig"), developer_signature) {
        eprintln!("cannot write developer signature: {err}");
        return 1;
    }
    if let Some(store_seed_hex) = args.get(3) {
        let Some(store_seed) = parse_seed(store_seed_hex) else {
            eprintln!("store seed must be 32 hex bytes");
            return 1;
        };
        let store_key = SigningKey::from_bytes(&store_seed);
        let store_signature = signature_bytes_for_domain(&eapp, &store_key, EAPP_STORE_DOMAIN);
        if let Err(err) = fs::write(app_dir.join("store.esig"), store_signature) {
            eprintln!("cannot write store signature: {err}");
            return 1;
        }
        println!(
            "store_public_key: {}",
            bytes_to_hex(store_key.verifying_key().as_bytes())
        );
    }
    println!("app_id: {}", bytes_to_hex(&app_id));
    println!("release_id: {}", bytes_to_hex(&sha256(&eapp)));
    println!("developer_public_key: {}", bytes_to_hex(&developer_public));
    println!("artifact_graph: {}", eapp_path.display());
    println!(
        "developer_signature: {}",
        app_dir.join("developer.esig").display()
    );
    0
}

pub(crate) fn rebind_app_manifest_developer(
    app_dir: &Path,
    app_slug: &str,
    developer_public: &[u8; 32],
    app_id: &[u8; 32],
) -> Result<(), String> {
    let Ok(mut manifest) = read_app_manifest_record(app_dir) else {
        return Ok(());
    };
    if manifest.app_slug != app_slug.as_bytes() {
        return Err("app manifest slug does not match package slug".to_owned());
    }
    manifest.developer_id = *developer_public;
    manifest.app_id = *app_id;
    fs::write(
        app_dir.join("app.edapp"),
        sdk_wire_record_bytes(SdkWireRecord::AppManifest(manifest)),
    )
    .map_err(|err| err.to_string())
}

pub(crate) fn cmd_verify_signed_app(args: Vec<String>) -> i32 {
    let Some(app_dir) = args.first().map(PathBuf::from) else {
        eprintln!("verify-signed-app requires app dir");
        return 1;
    };
    let require_store = args.get(1).is_some_and(|value| value == "store");
    let eapp_path = app_dir.join("app.eapp");
    let Ok(eapp) = fs::read(&eapp_path) else {
        eprintln!("cannot read {}", eapp_path.display());
        return 1;
    };
    let Some(graph) = parse_app_graph_record(&eapp) else {
        eprintln!("invalid app graph: {}", eapp_path.display());
        return 1;
    };
    let graph_ok = verify_packaged_app_graph(&app_dir, &graph);
    let developer_ok = fs::read(app_dir.join("developer.esig"))
        .ok()
        .is_some_and(|bytes| {
            parse_artifact_signature_record(&bytes).is_some_and(|signature| {
                verify_signature_for_domain(&eapp, &signature, EAPP_DEVELOPER_DOMAIN)
                    && signature.public_key == graph.developer_public_key
            })
        });
    let store_ok = fs::read(app_dir.join("store.esig"))
        .ok()
        .is_some_and(|bytes| {
            parse_artifact_signature_record(&bytes).is_some_and(|signature| {
                verify_signature_for_domain(&eapp, &signature, EAPP_STORE_DOMAIN)
            })
        });
    println!("app: {}", String::from_utf8_lossy(&graph.app_slug));
    println!("app_id: {}", bytes_to_hex(&graph.app_id));
    println!("release_id: {}", bytes_to_hex(&sha256(&eapp)));
    print_check("artifact-graph", graph_ok);
    print_check("developer-signature", developer_ok);
    print_check("store-signature", store_ok || !require_store);
    if graph_ok && developer_ok && (store_ok || !require_store) {
        0
    } else {
        1
    }
}

pub(crate) fn cmd_write_app_store_catalog(args: Vec<String>) -> i32 {
    if args.len() != 5 {
        eprintln!(
            "write-app-store-catalog requires out.ecat, store seed, sequence, apps root, and catalog source json"
        );
        return 1;
    }
    let out = PathBuf::from(&args[0]);
    let Some(store_seed) = parse_seed(&args[1]) else {
        eprintln!("store seed must be 32 hex bytes");
        return 1;
    };
    let sequence = match parse_u64_arg(&args[2], "sequence") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let apps_root = PathBuf::from(&args[3]);
    let specs = match read_app_catalog_source(&PathBuf::from(&args[4])) {
        Ok(specs) => specs,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let store_key = SigningKey::from_bytes(&store_seed);
    let mut entries = Vec::new();
    for spec in &specs {
        match app_store_catalog_entry(&apps_root, &spec) {
            Ok(entry) => entries.push(entry),
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        }
    }
    entries.sort_by(|left, right| left.app_slug.cmp(&right.app_slug));
    let entries_len = entries.len();
    let mut catalog = edgerun_wire::AppStoreCatalogRecord {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        store_id: *store_key.verifying_key().as_bytes(),
        generated_at: 0,
        sequence,
        previous_catalog_sha256: [0; 32],
        entries,
        signature: Vec::new(),
    };
    let unsigned = sdk_wire_record_bytes(SdkWireRecord::AppStoreCatalog(catalog.clone()));
    let signature = store_key.sign(&signature_payload_for_domain(
        APP_STORE_CATALOG_DOMAIN,
        &sha256(&unsigned),
    ));
    catalog.signature = signature.to_bytes().to_vec();
    let bytes = sdk_wire_record_bytes(SdkWireRecord::AppStoreCatalog(catalog));
    if let Some(parent) = out.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        if let Err(err) = fs::create_dir_all(parent) {
            eprintln!("cannot create app store catalog directory: {err}");
            return 1;
        }
    }
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write app store catalog: {err}");
        return 1;
    }
    println!("catalog: {}", out.display());
    println!(
        "store_public_key: {}",
        bytes_to_hex(store_key.verifying_key().as_bytes())
    );
    println!("entries: {entries_len}");
    0
}

pub(crate) const APP_STORE_CATALOG_DOMAIN: &[u8] = b"edgerun-sdk.ecat.v1.store-catalog";

pub(crate) struct AppCatalogSpec {
    slug: String,
    required_capabilities: Vec<Vec<u8>>,
    optional_capabilities: Vec<Vec<u8>>,
}

fn read_app_catalog_source(path: &Path) -> Result<Vec<AppCatalogSpec>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("cannot read app catalog source {}: {err}", path.display()))?;
    let root = parse_json(&raw)
        .map_err(|err| format!("invalid app catalog source json {}: {err}", path.display()))?;
    let object = root
        .as_object()
        .ok_or_else(|| "app catalog source must be a JSON object".to_owned())?;
    let format = object
        .required_str("format")
        .map_err(|err| format!("invalid app catalog source format: {err}"))?;
    if format != "edgerun-app-catalog-source-v1" {
        return Err(format!("unsupported app catalog source format: {format}"));
    }
    let specs = object
        .required_array("apps")
        .map_err(|err| format!("invalid app catalog source apps: {err}"))?
        .iter()
        .enumerate()
        .map(|(index, value)| app_catalog_spec_from_json(index, value))
        .collect::<Result<Vec<_>, _>>()?;
    for (index, spec) in specs.iter().enumerate() {
        if specs[..index].iter().any(|seen| seen.slug == spec.slug) {
            return Err(format!("duplicate app catalog source slug: {}", spec.slug));
        }
    }
    Ok(specs)
}

fn app_catalog_spec_from_json(index: usize, value: &JsonValue) -> Result<AppCatalogSpec, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("app catalog source app {index} must be an object"))?;
    let slug = object
        .required_str("slug")
        .map_err(|err| format!("invalid app catalog source app {index} slug: {err}"))?;
    if slug.is_empty() {
        return Err(format!(
            "app catalog source app {index} slug cannot be empty"
        ));
    }
    if !app_catalog_slug_is_safe(slug) {
        return Err(format!(
            "app catalog source app {index} slug must contain only ASCII letters, digits, dot, dash, or underscore"
        ));
    }
    Ok(AppCatalogSpec {
        slug: slug.to_owned(),
        required_capabilities: catalog_capabilities_from_json(
            index,
            "requiredCapabilityIds",
            object.get_array("requiredCapabilityIds"),
        )?,
        optional_capabilities: catalog_capabilities_from_json(
            index,
            "optionalCapabilityIds",
            object.get_array("optionalCapabilityIds"),
        )?,
    })
}

fn catalog_capabilities_from_json(
    app_index: usize,
    field: &str,
    values: Option<&Vec<JsonValue>>,
) -> Result<Vec<Vec<u8>>, String> {
    values
        .into_iter()
        .flatten()
        .enumerate()
        .map(|(capability_index, value)| {
            let capability = value.as_str().ok_or_else(|| {
                format!("app catalog source app {app_index} {field}[{capability_index}] must be a string")
            })?;
            if capability.is_empty() {
                Err(format!(
                    "app catalog source app {app_index} {field}[{capability_index}] cannot be empty"
                ))
            } else {
                Ok(capability.as_bytes().to_vec())
            }
        })
        .collect()
}

fn app_catalog_slug_is_safe(slug: &str) -> bool {
    slug.bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

pub(crate) fn app_store_catalog_entry(
    apps_root: &Path,
    spec: &AppCatalogSpec,
) -> Result<edgerun_wire::AppStoreCatalogEntry, String> {
    let slug = &spec.slug;
    let app_dir = apps_root.join(slug);
    let (eapp, graph) = read_app_graph(&app_dir)?;
    let manifest_bytes = fs::read(app_dir.join("app.edapp"))
        .map_err(|err| format!("cannot read {}/app.edapp: {err}", app_dir.display()))?;
    let (name, version, summary) = app_catalog_text(&app_dir)?;
    let mut asset_refs = Vec::new();
    for artifact in collect_app_artifacts(&app_dir)? {
        let bytes = fs::read(app_dir.join(&artifact.path))
            .map_err(|err| format!("cannot read app asset {}: {err}", artifact.path))?;
        asset_refs.push(edgerun_wire::AppStoreAssetRef {
            path: artifact.path.into_bytes(),
            sha256: artifact.sha256,
            bytes: bytes.len() as u64,
        });
    }
    asset_refs.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(edgerun_wire::AppStoreCatalogEntry {
        app_id: graph.app_id,
        release_id: graph.runtime_install.release_id,
        developer_id: graph.developer_public_key,
        app_graph_sha256: sha256(&eapp),
        manifest_sha256: sha256(&manifest_bytes),
        package_sha256: sha256(&eapp),
        package_bytes: eapp.len() as u64,
        status: edgerun_wire::APP_STORE_SUBMISSION_STATUS_PUBLISHED,
        name: name.into_bytes(),
        version: version.into_bytes(),
        summary: summary.into_bytes(),
        app_slug: slug.as_bytes().to_vec(),
        package_ref: format!("/apps/{slug}/app.eapp").into_bytes(),
        manifest_ref: format!("/apps/{slug}/app.edapp").into_bytes(),
        launch_ref: format!("/apps/{slug}/index.html").into_bytes(),
        required_capabilities: spec.required_capabilities.clone(),
        optional_capabilities: spec.optional_capabilities.clone(),
        asset_refs,
    })
}

fn app_catalog_text(app_dir: &Path) -> Result<(String, String, String), String> {
    let manifest = read_app_manifest_record(app_dir)?;
    Ok((
        String::from_utf8_lossy(&manifest.name).into_owned(),
        String::from_utf8_lossy(&manifest.version).into_owned(),
        String::from_utf8_lossy(&manifest.summary).into_owned(),
    ))
}

pub(crate) fn write_packaged_app_graph(out_dir: &Path, app_slug: &str) -> Result<(), String> {
    let developer_public = [0u8; 32];
    let app_id = app_id_for(app_slug, &developer_public);
    let bytes = packaged_app_graph_bytes(out_dir, app_slug, &developer_public, &app_id)?;
    fs::write(out_dir.join("app.eapp"), bytes).map_err(|err| err.to_string())
}

pub(crate) fn packaged_app_graph_bytes(
    app_dir: &Path,
    app_slug: &str,
    developer_public: &[u8; 32],
    app_id: &[u8; 32],
) -> Result<Vec<u8>, String> {
    let artifacts = collect_app_artifacts(app_dir)?;
    let app_manifest = read_app_manifest_record(app_dir).ok();
    let app_manifest_sha256 = artifacts
        .iter()
        .find(|artifact| artifact.path == "app.edapp")
        .map(|artifact| artifact.sha256)
        .ok_or_else(|| "app.edapp missing from app package".to_owned())?;
    if let Some(manifest) = app_manifest.as_ref() {
        validate_app_manifest_binding(manifest, app_slug, developer_public, app_id)?;
    }
    let runtime_projection = runtime_projection_for_app_graph(
        app_slug,
        developer_public,
        app_id,
        app_manifest_sha256,
        &artifacts,
        app_manifest.as_ref(),
    );
    Ok(sdk_wire_record_bytes(SdkWireRecord::AppGraph(
        edgerun_wire::AppGraphRecord {
            abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
            flags: 1,
            app_id: *app_id,
            developer_public_key: *developer_public,
            app_manifest_sha256,
            app_slug: app_slug.as_bytes().to_vec(),
            runtime_install: runtime_projection,
            artifacts: artifacts
                .into_iter()
                .map(|artifact| edgerun_wire::AppArtifactRecord {
                    kind: artifact.kind,
                    path: artifact.path.into_bytes(),
                    sha256: artifact.sha256,
                })
                .collect(),
        },
    )))
}

pub(crate) fn runtime_projection_for_app_graph(
    app_slug: &str,
    developer_public: &[u8; 32],
    app_id: &[u8; 32],
    app_manifest_sha256: [u8; 32],
    artifacts: &[AppArtifact],
    app_manifest: Option<&edgerun_wire::AppManifestRecord>,
) -> edgerun_wire::RuntimeAppInstall {
    let release_id = app_release_id_for(app_id, app_manifest_sha256, artifacts);
    if let Some(manifest) = app_manifest {
        return runtime_projection_for_app_manifest(manifest, app_manifest_sha256, release_id);
    }
    if app_slug == "edgerun-wallet-app" {
        return runtime_api::runtime_app_projection(
            *app_id,
            release_id,
            sha256(b"edgerun-wallet-app:0.1.0"),
            *developer_public,
            app_manifest_sha256,
            Vec::new(),
            vec![b"edgerun-wallet-app/state".to_vec()],
        );
    }
    runtime_api::runtime_app_projection(
        *app_id,
        release_id,
        app_code_sha256_for(app_slug, artifacts),
        *developer_public,
        app_manifest_sha256,
        Vec::new(),
        Vec::new(),
    )
}

pub(crate) fn runtime_projection_for_app_manifest(
    manifest: &edgerun_wire::AppManifestRecord,
    manifest_sha256: [u8; 32],
    release_id: [u8; 32],
) -> edgerun_wire::RuntimeAppInstall {
    runtime_api::runtime_app_projection_from_manifest(manifest, manifest_sha256, release_id)
}

pub(crate) fn validate_app_manifest_binding(
    manifest: &edgerun_wire::AppManifestRecord,
    app_slug: &str,
    developer_public: &[u8; 32],
    app_id: &[u8; 32],
) -> Result<(), String> {
    if manifest.abi_version != edgerun_wire::SDK_WIRE_ABI_VERSION || manifest.flags & 1 != 1 {
        return Err("invalid app manifest ABI".to_owned());
    }
    if manifest.app_slug != app_slug.as_bytes() {
        return Err("app manifest slug does not match package slug".to_owned());
    }
    if &manifest.developer_id != developer_public {
        return Err("app manifest developer does not match package developer".to_owned());
    }
    if &manifest.app_id != app_id {
        return Err("app manifest app_id does not match package app_id".to_owned());
    }
    Ok(())
}

pub(crate) struct AppArtifact {
    pub(crate) kind: u16,
    pub(crate) path: String,
    pub(crate) sha256: [u8; 32],
}

pub(crate) fn collect_app_artifacts(app_dir: &Path) -> Result<Vec<AppArtifact>, String> {
    let mut files = Vec::new();
    collect_app_files(app_dir, app_dir, &mut files)?;
    files.sort();
    let mut artifacts = Vec::with_capacity(files.len());
    for path in files {
        let path_string = path.display().to_string().replace('\\', "/");
        if matches!(
            path_string.as_str(),
            "app.eapp" | "developer.esig" | "store.esig"
        ) {
            continue;
        }
        if path_string.ends_with(".etru")
            || path_string.ends_with(".eprd")
            || path_string.ends_with(".eent")
            || path_string.ends_with(".eset")
            || path_string.ends_with(".epay")
        {
            continue;
        }
        let bytes = fs::read(app_dir.join(&path))
            .map_err(|err| format!("cannot read {}: {err}", path.display()))?;
        artifacts.push(AppArtifact {
            kind: app_artifact_kind(&path_string),
            path: path_string,
            sha256: sha256(&bytes),
        });
    }
    Ok(artifacts)
}

pub(crate) fn collect_app_files(
    root: &Path,
    dir: &Path,
    files: &mut Vec<PathBuf>,
) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_app_files(root, &path, files)?;
        } else if path.is_file() {
            files.push(path_relative_to(root, &path));
        }
    }
    Ok(())
}

pub(crate) fn app_artifact_kind(path: &str) -> u16 {
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

pub(crate) fn verify_packaged_app_graph(
    app_dir: &Path,
    graph: &edgerun_wire::AppGraphRecord,
) -> bool {
    let Ok(slug) = core::str::from_utf8(&graph.app_slug) else {
        return false;
    };
    if graph.app_id != app_id_for(slug, &graph.developer_public_key) {
        return false;
    }
    let Ok(actual) = collect_app_artifacts(app_dir) else {
        return false;
    };
    if actual.len() != graph.artifacts.len() {
        return false;
    }
    if actual
        .iter()
        .find(|artifact| artifact.path == "app.edapp")
        .is_none_or(|artifact| artifact.sha256 != graph.app_manifest_sha256)
    {
        return false;
    }
    let app_manifest = read_app_manifest_record(app_dir).ok();
    if let Some(manifest) = app_manifest.as_ref() {
        if validate_app_manifest_binding(manifest, slug, &graph.developer_public_key, &graph.app_id)
            .is_err()
        {
            return false;
        }
    }
    if graph.runtime_install
        != runtime_projection_for_app_graph(
            slug,
            &graph.developer_public_key,
            &graph.app_id,
            graph.app_manifest_sha256,
            &actual,
            app_manifest.as_ref(),
        )
    {
        return false;
    }
    actual.iter().all(|expected| {
        graph.artifacts.iter().any(|actual| {
            actual.kind == expected.kind
                && actual.path == expected.path.as_bytes()
                && actual.sha256 == expected.sha256
        })
    })
}

pub(crate) fn app_id_for(app_slug: &str, developer_public: &[u8; 32]) -> [u8; 32] {
    let mut bytes =
        Vec::with_capacity(EAPP_DOMAIN_APP_ID.len() + developer_public.len() + app_slug.len());
    bytes.extend_from_slice(EAPP_DOMAIN_APP_ID);
    bytes.extend_from_slice(developer_public);
    bytes.extend_from_slice(app_slug.as_bytes());
    sha256(&bytes)
}

pub(crate) fn app_release_id_for(
    app_id: &[u8; 32],
    app_manifest_sha256: [u8; 32],
    artifacts: &[AppArtifact],
) -> [u8; 32] {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"edgerun-sdk.eapp.v1.release-id");
    bytes.extend_from_slice(app_id);
    bytes.extend_from_slice(&app_manifest_sha256);
    for artifact in artifacts {
        bytes.extend_from_slice(artifact.path.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&artifact.kind.to_le_bytes());
        bytes.extend_from_slice(&artifact.sha256);
    }
    sha256(&bytes)
}

pub(crate) fn app_code_sha256_for(app_slug: &str, artifacts: &[AppArtifact]) -> [u8; 32] {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"edgerun-sdk.eapp.v1.code");
    bytes.extend_from_slice(app_slug.as_bytes());
    for artifact in artifacts
        .iter()
        .filter(|artifact| matches!(artifact.kind, 2 | 3 | 4 | 5))
    {
        bytes.extend_from_slice(artifact.path.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&artifact.sha256);
    }
    sha256(&bytes)
}

pub(crate) fn parse_seed(value: &str) -> Option<[u8; 32]> {
    parse_hex(value)?.try_into().ok()
}

pub(crate) fn parse_seal_key(value: &str) -> Option<SealKey> {
    Some(SealKey::from_bytes(parse_hex(value)?.try_into().ok()?))
}

pub(crate) const USER_PROFILE_PASSWORD_KDF_DOMAIN: &[u8] =
    b"edgerun-sdk.eusr.v1.pbkdf2-hmac-sha256";
pub(crate) const USER_PROFILE_PASSWORD_SALT_LEN: usize = 16;
pub(crate) const USER_PROFILE_PASSWORD_DEFAULT_ROUNDS: u32 = 100_000;

pub(crate) fn user_profile_password_kdf_none() -> edgerun_wire::UserProfilePasswordKdf {
    edgerun_wire::UserProfilePasswordKdf {
        kdf: edgerun_wire::USER_PROFILE_KDF_NONE,
        flags: 0,
        rounds: 0,
        salt: Vec::new(),
    }
}

pub(crate) fn random_user_profile_password_kdf()
-> Result<edgerun_wire::UserProfilePasswordKdf, String> {
    let mut salt = [0u8; USER_PROFILE_PASSWORD_SALT_LEN];
    fill_random(&mut salt).map_err(|err| format!("salt generation failed: {err:?}"))?;
    Ok(edgerun_wire::UserProfilePasswordKdf {
        kdf: edgerun_wire::USER_PROFILE_KDF_PBKDF2_HMAC_SHA256,
        flags: 1,
        rounds: USER_PROFILE_PASSWORD_DEFAULT_ROUNDS,
        salt: salt.to_vec(),
    })
}

pub(crate) fn seal_key_from_user_profile_password(
    password: &str,
    kdf: &edgerun_wire::UserProfilePasswordKdf,
) -> Result<SealKey, String> {
    if password.is_empty() {
        return Err("password must not be empty".to_owned());
    }
    if kdf.kdf != edgerun_wire::USER_PROFILE_KDF_PBKDF2_HMAC_SHA256 || kdf.flags & 1 != 1 {
        return Err("unsupported user profile password KDF".to_owned());
    }
    if kdf.salt.len() < USER_PROFILE_PASSWORD_SALT_LEN || kdf.rounds == 0 {
        return Err("invalid user profile password KDF parameters".to_owned());
    }
    let mut salt_block = Vec::new();
    salt_block.extend_from_slice(USER_PROFILE_PASSWORD_KDF_DOMAIN);
    salt_block.extend_from_slice(&kdf.salt);
    salt_block.extend_from_slice(&1u32.to_be_bytes());
    let mut u = user_profile_hmac_sha256(password.as_bytes(), &salt_block);
    let mut out = [0u8; 32];
    out.copy_from_slice(&u);
    for _ in 1..kdf.rounds {
        u = user_profile_hmac_sha256(password.as_bytes(), &u);
        for (dst, src) in out.iter_mut().zip(u.iter()) {
            *dst ^= *src;
        }
    }
    Ok(SealKey::from_bytes(out))
}

pub(crate) fn user_profile_hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut normalized = [0u8; 64];
    if key.len() > 64 {
        normalized[..32].copy_from_slice(&sha256(key));
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for index in 0..64 {
        ipad[index] ^= normalized[index];
        opad[index] ^= normalized[index];
    }
    let mut inner = Vec::with_capacity(64 + data.len());
    inner.extend_from_slice(&ipad);
    inner.extend_from_slice(data);
    let inner_digest = sha256(&inner);
    let mut outer = Vec::with_capacity(64 + inner_digest.len());
    outer.extend_from_slice(&opad);
    outer.extend_from_slice(&inner_digest);
    sha256(&outer)
}

pub(crate) fn user_profile_id(owner_id: &[u8; 32], epoch: u64) -> [u8; 32] {
    let seed = UserProfileIdSeedRecord {
        domain: EUPB_DOMAIN.to_vec(),
        owner_id: *owner_id,
        epoch,
    };
    sha256(
        &edgerun_wire::to_bytes::<edgerun_wire::WireError>(&seed)
            .expect("user profile id seed must serialize through rkyv"),
    )
}

pub(crate) fn parse_scope_hash(value: &str) -> Option<[u8; 32]> {
    if value == "any" {
        return Some([0u8; 32]);
    }
    if let Some(context_hex) = value.strip_prefix("context:") {
        return Some(sha256(&parse_hex(context_hex)?));
    }
    hex_to_32(value).ok()
}

pub(crate) fn cmd_issue_product(args: Vec<String>) -> i32 {
    if args.len() < 11 {
        eprintln!(
            "issue-product requires app dir, out path, developer seed, store seed, product id, kind, currency, price, split bps, and validity"
        );
        return 1;
    }
    let app_dir = PathBuf::from(&args[0]);
    let out = PathBuf::from(&args[1]);
    let Some(developer_seed) = parse_seed(&args[2]) else {
        eprintln!("developer seed must be 32 hex bytes");
        return 1;
    };
    let Some(store_seed) = parse_seed(&args[3]) else {
        eprintln!("store seed must be 32 hex bytes");
        return 1;
    };
    let kind = match parse_entitlement_kind(&args[5]) {
        Some(value) => value,
        None => {
            eprintln!("bad product kind: {}", args[5]);
            return 1;
        }
    };
    let price_minor = match parse_u64_arg(&args[7], "price-minor") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let store_fee_bps = match parse_bps_arg(&args[8], "store-fee-bps") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let developer_share_bps = match parse_bps_arg(&args[9], "developer-share-bps") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    if store_fee_bps as u32 + developer_share_bps as u32 > 10_000 {
        eprintln!("store and developer bps exceed 10000");
        return 1;
    }
    let validity_seconds = match parse_u64_arg(&args[10], "validity-seconds") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let terms_sha256 = match args.get(11) {
        Some(value) => match hex_to_32(value) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("bad terms sha256: {err}");
                return 1;
            }
        },
        None => sha256(args[4].as_bytes()),
    };
    let (eapp, graph) = match read_app_graph(&app_dir) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("cannot read app graph: {err}");
            return 1;
        }
    };
    let developer_key = SigningKey::from_bytes(&developer_seed);
    let store_key = SigningKey::from_bytes(&store_seed);
    if developer_key.verifying_key().as_bytes() != &graph.developer_public_key {
        eprintln!("developer seed does not match app developer");
        return 1;
    }
    let bytes = product_bytes(ProductInput {
        kind,
        store_fee_bps,
        developer_share_bps,
        price_minor,
        validity_seconds,
        app_id: &graph.app_id,
        developer_id: &graph.developer_public_key,
        store_id: store_key.verifying_key().as_bytes(),
        release_id: &sha256(&eapp),
        terms_sha256: &terms_sha256,
        product_id: args[4].as_bytes(),
        currency: args[6].as_bytes(),
        developer_key: &developer_key,
        store_key: &store_key,
    });
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write product: {err}");
        return 1;
    }
    println!("product: {}", out.display());
    println!("product_id: {}", args[4]);
    println!("app_id: {}", bytes_to_hex(&graph.app_id));
    println!("release_id: {}", bytes_to_hex(&sha256(&eapp)));
    println!("price_minor: {price_minor}");
    println!("currency: {}", args[6]);
    0
}

pub(crate) fn cmd_verify_product(args: Vec<String>) -> i32 {
    if args.len() < 2 {
        eprintln!("verify-product requires app dir and product path");
        return 1;
    }
    let app_dir = PathBuf::from(&args[0]);
    let product_path = PathBuf::from(&args[1]);
    let Ok(bytes) = fs::read(&product_path) else {
        eprintln!("cannot read product: {}", product_path.display());
        return 1;
    };
    let Some(product) = parse_product_record(&bytes) else {
        eprintln!("invalid product: {}", product_path.display());
        return 1;
    };
    let (eapp, graph) = match read_app_graph(&app_dir) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("cannot read app graph: {err}");
            return 1;
        }
    };
    let app_ok = product.app_id == graph.app_id
        && product.developer_id == graph.developer_public_key
        && product.release_id == sha256(&eapp);
    let developer_ok = verify_product_developer_signature(&product);
    let store_ok = verify_product_store_signature(&product);
    println!("product: {}", product_path.display());
    println!(
        "product_id: {}",
        String::from_utf8_lossy(&product.product_id)
    );
    println!("kind: {}", product.kind);
    println!("price_minor: {}", product.price_minor);
    println!("currency: {}", String::from_utf8_lossy(&product.currency));
    println!("store_fee_bps: {}", product.store_fee_bps);
    println!("developer_share_bps: {}", product.developer_share_bps);
    print_check("app-binding", app_ok);
    print_check("developer-signature", developer_ok);
    print_check("store-signature", store_ok);
    if app_ok && developer_ok && store_ok {
        0
    } else {
        1
    }
}

pub(crate) fn cmd_issue_entitlement(args: Vec<String>) -> i32 {
    if args.len() < 11 {
        eprintln!(
            "issue-entitlement requires app dir, out path, store seed, subject, product, purchase, kind, validity, and split bps"
        );
        return 1;
    }
    let app_dir = PathBuf::from(&args[0]);
    let out = PathBuf::from(&args[1]);
    let Some(store_seed) = parse_seed(&args[2]) else {
        eprintln!("store seed must be 32 hex bytes");
        return 1;
    };
    let kind = match parse_entitlement_kind(&args[6]) {
        Some(value) => value,
        None => {
            eprintln!("bad entitlement kind: {}", args[6]);
            return 1;
        }
    };
    let valid_from = match args[7].parse::<u64>() {
        Ok(value) => value,
        Err(_) => {
            eprintln!("bad valid-from timestamp: {}", args[7]);
            return 1;
        }
    };
    let valid_until = match args[8].parse::<u64>() {
        Ok(value) => value,
        Err(_) => {
            eprintln!("bad valid-until timestamp: {}", args[8]);
            return 1;
        }
    };
    let store_fee_bps = match args[9].parse::<u16>() {
        Ok(value) if value <= 10_000 => value,
        _ => {
            eprintln!("bad store-fee-bps: {}", args[9]);
            return 1;
        }
    };
    let developer_share_bps = match args[10].parse::<u16>() {
        Ok(value) if value <= 10_000 => value,
        _ => {
            eprintln!("bad developer-share-bps: {}", args[10]);
            return 1;
        }
    };
    if store_fee_bps as u32 + developer_share_bps as u32 > 10_000 {
        eprintln!("store and developer bps exceed 10000");
        return 1;
    }
    let (eapp, graph) = match read_app_graph(&app_dir) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("cannot read app graph: {err}");
            return 1;
        }
    };
    let store_key = SigningKey::from_bytes(&store_seed);
    let product_binding = match args.get(11) {
        Some(value) if value.ends_with(".eprd") => {
            match read_verified_product(value, &graph, &eapp) {
                Ok(binding) => Some(binding),
                Err(err) => {
                    eprintln!("{err}");
                    return 1;
                }
            }
        }
        _ => None,
    };
    let terms_sha256 = if let Some(binding) = &product_binding {
        if binding.product_id.as_slice() != args[4].as_bytes()
            || binding.kind != kind
            || binding.store_fee_bps != store_fee_bps
            || binding.developer_share_bps != developer_share_bps
            || binding.store_id.as_slice() != store_key.verifying_key().as_bytes()
        {
            eprintln!("product does not match entitlement terms");
            return 1;
        }
        binding.terms_sha256
    } else {
        match args.get(11) {
            Some(value) => match hex_to_32(value) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("bad terms sha256: {err}");
                    return 1;
                }
            },
            None => sha256(args[4].as_bytes()),
        }
    };
    let product_sha256 = product_binding
        .as_ref()
        .map(|binding| binding.sha256)
        .unwrap_or([0u8; 32]);
    let bytes = entitlement_bytes(EntitlementInput {
        kind,
        store_fee_bps,
        developer_share_bps,
        valid_from,
        valid_until,
        app_id: &graph.app_id,
        developer_id: &graph.developer_public_key,
        store_id: store_key.verifying_key().as_bytes(),
        release_id: &sha256(&eapp),
        terms_sha256: &terms_sha256,
        product_sha256: &product_sha256,
        subject_id: args[3].as_bytes(),
        product_id: args[4].as_bytes(),
        purchase_id: args[5].as_bytes(),
        store_key: &store_key,
    });
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write entitlement: {err}");
        return 1;
    }
    println!("entitlement: {}", out.display());
    println!("app_id: {}", bytes_to_hex(&graph.app_id));
    println!("release_id: {}", bytes_to_hex(&sha256(&eapp)));
    println!(
        "store_public_key: {}",
        bytes_to_hex(store_key.verifying_key().as_bytes())
    );
    0
}

pub(crate) fn cmd_verify_entitlement(args: Vec<String>) -> i32 {
    if args.len() < 2 {
        eprintln!("verify-entitlement requires app dir and entitlement path");
        return 1;
    }
    let app_dir = PathBuf::from(&args[0]);
    let entitlement_path = PathBuf::from(&args[1]);
    let Ok(bytes) = fs::read(&entitlement_path) else {
        eprintln!("cannot read entitlement: {}", entitlement_path.display());
        return 1;
    };
    let Some(entitlement) = parse_entitlement_record(&bytes) else {
        eprintln!("invalid entitlement: {}", entitlement_path.display());
        return 1;
    };
    let (eapp, graph) = match read_app_graph(&app_dir) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("cannot read app graph: {err}");
            return 1;
        }
    };
    let app_ok = entitlement.app_id == graph.app_id
        && entitlement.developer_id == graph.developer_public_key
        && entitlement.release_id == sha256(&eapp);
    let filter_ok = args
        .get(2)
        .is_none_or(|subject| entitlement.subject_id.as_slice() == subject.as_bytes())
        && args
            .get(3)
            .is_none_or(|product| entitlement.product_id.as_slice() == product.as_bytes());
    let signature_ok = verify_entitlement_signature(&entitlement);
    println!("entitlement: {}", entitlement_path.display());
    println!(
        "product: {}",
        String::from_utf8_lossy(&entitlement.product_id)
    );
    println!(
        "subject: {}",
        String::from_utf8_lossy(&entitlement.subject_id)
    );
    println!(
        "purchase: {}",
        String::from_utf8_lossy(&entitlement.purchase_id)
    );
    println!("kind: {}", entitlement.kind);
    println!("valid_from: {}", entitlement.valid_from);
    println!("valid_until: {}", entitlement.valid_until);
    println!("store_fee_bps: {}", entitlement.store_fee_bps);
    println!("developer_share_bps: {}", entitlement.developer_share_bps);
    println!(
        "product_sha256: {}",
        bytes_to_hex(&entitlement.product_sha256)
    );
    print_check("app-binding", app_ok);
    print_check("requested-scope", filter_ok);
    print_check("store-signature", signature_ok);
    if app_ok && filter_ok && signature_ok {
        0
    } else {
        1
    }
}

pub(crate) fn read_app_graph(
    app_dir: &Path,
) -> Result<(Vec<u8>, edgerun_wire::AppGraphRecord), String> {
    let bytes = fs::read(app_dir.join("app.eapp")).map_err(|err| err.to_string())?;
    let graph = parse_app_graph_record(&bytes).ok_or_else(|| "invalid app.eapp".to_owned())?;
    Ok((bytes, graph))
}

pub(crate) fn parse_app_graph_record(bytes: &[u8]) -> Option<edgerun_wire::AppGraphRecord> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned).ok()? {
        SdkWireRecord::AppGraph(graph)
            if graph.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION && graph.flags & 1 == 1 =>
        {
            Some(graph)
        }
        _ => None,
    }
}

pub(crate) fn read_app_manifest_record(
    app_dir: &Path,
) -> Result<edgerun_wire::AppManifestRecord, String> {
    let bytes = fs::read(app_dir.join("app.edapp")).map_err(|err| err.to_string())?;
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned)
        .map_err(|_| "invalid app.edapp wire record".to_owned())?
    {
        SdkWireRecord::AppManifest(manifest)
            if manifest.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && manifest.flags & 1 == 1 =>
        {
            Ok(manifest)
        }
        _ => Err("app.edapp is not an app manifest record".to_owned()),
    }
}

pub(crate) fn parse_entitlement_record(bytes: &[u8]) -> Option<edgerun_wire::EntitlementRecord> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned).ok()? {
        SdkWireRecord::Entitlement(entitlement)
            if entitlement.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && entitlement.flags & 1 == 1 =>
        {
            Some(entitlement)
        }
        _ => None,
    }
}

pub(crate) fn parse_product_record(bytes: &[u8]) -> Option<edgerun_wire::ProductRecord> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned).ok()? {
        SdkWireRecord::Product(product)
            if product.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && product.flags & 1 == 1 =>
        {
            Some(product)
        }
        _ => None,
    }
}

pub(crate) fn parse_settlement_record(bytes: &[u8]) -> Option<edgerun_wire::SettlementRecord> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned).ok()? {
        SdkWireRecord::Settlement(settlement)
            if settlement.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && settlement.flags & 1 == 1 =>
        {
            Some(settlement)
        }
        _ => None,
    }
}

pub(crate) fn parse_payment_record(bytes: &[u8]) -> Option<edgerun_wire::PaymentRecord> {
    let owned = bytes.to_vec();
    let payment =
        match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned).ok()? {
            SdkWireRecord::Payment(payment)
                if payment.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                    && payment.flags & 1 == 1 =>
            {
                payment
            }
            _ => return None,
        };
    let settled = payment.flags & 2 == 2;
    if !settled && (!payment.store_id.is_empty() || !payment.store_signature.is_empty()) {
        return None;
    }
    if settled && payment.store_id.len() != 32 {
        return None;
    }
    Some(payment)
}

pub(crate) fn parse_unit_manifest_record(bytes: &[u8]) -> Option<edgerun_wire::UnitManifestRecord> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned).ok()? {
        SdkWireRecord::UnitManifest(manifest) => Some(manifest),
        _ => None,
    }
}

pub(crate) fn parse_entitlement_kind(value: &str) -> Option<u16> {
    match value {
        "license" => Some(1),
        "subscription" => Some(2),
        "consumable" => Some(3),
        "feature" => Some(4),
        _ => value.parse().ok(),
    }
}

pub(crate) struct EntitlementInput<'a> {
    pub(crate) kind: u16,
    pub(crate) store_fee_bps: u16,
    pub(crate) developer_share_bps: u16,
    pub(crate) valid_from: u64,
    pub(crate) valid_until: u64,
    pub(crate) app_id: &'a [u8],
    pub(crate) developer_id: &'a [u8],
    pub(crate) store_id: &'a [u8],
    pub(crate) release_id: &'a [u8; 32],
    pub(crate) terms_sha256: &'a [u8; 32],
    pub(crate) product_sha256: &'a [u8; 32],
    pub(crate) subject_id: &'a [u8],
    pub(crate) product_id: &'a [u8],
    pub(crate) purchase_id: &'a [u8],
    pub(crate) store_key: &'a SigningKey,
}

pub(crate) fn entitlement_bytes(input: EntitlementInput<'_>) -> Vec<u8> {
    let mut record = edgerun_wire::EntitlementRecord {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        kind: input.kind,
        store_fee_bps: input.store_fee_bps,
        developer_share_bps: input.developer_share_bps,
        valid_from: input.valid_from,
        valid_until: input.valid_until,
        app_id: input
            .app_id
            .try_into()
            .expect("entitlement app id must be 32 bytes"),
        developer_id: input
            .developer_id
            .try_into()
            .expect("entitlement developer id must be 32 bytes"),
        store_id: input
            .store_id
            .try_into()
            .expect("entitlement store id must be 32 bytes"),
        release_id: *input.release_id,
        terms_sha256: *input.terms_sha256,
        product_sha256: *input.product_sha256,
        subject_id: input.subject_id.to_vec(),
        product_id: input.product_id.to_vec(),
        purchase_id: input.purchase_id.to_vec(),
        signature: Vec::new(),
    };
    let signature = input.store_key.sign(&signature_payload_for_domain(
        EENT_STORE_DOMAIN,
        &sha256(&entitlement_unsigned_bytes(&record)),
    ));
    record.signature = signature.to_bytes().to_vec();
    sdk_wire_record_bytes(SdkWireRecord::Entitlement(record))
}

pub(crate) fn entitlement_unsigned_bytes(entitlement: &edgerun_wire::EntitlementRecord) -> Vec<u8> {
    let mut unsigned = entitlement.clone();
    unsigned.signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::Entitlement(unsigned))
}

pub(crate) fn verify_entitlement_signature(entitlement: &edgerun_wire::EntitlementRecord) -> bool {
    verify_ed25519_signature(
        &entitlement.store_id,
        EENT_STORE_DOMAIN,
        &sha256(&entitlement_unsigned_bytes(entitlement)),
        &entitlement.signature,
    )
}

pub(crate) struct ProductBinding {
    pub(crate) sha256: [u8; 32],
    pub(crate) kind: u16,
    pub(crate) store_fee_bps: u16,
    pub(crate) developer_share_bps: u16,
    pub(crate) product_id: Vec<u8>,
    pub(crate) store_id: [u8; 32],
    pub(crate) terms_sha256: [u8; 32],
}

pub(crate) fn read_verified_product(
    path: &str,
    graph: &edgerun_wire::AppGraphRecord,
    eapp: &[u8],
) -> Result<ProductBinding, String> {
    let bytes = fs::read(path).map_err(|err| format!("cannot read product: {path}: {err}"))?;
    let product = parse_product_record(&bytes).ok_or_else(|| format!("invalid product: {path}"))?;
    if product.app_id != graph.app_id
        || product.developer_id != graph.developer_public_key
        || product.release_id != sha256(eapp)
    {
        return Err(format!("product app binding failed: {path}"));
    }
    if !verify_product_developer_signature(&product) || !verify_product_store_signature(&product) {
        return Err(format!("product signature failed: {path}"));
    }
    Ok(ProductBinding {
        sha256: sha256(&bytes),
        kind: product.kind,
        store_fee_bps: product.store_fee_bps,
        developer_share_bps: product.developer_share_bps,
        product_id: product.product_id,
        store_id: product.store_id,
        terms_sha256: product.terms_sha256,
    })
}

pub(crate) fn cmd_issue_settlement(args: Vec<String>) -> i32 {
    if args.len() < 11 {
        eprintln!(
            "issue-settlement requires output, store seed, developer key, period, currency, amounts, and entitlements"
        );
        return 1;
    }
    let out = PathBuf::from(&args[0]);
    let Some(store_seed) = parse_seed(&args[1]) else {
        eprintln!("store seed must be 32 hex bytes");
        return 1;
    };
    let developer_id = match hex_to_32(&args[2]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("developer public key must be 32 hex bytes: {err}");
            return 1;
        }
    };
    let period_start = match parse_u64_arg(&args[3], "period-start") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let period_end = match parse_u64_arg(&args[4], "period-end") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let gross_minor = match parse_u64_arg(&args[6], "gross-minor") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let processor_fee_minor = match parse_u64_arg(&args[7], "processor-fee-minor") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let store_fee_minor = match parse_u64_arg(&args[8], "store-fee-minor") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let developer_net_minor = match parse_u64_arg(&args[9], "developer-net-minor") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    if processor_fee_minor
        .checked_add(store_fee_minor)
        .and_then(|value| value.checked_add(developer_net_minor))
        != Some(gross_minor)
    {
        eprintln!("gross must equal processor fee + store fee + developer net");
        return 1;
    }
    let store_key = SigningKey::from_bytes(&store_seed);
    let store_id = *store_key.verifying_key().as_bytes();
    let mut entitlement_hashes = Vec::new();
    for path in &args[10..] {
        let Ok(bytes) = fs::read(path) else {
            eprintln!("cannot read entitlement: {path}");
            return 1;
        };
        let Some(entitlement) = parse_entitlement_record(&bytes) else {
            eprintln!("invalid entitlement: {path}");
            return 1;
        };
        if !verify_entitlement_signature(&entitlement) {
            eprintln!("entitlement signature failed: {path}");
            return 1;
        }
        if entitlement.developer_id != developer_id || entitlement.store_id != store_id {
            eprintln!("entitlement developer/store mismatch: {path}");
            return 1;
        }
        entitlement_hashes.push(sha256(&bytes));
    }
    let bytes = settlement_bytes(SettlementInput {
        period_start,
        period_end,
        gross_minor,
        processor_fee_minor,
        store_fee_minor,
        developer_net_minor,
        developer_id: &developer_id,
        store_id: &store_id,
        currency: args[5].as_bytes(),
        entitlement_hashes: &entitlement_hashes,
        store_key: &store_key,
    });
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write settlement: {err}");
        return 1;
    }
    println!("settlement: {}", out.display());
    println!("developer_id: {}", bytes_to_hex(&developer_id));
    println!("store_id: {}", bytes_to_hex(&store_id));
    println!("entitlements: {}", entitlement_hashes.len());
    println!("gross_minor: {gross_minor}");
    println!("developer_net_minor: {developer_net_minor}");
    0
}

pub(crate) fn cmd_verify_settlement(args: Vec<String>) -> i32 {
    let Some(path) = args.first() else {
        eprintln!("verify-settlement requires settlement path");
        return 1;
    };
    let Ok(bytes) = fs::read(path) else {
        eprintln!("cannot read settlement: {path}");
        return 1;
    };
    let Some(settlement) = parse_settlement_record(&bytes) else {
        eprintln!("invalid settlement: {path}");
        return 1;
    };
    let signature_ok = verify_settlement_signature(&settlement);
    let total_ok = settlement
        .processor_fee_minor
        .checked_add(settlement.store_fee_minor)
        .and_then(|value| value.checked_add(settlement.developer_net_minor))
        == Some(settlement.gross_minor);
    let entitlement_ok = if args.len() > 1 {
        verify_settlement_entitlements(&settlement, &args[1..])
    } else {
        true
    };
    println!("settlement: {path}");
    println!(
        "currency: {}",
        String::from_utf8_lossy(&settlement.currency)
    );
    println!("period_start: {}", settlement.period_start);
    println!("period_end: {}", settlement.period_end);
    println!("gross_minor: {}", settlement.gross_minor);
    println!("processor_fee_minor: {}", settlement.processor_fee_minor);
    println!("store_fee_minor: {}", settlement.store_fee_minor);
    println!("developer_net_minor: {}", settlement.developer_net_minor);
    println!("entitlements: {}", settlement.entitlement_hashes.len());
    print_check("amounts-balance", total_ok);
    print_check("store-signature", signature_ok);
    print_check("entitlement-hashes", entitlement_ok);
    if total_ok && signature_ok && entitlement_ok {
        0
    } else {
        1
    }
}

pub(crate) fn parse_u64_arg(value: &str, name: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("bad {name}: {value}"))
}

pub(crate) fn parse_bps_arg(value: &str, name: &str) -> Result<u16, String> {
    match value.parse::<u16>() {
        Ok(value) if value <= 10_000 => Ok(value),
        _ => Err(format!("bad {name}: {value}")),
    }
}

pub(crate) struct ProductInput<'a> {
    pub(crate) kind: u16,
    pub(crate) store_fee_bps: u16,
    pub(crate) developer_share_bps: u16,
    pub(crate) price_minor: u64,
    pub(crate) validity_seconds: u64,
    pub(crate) app_id: &'a [u8],
    pub(crate) developer_id: &'a [u8],
    pub(crate) store_id: &'a [u8],
    pub(crate) release_id: &'a [u8; 32],
    pub(crate) terms_sha256: &'a [u8; 32],
    pub(crate) product_id: &'a [u8],
    pub(crate) currency: &'a [u8],
    pub(crate) developer_key: &'a SigningKey,
    pub(crate) store_key: &'a SigningKey,
}

pub(crate) fn product_bytes(input: ProductInput<'_>) -> Vec<u8> {
    let mut record = edgerun_wire::ProductRecord {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        kind: input.kind,
        store_fee_bps: input.store_fee_bps,
        developer_share_bps: input.developer_share_bps,
        price_minor: input.price_minor,
        validity_seconds: input.validity_seconds,
        app_id: input
            .app_id
            .try_into()
            .expect("product app id must be 32 bytes"),
        developer_id: input
            .developer_id
            .try_into()
            .expect("product developer id must be 32 bytes"),
        store_id: input
            .store_id
            .try_into()
            .expect("product store id must be 32 bytes"),
        release_id: *input.release_id,
        terms_sha256: *input.terms_sha256,
        product_id: input.product_id.to_vec(),
        currency: input.currency.to_vec(),
        developer_signature: Vec::new(),
        store_signature: Vec::new(),
    };
    let developer_signature = input.developer_key.sign(&signature_payload_for_domain(
        EPRD_DEVELOPER_DOMAIN,
        &sha256(&product_developer_unsigned_bytes(&record)),
    ));
    record.developer_signature = developer_signature.to_bytes().to_vec();
    let store_signature = input.store_key.sign(&signature_payload_for_domain(
        EPRD_STORE_DOMAIN,
        &sha256(&product_store_unsigned_bytes(&record)),
    ));
    record.store_signature = store_signature.to_bytes().to_vec();
    sdk_wire_record_bytes(SdkWireRecord::Product(record))
}

pub(crate) fn product_developer_unsigned_bytes(product: &edgerun_wire::ProductRecord) -> Vec<u8> {
    let mut unsigned = product.clone();
    unsigned.developer_signature.clear();
    unsigned.store_signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::Product(unsigned))
}

pub(crate) fn product_store_unsigned_bytes(product: &edgerun_wire::ProductRecord) -> Vec<u8> {
    let mut unsigned = product.clone();
    unsigned.store_signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::Product(unsigned))
}

pub(crate) fn verify_product_developer_signature(product: &edgerun_wire::ProductRecord) -> bool {
    verify_ed25519_signature(
        &product.developer_id,
        EPRD_DEVELOPER_DOMAIN,
        &sha256(&product_developer_unsigned_bytes(product)),
        &product.developer_signature,
    )
}

pub(crate) fn verify_product_store_signature(product: &edgerun_wire::ProductRecord) -> bool {
    verify_ed25519_signature(
        &product.store_id,
        EPRD_STORE_DOMAIN,
        &sha256(&product_store_unsigned_bytes(product)),
        &product.store_signature,
    )
}

pub(crate) struct SettlementInput<'a> {
    pub(crate) period_start: u64,
    pub(crate) period_end: u64,
    pub(crate) gross_minor: u64,
    pub(crate) processor_fee_minor: u64,
    pub(crate) store_fee_minor: u64,
    pub(crate) developer_net_minor: u64,
    pub(crate) developer_id: &'a [u8; 32],
    pub(crate) store_id: &'a [u8; 32],
    pub(crate) currency: &'a [u8],
    pub(crate) entitlement_hashes: &'a [[u8; 32]],
    pub(crate) store_key: &'a SigningKey,
}

pub(crate) fn settlement_bytes(input: SettlementInput<'_>) -> Vec<u8> {
    let mut record = edgerun_wire::SettlementRecord {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        period_start: input.period_start,
        period_end: input.period_end,
        gross_minor: input.gross_minor,
        processor_fee_minor: input.processor_fee_minor,
        store_fee_minor: input.store_fee_minor,
        developer_net_minor: input.developer_net_minor,
        developer_id: *input.developer_id,
        store_id: *input.store_id,
        currency: input.currency.to_vec(),
        entitlement_hashes: input.entitlement_hashes.to_vec(),
        signature: Vec::new(),
    };
    let signature = input.store_key.sign(&signature_payload_for_domain(
        ESET_STORE_DOMAIN,
        &sha256(&settlement_unsigned_bytes(&record)),
    ));
    record.signature = signature.to_bytes().to_vec();
    sdk_wire_record_bytes(SdkWireRecord::Settlement(record))
}

pub(crate) fn settlement_unsigned_bytes(settlement: &edgerun_wire::SettlementRecord) -> Vec<u8> {
    let mut unsigned = settlement.clone();
    unsigned.signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::Settlement(unsigned))
}

pub(crate) fn verify_settlement_signature(settlement: &edgerun_wire::SettlementRecord) -> bool {
    verify_ed25519_signature(
        &settlement.store_id,
        ESET_STORE_DOMAIN,
        &sha256(&settlement_unsigned_bytes(settlement)),
        &settlement.signature,
    )
}

pub(crate) fn verify_settlement_entitlements(
    settlement: &edgerun_wire::SettlementRecord,
    paths: &[String],
) -> bool {
    if paths.len() != settlement.entitlement_hashes.len() {
        return false;
    }
    let mut hashes = Vec::new();
    for path in paths {
        let Ok(bytes) = fs::read(path) else {
            return false;
        };
        let Some(entitlement) = parse_entitlement_record(&bytes) else {
            return false;
        };
        if entitlement.developer_id != settlement.developer_id
            || entitlement.store_id != settlement.store_id
            || !verify_entitlement_signature(&entitlement)
        {
            return false;
        }
        hashes.push(sha256(&bytes));
    }
    hashes.sort();
    let mut expected = settlement.entitlement_hashes.clone();
    expected.sort();
    hashes == expected
}

pub(crate) fn cmd_issue_payment_intent(args: Vec<String>) -> i32 {
    if args.len() < 8 {
        eprintln!(
            "issue-payment-intent requires app dir, output, payer seed, payee id, purpose, amount, currency, and created-at"
        );
        return 1;
    }
    let app_dir = PathBuf::from(&args[0]);
    let out = PathBuf::from(&args[1]);
    let Some(payer_seed) = parse_seed(&args[2]) else {
        eprintln!("payer seed must be 32 hex bytes");
        return 1;
    };
    let payee_id = match hex_to_32(&args[3]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("payee id must be 32 hex bytes: {err}");
            return 1;
        }
    };
    let purpose = match parse_payment_purpose(&args[4]) {
        Some(value) => value,
        None => {
            eprintln!("bad payment purpose: {}", args[4]);
            return 1;
        }
    };
    let amount_minor = match parse_u64_arg(&args[5], "amount-minor") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let created_at = match parse_u64_arg(&args[7], "created-at") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let context_sha256 = match args.get(8) {
        Some(value) => match hex_to_32(value) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("bad context sha256: {err}");
                return 1;
            }
        },
        None => [0u8; 32],
    };
    let (eapp, graph) = match read_app_graph(&app_dir) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("cannot read app graph: {err}");
            return 1;
        }
    };
    let payer_key = SigningKey::from_bytes(&payer_seed);
    let payer_id = *payer_key.verifying_key().as_bytes();
    let bytes = payment_intent_bytes(PaymentIntentInput {
        purpose,
        amount_minor,
        created_at,
        app_id: &graph.app_id,
        release_id: &sha256(&eapp),
        payer_id: &payer_id,
        payee_id: &payee_id,
        context_sha256: &context_sha256,
        currency: args[6].as_bytes(),
        payer_key: &payer_key,
    });
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write payment: {err}");
        return 1;
    }
    println!("payment_intent: {}", out.display());
    println!("payer_id: {}", bytes_to_hex(&payer_id));
    println!("payee_id: {}", bytes_to_hex(&payee_id));
    println!("amount_minor: {amount_minor}");
    println!("currency: {}", args[6]);
    0
}

pub(crate) fn cmd_settle_payment(args: Vec<String>) -> i32 {
    if args.len() < 6 {
        eprintln!(
            "settle-payment requires intent, output, store seed, rail kind, rail ref hash, and settled-at"
        );
        return 1;
    }
    let intent_path = PathBuf::from(&args[0]);
    let out = PathBuf::from(&args[1]);
    let Ok(intent_bytes) = fs::read(&intent_path) else {
        eprintln!("cannot read payment intent: {}", intent_path.display());
        return 1;
    };
    let Some(intent) = parse_payment_record(&intent_bytes) else {
        eprintln!("invalid payment intent: {}", intent_path.display());
        return 1;
    };
    if !verify_payment_payer_signature(&intent) {
        eprintln!("payer signature failed");
        return 1;
    }
    let Some(store_seed) = parse_seed(&args[2]) else {
        eprintln!("store seed must be 32 hex bytes");
        return 1;
    };
    let rail_ref_sha256 = match hex_to_32(&args[4]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad rail ref sha256: {err}");
            return 1;
        }
    };
    let settled_at = match parse_u64_arg(&args[5], "settled-at") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let store_key = SigningKey::from_bytes(&store_seed);
    let bytes = settle_payment_bytes(
        &intent,
        PaymentSettlementInput {
            settled_at,
            store_id: store_key.verifying_key().as_bytes(),
            rail_ref_sha256: &rail_ref_sha256,
            rail_kind: args[3].as_bytes(),
            store_key: &store_key,
        },
    );
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write settled payment: {err}");
        return 1;
    }
    println!("payment: {}", out.display());
    println!(
        "store_id: {}",
        bytes_to_hex(store_key.verifying_key().as_bytes())
    );
    println!("rail_kind: {}", args[3]);
    println!("rail_ref_sha256: {}", bytes_to_hex(&rail_ref_sha256));
    0
}

pub(crate) fn cmd_verify_payment(args: Vec<String>) -> i32 {
    if args.len() < 2 {
        eprintln!("verify-payment requires app dir and payment path");
        return 1;
    }
    let app_dir = PathBuf::from(&args[0]);
    let path = PathBuf::from(&args[1]);
    let require_settled = args.get(2).is_some_and(|value| value == "settled");
    let Ok(bytes) = fs::read(&path) else {
        eprintln!("cannot read payment: {}", path.display());
        return 1;
    };
    let Some(payment) = parse_payment_record(&bytes) else {
        eprintln!("invalid payment: {}", path.display());
        return 1;
    };
    let (eapp, graph) = match read_app_graph(&app_dir) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("cannot read app graph: {err}");
            return 1;
        }
    };
    let app_ok = payment.app_id == graph.app_id && payment.release_id == sha256(&eapp);
    let payer_ok = verify_payment_payer_signature(&payment);
    let store_ok = !payment.store_signature.is_empty() && verify_payment_store_signature(&payment);
    println!("payment: {}", path.display());
    println!("purpose: {}", payment.purpose);
    println!("amount_minor: {}", payment.amount_minor);
    println!("currency: {}", String::from_utf8_lossy(&payment.currency));
    println!("payer_id: {}", bytes_to_hex(&payment.payer_id));
    println!("payee_id: {}", bytes_to_hex(&payment.payee_id));
    println!("settled_at: {}", payment.settled_at);
    println!("rail_kind: {}", String::from_utf8_lossy(&payment.rail_kind));
    println!(
        "rail_ref_sha256: {}",
        bytes_to_hex(&payment.rail_ref_sha256)
    );
    print_check("app-binding", app_ok);
    print_check("payer-signature", payer_ok);
    print_check("store-settlement", store_ok || !require_settled);
    if app_ok && payer_ok && (store_ok || !require_settled) {
        0
    } else {
        1
    }
}

pub(crate) fn cmd_write_trust_policy(args: Vec<String>) -> i32 {
    if args.len() < 3 {
        eprintln!("write-trust-policy requires output, role, and public key");
        return 1;
    }
    let out = PathBuf::from(&args[0]);
    let role = match parse_trust_role(&args[1]) {
        Some(value) => value,
        None => {
            eprintln!("bad trust role: {}", args[1]);
            return 1;
        }
    };
    let public_key = match hex_to_32(&args[2]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad public key: {err}");
            return 1;
        }
    };
    let app_id = match args.get(3).map(String::as_str) {
        Some("-") | None => [0u8; 32],
        Some(value) => match hex_to_32(value) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("bad app id: {err}");
                return 1;
            }
        },
    };
    let developer_id = match args.get(4).map(String::as_str) {
        Some("-") | None => [0u8; 32],
        Some(value) => match hex_to_32(value) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("bad developer id: {err}");
                return 1;
            }
        },
    };
    let valid_from = match args.get(5) {
        Some(value) => match parse_u64_arg(value, "valid-from") {
            Ok(value) => value,
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        },
        None => 0,
    };
    let valid_until = match args.get(6) {
        Some(value) => match parse_u64_arg(value, "valid-until") {
            Ok(value) => value,
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        },
        None => u64::MAX,
    };
    let mut entries = Vec::new();
    if out.exists() {
        let Some(existing) = read_trust_policy(out.to_string_lossy().as_ref()) else {
            eprintln!("existing trust policy is invalid: {}", out.display());
            return 1;
        };
        for entry in existing.entries {
            entries.push(OwnedTrustEntry {
                role: entry.role,
                valid_from: entry.valid_from,
                valid_until: entry.valid_until,
                public_key: entry.public_key,
                app_id: entry.app_id,
                developer_id: entry.developer_id,
            });
        }
    }
    entries.push(OwnedTrustEntry {
        role,
        valid_from,
        valid_until,
        public_key,
        app_id,
        developer_id,
    });
    let bytes = trust_policy_bytes(&entries);
    if let Err(err) = fs::write(&out, bytes) {
        eprintln!("cannot write trust policy: {err}");
        return 1;
    }
    println!("trust_policy: {}", out.display());
    println!("role: {}", args[1]);
    println!("public_key: {}", bytes_to_hex(&public_key));
    0
}

pub(crate) fn cmd_verify_trusted_app(args: Vec<String>) -> i32 {
    if args.len() < 2 {
        eprintln!("verify-trusted-app requires app dir and policy");
        return 1;
    }
    let app_dir = PathBuf::from(&args[0]);
    let Some(policy) = read_trust_policy(&args[1]) else {
        eprintln!("invalid trust policy: {}", args[1]);
        return 1;
    };
    let require_store = args.iter().skip(2).any(|value| value == "store");
    let revocations =
        read_revocations(args.iter().skip(2).filter(|value| value.ends_with(".erev")));
    let eapp_path = app_dir.join("app.eapp");
    let Ok(eapp) = fs::read(&eapp_path) else {
        eprintln!("cannot read {}", eapp_path.display());
        return 1;
    };
    let Some(graph) = parse_app_graph_record(&eapp) else {
        eprintln!("invalid app graph: {}", eapp_path.display());
        return 1;
    };
    let developer_signature = fs::read(app_dir.join("developer.esig")).ok();
    let developer_ok = developer_signature.as_ref().is_some_and(|bytes| {
        parse_artifact_signature_record(bytes).is_some_and(|signature| {
            verify_signature_for_domain(&eapp, &signature, EAPP_DEVELOPER_DOMAIN)
                && signature.public_key == graph.developer_public_key
        })
    });
    let store_signature = fs::read(app_dir.join("store.esig")).ok();
    let store_key = store_signature.as_ref().and_then(|bytes| {
        parse_artifact_signature_record(bytes).and_then(|signature| {
            verify_signature_for_domain(&eapp, &signature, EAPP_STORE_DOMAIN)
                .then_some(signature.public_key)
        })
    });
    let graph_ok = verify_packaged_app_graph(&app_dir, &graph);
    let trusted_developer = trust_allows(
        &policy,
        TRUST_ROLE_DEVELOPER,
        &graph.developer_public_key,
        &graph.app_id,
        &graph.developer_public_key,
        0,
    );
    let trusted_store = store_key.as_ref().is_some_and(|key| {
        trust_allows(
            &policy,
            TRUST_ROLE_APP_STORE,
            key,
            &graph.app_id,
            &graph.developer_public_key,
            0,
        )
    });
    let not_revoked = !revocations.revoke_app(
        &sha256(&eapp),
        &policy,
        &graph.app_id,
        &graph.developer_public_key,
    ) && !revocations.revoke_key(
        &graph.developer_public_key,
        &policy,
        &graph.app_id,
        &graph.developer_public_key,
    ) && !store_key.as_ref().is_some_and(|key| {
        revocations.revoke_key(key, &policy, &graph.app_id, &graph.developer_public_key)
    });
    print_check("artifact-graph", graph_ok);
    print_check("developer-signature", developer_ok);
    print_check("trusted-developer", trusted_developer);
    print_check("trusted-store", trusted_store || !require_store);
    print_check("not-revoked", not_revoked);
    if graph_ok
        && developer_ok
        && trusted_developer
        && (trusted_store || !require_store)
        && not_revoked
    {
        0
    } else {
        1
    }
}

pub(crate) fn cmd_verify_trusted_product(args: Vec<String>) -> i32 {
    if args.len() < 3 {
        eprintln!("verify-trusted-product requires app dir, product, and policy");
        return 1;
    }
    let app_dir = PathBuf::from(&args[0]);
    let Ok(bytes) = fs::read(&args[1]) else {
        eprintln!("cannot read product: {}", args[1]);
        return 1;
    };
    let Some(product) = parse_product_record(&bytes) else {
        eprintln!("invalid product: {}", args[1]);
        return 1;
    };
    let Some(policy) = read_trust_policy(&args[2]) else {
        eprintln!("invalid trust policy: {}", args[2]);
        return 1;
    };
    let revocations =
        read_revocations(args.iter().skip(3).filter(|value| value.ends_with(".erev")));
    let (eapp, graph) = match read_app_graph(&app_dir) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("cannot read app graph: {err}");
            return 1;
        }
    };
    let app_ok = product.app_id == graph.app_id
        && product.developer_id == graph.developer_public_key
        && product.release_id == sha256(&eapp);
    let developer_ok = verify_product_developer_signature(&product);
    let store_ok = verify_product_store_signature(&product);
    let trusted_developer = trust_allows(
        &policy,
        TRUST_ROLE_DEVELOPER,
        &product.developer_id,
        &product.app_id,
        &product.developer_id,
        0,
    );
    let trusted_store = trust_allows(
        &policy,
        TRUST_ROLE_PRODUCT_STORE,
        &product.store_id,
        &product.app_id,
        &product.developer_id,
        0,
    );
    let not_revoked = !revocations.revoke_product(
        &sha256(&bytes),
        &policy,
        &product.app_id,
        &product.developer_id,
    ) && !revocations.revoke_key(
        &product.developer_id,
        &policy,
        &product.app_id,
        &product.developer_id,
    ) && !revocations.revoke_key(
        &product.store_id,
        &policy,
        &product.app_id,
        &product.developer_id,
    );
    print_check("app-binding", app_ok);
    print_check("developer-signature", developer_ok);
    print_check("store-signature", store_ok);
    print_check("trusted-developer", trusted_developer);
    print_check("trusted-store", trusted_store);
    print_check("not-revoked", not_revoked);
    if app_ok && developer_ok && store_ok && trusted_developer && trusted_store && not_revoked {
        0
    } else {
        1
    }
}

pub(crate) fn cmd_verify_trusted_entitlement(args: Vec<String>) -> i32 {
    if args.len() < 3 {
        eprintln!("verify-trusted-entitlement requires app dir, entitlement, and policy");
        return 1;
    }
    let app_dir = PathBuf::from(&args[0]);
    let Ok(bytes) = fs::read(&args[1]) else {
        eprintln!("cannot read entitlement: {}", args[1]);
        return 1;
    };
    let Some(entitlement) = parse_entitlement_record(&bytes) else {
        eprintln!("invalid entitlement: {}", args[1]);
        return 1;
    };
    let Some(policy) = read_trust_policy(&args[2]) else {
        eprintln!("invalid trust policy: {}", args[2]);
        return 1;
    };
    let revocations =
        read_revocations(args.iter().skip(3).filter(|value| value.ends_with(".erev")));
    let (eapp, graph) = match read_app_graph(&app_dir) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("cannot read app graph: {err}");
            return 1;
        }
    };
    let app_ok = entitlement.app_id == graph.app_id
        && entitlement.developer_id == graph.developer_public_key
        && entitlement.release_id == sha256(&eapp);
    let signature_ok = verify_entitlement_signature(&entitlement);
    let trusted_store = trust_allows(
        &policy,
        TRUST_ROLE_ENTITLEMENT_STORE,
        &entitlement.store_id,
        &entitlement.app_id,
        &entitlement.developer_id,
        entitlement.valid_from,
    );
    let not_revoked = !revocations.revoke_entitlement(
        &sha256(&bytes),
        &policy,
        &entitlement.app_id,
        &entitlement.developer_id,
    ) && !revocations.revoke_key(
        &entitlement.store_id,
        &policy,
        &entitlement.app_id,
        &entitlement.developer_id,
    );
    print_check("app-binding", app_ok);
    print_check("store-signature", signature_ok);
    print_check("trusted-store", trusted_store);
    print_check("not-revoked", not_revoked);
    if app_ok && signature_ok && trusted_store && not_revoked {
        0
    } else {
        1
    }
}

pub(crate) fn cmd_verify_trusted_payment(args: Vec<String>) -> i32 {
    if args.len() < 3 {
        eprintln!("verify-trusted-payment requires app dir, payment, and policy");
        return 1;
    }
    let app_dir = PathBuf::from(&args[0]);
    let Ok(bytes) = fs::read(&args[1]) else {
        eprintln!("cannot read payment: {}", args[1]);
        return 1;
    };
    let Some(payment) = parse_payment_record(&bytes) else {
        eprintln!("invalid payment: {}", args[1]);
        return 1;
    };
    let Some(policy) = read_trust_policy(&args[2]) else {
        eprintln!("invalid trust policy: {}", args[2]);
        return 1;
    };
    let require_settled = args.iter().skip(3).any(|value| value == "settled");
    let revocations =
        read_revocations(args.iter().skip(3).filter(|value| value.ends_with(".erev")));
    let (eapp, graph) = match read_app_graph(&app_dir) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("cannot read app graph: {err}");
            return 1;
        }
    };
    let app_ok = payment.app_id == graph.app_id && payment.release_id == sha256(&eapp);
    let payer_ok = verify_payment_payer_signature(&payment);
    let payer_trusted = trust_allows(
        &policy,
        TRUST_ROLE_PAYER,
        &payment.payer_id,
        &payment.app_id,
        &[],
        payment.created_at,
    );
    let store_ok = !payment.store_signature.is_empty() && verify_payment_store_signature(&payment);
    let store_trusted = (!payment.store_id.is_empty())
        .then(|| {
            trust_allows(
                &policy,
                TRUST_ROLE_PAYMENT_STORE,
                &payment.store_id,
                &payment.app_id,
                &[],
                payment.settled_at,
            )
        })
        .unwrap_or(false);
    let not_revoked = !revocations.revoke_payment(&sha256(&bytes), &policy, &payment.app_id, &[])
        && !revocations.revoke_key(&payment.payer_id, &policy, &payment.app_id, &[])
        && (payment.store_id.is_empty()
            || !revocations.revoke_key(&payment.store_id, &policy, &payment.app_id, &[]));
    print_check("app-binding", app_ok);
    print_check("payer-signature", payer_ok);
    print_check("trusted-payer", payer_trusted);
    print_check("store-settlement", store_ok || !require_settled);
    print_check("trusted-store", store_trusted || !require_settled);
    print_check("not-revoked", not_revoked);
    if app_ok
        && payer_ok
        && payer_trusted
        && (store_ok || !require_settled)
        && (store_trusted || !require_settled)
        && not_revoked
    {
        0
    } else {
        1
    }
}

pub(crate) fn cmd_verify_trusted_settlement(args: Vec<String>) -> i32 {
    if args.len() < 2 {
        eprintln!("verify-trusted-settlement requires settlement and policy");
        return 1;
    }
    let Ok(bytes) = fs::read(&args[0]) else {
        eprintln!("cannot read settlement: {}", args[0]);
        return 1;
    };
    let Some(settlement) = parse_settlement_record(&bytes) else {
        eprintln!("invalid settlement: {}", args[0]);
        return 1;
    };
    let Some(policy) = read_trust_policy(&args[1]) else {
        eprintln!("invalid trust policy: {}", args[1]);
        return 1;
    };
    let revocations =
        read_revocations(args.iter().skip(2).filter(|value| value.ends_with(".erev")));
    let signature_ok = verify_settlement_signature(&settlement);
    let total_ok = settlement
        .processor_fee_minor
        .checked_add(settlement.store_fee_minor)
        .and_then(|value| value.checked_add(settlement.developer_net_minor))
        == Some(settlement.gross_minor);
    let trusted_store = trust_allows(
        &policy,
        TRUST_ROLE_SETTLEMENT_STORE,
        &settlement.store_id,
        &[],
        &settlement.developer_id,
        settlement.period_start,
    );
    let entitlement_paths: Vec<String> = args
        .iter()
        .skip(2)
        .filter(|value| !value.ends_with(".erev"))
        .cloned()
        .collect();
    let entitlement_ok = if !entitlement_paths.is_empty() {
        verify_settlement_entitlements(&settlement, &entitlement_paths)
    } else {
        true
    };
    let not_revoked =
        !revocations.revoke_settlement(&sha256(&bytes), &policy, &[], &settlement.developer_id)
            && !revocations.revoke_key(
                &settlement.store_id,
                &policy,
                &[],
                &settlement.developer_id,
            );
    print_check("amounts-balance", total_ok);
    print_check("store-signature", signature_ok);
    print_check("trusted-store", trusted_store);
    print_check("entitlement-hashes", entitlement_ok);
    print_check("not-revoked", not_revoked);
    if total_ok && signature_ok && trusted_store && entitlement_ok && not_revoked {
        0
    } else {
        1
    }
}

pub(crate) fn parse_payment_purpose(value: &str) -> Option<u16> {
    match value {
        "purchase" => Some(1),
        "tip" => Some(2),
        "reward" => Some(3),
        "refund" => Some(4),
        "payout" => Some(5),
        "bounty" => Some(6),
        "revenue_share" => Some(7),
        "escrow_deposit" => Some(8),
        "escrow_release" => Some(9),
        _ => value.parse().ok(),
    }
}

pub(crate) const TRUST_ROLE_DEVELOPER: u16 = 1;
pub(crate) const TRUST_ROLE_APP_STORE: u16 = 2;
pub(crate) const TRUST_ROLE_PRODUCT_STORE: u16 = 3;
pub(crate) const TRUST_ROLE_ENTITLEMENT_STORE: u16 = 4;
pub(crate) const TRUST_ROLE_PAYMENT_STORE: u16 = 5;
pub(crate) const TRUST_ROLE_SETTLEMENT_STORE: u16 = 6;
pub(crate) const TRUST_ROLE_PAYER: u16 = 7;
pub(crate) const TRUST_ROLE_PAYEE: u16 = 8;

pub(crate) fn parse_trust_role(value: &str) -> Option<u16> {
    match value {
        "developer" => Some(TRUST_ROLE_DEVELOPER),
        "app_store" | "store" => Some(TRUST_ROLE_APP_STORE),
        "product_store" => Some(TRUST_ROLE_PRODUCT_STORE),
        "entitlement_store" => Some(TRUST_ROLE_ENTITLEMENT_STORE),
        "payment_store" => Some(TRUST_ROLE_PAYMENT_STORE),
        "settlement_store" => Some(TRUST_ROLE_SETTLEMENT_STORE),
        "payer" => Some(TRUST_ROLE_PAYER),
        "payee" => Some(TRUST_ROLE_PAYEE),
        _ => value.parse().ok(),
    }
}

pub(crate) struct OwnedTrustEntry {
    pub(crate) role: u16,
    pub(crate) valid_from: u64,
    pub(crate) valid_until: u64,
    pub(crate) public_key: [u8; 32],
    pub(crate) app_id: [u8; 32],
    pub(crate) developer_id: [u8; 32],
}

pub(crate) fn trust_policy_bytes(entries: &[OwnedTrustEntry]) -> Vec<u8> {
    sdk_wire_record_bytes(SdkWireRecord::TrustPolicy(edgerun_wire::TrustPolicy {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        entries: entries
            .iter()
            .map(|entry| edgerun_wire::TrustEntry {
                role: entry.role,
                flags: 0,
                valid_from: entry.valid_from,
                valid_until: entry.valid_until,
                public_key: entry.public_key,
                app_id: entry.app_id,
                developer_id: entry.developer_id,
            })
            .collect(),
    }))
}

pub(crate) fn read_trust_policy(path: &str) -> Option<edgerun_wire::TrustPolicy> {
    let bytes = fs::read(path).ok()?;
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned).ok()? {
        SdkWireRecord::TrustPolicy(policy)
            if policy.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && policy.flags & 1 == 1 =>
        {
            Some(policy)
        }
        _ => None,
    }
}

pub(crate) fn trust_allows(
    policy: &edgerun_wire::TrustPolicy,
    role: u16,
    public_key: &[u8],
    app_id: &[u8],
    developer_id: &[u8],
    at: u64,
) -> bool {
    policy.entries.iter().any(|entry| {
        entry.role == role
            && entry.public_key.as_slice() == public_key
            && (entry.valid_from == 0 || at >= entry.valid_from)
            && (entry.valid_until == u64::MAX || at <= entry.valid_until)
            && (entry.app_id == [0u8; 32] || entry.app_id.as_slice() == app_id)
            && (entry.developer_id == [0u8; 32] || entry.developer_id.as_slice() == developer_id)
    })
}

#[derive(Clone)]
pub(crate) struct OwnedUserGrant {
    pub(crate) app_id: [u8; 32],
    pub(crate) release_id: [u8; 32],
    pub(crate) scope_sha256: [u8; 32],
    pub(crate) constraints_sha256: [u8; 32],
    pub(crate) capability_kind: u16,
    pub(crate) operation: u16,
    pub(crate) min_assurance: u16,
    pub(crate) flags: u16,
    pub(crate) valid_from: u64,
    pub(crate) valid_until: u64,
    pub(crate) user_signature: Vec<u8>,
}

impl From<&edgerun_wire::RuntimeCapabilityGrant> for OwnedUserGrant {
    fn from(grant: &edgerun_wire::RuntimeCapabilityGrant) -> Self {
        Self {
            app_id: grant.app_id,
            release_id: grant.release_id,
            scope_sha256: grant.scope_sha256,
            constraints_sha256: grant.constraints_sha256,
            capability_kind: grant.capability_kind,
            operation: grant.operation,
            min_assurance: grant.min_assurance,
            flags: grant.flags as u16,
            valid_from: grant.valid_from,
            valid_until: grant.valid_until,
            user_signature: grant.user_signature.clone(),
        }
    }
}

pub(crate) fn user_profile_body_bytes(
    profile_id: &[u8; 32],
    owner_key: &SigningKey,
    epoch: u64,
    monotonic_version: u64,
    grants: &[OwnedUserGrant],
) -> Vec<u8> {
    wire_user_profile_body_bytes(profile_id, owner_key, epoch, monotonic_version, grants)
}

pub(crate) fn user_profile_body_with_owner_seed_bytes(
    profile_id: &[u8; 32],
    owner_seed: &[u8; 32],
    epoch: u64,
    monotonic_version: u64,
    grants: &[OwnedUserGrant],
) -> Vec<u8> {
    let owner_key = SigningKey::from_bytes(owner_seed);
    wire_user_profile_body_with_owner_seed_bytes(
        profile_id,
        &owner_key,
        Some(owner_seed),
        epoch,
        monotonic_version,
        grants,
    )
}

pub(crate) fn user_profile_file_bytes(body: &[u8], seal_key: &SealKey) -> Result<Vec<u8>, String> {
    wire_user_profile_file_bytes(body, seal_key)
}

pub(crate) fn user_profile_file_bytes_with_password(
    body: &[u8],
    password: &str,
) -> Result<Vec<u8>, String> {
    let kdf = random_user_profile_password_kdf()?;
    let seal_key = seal_key_from_user_profile_password(password, &kdf)?;
    wire_user_profile_file_bytes_with_kdf(body, &seal_key, kdf)
}

pub(crate) fn open_user_profile_file(
    bytes: &[u8],
    seal_key: &SealKey,
) -> Result<edgerun_wire::UserProfileBody, String> {
    open_wire_user_profile_file(bytes, seal_key)
}

pub(crate) fn open_user_profile_file_with_password(
    bytes: &[u8],
    password: &str,
) -> Result<edgerun_wire::UserProfileBody, String> {
    let profile = parse_wire_user_profile(bytes)?;
    let seal_key = seal_key_from_user_profile_password(password, &profile.password_kdf)?;
    open_wire_user_profile_with_record(&profile, &seal_key)
}

pub(crate) fn verify_user_profile_body_signature(profile: &edgerun_wire::UserProfileBody) -> bool {
    verify_wire_user_profile_body_signature(profile)
}

pub(crate) fn sdk_wire_record_bytes(record: SdkWireRecord) -> Vec<u8> {
    sdk_wire_bytes(&record)
}

pub(crate) fn read_sdk_wire_record(path: &str) -> Result<SdkWireRecord, String> {
    let bytes = fs::read(path).map_err(|err| format!("cannot read {path}: {err}"))?;
    let owned = bytes.to_vec();
    edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned)
        .map_err(|err| format!("invalid SDK wire record {path}: {err:?}"))
}

pub(crate) fn parse_capability_request_record(
    bytes: &[u8],
) -> Option<edgerun_wire::CapabilityRequest> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned).ok()? {
        SdkWireRecord::CapabilityRequest(request)
            if request.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && request.flags & 1 == 1 =>
        {
            Some(request)
        }
        _ => None,
    }
}

pub(crate) fn parse_capability_response_record(
    bytes: &[u8],
) -> Option<edgerun_wire::CapabilityResponse> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned).ok()? {
        SdkWireRecord::CapabilityResponse(response)
            if response.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && response.flags & 1 == 1 =>
        {
            Some(response)
        }
        _ => None,
    }
}

pub(crate) fn wire_capability_response_binding_ok(
    request_bytes: &[u8],
    request: &edgerun_wire::CapabilityRequest,
    response: &edgerun_wire::CapabilityResponse,
) -> bool {
    response.request_sha256 == sha256(request_bytes)
        && response.capability_kind == request.capability_kind
        && response.operation == request.operation
        && response.assurance >= request.assurance
}

pub(crate) fn wire_user_grant_from_owned(
    profile_id: &[u8; 32],
    owner_id: &[u8; 32],
    grant: &OwnedUserGrant,
) -> edgerun_wire::RuntimeCapabilityGrant {
    let grant_id = edgerun_wire::runtime_capability_grant_id(
        *profile_id,
        *owner_id,
        grant.app_id,
        grant.release_id,
        grant.capability_kind,
        grant.operation,
        grant.scope_sha256,
        grant.constraints_sha256,
        grant.valid_from,
        grant.valid_until,
    );
    edgerun_wire::RuntimeCapabilityGrant {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: grant.flags as u32,
        grant_id,
        profile_id: *profile_id,
        user_id: *owner_id,
        app_id: grant.app_id,
        release_id: grant.release_id,
        capability_kind: grant.capability_kind,
        operation: grant.operation,
        min_assurance: grant.min_assurance,
        scope_sha256: grant.scope_sha256,
        constraints_sha256: grant.constraints_sha256,
        valid_from: grant.valid_from,
        valid_until: grant.valid_until,
        user_signature: grant.user_signature.clone(),
    }
}

pub(crate) fn wire_user_profile_body_unsigned_bytes(
    body: &edgerun_wire::UserProfileBody,
) -> Vec<u8> {
    let mut unsigned = body.clone();
    unsigned.signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::UserProfileBody(unsigned))
}

pub(crate) fn wire_user_profile_body_bytes(
    profile_id: &[u8; 32],
    owner_key: &SigningKey,
    epoch: u64,
    monotonic_version: u64,
    grants: &[OwnedUserGrant],
) -> Vec<u8> {
    wire_user_profile_body_with_owner_seed_bytes(
        profile_id,
        owner_key,
        None,
        epoch,
        monotonic_version,
        grants,
    )
}

pub(crate) fn wire_user_profile_body_with_owner_seed_bytes(
    profile_id: &[u8; 32],
    owner_key: &SigningKey,
    owner_seed: Option<&[u8; 32]>,
    epoch: u64,
    monotonic_version: u64,
    grants: &[OwnedUserGrant],
) -> Vec<u8> {
    let owner_id = *owner_key.verifying_key().as_bytes();
    let mut body = edgerun_wire::UserProfileBody {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        epoch,
        monotonic_version,
        profile_id: *profile_id,
        owner_id,
        owner_key_algorithm: if owner_seed.is_some() {
            edgerun_wire::USER_PROFILE_OWNER_KEY_ED25519
        } else {
            0
        },
        owner_private_key: owner_seed.map_or_else(Vec::new, |seed| seed.to_vec()),
        grants: grants
            .iter()
            .map(|grant| wire_user_grant_from_owned(profile_id, &owner_id, grant))
            .collect(),
        signature: Vec::new(),
    };
    let unsigned = wire_user_profile_body_unsigned_bytes(&body);
    let signature = owner_key.sign(&signature_payload_for_domain(
        EUPB_DOMAIN,
        &sha256(&unsigned),
    ));
    body.signature = signature.to_bytes().to_vec();
    sdk_wire_record_bytes(SdkWireRecord::UserProfileBody(body))
}

pub(crate) fn wire_user_profile_file_bytes(
    body_bytes: &[u8],
    seal_key: &SealKey,
) -> Result<Vec<u8>, String> {
    wire_user_profile_file_bytes_with_kdf(body_bytes, seal_key, user_profile_password_kdf_none())
}

pub(crate) fn wire_user_profile_file_bytes_with_kdf(
    body_bytes: &[u8],
    seal_key: &SealKey,
    password_kdf: edgerun_wire::UserProfilePasswordKdf,
) -> Result<Vec<u8>, String> {
    let owned = body_bytes.to_vec();
    let body = match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned)
        .map_err(|err| format!("invalid wire profile body: {err:?}"))?
    {
        SdkWireRecord::UserProfileBody(body) => body,
        _ => return Err("wire body is not a user profile body".to_owned()),
    };
    if !verify_wire_user_profile_body_signature(&body) {
        return Err("wire profile body signature failed".to_owned());
    }
    let sealed =
        seal_with_key(body_bytes, seal_key).map_err(|err| format!("seal failed: {err:?}"))?;
    let profile = edgerun_wire::UserProfile {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        monotonic_version: body.monotonic_version,
        profile_id: body.profile_id,
        owner_id: body.owner_id,
        password_kdf,
        body_sha256: sha256(body_bytes),
        sealed_body: sealed,
    };
    Ok(sdk_wire_record_bytes(SdkWireRecord::UserProfile(profile)))
}

pub(crate) fn open_wire_user_profile_file(
    bytes: &[u8],
    seal_key: &SealKey,
) -> Result<edgerun_wire::UserProfileBody, String> {
    let profile = parse_wire_user_profile(bytes)?;
    open_wire_user_profile_with_record(&profile, seal_key)
}

pub(crate) fn parse_wire_user_profile(bytes: &[u8]) -> Result<edgerun_wire::UserProfile, String> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned)
        .map_err(|err| format!("invalid wire user profile: {err:?}"))?
    {
        SdkWireRecord::UserProfile(profile) => Ok(profile),
        _ => Err("wire record is not a user profile".to_owned()),
    }
}

pub(crate) fn open_wire_user_profile_with_record(
    profile: &edgerun_wire::UserProfile,
    seal_key: &SealKey,
) -> Result<edgerun_wire::UserProfileBody, String> {
    let body_bytes = unseal_with_key(&profile.sealed_body, seal_key)
        .map_err(|err| format!("unseal failed: {err:?}"))?;
    let body_owned = body_bytes.to_vec();
    let body = match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&body_owned)
        .map_err(|err| format!("invalid wire profile body: {err:?}"))?
    {
        SdkWireRecord::UserProfileBody(body) => body,
        _ => return Err("wire sealed body is not a user profile body".to_owned()),
    };
    if body.profile_id != profile.profile_id
        || body.owner_id != profile.owner_id
        || sha256(&body_bytes) != profile.body_sha256
        || body.monotonic_version != profile.monotonic_version
    {
        return Err("wire profile header does not match decrypted body".to_owned());
    }
    if !verify_wire_user_profile_body_signature(&body) {
        return Err("wire profile body signature failed".to_owned());
    }
    Ok(body)
}

pub(crate) fn verify_wire_user_profile_body_signature(
    profile: &edgerun_wire::UserProfileBody,
) -> bool {
    if !profile.owner_private_key.is_empty() {
        if profile.owner_key_algorithm != edgerun_wire::USER_PROFILE_OWNER_KEY_ED25519 {
            return false;
        }
        let Ok(owner_seed) = <[u8; 32]>::try_from(profile.owner_private_key.as_slice()) else {
            return false;
        };
        let owner_key = SigningKey::from_bytes(&owner_seed);
        if owner_key.verifying_key().as_bytes() != &profile.owner_id {
            return false;
        }
    }
    let unsigned = wire_user_profile_body_unsigned_bytes(profile);
    verify_ed25519_signature(
        &profile.owner_id,
        EUPB_DOMAIN,
        &sha256(&unsigned),
        &profile.signature,
    )
}

pub(crate) fn user_profile_allows_request(
    profile: &edgerun_wire::UserProfileBody,
    request: &edgerun_wire::CapabilityRequest,
    at: u64,
) -> bool {
    wire_user_profile_allows_request(profile, request, at)
}

pub(crate) fn wire_user_profile_allows_request(
    profile: &edgerun_wire::UserProfileBody,
    request: &edgerun_wire::CapabilityRequest,
    at: u64,
) -> bool {
    profile.grants.iter().any(|grant| {
        grant.app_id == request.app_id
            && (grant.release_id == [0u8; 32] || grant.release_id == request.release_id)
            && grant.capability_kind == request.capability_kind
            && grant.operation == request.operation
            && request.assurance >= grant.min_assurance
            && (grant.valid_from == 0 || at >= grant.valid_from)
            && (grant.valid_until == u64::MAX || at <= grant.valid_until)
            && (grant.scope_sha256 == [0u8; 32] || grant.scope_sha256 == sha256(&request.context))
    })
}

pub(crate) const REV_KIND_KEY: u16 = 1;
pub(crate) const REV_KIND_APP: u16 = 2;
pub(crate) const REV_KIND_PRODUCT: u16 = 3;
pub(crate) const REV_KIND_ENTITLEMENT: u16 = 4;
pub(crate) const REV_KIND_PAYMENT: u16 = 5;
pub(crate) const REV_KIND_SETTLEMENT: u16 = 6;
pub(crate) const CAPABILITY_KIND_SIGNING: u16 = 1;
pub(crate) const CAPABILITY_KIND_SEALING: u16 = 2;
pub(crate) const CAPABILITY_KIND_STORAGE: u16 = 4;
pub(crate) const CAPABILITY_OPERATION_SIGN: u16 = 1;
pub(crate) const CAPABILITY_OPERATION_SEAL: u16 = 3;
pub(crate) const CAPABILITY_OPERATION_UNSEAL: u16 = 4;
pub(crate) const CAPABILITY_OPERATION_READ: u16 = 6;
pub(crate) const CAPABILITY_OPERATION_WRITE: u16 = 7;
pub(crate) const CAPABILITY_STATUS_OK: u16 = 0;
pub(crate) const CAPABILITY_STATUS_POLICY_DENIED: u16 = 1;
pub(crate) const CAPABILITY_STATUS_INVALID_REQUEST: u16 = 2;
pub(crate) const CAPABILITY_STATUS_PROVIDER_FAILED: u16 = 3;
pub(crate) const SIGN_ALGORITHM_ED25519: u16 = 1;

pub(crate) fn parse_capability_kind(value: &str) -> Option<u16> {
    match value {
        "sign" | "signing" => Some(CAPABILITY_KIND_SIGNING),
        "seal" | "sealing" => Some(CAPABILITY_KIND_SEALING),
        "payment" | "payments" => Some(3),
        "storage" => Some(CAPABILITY_KIND_STORAGE),
        "network" => Some(5),
        _ => value.parse().ok(),
    }
}

pub(crate) fn parse_capability_operation(value: &str) -> Option<u16> {
    match value {
        "sign" => Some(CAPABILITY_OPERATION_SIGN),
        "verify" => Some(2),
        "seal" => Some(CAPABILITY_OPERATION_SEAL),
        "unseal" => Some(CAPABILITY_OPERATION_UNSEAL),
        "authorize" => Some(5),
        "read" => Some(CAPABILITY_OPERATION_READ),
        "write" => Some(CAPABILITY_OPERATION_WRITE),
        "send" => Some(8),
        "receive" => Some(9),
        _ => value.parse().ok(),
    }
}

pub(crate) fn parse_sign_algorithm(value: &str) -> Option<u16> {
    match value {
        "ed25519" => Some(SIGN_ALGORITHM_ED25519),
        _ => value.parse().ok(),
    }
}

pub(crate) fn parse_u16_arg(value: &str, name: &str) -> Result<u16, String> {
    value
        .parse()
        .map_err(|_| format!("{name} must be an unsigned 16-bit integer"))
}

pub(crate) fn cmd_write_revocation(args: Vec<String>) -> i32 {
    if args.len() < 5 {
        eprintln!(
            "write-revocation requires output, kind, target hash/key, issuer seed, and issued-at"
        );
        return 1;
    }
    let out = PathBuf::from(&args[0]);
    let kind = match parse_revocation_kind(&args[1]) {
        Some(value) => value,
        None => {
            eprintln!("bad revocation kind: {}", args[1]);
            return 1;
        }
    };
    let target = match hex_to_32(&args[2]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad revocation target: {err}");
            return 1;
        }
    };
    let Some(issuer_seed) = parse_seed(&args[3]) else {
        eprintln!("issuer seed must be 32 hex bytes");
        return 1;
    };
    let issued_at = match parse_u64_arg(&args[4], "issued-at") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let reason = args.get(5).map(String::as_bytes).unwrap_or(&[]);
    let issuer = SigningKey::from_bytes(&issuer_seed);
    let bytes = revocation_bytes(RevocationInput {
        kind,
        issued_at,
        target: &target,
        reason,
        issuer_key: &issuer,
    });
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write revocation: {err}");
        return 1;
    }
    println!("revocation: {}", out.display());
    println!("kind: {}", args[1]);
    println!("target: {}", bytes_to_hex(&target));
    println!(
        "issuer: {}",
        bytes_to_hex(issuer.verifying_key().as_bytes())
    );
    0
}

pub(crate) fn cmd_verify_revocation(args: Vec<String>) -> i32 {
    let Some(path) = args.first() else {
        eprintln!("verify-revocation requires revocation path");
        return 1;
    };
    let Ok(bytes) = fs::read(path) else {
        eprintln!("cannot read revocation: {path}");
        return 1;
    };
    let Some(revocation) = parse_revocation_record(&bytes) else {
        eprintln!("invalid revocation: {path}");
        return 1;
    };
    let signature_ok = verify_revocation_signature(&revocation);
    println!("revocation: {path}");
    println!("kind: {}", revocation.kind);
    println!("target: {}", bytes_to_hex(&revocation.target));
    println!("issuer: {}", bytes_to_hex(&revocation.issuer));
    print_check("issuer-signature", signature_ok);
    if signature_ok { 0 } else { 1 }
}

pub(crate) fn cmd_write_sign_request(args: Vec<String>) -> i32 {
    if args.len() < 9 {
        eprintln!(
            "write-sign-request requires out.rkyv, domain, app id, release id, subject hash, payload hash, algorithm, assurance, and nonce hex"
        );
        return 1;
    }
    let out = PathBuf::from(&args[0]);
    let app_id = match hex_to_32(&args[2]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad app id: {err}");
            return 1;
        }
    };
    let release_id = match hex_to_32(&args[3]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad release id: {err}");
            return 1;
        }
    };
    let subject_sha256 = match hex_to_32(&args[4]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad subject hash: {err}");
            return 1;
        }
    };
    let payload_sha256 = match hex_to_32(&args[5]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad payload hash: {err}");
            return 1;
        }
    };
    let algorithm = match parse_sign_algorithm(&args[6]) {
        Some(value) => value,
        None => {
            eprintln!("bad signing algorithm: {}", args[6]);
            return 1;
        }
    };
    let assurance = match parse_u16_arg(&args[7], "assurance") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let nonce = match parse_hex(&args[8]) {
        Some(value) => value,
        None => {
            eprintln!("nonce must be hex bytes");
            return 1;
        }
    };
    let bytes = sign_request_bytes(SignRequestInput {
        domain: args[1].as_bytes(),
        app_id: &app_id,
        release_id: &release_id,
        subject_sha256: &subject_sha256,
        payload_sha256: &payload_sha256,
        algorithm,
        assurance,
        nonce: &nonce,
    });
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write signing request: {err}");
        return 1;
    }
    println!("sign_request: {}", out.display());
    println!("request_sha256: {}", bytes_to_hex(&sha256(&bytes)));
    0
}

pub(crate) fn cmd_sign_request(args: Vec<String>) -> i32 {
    if args.len() < 4 {
        eprintln!("sign-request requires request.rkyv, out.rkyv, provider, and signer seed");
        return 1;
    }
    execute_sign_request(&args, None)
}

pub(crate) fn cmd_sign_request_authorized(args: Vec<String>) -> i32 {
    if args.len() < 7 {
        eprintln!(
            "sign-request-authorized requires profile.eusr, profile key hex, request.rkyv, out.rkyv, provider, signer seed, at, and optional assurance"
        );
        return 1;
    }
    let mut execution_args = vec![
        args[2].clone(),
        args[3].clone(),
        args[4].clone(),
        args[5].clone(),
    ];
    let Some(at) = parse_authorized_at(&args[6]) else {
        return 1;
    };
    if let Some(assurance) = args.get(7) {
        execution_args.push(assurance.clone());
    }
    execute_sign_request(
        &execution_args,
        Some(ProfileAuthorizationInput {
            profile_path: &args[0],
            profile_key_hex: &args[1],
            at,
        }),
    )
}

pub(crate) fn execute_sign_request(
    args: &[String],
    authorization: Option<ProfileAuthorizationInput<'_>>,
) -> i32 {
    let request_path = PathBuf::from(&args[0]);
    let out = PathBuf::from(&args[1]);
    let Ok(request_bytes) = fs::read(&request_path) else {
        eprintln!("cannot read request: {}", request_path.display());
        return 1;
    };
    let Some(capability_request) = parse_capability_request_record(&request_bytes) else {
        eprintln!("invalid capability request: {}", request_path.display());
        return 1;
    };
    let Some(algorithm) = signing_request_algorithm(&capability_request) else {
        eprintln!("invalid signing request: {}", request_path.display());
        return 1;
    };
    if algorithm != SIGN_ALGORITHM_ED25519 {
        eprintln!("CLI signer only supports ed25519 requests");
        return 1;
    }
    if let Some(authorization) = authorization {
        let authorized = match profile_authorizes_request(&authorization, &capability_request) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        };
        print_check("profile-authorization", authorized);
        if !authorized {
            return write_capability_denial_response(
                &out,
                &request_bytes,
                &capability_request,
                args[2].as_bytes(),
                CAPABILITY_STATUS_POLICY_DENIED,
                b"policy_denied",
            );
        }
    }
    let Some(seed) = parse_seed(&args[3]) else {
        eprintln!("signer seed must be 32 hex bytes");
        return 1;
    };
    let assurance = match args.get(4) {
        Some(value) => match parse_u16_arg(value, "assurance") {
            Ok(value) => value,
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        },
        None => capability_request.assurance,
    };
    let key = SigningKey::from_bytes(&seed);
    let bytes = sign_response_bytes(SignResponseInput {
        request_bytes: &request_bytes,
        provider: args[2].as_bytes(),
        algorithm: SIGN_ALGORITHM_ED25519,
        assurance,
        signer: key.verifying_key().as_bytes(),
        signing_key: &key,
    });
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write signing response: {err}");
        return 1;
    }
    println!("sign_response: {}", out.display());
    println!("request_sha256: {}", bytes_to_hex(&sha256(&request_bytes)));
    println!("signer: {}", bytes_to_hex(key.verifying_key().as_bytes()));
    0
}

pub(crate) fn cmd_verify_sign_response(args: Vec<String>) -> i32 {
    if args.len() < 2 {
        eprintln!("verify-sign-response requires request.rkyv and response.rkyv");
        return 1;
    }
    let Ok(request_bytes) = fs::read(&args[0]) else {
        eprintln!("cannot read request: {}", args[0]);
        return 1;
    };
    let Some(request) = parse_capability_request_record(&request_bytes) else {
        eprintln!("invalid signing request: {}", args[0]);
        return 1;
    };
    let Some(request_algorithm) = signing_request_algorithm(&request) else {
        eprintln!("invalid signing request: {}", args[0]);
        return 1;
    };
    let Ok(response_bytes) = fs::read(&args[1]) else {
        eprintln!("cannot read response: {}", args[1]);
        return 1;
    };
    let Some(response) = parse_capability_response_record(&response_bytes) else {
        eprintln!("invalid signing response: {}", args[1]);
        return 1;
    };
    let Some(response_algorithm) = signing_response_algorithm(&response) else {
        eprintln!("invalid signing response: {}", args[1]);
        return 1;
    };
    let binding_ok = response.request_sha256 == sha256(&request_bytes)
        && response_algorithm == request_algorithm
        && response.assurance >= request.assurance;
    let signature_ok = verify_sign_response_signature(&request_bytes, &response);
    println!("sign_request: {}", args[0]);
    println!("sign_response: {}", args[1]);
    println!("domain: {}", String::from_utf8_lossy(&request.context));
    println!("provider: {}", String::from_utf8_lossy(&response.provider));
    print_check("request-binding", binding_ok);
    print_check("runtime-signature", signature_ok);
    if binding_ok && signature_ok { 0 } else { 1 }
}

pub(crate) fn parse_revocation_kind(value: &str) -> Option<u16> {
    match value {
        "key" => Some(REV_KIND_KEY),
        "app" | "release" => Some(REV_KIND_APP),
        "product" => Some(REV_KIND_PRODUCT),
        "entitlement" => Some(REV_KIND_ENTITLEMENT),
        "payment" => Some(REV_KIND_PAYMENT),
        "settlement" => Some(REV_KIND_SETTLEMENT),
        _ => value.parse().ok(),
    }
}

pub(crate) struct RevocationInput<'a> {
    pub(crate) kind: u16,
    pub(crate) issued_at: u64,
    pub(crate) target: &'a [u8; 32],
    pub(crate) reason: &'a [u8],
    pub(crate) issuer_key: &'a SigningKey,
}

pub(crate) fn revocation_bytes(input: RevocationInput<'_>) -> Vec<u8> {
    let mut revocation = edgerun_wire::Revocation {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        kind: input.kind,
        issued_at: input.issued_at,
        target: *input.target,
        issuer: *input.issuer_key.verifying_key().as_bytes(),
        reason: input.reason.to_vec(),
        signature: Vec::new(),
    };
    let body = revocation_unsigned_bytes(&revocation);
    let signature = input
        .issuer_key
        .sign(&signature_payload_for_domain(EREV_DOMAIN, &sha256(&body)));
    revocation.signature = signature.to_bytes().to_vec();
    sdk_wire_record_bytes(SdkWireRecord::Revocation(revocation))
}

pub(crate) fn revocation_unsigned_bytes(revocation: &edgerun_wire::Revocation) -> Vec<u8> {
    let mut unsigned = revocation.clone();
    unsigned.signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::Revocation(unsigned))
}

pub(crate) fn parse_revocation_record(bytes: &[u8]) -> Option<edgerun_wire::Revocation> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned).ok()? {
        SdkWireRecord::Revocation(revocation)
            if revocation.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && revocation.flags & 1 == 1 =>
        {
            Some(revocation)
        }
        _ => None,
    }
}

pub(crate) fn verify_revocation_signature(revocation: &edgerun_wire::Revocation) -> bool {
    let body = revocation_unsigned_bytes(revocation);
    verify_ed25519_signature(
        &revocation.issuer,
        EREV_DOMAIN,
        &sha256(&body),
        &revocation.signature,
    )
}

pub(crate) fn cmd_write_capability_request(args: Vec<String>) -> i32 {
    if args.len() < 11 {
        eprintln!(
            "write-capability-request requires out.rkyv, kind, operation, assurance, app id, release id, subject hash, payload hash, context hex, payload hex, and nonce hex"
        );
        return 1;
    }
    let out = PathBuf::from(&args[0]);
    let kind = match parse_capability_kind(&args[1]) {
        Some(value) => value,
        None => {
            eprintln!("bad capability kind: {}", args[1]);
            return 1;
        }
    };
    let operation = match parse_capability_operation(&args[2]) {
        Some(value) => value,
        None => {
            eprintln!("bad capability operation: {}", args[2]);
            return 1;
        }
    };
    let assurance = match parse_u16_arg(&args[3], "assurance") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let app_id = match hex_to_32(&args[4]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad app id: {err}");
            return 1;
        }
    };
    let release_id = match hex_to_32(&args[5]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad release id: {err}");
            return 1;
        }
    };
    let subject_sha256 = match hex_to_32(&args[6]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad subject hash: {err}");
            return 1;
        }
    };
    let payload_sha256 = match hex_to_32(&args[7]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad payload hash: {err}");
            return 1;
        }
    };
    let context = match parse_hex(&args[8]) {
        Some(value) => value,
        None => {
            eprintln!("context must be hex bytes");
            return 1;
        }
    };
    let payload = match parse_hex(&args[9]) {
        Some(value) => value,
        None => {
            eprintln!("payload must be hex bytes");
            return 1;
        }
    };
    let nonce = match parse_hex(&args[10]) {
        Some(value) => value,
        None => {
            eprintln!("nonce must be hex bytes");
            return 1;
        }
    };
    let bytes = capability_request_bytes(CapabilityRequestInput {
        kind,
        operation,
        assurance,
        app_id: &app_id,
        release_id: &release_id,
        subject_sha256: &subject_sha256,
        payload_sha256: &payload_sha256,
        context: &context,
        payload: &payload,
        nonce: &nonce,
    });
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write capability request: {err}");
        return 1;
    }
    println!("capability_request: {}", out.display());
    println!("request_sha256: {}", bytes_to_hex(&sha256(&bytes)));
    0
}

pub(crate) fn cmd_verify_capability_response(args: Vec<String>) -> i32 {
    if args.len() < 2 {
        eprintln!("verify-capability-response requires request.rkyv and response.rkyv");
        return 1;
    }
    let Ok(request_bytes) = fs::read(&args[0]) else {
        eprintln!("cannot read request: {}", args[0]);
        return 1;
    };
    let Some(request) = parse_capability_request_record(&request_bytes) else {
        eprintln!("invalid capability request: {}", args[0]);
        return 1;
    };
    let Ok(response_bytes) = fs::read(&args[1]) else {
        eprintln!("cannot read response: {}", args[1]);
        return 1;
    };
    let Some(response) = parse_capability_response_record(&response_bytes) else {
        eprintln!("invalid capability response: {}", args[1]);
        return 1;
    };
    let binding_ok = capability_response_binding_ok(&request_bytes, &request, &response);
    let denial_proof_ok = response.status == CAPABILITY_STATUS_OK
        || response.proof
            == capability_denial_proof(
                &request_bytes,
                response.status,
                &response.provider,
                &response.payload,
            );
    println!("capability_request: {}", args[0]);
    println!("capability_response: {}", args[1]);
    println!("kind: {}", response.capability_kind);
    println!("operation: {}", response.operation);
    println!("status: {}", response.status);
    println!("provider: {}", String::from_utf8_lossy(&response.provider));
    print_check("request-binding", binding_ok);
    if response.status != CAPABILITY_STATUS_OK {
        print_check("denial-proof", denial_proof_ok);
    }
    if binding_ok && denial_proof_ok { 0 } else { 1 }
}

pub(crate) fn cmd_write_seal_request(args: Vec<String>) -> i32 {
    if args.len() < 8 {
        eprintln!(
            "write-seal-request requires out.rkyv, app id, release id, subject hash, plaintext hex, policy/context hex, assurance, and nonce hex"
        );
        return 1;
    }
    write_sealing_request(
        &args,
        CAPABILITY_OPERATION_SEAL,
        "plaintext",
        "cannot write seal request",
    )
}

pub(crate) fn cmd_write_unseal_request(args: Vec<String>) -> i32 {
    if args.len() < 8 {
        eprintln!(
            "write-unseal-request requires out.rkyv, app id, release id, subject hash, sealed envelope hex, policy/context hex, assurance, and nonce hex"
        );
        return 1;
    }
    write_sealing_request(
        &args,
        CAPABILITY_OPERATION_UNSEAL,
        "sealed envelope",
        "cannot write unseal request",
    )
}

pub(crate) fn write_sealing_request(
    args: &[String],
    operation: u16,
    payload_name: &str,
    write_error: &str,
) -> i32 {
    let out = PathBuf::from(&args[0]);
    let app_id = match hex_to_32(&args[1]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad app id: {err}");
            return 1;
        }
    };
    let release_id = match hex_to_32(&args[2]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad release id: {err}");
            return 1;
        }
    };
    let subject_sha256 = match hex_to_32(&args[3]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad subject hash: {err}");
            return 1;
        }
    };
    let payload = match parse_hex(&args[4]) {
        Some(value) => value,
        None => {
            eprintln!("{payload_name} must be hex bytes");
            return 1;
        }
    };
    let context = match parse_hex(&args[5]) {
        Some(value) => value,
        None => {
            eprintln!("policy/context must be hex bytes");
            return 1;
        }
    };
    let assurance = match parse_u16_arg(&args[6], "assurance") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let nonce = match parse_hex(&args[7]) {
        Some(value) => value,
        None => {
            eprintln!("nonce must be hex bytes");
            return 1;
        }
    };
    let payload_sha256 = sha256(&payload);
    let bytes = capability_request_bytes(CapabilityRequestInput {
        kind: CAPABILITY_KIND_SEALING,
        operation,
        assurance,
        app_id: &app_id,
        release_id: &release_id,
        subject_sha256: &subject_sha256,
        payload_sha256: &payload_sha256,
        context: &context,
        payload: &payload,
        nonce: &nonce,
    });
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("{write_error}: {err}");
        return 1;
    }
    println!("capability_request: {}", out.display());
    println!("request_sha256: {}", bytes_to_hex(&sha256(&bytes)));
    println!("payload_sha256: {}", bytes_to_hex(&payload_sha256));
    0
}

pub(crate) fn cmd_seal_request(args: Vec<String>) -> i32 {
    if args.len() < 4 {
        eprintln!(
            "seal-request requires request.rkyv, out.rkyv, provider, seal key hex, and optional assurance"
        );
        return 1;
    }
    execute_sealing_request(&args, CAPABILITY_OPERATION_SEAL, None)
}

pub(crate) fn cmd_seal_request_authorized(args: Vec<String>) -> i32 {
    if args.len() < 7 {
        eprintln!(
            "seal-request-authorized requires profile.eusr, profile key hex, request.rkyv, out.rkyv, provider, seal key hex, at, and optional assurance"
        );
        return 1;
    }
    let execution_args = vec![
        args[2].clone(),
        args[3].clone(),
        args[4].clone(),
        args[5].clone(),
    ];
    let Some(at) = parse_authorized_at(&args[6]) else {
        return 1;
    };
    let mut execution_args = execution_args;
    if let Some(assurance) = args.get(7) {
        execution_args.push(assurance.clone());
    }
    execute_sealing_request(
        &execution_args,
        CAPABILITY_OPERATION_SEAL,
        Some(ProfileAuthorizationInput {
            profile_path: &args[0],
            profile_key_hex: &args[1],
            at,
        }),
    )
}

pub(crate) fn cmd_unseal_request(args: Vec<String>) -> i32 {
    if args.len() < 4 {
        eprintln!(
            "unseal-request requires request.rkyv, out.rkyv, provider, seal key hex, and optional assurance"
        );
        return 1;
    }
    execute_sealing_request(&args, CAPABILITY_OPERATION_UNSEAL, None)
}

pub(crate) fn cmd_unseal_request_authorized(args: Vec<String>) -> i32 {
    if args.len() < 7 {
        eprintln!(
            "unseal-request-authorized requires profile.eusr, profile key hex, request.rkyv, out.rkyv, provider, seal key hex, at, and optional assurance"
        );
        return 1;
    }
    let mut execution_args = vec![
        args[2].clone(),
        args[3].clone(),
        args[4].clone(),
        args[5].clone(),
    ];
    let Some(at) = parse_authorized_at(&args[6]) else {
        return 1;
    };
    if let Some(assurance) = args.get(7) {
        execution_args.push(assurance.clone());
    }
    execute_sealing_request(
        &execution_args,
        CAPABILITY_OPERATION_UNSEAL,
        Some(ProfileAuthorizationInput {
            profile_path: &args[0],
            profile_key_hex: &args[1],
            at,
        }),
    )
}

pub(crate) struct ProfileAuthorizationInput<'a> {
    pub(crate) profile_path: &'a str,
    pub(crate) profile_key_hex: &'a str,
    pub(crate) at: u64,
}

pub(crate) fn execute_sealing_request(
    args: &[String],
    expected_operation: u16,
    authorization: Option<ProfileAuthorizationInput<'_>>,
) -> i32 {
    let request_path = PathBuf::from(&args[0]);
    let out = PathBuf::from(&args[1]);
    let Ok(request_bytes) = fs::read(&request_path) else {
        eprintln!("cannot read request: {}", request_path.display());
        return 1;
    };
    let Some(request) = parse_capability_request_record(&request_bytes) else {
        eprintln!("invalid capability request: {}", request_path.display());
        return 1;
    };
    if request.capability_kind != CAPABILITY_KIND_SEALING || request.operation != expected_operation
    {
        eprintln!("request is not the expected sealing operation");
        return 1;
    }
    if request.payload_sha256 != sha256(&request.payload).as_slice() {
        eprintln!("request payload hash does not match inline payload");
        return 1;
    }
    if let Some(authorization) = authorization {
        let authorized = match profile_authorizes_request(&authorization, &request) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        };
        print_check("profile-authorization", authorized);
        if !authorized {
            return write_capability_denial_response(
                &out,
                &request_bytes,
                &request,
                args[2].as_bytes(),
                CAPABILITY_STATUS_POLICY_DENIED,
                b"policy_denied",
            );
        }
    }
    let Some(key) = parse_seal_key(&args[3]) else {
        eprintln!("seal key must be 32 hex bytes");
        return 1;
    };
    let assurance = match args.get(4) {
        Some(value) => match parse_u16_arg(value, "assurance") {
            Ok(value) => value,
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        },
        None => request.assurance,
    };
    let result = if expected_operation == CAPABILITY_OPERATION_SEAL {
        seal_with_key(&request.payload, &key)
    } else {
        unseal_with_key(&request.payload, &key)
    };
    let payload = match result {
        Ok(value) => value,
        Err(err) => {
            eprintln!("sealing capability failed: {err:?}");
            return 1;
        }
    };
    let responder = sha256(key.expose_secret());
    let proof = sealing_response_proof(
        &request_bytes,
        expected_operation,
        args[2].as_bytes(),
        &payload,
    );
    let bytes = capability_response_bytes(CapabilityResponseInput {
        request_bytes: &request_bytes,
        kind: CAPABILITY_KIND_SEALING,
        operation: expected_operation,
        status: CAPABILITY_STATUS_OK,
        assurance,
        provider: args[2].as_bytes(),
        responder: &responder,
        payload: &payload,
        proof: &proof,
    });
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write sealing response: {err}");
        return 1;
    }
    println!("capability_response: {}", out.display());
    println!("request_sha256: {}", bytes_to_hex(&sha256(&request_bytes)));
    println!("provider: {}", args[2]);
    println!("responder: {}", bytes_to_hex(&responder));
    println!("payload_sha256: {}", bytes_to_hex(&sha256(&payload)));
    0
}

pub(crate) fn parse_authorized_at(value: &str) -> Option<u64> {
    match parse_u64_arg(value, "at") {
        Ok(value) => Some(value),
        Err(err) => {
            eprintln!("{err}");
            None
        }
    }
}

pub(crate) fn profile_authorizes_request(
    input: &ProfileAuthorizationInput<'_>,
    request: &edgerun_wire::CapabilityRequest,
) -> Result<bool, String> {
    let profile_bytes = fs::read(input.profile_path)
        .map_err(|_| format!("cannot read user profile: {}", input.profile_path))?;
    let seal_key = parse_seal_key(input.profile_key_hex)
        .ok_or_else(|| "profile key must be 32 hex bytes".to_owned())?;
    let profile = open_user_profile_file(&profile_bytes, &seal_key)?;
    if !verify_user_profile_body_signature(&profile) {
        return Ok(false);
    }
    Ok(user_profile_allows_request(&profile, request, input.at))
}

pub(crate) fn cmd_verify_seal_response(args: Vec<String>) -> i32 {
    if args.len() < 2 {
        eprintln!(
            "verify-seal-response requires request.rkyv, response.rkyv, and optional seal key hex"
        );
        return 1;
    }
    let Ok(request_bytes) = fs::read(&args[0]) else {
        eprintln!("cannot read request: {}", args[0]);
        return 1;
    };
    let Some(request) = parse_capability_request_record(&request_bytes) else {
        eprintln!("invalid capability request: {}", args[0]);
        return 1;
    };
    let Ok(response_bytes) = fs::read(&args[1]) else {
        eprintln!("cannot read response: {}", args[1]);
        return 1;
    };
    let Some(response) = parse_capability_response_record(&response_bytes) else {
        eprintln!("invalid capability response: {}", args[1]);
        return 1;
    };
    let binding_ok = capability_response_binding_ok(&request_bytes, &request, &response)
        && request.capability_kind == CAPABILITY_KIND_SEALING
        && response.capability_kind == CAPABILITY_KIND_SEALING
        && response.status == CAPABILITY_STATUS_OK;
    let proof_ok = response.proof
        == sealing_response_proof(
            &request_bytes,
            response.operation,
            &response.provider,
            &response.payload,
        );
    let key_check = args.get(2).and_then(|value| parse_seal_key(value));
    let key_ok = key_check.as_ref().is_none_or(|key| {
        if response.operation == CAPABILITY_OPERATION_SEAL {
            unseal_with_key(&response.payload, key)
                .is_ok_and(|plaintext| plaintext == request.payload)
        } else if response.operation == CAPABILITY_OPERATION_UNSEAL {
            unseal_with_key(&request.payload, key)
                .is_ok_and(|plaintext| plaintext == response.payload)
        } else {
            false
        }
    });
    println!("capability_request: {}", args[0]);
    println!("capability_response: {}", args[1]);
    println!("operation: {}", response.operation);
    println!("provider: {}", String::from_utf8_lossy(&response.provider));
    print_check("request-binding", binding_ok);
    print_check("runtime-proof", proof_ok);
    if args.get(2).is_some() {
        print_check("seal-key-roundtrip", key_ok);
    }
    if binding_ok && proof_ok && key_ok {
        0
    } else {
        1
    }
}

pub(crate) fn cmd_write_storage_read_request(args: Vec<String>) -> i32 {
    if args.len() < 8 {
        eprintln!(
            "write-storage-read-request requires out.rkyv, app id, release id, subject hash, key/context hex, expected sha256|any, assurance, and nonce hex"
        );
        return 1;
    }
    write_storage_request(&args, CAPABILITY_OPERATION_READ)
}

pub(crate) fn cmd_write_storage_write_request(args: Vec<String>) -> i32 {
    if args.len() < 8 {
        eprintln!(
            "write-storage-write-request requires out.rkyv, app id, release id, subject hash, key/context hex, payload hex, assurance, and nonce hex"
        );
        return 1;
    }
    write_storage_request(&args, CAPABILITY_OPERATION_WRITE)
}

pub(crate) fn write_storage_request(args: &[String], operation: u16) -> i32 {
    let out = PathBuf::from(&args[0]);
    let app_id = match hex_to_32(&args[1]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad app id: {err}");
            return 1;
        }
    };
    let release_id = match hex_to_32(&args[2]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad release id: {err}");
            return 1;
        }
    };
    let subject_sha256 = match hex_to_32(&args[3]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad subject hash: {err}");
            return 1;
        }
    };
    let context = match parse_hex(&args[4]) {
        Some(value) => value,
        None => {
            eprintln!("key/context must be hex bytes");
            return 1;
        }
    };
    let (payload, payload_sha256) = if operation == CAPABILITY_OPERATION_WRITE {
        let payload = match parse_hex(&args[5]) {
            Some(value) => value,
            None => {
                eprintln!("payload must be hex bytes");
                return 1;
            }
        };
        let payload_sha256 = sha256(&payload);
        (payload, payload_sha256)
    } else {
        let payload_sha256 = if args[5] == "any" {
            [0u8; 32]
        } else {
            match hex_to_32(&args[5]) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("bad expected sha256: {err}");
                    return 1;
                }
            }
        };
        (Vec::new(), payload_sha256)
    };
    let assurance = match parse_u16_arg(&args[6], "assurance") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let nonce = match parse_hex(&args[7]) {
        Some(value) => value,
        None => {
            eprintln!("nonce must be hex bytes");
            return 1;
        }
    };
    let bytes = capability_request_bytes(CapabilityRequestInput {
        kind: CAPABILITY_KIND_STORAGE,
        operation,
        assurance,
        app_id: &app_id,
        release_id: &release_id,
        subject_sha256: &subject_sha256,
        payload_sha256: &payload_sha256,
        context: &context,
        payload: &payload,
        nonce: &nonce,
    });
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write storage request: {err}");
        return 1;
    }
    println!("capability_request: {}", out.display());
    println!("request_sha256: {}", bytes_to_hex(&sha256(&bytes)));
    println!("payload_sha256: {}", bytes_to_hex(&payload_sha256));
    0
}

pub(crate) fn cmd_storage_read_request(args: Vec<String>) -> i32 {
    if args.len() < 4 {
        eprintln!(
            "storage-read-request requires request.rkyv, out.rkyv, provider, root dir, and optional assurance"
        );
        return 1;
    }
    execute_storage_request(&args, CAPABILITY_OPERATION_READ, None)
}

pub(crate) fn cmd_storage_read_request_authorized(args: Vec<String>) -> i32 {
    if args.len() < 7 {
        eprintln!(
            "storage-read-request-authorized requires profile.eusr, profile key hex, request.rkyv, out.rkyv, provider, root dir, at, and optional assurance"
        );
        return 1;
    }
    execute_storage_authorized(&args, CAPABILITY_OPERATION_READ)
}

pub(crate) fn cmd_storage_write_request(args: Vec<String>) -> i32 {
    if args.len() < 4 {
        eprintln!(
            "storage-write-request requires request.rkyv, out.rkyv, provider, root dir, and optional assurance"
        );
        return 1;
    }
    execute_storage_request(&args, CAPABILITY_OPERATION_WRITE, None)
}

pub(crate) fn cmd_storage_write_request_authorized(args: Vec<String>) -> i32 {
    if args.len() < 7 {
        eprintln!(
            "storage-write-request-authorized requires profile.eusr, profile key hex, request.rkyv, out.rkyv, provider, root dir, at, and optional assurance"
        );
        return 1;
    }
    execute_storage_authorized(&args, CAPABILITY_OPERATION_WRITE)
}

pub(crate) fn execute_storage_authorized(args: &[String], expected_operation: u16) -> i32 {
    let mut execution_args = vec![
        args[2].clone(),
        args[3].clone(),
        args[4].clone(),
        args[5].clone(),
    ];
    let Some(at) = parse_authorized_at(&args[6]) else {
        return 1;
    };
    if let Some(assurance) = args.get(7) {
        execution_args.push(assurance.clone());
    }
    execute_storage_request(
        &execution_args,
        expected_operation,
        Some(ProfileAuthorizationInput {
            profile_path: &args[0],
            profile_key_hex: &args[1],
            at,
        }),
    )
}

pub(crate) fn execute_storage_request(
    args: &[String],
    expected_operation: u16,
    authorization: Option<ProfileAuthorizationInput<'_>>,
) -> i32 {
    let request_path = PathBuf::from(&args[0]);
    let out = PathBuf::from(&args[1]);
    let Ok(request_bytes) = fs::read(&request_path) else {
        eprintln!("cannot read request: {}", request_path.display());
        return 1;
    };
    let Some(request) = parse_capability_request_record(&request_bytes) else {
        eprintln!("invalid capability request: {}", request_path.display());
        return 1;
    };
    if request.capability_kind != CAPABILITY_KIND_STORAGE || request.operation != expected_operation
    {
        eprintln!("request is not the expected storage operation");
        return 1;
    }
    if expected_operation == CAPABILITY_OPERATION_WRITE
        && request.payload_sha256 != sha256(&request.payload).as_slice()
    {
        eprintln!("request payload hash does not match inline payload");
        return 1;
    }
    if let Some(authorization) = authorization {
        let authorized = match profile_authorizes_request(&authorization, &request) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        };
        print_check("profile-authorization", authorized);
        if !authorized {
            return write_capability_denial_response(
                &out,
                &request_bytes,
                &request,
                args[2].as_bytes(),
                CAPABILITY_STATUS_POLICY_DENIED,
                b"policy_denied",
            );
        }
    }
    let root = PathBuf::from(&args[3]);
    let storage_path = storage_provider_path(&root, &request.context);
    let payload = if expected_operation == CAPABILITY_OPERATION_WRITE {
        if let Err(err) = fs::create_dir_all(&root) {
            eprintln!("cannot create storage root: {err}");
            return 1;
        }
        if let Err(err) = fs::write(&storage_path, &request.payload) {
            eprintln!("cannot write storage object: {err}");
            return 1;
        }
        storage_write_receipt_payload(&request.payload)
    } else {
        let Ok(bytes) = fs::read(&storage_path) else {
            eprintln!("cannot read storage object: {}", storage_path.display());
            return 1;
        };
        if request.payload_sha256 != [0u8; 32].as_slice()
            && request.payload_sha256 != sha256(&bytes).as_slice()
        {
            eprintln!("read payload hash does not match request expectation");
            return 1;
        }
        bytes
    };
    let assurance = match args.get(4) {
        Some(value) => match parse_u16_arg(value, "assurance") {
            Ok(value) => value,
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        },
        None => request.assurance,
    };
    let responder = sha256(root.to_string_lossy().as_bytes());
    let proof = storage_response_proof(
        &request_bytes,
        expected_operation,
        args[2].as_bytes(),
        &payload,
    );
    let bytes = capability_response_bytes(CapabilityResponseInput {
        request_bytes: &request_bytes,
        kind: CAPABILITY_KIND_STORAGE,
        operation: expected_operation,
        status: CAPABILITY_STATUS_OK,
        assurance,
        provider: args[2].as_bytes(),
        responder: &responder,
        payload: &payload,
        proof: &proof,
    });
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write storage response: {err}");
        return 1;
    }
    println!("capability_response: {}", out.display());
    println!("request_sha256: {}", bytes_to_hex(&sha256(&request_bytes)));
    println!("provider: {}", args[2]);
    println!("object: {}", storage_path.display());
    println!("payload_sha256: {}", bytes_to_hex(&sha256(&payload)));
    0
}

pub(crate) fn cmd_verify_storage_response(args: Vec<String>) -> i32 {
    if args.len() < 2 {
        eprintln!("verify-storage-response requires request.rkyv and response.rkyv");
        return 1;
    }
    let Ok(request_bytes) = fs::read(&args[0]) else {
        eprintln!("cannot read request: {}", args[0]);
        return 1;
    };
    let Some(request) = parse_capability_request_record(&request_bytes) else {
        eprintln!("invalid capability request: {}", args[0]);
        return 1;
    };
    let Ok(response_bytes) = fs::read(&args[1]) else {
        eprintln!("cannot read response: {}", args[1]);
        return 1;
    };
    let Some(response) = parse_capability_response_record(&response_bytes) else {
        eprintln!("invalid capability response: {}", args[1]);
        return 1;
    };
    let binding_ok = capability_response_binding_ok(&request_bytes, &request, &response)
        && request.capability_kind == CAPABILITY_KIND_STORAGE
        && response.capability_kind == CAPABILITY_KIND_STORAGE
        && response.status == CAPABILITY_STATUS_OK;
    let proof_ok = response.proof
        == storage_response_proof(
            &request_bytes,
            response.operation,
            &response.provider,
            &response.payload,
        );
    let payload_ok = if response.operation == CAPABILITY_OPERATION_WRITE {
        storage_write_receipt_matches(&response.payload, &request.payload)
    } else {
        request.payload_sha256 == [0u8; 32].as_slice()
            || request.payload_sha256 == sha256(&response.payload).as_slice()
    };
    println!("capability_request: {}", args[0]);
    println!("capability_response: {}", args[1]);
    println!("operation: {}", response.operation);
    println!("provider: {}", String::from_utf8_lossy(&response.provider));
    print_check("request-binding", binding_ok);
    print_check("runtime-proof", proof_ok);
    print_check("payload-commitment", payload_ok);
    if binding_ok && proof_ok && payload_ok {
        0
    } else {
        1
    }
}

pub(crate) fn cmd_create_user_profile(args: Vec<String>) -> i32 {
    if args.len() < 5 {
        eprintln!(
            "create-user-profile requires out.eusr, owner seed hex, seal key hex, epoch, and monotonic-version"
        );
        return 1;
    }
    let out = PathBuf::from(&args[0]);
    let Some(owner_seed) = parse_seed(&args[1]) else {
        eprintln!("owner seed must be 32 hex bytes");
        return 1;
    };
    let Some(seal_key) = parse_seal_key(&args[2]) else {
        eprintln!("seal key must be 32 hex bytes");
        return 1;
    };
    let epoch = match parse_u64_arg(&args[3], "epoch") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let monotonic_version = match parse_u64_arg(&args[4], "monotonic-version") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let owner_key = SigningKey::from_bytes(&owner_seed);
    let profile_id = user_profile_id(owner_key.verifying_key().as_bytes(), epoch);
    let body = user_profile_body_with_owner_seed_bytes(
        &profile_id,
        &owner_seed,
        epoch,
        monotonic_version,
        &[],
    );
    let bytes = match user_profile_file_bytes(&body, &seal_key) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write user profile: {err}");
        return 1;
    }
    println!("user_profile: {}", out.display());
    println!("profile_id: {}", bytes_to_hex(&profile_id));
    println!(
        "owner_id: {}",
        bytes_to_hex(owner_key.verifying_key().as_bytes())
    );
    println!("body_sha256: {}", bytes_to_hex(&sha256(&body)));
    0
}

pub(crate) fn cmd_create_user_profile_password(args: Vec<String>) -> i32 {
    if args.len() < 4 {
        eprintln!(
            "create-user-profile-password requires out.eusr, owner seed hex, password, and epoch [monotonic-version]"
        );
        return 1;
    }
    let out = PathBuf::from(&args[0]);
    let Some(owner_seed) = parse_seed(&args[1]) else {
        eprintln!("owner seed must be 32 hex bytes");
        return 1;
    };
    let epoch = match parse_u64_arg(&args[3], "epoch") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let monotonic_version = match args.get(4) {
        Some(value) => match parse_u64_arg(value, "monotonic-version") {
            Ok(value) => value,
            Err(err) => {
                eprintln!("{err}");
                return 1;
            }
        },
        None => 1,
    };
    let owner_key = SigningKey::from_bytes(&owner_seed);
    let profile_id = user_profile_id(owner_key.verifying_key().as_bytes(), epoch);
    let body = user_profile_body_with_owner_seed_bytes(
        &profile_id,
        &owner_seed,
        epoch,
        monotonic_version,
        &[],
    );
    let bytes = match user_profile_file_bytes_with_password(&body, &args[2]) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    if let Err(err) = fs::write(&out, &bytes) {
        eprintln!("cannot write user profile: {err}");
        return 1;
    }
    println!("user_profile: {}", out.display());
    println!("profile_id: {}", bytes_to_hex(&profile_id));
    println!(
        "owner_id: {}",
        bytes_to_hex(owner_key.verifying_key().as_bytes())
    );
    println!("owner_private_key: sealed");
    println!("body_sha256: {}", bytes_to_hex(&sha256(&body)));
    0
}

pub(crate) fn cmd_grant_profile_capability(args: Vec<String>) -> i32 {
    if args.len() < 13 {
        eprintln!(
            "grant-profile-capability requires in.eusr, out.eusr, seal key hex, owner seed hex, app id, release id|any, kind, operation, scope hash|any|context:<hex>, min assurance, valid-from, valid-until, and monotonic-version"
        );
        return 1;
    }
    let Ok(profile_bytes) = fs::read(&args[0]) else {
        eprintln!("cannot read user profile: {}", args[0]);
        return 1;
    };
    let Some(seal_key) = parse_seal_key(&args[2]) else {
        eprintln!("seal key must be 32 hex bytes");
        return 1;
    };
    let existing = match open_user_profile_file(&profile_bytes, &seal_key) {
        Ok(body) => body,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let Some(owner_seed) = parse_seed(&args[3]) else {
        eprintln!("owner seed must be 32 hex bytes");
        return 1;
    };
    let owner_key = SigningKey::from_bytes(&owner_seed);
    if owner_key.verifying_key().as_bytes() != &existing.owner_id {
        eprintln!("owner seed does not match profile owner");
        return 1;
    }
    let app_id = match hex_to_32(&args[4]) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("bad app id: {err}");
            return 1;
        }
    };
    let release_id = if args[5] == "any" {
        [0u8; 32]
    } else {
        match hex_to_32(&args[5]) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("bad release id: {err}");
                return 1;
            }
        }
    };
    let capability_kind = match parse_capability_kind(&args[6]) {
        Some(value) => value,
        None => {
            eprintln!("bad capability kind: {}", args[6]);
            return 1;
        }
    };
    let operation = match parse_capability_operation(&args[7]) {
        Some(value) => value,
        None => {
            eprintln!("bad capability operation: {}", args[7]);
            return 1;
        }
    };
    let scope_sha256 = match parse_scope_hash(&args[8]) {
        Some(value) => value,
        None => {
            eprintln!("scope must be any, 32-byte hex, or context:<hex>");
            return 1;
        }
    };
    let min_assurance = match parse_u16_arg(&args[9], "min-assurance") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let valid_from = match parse_u64_arg(&args[10], "valid-from") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let valid_until = match parse_u64_arg(&args[11], "valid-until") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let monotonic_version = match parse_u64_arg(&args[12], "monotonic-version") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    if monotonic_version <= existing.monotonic_version {
        eprintln!("monotonic-version must increase");
        return 1;
    }
    let mut grants: Vec<OwnedUserGrant> =
        existing.grants.iter().map(OwnedUserGrant::from).collect();
    grants.push(OwnedUserGrant {
        app_id,
        release_id,
        scope_sha256,
        constraints_sha256: [0; 32],
        capability_kind,
        operation,
        min_assurance,
        flags: 0,
        valid_from,
        valid_until,
        user_signature: Vec::new(),
    });
    let profile_id = existing.profile_id;
    let new_body = user_profile_body_with_owner_seed_bytes(
        &profile_id,
        &owner_seed,
        existing.epoch,
        monotonic_version,
        &grants,
    );
    let new_file = match user_profile_file_bytes(&new_body, &seal_key) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    if let Err(err) = fs::write(&args[1], &new_file) {
        eprintln!("cannot write user profile: {err}");
        return 1;
    }
    println!("user_profile: {}", args[1]);
    println!("profile_id: {}", bytes_to_hex(&profile_id));
    println!("grant_count: {}", grants.len());
    println!("body_sha256: {}", bytes_to_hex(&sha256(&new_body)));
    0
}

pub(crate) fn cmd_open_user_profile(args: Vec<String>) -> i32 {
    if args.len() < 2 {
        eprintln!("open-user-profile requires profile.eusr and seal key hex");
        return 1;
    }
    let Ok(profile_bytes) = fs::read(&args[0]) else {
        eprintln!("cannot read user profile: {}", args[0]);
        return 1;
    };
    let Some(seal_key) = parse_seal_key(&args[1]) else {
        eprintln!("seal key must be 32 hex bytes");
        return 1;
    };
    let parsed = match open_user_profile_file(&profile_bytes, &seal_key) {
        Ok(body) => body,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    println!("user_profile: {}", args[0]);
    println!("profile_id: {}", bytes_to_hex(&parsed.profile_id));
    println!("owner_id: {}", bytes_to_hex(&parsed.owner_id));
    println!("epoch: {}", parsed.epoch);
    println!("monotonic_version: {}", parsed.monotonic_version);
    println!(
        "owner_private_key: {}",
        if parsed.owner_private_key.is_empty() {
            "missing"
        } else {
            "sealed"
        }
    );
    println!("grant_count: {}", parsed.grants.len());
    print_check(
        "owner-signature",
        verify_user_profile_body_signature(&parsed),
    );
    0
}

pub(crate) fn cmd_open_user_profile_password(args: Vec<String>) -> i32 {
    if args.len() < 2 {
        eprintln!("open-user-profile-password requires profile.eusr and password");
        return 1;
    }
    let Ok(profile_bytes) = fs::read(&args[0]) else {
        eprintln!("cannot read user profile: {}", args[0]);
        return 1;
    };
    let parsed = match open_user_profile_file_with_password(&profile_bytes, &args[1]) {
        Ok(body) => body,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    println!("user_profile: {}", args[0]);
    println!("profile_id: {}", bytes_to_hex(&parsed.profile_id));
    println!("owner_id: {}", bytes_to_hex(&parsed.owner_id));
    println!("epoch: {}", parsed.epoch);
    println!("monotonic_version: {}", parsed.monotonic_version);
    println!(
        "owner_private_key: {}",
        if parsed.owner_private_key.is_empty() {
            "missing"
        } else {
            "sealed"
        }
    );
    println!("grant_count: {}", parsed.grants.len());
    print_check(
        "owner-signature",
        verify_user_profile_body_signature(&parsed),
    );
    0
}

pub(crate) fn cmd_verify_profile_access(args: Vec<String>) -> i32 {
    if args.len() < 4 {
        eprintln!(
            "verify-profile-access requires profile.eusr, seal key hex, request.rkyv, and at"
        );
        return 1;
    }
    let Ok(profile_bytes) = fs::read(&args[0]) else {
        eprintln!("cannot read user profile: {}", args[0]);
        return 1;
    };
    let Some(seal_key) = parse_seal_key(&args[1]) else {
        eprintln!("seal key must be 32 hex bytes");
        return 1;
    };
    let profile = match open_user_profile_file(&profile_bytes, &seal_key) {
        Ok(body) => body,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let Ok(request_bytes) = fs::read(&args[2]) else {
        eprintln!("cannot read capability request: {}", args[2]);
        return 1;
    };
    let Some(request) = parse_capability_request_record(&request_bytes) else {
        eprintln!("invalid capability request: {}", args[2]);
        return 1;
    };
    let at = match parse_u64_arg(&args[3], "at") {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    let signature_ok = verify_user_profile_body_signature(&profile);
    let allowed = signature_ok && user_profile_allows_request(&profile, &request, at);
    println!("user_profile: {}", args[0]);
    println!("capability_request: {}", args[2]);
    print_check("profile-signature", signature_ok);
    print_check("capability-access", allowed);
    if allowed { 0 } else { 1 }
}

pub(crate) struct CapabilityRequestInput<'a> {
    pub(crate) kind: u16,
    pub(crate) operation: u16,
    pub(crate) assurance: u16,
    pub(crate) app_id: &'a [u8; 32],
    pub(crate) release_id: &'a [u8; 32],
    pub(crate) subject_sha256: &'a [u8; 32],
    pub(crate) payload_sha256: &'a [u8; 32],
    pub(crate) context: &'a [u8],
    pub(crate) payload: &'a [u8],
    pub(crate) nonce: &'a [u8],
}

pub(crate) fn capability_request_bytes(input: CapabilityRequestInput<'_>) -> Vec<u8> {
    sdk_wire_record_bytes(SdkWireRecord::CapabilityRequest(
        edgerun_wire::CapabilityRequest::new(
            input.kind,
            input.operation,
            input.assurance,
            *input.app_id,
            *input.release_id,
            *input.subject_sha256,
            *input.payload_sha256,
            input.context.to_vec(),
            input.payload.to_vec(),
            input.nonce.to_vec(),
        ),
    ))
}

pub(crate) struct CapabilityResponseInput<'a> {
    pub(crate) request_bytes: &'a [u8],
    pub(crate) kind: u16,
    pub(crate) operation: u16,
    pub(crate) status: u16,
    pub(crate) assurance: u16,
    pub(crate) provider: &'a [u8],
    pub(crate) responder: &'a [u8],
    pub(crate) payload: &'a [u8],
    pub(crate) proof: &'a [u8],
}

pub(crate) fn capability_response_bytes(input: CapabilityResponseInput<'_>) -> Vec<u8> {
    sdk_wire_record_bytes(SdkWireRecord::CapabilityResponse(
        edgerun_wire::CapabilityResponse {
            abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
            flags: 1,
            capability_kind: input.kind,
            operation: input.operation,
            status: input.status,
            assurance: input.assurance,
            request_sha256: sha256(input.request_bytes),
            provider: input.provider.to_vec(),
            responder: input.responder.to_vec(),
            payload: input.payload.to_vec(),
            proof: input.proof.to_vec(),
        },
    ))
}

pub(crate) fn capability_response_binding_ok(
    request_bytes: &[u8],
    request: &edgerun_wire::CapabilityRequest,
    response: &edgerun_wire::CapabilityResponse,
) -> bool {
    response.request_sha256 == sha256(request_bytes)
        && response.capability_kind == request.capability_kind
        && response.operation == request.operation
        && response.assurance >= request.assurance
}

pub(crate) fn capability_denial_response_bytes(
    request_bytes: &[u8],
    request: &edgerun_wire::CapabilityRequest,
    provider: &[u8],
    status: u16,
    reason: &[u8],
) -> Vec<u8> {
    let responder = sha256(b"edgerun-user-profile-policy");
    let proof = capability_denial_proof(request_bytes, status, provider, reason);
    capability_response_bytes(CapabilityResponseInput {
        request_bytes,
        kind: request.capability_kind,
        operation: request.operation,
        status,
        assurance: request.assurance,
        provider,
        responder: &responder,
        payload: reason,
        proof: proof.as_slice(),
    })
}

pub(crate) fn write_capability_denial_response(
    out: &Path,
    request_bytes: &[u8],
    request: &edgerun_wire::CapabilityRequest,
    provider: &[u8],
    status: u16,
    reason: &[u8],
) -> i32 {
    let bytes = capability_denial_response_bytes(request_bytes, request, provider, status, reason);
    if let Err(err) = fs::write(out, &bytes) {
        eprintln!("cannot write denial response: {err}");
        return 1;
    }
    println!("capability_response: {}", out.display());
    println!("request_sha256: {}", bytes_to_hex(&sha256(request_bytes)));
    println!("provider: {}", String::from_utf8_lossy(provider));
    println!("status: {status}");
    println!("reason: {}", String::from_utf8_lossy(reason));
    1
}

pub(crate) fn capability_denial_proof(
    request_bytes: &[u8],
    status: u16,
    provider: &[u8],
    payload: &[u8],
) -> [u8; 32] {
    capability_response_proof(request_bytes, status, provider, payload)
}

pub(crate) fn sealing_response_proof(
    request_bytes: &[u8],
    operation: u16,
    provider: &[u8],
    payload: &[u8],
) -> [u8; 32] {
    capability_response_proof(request_bytes, operation, provider, payload)
}

pub(crate) fn storage_provider_path(root: &Path, context: &[u8]) -> PathBuf {
    root.join(format!("{}.bin", bytes_to_hex(&sha256(context))))
}

pub(crate) fn storage_write_receipt_payload(payload: &[u8]) -> Vec<u8> {
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&StorageWriteReceiptRecord {
        payload_sha256: sha256(payload),
        payload_len: payload.len() as u64,
    })
    .expect("storage write receipt must serialize through rkyv")
    .into_vec()
}

pub(crate) fn storage_write_receipt_matches(receipt: &[u8], payload: &[u8]) -> bool {
    let Ok(receipt) =
        edgerun_wire::from_bytes::<StorageWriteReceiptRecord, edgerun_wire::WireError>(receipt)
    else {
        return false;
    };
    receipt.payload_sha256 == sha256(payload) && receipt.payload_len == payload.len() as u64
}

pub(crate) fn storage_response_proof(
    request_bytes: &[u8],
    operation: u16,
    provider: &[u8],
    payload: &[u8],
) -> [u8; 32] {
    capability_response_proof(request_bytes, operation, provider, payload)
}

pub(crate) fn capability_response_proof(
    request_bytes: &[u8],
    operation_or_status: u16,
    provider: &[u8],
    payload: &[u8],
) -> [u8; 32] {
    let record = CapabilityResponseProofRecord {
        domain: CAPABILITY_RESPONSE_DOMAIN.to_vec(),
        request_sha256: sha256(request_bytes),
        operation_or_status,
        provider: provider.to_vec(),
        payload_sha256: sha256(payload),
    };
    sha256(
        &edgerun_wire::to_bytes::<edgerun_wire::WireError>(&record)
            .expect("capability response proof must serialize through rkyv"),
    )
}

pub(crate) struct SignRequestInput<'a> {
    pub(crate) domain: &'a [u8],
    pub(crate) app_id: &'a [u8; 32],
    pub(crate) release_id: &'a [u8; 32],
    pub(crate) subject_sha256: &'a [u8; 32],
    pub(crate) payload_sha256: &'a [u8; 32],
    pub(crate) algorithm: u16,
    pub(crate) assurance: u16,
    pub(crate) nonce: &'a [u8],
}

pub(crate) fn sign_request_bytes(input: SignRequestInput<'_>) -> Vec<u8> {
    let payload = signing_algorithm_payload(input.algorithm);
    capability_request_bytes(CapabilityRequestInput {
        kind: CAPABILITY_KIND_SIGNING,
        operation: CAPABILITY_OPERATION_SIGN,
        assurance: input.assurance,
        app_id: input.app_id,
        release_id: input.release_id,
        subject_sha256: input.subject_sha256,
        payload_sha256: input.payload_sha256,
        context: input.domain,
        payload: &payload,
        nonce: input.nonce,
    })
}

pub(crate) struct SignResponseInput<'a> {
    pub(crate) request_bytes: &'a [u8],
    pub(crate) provider: &'a [u8],
    pub(crate) algorithm: u16,
    pub(crate) assurance: u16,
    pub(crate) signer: &'a [u8],
    pub(crate) signing_key: &'a SigningKey,
}

pub(crate) fn sign_response_bytes(input: SignResponseInput<'_>) -> Vec<u8> {
    let request_sha256 = sha256(input.request_bytes);
    let signature = input.signing_key.sign(&signature_payload_for_domain(
        CAPABILITY_RESPONSE_DOMAIN,
        &request_sha256,
    ));
    let payload = signing_algorithm_payload(input.algorithm);
    capability_response_bytes(CapabilityResponseInput {
        request_bytes: input.request_bytes,
        kind: CAPABILITY_KIND_SIGNING,
        operation: CAPABILITY_OPERATION_SIGN,
        status: CAPABILITY_STATUS_OK,
        assurance: input.assurance,
        provider: input.provider,
        responder: input.signer,
        payload: &payload,
        proof: &signature.to_bytes(),
    })
}

pub(crate) fn signing_algorithm_payload(algorithm: u16) -> Vec<u8> {
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&SigningAlgorithmRecord { algorithm })
        .expect("signing algorithm payload must serialize through rkyv")
        .into_vec()
}

pub(crate) fn parse_signing_algorithm_payload(payload: &[u8]) -> Option<u16> {
    let owned = payload.to_vec();
    edgerun_wire::from_bytes::<SigningAlgorithmRecord, edgerun_wire::WireError>(&owned)
        .ok()
        .map(|record| record.algorithm)
}

pub(crate) fn signing_request_algorithm(request: &edgerun_wire::CapabilityRequest) -> Option<u16> {
    (request.capability_kind == CAPABILITY_KIND_SIGNING
        && request.operation == CAPABILITY_OPERATION_SIGN)
        .then(|| parse_signing_algorithm_payload(&request.payload))
        .flatten()
}

pub(crate) fn signing_response_algorithm(
    response: &edgerun_wire::CapabilityResponse,
) -> Option<u16> {
    (response.capability_kind == CAPABILITY_KIND_SIGNING
        && response.operation == CAPABILITY_OPERATION_SIGN)
        .then(|| parse_signing_algorithm_payload(&response.payload))
        .flatten()
}

pub(crate) fn verify_sign_response_signature(
    request_bytes: &[u8],
    response: &edgerun_wire::CapabilityResponse,
) -> bool {
    if signing_response_algorithm(response) != Some(SIGN_ALGORITHM_ED25519)
        || response.responder.len() != 32
        || response.proof.len() != 64
    {
        return false;
    }
    let request_sha256 = sha256(request_bytes);
    if response.request_sha256 != request_sha256 {
        return false;
    }
    let Ok(public_key_bytes) = <[u8; 32]>::try_from(response.responder.as_slice()) else {
        return false;
    };
    let Ok(signature_bytes) = <[u8; 64]>::try_from(response.proof.as_slice()) else {
        return false;
    };
    verify_ed25519_signature(
        &public_key_bytes,
        CAPABILITY_RESPONSE_DOMAIN,
        &request_sha256,
        &signature_bytes,
    )
}

#[derive(Clone, Copy)]
pub(crate) struct RevokedTarget {
    pub(crate) target: [u8; 32],
    pub(crate) issuer: [u8; 32],
    pub(crate) issued_at: u64,
}

#[derive(Default)]
pub(crate) struct RevocationSet {
    pub(crate) key: Vec<RevokedTarget>,
    pub(crate) app: Vec<RevokedTarget>,
    pub(crate) product: Vec<RevokedTarget>,
    pub(crate) entitlement: Vec<RevokedTarget>,
    pub(crate) payment: Vec<RevokedTarget>,
    pub(crate) settlement: Vec<RevokedTarget>,
}

impl RevocationSet {
    fn revoke_key(
        &self,
        target: &[u8],
        policy: &edgerun_wire::TrustPolicy,
        app_id: &[u8],
        developer_id: &[u8],
    ) -> bool {
        self.key.iter().any(|value| {
            value.target.as_slice() == target
                && revocation_authorized(
                    policy,
                    REV_KIND_KEY,
                    &value.issuer,
                    app_id,
                    developer_id,
                    value.issued_at,
                )
        })
    }
    fn revoke_app(
        &self,
        target: &[u8; 32],
        policy: &edgerun_wire::TrustPolicy,
        app_id: &[u8],
        developer_id: &[u8],
    ) -> bool {
        revoked_by_authorized_issuer(
            &self.app,
            target,
            policy,
            REV_KIND_APP,
            app_id,
            developer_id,
        )
    }
    fn revoke_product(
        &self,
        target: &[u8; 32],
        policy: &edgerun_wire::TrustPolicy,
        app_id: &[u8],
        developer_id: &[u8],
    ) -> bool {
        revoked_by_authorized_issuer(
            &self.product,
            target,
            policy,
            REV_KIND_PRODUCT,
            app_id,
            developer_id,
        )
    }
    fn revoke_entitlement(
        &self,
        target: &[u8; 32],
        policy: &edgerun_wire::TrustPolicy,
        app_id: &[u8],
        developer_id: &[u8],
    ) -> bool {
        revoked_by_authorized_issuer(
            &self.entitlement,
            target,
            policy,
            REV_KIND_ENTITLEMENT,
            app_id,
            developer_id,
        )
    }
    pub(crate) fn revoke_payment(
        &self,
        target: &[u8; 32],
        policy: &edgerun_wire::TrustPolicy,
        app_id: &[u8],
        developer_id: &[u8],
    ) -> bool {
        revoked_by_authorized_issuer(
            &self.payment,
            target,
            policy,
            REV_KIND_PAYMENT,
            app_id,
            developer_id,
        )
    }
    fn revoke_settlement(
        &self,
        target: &[u8; 32],
        policy: &edgerun_wire::TrustPolicy,
        app_id: &[u8],
        developer_id: &[u8],
    ) -> bool {
        revoked_by_authorized_issuer(
            &self.settlement,
            target,
            policy,
            REV_KIND_SETTLEMENT,
            app_id,
            developer_id,
        )
    }
}

pub(crate) fn revoked_by_authorized_issuer(
    values: &[RevokedTarget],
    target: &[u8; 32],
    policy: &edgerun_wire::TrustPolicy,
    revocation_kind: u16,
    app_id: &[u8],
    developer_id: &[u8],
) -> bool {
    values.iter().any(|value| {
        &value.target == target
            && revocation_authorized(
                policy,
                revocation_kind,
                &value.issuer,
                app_id,
                developer_id,
                value.issued_at,
            )
    })
}

pub(crate) fn revocation_authorized(
    policy: &edgerun_wire::TrustPolicy,
    revocation_kind: u16,
    issuer: &[u8],
    app_id: &[u8],
    developer_id: &[u8],
    issued_at: u64,
) -> bool {
    let roles: &[u16] = match revocation_kind {
        REV_KIND_KEY => &[
            TRUST_ROLE_DEVELOPER,
            TRUST_ROLE_APP_STORE,
            TRUST_ROLE_PRODUCT_STORE,
            TRUST_ROLE_ENTITLEMENT_STORE,
            TRUST_ROLE_PAYMENT_STORE,
            TRUST_ROLE_SETTLEMENT_STORE,
        ],
        REV_KIND_APP => &[TRUST_ROLE_DEVELOPER, TRUST_ROLE_APP_STORE],
        REV_KIND_PRODUCT => &[TRUST_ROLE_DEVELOPER, TRUST_ROLE_PRODUCT_STORE],
        REV_KIND_ENTITLEMENT => &[TRUST_ROLE_ENTITLEMENT_STORE],
        REV_KIND_PAYMENT => &[TRUST_ROLE_PAYMENT_STORE, TRUST_ROLE_PAYER],
        REV_KIND_SETTLEMENT => &[TRUST_ROLE_SETTLEMENT_STORE],
        _ => &[],
    };
    roles
        .iter()
        .any(|role| trust_allows(policy, *role, issuer, app_id, developer_id, issued_at))
}

pub(crate) fn read_revocations<'a>(paths: impl Iterator<Item = &'a String>) -> RevocationSet {
    let mut set = RevocationSet::default();
    for path in paths {
        let Ok(bytes) = fs::read(path) else {
            continue;
        };
        let Some(revocation) = parse_revocation_record(&bytes) else {
            continue;
        };
        if !verify_revocation_signature(&revocation) {
            continue;
        }
        let revoked = RevokedTarget {
            target: revocation.target,
            issuer: revocation.issuer,
            issued_at: revocation.issued_at,
        };
        match revocation.kind {
            REV_KIND_KEY => set.key.push(revoked),
            REV_KIND_APP => set.app.push(revoked),
            REV_KIND_PRODUCT => set.product.push(revoked),
            REV_KIND_ENTITLEMENT => set.entitlement.push(revoked),
            REV_KIND_PAYMENT => set.payment.push(revoked),
            REV_KIND_SETTLEMENT => set.settlement.push(revoked),
            _ => {}
        }
    }
    set
}

pub(crate) struct PaymentIntentInput<'a> {
    pub(crate) purpose: u16,
    pub(crate) amount_minor: u64,
    pub(crate) created_at: u64,
    pub(crate) app_id: &'a [u8],
    pub(crate) release_id: &'a [u8; 32],
    pub(crate) payer_id: &'a [u8; 32],
    pub(crate) payee_id: &'a [u8; 32],
    pub(crate) context_sha256: &'a [u8; 32],
    pub(crate) currency: &'a [u8],
    pub(crate) payer_key: &'a SigningKey,
}

pub(crate) fn payment_intent_bytes(input: PaymentIntentInput<'_>) -> Vec<u8> {
    let mut record = edgerun_wire::PaymentRecord {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        purpose: input.purpose,
        amount_minor: input.amount_minor,
        created_at: input.created_at,
        settled_at: 0,
        app_id: input
            .app_id
            .try_into()
            .expect("payment app id must be 32 bytes"),
        release_id: *input.release_id,
        payer_id: *input.payer_id,
        payee_id: *input.payee_id,
        context_sha256: *input.context_sha256,
        rail_ref_sha256: [0; 32],
        currency: input.currency.to_vec(),
        rail_kind: Vec::new(),
        payer_signature: Vec::new(),
        store_id: Vec::new(),
        store_signature: Vec::new(),
    };
    let signature = input.payer_key.sign(&signature_payload_for_domain(
        EPAY_PAYER_DOMAIN,
        &sha256(&payment_intent_unsigned_bytes(&record)),
    ));
    record.payer_signature = signature.to_bytes().to_vec();
    sdk_wire_record_bytes(SdkWireRecord::Payment(record))
}

pub(crate) fn payment_intent_unsigned_bytes(payment: &edgerun_wire::PaymentRecord) -> Vec<u8> {
    let mut unsigned = payment.clone();
    unsigned.flags = 1;
    unsigned.settled_at = 0;
    unsigned.rail_ref_sha256 = [0; 32];
    unsigned.rail_kind.clear();
    unsigned.payer_signature.clear();
    unsigned.store_id.clear();
    unsigned.store_signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::Payment(unsigned))
}

pub(crate) struct PaymentSettlementInput<'a> {
    pub(crate) settled_at: u64,
    pub(crate) store_id: &'a [u8],
    pub(crate) rail_ref_sha256: &'a [u8; 32],
    pub(crate) rail_kind: &'a [u8],
    pub(crate) store_key: &'a SigningKey,
}

pub(crate) fn settle_payment_bytes(
    intent: &edgerun_wire::PaymentRecord,
    input: PaymentSettlementInput<'_>,
) -> Vec<u8> {
    let mut record = edgerun_wire::PaymentRecord {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 3,
        purpose: intent.purpose,
        amount_minor: intent.amount_minor,
        created_at: intent.created_at,
        settled_at: input.settled_at,
        app_id: intent.app_id,
        release_id: intent.release_id,
        payer_id: intent.payer_id,
        payee_id: intent.payee_id,
        context_sha256: intent.context_sha256,
        rail_ref_sha256: *input.rail_ref_sha256,
        currency: intent.currency.clone(),
        rail_kind: input.rail_kind.to_vec(),
        payer_signature: intent.payer_signature.clone(),
        store_id: input.store_id.to_vec(),
        store_signature: Vec::new(),
    };
    let signature = input.store_key.sign(&signature_payload_for_domain(
        EPAY_STORE_DOMAIN,
        &sha256(&payment_store_unsigned_bytes(&record)),
    ));
    record.store_signature = signature.to_bytes().to_vec();
    sdk_wire_record_bytes(SdkWireRecord::Payment(record))
}

pub(crate) fn payment_store_unsigned_bytes(payment: &edgerun_wire::PaymentRecord) -> Vec<u8> {
    let mut unsigned = payment.clone();
    unsigned.store_signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::Payment(unsigned))
}

pub(crate) fn verify_payment_payer_signature(payment: &edgerun_wire::PaymentRecord) -> bool {
    verify_ed25519_signature(
        &payment.payer_id,
        EPAY_PAYER_DOMAIN,
        &sha256(&payment_intent_unsigned_bytes(payment)),
        &payment.payer_signature,
    )
}

pub(crate) fn verify_payment_store_signature(payment: &edgerun_wire::PaymentRecord) -> bool {
    if payment.store_id.len() != 32 {
        return false;
    }
    let Ok(public_key_bytes) = <[u8; 32]>::try_from(payment.store_id.as_slice()) else {
        return false;
    };
    let Ok(signature_bytes) = <[u8; 64]>::try_from(payment.store_signature.as_slice()) else {
        return false;
    };
    verify_ed25519_signature(
        &public_key_bytes,
        EPAY_STORE_DOMAIN,
        &sha256(&payment_store_unsigned_bytes(payment)),
        &signature_bytes,
    )
}
