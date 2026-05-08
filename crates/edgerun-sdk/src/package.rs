use super::*;

pub(crate) fn cmd_build_artifacts() -> i32 {
    match build_artifacts() {
        Ok(paths) => {
            println!("built {} artifacts", paths.len());
            for path in paths {
                println!("  {}", path.display());
            }
            0
        }
        Err(err) => {
            eprintln!("build-artifacts failed: {err}");
            1
        }
    }
}

pub(crate) fn cmd_generate_unit_metadata(id: Option<&str>) -> i32 {
    let Some(id) = id else {
        eprintln!("generate-unit-metadata requires a unit id");
        return 1;
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = match discover_rust_unit_sources(&root).and_then(|sources| {
        sources
            .into_iter()
            .find(|source| source.id == id)
            .ok_or_else(|| format!("unknown Rust unit source: {id}"))
    }) {
        Ok(source) => source,
        Err(err) => {
            eprintln!("{err}");
            return 1;
        }
    };
    if let Err(err) = compile_rust_unit_source(&root, &source) {
        eprintln!("generate-unit-metadata failed: {err}");
        return 1;
    }
    match generate_rust_unit_metadata(&root, &source) {
        Ok((manifest_path, api_path, wasm_sha256)) => {
            println!("unit: {}", source.id);
            println!("wasm_sha256: {wasm_sha256}");
            println!("manifest: {}", manifest_path.display());
            println!("api: {}", api_path.display());
            0
        }
        Err(err) => {
            eprintln!("generate-unit-metadata failed: {err}");
            1
        }
    }
}

pub(crate) fn cmd_bench_unit(args: Vec<String>) -> i32 {
    let unit_id = args.first().map(String::as_str).unwrap_or("sha256-fips180");
    if unit_id != "sha256-fips180" {
        eprintln!("bench-unit currently supports sha256-fips180");
        return 1;
    }
    let bytes = match args.get(1) {
        Some(value) => match value.parse::<usize>() {
            Ok(value) => value,
            Err(_) => {
                eprintln!("bad byte length: {value}");
                return 1;
            }
        },
        None => 65_536,
    };
    let iterations = match args.get(2) {
        Some(value) => match value.parse::<usize>() {
            Ok(value) => value,
            Err(_) => {
                eprintln!("bad iteration count: {value}");
                return 1;
            }
        },
        None => 1_000,
    };
    match bench_sha256_unit(bytes, iterations) {
        Ok(result) => {
            println!("unit: sha256-fips180");
            println!("bytes_per_call: {}", result.bytes);
            println!("iterations: {}", result.iterations);
            println!(
                "wasmtime_system_invoke_ns_per_call: {:.1}",
                result.wasmtime_invoke_ns
            );
            println!(
                "wasmtime_system_compile_ns_per_call: {:.1}",
                result.wasmtime_compile_ns
            );
            println!("native_dylib_ns_per_call: {:.1}", result.native_dylib_ns);
            println!(
                "native_dylib_vs_wasmtime_invoke: {:.2}x",
                result.native_dylib_ns / result.wasmtime_invoke_ns
            );
            println!("checksum: {}", result.checksum);
            0
        }
        Err(err) => {
            eprintln!("bench-unit failed: {err}");
            1
        }
    }
}

pub(crate) fn cmd_package_app(args: Vec<String>) -> i32 {
    let app_id = args
        .first()
        .map(String::as_str)
        .unwrap_or("hmac-sha256-rfc2104-composed");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("dist").join(format!("{app_id}-app")));
    let result = match app_id {
        "edgerun-wallet-app" => package_wallet_browser_app(&out_dir),
        "edgerun-wallet-runtime" => package_wallet_runtime_app(&out_dir),
        "edgerun-exchange-api" => {
            let host = args
                .get(2)
                .map(String::as_str)
                .unwrap_or("dash.edgerun.tech");
            package_exchange_api_runtime_app(&out_dir, host.as_bytes())
        }
        "sha256-fips180" => {
            build_artifacts().and_then(|_| package_sha256_browser_app(&root, &out_dir))
        }
        "hmac-sha256-rfc2104-composed" => {
            build_artifacts().and_then(|_| package_hmac_browser_app(&root, &out_dir))
        }
        "assemblyscript-compiler" => package_assemblyscript_compiler_browser_app(&out_dir),
        _ => Err(format!("unsupported app package: {app_id}")),
    };
    match result {
        Ok(()) => {
            println!("app: {app_id}");
            println!("path: {}", out_dir.display());
            let entry = out_dir.join("index.html");
            if entry.exists() {
                println!("entry: {}", entry.display());
            }
            println!("artifact_graph: {}", out_dir.join("app.eapp").display());
            0
        }
        Err(err) => {
            eprintln!("package-app failed: {err}");
            1
        }
    }
}

pub(crate) fn package_wallet_runtime_app(out_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|err| err.to_string())?;
    write_rkyv_app_package(
        out_dir,
        RkyvAppSpec {
            slug: "edgerun-wallet-app",
            name: "EdgeRun Wallet",
            version: "0.1.0",
            summary: "Internal wallet and finances app",
            developer_public: [0; 32],
            code_sha256: sha256(b"edgerun-wallet-app:0.1.0"),
            routes: &[],
            storage_namespaces: &[b"edgerun-wallet-app/state"],
        },
    )
}

pub(crate) fn package_wallet_browser_app(out_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|err| err.to_string())?;
    for path in ["index.html", "edgerun-browser.js", "app.edapp"] {
        if !out_dir.join(path).exists() {
            return Err(format!(
                "missing wallet browser asset: {}",
                out_dir.join(path).display()
            ));
        }
    }
    write_packaged_app_graph(out_dir, "edgerun-wallet-app")
}

pub(crate) fn package_exchange_api_runtime_app(out_dir: &Path, host: &[u8]) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|err| err.to_string())?;
    write_rkyv_app_package(
        out_dir,
        RkyvAppSpec {
            slug: "edgerun-exchange-api",
            name: "EdgeRun Exchange API",
            version: "0.1.0",
            summary: "Provider-backed exchange quote and order API",
            developer_public: [0; 32],
            code_sha256: sha256(b"edgerun-exchange-api:0.1.0"),
            routes: &[
                AppRouteSpec {
                    scheme: edgerun_wire::ROUTE_SCHEME_HTTPS,
                    host,
                    path_prefix: b"/v1/quote",
                },
                AppRouteSpec {
                    scheme: edgerun_wire::ROUTE_SCHEME_HTTPS,
                    host,
                    path_prefix: b"/v1/payment-request",
                },
                AppRouteSpec {
                    scheme: edgerun_wire::ROUTE_SCHEME_HTTPS,
                    host,
                    path_prefix: b"/v1/order",
                },
                AppRouteSpec {
                    scheme: edgerun_wire::ROUTE_SCHEME_HTTPS,
                    host,
                    path_prefix: b"/v1/assets",
                },
                AppRouteSpec {
                    scheme: edgerun_wire::ROUTE_SCHEME_HTTPS,
                    host,
                    path_prefix: b"/health",
                },
            ],
            storage_namespaces: &[b"edgerun-exchange-api/state"],
        },
    )
}

pub(crate) struct RkyvAppSpec<'a> {
    pub(crate) slug: &'a str,
    pub(crate) name: &'a str,
    pub(crate) version: &'a str,
    pub(crate) summary: &'a str,
    pub(crate) developer_public: [u8; 32],
    pub(crate) code_sha256: [u8; 32],
    pub(crate) routes: &'a [AppRouteSpec<'a>],
    pub(crate) storage_namespaces: &'a [&'a [u8]],
}

pub(crate) struct AppRouteSpec<'a> {
    pub(crate) scheme: u16,
    pub(crate) host: &'a [u8],
    pub(crate) path_prefix: &'a [u8],
}

pub(crate) fn write_rkyv_app_package(out_dir: &Path, spec: RkyvAppSpec<'_>) -> Result<(), String> {
    let app_id = app_id_for(spec.slug, &spec.developer_public);
    let manifest = edgerun_wire::AppManifestRecord {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        app_id,
        developer_id: spec.developer_public,
        app_slug: spec.slug.as_bytes().to_vec(),
        name: spec.name.as_bytes().to_vec(),
        version: spec.version.as_bytes().to_vec(),
        summary: spec.summary.as_bytes().to_vec(),
        code_sha256: spec.code_sha256,
        routes: spec
            .routes
            .iter()
            .map(|route| edgerun_wire::AppHttpRouteRecord {
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
        provided_capabilities: Vec::new(),
        required_capabilities: Vec::new(),
    };
    fs::write(
        out_dir.join("app.edapp"),
        sdk_wire_record_bytes(SdkWireRecord::AppManifest(manifest)),
    )
    .map_err(|err| err.to_string())?;
    write_packaged_app_graph(out_dir, spec.slug)
}

pub(crate) fn package_hmac_browser_app(root: &Path, out_dir: &Path) -> Result<(), String> {
    let composition = composition("hmac-sha256-rfc2104-composed")
        .ok_or_else(|| "missing hmac composition".to_owned())?;
    fs::create_dir_all(out_dir).map_err(|err| err.to_string())?;
    let mut unit_json = Vec::new();
    for component in composition.components {
        let unit = unit(component.unit_id)
            .ok_or_else(|| format!("missing composition unit: {}", component.unit_id))?;
        unit_json.push(package_unit_for_app(root, out_dir, unit)?);
    }
    let app_composition_dir = out_dir.join("compositions/hmac-sha256-rfc2104");
    copy_file(
        root.join(composition.path),
        app_composition_dir.join("compose.edm"),
    )?;
    let composition_bytes = fs::read(root.join(composition.path)).map_err(|err| err.to_string())?;
    let composition_graph = parse_composition(&composition_bytes)
        .ok_or_else(|| format!("bad composition: {}", composition.id))?;
    fs::write(
        out_dir.join("manifest.json"),
        hmac_browser_app_manifest(composition, &composition_graph, &unit_json),
    )
    .map_err(|err| err.to_string())?;
    fs::write(out_dir.join("index.html"), HMAC_BROWSER_INDEX).map_err(|err| err.to_string())?;
    fs::write(
        out_dir.join("edgerun-browser.js"),
        COMPOSITION_BROWSER_RUNNER,
    )
    .map_err(|err| err.to_string())?;
    write_rkyv_app_package(
        out_dir,
        RkyvAppSpec {
            slug: "hmac-sha256-rfc2104-composed",
            name: "HMAC-SHA256 RFC 2104",
            version: "0.1.0",
            summary: "Browser package for the composed HMAC-SHA256 RFC 2104 unit workflow.",
            developer_public: [0; 32],
            code_sha256: sha256(COMPOSITION_BROWSER_RUNNER.as_bytes()),
            routes: &[],
            storage_namespaces: &[],
        },
    )
}

pub(crate) fn package_sha256_browser_app(root: &Path, out_dir: &Path) -> Result<(), String> {
    let unit = unit("sha256-fips180").ok_or_else(|| "missing sha256-fips180 unit".to_owned())?;
    fs::create_dir_all(out_dir).map_err(|err| err.to_string())?;
    let unit_json = package_unit_for_app(root, out_dir, unit)?;
    fs::write(
        out_dir.join("manifest.json"),
        sha256_browser_app_manifest(&unit_json),
    )
    .map_err(|err| err.to_string())?;
    fs::write(out_dir.join("index.html"), SHA256_BROWSER_INDEX).map_err(|err| err.to_string())?;
    fs::write(out_dir.join("edgerun-browser.js"), SHA256_BROWSER_RUNNER)
        .map_err(|err| err.to_string())?;
    write_rkyv_app_package(
        out_dir,
        RkyvAppSpec {
            slug: "sha256-fips180",
            name: "SHA-256 FIPS 180",
            version: "0.1.0",
            summary: "Browser package for the SHA-256 FIPS 180 unit.",
            developer_public: [0; 32],
            code_sha256: sha256(SHA256_BROWSER_RUNNER.as_bytes()),
            routes: &[],
            storage_namespaces: &[],
        },
    )
}

pub(crate) fn package_assemblyscript_compiler_browser_app(out_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|err| err.to_string())?;
    for path in [
        "index.html",
        "edgerun-browser.js",
        "assemblyscript-compiler-worker.js",
    ] {
        if !out_dir.join(path).exists() {
            return Err(format!(
                "missing AS compiler browser asset: {}",
                out_dir.join(path).display()
            ));
        }
    }
    fs::write(
        out_dir.join("manifest.json"),
        ASSEMBLYSCRIPT_COMPILER_APP_MANIFEST,
    )
    .map_err(|err| err.to_string())?;
    let runner = fs::read(out_dir.join("edgerun-browser.js")).map_err(|err| err.to_string())?;
    write_rkyv_app_package(
        out_dir,
        RkyvAppSpec {
            slug: "assemblyscript-compiler",
            name: "AssemblyScript Compiler",
            version: "0.2.0",
            summary: "Browser package for compiling AssemblyScript units.",
            developer_public: [0; 32],
            code_sha256: sha256(&runner),
            routes: &[],
            storage_namespaces: &[],
        },
    )
}

pub(crate) fn package_unit_for_app(
    root: &Path,
    out_dir: &Path,
    unit: &UnitManifest,
) -> Result<String, String> {
    let api_path = root.join(unit.wasm_path).with_file_name("api.edm");
    let api_sha256 = file_sha256_hex(api_path.clone())?;
    let api_bytes = fs::read(&api_path).map_err(|err| err.to_string())?;
    let api = parse_api(&api_bytes).ok_or_else(|| format!("bad api: {}", unit.id))?;
    let api_functions_json = browser_api_functions_json(&api);
    let app_units_dir = out_dir.join("units").join(unit.id);
    copy_file(root.join(unit.wasm_path), app_units_dir.join("unit.wasm"))?;
    copy_file(
        root.join(unit.manifest_path),
        app_units_dir.join("manifest.edm"),
    )?;
    copy_file(api_path, app_units_dir.join("api.edm"))?;

    let mut implementations = vec![format!(
        concat!(
            "        {{\n",
            "          \"target\": \"wasm32-unknown-unknown\",\n",
            "          \"kind\": \"wasm\",\n",
            "          \"path\": \"units/{}/unit.wasm\",\n",
            "          \"sha256\": \"{}\"\n",
            "        }}"
        ),
        unit.id, unit.wasm_sha256
    )];
    if let Some(native) = native_implementation_for_unit(root, unit.id)? {
        copy_file(native.source_path, out_dir.join(&native.app_path))?;
        implementations.push(format!(
            concat!(
                "        {{\n",
                "          \"target\": \"{}\",\n",
                "          \"kind\": \"native-dylib\",\n",
                "          \"path\": \"{}\",\n",
                "          \"sha256\": \"{}\"\n",
                "        }}"
            ),
            native.target,
            native.app_path.display().to_string().replace('\\', "/"),
            native.sha256
        ));
    }

    Ok(format!(
        concat!(
            "    {{\n",
            "      \"id\": \"{}\",\n",
            "      \"standard\": \"{}\",\n",
            "      \"manifest\": \"units/{}/manifest.edm\",\n",
            "      \"api\": \"units/{}/api.edm\",\n",
            "      \"api_sha256\": \"{}\",\n",
            "      \"api_functions\": [\n",
            "{}\n",
            "      ],\n",
            "      \"implementations\": [\n",
            "{}\n",
            "      ]\n",
            "    }}"
        ),
        unit.id,
        unit.standard,
        unit.id,
        unit.id,
        api_sha256,
        api_functions_json,
        implementations.join(",\n")
    ))
}

pub(crate) fn browser_api_functions_json(api: &edgerun_wire::UnitApi) -> String {
    api.functions
        .iter()
        .map(|function| {
            format!(
                "        {{ \"name\": \"{}\", \"params\": {}, \"results\": {}, \"cost_base\": {}, \"cost_per_byte\": {} }}",
                json_escape_bytes(&function.name),
                json_u8_array(&function.params),
                json_u8_array(&function.results),
                function.cost_base,
                function.cost_per_byte
            )
        })
        .collect::<Vec<_>>()
        .join(",\n")
}

pub(crate) fn json_u8_array(bytes: &[u8]) -> String {
    let values = bytes
        .iter()
        .map(|byte| byte.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{values}]")
}

pub(crate) fn json_escape_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &byte in bytes {
        match byte {
            b'"' => out.push_str("\\\""),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7e => out.push(byte as char),
            _ => out.push_str(&format!("\\u{byte:04x}")),
        }
    }
    out
}

pub(crate) const EAPP_DOMAIN_APP_ID: &[u8] = b"edgerun-sdk.eapp.v1.app-id";
pub(crate) const EAPP_DEVELOPER_DOMAIN: &[u8] = b"edgerun-sdk.esig.v1.app.developer";
pub(crate) const EAPP_STORE_DOMAIN: &[u8] = b"edgerun-sdk.esig.v1.app.store";
pub(crate) const EPRD_DEVELOPER_DOMAIN: &[u8] = b"edgerun-sdk.eprd.v1.developer-product";
pub(crate) const EPRD_STORE_DOMAIN: &[u8] = b"edgerun-sdk.eprd.v1.store-product";
pub(crate) const EENT_STORE_DOMAIN: &[u8] = b"edgerun-sdk.eent.v1.store-entitlement";
pub(crate) const ESET_STORE_DOMAIN: &[u8] = b"edgerun-sdk.eset.v1.store-settlement";
pub(crate) const EPAY_PAYER_DOMAIN: &[u8] = b"edgerun-sdk.epay.v1.payer-intent";
pub(crate) const EPAY_STORE_DOMAIN: &[u8] = b"edgerun-sdk.epay.v1.store-settlement";
pub(crate) const EREV_DOMAIN: &[u8] = b"edgerun-sdk.erev.v1.revocation";
pub(crate) const CAPABILITY_RESPONSE_DOMAIN: &[u8] = b"edgerun-sdk.rkyv.v1.capability-response";
pub(crate) const EUPB_DOMAIN: &[u8] = b"edgerun-sdk.eupb.v1.user-profile-body";

pub(crate) fn hmac_browser_app_manifest(
    composition: &CompositionManifest,
    composition_graph: &edgerun_wire::CompositionRecord,
    unit_json: &[String],
) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"format\": \"edgerun-app-v1\",\n",
            "  \"id\": \"hmac-sha256-rfc2104-browser\",\n",
            "  \"name\": \"HMAC-SHA256 RFC 2104\",\n",
            "  \"version\": \"0.1.0\",\n",
            "  \"abi\": \"standard-module-v1\",\n",
            "  \"entry\": {{ \"kind\": \"composition\", \"composition\": \"{}\" }},\n",
            "  \"units\": [\n",
            "{}\n",
            "  ],\n",
            "  \"compositions\": [\n",
            "    {{\n",
            "      \"id\": \"{}\",\n",
            "      \"path\": \"compositions/hmac-sha256-rfc2104/compose.edm\",\n",
            "      \"sha256\": \"{}\",\n",
            "      \"graph\": {}\n",
            "    }}\n",
            "  ]\n",
            "}}\n"
        ),
        composition.id,
        unit_json.join(",\n"),
        composition.id,
        composition.sha256,
        browser_composition_json(composition_graph)
    )
}

pub(crate) fn browser_composition_json(composition: &edgerun_wire::CompositionRecord) -> String {
    let components = composition
        .components
        .iter()
        .map(|component| {
            format!(
                "        {{ \"unitId\": \"{}\", \"wasmSha256\": \"{}\" }}",
                json_escape_bytes(&component.unit_id),
                bytes_to_hex(&component.wasm_sha256)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let steps = composition
        .steps
        .iter()
        .map(|step| {
            format!(
                "        {{ \"opcode\": {}, \"componentIndex\": {}, \"functionIndex\": {}, \"arg0\": {}, \"arg1\": {}, \"arg2\": {}, \"arg3\": {}, \"arg4\": {} }}",
                step.opcode,
                step.component_index,
                step.function_index,
                step.arg0,
                step.arg1,
                step.arg2,
                step.arg3,
                step.arg4
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    format!(
        concat!(
            "{{\n",
            "      \"id\": \"{}\",\n",
            "      \"outputUnit\": \"{}\",\n",
            "      \"components\": [\n",
            "{}\n",
            "      ],\n",
            "      \"steps\": [\n",
            "{}\n",
            "      ]\n",
            "    }}"
        ),
        json_escape_bytes(&composition.id),
        json_escape_bytes(&composition.output_unit),
        components,
        steps
    )
}

pub(crate) fn copy_file(from: PathBuf, to: PathBuf) -> Result<(), String> {
    let parent = to
        .parent()
        .ok_or_else(|| format!("bad destination path: {}", to.display()))?;
    fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    fs::copy(&from, &to)
        .map(|_| ())
        .map_err(|err| format!("cannot copy {} to {}: {err}", from.display(), to.display()))
}

pub(crate) fn file_sha256_hex(path: PathBuf) -> Result<String, String> {
    let bytes = fs::read(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    Ok(String::from_utf8_lossy(&sha256_hex(&bytes)).into_owned())
}

pub(crate) struct NativeImplementation {
    pub(crate) target: &'static str,
    pub(crate) source_path: PathBuf,
    pub(crate) app_path: PathBuf,
    pub(crate) sha256: String,
}

pub(crate) fn native_implementation_for_unit(
    root: &Path,
    unit_id: &str,
) -> Result<Option<NativeImplementation>, String> {
    match unit_id {
        "sha256-fips180" => build_native_sha256_implementation(root),
        _ => Ok(None),
    }
}

pub(crate) fn build_native_sha256_implementation(
    root: &Path,
) -> Result<Option<NativeImplementation>, String> {
    let Some(target) = native_host_target() else {
        return Ok(None);
    };
    let manifest = root.join("units/sha256-fips180/native/rust/Cargo.toml");
    if !manifest.exists() {
        return Ok(None);
    }
    let target_dir = root.join("units/sha256-fips180/native/rust/target");
    let status = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target-dir")
        .arg(&target_dir)
        .arg("--release")
        .status()
        .map_err(|err| format!("cannot run cargo for sha256-fips180 native: {err}"))?;
    if !status.success() {
        return Err("cargo native build failed for sha256-fips180".to_owned());
    }
    let built = native_build_output_path(&target_dir, target);
    if !built.exists() {
        return Err(format!("native sha256 output missing: {}", built.display()));
    }
    let output = root
        .join("units/sha256-fips180/native")
        .join(target)
        .join(native_unit_library_name());
    copy_file(built, output.clone())?;
    verify_native_sha256_library(&output)?;
    Ok(Some(NativeImplementation {
        target,
        source_path: output,
        app_path: PathBuf::from("units")
            .join("sha256-fips180")
            .join("native")
            .join(target)
            .join(native_unit_library_name()),
        sha256: file_sha256_hex(
            root.join("units/sha256-fips180/native")
                .join(target)
                .join(native_unit_library_name()),
        )?,
    }))
}

pub(crate) fn native_host_target() -> Option<&'static str> {
    match (env::consts::ARCH, env::consts::OS) {
        ("x86_64", "linux") => Some("x86_64-unknown-linux-gnu"),
        ("aarch64", "linux") => Some("aarch64-unknown-linux-gnu"),
        ("x86_64", "macos") => Some("x86_64-apple-darwin"),
        ("aarch64", "macos") => Some("aarch64-apple-darwin"),
        _ => None,
    }
}

pub(crate) fn native_sha256_built_library_name() -> &'static str {
    match env::consts::OS {
        "macos" => "libedgerun_sdk_native_sha256_fips180.dylib",
        _ => "libedgerun_sdk_native_sha256_fips180.so",
    }
}

pub(crate) fn native_build_output_path(target_dir: &Path, target: &str) -> PathBuf {
    let direct = target_dir
        .join("release")
        .join(native_sha256_built_library_name());
    if direct.exists() {
        return direct;
    }
    target_dir
        .join(target)
        .join("release")
        .join(native_sha256_built_library_name())
}

pub(crate) fn native_unit_library_name() -> &'static str {
    match env::consts::OS {
        "macos" => "libunit.dylib",
        _ => "libunit.so",
    }
}

pub(crate) fn native_unit_library_path(root: &Path, unit_id: &str) -> Result<PathBuf, String> {
    let target = native_host_target().ok_or_else(|| "unsupported native host target".to_owned())?;
    let path = root
        .join("units")
        .join(unit_id)
        .join("native")
        .join(target)
        .join(native_unit_library_name());
    if path.exists() {
        return Ok(path);
    }
    build_native_unit_implementation(root, unit_id)?;
    if path.exists() {
        return Ok(path);
    }
    Err(format!(
        "missing native implementation for {unit_id}: {}",
        path.display()
    ))
}

pub(crate) fn build_native_unit_implementation(root: &Path, unit_id: &str) -> Result<(), String> {
    if unit_id == "sha256-fips180"
        && root
            .join("units/sha256-fips180/native/rust/Cargo.toml")
            .exists()
    {
        let _ = build_native_sha256_implementation(root)?;
        return Ok(());
    }
    let target = native_host_target().ok_or_else(|| "unsupported native host target".to_owned())?;
    let manifest = root.join("units").join(unit_id).join("rust/Cargo.toml");
    if !manifest.exists() {
        return Err(format!(
            "missing native Rust source for {unit_id}: {}",
            manifest.display()
        ));
    }
    let target_dir = manifest
        .parent()
        .ok_or_else(|| format!("bad native manifest path: {}", manifest.display()))?
        .join("target-native");
    let status = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target-dir")
        .arg(&target_dir)
        .arg("--release")
        .status()
        .map_err(|err| format!("cannot run cargo for {unit_id} native: {err}"))?;
    if !status.success() {
        return Err(format!("cargo native build failed for {unit_id}"));
    }
    let package = rust_package_name(&manifest)?.unwrap_or_else(|| unit_id.to_owned());
    let built = native_cdylib_output_path(&target_dir, target, &package);
    if !built.exists() {
        return Err(format!(
            "native output missing for {unit_id}: {}",
            built.display()
        ));
    }
    let output = root
        .join("units")
        .join(unit_id)
        .join("native")
        .join(target)
        .join(native_unit_library_name());
    copy_file(built, output)?;
    Ok(())
}

pub(crate) fn native_cdylib_output_path(target_dir: &Path, target: &str, package: &str) -> PathBuf {
    let lib_name = native_cdylib_library_name(package);
    let direct = target_dir.join("release").join(&lib_name);
    if direct.exists() {
        return direct;
    }
    target_dir.join(target).join("release").join(lib_name)
}

pub(crate) fn native_cdylib_library_name(package: &str) -> String {
    let crate_name = package.replace('-', "_");
    match env::consts::OS {
        "macos" => format!("lib{crate_name}.dylib"),
        _ => format!("lib{crate_name}.so"),
    }
}

pub(crate) fn verify_native_sha256_library(path: &Path) -> Result<(), String> {
    unsafe {
        let library = NativeLibrary::open(path)?;
        let abi: unsafe extern "C" fn() -> i32 = library.symbol("proto_abi_version")?;
        let standard: unsafe extern "C" fn() -> i32 = library.symbol("proto_standard_id")?;
        let memory_ptr: unsafe extern "C" fn() -> *mut u8 =
            library.symbol("edgerun_native_memory_ptr")?;
        let memory_len: unsafe extern "C" fn() -> usize =
            library.symbol("edgerun_native_memory_len")?;
        let digest: unsafe extern "C" fn(i32, i32, i32) -> i32 = library.symbol("sha256_digest")?;
        if abi() != 2 || standard() != 180256 || memory_len() < 4096 {
            return Err("native sha256 metadata mismatch".to_owned());
        }
        let memory = core::slice::from_raw_parts_mut(memory_ptr(), memory_len());
        memory[0..3].copy_from_slice(b"abc");
        let status = digest(0, 3, 1024);
        if status != 0 {
            return Err(format!("native sha256 returned status {status}"));
        }
        let actual = bytes_to_hex(&memory[1024..1056]);
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        if actual != expected {
            return Err(format!("native sha256 vector mismatch: {actual}"));
        }
    }
    Ok(())
}

pub(crate) struct NativeLibrary {
    pub(crate) handle: *mut core::ffi::c_void,
}

impl NativeLibrary {
    #[cfg(unix)]
    pub(crate) unsafe fn open(path: &Path) -> Result<Self, String> {
        let path = CString::new(path.as_os_str().as_bytes())
            .map_err(|_| format!("library path contains NUL: {}", path.display()))?;
        let handle = dlopen(path.as_ptr(), RTLD_NOW);
        if handle.is_null() {
            Err(dl_error())
        } else {
            Ok(Self { handle })
        }
    }

    #[cfg(not(unix))]
    pub(crate) unsafe fn open(path: &Path) -> Result<Self, String> {
        let _ = path;
        Err("native library verification requires dlopen-compatible host".to_owned())
    }

    #[cfg(unix)]
    pub(crate) unsafe fn symbol<T: Copy>(&self, name: &str) -> Result<T, String> {
        let name = CString::new(name).map_err(|_| format!("symbol contains NUL: {name}"))?;
        let symbol = dlsym(self.handle, name.as_ptr());
        if symbol.is_null() {
            Err(dl_error())
        } else {
            Ok(core::mem::transmute_copy(&symbol))
        }
    }

    #[cfg(not(unix))]
    pub(crate) unsafe fn symbol<T: Copy>(&self, name: &str) -> Result<T, String> {
        let _ = name;
        Err("native library verification requires dlopen-compatible host".to_owned())
    }
}

impl Drop for NativeLibrary {
    fn drop(&mut self) {
        #[cfg(unix)]
        unsafe {
            let _ = dlclose(self.handle);
        }
    }
}

#[cfg(unix)]
pub(crate) const RTLD_NOW: core::ffi::c_int = 2;

#[cfg(unix)]
#[cfg_attr(target_os = "linux", link(name = "dl"))]
extern "C" {
    fn dlopen(
        filename: *const core::ffi::c_char,
        flags: core::ffi::c_int,
    ) -> *mut core::ffi::c_void;
    fn dlsym(
        handle: *mut core::ffi::c_void,
        symbol: *const core::ffi::c_char,
    ) -> *mut core::ffi::c_void;
    fn dlclose(handle: *mut core::ffi::c_void) -> core::ffi::c_int;
    fn dlerror() -> *const core::ffi::c_char;
    fn mmap(
        addr: *mut core::ffi::c_void,
        len: usize,
        prot: core::ffi::c_int,
        flags: core::ffi::c_int,
        fd: core::ffi::c_int,
        offset: isize,
    ) -> *mut core::ffi::c_void;
    pub(crate) fn munmap(addr: *mut core::ffi::c_void, len: usize) -> core::ffi::c_int;
}

#[cfg(unix)]
pub(crate) unsafe fn dl_error() -> String {
    let err = dlerror();
    if err.is_null() {
        "dynamic loader error".to_owned()
    } else {
        CStr::from_ptr(err).to_string_lossy().into_owned()
    }
}

#[cfg(unix)]
pub(crate) unsafe fn map_low_memory(len: usize) -> Result<*mut u8, String> {
    const PROT_READ: core::ffi::c_int = 0x1;
    const PROT_WRITE: core::ffi::c_int = 0x2;
    const MAP_PRIVATE: core::ffi::c_int = 0x02;
    const MAP_ANON: core::ffi::c_int = 0x20;
    #[cfg(target_arch = "x86_64")]
    const MAP_32BIT: core::ffi::c_int = 0x40;
    #[cfg(not(target_arch = "x86_64"))]
    const MAP_32BIT: core::ffi::c_int = 0x0;

    let ptr = mmap(
        core::ptr::null_mut(),
        len,
        PROT_READ | PROT_WRITE,
        MAP_PRIVATE | MAP_ANON | MAP_32BIT,
        -1,
        0,
    );
    if ptr as isize == -1 {
        return Err("mmap failed for native unit memory".to_owned());
    }
    if u32::try_from(ptr as usize).is_err() {
        let _ = munmap(ptr, len);
        return Err("native unit memory did not map below 4GiB".to_owned());
    }
    Ok(ptr.cast::<u8>())
}

#[cfg(not(unix))]
pub(crate) unsafe fn map_low_memory(_len: usize) -> Result<*mut u8, String> {
    Err("native unit direct-pointer memory requires Unix mmap".to_owned())
}

pub(crate) fn sha256_browser_app_manifest(unit_json: &str) -> String {
    format!(
        concat!(
            "{{\n",
            "  \"format\": \"edgerun-app-v1\",\n",
            "  \"id\": \"sha256-fips180-browser\",\n",
            "  \"name\": \"SHA-256 FIPS 180\",\n",
            "  \"version\": \"0.1.0\",\n",
            "  \"abi\": \"standard-module-v1\",\n",
            "  \"entry\": {{ \"kind\": \"unit\", \"unit\": \"sha256-fips180\", \"function\": \"sha256_digest\" }},\n",
            "  \"units\": [\n",
            "{}\n",
            "  ]\n",
            "}}\n"
        ),
        unit_json
    )
}

pub(crate) const ASSEMBLYSCRIPT_COMPILER_APP_MANIFEST: &str = r#"{
  "format": "edgerun-app-v1",
  "id": "assemblyscript-compiler-browser",
  "name": "AssemblyScript Compiler",
  "version": "0.2.0",
  "abi": "browser-iframe-v1",
  "entry": {
    "kind": "browser-iframe",
    "path": "index.html"
  },
  "developer": {
    "name": "EdgeRun SDK"
  },
  "runtime": {
    "worker": "assemblyscript-compiler-worker.js",
    "compile_mode": "client-worker",
    "compiles": "AssemblyScript",
    "emits": "wasm32-unknown-unknown"
  },
  "capabilities": {
    "required": [
      "browser.worker.execute",
      "browser.wasm.execute"
    ],
    "optional": [
      "browser.storage.local"
    ]
  }
}
"#;

pub(crate) const SHA256_BROWSER_INDEX: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Edgerun SHA-256 Unit</title>
  <style>
    :root { color-scheme: light dark; font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    body { margin: 0; min-height: 100vh; display: grid; place-items: center; background: #f6f7f9; color: #18202a; }
    main { width: min(760px, calc(100vw - 32px)); display: grid; gap: 14px; }
    h1 { margin: 0; font-size: 28px; font-weight: 650; letter-spacing: 0; }
    textarea { width: 100%; min-height: 160px; resize: vertical; box-sizing: border-box; padding: 12px; font: 15px/1.45 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; border: 1px solid #c7ced8; border-radius: 8px; background: #fff; color: #111827; }
    button { width: fit-content; padding: 9px 14px; border: 1px solid #1f6feb; border-radius: 8px; background: #1f6feb; color: #fff; font-weight: 600; cursor: pointer; }
    button:disabled { opacity: .55; cursor: wait; }
    output { display: block; min-height: 24px; overflow-wrap: anywhere; padding: 12px; border: 1px solid #c7ced8; border-radius: 8px; background: #fff; font: 14px/1.45 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
    .meta { font-size: 13px; color: #4b5563; }
    @media (prefers-color-scheme: dark) {
      body { background: #101418; color: #e8edf3; }
      textarea, output { background: #161b22; color: #e8edf3; border-color: #303946; }
      .meta { color: #9aa7b5; }
    }
  </style>
</head>
<body>
  <main>
    <h1>SHA-256 FIPS 180</h1>
    <p class="meta" id="status">Loading Edgerun wasm unit...</p>
    <textarea id="input" spellcheck="false">hello edgerun</textarea>
    <button id="run" disabled>Digest</button>
    <output id="output"></output>
  </main>
  <script type="module" src="./edgerun-browser.js"></script>
</body>
</html>
"#;

pub(crate) const HMAC_BROWSER_INDEX: &str = r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>Edgerun HMAC-SHA256 Composition</title>
  <style>
    :root { color-scheme: light dark; font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    body { margin: 0; min-height: 100vh; display: grid; place-items: center; background: #f6f7f9; color: #18202a; }
    main { width: min(820px, calc(100vw - 32px)); display: grid; gap: 14px; }
    h1 { margin: 0; font-size: 28px; font-weight: 650; letter-spacing: 0; }
    label { display: grid; gap: 6px; font-size: 13px; color: #4b5563; }
    textarea, input { width: 100%; box-sizing: border-box; padding: 12px; font: 15px/1.45 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; border: 1px solid #c7ced8; border-radius: 8px; background: #fff; color: #111827; }
    textarea { min-height: 130px; resize: vertical; }
    button { width: fit-content; padding: 9px 14px; border: 1px solid #1f6feb; border-radius: 8px; background: #1f6feb; color: #fff; font-weight: 600; cursor: pointer; }
    button:disabled { opacity: .55; cursor: wait; }
    output { display: block; min-height: 24px; overflow-wrap: anywhere; padding: 12px; border: 1px solid #c7ced8; border-radius: 8px; background: #fff; font: 14px/1.45 ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
    .meta { font-size: 13px; color: #4b5563; }
    @media (prefers-color-scheme: dark) {
      body { background: #101418; color: #e8edf3; }
      textarea, input, output { background: #161b22; color: #e8edf3; border-color: #303946; }
      label, .meta { color: #9aa7b5; }
    }
  </style>
</head>
<body>
  <main>
    <h1>HMAC-SHA256 RFC 2104</h1>
    <p class="meta" id="status">Loading Edgerun composition...</p>
    <label>Key <input id="key" spellcheck="false" value="Jefe"></label>
    <label>Message <textarea id="message" spellcheck="false">what do ya want for nothing?</textarea></label>
    <button id="run" disabled>Compute</button>
    <output id="output"></output>
  </main>
  <script type="module" src="./edgerun-browser.js"></script>
</body>
</html>
"#;

pub(crate) const SHA256_BROWSER_RUNNER: &str = r#"const statusEl = document.getElementById("status");
const inputEl = document.getElementById("input");
const outputEl = document.getElementById("output");
const runEl = document.getElementById("run");

const textEncoder = new TextEncoder();

function hex(bytes) {
  return [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function sha256(bytes) {
  return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
}

async function fetchBytes(path) {
  const response = await fetch(path);
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

function ensureMemory(memory, needed) {
  const page = 65536;
  if (memory.buffer.byteLength >= needed) return;
  memory.grow(Math.ceil((needed - memory.buffer.byteLength) / page));
}

async function loadApp() {
  const app = await (await fetch("./manifest.json")).json();
  const unit = app.units.find((candidate) => candidate.id === "sha256-fips180");
  if (!unit) throw new Error("sha256-fips180 unit missing from app manifest");
  const implementation = unit.implementations.find((candidate) => candidate.kind === "wasm" && candidate.target === "wasm32-unknown-unknown");
  if (!implementation) throw new Error("sha256-fips180 wasm implementation missing");
  const [wasmBytes, apiBytes] = await Promise.all([fetchBytes(implementation.path), fetchBytes(unit.api)]);
  const apiActual = hex(await sha256(apiBytes));
  if (apiActual !== unit.api_sha256) {
    throw new Error(`api hash mismatch: expected ${unit.api_sha256}, got ${apiActual}`);
  }
  const actual = hex(await sha256(wasmBytes));
  if (actual !== implementation.sha256) {
    throw new Error(`wasm hash mismatch: expected ${implementation.sha256}, got ${actual}`);
  }
  const module = await WebAssembly.instantiate(wasmBytes, {});
  const exports = module.instance.exports;
  if (!exports.memory || typeof exports.sha256_digest !== "function") {
    throw new Error("unit does not expose expected standard-module-v1 API");
  }
  if (exports.proto_abi_version() !== 2) {
    throw new Error(`unsupported unit ABI ${exports.proto_abi_version()}`);
  }
  return { app, unit, implementation, exports };
}

let loaded;

try {
  loaded = await loadApp();
  statusEl.textContent = `Verified ${loaded.unit.id} (${loaded.implementation.sha256})`;
  runEl.disabled = false;
} catch (error) {
  statusEl.textContent = error.message;
  outputEl.value = "";
}

runEl.addEventListener("click", () => {
  if (!loaded) return;
  const input = textEncoder.encode(inputEl.value);
  const outPtr = input.length;
  ensureMemory(loaded.exports.memory, outPtr + 32);
  const memory = new Uint8Array(loaded.exports.memory.buffer);
  memory.set(input, 0);
  const status = loaded.exports.sha256_digest(0, input.length, outPtr);
  if (status !== 0) {
    outputEl.value = `unit failed with status ${status}`;
    return;
  }
  outputEl.value = hex(memory.slice(outPtr, outPtr + 32));
});
"#;

pub(crate) const COMPOSITION_BROWSER_RUNNER: &str = r#"const statusEl = document.getElementById("status");
const keyEl = document.getElementById("key");
const messageEl = document.getElementById("message");
const outputEl = document.getElementById("output");
const runEl = document.getElementById("run");
const textEncoder = new TextEncoder();

function hex(bytes) {
  return [...bytes].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

async function sha256(bytes) {
  return new Uint8Array(await crypto.subtle.digest("SHA-256", bytes));
}

async function fetchBytes(path) {
  const response = await fetch(path);
  if (!response.ok) throw new Error(`${path}: HTTP ${response.status}`);
  return new Uint8Array(await response.arrayBuffer());
}

function ensureMemory(memory, needed) {
  const page = 65536;
  if (memory.buffer.byteLength >= needed) return;
  memory.grow(Math.ceil((needed - memory.buffer.byteLength) / page));
}

function resolveRef(ref, inputs, scalars) {
  const tag = ref >>> 24;
  if (tag === 0) return ref;
  if (tag === 1) return inputs[ref & 0xff].length;
  if (tag === 2) return scalars[ref & 0xff] ?? 0;
  if (tag === 3) return ((ref >>> 8) & 0xffff) + inputs[ref & 0xff].length;
  throw new Error(`unsupported ref ${ref}`);
}

function compareRefs(leftRef, rightRef, inputs, scalars) {
  const comparison = leftRef >>> 28;
  const left = resolveRef(leftRef & 0x0fffffff, inputs, scalars);
  const right = resolveRef(rightRef, inputs, scalars);
  if (comparison === 1) return left > right;
  if (comparison === 2) return left === right;
  if (comparison === 3) return left !== right;
  throw new Error(`unsupported comparison ${comparison}`);
}

async function loadApp() {
  const app = await (await fetch("./manifest.json")).json();
  const units = new Map();
  for (const unit of app.units) {
    const implementation = unit.implementations.find((candidate) => candidate.kind === "wasm" && candidate.target === "wasm32-unknown-unknown");
    if (!implementation) throw new Error(`${unit.id} wasm implementation missing`);
    const [wasmBytes, apiBytes] = await Promise.all([fetchBytes(implementation.path), fetchBytes(unit.api)]);
    const apiActual = hex(await sha256(apiBytes));
    if (apiActual !== unit.api_sha256) {
      throw new Error(`${unit.id} api hash mismatch: expected ${unit.api_sha256}, got ${apiActual}`);
    }
    const actual = hex(await sha256(wasmBytes));
    if (actual !== implementation.sha256) {
      throw new Error(`${unit.id} wasm hash mismatch: expected ${implementation.sha256}, got ${actual}`);
    }
    const instance = (await WebAssembly.instantiate(wasmBytes, {})).instance;
    if (!instance.exports.memory) throw new Error(`${unit.id} missing memory export`);
    if (instance.exports.proto_abi_version() !== 2) throw new Error(`${unit.id} unsupported ABI`);
    units.set(unit.id, { ...unit, implementation, exports: instance.exports, api: unit.api_functions });
  }
  const compositionManifest = app.compositions.find((item) => item.id === app.entry.composition);
  const compositionBytes = await fetchBytes(compositionManifest.path);
  const compositionHash = hex(await sha256(compositionBytes));
  if (compositionHash !== compositionManifest.sha256) {
    throw new Error(`${compositionManifest.id} composition hash mismatch`);
  }
  const composition = compositionManifest.graph;
  for (const component of composition.components) {
    const unit = units.get(component.unitId);
    if (!unit) throw new Error(`component unit missing: ${component.unitId}`);
    if (unit.implementation.sha256 !== component.wasmSha256) throw new Error(`component hash mismatch: ${component.unitId}`);
  }
  return { app, units, composition };
}

function executeComposition(loaded, inputs) {
  const components = loaded.composition.components.map((component) => loaded.units.get(component.unitId));
  const scalars = new Array(256).fill(0);
  let output = new Uint8Array();
  let pc = 0;
  let executed = 0;
  while (pc < loaded.composition.steps.length) {
    if (++executed > loaded.composition.steps.length * 4) throw new Error("composition step limit exceeded");
    const step = loaded.composition.steps[pc];
    const target = components[step.componentIndex];
    switch (step.opcode) {
      case 1: {
        const input = inputs[step.arg0];
        const dst = resolveRef(step.arg1, inputs, scalars);
        const len = resolveRef(step.arg2, inputs, scalars);
        ensureMemory(target.exports.memory, dst + len);
        new Uint8Array(target.exports.memory.buffer).set(input.slice(0, len), dst);
        pc++;
        break;
      }
      case 2: {
        const fn = target.api[step.functionIndex];
        const callable = target.exports[fn.name];
        const args = [step.arg0, step.arg1, step.arg2, step.arg3, step.arg4].map((arg) => resolveRef(arg, inputs, scalars));
        const status = callable(...args.slice(0, fn.params.length));
        if (status !== 0) throw new Error(`${target.id}.${fn.name} failed with status ${status}`);
        pc++;
        break;
      }
      case 3: {
        const source = components[step.arg0];
        const src = resolveRef(step.arg1, inputs, scalars);
        const dst = resolveRef(step.arg2, inputs, scalars);
        const len = resolveRef(step.arg3, inputs, scalars);
        const bytes = new Uint8Array(source.exports.memory.buffer).slice(src, src + len);
        ensureMemory(target.exports.memory, dst + len);
        new Uint8Array(target.exports.memory.buffer).set(bytes, dst);
        pc++;
        break;
      }
      case 4: {
        const src = resolveRef(step.arg1, inputs, scalars);
        const len = resolveRef(step.arg2, inputs, scalars);
        output = new Uint8Array(target.exports.memory.buffer).slice(src, src + len);
        pc++;
        break;
      }
      case 5:
        pc = compareRefs(step.arg0, step.arg1, inputs, scalars) ? step.arg2 : step.arg3;
        break;
      case 6:
        pc = step.arg0;
        break;
      case 7: {
        const ptr = resolveRef(step.arg0, inputs, scalars);
        ensureMemory(target.exports.memory, ptr + 1);
        new Uint8Array(target.exports.memory.buffer)[ptr] = step.arg1 & 0xff;
        pc++;
        break;
      }
      case 8: {
        const fn = target.api[step.functionIndex];
        const callable = target.exports[fn.name];
        const args = [step.arg0, step.arg1, step.arg2, step.arg3].map((arg) => resolveRef(arg, inputs, scalars));
        scalars[step.arg4] = callable(...args.slice(0, fn.params.length)) >>> 0;
        pc++;
        break;
      }
      case 9: {
        const ptr = resolveRef(step.arg0, inputs, scalars);
        const view = new DataView(target.exports.memory.buffer, ptr, 4);
        scalars[step.arg1] = view.getUint32(0, true);
        pc++;
        break;
      }
      default:
        throw new Error(`unknown opcode ${step.opcode}`);
    }
  }
  return output;
}

let loaded;

function run() {
  const key = textEncoder.encode(keyEl.value);
  const message = textEncoder.encode(messageEl.value);
  outputEl.value = hex(executeComposition(loaded, [key, message]));
}

try {
  loaded = await loadApp();
  statusEl.textContent = `Verified ${loaded.composition.id}`;
  runEl.disabled = false;
  run();
} catch (error) {
  statusEl.textContent = error.message;
}

runEl.addEventListener("click", run);
"#;

pub(crate) struct BenchResult {
    pub(crate) bytes: usize,
    pub(crate) iterations: usize,
    pub(crate) wasmtime_invoke_ns: f64,
    pub(crate) wasmtime_compile_ns: f64,
    pub(crate) native_dylib_ns: f64,
    pub(crate) checksum: u8,
}

pub(crate) fn bench_sha256_unit(bytes: usize, iterations: usize) -> Result<BenchResult, String> {
    if bytes == 0 || iterations == 0 {
        return Err("bytes and iterations must be non-zero".to_owned());
    }
    let input: Vec<u8> = (0..bytes)
        .map(|index| (index as u8).wrapping_mul(31).wrapping_add(17))
        .collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut checksum = 0u8;
    let manifest = unit("sha256-fips180").ok_or_else(|| "missing sha256 unit".to_owned())?;
    let wasm_path = root.join(manifest.wasm_path);
    let (wasmtime_invoke_ns, wasmtime_compile_ns, wasmtime_checksum) =
        bench_sha256_system_wasmtime(&wasm_path, bytes, iterations)?;
    checksum ^= wasmtime_checksum;
    let (native_dylib_ns, native_dylib_checksum) =
        bench_sha256_native_dylib(&root, &input, bytes, iterations)?;
    checksum ^= native_dylib_checksum;

    Ok(BenchResult {
        bytes,
        iterations,
        wasmtime_invoke_ns,
        wasmtime_compile_ns,
        native_dylib_ns,
        checksum,
    })
}

pub(crate) fn bench_sha256_system_wasmtime(
    wasm_path: &Path,
    bytes: usize,
    iterations: usize,
) -> Result<(f64, f64, u8), String> {
    let wasmtime = env::var_os("WASMTIME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("wasmtime"));
    run_system_wasmtime_invoke(&wasmtime, wasm_path, bytes)?;

    let start = Instant::now();
    let mut checksum = 0u8;
    for _ in 0..iterations {
        checksum ^= run_system_wasmtime_invoke(&wasmtime, wasm_path, bytes)?;
    }
    let invoke_ns = start.elapsed().as_secs_f64() * 1_000_000_000.0 / iterations as f64;

    let compile_iterations = iterations.min(16);
    let start = Instant::now();
    for _ in 0..compile_iterations {
        run_system_wasmtime_compile(&wasmtime, wasm_path)?;
    }
    let compile_ns = start.elapsed().as_secs_f64() * 1_000_000_000.0 / compile_iterations as f64;
    Ok((invoke_ns, compile_ns, checksum))
}

pub(crate) fn run_system_wasmtime_invoke(
    wasmtime: &Path,
    wasm_path: &Path,
    bytes: usize,
) -> Result<u8, String> {
    let output = Command::new(wasmtime)
        .arg("--invoke")
        .arg("sha256_digest")
        .arg(wasm_path)
        .arg("0")
        .arg(bytes.to_string())
        .arg(bytes.to_string())
        .output()
        .map_err(|err| format!("cannot execute {}: {err}", wasmtime.display()))?;
    if !output.status.success() {
        return Err(system_command_error("wasmtime --invoke", &output));
    }
    Ok(output
        .stdout
        .iter()
        .find(|byte| byte.is_ascii_digit())
        .copied()
        .unwrap_or_default())
}

pub(crate) fn run_system_wasmtime_compile(wasmtime: &Path, wasm_path: &Path) -> Result<(), String> {
    let output = Command::new(wasmtime)
        .arg("compile")
        .arg(wasm_path)
        .arg("-o")
        .arg("/dev/null")
        .output()
        .map_err(|err| format!("cannot execute {}: {err}", wasmtime.display()))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(system_command_error("wasmtime compile", &output))
    }
}

pub(crate) fn system_command_error(command: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    format!(
        "{command} failed with status {}: {}{}",
        output.status,
        stderr.trim(),
        stdout.trim()
    )
}

pub(crate) fn bench_sha256_native_dylib(
    root: &Path,
    input: &[u8],
    bytes: usize,
    iterations: usize,
) -> Result<(f64, u8), String> {
    let native_path = native_unit_library_path(root, "sha256-fips180")?;
    let library = unsafe { NativeLibrary::open(&native_path)? };
    let memory_ptr: unsafe extern "C" fn() -> *mut u8 =
        unsafe { library.symbol("edgerun_native_memory_ptr")? };
    let memory_len_fn: unsafe extern "C" fn() -> usize =
        unsafe { library.symbol("edgerun_native_memory_len")? };
    let digest: unsafe extern "C" fn(i32, i32, i32) -> i32 =
        unsafe { library.symbol("sha256_digest")? };
    let memory = unsafe { memory_ptr() };
    let memory_len = unsafe { memory_len_fn() };
    if memory.is_null() || bytes.checked_add(32).is_none_or(|end| end > memory_len) {
        return Err("native sha256 memory is too small".to_owned());
    }
    unsafe {
        core::slice::from_raw_parts_mut(memory, input.len()).copy_from_slice(input);
        digest(0, bytes as i32, bytes as i32);
    }
    let start = Instant::now();
    let mut checksum = 0u8;
    for _ in 0..iterations {
        let status = unsafe { digest(0, bytes as i32, bytes as i32) };
        if status != 0 {
            return Err(format!("native sha256 returned status {status}"));
        }
        checksum ^= unsafe { *memory.add(bytes) };
    }
    let ns = start.elapsed().as_secs_f64() * 1_000_000_000.0 / iterations as f64;
    Ok((ns, checksum))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_app_dir(name: &str) -> PathBuf {
        let mut path = env::temp_dir();
        path.push(format!(
            "edgerun-sdk-{name}-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time")
                .as_nanos()
        ));
        path
    }

    #[test]
    fn wallet_package_writes_internal_protocol_app_without_routes() {
        let app_dir = temp_app_dir("wallet-package");
        package_wallet_runtime_app(&app_dir).expect("package app");

        let manifest = read_app_manifest_record(&app_dir).expect("read app manifest");
        assert_eq!(manifest.app_slug, b"edgerun-wallet-app");
        assert!(manifest.routes.is_empty());
        assert_eq!(
            manifest.storage_namespaces,
            vec![b"edgerun-wallet-app/state".to_vec()]
        );

        let (_graph_bytes, graph) = read_app_graph(&app_dir).expect("read app graph");
        assert!(verify_packaged_app_graph(&app_dir, &graph));
        assert!(graph.runtime_install.declared_routes.is_empty());
        assert_eq!(
            graph.runtime_install.storage_namespaces,
            vec![b"edgerun-wallet-app/state".to_vec()]
        );

        fs::remove_dir_all(&app_dir).expect("remove temp app dir");
    }

    #[test]
    fn exchange_api_package_writes_rkyv_manifest_and_verifiable_graph() {
        let app_dir = temp_app_dir("exchange-api-package");
        package_exchange_api_runtime_app(&app_dir, b"dash.edgerun.tech").expect("package app");

        assert!(!app_dir.join("index.html").exists());

        let manifest = read_app_manifest_record(&app_dir).expect("read app manifest");
        assert_eq!(manifest.app_slug, b"edgerun-exchange-api");
        assert_eq!(manifest.routes.len(), 5);
        assert!(manifest.routes.iter().any(|route| {
            route.host == b"dash.edgerun.tech" && route.path_prefix == b"/v1/order"
        }));
        assert_eq!(
            manifest.storage_namespaces,
            vec![b"edgerun-exchange-api/state".to_vec()]
        );

        let (_graph_bytes, graph) = read_app_graph(&app_dir).expect("read app graph");
        assert!(verify_packaged_app_graph(&app_dir, &graph));
        assert_eq!(graph.runtime_install.app_id, graph.app_id);
        assert_eq!(
            graph.runtime_install.manifest_sha256,
            graph.app_manifest_sha256
        );
        assert_eq!(graph.runtime_install.declared_routes.len(), 5);
        assert!(graph.runtime_install.declared_routes.iter().any(|route| {
            route.host == b"dash.edgerun.tech" && route.path_prefix == b"/health"
        }));

        fs::remove_dir_all(&app_dir).expect("remove temp app dir");
    }

    #[test]
    fn assemblyscript_compiler_package_writes_browser_manifest_and_graph() {
        let app_dir = temp_app_dir("assemblyscript-compiler-package");
        fs::create_dir_all(&app_dir).expect("create temp app dir");
        fs::write(app_dir.join("index.html"), b"<!doctype html>").expect("write index");
        fs::write(app_dir.join("edgerun-browser.js"), b"export {};").expect("write runner");
        fs::write(
            app_dir.join("assemblyscript-compiler-worker.js"),
            b"self.onmessage=null;",
        )
        .expect("write worker");

        package_assemblyscript_compiler_browser_app(&app_dir).expect("package app");

        let manifest_json =
            fs::read_to_string(app_dir.join("manifest.json")).expect("read browser manifest");
        assert!(manifest_json.contains("\"id\": \"assemblyscript-compiler-browser\""));
        assert!(manifest_json.contains("\"runtime\""));
        let manifest = read_app_manifest_record(&app_dir).expect("read rkyv app manifest");
        assert_eq!(manifest.app_slug, b"assemblyscript-compiler");
        assert_eq!(manifest.name, b"AssemblyScript Compiler");
        assert_eq!(manifest.version, b"0.2.0");

        let (_graph_bytes, graph) = read_app_graph(&app_dir).expect("read app graph");
        assert!(verify_packaged_app_graph(&app_dir, &graph));
        assert_eq!(graph.app_slug, b"assemblyscript-compiler");
        assert!(graph
            .artifacts
            .iter()
            .any(|artifact| artifact.path == b"assemblyscript-compiler-worker.js"));

        fs::remove_dir_all(&app_dir).expect("remove temp app dir");
    }
}
