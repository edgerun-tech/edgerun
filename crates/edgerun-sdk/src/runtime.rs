use super::*;

pub(crate) struct RuntimeIndex {
    pub(crate) units: Vec<UnitManifest>,
    pub(crate) compositions: Vec<CompositionManifest>,
    pub(crate) segments: Vec<SegmentManifest>,
    pub(crate) chains: Vec<ChainManifest>,
}

pub(crate) static INDEX: OnceLock<RuntimeIndex> = OnceLock::new();
pub(crate) fn cmd_verify_segment(id: Option<&str>) -> i32 {
    let segments: Vec<&'static SegmentManifest> = match id {
        Some(id) => match segment(id) {
            Some(segment) => vec![segment],
            None => {
                eprintln!("unknown segment: {id}");
                return 1;
            }
        },
        None => segments().iter().collect(),
    };
    let mut ok = true;
    for segment in segments {
        ok &= verify_segment(segment);
    }
    if ok {
        0
    } else {
        1
    }
}

pub(crate) fn cmd_verify_chain(id: Option<&str>) -> i32 {
    let chains: Vec<&'static ChainManifest> = match id {
        Some(id) => match chain(id) {
            Some(chain) => vec![chain],
            None => {
                eprintln!("unknown chain: {id}");
                return 1;
            }
        },
        None => chains().iter().collect(),
    };
    let mut ok = true;
    for chain in chains {
        ok &= verify_chain(chain);
    }
    if ok {
        0
    } else {
        1
    }
}

pub(crate) fn cmd_verify_chain_reports(args: Vec<String>) -> i32 {
    let Some(id) = args.first() else {
        eprintln!("verify-chain-reports requires a chain id and segment reports");
        return 1;
    };
    let Some(manifest) = chain(id) else {
        eprintln!("unknown chain: {id}");
        return 1;
    };
    if args.len() - 1 != manifest.segment_count as usize {
        eprintln!(
            "chain report count mismatch: expected {}, got {}",
            manifest.segment_count,
            args.len() - 1
        );
        return 1;
    }
    let mut report_bytes = Vec::new();
    for path in &args[1..] {
        let Ok(bytes) = fs::read(path) else {
            eprintln!("cannot read segment report: {path}");
            return 1;
        };
        report_bytes.push(bytes);
    }
    let reports: Option<Vec<edgerun_wire::SegmentReportRecord>> = report_bytes
        .iter()
        .map(|bytes| parse_segment_report(bytes))
        .collect();
    let Some(reports) = reports else {
        eprintln!("invalid segment report in chain");
        return 1;
    };
    let ok = verify_chain_reports(manifest, &reports);
    println!("chain: {}", manifest.id);
    print_check("chain-report-preflight-bindings", ok);
    if ok {
        0
    } else {
        1
    }
}

pub(crate) fn cmd_verify_report(path: Option<&str>) -> i32 {
    let Some(path) = path else {
        eprintln!("verify-report requires a report path");
        return 1;
    };
    let Ok(bytes) = fs::read(path) else {
        eprintln!("cannot read report: {path}");
        return 1;
    };
    let Some(report) = parse_report(&bytes) else {
        eprintln!("invalid report: {path}");
        return 1;
    };
    let Some(id) = core::str::from_utf8(&report.composition_id).ok() else {
        eprintln!("report composition id is not utf-8");
        return 1;
    };
    let Some(manifest) = composition(id) else {
        eprintln!("unknown report composition: {id}");
        return 1;
    };
    let ok = verify_binary_report(manifest, &report);
    println!("report: {path}");
    println!("composition: {}", manifest.id);
    print_check("report-bindings", ok);
    if ok {
        println!("cost: {}", report.cost);
        println!("output_len: {}", report.output_len);
        println!("output_sha256: {}", bytes_to_hex(&report.output_sha256));
        0
    } else {
        1
    }
}

pub(crate) fn cmd_verify_segment_report(path: Option<&str>) -> i32 {
    let Some(path) = path else {
        eprintln!("verify-segment-report requires a report path");
        return 1;
    };
    let Ok(bytes) = fs::read(path) else {
        eprintln!("cannot read segment report: {path}");
        return 1;
    };
    let Some(report) = parse_segment_report(&bytes) else {
        eprintln!("invalid segment report: {path}");
        return 1;
    };
    let Some(id) = core::str::from_utf8(&report.segment_id).ok() else {
        eprintln!("segment report id is not utf-8");
        return 1;
    };
    let Some(manifest) = segment(id) else {
        eprintln!("unknown segment report: {id}");
        return 1;
    };
    let ok = verify_binary_segment_report(manifest, &report);
    println!("segment_report: {path}");
    println!("segment: {}", manifest.id);
    print_check("segment-report-preflight-bindings", ok);
    if ok {
        println!("status: {}", report.status);
        println!("cost: {}", report.cost);
        println!("output_len: {}", report.output_len);
        println!("output_sha256: {}", bytes_to_hex(&report.output_sha256));
        println!("output_replayed: no");
        0
    } else {
        1
    }
}

pub(crate) fn cmd_replay_segment_report(args: Vec<String>) -> i32 {
    let Some(path) = args.first() else {
        eprintln!("replay-segment-report requires a report path and hex inputs");
        return 1;
    };
    let Ok(bytes) = fs::read(path) else {
        eprintln!("cannot read segment report: {path}");
        return 1;
    };
    let Some(report) = parse_segment_report(&bytes) else {
        eprintln!("invalid segment report: {path}");
        return 1;
    };
    let Some(id) = core::str::from_utf8(&report.segment_id).ok() else {
        eprintln!("segment report id is not utf-8");
        return 1;
    };
    let Some(manifest) = segment(id) else {
        eprintln!("unknown segment report: {id}");
        return 1;
    };
    let mut inputs = Vec::new();
    for input in &args[1..] {
        let Some(bytes) = parse_hex(input) else {
            eprintln!("invalid hex input: {input}");
            return 1;
        };
        inputs.push(bytes);
    }
    let ok = replay_binary_segment_report(manifest, &report, &inputs);
    println!("segment_report: {path}");
    println!("segment: {}", manifest.id);
    print_check("segment-report-replay", ok);
    if ok {
        println!("status: {}", report.status);
        println!("cost: {}", report.cost);
        println!("output_len: {}", report.output_len);
        println!("output_sha256: {}", bytes_to_hex(&report.output_sha256));
        0
    } else {
        1
    }
}

pub(crate) fn cmd_sign_segment_report(args: Vec<String>) -> i32 {
    let Some(path) = args.first() else {
        eprintln!("sign-segment-report requires a report path and ed25519 seed hex");
        return 1;
    };
    let Some(seed_hex) = args.get(1) else {
        eprintln!("sign-segment-report requires a report path and ed25519 seed hex");
        return 1;
    };
    let Ok(report_bytes) = fs::read(path) else {
        eprintln!("cannot read segment report: {path}");
        return 1;
    };
    let Some(report) = parse_segment_report(&report_bytes) else {
        eprintln!("invalid segment report: {path}");
        return 1;
    };
    let Some(id) = core::str::from_utf8(&report.segment_id).ok() else {
        eprintln!("segment report id is not utf-8");
        return 1;
    };
    let Some(manifest) = segment(id) else {
        eprintln!("unknown segment report: {id}");
        return 1;
    };
    if !verify_binary_segment_report(manifest, &report) {
        eprintln!("segment report failed preflight verification");
        return 1;
    }
    let Some(seed) = parse_hex(seed_hex).and_then(|bytes| bytes.try_into().ok()) else {
        eprintln!("ed25519 seed must be 32 hex bytes");
        return 1;
    };
    let signing_key = SigningKey::from_bytes(&seed);
    let esig = signature_bytes(&report_bytes, &signing_key);
    let path = PathBuf::from(path).with_extension("esig");
    if let Err(err) = fs::write(&path, esig) {
        eprintln!("cannot write signature: {err}");
        return 1;
    }
    println!("signature: {}", path.display());
    println!(
        "public_key: {}",
        bytes_to_hex(signing_key.verifying_key().as_bytes())
    );
    0
}

pub(crate) fn cmd_write_signer_policy(args: Vec<String>) -> i32 {
    let Some(path) = args.first() else {
        eprintln!("write-signer-policy requires a policy path, segment id, and public keys");
        return 1;
    };
    let Some(segment_id) = args.get(1) else {
        eprintln!("write-signer-policy requires a policy path, segment id, and public keys");
        return 1;
    };
    let Some(manifest) = segment(segment_id) else {
        eprintln!("unknown segment: {segment_id}");
        return 1;
    };
    if args.len() < 3 {
        eprintln!("write-signer-policy requires at least one ed25519 public key");
        return 1;
    }
    let mut public_keys = Vec::new();
    for public_key_hex in &args[2..] {
        let Some(public_key) = parse_hex(public_key_hex).filter(|bytes| bytes.len() == 32) else {
            eprintln!("ed25519 public keys must be 32 hex bytes");
            return 1;
        };
        public_keys.push(public_key);
    }
    let Ok(policy) = signer_policy_bytes(manifest, &public_keys) else {
        eprintln!("cannot build signer policy");
        return 1;
    };
    if let Err(err) = fs::write(path, policy) {
        eprintln!("cannot write signer policy: {err}");
        return 1;
    }
    println!("signer_policy: {path}");
    println!("segment: {}", manifest.id);
    println!("authorized_keys: {}", public_keys.len());
    0
}

pub(crate) fn cmd_verify_signed_segment_report(args: Vec<String>) -> i32 {
    let Some(report_path) = args.first() else {
        eprintln!("verify-signed-segment-report requires report and signature paths");
        return 1;
    };
    let Some(signature_path) = args.get(1) else {
        eprintln!("verify-signed-segment-report requires report and signature paths");
        return 1;
    };
    let Ok(report_bytes) = fs::read(report_path) else {
        eprintln!("cannot read segment report: {report_path}");
        return 1;
    };
    let Ok(signature_bytes) = fs::read(signature_path) else {
        eprintln!("cannot read segment signature: {signature_path}");
        return 1;
    };
    let Some(report) = parse_segment_report(&report_bytes) else {
        eprintln!("invalid segment report: {report_path}");
        return 1;
    };
    let Some(signature) = parse_artifact_signature_record(&signature_bytes) else {
        eprintln!("invalid segment signature: {signature_path}");
        return 1;
    };
    let Some(id) = core::str::from_utf8(&report.segment_id).ok() else {
        eprintln!("segment report id is not utf-8");
        return 1;
    };
    let Some(manifest) = segment(id) else {
        eprintln!("unknown segment report: {id}");
        return 1;
    };
    let report_ok = verify_binary_segment_report(manifest, &report);
    let signature_ok = verify_binary_signature(&report_bytes, &signature);
    let policy_ok = if let Some(policy_path) = args.get(2) {
        let Ok(policy_bytes) = fs::read(policy_path) else {
            eprintln!("cannot read signer policy: {policy_path}");
            return 1;
        };
        let Some(policy) = parse_signer_policy_record(&policy_bytes) else {
            eprintln!("invalid signer policy: {policy_path}");
            return 1;
        };
        Some(verify_binary_signer_policy(manifest, &signature, &policy))
    } else {
        None
    };
    println!("segment_report: {report_path}");
    println!("signature: {signature_path}");
    println!("segment: {}", manifest.id);
    print_check("segment-report-preflight-bindings", report_ok);
    print_check("segment-report-signature", signature_ok);
    if let Some(policy_ok) = policy_ok {
        print_check("segment-report-signer-policy", policy_ok);
    }
    if report_ok && signature_ok && policy_ok.unwrap_or(true) {
        println!("public_key: {}", bytes_to_hex(&signature.public_key));
        println!(
            "report_sha256: {}",
            bytes_to_hex(&signature.artifact_sha256)
        );
        0
    } else {
        1
    }
}

pub(crate) fn cmd_quote_composition(args: Vec<String>) -> i32 {
    let Some(id) = args.first() else {
        eprintln!("quote-composition requires a composition id");
        return 1;
    };
    let Some(manifest) = composition(id) else {
        eprintln!("unknown composition: {id}");
        return 1;
    };
    let mut input_lengths = Vec::new();
    for input in &args[1..] {
        let Ok(len) = input.parse::<u32>() else {
            eprintln!("invalid input length: {input}");
            return 1;
        };
        input_lengths.push(len);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let composition_path = root.join(manifest.path);
    let Ok(bytes) = fs::read(&composition_path) else {
        eprintln!("cannot read composition: {}", composition_path.display());
        return 1;
    };
    let Some(parsed) = parse_composition(&bytes) else {
        eprintln!("invalid composition: {}", manifest.path);
        return 1;
    };
    if !verify_binary_composition(manifest, &bytes) {
        eprintln!("composition failed verification: {}", manifest.id);
        return 1;
    }
    let quote = match quote_composition(manifest, &parsed, &input_lengths) {
        Ok(quote) => quote,
        Err(err) => {
            eprintln!("composition quote failed: {err}");
            return 1;
        }
    };
    println!("composition: {}", manifest.id);
    print!("input_lengths:");
    for len in &input_lengths {
        print!(" {len}");
    }
    println!();
    println!("cost: {}", quote.cost);
    println!("output_len: {}", quote.output_len);
    println!("steps_executed: {}", quote.steps_executed);
    0
}

pub(crate) fn cmd_preflight_composition(args: Vec<String>) -> i32 {
    let Some(id) = args.first() else {
        eprintln!("preflight-composition requires a composition id");
        return 1;
    };
    let Some(manifest) = composition(id) else {
        eprintln!("unknown composition: {id}");
        return 1;
    };
    let mut input_lengths = Vec::new();
    for input in &args[1..] {
        let Ok(len) = input.parse::<u32>() else {
            eprintln!("invalid input length: {input}");
            return 1;
        };
        input_lengths.push(len);
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let composition_path = root.join(manifest.path);
    let Ok(bytes) = fs::read(&composition_path) else {
        eprintln!("cannot read composition: {}", composition_path.display());
        return 1;
    };
    let Some(parsed) = parse_composition(&bytes) else {
        eprintln!("invalid composition: {}", manifest.path);
        return 1;
    };
    if !verify_binary_composition(manifest, &bytes) {
        eprintln!("composition failed verification: {}", manifest.id);
        return 1;
    }
    let report = match preflight_composition(manifest, &parsed, &input_lengths) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("composition preflight failed: {err}");
            return 1;
        }
    };
    println!("composition: {}", manifest.id);
    print!("input_lengths:");
    for len in &input_lengths {
        print!(" {len}");
    }
    println!();
    println!("shape: ok");
    println!("cost_min: {}", report.cost_min);
    println!("cost_max: {}", report.cost_max);
    match report.output_len {
        Some(len) => println!("output_len: {len}"),
        None => println!("output_len: unknown"),
    }
    println!("steps_min: {}", report.steps_min);
    println!("steps_max: {}", report.steps_max);
    if !report.unknowns.is_empty() {
        println!("unknowns:");
        for item in &report.unknowns {
            println!("  {item}");
        }
    }
    if !report.possible_failures.is_empty() {
        println!("possible_failures:");
        for item in &report.possible_failures {
            println!("  {item}");
        }
    }
    0
}

pub(crate) fn cmd_run_segment(args: Vec<String>) -> i32 {
    let Some(id) = args.first() else {
        eprintln!("run-segment requires a segment id");
        return 1;
    };
    let Some(segment_manifest) = segment(id) else {
        eprintln!("unknown segment: {id}");
        return 1;
    };
    if !verify_segment(segment_manifest) {
        eprintln!("segment failed verification: {}", segment_manifest.id);
        return 1;
    }
    let Some(composition_manifest) = composition(segment_manifest.composition_id) else {
        eprintln!(
            "unknown segment composition: {}",
            segment_manifest.composition_id
        );
        return 1;
    };
    let mut inputs = Vec::new();
    for input in &args[1..] {
        let Some(bytes) = parse_hex(input) else {
            eprintln!("invalid hex input: {input}");
            return 1;
        };
        inputs.push(bytes);
    }
    if inputs.len() != segment_manifest.input_count as usize {
        eprintln!(
            "segment input count mismatch: expected {}, got {}",
            segment_manifest.input_count,
            inputs.len()
        );
        return 1;
    }
    let input_refs: Vec<&[u8]> = inputs.iter().map(Vec::as_slice).collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let composition_path = root.join(composition_manifest.path);
    let Ok(bytes) = fs::read(&composition_path) else {
        eprintln!("cannot read composition: {}", composition_path.display());
        return 1;
    };
    let Some(parsed) = parse_composition(&bytes) else {
        eprintln!("invalid composition: {}", composition_manifest.path);
        return 1;
    };
    let report = match execute_composition(composition_manifest, &parsed, &input_refs) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("segment execution failed: {err}");
            return 1;
        }
    };
    let output_sha256 = sha256_hex(&report.output);
    let segment_report_path =
        match write_segment_report(segment_manifest, &inputs, &report, &output_sha256) {
            Ok(path) => path,
            Err(err) => {
                eprintln!("cannot write segment report: {err}");
                return 1;
            }
        };
    println!("segment: {}", segment_manifest.id);
    println!("composition: {}", composition_manifest.id);
    println!("node_role: {}", segment_manifest.node_role);
    print!("input_lengths:");
    for input in &inputs {
        print!(" {}", input.len());
    }
    println!();
    println!("cost: {}", report.cost);
    println!("output: {}", bytes_to_hex(&report.output));
    println!("output_sha256: {}", String::from_utf8_lossy(&output_sha256));
    println!("segment_report: {}", segment_report_path.display());
    0
}

pub(crate) fn cmd_run_composition(args: Vec<String>) -> i32 {
    let Some(id) = args.first() else {
        eprintln!("run-composition requires a composition id");
        return 1;
    };
    let Some(manifest) = composition(id) else {
        eprintln!("unknown composition: {id}");
        return 1;
    };
    let mut inputs = Vec::new();
    for input in &args[1..] {
        let Some(bytes) = parse_hex(input) else {
            eprintln!("invalid hex input: {input}");
            return 1;
        };
        inputs.push(bytes);
    }
    let input_refs: Vec<&[u8]> = inputs.iter().map(Vec::as_slice).collect();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let composition_path = root.join(manifest.path);
    let Ok(bytes) = fs::read(&composition_path) else {
        eprintln!("cannot read composition: {}", composition_path.display());
        return 1;
    };
    let Some(parsed) = parse_composition(&bytes) else {
        eprintln!("invalid composition: {}", manifest.path);
        return 1;
    };
    if !verify_binary_composition(manifest, &bytes) {
        eprintln!("composition failed verification: {}", manifest.id);
        return 1;
    }
    let report = match execute_composition(manifest, &parsed, &input_refs) {
        Ok(report) => report,
        Err(err) => {
            eprintln!("composition execution failed: {err}");
            return 1;
        }
    };
    println!("composition: {}", manifest.id);
    println!("output_unit: {}", manifest.output_unit);
    println!("components:");
    for component in manifest.components {
        println!("  {} {}", component.unit_id, component.wasm_sha256);
    }
    print!("input_lengths:");
    for input in &inputs {
        print!(" {}", input.len());
    }
    println!();
    println!("cost: {}", report.cost);
    println!("output: {}", bytes_to_hex(&report.output));
    let output_sha256 = sha256_hex(&report.output);
    println!("output_sha256: {}", String::from_utf8_lossy(&output_sha256));
    match write_execution_report(manifest, &inputs, &report, &output_sha256) {
        Ok(path) => println!("report: {}", path.display()),
        Err(err) => {
            eprintln!("cannot write execution report: {err}");
            return 1;
        }
    }
    0
}

pub(crate) fn runtime_index() -> &'static RuntimeIndex {
    INDEX.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        load_runtime_index(&root).expect("edgerun-sdk runtime index")
    })
}

pub(crate) fn units() -> &'static [UnitManifest] {
    &runtime_index().units
}

pub(crate) fn compositions() -> &'static [CompositionManifest] {
    &runtime_index().compositions
}

pub(crate) fn segments() -> &'static [SegmentManifest] {
    &runtime_index().segments
}

pub(crate) fn chains() -> &'static [ChainManifest] {
    &runtime_index().chains
}

pub(crate) fn unit(id: &str) -> Option<&'static UnitManifest> {
    units().iter().find(|unit| unit.id == id)
}

pub(crate) fn composition(id: &str) -> Option<&'static CompositionManifest> {
    compositions()
        .iter()
        .find(|composition| composition.id == id)
}

pub(crate) fn segment(id: &str) -> Option<&'static SegmentManifest> {
    segments().iter().find(|segment| segment.id == id)
}

pub(crate) fn chain(id: &str) -> Option<&'static ChainManifest> {
    chains().iter().find(|chain| chain.id == id)
}

pub(crate) fn build_artifacts() -> Result<Vec<PathBuf>, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();

    for source in discover_rust_unit_sources(&root)? {
        compile_rust_unit_source(&root, &source)?;
        let (manifest_path, api_path, _wasm_sha256) = generate_rust_unit_metadata(&root, &source)?;
        paths.push(manifest_path);
        paths.push(api_path);
    }

    for manifest in compositions() {
        let bytes = composition_manifest_bytes(manifest)?;
        let actual = String::from_utf8_lossy(&sha256_hex(&bytes)).into_owned();
        if actual != manifest.sha256 {
            return Err(format!(
                "{} composition hash drift: index {}, built {}",
                manifest.id, manifest.sha256, actual
            ));
        }
        let path = root.join(manifest.path);
        fs::write(&path, bytes).map_err(|err| err.to_string())?;
        paths.push(path_relative_to(&root, &path));
    }

    for manifest in segments() {
        let bytes = segment_manifest_bytes(manifest)?;
        let actual = String::from_utf8_lossy(&sha256_hex(&bytes)).into_owned();
        if actual != manifest.sha256 {
            return Err(format!(
                "{} segment hash drift: index {}, built {}",
                manifest.id, manifest.sha256, actual
            ));
        }
        let path = root.join(manifest.path);
        fs::write(&path, bytes).map_err(|err| err.to_string())?;
        paths.push(path_relative_to(&root, &path));
    }

    for manifest in chains() {
        let bytes = chain_manifest_bytes(manifest)?;
        let actual = String::from_utf8_lossy(&sha256_hex(&bytes)).into_owned();
        if actual != manifest.sha256 {
            return Err(format!(
                "{} chain hash drift: index {}, built {}",
                manifest.id, manifest.sha256, actual
            ));
        }
        let path = root.join(manifest.path);
        fs::write(&path, bytes).map_err(|err| err.to_string())?;
        paths.push(path_relative_to(&root, &path));
    }

    Ok(paths)
}

#[derive(Clone)]
pub(crate) struct RustUnitSource {
    pub(crate) id: String,
    pub(crate) standard: String,
    pub(crate) standard_id: i32,
    pub(crate) rust_manifest: PathBuf,
    pub(crate) rust_source: PathBuf,
    pub(crate) wasm_path: PathBuf,
    pub(crate) manifest_path: PathBuf,
}

pub(crate) fn discover_rust_unit_sources(root: &Path) -> Result<Vec<RustUnitSource>, String> {
    let units_dir = root.join("units");
    let mut sources = Vec::new();
    for entry in fs::read_dir(&units_dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let rust_manifest = path.join("rust/Cargo.toml");
        if !rust_manifest.exists() {
            continue;
        }
        let metadata = rust_unit_metadata(&rust_manifest)?;
        let dir_id = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| format!("bad unit dir: {}", path.display()))?;
        if metadata.id != dir_id {
            return Err(format!(
                "{} metadata id mismatch: expected {dir_id}, got {}",
                rust_manifest.display(),
                metadata.id
            ));
        }
        let source = RustUnitSource {
            id: metadata.id,
            standard: metadata.standard,
            standard_id: read_rust_unit_standard_id(&path.join("rust/src/lib.rs"))?,
            rust_source: path.join("rust/src/lib.rs"),
            rust_manifest,
            wasm_path: path.join("unit.wasm"),
            manifest_path: path.join("manifest.edm"),
        };
        validate_rust_unit_source(&source)?;
        sources.push(source);
    }
    sources.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(sources)
}

pub(crate) struct RustUnitMetadata {
    pub(crate) id: String,
    pub(crate) standard: String,
}

pub(crate) fn load_runtime_index(root: &Path) -> Result<RuntimeIndex, String> {
    let sources = discover_rust_unit_sources(root)?;
    let mut units = Vec::with_capacity(sources.len());
    for source in &sources {
        units.push(load_runtime_unit(source)?);
    }

    let mut compositions = discover_compositions(root, &units)?;
    let mut segments = discover_segments(root, &compositions)?;
    let mut chains = discover_chains(root)?;
    compositions.sort_by(|left, right| left.id.cmp(right.id));
    segments.sort_by(|left, right| left.id.cmp(right.id));
    chains.sort_by(|left, right| left.id.cmp(right.id));

    Ok(RuntimeIndex {
        units,
        compositions,
        segments,
        chains,
    })
}

pub(crate) fn load_runtime_unit(source: &RustUnitSource) -> Result<UnitManifest, String> {
    let wasm = fs::read(&source.wasm_path)
        .map_err(|err| format!("cannot read {}: {err}", source.wasm_path.display()))?;
    let wasm_sha256 = leak_str(String::from_utf8_lossy(&sha256_hex(&wasm)).into_owned());
    let surface =
        WasmSurface::parse(&wasm).ok_or_else(|| format!("invalid wasm: {}", source.id))?;
    validate_wasm_unit_surface(&source.id, &surface)?;
    let exports = runtime_exports_from_surface(&surface)?;
    Ok(UnitManifest {
        id: leak_str(source.id.clone()),
        standard: leak_str(source.standard.clone()),
        standard_id: source.standard_id,
        abi: SDK_ABI_NAME,
        deterministic: Determinism::Pure,
        wasm_path: leak_str(
            path_relative_to(Path::new(env!("CARGO_MANIFEST_DIR")), &source.wasm_path)
                .display()
                .to_string(),
        ),
        manifest_path: leak_str(
            path_relative_to(Path::new(env!("CARGO_MANIFEST_DIR")), &source.manifest_path)
                .display()
                .to_string(),
        ),
        wasm_sha256,
        imports: &[],
        exports,
    })
}

pub(crate) fn runtime_exports_from_surface(
    surface: &WasmSurface,
) -> Result<&'static [ApiFunction], String> {
    let mut exports = Vec::new();
    for export in &surface.exports {
        match export.kind {
            ExternalKind::Memory => exports.push(ApiFunction {
                module: None,
                name: leak_str(export.name.clone()),
                ty: "memory",
                unit: None,
            }),
            ExternalKind::Func => {
                let ty = export
                    .ty
                    .as_ref()
                    .ok_or_else(|| format!("missing function type: {}", export.name))?;
                exports.push(ApiFunction {
                    module: None,
                    name: leak_str(export.name.clone()),
                    ty: leak_str(api_type_string(ty)),
                    unit: None,
                });
            }
            _ => {}
        }
    }
    Ok(leak_slice(exports))
}

pub(crate) fn api_type_string(ty: &FuncType) -> String {
    let mut out = String::new();
    out.push('(');
    for (index, param) in ty.params().iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        out.push_str(valtype_name(param));
    }
    out.push_str(") -> ");
    match ty.results() {
        [] => out.push_str("()"),
        [one] => out.push_str(valtype_name(one)),
        many => {
            out.push('(');
            for (index, result) in many.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                out.push_str(valtype_name(result));
            }
            out.push(')');
        }
    }
    out
}

pub(crate) fn valtype_name(ty: &ValType) -> &'static str {
    match ty {
        ValType::I32 => "i32",
        ValType::I64 => "i64",
        ValType::F32 => "f32",
        ValType::F64 => "f64",
    }
}

pub(crate) fn discover_compositions(
    root: &Path,
    units: &[UnitManifest],
) -> Result<Vec<CompositionManifest>, String> {
    let mut manifests = Vec::new();
    let composition_dir = root.join("compositions");
    for entry in fs::read_dir(&composition_dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let source_path = entry.path().join("compose.edsl");
        if !source_path.exists() {
            continue;
        }
        let source = fs::read_to_string(&source_path).map_err(|err| err.to_string())?;
        let mut id = None;
        let mut output_unit = None;
        let mut components = Vec::new();
        for raw in source.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            let tokens: Vec<&str> = line.split_whitespace().collect();
            match tokens.as_slice() {
                ["composition", value] => id = Some((*value).to_owned()),
                ["output", value] => output_unit = Some((*value).to_owned()),
                ["component", _alias, unit_id] => {
                    let unit = units
                        .iter()
                        .find(|unit| unit.id == *unit_id)
                        .ok_or_else(|| {
                            format!("unknown unit in {}: {unit_id}", source_path.display())
                        })?;
                    components.push(CompositionComponent {
                        unit_id: unit.id,
                        wasm_sha256: unit.wasm_sha256,
                    });
                }
                _ => {}
            }
        }
        let id = id.ok_or_else(|| format!("missing composition id: {}", source_path.display()))?;
        let output_unit = output_unit
            .ok_or_else(|| format!("missing composition output: {}", source_path.display()))?;
        let path = path_relative_to(root, &source_path.with_file_name("compose.edm"));
        let mut manifest = CompositionManifest {
            id: leak_str(id),
            path: leak_str(path.display().to_string()),
            sha256: "",
            output_unit: leak_str(output_unit),
            steps: 0,
            components: leak_slice(components),
        };
        manifest.steps = parse_composition_source(&manifest, &source, units)?.len() as u16;
        let bytes = composition_manifest_bytes_with_units(&manifest, units)?;
        manifest.sha256 = leak_str(String::from_utf8_lossy(&sha256_hex(&bytes)).into_owned());
        manifests.push(manifest);
    }
    Ok(manifests)
}

pub(crate) fn discover_segments(
    root: &Path,
    compositions: &[CompositionManifest],
) -> Result<Vec<SegmentManifest>, String> {
    let mut manifests = Vec::new();
    let segment_dir = root.join("segments");
    for entry in fs::read_dir(&segment_dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let source_path = entry.path().join("segment.edsl");
        if !source_path.exists() {
            continue;
        }
        let source = fs::read_to_string(&source_path).map_err(|err| err.to_string())?;
        let parsed = parse_segment_source_values(&source, &source_path)?;
        let composition = compositions
            .iter()
            .find(|composition| composition.id == parsed.composition_id)
            .ok_or_else(|| format!("unknown segment composition: {}", parsed.composition_id))?;
        let path = path_relative_to(root, &source_path.with_file_name("segment.eseg"));
        let mut manifest = SegmentManifest {
            id: leak_str(parsed.id),
            path: leak_str(path.display().to_string()),
            sha256: "",
            composition_id: composition.id,
            composition_sha256: composition.sha256,
            node_role: leak_str(parsed.node_role),
            capability: leak_str(parsed.capability),
            input_count: parsed.input_kinds.len() as u16,
            output_count: parsed.output_count,
            component_start: parsed.component_start,
            component_count: parsed.component_count,
            step_start: parsed.step_start,
            step_count: parsed.step_count,
            input_kinds: leak_slice(parsed.input_kinds),
        };
        let bytes = segment_manifest_bytes(&manifest)?;
        manifest.sha256 = leak_str(String::from_utf8_lossy(&sha256_hex(&bytes)).into_owned());
        manifests.push(manifest);
    }
    Ok(manifests)
}

pub(crate) fn discover_chains(root: &Path) -> Result<Vec<ChainManifest>, String> {
    let mut manifests = Vec::new();
    let chain_dir = root.join("chains");
    for entry in fs::read_dir(&chain_dir).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let source_path = entry.path().join("chain.edsl");
        if !source_path.exists() {
            continue;
        }
        let source = fs::read_to_string(&source_path).map_err(|err| err.to_string())?;
        let parsed = parse_chain_source_values(&source, &source_path)?;
        let path = path_relative_to(root, &source_path.with_file_name("chain.echn"));
        let mut manifest = ChainManifest {
            id: leak_str(parsed.id),
            path: leak_str(path.display().to_string()),
            sha256: "",
            segment_count: parsed.segments.len() as u16,
            link_count: parsed.link_count,
            segments: leak_slice(parsed.segments.into_iter().map(leak_str).collect()),
        };
        let bytes = chain_manifest_bytes(&manifest)?;
        manifest.sha256 = leak_str(String::from_utf8_lossy(&sha256_hex(&bytes)).into_owned());
        manifests.push(manifest);
    }
    Ok(manifests)
}

pub(crate) fn leak_str(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

pub(crate) fn leak_slice<T>(value: Vec<T>) -> &'static [T] {
    Box::leak(value.into_boxed_slice())
}

pub(crate) fn rust_unit_metadata(manifest_path: &Path) -> Result<RustUnitMetadata, String> {
    let source = fs::read_to_string(manifest_path).map_err(|err| err.to_string())?;
    let mut in_unit = false;
    let mut id = None;
    let mut standard = None;
    for raw in source.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line == "[package.metadata.edgerun.unit]" {
            in_unit = true;
            continue;
        }
        if line.starts_with('[') {
            in_unit = false;
        }
        if !in_unit {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').to_owned();
        match key.trim() {
            "id" => id = Some(value),
            "standard" => standard = Some(value),
            _ => {}
        }
    }
    Ok(RustUnitMetadata {
        id: id.ok_or_else(|| format!("missing edgerun unit id: {}", manifest_path.display()))?,
        standard: standard
            .ok_or_else(|| format!("missing edgerun unit standard: {}", manifest_path.display()))?,
    })
}

pub(crate) fn compile_rust_unit_source(root: &Path, source: &RustUnitSource) -> Result<(), String> {
    let status = Command::new("cargo")
        .arg("build")
        .arg("--manifest-path")
        .arg(&source.rust_manifest)
        .arg("--target-dir")
        .arg(
            source
                .rust_manifest
                .parent()
                .ok_or_else(|| format!("bad rust manifest path for {}", source.id))?
                .join("target"),
        )
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--release")
        .status()
        .map_err(|err| format!("cannot run cargo for {}: {err}", source.id))?;
    if !status.success() {
        return Err(format!("cargo wasm build failed for {}", source.id));
    }
    let package = rust_package_name(&source.rust_manifest)?.unwrap_or(source.id.clone());
    let crate_name = package.replace('-', "_");
    let built_wasm = source
        .rust_manifest
        .parent()
        .ok_or_else(|| format!("bad rust manifest path for {}", source.id))?
        .join("target/wasm32-unknown-unknown/release")
        .join(format!("{crate_name}.wasm"));
    if !built_wasm.exists() {
        return Err(format!(
            "rust wasm output missing for {}: {}",
            source.id,
            built_wasm.display()
        ));
    }
    fs::copy(&built_wasm, &source.wasm_path).map_err(|err| err.to_string())?;
    let _ = root;
    Ok(())
}

pub(crate) fn generate_rust_unit_metadata(
    root: &Path,
    source: &RustUnitSource,
) -> Result<(PathBuf, PathBuf, String), String> {
    let wasm = fs::read(&source.wasm_path).map_err(|err| err.to_string())?;
    let wasm_sha256 = String::from_utf8_lossy(&sha256_hex(&wasm)).into_owned();
    let wasm_sha256_bytes = hex_to_32(&wasm_sha256)?;
    let surface =
        WasmSurface::parse(&wasm).ok_or_else(|| format!("invalid wasm: {}", source.id))?;
    validate_wasm_unit_surface(&source.id, &surface)?;
    verify_unit_identity_exports(&surface, source.standard_id)?;
    let unit_manifest = runtime_unit_manifest_bytes(source, &surface, &wasm_sha256_bytes);
    let api = runtime_api_manifest_bytes(&source.id, &surface)?;
    let api_path = source.wasm_path.with_file_name("api.edm");
    fs::write(&source.manifest_path, unit_manifest).map_err(|err| err.to_string())?;
    fs::write(&api_path, api).map_err(|err| err.to_string())?;
    Ok((
        path_relative_to(root, &source.manifest_path),
        path_relative_to(root, &api_path),
        wasm_sha256,
    ))
}

pub(crate) fn validate_rust_unit_source(source: &RustUnitSource) -> Result<(), String> {
    let code = fs::read_to_string(&source.rust_source)
        .map_err(|err| format!("cannot read {}: {err}", source.rust_source.display()))?;
    let metadata_count = code.matches("edgerun_unit::metadata!(").count();
    if metadata_count != 1 {
        return Err(format!(
            "{} must declare exactly one edgerun_unit::metadata!(...) block",
            source.rust_source.display()
        ));
    }
    if !code.contains("#[edgerun_unit::export]") {
        return Err(format!(
            "{} must mark public unit functions with #[edgerun_unit::export]",
            source.rust_source.display()
        ));
    }
    if code.contains("#[no_mangle]") {
        return Err(format!(
            "{} must use edgerun_unit::export instead of manual #[no_mangle]",
            source.rust_source.display()
        ));
    }
    Ok(())
}

pub(crate) fn read_rust_unit_standard_id(path: &Path) -> Result<i32, String> {
    let code =
        fs::read_to_string(path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    let Some(start) = code.find("edgerun_unit::metadata!(") else {
        return Err(format!(
            "{} missing edgerun_unit::metadata!(...)",
            path.display()
        ));
    };
    let start = start + "edgerun_unit::metadata!(".len();
    let Some(end) = code[start..].find(')') else {
        return Err(format!(
            "{} has unterminated edgerun_unit::metadata!(...)",
            path.display()
        ));
    };
    let value = code[start..start + end].trim();
    value.parse().map_err(|_| {
        format!(
            "{} metadata standard id must be an i32 literal",
            path.display()
        )
    })
}

pub(crate) fn validate_wasm_unit_surface(
    unit_id: &str,
    surface: &WasmSurface,
) -> Result<(), String> {
    if surface.import_count != 0 {
        return Err(format!(
            "{unit_id} imports host or module state; units must be closed deterministic wasm"
        ));
    }
    if !surface
        .exports
        .iter()
        .any(|export| export.kind == ExternalKind::Memory)
    {
        return Err(format!("{unit_id} must export its own memory"));
    }
    if !has_i32_zero_arg_export(surface, "proto_abi_version") {
        return Err(format!("{unit_id} must export proto_abi_version() -> i32"));
    }
    if !has_i32_zero_arg_export(surface, "proto_standard_id") {
        return Err(format!("{unit_id} must export proto_standard_id() -> i32"));
    }
    let unit_functions: Vec<&ExportSurface> = surface
        .exports
        .iter()
        .filter(|export| {
            export.kind == ExternalKind::Func
                && export.name != "proto_abi_version"
                && export.name != "proto_standard_id"
        })
        .collect();
    if unit_functions.is_empty() {
        return Err(format!("{unit_id} must export at least one unit function"));
    }
    for function in unit_functions {
        if api_cost_profile(&function.name).is_none() {
            return Err(format!(
                "missing deterministic cost profile: {unit_id}.{}",
                function.name
            ));
        }
    }
    Ok(())
}

pub(crate) fn verify_unit_identity_exports(
    surface: &WasmSurface,
    _expected_standard_id: i32,
) -> Result<(), String> {
    if !has_i32_zero_arg_export(surface, "proto_abi_version") {
        return Err("missing proto_abi_version() -> i32".to_owned());
    }
    if !has_i32_zero_arg_export(surface, "proto_standard_id") {
        return Err("missing proto_standard_id() -> i32".to_owned());
    }
    Ok(())
}

pub(crate) fn has_i32_zero_arg_export(surface: &WasmSurface, name: &str) -> bool {
    surface.exports.iter().any(|export| {
        export.kind == ExternalKind::Func
            && export.name == name
            && export.ty.as_ref().is_some_and(|ty| {
                ty.params().is_empty()
                    && ty.results().len() == 1
                    && matches!(ty.results()[0], ValType::I32)
            })
    })
}

pub(crate) fn runtime_unit_manifest_bytes(
    source: &RustUnitSource,
    surface: &WasmSurface,
    wasm_sha256: &[u8; 32],
) -> Vec<u8> {
    let export_count = surface
        .exports
        .iter()
        .filter(|export| export.kind == ExternalKind::Memory || export.kind == ExternalKind::Func)
        .count();
    sdk_wire_record_bytes(SdkWireRecord::UnitManifest(
        edgerun_wire::UnitManifestRecord {
            abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
            flags: 3,
            import_count: 0,
            export_count: export_count as u16,
            standard_id: source.standard_id,
            wasm_sha256: *wasm_sha256,
            unit_id: source.id.as_bytes().to_vec(),
            standard: source.standard.as_bytes().to_vec(),
        },
    ))
}

pub(crate) fn runtime_api_manifest_bytes(
    unit_id: &str,
    surface: &WasmSurface,
) -> Result<Vec<u8>, String> {
    let functions: Vec<&ExportSurface> = surface
        .exports
        .iter()
        .filter(|export| export.kind == ExternalKind::Func)
        .collect();
    let mut wire_functions = Vec::with_capacity(functions.len());
    for function in functions {
        let ty = function
            .ty
            .as_ref()
            .ok_or_else(|| format!("missing type for {unit_id}.{}", function.name))?;
        let Some((cost_base, cost_per_byte)) = api_cost_profile(&function.name) else {
            return Err(format!(
                "missing api cost profile: {unit_id}.{}",
                function.name
            ));
        };
        wire_functions.push(edgerun_wire::UnitApiFunction {
            name: function.name.as_bytes().to_vec(),
            params: valtypes_to_api_bytes(ty.params()),
            results: valtypes_to_api_bytes(ty.results()),
            cost_base,
            cost_per_byte,
        });
    }
    Ok(sdk_wire_record_bytes(SdkWireRecord::UnitApi(
        edgerun_wire::UnitApi {
            abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
            functions: wire_functions,
        },
    )))
}

pub(crate) fn rust_package_name(manifest_path: &Path) -> Result<Option<String>, String> {
    let source = fs::read_to_string(manifest_path).map_err(|err| err.to_string())?;
    let mut in_package = false;
    for raw in source.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line == "[package]" {
            in_package = true;
            continue;
        }
        if line.starts_with('[') {
            in_package = false;
        }
        if in_package {
            let Some(value) = line.strip_prefix("name") else {
                continue;
            };
            let Some(value) = value.trim_start().strip_prefix('=') else {
                continue;
            };
            let value = value.trim().trim_matches('"');
            return Ok(Some(value.to_owned()));
        }
    }
    Ok(None)
}

#[derive(Clone, Copy)]
pub(crate) struct BuildStep {
    pub(crate) opcode: u8,
    pub(crate) component_index: u8,
    pub(crate) function_index: u8,
    pub(crate) arg0: u32,
    pub(crate) arg1: u32,
    pub(crate) arg2: u32,
    pub(crate) arg3: u32,
    pub(crate) arg4: u32,
}

pub(crate) const fn build_step(
    opcode: u8,
    component_index: u8,
    function_index: u8,
    arg0: u32,
    arg1: u32,
    arg2: u32,
    arg3: u32,
    arg4: u32,
) -> BuildStep {
    BuildStep {
        opcode,
        component_index,
        function_index,
        arg0,
        arg1,
        arg2,
        arg3,
        arg4,
    }
}

pub(crate) fn composition_manifest_bytes(
    manifest: &CompositionManifest,
) -> Result<Vec<u8>, String> {
    composition_manifest_bytes_with_units(manifest, units())
}

pub(crate) fn composition_manifest_bytes_with_units(
    manifest: &CompositionManifest,
    units: &[UnitManifest],
) -> Result<Vec<u8>, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = root.join(manifest.path).with_file_name("compose.edsl");
    let source = fs::read_to_string(&source_path).map_err(|err| err.to_string())?;
    let steps = parse_composition_source(manifest, &source, units)?;
    if steps.len() != manifest.steps as usize {
        return Err(format!("composition step count mismatch: {}", manifest.id));
    }
    Ok(sdk_wire_record_bytes(SdkWireRecord::Composition(
        edgerun_wire::CompositionRecord {
            abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
            flags: 1,
            id: manifest.id.as_bytes().to_vec(),
            output_unit: manifest.output_unit.as_bytes().to_vec(),
            components: manifest
                .components
                .iter()
                .map(|component| {
                    Ok(edgerun_wire::CompositionComponentRecord {
                        unit_id: component.unit_id.as_bytes().to_vec(),
                        wasm_sha256: hex_to_32(component.wasm_sha256)?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
            steps: steps
                .into_iter()
                .map(|step| edgerun_wire::CompositionStepRecord {
                    opcode: step.opcode,
                    component_index: step.component_index,
                    function_index: step.function_index,
                    arg0: step.arg0,
                    arg1: step.arg1,
                    arg2: step.arg2,
                    arg3: step.arg3,
                    arg4: step.arg4,
                })
                .collect(),
        },
    )))
}

pub(crate) fn parse_composition_source(
    manifest: &CompositionManifest,
    source: &str,
    units: &[UnitManifest],
) -> Result<Vec<BuildStep>, String> {
    let mut composition_id = None;
    let mut output_unit = None;
    let mut inputs: Vec<String> = Vec::new();
    let mut components: Vec<(String, String)> = Vec::new();
    let mut steps = Vec::new();

    for (line_index, raw) in source.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let context = || format!("{} line {}", manifest.id, line_index + 1);
        match tokens.as_slice() {
            ["composition", id] => composition_id = Some((*id).to_owned()),
            ["output", id] => output_unit = Some((*id).to_owned()),
            ["input", name] => inputs.push((*name).to_owned()),
            ["component", alias, unit_id] => {
                components.push(((*alias).to_owned(), (*unit_id).to_owned()));
            }
            ["copy-input", input, target, dst, len] => {
                steps.push(build_step(
                    1,
                    component_index(&components, target, &context())?,
                    0,
                    input_index(&inputs, input, &context())? as u32,
                    parse_ref(dst, &inputs, &context())?,
                    parse_ref(len, &inputs, &context())?,
                    0,
                    0,
                ));
            }
            ["call", function, arg0, arg1, arg2, arg3, arg4] => {
                let (component, function_index) =
                    resolve_function(&components, function, &context(), units)?;
                steps.push(build_step(
                    2,
                    component,
                    function_index,
                    parse_ref(arg0, &inputs, &context())?,
                    parse_ref(arg1, &inputs, &context())?,
                    parse_ref(arg2, &inputs, &context())?,
                    parse_ref(arg3, &inputs, &context())?,
                    parse_ref(arg4, &inputs, &context())?,
                ));
            }
            ["copy", source, src, target, dst, len] => {
                steps.push(build_step(
                    3,
                    component_index(&components, target, &context())?,
                    0,
                    component_index(&components, source, &context())? as u32,
                    parse_ref(src, &inputs, &context())?,
                    parse_ref(dst, &inputs, &context())?,
                    parse_ref(len, &inputs, &context())?,
                    0,
                ));
            }
            ["publish", source, src, len] => {
                steps.push(build_step(
                    4,
                    component_index(&components, source, &context())?,
                    0,
                    0,
                    parse_ref(src, &inputs, &context())?,
                    parse_ref(len, &inputs, &context())?,
                    0,
                    0,
                ));
            }
            ["branch", cmp, left, right, then_step, else_step] => {
                steps.push(build_step(
                    5,
                    0,
                    0,
                    comparison_ref(cmp, parse_ref(left, &inputs, &context())?, &context())?,
                    parse_ref(right, &inputs, &context())?,
                    parse_u32(then_step, &context())?,
                    parse_u32(else_step, &context())?,
                    0,
                ));
            }
            ["jump", target] => {
                steps.push(build_step(
                    6,
                    0,
                    0,
                    parse_u32(target, &context())?,
                    0,
                    0,
                    0,
                    0,
                ));
            }
            ["write-byte", target, ptr, byte] => {
                steps.push(build_step(
                    7,
                    component_index(&components, target, &context())?,
                    0,
                    parse_ref(ptr, &inputs, &context())?,
                    parse_u32(byte, &context())?,
                    0,
                    0,
                    0,
                ));
            }
            ["capture", function, arg0, arg1, arg2, arg3, "->", slot] => {
                let (component, function_index) =
                    resolve_function(&components, function, &context(), units)?;
                steps.push(build_step(
                    8,
                    component,
                    function_index,
                    parse_ref(arg0, &inputs, &context())?,
                    parse_ref(arg1, &inputs, &context())?,
                    parse_ref(arg2, &inputs, &context())?,
                    parse_ref(arg3, &inputs, &context())?,
                    parse_slot(slot, &context())?,
                ));
            }
            ["load-u32", source, ptr, "->", slot] => {
                steps.push(build_step(
                    9,
                    component_index(&components, source, &context())?,
                    0,
                    parse_ref(ptr, &inputs, &context())?,
                    parse_slot(slot, &context())?,
                    0,
                    0,
                    0,
                ));
            }
            _ => return Err(format!("{}: bad source line: {line}", context())),
        }
    }

    if composition_id.as_deref() != Some(manifest.id) {
        return Err(format!("{} source composition id mismatch", manifest.id));
    }
    if output_unit.as_deref() != Some(manifest.output_unit) {
        return Err(format!("{} source output unit mismatch", manifest.id));
    }
    if components.len() != manifest.components.len() {
        return Err(format!("{} source component count mismatch", manifest.id));
    }
    for (actual, expected) in components.iter().zip(manifest.components.iter()) {
        if actual.1 != expected.unit_id {
            return Err(format!("{} source component order mismatch", manifest.id));
        }
    }
    Ok(steps)
}

pub(crate) fn component_index(
    components: &[(String, String)],
    alias: &str,
    context: &str,
) -> Result<u8, String> {
    components
        .iter()
        .position(|(actual, _)| actual == alias)
        .map(|index| index as u8)
        .ok_or_else(|| format!("{context}: unknown component alias: {alias}"))
}

pub(crate) fn input_index(inputs: &[String], name: &str, context: &str) -> Result<usize, String> {
    inputs
        .iter()
        .position(|actual| actual == name)
        .ok_or_else(|| format!("{context}: unknown input: {name}"))
}

pub(crate) fn resolve_function(
    components: &[(String, String)],
    function: &str,
    context: &str,
    units: &[UnitManifest],
) -> Result<(u8, u8), String> {
    let Some((alias, function_name)) = function.split_once('.') else {
        return Err(format!("{context}: bad function reference: {function}"));
    };
    let component = component_index(components, alias, context)?;
    let unit_id = &components[component as usize].1;
    let Some(unit) = units.iter().find(|unit| unit.id == unit_id) else {
        return Err(format!("{context}: unknown unit: {unit_id}"));
    };
    let Some(function_index) = unit
        .exports
        .iter()
        .filter(|export| export.ty != "memory")
        .position(|export| export.name == function_name)
    else {
        return Err(format!("{context}: unknown function: {function}"));
    };
    Ok((component, function_index as u8))
}

pub(crate) fn parse_ref(token: &str, inputs: &[String], context: &str) -> Result<u32, String> {
    if let Some(value) = token.strip_prefix("const:") {
        return parse_u32(value, context);
    }
    if let Some(name) = token.strip_prefix("len:") {
        return Ok(0x0100_0000 | input_index(inputs, name, context)? as u32);
    }
    if let Some(value) = token.strip_prefix("slot:") {
        return Ok(0x0200_0000 | parse_u32(value, context)?);
    }
    if let Some(rest) = token.strip_prefix("add:") {
        let Some((constant, input)) = rest.split_once("+len:") else {
            return Err(format!("{context}: bad add ref: {token}"));
        };
        let constant = parse_u32(constant, context)?;
        if constant > 0xffff {
            return Err(format!("{context}: add constant out of range: {token}"));
        }
        let input = input_index(inputs, input, context)? as u32;
        return Ok(0x0300_0000 | (constant << 8) | input);
    }
    Err(format!("{context}: bad ref: {token}"))
}

pub(crate) fn comparison_ref(cmp: &str, reference: u32, context: &str) -> Result<u32, String> {
    let comparison = match cmp {
        "gt" => 1,
        "eq" => 2,
        "ne" => 3,
        _ => return Err(format!("{context}: bad comparison: {cmp}")),
    };
    Ok((comparison << 28) | (reference & 0x0fff_ffff))
}

pub(crate) fn parse_slot(token: &str, context: &str) -> Result<u32, String> {
    let Some(slot) = token.strip_prefix("slot:") else {
        return Err(format!("{context}: bad slot: {token}"));
    };
    parse_u32(slot, context)
}

pub(crate) fn parse_u32(token: &str, context: &str) -> Result<u32, String> {
    token
        .parse::<u32>()
        .map_err(|_| format!("{context}: bad integer: {token}"))
}

pub(crate) fn segment_manifest_bytes(manifest: &SegmentManifest) -> Result<Vec<u8>, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = root.join(manifest.path).with_file_name("segment.edsl");
    let source = fs::read_to_string(&source_path).map_err(|err| err.to_string())?;
    parse_segment_source(manifest, &source)?;
    Ok(sdk_wire_record_bytes(SdkWireRecord::Segment(
        edgerun_wire::SegmentRecord {
            abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
            flags: 1,
            input_count: manifest.input_count,
            output_count: manifest.output_count,
            component_start: manifest.component_start,
            component_count: manifest.component_count,
            step_start: manifest.step_start,
            step_count: manifest.step_count,
            composition_sha256: hex_to_32(manifest.composition_sha256)?,
            id: manifest.id.as_bytes().to_vec(),
            composition_id: manifest.composition_id.as_bytes().to_vec(),
            node_role: manifest.node_role.as_bytes().to_vec(),
            capability: manifest.capability.as_bytes().to_vec(),
            input_kinds: manifest.input_kinds.to_vec(),
        },
    )))
}

pub(crate) fn chain_manifest_bytes(manifest: &ChainManifest) -> Result<Vec<u8>, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source_path = root.join(manifest.path).with_file_name("chain.edsl");
    let source = fs::read_to_string(&source_path).map_err(|err| err.to_string())?;
    let links = parse_chain_source(manifest, &source)?;
    Ok(sdk_wire_record_bytes(SdkWireRecord::Chain(
        edgerun_wire::ChainRecord {
            abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
            flags: 1,
            id: manifest.id.as_bytes().to_vec(),
            segments: manifest
                .segments
                .iter()
                .map(|segment_id| segment_id.as_bytes().to_vec())
                .collect(),
            links: links
                .into_iter()
                .map(|(from_segment, from_output, to_segment, to_input)| {
                    edgerun_wire::ChainLinkRecord {
                        from_segment,
                        from_output,
                        to_segment,
                        to_input,
                    }
                })
                .collect(),
        },
    )))
}

pub(crate) fn parse_segment_source(manifest: &SegmentManifest, source: &str) -> Result<(), String> {
    let mut id = None;
    let mut composition_id = None;
    let mut node_role = None;
    let mut capability = None;
    let mut input_kinds = Vec::new();
    let mut outputs = None;
    let mut components = None;
    let mut steps = None;

    for (line_index, raw) in source.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let context = || format!("{} line {}", manifest.id, line_index + 1);
        match tokens.as_slice() {
            ["segment", value] => id = Some((*value).to_owned()),
            ["composition", value] => composition_id = Some((*value).to_owned()),
            ["node-role", value] => node_role = Some((*value).to_owned()),
            ["capability", value] => capability = Some((*value).to_owned()),
            ["input", _name, kind] => input_kinds.push(parse_input_kind(kind, &context())?),
            ["outputs", value] => outputs = Some(parse_u32(value, &context())? as u16),
            ["components", start, count] => {
                components = Some((
                    parse_u32(start, &context())? as u16,
                    parse_u32(count, &context())? as u16,
                ));
            }
            ["steps", start, count] => {
                steps = Some((
                    parse_u32(start, &context())? as u16,
                    parse_u32(count, &context())? as u16,
                ));
            }
            _ => return Err(format!("{}: bad source line: {line}", context())),
        }
    }

    if id.as_deref() != Some(manifest.id)
        || composition_id.as_deref() != Some(manifest.composition_id)
        || node_role.as_deref() != Some(manifest.node_role)
        || capability.as_deref() != Some(manifest.capability)
        || outputs != Some(manifest.output_count)
        || components != Some((manifest.component_start, manifest.component_count))
        || steps != Some((manifest.step_start, manifest.step_count))
        || input_kinds != manifest.input_kinds
        || input_kinds.len() != manifest.input_count as usize
    {
        return Err(format!("{} segment source mismatch", manifest.id));
    }
    Ok(())
}

pub(crate) fn parse_chain_source(
    manifest: &ChainManifest,
    source: &str,
) -> Result<Vec<(u16, u16, u16, u16)>, String> {
    let mut id = None;
    let mut segments = Vec::new();
    let mut links = Vec::new();

    for (line_index, raw) in source.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let context = || format!("{} line {}", manifest.id, line_index + 1);
        match tokens.as_slice() {
            ["chain", value] => id = Some((*value).to_owned()),
            ["segment", value] => segments.push((*value).to_owned()),
            ["link", from, "->", to] => {
                let (from_segment, from_output) = parse_endpoint(from, &context())?;
                let (to_segment, to_input) = parse_endpoint(to, &context())?;
                links.push((from_segment, from_output, to_segment, to_input));
            }
            _ => return Err(format!("{}: bad source line: {line}", context())),
        }
    }

    if id.as_deref() != Some(manifest.id)
        || segments.len() != manifest.segment_count as usize
        || links.len() != manifest.link_count as usize
        || segments
            .iter()
            .zip(manifest.segments.iter())
            .any(|(actual, expected)| actual != expected)
    {
        return Err(format!("{} chain source mismatch", manifest.id));
    }
    Ok(links)
}

pub(crate) struct ParsedSegmentSource {
    pub(crate) id: String,
    pub(crate) composition_id: String,
    pub(crate) node_role: String,
    pub(crate) capability: String,
    pub(crate) input_kinds: Vec<u8>,
    pub(crate) output_count: u16,
    pub(crate) component_start: u16,
    pub(crate) component_count: u16,
    pub(crate) step_start: u16,
    pub(crate) step_count: u16,
}

pub(crate) fn parse_segment_source_values(
    source: &str,
    source_path: &Path,
) -> Result<ParsedSegmentSource, String> {
    let mut id = None;
    let mut composition_id = None;
    let mut node_role = None;
    let mut capability = None;
    let mut input_kinds = Vec::new();
    let mut outputs = None;
    let mut components = None;
    let mut steps = None;

    for (line_index, raw) in source.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let context = || format!("{} line {}", source_path.display(), line_index + 1);
        match tokens.as_slice() {
            ["segment", value] => id = Some((*value).to_owned()),
            ["composition", value] => composition_id = Some((*value).to_owned()),
            ["node-role", value] => node_role = Some((*value).to_owned()),
            ["capability", value] => capability = Some((*value).to_owned()),
            ["input", _name, kind] => input_kinds.push(parse_input_kind(kind, &context())?),
            ["outputs", value] => outputs = Some(parse_u32(value, &context())? as u16),
            ["components", start, count] => {
                components = Some((
                    parse_u32(start, &context())? as u16,
                    parse_u32(count, &context())? as u16,
                ));
            }
            ["steps", start, count] => {
                steps = Some((
                    parse_u32(start, &context())? as u16,
                    parse_u32(count, &context())? as u16,
                ));
            }
            _ => return Err(format!("{}: bad source line: {line}", context())),
        }
    }

    let (component_start, component_count) =
        components.ok_or_else(|| format!("missing components: {}", source_path.display()))?;
    let (step_start, step_count) =
        steps.ok_or_else(|| format!("missing steps: {}", source_path.display()))?;
    Ok(ParsedSegmentSource {
        id: id.ok_or_else(|| format!("missing segment id: {}", source_path.display()))?,
        composition_id: composition_id
            .ok_or_else(|| format!("missing segment composition: {}", source_path.display()))?,
        node_role: node_role
            .ok_or_else(|| format!("missing segment node-role: {}", source_path.display()))?,
        capability: capability
            .ok_or_else(|| format!("missing segment capability: {}", source_path.display()))?,
        input_kinds,
        output_count: outputs
            .ok_or_else(|| format!("missing segment outputs: {}", source_path.display()))?,
        component_start,
        component_count,
        step_start,
        step_count,
    })
}

pub(crate) struct ParsedChainSource {
    pub(crate) id: String,
    pub(crate) segments: Vec<String>,
    pub(crate) link_count: u16,
}

pub(crate) fn parse_chain_source_values(
    source: &str,
    source_path: &Path,
) -> Result<ParsedChainSource, String> {
    let mut id = None;
    let mut segments = Vec::new();
    let mut links = 0u16;

    for (line_index, raw) in source.lines().enumerate() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let tokens: Vec<&str> = line.split_whitespace().collect();
        let context = || format!("{} line {}", source_path.display(), line_index + 1);
        match tokens.as_slice() {
            ["chain", value] => id = Some((*value).to_owned()),
            ["segment", value] => segments.push((*value).to_owned()),
            ["link", from, "->", to] => {
                let _ = parse_endpoint(from, &context())?;
                let _ = parse_endpoint(to, &context())?;
                links = links.saturating_add(1);
            }
            _ => return Err(format!("{}: bad source line: {line}", context())),
        }
    }

    Ok(ParsedChainSource {
        id: id.ok_or_else(|| format!("missing chain id: {}", source_path.display()))?,
        segments,
        link_count: links,
    })
}

pub(crate) fn parse_input_kind(token: &str, context: &str) -> Result<u8, String> {
    match token {
        "public" => Ok(1),
        "linked" => Ok(2),
        "private" => Ok(3),
        _ => Err(format!("{context}: bad input kind: {token}")),
    }
}

pub(crate) fn parse_endpoint(token: &str, context: &str) -> Result<(u16, u16), String> {
    let Some((left, right)) = token.split_once('.') else {
        return Err(format!("{context}: bad chain endpoint: {token}"));
    };
    Ok((
        parse_u32(left, context)? as u16,
        parse_u32(right, context)? as u16,
    ))
}

pub(crate) fn write_execution_report(
    manifest: &CompositionManifest,
    inputs: &[Vec<u8>],
    report: &ExecutionReport,
    output_sha256: &[u8; 64],
) -> Result<PathBuf, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let composition_path = root.join(manifest.path);
    let report_path = composition_path.with_file_name("report.edr");
    let input_lengths: Vec<u32> = inputs.iter().map(|input| input.len() as u32).collect();
    let bytes = execution_report_bytes(
        manifest,
        &input_lengths,
        report.cost,
        report.output.len() as u32,
        output_sha256,
    )?;
    fs::write(&report_path, bytes).map_err(|err| err.to_string())?;
    Ok(path_relative_to(&root, &report_path))
}

pub(crate) fn write_segment_report(
    manifest: &SegmentManifest,
    inputs: &[Vec<u8>],
    report: &ExecutionReport,
    output_sha256: &[u8; 64],
) -> Result<PathBuf, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let segment_path = root.join(manifest.path);
    let report_path = segment_path.with_file_name("report.esrr");
    let input_lengths: Vec<u32> = inputs.iter().map(|input| input.len() as u32).collect();
    let input_hashes: Vec<[u8; 32]> = inputs.iter().map(|input| sha256(input)).collect();
    let bytes = segment_report_bytes(
        manifest,
        &input_lengths,
        &input_hashes,
        0,
        report.cost,
        report.output.len() as u32,
        u16::MAX,
        output_sha256,
    )?;
    fs::write(&report_path, bytes).map_err(|err| err.to_string())?;
    Ok(path_relative_to(&root, &report_path))
}

pub(crate) fn execution_report_bytes(
    manifest: &CompositionManifest,
    input_lengths: &[u32],
    cost: u64,
    output_len: u32,
    output_sha256: &[u8; 64],
) -> Result<Vec<u8>, String> {
    Ok(sdk_wire_record_bytes(SdkWireRecord::ExecutionReport(
        edgerun_wire::ExecutionReportRecord {
            abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
            flags: 1,
            input_count: input_lengths.len() as u16,
            step_count: manifest.steps,
            cost,
            output_len,
            composition_sha256: hex_to_32(manifest.sha256)?,
            output_sha256: hex_to_32(
                core::str::from_utf8(output_sha256).map_err(|_| "bad output hash")?,
            )?,
            composition_id: manifest.id.as_bytes().to_vec(),
            output_unit_id: manifest.output_unit.as_bytes().to_vec(),
            input_lengths: input_lengths.to_vec(),
            components: manifest
                .components
                .iter()
                .map(|component| {
                    Ok(edgerun_wire::CompositionComponentRecord {
                        unit_id: component.unit_id.as_bytes().to_vec(),
                        wasm_sha256: hex_to_32(component.wasm_sha256)?,
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
        },
    )))
}

pub(crate) fn segment_report_bytes(
    manifest: &SegmentManifest,
    input_lengths: &[u32],
    input_sha256: &[[u8; 32]],
    status: u16,
    cost: u64,
    output_len: u32,
    failed_step: u16,
    output_sha256: &[u8; 64],
) -> Result<Vec<u8>, String> {
    if input_lengths.len() != input_sha256.len() {
        return Err("input length/hash count mismatch".to_owned());
    }
    Ok(sdk_wire_record_bytes(SdkWireRecord::SegmentReport(
        edgerun_wire::SegmentReportRecord {
            abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
            flags: 1,
            status,
            input_count: input_lengths.len() as u16,
            output_count: manifest.output_count,
            cost,
            output_len,
            failed_step,
            segment_sha256: hex_to_32(manifest.sha256)?,
            composition_sha256: hex_to_32(manifest.composition_sha256)?,
            output_sha256: hex_to_32(
                core::str::from_utf8(output_sha256).map_err(|_| "bad output hash")?,
            )?,
            segment_id: manifest.id.as_bytes().to_vec(),
            composition_id: manifest.composition_id.as_bytes().to_vec(),
            node_role: manifest.node_role.as_bytes().to_vec(),
            input_lengths: input_lengths.to_vec(),
            input_sha256: input_sha256.to_vec(),
        },
    )))
}

pub(crate) fn path_relative_to(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

pub(crate) fn hex_to_32(hex: &str) -> Result<[u8; 32], String> {
    let bytes = parse_hex(hex).ok_or_else(|| "invalid sha256 hex".to_owned())?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "sha256 hex is not 32 bytes".to_owned())?;
    Ok(array)
}

pub(crate) fn verify_binary_report(
    manifest: &CompositionManifest,
    report: &edgerun_wire::ExecutionReportRecord,
) -> bool {
    if report.composition_id != manifest.id.as_bytes()
        || report.output_unit_id != manifest.output_unit.as_bytes()
        || report.step_count != manifest.steps
        || bytes_to_hex(&report.composition_sha256) != manifest.sha256
        || report.input_lengths.len() != report.input_count as usize
        || report.components.len() != manifest.components.len()
    {
        return false;
    }
    for expected in manifest.components {
        let Some(actual) = report
            .components
            .iter()
            .find(|component| component.unit_id == expected.unit_id.as_bytes())
        else {
            return false;
        };
        if bytes_to_hex(&actual.wasm_sha256) != expected.wasm_sha256 {
            return false;
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Ok(composition_bytes) = fs::read(root.join(manifest.path)) else {
        return false;
    };
    let Some(composition) = parse_composition(&composition_bytes) else {
        return false;
    };
    if !verify_binary_composition(manifest, &composition_bytes) {
        return false;
    }
    let Ok(quote) = quote_composition(manifest, &composition, &report.input_lengths) else {
        return false;
    };
    quote.cost == report.cost && quote.output_len == report.output_len
}

pub(crate) fn cmd_list() -> i32 {
    for manifest in units() {
        println!(
            "{}\n  standard: {}\n  wasm: {}\n  sha256: {}",
            manifest.id, manifest.standard, manifest.wasm_path, manifest.wasm_sha256
        );
    }
    0
}

pub(crate) fn cmd_explain(id: Option<&str>) -> i32 {
    let Some(id) = id else {
        eprintln!("explain requires a unit id");
        return 1;
    };
    let Some(manifest) = unit(id) else {
        eprintln!("unknown unit: {id}");
        return 1;
    };
    println!("{}", manifest.id);
    println!("  standard: {}", manifest.standard);
    println!("  abi: {}", manifest.abi);
    println!("  determinism: {}", manifest.deterministic.as_str());
    println!("  wasm: {}", manifest.wasm_path);
    println!("  wasm_sha256: {}", manifest.wasm_sha256);
    println!("  imports:");
    print_api(manifest.imports);
    println!("  exports:");
    print_api(manifest.exports);
    0
}

pub(crate) fn cmd_verify(id: Option<&str>) -> i32 {
    let manifests: Vec<&'static UnitManifest> = match id {
        Some(id) => match unit(id) {
            Some(unit) => vec![unit],
            None => {
                eprintln!("unknown unit: {id}");
                return 1;
            }
        },
        None => units().iter().collect(),
    };

    let mut ok = true;
    for manifest in manifests {
        let result = verify_unit(manifest);
        ok &= result;
    }
    if id.is_none() {
        for composition in compositions() {
            ok &= verify_composition(composition);
        }
    }
    if ok {
        0
    } else {
        1
    }
}

pub(crate) fn verify_composition(manifest: &CompositionManifest) -> bool {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let composition_path = root.join(manifest.path);
    let mut ok = true;

    println!("verify {}", manifest.id);
    match fs::read(&composition_path) {
        Ok(bytes) => {
            let actual = String::from_utf8_lossy(&sha256_hex(&bytes)).into_owned();
            let hash_ok = str_eq(&actual, manifest.sha256);
            ok &= hash_ok;
            print_check("composition-sha256", hash_ok);
            if !hash_ok {
                println!("    expected: {}", manifest.sha256);
                println!("    actual:   {actual}");
            }
            let composition_ok = verify_binary_composition(manifest, &bytes);
            ok &= composition_ok;
            print_check("binary-composition", composition_ok);
            if composition_ok && manifest.id == "hmac-sha256-rfc2104-composed" {
                let execute_ok = verify_hmac_sha256_composition(manifest, &bytes);
                ok &= execute_ok;
                print_check("composition-execute", execute_ok);
            }
            if composition_ok && manifest.id == "hkdf-extract-sha256-rfc5869" {
                let execute_ok = verify_hkdf_extract_sha256_composition(manifest, &bytes);
                ok &= execute_ok;
                print_check("composition-execute", execute_ok);
            }
            if composition_ok && manifest.id == "hkdf-expand-sha256-rfc5869-l42" {
                let execute_ok = verify_hkdf_expand_sha256_l42_composition(manifest, &bytes);
                ok &= execute_ok;
                print_check("composition-execute", execute_ok);
            }
            if composition_ok && manifest.id == "http-auth-preflight-rfc9110" {
                let execute_ok = verify_http_auth_preflight_composition(manifest, &bytes);
                ok &= execute_ok;
                print_check("composition-execute", execute_ok);
            }
            if composition_ok && manifest.id == "auth-decision-private-v1" {
                let execute_ok = verify_auth_decision_private_composition(manifest, &bytes);
                ok &= execute_ok;
                print_check("composition-execute", execute_ok);
            }
            if composition_ok && manifest.id == "hmac-sha256-verify-rfc2104" {
                let execute_ok = verify_hmac_sha256_verify_composition(manifest, &bytes);
                ok &= execute_ok;
                print_check("composition-execute", execute_ok);
            }
        }
        Err(err) => {
            ok = false;
            print_check("binary-composition", false);
            println!("    {}: {err}", composition_path.display());
        }
    }
    ok
}

pub(crate) fn verify_segment(manifest: &SegmentManifest) -> bool {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join(manifest.path);
    let mut ok = true;
    println!("verify {}", manifest.id);
    match fs::read(&path) {
        Ok(bytes) => {
            let actual = String::from_utf8_lossy(&sha256_hex(&bytes)).into_owned();
            let hash_ok = str_eq(&actual, manifest.sha256);
            ok &= hash_ok;
            print_check("segment-sha256", hash_ok);
            let parsed_ok = parse_segment(&bytes)
                .is_some_and(|parsed| verify_binary_segment(manifest, &parsed));
            ok &= parsed_ok;
            print_check("binary-segment", parsed_ok);
        }
        Err(err) => {
            ok = false;
            print_check("binary-segment", false);
            println!("    {}: {err}", path.display());
        }
    }
    ok
}

pub(crate) fn verify_chain(manifest: &ChainManifest) -> bool {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = root.join(manifest.path);
    let mut ok = true;
    println!("verify {}", manifest.id);
    match fs::read(&path) {
        Ok(bytes) => {
            let actual = String::from_utf8_lossy(&sha256_hex(&bytes)).into_owned();
            let hash_ok = str_eq(&actual, manifest.sha256);
            ok &= hash_ok;
            print_check("chain-sha256", hash_ok);
            let parsed_ok =
                parse_chain(&bytes).is_some_and(|parsed| verify_binary_chain(manifest, &parsed));
            ok &= parsed_ok;
            print_check("binary-chain", parsed_ok);
        }
        Err(err) => {
            ok = false;
            print_check("binary-chain", false);
            println!("    {}: {err}", path.display());
        }
    }
    ok
}

pub(crate) fn verify_binary_chain(
    expected: &ChainManifest,
    actual: &edgerun_wire::ChainRecord,
) -> bool {
    if actual.id != expected.id.as_bytes()
        || actual.segments.len() != expected.segment_count as usize
        || actual.links.len() != expected.link_count as usize
        || actual.segments.len() != expected.segments.len()
    {
        return false;
    }
    for (actual, expected_id) in actual.segments.iter().zip(expected.segments.iter()) {
        if *actual != expected_id.as_bytes() || segment(expected_id).is_none() {
            return false;
        }
    }
    for link in &actual.links {
        let Some(from) = expected
            .segments
            .get(link.from_segment as usize)
            .and_then(|id| segment(id))
        else {
            return false;
        };
        let Some(to) = expected
            .segments
            .get(link.to_segment as usize)
            .and_then(|id| segment(id))
        else {
            return false;
        };
        if link.from_output >= from.output_count || link.to_input >= to.input_count {
            return false;
        }
    }
    true
}

pub(crate) fn verify_binary_segment(
    expected: &SegmentManifest,
    actual: &edgerun_wire::SegmentRecord,
) -> bool {
    if actual.id != expected.id.as_bytes()
        || actual.composition_id != expected.composition_id.as_bytes()
        || actual.node_role != expected.node_role.as_bytes()
        || actual.capability != expected.capability.as_bytes()
        || bytes_to_hex(&actual.composition_sha256) != expected.composition_sha256
        || actual.input_count != expected.input_count
        || actual.output_count != expected.output_count
        || actual.component_start != expected.component_start
        || actual.component_count != expected.component_count
        || actual.step_start != expected.step_start
        || actual.step_count != expected.step_count
        || actual.input_kinds != expected.input_kinds
    {
        return false;
    }
    let Some(composition) = compositions()
        .iter()
        .find(|composition| composition.id == expected.composition_id)
    else {
        return false;
    };
    if composition.sha256 != expected.composition_sha256 {
        return false;
    }
    let component_end = expected
        .component_start
        .checked_add(expected.component_count)
        .unwrap_or(u16::MAX);
    let step_end = expected
        .step_start
        .checked_add(expected.step_count)
        .unwrap_or(u16::MAX);
    component_end as usize <= composition.components.len() && step_end <= composition.steps
}

pub(crate) fn verify_binary_segment_report(
    expected: &SegmentManifest,
    actual: &edgerun_wire::SegmentReportRecord,
) -> bool {
    if actual.segment_id != expected.id.as_bytes()
        || actual.composition_id != expected.composition_id.as_bytes()
        || actual.node_role != expected.node_role.as_bytes()
        || bytes_to_hex(&actual.segment_sha256) != expected.sha256
        || bytes_to_hex(&actual.composition_sha256) != expected.composition_sha256
        || actual.input_count != expected.input_count
        || actual.output_count != expected.output_count
        || actual.input_lengths.len() != expected.input_count as usize
        || actual.input_sha256.len() != expected.input_count as usize
    {
        return false;
    }
    let Some(composition_manifest) = composition(expected.composition_id) else {
        return false;
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Ok(composition_bytes) = fs::read(root.join(composition_manifest.path)) else {
        return false;
    };
    let Some(composition) = parse_composition(&composition_bytes) else {
        return false;
    };
    let Ok(preflight) =
        preflight_composition(composition_manifest, &composition, &actual.input_lengths)
    else {
        return false;
    };
    if actual.status == 0 {
        actual.failed_step == u16::MAX
            && actual.cost >= preflight.cost_min
            && actual.cost <= preflight.cost_max
            && preflight
                .output_len
                .is_none_or(|output_len| output_len == actual.output_len)
    } else {
        actual.failed_step < expected.step_count
    }
}

pub(crate) fn replay_binary_segment_report(
    expected: &SegmentManifest,
    actual: &edgerun_wire::SegmentReportRecord,
    inputs: &[Vec<u8>],
) -> bool {
    if !verify_binary_segment_report(expected, actual) || actual.status != 0 {
        return false;
    }
    if inputs.len() != expected.input_count as usize {
        return false;
    }
    for (index, input) in inputs.iter().enumerate() {
        if actual.input_lengths.get(index).copied() != Some(input.len() as u32) {
            return false;
        }
        let input_hash = sha256(input);
        if actual.input_sha256.get(index).copied() != Some(input_hash) {
            return false;
        }
    }
    let Some(composition_manifest) = composition(expected.composition_id) else {
        return false;
    };
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Ok(composition_bytes) = fs::read(root.join(composition_manifest.path)) else {
        return false;
    };
    let Some(composition) = parse_composition(&composition_bytes) else {
        return false;
    };
    let input_refs: Vec<&[u8]> = inputs.iter().map(Vec::as_slice).collect();
    let Ok(replayed) = execute_composition(composition_manifest, &composition, &input_refs) else {
        return false;
    };
    let output_hash = sha256(&replayed.output);
    actual.failed_step == u16::MAX
        && actual.cost == replayed.cost
        && actual.output_len == replayed.output.len() as u32
        && actual.output_sha256 == output_hash.as_slice()
}

pub(crate) const ESIG_ALGORITHM_ED25519: u16 = 1;
pub(crate) const ESIG_DOMAIN: &[u8] = b"edgerun-sdk.esig.v1.segment-report";

pub(crate) fn signature_bytes(report_bytes: &[u8], signing_key: &SigningKey) -> Vec<u8> {
    signature_bytes_for_domain(report_bytes, signing_key, ESIG_DOMAIN)
}

pub(crate) fn signature_bytes_for_domain(
    artifact_bytes: &[u8],
    signing_key: &SigningKey,
    domain: &[u8],
) -> Vec<u8> {
    let artifact_hash = sha256(artifact_bytes);
    let payload = signature_payload_for_domain(domain, &artifact_hash);
    let signature = signing_key.sign(&payload);
    let public_key = signing_key.verifying_key();
    sdk_wire_record_bytes(SdkWireRecord::ArtifactSignature(
        edgerun_wire::ArtifactSignature {
            abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
            flags: 1,
            algorithm: ESIG_ALGORITHM_ED25519,
            artifact_sha256: artifact_hash,
            public_key: public_key.as_bytes().to_vec(),
            signature: signature.to_bytes().to_vec(),
        },
    ))
}

pub(crate) fn signer_policy_bytes(
    manifest: &SegmentManifest,
    public_keys: &[Vec<u8>],
) -> Result<Vec<u8>, String> {
    let segment_sha256 = hex_to_32(manifest.sha256)?;
    Ok(sdk_wire_record_bytes(SdkWireRecord::SignerPolicy(
        edgerun_wire::SignerPolicy {
            abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
            flags: 1,
            entries: public_keys
                .iter()
                .map(|public_key| {
                    if public_key.len() != 32 {
                        return Err("ed25519 public key must be 32 bytes".to_owned());
                    }
                    Ok(edgerun_wire::SignerPolicyEntry {
                        algorithm: ESIG_ALGORITHM_ED25519,
                        segment_sha256,
                        public_key: public_key.clone(),
                        segment_id: manifest.id.as_bytes().to_vec(),
                        node_role: manifest.node_role.as_bytes().to_vec(),
                        capability: manifest.capability.as_bytes().to_vec(),
                    })
                })
                .collect::<Result<Vec<_>, String>>()?,
        },
    )))
}

pub(crate) fn parse_artifact_signature_record(
    bytes: &[u8],
) -> Option<edgerun_wire::ArtifactSignature> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned).ok()? {
        SdkWireRecord::ArtifactSignature(signature)
            if signature.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && signature.flags & 1 == 1 =>
        {
            Some(signature)
        }
        _ => None,
    }
}

pub(crate) fn parse_signer_policy_record(bytes: &[u8]) -> Option<edgerun_wire::SignerPolicy> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned).ok()? {
        SdkWireRecord::SignerPolicy(policy)
            if policy.abi_version == edgerun_wire::SDK_WIRE_ABI_VERSION
                && policy.flags & 1 == 1 =>
        {
            Some(policy)
        }
        _ => None,
    }
}

pub(crate) fn verify_binary_signature(
    report_bytes: &[u8],
    signature: &edgerun_wire::ArtifactSignature,
) -> bool {
    verify_signature_for_domain(report_bytes, signature, ESIG_DOMAIN)
}

pub(crate) fn verify_signature_for_domain(
    artifact_bytes: &[u8],
    signature: &edgerun_wire::ArtifactSignature,
    domain: &[u8],
) -> bool {
    if signature.algorithm != ESIG_ALGORITHM_ED25519
        || signature.public_key.len() != 32
        || signature.signature.len() != 64
    {
        return false;
    }
    let artifact_hash = sha256(artifact_bytes);
    if signature.artifact_sha256 != artifact_hash {
        return false;
    }
    let Ok(public_key_bytes) = <[u8; 32]>::try_from(signature.public_key.as_slice()) else {
        return false;
    };
    let Ok(signature_bytes) = <[u8; 64]>::try_from(signature.signature.as_slice()) else {
        return false;
    };
    let Ok(public_key) = VerifyingKey::from_bytes(&public_key_bytes) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);
    public_key
        .verify(
            &signature_payload_for_domain(domain, &artifact_hash),
            &signature,
        )
        .is_ok()
}

pub(crate) fn verify_binary_signer_policy(
    manifest: &SegmentManifest,
    signature: &edgerun_wire::ArtifactSignature,
    policy: &edgerun_wire::SignerPolicy,
) -> bool {
    let Ok(segment_sha256) = hex_to_32(manifest.sha256) else {
        return false;
    };
    policy.entries.iter().any(|entry| {
        entry.algorithm == signature.algorithm
            && entry.algorithm == ESIG_ALGORITHM_ED25519
            && entry.public_key == signature.public_key
            && entry.segment_id == manifest.id.as_bytes()
            && entry.node_role == manifest.node_role.as_bytes()
            && entry.capability == manifest.capability.as_bytes()
            && entry.segment_sha256 == segment_sha256
    })
}

pub(crate) fn signature_payload(report_hash: &[u8; 32]) -> Vec<u8> {
    signature_payload_for_domain(ESIG_DOMAIN, report_hash)
}

pub(crate) fn signature_payload_for_domain(domain: &[u8], report_hash: &[u8; 32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(domain.len() + report_hash.len());
    payload.extend_from_slice(domain);
    payload.extend_from_slice(report_hash);
    payload
}

pub(crate) fn verify_chain_reports(
    manifest: &ChainManifest,
    reports: &[edgerun_wire::SegmentReportRecord],
) -> bool {
    if reports.len() != manifest.segment_count as usize {
        return false;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Ok(bytes) = fs::read(root.join(manifest.path)) else {
        return false;
    };
    let Some(chain) = parse_chain(&bytes) else {
        return false;
    };
    if !verify_binary_chain(manifest, &chain) {
        return false;
    }
    for (index, report) in reports.iter().enumerate() {
        let Some(segment_id) = manifest.segments.get(index) else {
            return false;
        };
        let Some(segment_manifest) = segment(segment_id) else {
            return false;
        };
        if !verify_binary_segment_report(segment_manifest, report) {
            return false;
        }
    }
    for link in &chain.links {
        let Some(from) = reports.get(link.from_segment as usize) else {
            return false;
        };
        let Some(to) = reports.get(link.to_segment as usize) else {
            return false;
        };
        let Some(input_hash) = to.input_sha256.get(link.to_input as usize) else {
            return false;
        };
        if from.output_sha256 != *input_hash {
            return false;
        }
    }
    true
}

pub(crate) fn verify_http_auth_preflight_composition(
    manifest: &CompositionManifest,
    bytes: &[u8],
) -> bool {
    let Some(parsed) = parse_composition(bytes) else {
        return false;
    };
    let vectors = [
        (
            b"Authorization: Bearer abc".as_slice(),
            b"authorization".as_slice(),
            0u8,
        ),
        (
            b"X-Other: Bearer abc".as_slice(),
            b"authorization".as_slice(),
            2u8,
        ),
        (
            b"Authorization Bearer abc".as_slice(),
            b"authorization".as_slice(),
            1u8,
        ),
    ];
    vectors.into_iter().all(|(header, expected_name, status)| {
        let Ok(report) = execute_composition(manifest, &parsed, &[header, expected_name]) else {
            return false;
        };
        report.output == [status] && report.cost > 0
    })
}

pub(crate) fn verify_auth_decision_private_composition(
    manifest: &CompositionManifest,
    bytes: &[u8],
) -> bool {
    let Some(parsed) = parse_composition(bytes) else {
        return false;
    };
    let vectors = [
        (b"\x00".as_slice(), b"\x00".as_slice(), 0u8),
        (b"\x01".as_slice(), b"\x00".as_slice(), 1u8),
    ];
    vectors
        .into_iter()
        .all(|(linked_status, private_expected, status)| {
            let Ok(report) =
                execute_composition(manifest, &parsed, &[linked_status, private_expected])
            else {
                return false;
            };
            report.output == [status] && report.cost > 0
        })
}

pub(crate) fn verify_hmac_sha256_verify_composition(
    manifest: &CompositionManifest,
    bytes: &[u8],
) -> bool {
    let Some(parsed) = parse_composition(bytes) else {
        return false;
    };
    let vectors = [
        (
            b"Jefe".as_slice(),
            b"what do ya want for nothing?".as_slice(),
            hex_to_32("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843")
                .expect("tag")
                .to_vec(),
            0u8,
        ),
        (
            b"Jefe".as_slice(),
            b"what do ya want for nothing?".as_slice(),
            [0u8; 32].to_vec(),
            1u8,
        ),
        (
            b"Jefe".as_slice(),
            b"what do ya want for nothing?".as_slice(),
            [0u8; 31].to_vec(),
            2u8,
        ),
    ];
    vectors.into_iter().all(|(key, message, tag, status)| {
        let Ok(report) = execute_composition(manifest, &parsed, &[key, message, tag.as_slice()])
        else {
            return false;
        };
        report.output == [status] && report.cost > 0
    })
}

pub(crate) fn verify_hmac_sha256_composition(manifest: &CompositionManifest, bytes: &[u8]) -> bool {
    let Some(parsed) = parse_composition(bytes) else {
        return false;
    };
    let vectors = [
        (
            vec![0x0b; 20],
            b"Hi There".to_vec(),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
        ),
        (
            b"Jefe".to_vec(),
            b"what do ya want for nothing?".to_vec(),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
        ),
        (
            vec![0xaa; 131],
            b"Test Using Larger Than Block-Size Key - Hash Key First".to_vec(),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54",
        ),
    ];
    vectors.into_iter().all(|(key, data, expected)| {
        let Ok(report) = execute_composition(manifest, &parsed, &[&key, &data]) else {
            return false;
        };
        report.output.len() == 32 && report.cost > 0 && bytes_to_hex(&report.output) == expected
    })
}

pub(crate) fn verify_hkdf_expand_sha256_l42_composition(
    manifest: &CompositionManifest,
    bytes: &[u8],
) -> bool {
    let Some(parsed) = parse_composition(bytes) else {
        return false;
    };
    let prk = parse_hex("077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5")
        .expect("valid hex");
    let info = parse_hex("f0f1f2f3f4f5f6f7f8f9").expect("valid hex");
    let expected = concat!(
        "3cb25f25faacd57a90434f64d0362f2a",
        "2d2d0a90cf1a5a4c5db02d56ecc4c5bf",
        "34007208d5b887185865"
    );
    let Ok(report) = execute_composition(manifest, &parsed, &[&prk, &info]) else {
        return false;
    };
    report.output.len() == 42 && report.cost > 0 && bytes_to_hex(&report.output) == expected
}

pub(crate) fn verify_hkdf_extract_sha256_composition(
    manifest: &CompositionManifest,
    bytes: &[u8],
) -> bool {
    let Some(parsed) = parse_composition(bytes) else {
        return false;
    };
    let salt = parse_hex("000102030405060708090a0b0c").expect("valid hex");
    let ikm = parse_hex("0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b").expect("valid hex");
    let expected = "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5";
    let Ok(report) = execute_composition(manifest, &parsed, &[&salt, &ikm]) else {
        return false;
    };
    report.output.len() == 32 && report.cost > 0 && bytes_to_hex(&report.output) == expected
}

pub(crate) fn verify_unit(manifest: &UnitManifest) -> bool {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let wasm_path = root.join(manifest.wasm_path);
    let manifest_path = root.join(manifest.manifest_path);
    let mut ok = true;

    println!("verify {}", manifest.id);

    match fs::read(&wasm_path) {
        Ok(bytes) => {
            let actual = String::from_utf8_lossy(&sha256_hex(&bytes)).into_owned();
            let hash_ok = str_eq(&actual, manifest.wasm_sha256);
            ok &= hash_ok;
            print_check("wasm-sha256", hash_ok);
            if !hash_ok {
                println!("    expected: {}", manifest.wasm_sha256);
                println!("    actual:   {actual}");
            }
        }
        Err(err) => {
            ok = false;
            print_check("wasm-readable", false);
            println!("    {}: {err}", wasm_path.display());
        }
    }

    match fs::read(&manifest_path) {
        Ok(bytes) => {
            let manifest_ok = verify_binary_manifest(manifest, &bytes);
            ok &= manifest_ok;
            print_check("binary-manifest", manifest_ok);
        }
        Err(err) => {
            ok = false;
            print_check("binary-manifest", false);
            println!("    {}: {err}", manifest_path.display());
        }
    }

    let api_path = wasm_path.with_file_name("api.edm");
    match fs::read(&api_path) {
        Ok(bytes) => {
            let api_ok = verify_binary_api(manifest, &bytes);
            ok &= api_ok;
            print_check("binary-api", api_ok);
        }
        Err(err) => {
            ok = false;
            print_check("binary-api", false);
            println!("    {}: {err}", api_path.display());
        }
    }

    match fs::read(&wasm_path) {
        Ok(bytes) => match WasmSurface::parse(&bytes) {
            Some(surface) => {
                let validate_ok = surface.valid;
                ok &= validate_ok;
                print_check("wasm-validate", validate_ok);

                let unit_surface_ok = validate_wasm_unit_surface(manifest.id, &surface).is_ok();
                ok &= unit_surface_ok;
                print_check("unit-surface-policy", unit_surface_ok);

                let identity_ok =
                    verify_unit_identity_exports(&surface, manifest.standard_id).is_ok();
                ok &= identity_ok;
                print_check("unit-identity-exports", identity_ok);

                let surface_ok = verify_surface(manifest, &surface);
                ok &= surface_ok;
                print_check("wasm-surface", surface_ok);

                let no_shared_memory = !surface.imports_memory;
                ok &= no_shared_memory;
                print_check("no-shared-memory-import", no_shared_memory);
            }
            None => {
                ok = false;
                print_check("wasm-validate", false);
                print_check("wasm-surface", false);
                print_check("no-shared-memory-import", false);
            }
        },
        Err(_) => {
            ok = false;
            print_check("wasm-validate", false);
            print_check("wasm-surface", false);
            print_check("no-shared-memory-import", false);
        }
    }

    if manifest.id == "sha256-fips180" {
        match build_native_sha256_implementation(&root) {
            Ok(Some(_)) => print_check("native-dylib-conformance", true),
            Ok(None) => print_check("native-dylib-conformance", true),
            Err(err) => {
                ok = false;
                print_check("native-dylib-conformance", false);
                println!("    {err}");
            }
        }
    }

    ok
}

pub(crate) fn verify_binary_manifest(expected: &UnitManifest, bytes: &[u8]) -> bool {
    let Some(parsed) = parse_unit_manifest_record(bytes) else {
        return false;
    };
    parsed.abi_version == 2
        && parsed.flags & 1 == 1
        && parsed.flags & 2 == 2
        && parsed.import_count == expected.imports.len() as u16
        && parsed.export_count == expected.exports.len() as u16
        && parsed.standard_id == expected.standard_id
        && parsed.unit_id.as_slice() == expected.id.as_bytes()
        && parsed.standard.as_slice() == expected.standard.as_bytes()
        && bytes_to_hex(&parsed.wasm_sha256) == expected.wasm_sha256
}

pub(crate) fn verify_binary_api(expected: &UnitManifest, bytes: &[u8]) -> bool {
    let Some(parsed) = parse_api(bytes) else {
        return false;
    };
    let expected_functions: Vec<&ApiFunction> = expected
        .exports
        .iter()
        .filter(|export| export.ty != "memory")
        .collect();
    if parsed.functions.len() != expected_functions.len() {
        return false;
    }
    for expected in expected_functions {
        let Some(actual) = parsed
            .functions
            .iter()
            .find(|function| function.name == expected.name.as_bytes())
        else {
            return false;
        };
        let Some((params, results)) = parse_api_type(expected.ty) else {
            return false;
        };
        let Some((cost_base, cost_per_byte)) = api_cost_profile(expected.name) else {
            return false;
        };
        if actual.params != valtypes_to_api_bytes(&params)
            || actual.results != valtypes_to_api_bytes(&results)
            || actual.cost_base != cost_base
            || actual.cost_per_byte != cost_per_byte
        {
            return false;
        }
    }
    true
}

pub(crate) fn api_cost_profile(name: &str) -> Option<(u32, u32)> {
    match name {
        "proto_abi_version" | "proto_standard_id" => Some((1, 0)),
        "sha256_digest" | "sha384_digest" | "sha512_digest" => Some((12, 1)),
        "rfc2104_key_pad" => Some((8, 1)),
        "hmac_sha256" => Some((32, 1)),
        "udp_minimum_length"
        | "tftp_minimum_length"
        | "ipv4_minimum_length"
        | "dns_header_length"
        | "base64url_encoded_len"
        | "base64url_decoded_bound"
        | "base32hex_encoded_len"
        | "base32hex_decoded_bound"
        | "base16_encoded_len"
        | "base16_decoded_bound"
        | "quic_varint_encoded_len"
        | "edgerun_p256_private_key_len"
        | "edgerun_p256_public_key_len"
        | "edgerun_signature_algorithm_p256_sha256"
        | "edgerun_signature_len_p256"
        | "edgerun_signature_input_len"
        | "edgerun_p256_signature_len" => Some((2, 0)),
        "tftp_opcode_valid" | "dns_is_query" | "http_tchar_valid" | "utf8_scalar_width"
        | "cbor_major_valid" => Some((4, 0)),
        "byte_eq" | "byte_prefix" | "byte_find" | "byte_ascii_lower" | "byte_ascii_case_eq" => {
            Some((4, 1))
        }
        "constant_time_eq" => Some((6, 1)),
        "udp_parse"
        | "tftp_parse"
        | "ipv4_parse"
        | "dns_header_parse"
        | "http_token_validate"
        | "utf8_validate"
        | "base64url_encode"
        | "base64url_decode"
        | "base32hex_encode"
        | "base32hex_decode"
        | "base16_encode"
        | "base16_decode"
        | "cbor_head_parse"
        | "http_field_line_parse"
        | "crc32_ieee"
        | "crc32_ieee_write"
        | "quic_varint_encode"
        | "quic_varint_decode"
        | "edgerun_p256_private_key_valid"
        | "edgerun_p256_public_key_from_private"
        | "edgerun_signature_input"
        | "edgerun_p256_raw64_public_key_valid" => Some((8, 1)),
        "edgerun_p256_sign_prehash_input" | "edgerun_p256_verify_prehash_input" => Some((64, 1)),
        "capability_operation_valid" | "capability_access_class_valid" => Some((2, 0)),
        "capability_authorize_invocation"
        | "capability_session_mode_valid"
        | "capability_session_open_validate"
        | "capability_session_accept_unchecked_status"
        | "capability_session_reject_status"
        | "capability_invocation_validate"
        | "capability_result_validate"
        | "capability_result_frame_validate"
        | "decision_byte_eq" => Some((4, 0)),
        _ => None,
    }
}

pub(crate) fn verify_binary_composition(expected: &CompositionManifest, bytes: &[u8]) -> bool {
    let Some(parsed) = parse_composition(bytes) else {
        return false;
    };
    if parsed.id != expected.id.as_bytes()
        || parsed.output_unit != expected.output_unit.as_bytes()
        || parsed.steps.len() != expected.steps as usize
        || parsed.components.len() != expected.components.len()
    {
        return false;
    }
    for expected_component in expected.components {
        let Some(actual) = parsed
            .components
            .iter()
            .find(|component| component.unit_id == expected_component.unit_id.as_bytes())
        else {
            return false;
        };
        if bytes_to_hex(&actual.wasm_sha256) != expected_component.wasm_sha256 {
            return false;
        }
        let Some(unit) = unit(expected_component.unit_id) else {
            return false;
        };
        if unit.wasm_sha256 != expected_component.wasm_sha256 {
            return false;
        }
    }
    for step in &parsed.steps {
        if step.component_index as usize >= parsed.components.len() {
            return false;
        }
        match step.opcode {
            1 => {
                if step.arg0 > 15 || !valid_len_source(step.arg2) {
                    return false;
                }
            }
            2 => {
                let component = &parsed.components[step.component_index as usize];
                let Some(unit) = core::str::from_utf8(&component.unit_id).ok().and_then(unit)
                else {
                    return false;
                };
                let function_exports = unit
                    .exports
                    .iter()
                    .filter(|export| export.ty != "memory")
                    .count();
                if step.function_index as usize >= function_exports {
                    return false;
                }
                if !valid_arg_ref(step.arg0)
                    || !valid_arg_ref(step.arg1)
                    || !valid_arg_ref(step.arg2)
                    || !valid_arg_ref(step.arg3)
                    || !valid_arg_ref(step.arg4)
                {
                    return false;
                }
            }
            3 => {
                if step.arg0 as usize >= parsed.components.len() || !valid_len_source(step.arg3) {
                    return false;
                }
            }
            4 => {
                if step.arg0 > 0 || !valid_len_source(step.arg2) {
                    return false;
                }
            }
            5 => {
                if !valid_comparison(step.arg0)
                    || !valid_comparison(step.arg1)
                    || step.arg2 as usize >= parsed.steps.len()
                    || step.arg3 as usize >= parsed.steps.len()
                {
                    return false;
                }
            }
            6 => {
                if step.arg0 as usize >= parsed.steps.len() {
                    return false;
                }
            }
            7 => {
                if !valid_arg_ref(step.arg0) || step.arg1 > 255 {
                    return false;
                }
            }
            8 => {
                let component = &parsed.components[step.component_index as usize];
                let Some(unit) = core::str::from_utf8(&component.unit_id).ok().and_then(unit)
                else {
                    return false;
                };
                let function_exports = unit
                    .exports
                    .iter()
                    .filter(|export| export.ty != "memory")
                    .count();
                if step.function_index as usize >= function_exports
                    || step.arg4 > 255
                    || !valid_arg_ref(step.arg0)
                    || !valid_arg_ref(step.arg1)
                    || !valid_arg_ref(step.arg2)
                    || !valid_arg_ref(step.arg3)
                {
                    return false;
                }
            }
            9 => {
                if !valid_arg_ref(step.arg0) || step.arg1 > 255 {
                    return false;
                }
            }
            _ => return false,
        }
    }
    true
}

pub(crate) fn valid_len_source(value: u32) -> bool {
    let kind = value >> 24;
    kind <= 3
}

pub(crate) fn valid_arg_ref(value: u32) -> bool {
    let kind = value >> 24;
    kind <= 3
}

pub(crate) fn valid_comparison(value: u32) -> bool {
    let comparison = value >> 28;
    let kind = (value >> 24) & 0x0f;
    comparison <= 3 && kind <= 3
}

pub(crate) struct ComponentRuntime {
    pub(crate) _library: NativeLibrary,
    pub(crate) memory: NativeMemory,
    pub(crate) pointer_args_are_offsets: bool,
    pub(crate) functions: Vec<FunctionRuntime>,
}

#[derive(Clone)]
pub(crate) struct FunctionRuntime {
    pub(crate) func: NativeFunction,
    pub(crate) name: String,
    pub(crate) param_count: usize,
    pub(crate) pointer_args: [bool; 5],
    pub(crate) output_arg: Option<usize>,
    pub(crate) output_pointer_fields: Vec<usize>,
    pub(crate) cost_base: u32,
    pub(crate) cost_per_byte: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NativeAbiFunction {
    pub(crate) name: String,
    pub(crate) pointer_args: [bool; 5],
    pub(crate) output_arg: Option<usize>,
    pub(crate) output_pointer_fields: Vec<usize>,
}

#[derive(Clone, Copy)]
pub(crate) enum NativeFunction {
    Arity0(unsafe extern "C" fn() -> i32),
    Arity1(unsafe extern "C" fn(i32) -> i32),
    Arity2(unsafe extern "C" fn(i32, i32) -> i32),
    Arity3(unsafe extern "C" fn(i32, i32, i32) -> i32),
    Arity4(unsafe extern "C" fn(i32, i32, i32, i32) -> i32),
    Arity5(unsafe extern "C" fn(i32, i32, i32, i32, i32) -> i32),
}

impl NativeFunction {
    unsafe fn load(
        library: &NativeLibrary,
        name: &str,
        param_count: usize,
    ) -> Result<Self, String> {
        match param_count {
            0 => Ok(Self::Arity0(library.symbol(name)?)),
            1 => Ok(Self::Arity1(library.symbol(name)?)),
            2 => Ok(Self::Arity2(library.symbol(name)?)),
            3 => Ok(Self::Arity3(library.symbol(name)?)),
            4 => Ok(Self::Arity4(library.symbol(name)?)),
            5 => Ok(Self::Arity5(library.symbol(name)?)),
            _ => Err(format!(
                "unsupported native function arity: {name}/{param_count}"
            )),
        }
    }

    unsafe fn call(&self, args: &[u32; 5]) -> i32 {
        let arg = |index: usize| args[index] as i32;
        match self {
            Self::Arity0(func) => func(),
            Self::Arity1(func) => func(arg(0)),
            Self::Arity2(func) => func(arg(0), arg(1)),
            Self::Arity3(func) => func(arg(0), arg(1), arg(2)),
            Self::Arity4(func) => func(arg(0), arg(1), arg(2), arg(3)),
            Self::Arity5(func) => func(arg(0), arg(1), arg(2), arg(3), arg(4)),
        }
    }
}

impl ComponentRuntime {
    fn read_memory(&self, offset: usize, out: &mut [u8]) -> Result<(), String> {
        self.memory.read(offset, out)
    }

    fn write_memory(&mut self, offset: usize, input: &[u8]) -> Result<(), String> {
        self.memory.write(offset, input)
    }

    fn native_call_args(
        &self,
        function: &FunctionRuntime,
        args: [u32; 5],
    ) -> Result<[u32; 5], String> {
        if self.pointer_args_are_offsets {
            return Ok(args);
        }
        let mut mapped = args;
        for index in 0..mapped.len() {
            if function.pointer_args[index] {
                mapped[index] = self.memory.native_pointer_arg(mapped[index] as usize)?;
            }
        }
        Ok(mapped)
    }

    fn normalize_native_outputs(
        &mut self,
        function: &FunctionRuntime,
        args: [u32; 5],
    ) -> Result<(), String> {
        if self.pointer_args_are_offsets {
            return Ok(());
        }
        if function.output_pointer_fields.is_empty() {
            return Ok(());
        }
        let Some(out_arg) = function.output_arg else {
            return Err(format!(
                "{} has pointer output fields but no pointer args",
                function.name
            ));
        };
        let out_offset = args[out_arg] as usize;
        for field in &function.output_pointer_fields {
            let offset = out_offset
                .checked_add(field.saturating_mul(4))
                .ok_or_else(|| "native output field overflow".to_owned())?;
            let value = self.memory.read_u32(offset)?;
            if let Some(normalized) = self.memory.native_pointer_to_offset(value) {
                self.memory.write_u32(offset, normalized)?;
            }
        }
        Ok(())
    }
}

pub(crate) struct NativeMemory {
    pub(crate) ptr: *mut u8,
    pub(crate) len: usize,
    pub(crate) owned: bool,
}

impl NativeMemory {
    fn borrowed(ptr: *mut u8, len: usize) -> Self {
        Self {
            ptr,
            len,
            owned: false,
        }
    }

    fn allocate(len: usize) -> Result<Self, String> {
        let ptr = unsafe { map_low_memory(len)? };
        Ok(Self {
            ptr,
            len,
            owned: true,
        })
    }

    fn read(&self, offset: usize, out: &mut [u8]) -> Result<(), String> {
        let end = offset
            .checked_add(out.len())
            .ok_or_else(|| "native memory read overflow".to_owned())?;
        if end > self.len {
            return Err("native memory read out of range".to_owned());
        }
        unsafe {
            out.copy_from_slice(core::slice::from_raw_parts(self.ptr.add(offset), out.len()));
        }
        Ok(())
    }

    fn write(&mut self, offset: usize, input: &[u8]) -> Result<(), String> {
        let end = offset
            .checked_add(input.len())
            .ok_or_else(|| "native memory write overflow".to_owned())?;
        if end > self.len {
            return Err("native memory write out of range".to_owned());
        }
        unsafe {
            core::slice::from_raw_parts_mut(self.ptr.add(offset), input.len())
                .copy_from_slice(input);
        }
        Ok(())
    }

    fn native_pointer_arg(&self, offset: usize) -> Result<u32, String> {
        if offset > self.len {
            return Err("native pointer argument out of range".to_owned());
        }
        let pointer = unsafe { self.ptr.add(offset) } as usize;
        u32::try_from(pointer).map_err(|_| "native pointer does not fit i32 ABI".to_owned())
    }

    fn native_pointer_to_offset(&self, pointer: u32) -> Option<u32> {
        let pointer = pointer as usize;
        let start = self.ptr as usize;
        let end = start.checked_add(self.len)?;
        (start..=end)
            .contains(&pointer)
            .then(|| u32::try_from(pointer - start).ok())
            .flatten()
    }

    fn read_u32(&self, offset: usize) -> Result<u32, String> {
        let mut bytes = [0u8; 4];
        self.read(offset, &mut bytes)?;
        Ok(u32::from_le_bytes(bytes))
    }

    fn write_u32(&mut self, offset: usize, value: u32) -> Result<(), String> {
        self.write(offset, &value.to_le_bytes())
    }
}

impl Drop for NativeMemory {
    fn drop(&mut self) {
        if self.owned {
            #[cfg(unix)]
            unsafe {
                let _ = munmap(self.ptr.cast::<core::ffi::c_void>(), self.len);
            }
        }
    }
}

pub(crate) struct ExecutionReport {
    pub(crate) output: Vec<u8>,
    pub(crate) cost: u64,
}

pub(crate) struct CostQuote {
    pub(crate) cost: u64,
    pub(crate) output_len: u32,
    pub(crate) steps_executed: usize,
}

pub(crate) struct PreflightReport {
    pub(crate) cost_min: u64,
    pub(crate) cost_max: u64,
    pub(crate) output_len: Option<u32>,
    pub(crate) steps_min: usize,
    pub(crate) steps_max: usize,
    pub(crate) unknowns: Vec<String>,
    pub(crate) possible_failures: Vec<String>,
}

#[derive(Clone)]
pub(crate) struct PreflightState {
    pub(crate) pc: usize,
    pub(crate) cost_min: u64,
    pub(crate) cost_max: u64,
    pub(crate) steps: usize,
    pub(crate) output_len: Option<u32>,
    pub(crate) scalars: [SymVal; 256],
}

#[derive(Clone, Copy)]
pub(crate) enum SymVal {
    Range(u32, u32),
}

impl SymVal {
    const fn exact(value: u32) -> Self {
        Self::Range(value, value)
    }

    const fn range(min: u32, max: u32) -> Self {
        Self::Range(min, max)
    }

    const fn min(self) -> u32 {
        match self {
            Self::Range(min, _) => min,
        }
    }

    const fn max(self) -> u32 {
        match self {
            Self::Range(_, max) => max,
        }
    }

    const fn exact_value(self) -> Option<u32> {
        match self {
            Self::Range(min, max) if min == max => Some(min),
            _ => None,
        }
    }
}

pub(crate) fn execute_composition(
    manifest: &CompositionManifest,
    composition: &edgerun_wire::CompositionRecord,
    inputs: &[&[u8]],
) -> Result<ExecutionReport, String> {
    if composition.id != manifest.id.as_bytes() {
        return Err("composition id mismatch".to_owned());
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut components = Vec::with_capacity(composition.components.len());
    for component in &composition.components {
        let unit_id = core::str::from_utf8(&component.unit_id).map_err(|_| "bad unit id")?;
        let unit = unit(unit_id).ok_or_else(|| format!("unknown unit: {unit_id}"))?;
        if bytes_to_hex(&component.wasm_sha256) != unit.wasm_sha256 {
            return Err(format!("component hash mismatch: {unit_id}"));
        }
        components.push(load_component_runtime(&root, unit_id)?);
    }

    let mut pc = 0usize;
    let mut output = Vec::new();
    let mut cost = 0u64;
    let mut executed = 0usize;
    let mut scalars = [0u32; 256];
    while pc < composition.steps.len() {
        if executed > composition.steps.len().saturating_mul(4) {
            return Err("composition step limit exceeded".to_owned());
        }
        executed += 1;
        let step = &composition.steps[pc];
        match step.opcode {
            1 => {
                let input = inputs
                    .get(step.arg0 as usize)
                    .ok_or_else(|| "input index out of range".to_owned())?;
                let len = resolve_ref(step.arg2, inputs, &scalars)? as usize;
                if len > input.len() {
                    return Err("input length out of range".to_owned());
                }
                cost = cost.saturating_add(len as u64);
                let target = component_mut(&mut components, step.component_index)?;
                target.write_memory(step.arg1 as usize, &input[..len])?;
                pc += 1;
            }
            2 => {
                let target = component_mut(&mut components, step.component_index)?;
                let function = target
                    .functions
                    .get(step.function_index as usize)
                    .cloned()
                    .ok_or_else(|| "function index out of range".to_owned())?;
                let args = [
                    resolve_ref(step.arg0, inputs, &scalars)?,
                    resolve_ref(step.arg1, inputs, &scalars)?,
                    resolve_ref(step.arg2, inputs, &scalars)?,
                    resolve_ref(step.arg3, inputs, &scalars)?,
                    resolve_ref(step.arg4, inputs, &scalars)?,
                ];
                let meter = metered_len_u32(&function.name, &args);
                cost = cost.saturating_add(
                    function.cost_base as u64
                        + (function.cost_per_byte as u64).saturating_mul(meter as u64),
                );
                let native_args = target.native_call_args(&function, args)?;
                let status = unsafe { function.func.call(&native_args) };
                if status != 0 {
                    return Err(format!("component call failed with status {status}"));
                }
                target.normalize_native_outputs(&function, args)?;
                pc += 1;
            }
            3 => {
                let len = resolve_ref(step.arg3, inputs, &scalars)? as usize;
                cost = cost.saturating_add(len as u64);
                let source_index = step.arg0 as usize;
                let mut buffer = vec![0u8; len];
                {
                    let source = components
                        .get(source_index)
                        .ok_or_else(|| "source component index out of range".to_owned())?;
                    source.read_memory(
                        resolve_ref(step.arg1, inputs, &scalars)? as usize,
                        &mut buffer,
                    )?;
                }
                let target_ptr = resolve_ref(step.arg2, inputs, &scalars)? as usize;
                let target = component_mut(&mut components, step.component_index)?;
                target.write_memory(target_ptr, &buffer)?;
                pc += 1;
            }
            4 => {
                let len = resolve_ref(step.arg2, inputs, &scalars)? as usize;
                cost = cost.saturating_add(len as u64);
                output.resize(len, 0);
                let source = components
                    .get(step.component_index as usize)
                    .ok_or_else(|| "output component index out of range".to_owned())?;
                source.read_memory(
                    resolve_ref(step.arg1, inputs, &scalars)? as usize,
                    &mut output,
                )?;
                pc += 1;
            }
            5 => {
                cost = cost.saturating_add(1);
                pc = if compare_refs(step.arg0, step.arg1, inputs, &scalars)? {
                    step.arg2 as usize
                } else {
                    step.arg3 as usize
                };
            }
            6 => {
                cost = cost.saturating_add(1);
                pc = step.arg0 as usize;
            }
            7 => {
                cost = cost.saturating_add(1);
                let ptr = resolve_ref(step.arg0, inputs, &scalars)? as usize;
                let target = component_mut(&mut components, step.component_index)?;
                target.write_memory(ptr, &[step.arg1 as u8])?;
                pc += 1;
            }
            8 => {
                let target = component_mut(&mut components, step.component_index)?;
                let function = target
                    .functions
                    .get(step.function_index as usize)
                    .cloned()
                    .ok_or_else(|| "function index out of range".to_owned())?;
                let args = [
                    resolve_ref(step.arg0, inputs, &scalars)?,
                    resolve_ref(step.arg1, inputs, &scalars)?,
                    resolve_ref(step.arg2, inputs, &scalars)?,
                    resolve_ref(step.arg3, inputs, &scalars)?,
                    0,
                ];
                let meter = metered_len_u32(&function.name, &args);
                cost = cost.saturating_add(
                    function.cost_base as u64
                        + (function.cost_per_byte as u64).saturating_mul(meter as u64),
                );
                let native_args = target.native_call_args(&function, args)?;
                scalars[step.arg4 as usize] = unsafe { function.func.call(&native_args) } as u32;
                target.normalize_native_outputs(&function, args)?;
                pc += 1;
            }
            9 => {
                cost = cost.saturating_add(1);
                let ptr = resolve_ref(step.arg0, inputs, &scalars)? as usize;
                let source = components
                    .get(step.component_index as usize)
                    .ok_or_else(|| "source component index out of range".to_owned())?;
                let mut bytes = [0u8; 4];
                source.read_memory(ptr, &mut bytes)?;
                scalars[step.arg1 as usize] = u32::from_le_bytes(bytes);
                pc += 1;
            }
            _ => return Err("unknown opcode".to_owned()),
        }
    }
    Ok(ExecutionReport { output, cost })
}

pub(crate) fn load_component_runtime(
    root: &Path,
    unit_id: &str,
) -> Result<ComponentRuntime, String> {
    let unit = unit(unit_id).ok_or_else(|| format!("unknown unit: {unit_id}"))?;
    let native_path = native_unit_library_path(root, unit_id)?;
    let library = unsafe { NativeLibrary::open(&native_path)? };
    let (memory, pointer_args_are_offsets) = match unsafe {
        library.symbol::<unsafe extern "C" fn() -> *mut u8>("edgerun_native_memory_ptr")
    } {
        Ok(memory_ptr) => {
            let memory_len_fn: unsafe extern "C" fn() -> usize =
                unsafe { library.symbol("edgerun_native_memory_len")? };
            let ptr = unsafe { memory_ptr() };
            let len = unsafe { memory_len_fn() };
            if ptr.is_null() || len == 0 {
                return Err(format!("bad native memory export: {unit_id}"));
            }
            (NativeMemory::borrowed(ptr, len), true)
        }
        Err(_) => (NativeMemory::allocate(65_536)?, false),
    };
    let api_path = root.join(unit.wasm_path).with_file_name("api.edm");
    let api_bytes = fs::read(&api_path).map_err(|err| err.to_string())?;
    let api = parse_api(&api_bytes).ok_or_else(|| format!("bad api: {unit_id}"))?;
    let native_abi = native_abi_for_unit(root, unit_id)?;
    let mut functions = Vec::new();
    for export in unit.exports.iter().filter(|export| export.ty != "memory") {
        let Some((params, _)) = parse_api_type(export.ty) else {
            return Err(format!("bad api type: {unit_id}.{}", export.name));
        };
        let func = unsafe { NativeFunction::load(&library, export.name, params.len()) }
            .map_err(|err| format!("{unit_id}: {err}"))?;
        let Some(api_func) = api
            .functions
            .iter()
            .find(|function| function.name == export.name.as_bytes())
        else {
            return Err(format!("missing api cost: {unit_id}.{}", export.name));
        };
        let abi_func = native_abi
            .iter()
            .find(|function| function.name == export.name)
            .cloned()
            .unwrap_or_else(|| NativeAbiFunction {
                name: export.name.to_owned(),
                pointer_args: [false; 5],
                output_arg: None,
                output_pointer_fields: Vec::new(),
            });
        functions.push(FunctionRuntime {
            func,
            name: export.name.to_owned(),
            param_count: params.len(),
            pointer_args: abi_func.pointer_args,
            output_arg: abi_func.output_arg,
            output_pointer_fields: abi_func.output_pointer_fields,
            cost_base: api_func.cost_base,
            cost_per_byte: api_func.cost_per_byte,
        });
    }
    Ok(ComponentRuntime {
        _library: library,
        memory,
        pointer_args_are_offsets,
        functions,
    })
}

pub(crate) fn native_abi_for_unit(
    root: &Path,
    unit_id: &str,
) -> Result<Vec<NativeAbiFunction>, String> {
    let path = root.join("units").join(unit_id).join("rust/src/lib.rs");
    let source = fs::read_to_string(&path)
        .map_err(|err| format!("cannot read native abi source {}: {err}", path.display()))?;
    parse_native_abi_source(&source).map_err(|err| format!("{unit_id}: {err}"))
}

pub(crate) fn parse_native_abi_source(source: &str) -> Result<Vec<NativeAbiFunction>, String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < lines.len() {
        if !lines[index].contains("#[edgerun_unit::export]") {
            index += 1;
            continue;
        }
        index += 1;
        let mut signature = String::new();
        while index < lines.len() {
            signature.push_str(lines[index].trim());
            signature.push(' ');
            if lines[index].contains('{') {
                break;
            }
            index += 1;
        }
        let (name, arg_names) = parse_export_signature(&signature)?;
        let mut pointer_args = [false; 5];
        let mut output_arg = None;
        for (arg_index, arg_name) in arg_names.iter().take(5).enumerate() {
            if native_arg_is_pointer(arg_name) {
                pointer_args[arg_index] = true;
            }
            if native_arg_is_output_pointer(arg_name) {
                output_arg = Some(arg_index);
            }
        }
        let body_start = index.saturating_add(1);
        let mut body_end = body_start;
        while body_end < lines.len() && !lines[body_end].contains("#[edgerun_unit::export]") {
            body_end += 1;
        }
        let output_pointer_fields =
            parse_native_output_pointer_fields(&lines[body_start..body_end], &arg_names);
        out.push(NativeAbiFunction {
            name,
            pointer_args,
            output_arg,
            output_pointer_fields,
        });
        index = body_end;
    }
    Ok(out)
}

pub(crate) fn parse_export_signature(signature: &str) -> Result<(String, Vec<String>), String> {
    let Some(fn_start) = signature.find("fn ") else {
        return Err(format!("export missing fn signature: {signature}"));
    };
    let name_start = fn_start + 3;
    let Some(open_offset) = signature[name_start..].find('(') else {
        return Err(format!("export missing argument list: {signature}"));
    };
    let open = name_start + open_offset;
    let name = signature[name_start..open].trim().to_owned();
    let Some(close_offset) = signature[open + 1..].find(')') else {
        return Err(format!("export has unterminated argument list: {name}"));
    };
    let args = &signature[open + 1..open + 1 + close_offset];
    let arg_names = args
        .split(',')
        .filter_map(|arg| arg.split_once(':').map(|(name, _)| name.trim().to_owned()))
        .filter(|name| !name.is_empty())
        .collect();
    Ok((name, arg_names))
}

pub(crate) fn native_arg_is_pointer(name: &str) -> bool {
    name == "ptr" || name.ends_with("_ptr") || name.ends_with("_out")
}

pub(crate) fn native_arg_is_output_pointer(name: &str) -> bool {
    name == "out_ptr"
        || name.ends_with("_out_ptr")
        || name.ends_with("output_ptr")
        || name.ends_with("_out")
}

pub(crate) fn parse_native_output_pointer_fields(
    lines: &[&str],
    arg_names: &[String],
) -> Vec<usize> {
    let pointer_args: Vec<&str> = arg_names
        .iter()
        .map(String::as_str)
        .filter(|name| native_arg_is_pointer(name) && !native_arg_is_output_pointer(name))
        .collect();
    let mut fields = Vec::new();
    let mut current_field = None;
    for line in lines {
        if let Some(field) = parse_out_add_field(line) {
            current_field = Some(field);
        }
        let Some(field) = current_field else {
            continue;
        };
        if line.contains("as u32")
            && pointer_args.iter().any(|arg| {
                line.contains(&format!("({arg} +")) || line.contains(&format!("{arg} +"))
            })
            && !fields.contains(&field)
        {
            fields.push(field);
        }
    }
    fields
}

pub(crate) fn parse_out_add_field(line: &str) -> Option<usize> {
    let start = line.find("out.add(")? + "out.add(".len();
    let end = line[start..].find(')')?;
    line[start..start + end].trim().parse().ok()
}

pub(crate) fn quote_composition(
    manifest: &CompositionManifest,
    composition: &edgerun_wire::CompositionRecord,
    input_lengths: &[u32],
) -> Result<CostQuote, String> {
    if composition.id != manifest.id.as_bytes() {
        return Err("composition id mismatch".to_owned());
    }
    let function_tables = load_function_tables(composition)?;
    let mut pc = 0usize;
    let mut cost = 0u64;
    let mut output_len = 0u32;
    let mut executed = 0usize;
    let mut scalars = [None; 256];
    while pc < composition.steps.len() {
        if executed > composition.steps.len().saturating_mul(4) {
            return Err("composition step limit exceeded".to_owned());
        }
        executed += 1;
        let step = &composition.steps[pc];
        match step.opcode {
            1 => {
                let len = resolve_ref_len(step.arg2, input_lengths, &scalars)?;
                let input_len = *input_lengths
                    .get(step.arg0 as usize)
                    .ok_or_else(|| "input index out of range".to_owned())?;
                if len > input_len {
                    return Err("input length out of range".to_owned());
                }
                cost = cost.saturating_add(len as u64);
                pc += 1;
            }
            2 => {
                let function = function_tables
                    .get(step.component_index as usize)
                    .and_then(|functions| functions.get(step.function_index as usize))
                    .ok_or_else(|| "function index out of range".to_owned())?;
                let args = [
                    resolve_ref_len(step.arg0, input_lengths, &scalars)?,
                    resolve_ref_len(step.arg1, input_lengths, &scalars)?,
                    resolve_ref_len(step.arg2, input_lengths, &scalars)?,
                    resolve_ref_len(step.arg3, input_lengths, &scalars)?,
                    resolve_ref_len(step.arg4, input_lengths, &scalars)?,
                ];
                let meter = metered_len_u32(&function.name, &args);
                cost = cost.saturating_add(
                    function.cost_base as u64
                        + (function.cost_per_byte as u64).saturating_mul(meter as u64),
                );
                pc += 1;
            }
            3 => {
                let len = resolve_ref_len(step.arg3, input_lengths, &scalars)?;
                cost = cost.saturating_add(len as u64);
                pc += 1;
            }
            4 => {
                output_len = resolve_ref_len(step.arg2, input_lengths, &scalars)?;
                cost = cost.saturating_add(output_len as u64);
                pc += 1;
            }
            5 => {
                cost = cost.saturating_add(1);
                pc = if compare_refs_len(step.arg0, step.arg1, input_lengths, &scalars)? {
                    step.arg2 as usize
                } else {
                    step.arg3 as usize
                };
            }
            6 => {
                cost = cost.saturating_add(1);
                pc = step.arg0 as usize;
            }
            7 => {
                cost = cost.saturating_add(1);
                let _ptr = resolve_ref_len(step.arg0, input_lengths, &scalars)?;
                pc += 1;
            }
            8 => {
                let function = function_tables
                    .get(step.component_index as usize)
                    .and_then(|functions| functions.get(step.function_index as usize))
                    .ok_or_else(|| "function index out of range".to_owned())?;
                let args = [
                    resolve_ref_len(step.arg0, input_lengths, &scalars)?,
                    resolve_ref_len(step.arg1, input_lengths, &scalars)?,
                    resolve_ref_len(step.arg2, input_lengths, &scalars)?,
                    resolve_ref_len(step.arg3, input_lengths, &scalars)?,
                    0,
                ];
                let meter = metered_len_u32(&function.name, &args);
                cost = cost.saturating_add(
                    function.cost_base as u64
                        + (function.cost_per_byte as u64).saturating_mul(meter as u64),
                );
                scalars[step.arg4 as usize] = None;
                pc += 1;
            }
            9 => {
                cost = cost.saturating_add(1);
                let _ptr = resolve_ref_len(step.arg0, input_lengths, &scalars)?;
                scalars[step.arg1 as usize] = None;
                pc += 1;
            }
            _ => return Err("unknown opcode".to_owned()),
        }
    }
    Ok(CostQuote {
        cost,
        output_len,
        steps_executed: executed,
    })
}

pub(crate) fn preflight_composition(
    manifest: &CompositionManifest,
    composition: &edgerun_wire::CompositionRecord,
    input_lengths: &[u32],
) -> Result<PreflightReport, String> {
    if composition.id != manifest.id.as_bytes() {
        return Err("composition id mismatch".to_owned());
    }
    let function_tables = load_function_tables(composition)?;
    let max_input_len = input_lengths.iter().copied().max().unwrap_or(0);
    let initial = PreflightState {
        pc: 0,
        cost_min: 0,
        cost_max: 0,
        steps: 0,
        output_len: None,
        scalars: [SymVal::exact(0); 256],
    };
    let mut stack = vec![initial];
    let mut exits = Vec::new();
    let mut unknowns = Vec::new();
    let mut possible_failures = Vec::new();
    let mut processed = 0usize;
    while let Some(mut state) = stack.pop() {
        while state.pc < composition.steps.len() {
            if state.steps > composition.steps.len().saturating_mul(4) {
                possible_failures.push(format!("step {} exceeded symbolic step budget", state.pc));
                break;
            }
            processed += 1;
            if processed > composition.steps.len().saturating_mul(64).max(64) {
                possible_failures.push("symbolic path budget exceeded".to_owned());
                break;
            }
            state.steps += 1;
            let step_index = state.pc;
            let step = &composition.steps[step_index];
            match step.opcode {
                1 => {
                    let len = resolve_sym_ref(step.arg2, input_lengths, &state.scalars)?;
                    let input_len = *input_lengths
                        .get(step.arg0 as usize)
                        .ok_or_else(|| format!("step {step_index}: input index out of range"))?;
                    if len.max() > input_len {
                        possible_failures
                            .push(format!("step {step_index}: copy may exceed input length"));
                    }
                    state.cost_min = state.cost_min.saturating_add(len.min() as u64);
                    state.cost_max = state.cost_max.saturating_add(len.max() as u64);
                    state.pc += 1;
                }
                2 | 8 => {
                    let function = function_tables
                        .get(step.component_index as usize)
                        .and_then(|functions| functions.get(step.function_index as usize))
                        .ok_or_else(|| format!("step {step_index}: function index out of range"))?;
                    let args = [
                        resolve_sym_ref(step.arg0, input_lengths, &state.scalars)?,
                        resolve_sym_ref(step.arg1, input_lengths, &state.scalars)?,
                        resolve_sym_ref(step.arg2, input_lengths, &state.scalars)?,
                        resolve_sym_ref(step.arg3, input_lengths, &state.scalars)?,
                        resolve_sym_ref(step.arg4, input_lengths, &state.scalars)?,
                    ];
                    let meter = metered_len_sym(&function.name, &args);
                    state.cost_min = state.cost_min.saturating_add(
                        function.cost_base as u64
                            + (function.cost_per_byte as u64).saturating_mul(meter.min() as u64),
                    );
                    state.cost_max = state.cost_max.saturating_add(
                        function.cost_base as u64
                            + (function.cost_per_byte as u64).saturating_mul(meter.max() as u64),
                    );
                    if step.opcode == 2 {
                        possible_failures.push(format!(
                            "step {step_index}: {} status depends on wasm execution",
                            function.name
                        ));
                    } else {
                        state.scalars[step.arg4 as usize] = captured_result_range(&function.name);
                        unknowns.push(format!(
                            "s{} produced_by step {step_index} {}",
                            step.arg4, function.name
                        ));
                    }
                    state.pc += 1;
                }
                3 => {
                    let len = resolve_sym_ref(step.arg3, input_lengths, &state.scalars)?;
                    state.cost_min = state.cost_min.saturating_add(len.min() as u64);
                    state.cost_max = state.cost_max.saturating_add(len.max() as u64);
                    state.pc += 1;
                }
                4 => {
                    let len = resolve_sym_ref(step.arg2, input_lengths, &state.scalars)?;
                    state.cost_min = state.cost_min.saturating_add(len.min() as u64);
                    state.cost_max = state.cost_max.saturating_add(len.max() as u64);
                    state.output_len = len.exact_value();
                    state.pc += 1;
                }
                5 => {
                    state.cost_min = state.cost_min.saturating_add(1);
                    state.cost_max = state.cost_max.saturating_add(1);
                    match compare_sym_refs(step.arg0, step.arg1, input_lengths, &state.scalars)? {
                        Some(true) => state.pc = step.arg2 as usize,
                        Some(false) => state.pc = step.arg3 as usize,
                        None => {
                            unknowns
                                .push(format!("branch at step {step_index} is input-dependent"));
                            let mut false_state = state.clone();
                            false_state.pc = step.arg3 as usize;
                            stack.push(false_state);
                            state.pc = step.arg2 as usize;
                        }
                    }
                }
                6 => {
                    state.cost_min = state.cost_min.saturating_add(1);
                    state.cost_max = state.cost_max.saturating_add(1);
                    state.pc = step.arg0 as usize;
                }
                7 => {
                    state.cost_min = state.cost_min.saturating_add(1);
                    state.cost_max = state.cost_max.saturating_add(1);
                    state.pc += 1;
                }
                9 => {
                    state.cost_min = state.cost_min.saturating_add(1);
                    state.cost_max = state.cost_max.saturating_add(1);
                    state.scalars[step.arg1 as usize] = SymVal::range(0, max_input_len);
                    unknowns.push(format!(
                        "s{} loaded_from_memory at step {step_index}",
                        step.arg1
                    ));
                    state.pc += 1;
                }
                _ => return Err(format!("step {step_index}: unknown opcode")),
            }
        }
        if state.pc >= composition.steps.len() {
            exits.push((
                state.cost_min,
                state.cost_max,
                state.steps,
                state.output_len,
            ));
        }
    }
    if exits.is_empty() {
        return Err("no terminating symbolic path".to_owned());
    }
    let cost_min = exits.iter().map(|exit| exit.0).min().unwrap_or(0);
    let cost_max = exits.iter().map(|exit| exit.1).max().unwrap_or(0);
    let steps_min = exits.iter().map(|exit| exit.2).min().unwrap_or(0);
    let steps_max = exits.iter().map(|exit| exit.2).max().unwrap_or(0);
    let first_output = exits[0].3;
    let output_len = exits
        .iter()
        .all(|exit| exit.3 == first_output)
        .then_some(first_output)
        .flatten();
    unknowns.sort();
    unknowns.dedup();
    possible_failures.sort();
    possible_failures.dedup();
    Ok(PreflightReport {
        cost_min,
        cost_max,
        output_len,
        steps_min,
        steps_max,
        unknowns,
        possible_failures,
    })
}

#[derive(Clone)]
pub(crate) struct FunctionCost {
    pub(crate) name: String,
    pub(crate) cost_base: u32,
    pub(crate) cost_per_byte: u32,
}

pub(crate) fn load_function_tables(
    composition: &edgerun_wire::CompositionRecord,
) -> Result<Vec<Vec<FunctionCost>>, String> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut tables = Vec::with_capacity(composition.components.len());
    for component in &composition.components {
        let unit_id = core::str::from_utf8(&component.unit_id).map_err(|_| "bad unit id")?;
        let unit = unit(unit_id).ok_or_else(|| format!("unknown unit: {unit_id}"))?;
        let api_path = root.join(unit.wasm_path).with_file_name("api.edm");
        let api_bytes = fs::read(&api_path).map_err(|err| err.to_string())?;
        let api = parse_api(&api_bytes).ok_or_else(|| format!("bad api: {unit_id}"))?;
        let mut table = Vec::new();
        for export in unit.exports.iter().filter(|export| export.ty != "memory") {
            let Some(api_func) = api
                .functions
                .iter()
                .find(|function| function.name == export.name.as_bytes())
            else {
                return Err(format!("missing api function: {unit_id}.{}", export.name));
            };
            table.push(FunctionCost {
                name: export.name.to_owned(),
                cost_base: api_func.cost_base,
                cost_per_byte: api_func.cost_per_byte,
            });
        }
        tables.push(table);
    }
    Ok(tables)
}

pub(crate) fn component_mut(
    components: &mut [ComponentRuntime],
    index: u8,
) -> Result<&mut ComponentRuntime, String> {
    components
        .get_mut(index as usize)
        .ok_or_else(|| "component index out of range".to_owned())
}

pub(crate) fn metered_len_u32(name: &str, args: &[u32; 5]) -> u32 {
    let arg = |index: usize| args[index];
    match name {
        "sha256_digest" | "sha384_digest" | "sha512_digest" => arg(1),
        "rfc2104_key_pad" => arg(2),
        "hmac_sha256" => arg(1).saturating_add(arg(3)),
        "udp_parse" | "tftp_parse" | "ipv4_parse" | "dns_header_parse" => arg(1),
        "http_token_validate"
        | "utf8_validate"
        | "base64url_encode"
        | "base64url_decode"
        | "base32hex_encode"
        | "base32hex_decode"
        | "base16_encode"
        | "base16_decode"
        | "cbor_head_parse"
        | "http_field_line_parse"
        | "byte_find"
        | "byte_ascii_lower"
        | "crc32_ieee"
        | "crc32_ieee_write"
        | "quic_varint_decode"
        | "edgerun_p256_private_key_valid"
        | "edgerun_p256_public_key_from_private"
        | "edgerun_p256_raw64_public_key_valid" => arg(1),
        "quic_varint_encode" => 8,
        "edgerun_signature_input" => arg(1).saturating_add(arg(3)),
        "edgerun_p256_sign_prehash_input" => arg(1).saturating_add(arg(3)),
        "edgerun_p256_verify_prehash_input" => 128u32.saturating_add(arg(2)),
        "constant_time_eq" => arg(1).max(arg(3)),
        "byte_eq" | "byte_ascii_case_eq" => arg(1).max(arg(3)),
        "byte_prefix" => arg(3),
        _ => 1,
    }
}

pub(crate) fn metered_len_sym(name: &str, args: &[SymVal; 5]) -> SymVal {
    let arg = |index: usize| args[index];
    match name {
        "sha256_digest" | "sha384_digest" | "sha512_digest" => arg(1),
        "rfc2104_key_pad" => arg(2),
        "hmac_sha256" => SymVal::range(
            arg(1).min().saturating_add(arg(3).min()),
            arg(1).max().saturating_add(arg(3).max()),
        ),
        "udp_parse"
        | "tftp_parse"
        | "ipv4_parse"
        | "dns_header_parse"
        | "http_token_validate"
        | "utf8_validate"
        | "base64url_encode"
        | "base64url_decode"
        | "base32hex_encode"
        | "base32hex_decode"
        | "base16_encode"
        | "base16_decode"
        | "cbor_head_parse"
        | "http_field_line_parse"
        | "byte_find"
        | "byte_ascii_lower"
        | "crc32_ieee"
        | "crc32_ieee_write"
        | "quic_varint_decode"
        | "edgerun_p256_private_key_valid"
        | "edgerun_p256_public_key_from_private"
        | "edgerun_p256_raw64_public_key_valid" => arg(1),
        "quic_varint_encode" => SymVal::exact(8),
        "edgerun_signature_input" => SymVal::range(
            arg(1).min().saturating_add(arg(3).min()),
            arg(1).max().saturating_add(arg(3).max()),
        ),
        "edgerun_p256_sign_prehash_input" => SymVal::range(
            arg(1).min().saturating_add(arg(3).min()),
            arg(1).max().saturating_add(arg(3).max()),
        ),
        "edgerun_p256_verify_prehash_input" => SymVal::range(
            128u32.saturating_add(arg(2).min()),
            128u32.saturating_add(arg(2).max()),
        ),
        "constant_time_eq" => SymVal::range(
            arg(1).min().max(arg(3).min()),
            arg(1).max().max(arg(3).max()),
        ),
        "byte_eq" | "byte_ascii_case_eq" => SymVal::range(
            arg(1).min().max(arg(3).min()),
            arg(1).max().max(arg(3).max()),
        ),
        "byte_prefix" => arg(3),
        _ => SymVal::exact(1),
    }
}

pub(crate) fn captured_result_range(name: &str) -> SymVal {
    match name {
        "byte_eq" | "byte_prefix" | "byte_ascii_case_eq" | "dns_is_query" | "http_tchar_valid"
        | "tftp_opcode_valid" | "cbor_major_valid" => SymVal::range(0, 1),
        "http_field_line_parse" => SymVal::range(0, 3),
        "utf8_validate" => SymVal::range(0, 5),
        "base64url_decode" => SymVal::range(0, 2),
        "base32hex_decode" => SymVal::range(0, 2),
        "cbor_head_parse" => SymVal::range(0, 3),
        "byte_find" => SymVal::range(0, u32::MAX),
        _ => SymVal::range(0, u32::MAX),
    }
}

pub(crate) fn resolve_sym_ref(
    value: u32,
    input_lengths: &[u32],
    scalars: &[SymVal; 256],
) -> Result<SymVal, String> {
    let kind = value >> 24;
    let payload = value & 0x00ff_ffff;
    match kind {
        0 => Ok(SymVal::exact(payload)),
        1 => input_lengths
            .get(payload as usize)
            .copied()
            .map(SymVal::exact)
            .ok_or_else(|| "input length index out of range".to_owned()),
        2 => scalars
            .get(payload as usize)
            .copied()
            .ok_or_else(|| "scalar index out of range".to_owned()),
        3 => {
            let constant = (payload >> 8) & 0x0000_ffff;
            let input_index = payload & 0x0000_00ff;
            let input_len = *input_lengths
                .get(input_index as usize)
                .ok_or_else(|| "input length index out of range".to_owned())?;
            Ok(SymVal::exact(constant + input_len))
        }
        _ => Err("unsupported reference kind".to_owned()),
    }
}

pub(crate) fn compare_sym_refs(
    left: u32,
    right: u32,
    input_lengths: &[u32],
    scalars: &[SymVal; 256],
) -> Result<Option<bool>, String> {
    let comparison = left >> 28;
    let left = resolve_sym_ref(left & 0x0fff_ffff, input_lengths, scalars)?;
    let right = resolve_sym_ref(right & 0x0fff_ffff, input_lengths, scalars)?;
    let known = match comparison {
        1 if left.min() > right.max() => Some(true),
        1 if left.max() <= right.min() => Some(false),
        2 if left.exact_value().is_some() && left.exact_value() == right.exact_value() => {
            Some(true)
        }
        2 if left.max() < right.min() || right.max() < left.min() => Some(false),
        3 if left.exact_value().is_some() && left.exact_value() == right.exact_value() => {
            Some(false)
        }
        3 if left.max() < right.min() || right.max() < left.min() => Some(true),
        1..=3 => None,
        _ => return Err("unsupported comparison".to_owned()),
    };
    Ok(known)
}

pub(crate) fn resolve_ref(
    value: u32,
    inputs: &[&[u8]],
    scalars: &[u32; 256],
) -> Result<u32, String> {
    let kind = value >> 24;
    let payload = value & 0x00ff_ffff;
    match kind {
        0 => Ok(payload),
        1 => inputs
            .get(payload as usize)
            .map(|input| input.len() as u32)
            .ok_or_else(|| "input length index out of range".to_owned()),
        2 => scalars
            .get(payload as usize)
            .copied()
            .ok_or_else(|| "scalar index out of range".to_owned()),
        3 => {
            let constant = (payload >> 8) & 0x0000_ffff;
            let input_index = payload & 0x0000_00ff;
            let input_len = inputs
                .get(input_index as usize)
                .ok_or_else(|| "input length index out of range".to_owned())?
                .len() as u32;
            Ok(constant + input_len)
        }
        _ => Err("unsupported reference kind".to_owned()),
    }
}

pub(crate) fn resolve_ref_len(
    value: u32,
    input_lengths: &[u32],
    scalars: &[Option<u32>; 256],
) -> Result<u32, String> {
    let kind = value >> 24;
    let payload = value & 0x00ff_ffff;
    match kind {
        0 => Ok(payload),
        1 => input_lengths
            .get(payload as usize)
            .copied()
            .ok_or_else(|| "input length index out of range".to_owned()),
        2 => scalars
            .get(payload as usize)
            .copied()
            .flatten()
            .ok_or_else(|| "scalar value unavailable from input lengths".to_owned()),
        3 => {
            let constant = (payload >> 8) & 0x0000_ffff;
            let input_index = payload & 0x0000_00ff;
            let input_len = *input_lengths
                .get(input_index as usize)
                .ok_or_else(|| "input length index out of range".to_owned())?;
            Ok(constant + input_len)
        }
        _ => Err("unsupported reference kind".to_owned()),
    }
}

pub(crate) fn compare_refs(
    left: u32,
    right: u32,
    inputs: &[&[u8]],
    scalars: &[u32; 256],
) -> Result<bool, String> {
    let comparison = left >> 28;
    let left_value = resolve_ref(left & 0x0fff_ffff, inputs, scalars)?;
    let right_value = resolve_ref(right & 0x0fff_ffff, inputs, scalars)?;
    match comparison {
        1 => Ok(left_value > right_value),
        2 => Ok(left_value == right_value),
        3 => Ok(left_value != right_value),
        _ => Err("unsupported comparison".to_owned()),
    }
}

pub(crate) fn compare_refs_len(
    left: u32,
    right: u32,
    input_lengths: &[u32],
    scalars: &[Option<u32>; 256],
) -> Result<bool, String> {
    let comparison = left >> 28;
    let left_value = resolve_ref_len(left & 0x0fff_ffff, input_lengths, scalars)?;
    let right_value = resolve_ref_len(right & 0x0fff_ffff, input_lengths, scalars)?;
    match comparison {
        1 => Ok(left_value > right_value),
        2 => Ok(left_value == right_value),
        3 => Ok(left_value != right_value),
        _ => Err("unsupported comparison".to_owned()),
    }
}

pub(crate) fn parse_hex(input: &str) -> Option<Vec<u8>> {
    if input.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(input.len() / 2);
    let bytes = input.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        let high = hex_value(bytes[index])?;
        let low = hex_value(bytes[index + 1])?;
        out.push((high << 4) | low);
        index += 2;
    }
    Some(out)
}

pub(crate) fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[derive(Default)]
pub(crate) struct WasmSurface {
    pub(crate) valid: bool,
    pub(crate) import_count: usize,
    pub(crate) imports_memory: bool,
    pub(crate) exports: Vec<ExportSurface>,
}

pub(crate) struct ExportSurface {
    pub(crate) name: String,
    pub(crate) kind: ExternalKind,
    pub(crate) ty: Option<FuncType>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ExternalKind {
    Func,
    Memory,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FuncType {
    pub(crate) params: Vec<ValType>,
    pub(crate) results: Vec<ValType>,
}

impl FuncType {
    fn new<P, R>(params: P, results: R) -> Self
    where
        P: IntoIterator<Item = ValType>,
        R: IntoIterator<Item = ValType>,
    {
        Self {
            params: params.into_iter().collect(),
            results: results.into_iter().collect(),
        }
    }

    fn params(&self) -> &[ValType] {
        &self.params
    }

    fn results(&self) -> &[ValType] {
        &self.results
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ValType {
    I32,
    I64,
    F32,
    F64,
}

impl WasmSurface {
    fn parse(bytes: &[u8]) -> Option<Self> {
        let mut cursor = WasmCursor::new(bytes);
        if cursor.read_bytes(4)? != b"\0asm" || cursor.read_bytes(4)? != b"\x01\0\0\0" {
            return None;
        }
        let mut types = Vec::new();
        let mut funcs = Vec::new();
        let mut exports = Vec::new();
        let mut import_count = 0usize;
        let mut imports_memory = false;

        while !cursor.is_empty() {
            let section_id = cursor.read_u8()?;
            let section_len = cursor.read_leb_u32()? as usize;
            let mut section = WasmCursor::new(cursor.read_bytes(section_len)?);
            match section_id {
                1 => {
                    let count = section.read_leb_u32()?;
                    for _ in 0..count {
                        types.push(section.read_func_type()?);
                    }
                }
                2 => {
                    let count = section.read_leb_u32()?;
                    for _ in 0..count {
                        section.read_name()?;
                        section.read_name()?;
                        let kind = section.read_u8()?;
                        import_count += 1;
                        match kind {
                            0x00 => {
                                let index = section.read_leb_u32()?;
                                funcs.push(types.get(index as usize)?.clone());
                            }
                            0x02 => {
                                section.skip_limits()?;
                                imports_memory = true;
                            }
                            0x01 => section.skip_table_type()?,
                            0x03 => section.skip_global_type()?,
                            0x04 => {
                                section.read_leb_u32()?;
                            }
                            _ => return None,
                        }
                    }
                }
                3 => {
                    let count = section.read_leb_u32()?;
                    for _ in 0..count {
                        let ty_index = section.read_leb_u32()?;
                        funcs.push(types.get(ty_index as usize)?.clone());
                    }
                }
                7 => {
                    let count = section.read_leb_u32()?;
                    for _ in 0..count {
                        let name = section.read_name()?.to_owned();
                        let kind = match section.read_u8()? {
                            0x00 => ExternalKind::Func,
                            0x02 => ExternalKind::Memory,
                            _ => ExternalKind::Other,
                        };
                        let index = section.read_leb_u32()? as usize;
                        let ty = if kind == ExternalKind::Func {
                            funcs.get(index).cloned()
                        } else {
                            None
                        };
                        exports.push(ExportSurface { name, kind, ty });
                    }
                }
                _ => {}
            }
        }

        Some(Self {
            valid: true,
            import_count,
            imports_memory,
            exports,
        })
    }
}

pub(crate) struct WasmCursor<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) offset: usize,
}

impl<'a> WasmCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn is_empty(&self) -> bool {
        self.offset == self.bytes.len()
    }

    fn read_u8(&mut self) -> Option<u8> {
        let byte = *self.bytes.get(self.offset)?;
        self.offset += 1;
        Some(byte)
    }

    fn read_bytes(&mut self, len: usize) -> Option<&'a [u8]> {
        let end = self.offset.checked_add(len)?;
        let bytes = self.bytes.get(self.offset..end)?;
        self.offset = end;
        Some(bytes)
    }

    fn read_leb_u32(&mut self) -> Option<u32> {
        let mut result = 0u32;
        let mut shift = 0;
        loop {
            let byte = self.read_u8()?;
            result |= ((byte & 0x7f) as u32).checked_shl(shift)?;
            if byte & 0x80 == 0 {
                return Some(result);
            }
            shift += 7;
            if shift >= 35 {
                return None;
            }
        }
    }

    fn read_name(&mut self) -> Option<&'a str> {
        let len = self.read_leb_u32()? as usize;
        core::str::from_utf8(self.read_bytes(len)?).ok()
    }

    fn read_func_type(&mut self) -> Option<FuncType> {
        if self.read_u8()? != 0x60 {
            return None;
        }
        Some(FuncType {
            params: self.read_valtypes()?,
            results: self.read_valtypes()?,
        })
    }

    fn read_valtypes(&mut self) -> Option<Vec<ValType>> {
        let count = self.read_leb_u32()?;
        let mut values = Vec::new();
        for _ in 0..count {
            values.push(match self.read_u8()? {
                0x7f => ValType::I32,
                0x7e => ValType::I64,
                0x7d => ValType::F32,
                0x7c => ValType::F64,
                _ => return None,
            });
        }
        Some(values)
    }

    fn skip_limits(&mut self) -> Option<()> {
        let flags = self.read_u8()?;
        self.read_leb_u32()?;
        if flags & 0x01 != 0 {
            self.read_leb_u32()?;
        }
        Some(())
    }

    fn skip_table_type(&mut self) -> Option<()> {
        self.read_u8()?;
        self.skip_limits()
    }

    fn skip_global_type(&mut self) -> Option<()> {
        self.read_u8()?;
        self.read_u8()?;
        Some(())
    }
}

pub(crate) fn verify_surface(manifest: &UnitManifest, surface: &WasmSurface) -> bool {
    let mut ok = true;
    for import in manifest.imports {
        if let Some(module) = import.module {
            let _ = module;
            ok = false;
        }
    }
    for export in manifest.exports {
        ok &= verify_export(export, surface);
    }
    ok
}

pub(crate) fn verify_export(expected: &ApiFunction, surface: &WasmSurface) -> bool {
    let Some(actual) = surface
        .exports
        .iter()
        .find(|export| export.name == expected.name)
    else {
        return false;
    };

    if expected.ty == "memory" {
        return actual.kind == ExternalKind::Memory;
    }
    if actual.kind != ExternalKind::Func {
        return false;
    }

    let Some(actual_ty) = &actual.ty else {
        return false;
    };
    let Some((params, results)) = parse_api_type(expected.ty) else {
        return false;
    };
    actual_ty.params() == params.as_slice() && actual_ty.results() == results.as_slice()
}

pub(crate) fn parse_api_type(ty: &str) -> Option<(Vec<ValType>, Vec<ValType>)> {
    let (params, results) = ty.split_once("->")?;
    Some((parse_val_types(params), parse_val_types(results)))
}

pub(crate) fn parse_val_types(text: &str) -> Vec<ValType> {
    text.split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter_map(|token| match token {
            "i32" => Some(ValType::I32),
            "i64" => Some(ValType::I64),
            "f32" => Some(ValType::F32),
            "f64" => Some(ValType::F64),
            _ => None,
        })
        .collect()
}

pub(crate) fn valtypes_to_api_bytes(types: &[ValType]) -> Vec<u8> {
    types
        .iter()
        .filter_map(|ty| match ty {
            ValType::I32 => Some(0x7f),
            ValType::I64 => Some(0x7e),
            ValType::F32 => Some(0x7d),
            ValType::F64 => Some(0x7c),
        })
        .collect()
}

pub(crate) fn print_api(items: &[ApiFunction]) {
    for item in items {
        match (item.module, item.unit) {
            (Some(module), Some(unit)) => {
                println!("    {}.{}: {} [{}]", module, item.name, item.ty, unit);
            }
            (Some(module), None) => {
                println!("    {}.{}: {}", module, item.name, item.ty);
            }
            (None, _) => {
                println!("    {}: {}", item.name, item.ty);
            }
        }
    }
}

pub(crate) fn print_check(name: &str, ok: bool) {
    println!("  {}: {}", name, if ok { "ok" } else { "failed" });
}

mod tests {
    use super::*;

    const HMAC_COMPOSE: &[u8] = include_bytes!("../compositions/hmac-sha256-rfc2104/compose.edm");

    fn hmac_manifest() -> &'static CompositionManifest {
        composition("hmac-sha256-rfc2104-composed").expect("composition")
    }

    fn temp_test_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let dir = env::temp_dir().join(format!("edgerun-sdk-{name}-{nanos}"));
        fs::create_dir_all(&dir).expect("temp dir");
        dir
    }

    fn rust_unit_source_fixture(name: &str, code: &str) -> RustUnitSource {
        let dir = temp_test_dir(name);
        let rust_dir = dir.join("rust");
        let source_dir = rust_dir.join("src");
        fs::create_dir_all(&source_dir).expect("source dir");
        let rust_source = source_dir.join("lib.rs");
        fs::write(&rust_source, code).expect("source");
        RustUnitSource {
            id: name.to_owned(),
            standard: "TEST".to_owned(),
            standard_id: 1,
            rust_manifest: rust_dir.join("Cargo.toml"),
            rust_source,
            wasm_path: dir.join("unit.wasm"),
            manifest_path: dir.join("manifest.edm"),
        }
    }

    fn valid_test_surface() -> WasmSurface {
        WasmSurface {
            valid: true,
            import_count: 0,
            imports_memory: false,
            exports: vec![
                ExportSurface {
                    name: "memory".to_owned(),
                    kind: ExternalKind::Memory,
                    ty: None,
                },
                ExportSurface {
                    name: "proto_abi_version".to_owned(),
                    kind: ExternalKind::Func,
                    ty: Some(FuncType::new([], [ValType::I32])),
                },
                ExportSurface {
                    name: "proto_standard_id".to_owned(),
                    kind: ExternalKind::Func,
                    ty: Some(FuncType::new([], [ValType::I32])),
                },
                ExportSurface {
                    name: "sha256_digest".to_owned(),
                    kind: ExternalKind::Func,
                    ty: Some(FuncType::new(
                        [ValType::I32, ValType::I32, ValType::I32],
                        [ValType::I32],
                    )),
                },
            ],
        }
    }

    fn find_subslice(haystack: &[u8], needle: &[u8]) -> usize {
        haystack
            .windows(needle.len())
            .position(|window| window == needle)
            .expect("subslice")
    }

    fn step_offset(bytes: &[u8]) -> usize {
        let id_len = u16::from_le_bytes([bytes[16], bytes[17]]) as usize;
        let output_len_offset = 18 + id_len;
        let output_len =
            u16::from_le_bytes([bytes[output_len_offset], bytes[output_len_offset + 1]]) as usize;
        let mut offset = output_len_offset + 2 + output_len;
        let component_count = u16::from_le_bytes([bytes[12], bytes[13]]) as usize;
        for _ in 0..component_count {
            let unit_id_len = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize;
            offset += 34 + unit_id_len;
        }
        offset
    }

    struct NativeTestUnit {
        runtime: ComponentRuntime,
    }

    impl NativeTestUnit {
        fn load(id: &str) -> Self {
            let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            Self {
                runtime: load_component_runtime(&root, id).expect("native unit"),
            }
        }

        fn write(&mut self, offset: usize, bytes: &[u8]) {
            self.runtime
                .write_memory(offset, bytes)
                .expect("write memory");
        }

        fn read<const N: usize>(&self, offset: usize) -> [u8; N] {
            let mut bytes = [0u8; N];
            self.runtime
                .read_memory(offset, &mut bytes)
                .expect("read memory");
            bytes
        }

        fn call(&mut self, name: &str, args: [u32; 5]) -> i32 {
            let function = self
                .runtime
                .functions
                .iter()
                .find(|function| function.name == name)
                .cloned()
                .expect("function");
            assert!(function.param_count <= args.len());
            let native_args = self
                .runtime
                .native_call_args(&function, args)
                .expect("native args");
            let status = unsafe { function.func.call(&native_args) };
            self.runtime
                .normalize_native_outputs(&function, args)
                .expect("normalize native outputs");
            status
        }
    }

    fn instantiate_test_unit(id: &str) -> NativeTestUnit {
        NativeTestUnit::load(id)
    }

    #[test]
    fn composition_verifier_accepts_source_bytes() {
        assert!(verify_binary_composition(hmac_manifest(), HMAC_COMPOSE));
    }

    #[test]
    fn native_hmac_verify_composition_accepts_and_rejects_tags() {
        let manifest = composition("hmac-sha256-verify-rfc2104").expect("composition");
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let bytes = fs::read(root.join(manifest.path)).expect("composition bytes");
        let parsed = parse_composition(&bytes).expect("composition");
        let valid_tag =
            hex_to_32("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843")
                .expect("tag");

        let valid = execute_composition(
            manifest,
            &parsed,
            &[b"Jefe", b"what do ya want for nothing?", &valid_tag],
        )
        .expect("valid hmac verify");
        assert_eq!(valid.output, [0]);

        let invalid_tag = [0u8; 32];
        let invalid = execute_composition(
            manifest,
            &parsed,
            &[b"Jefe", b"what do ya want for nothing?", &invalid_tag],
        )
        .expect("invalid hmac verify");
        assert_eq!(invalid.output, [1]);
    }

    #[test]
    fn native_abi_derives_pointer_args_and_pointer_outputs() {
        let source = r#"
#[edgerun_unit::export]
unsafe fn tftp_parse(message_ptr: i32, message_len: i32, out_ptr: i32) -> i32 {
    let out = out_ptr as *mut u32;
    out.add(0).write_unaligned(3);
    out.add(1).write_unaligned((message_ptr + 2) as u32);
    out.add(2).write_unaligned((message_len - 2) as u32);
    0
}
"#;
        let abi = parse_native_abi_source(source).expect("native abi");
        assert_eq!(abi.len(), 1);
        assert_eq!(abi[0].name, "tftp_parse");
        assert_eq!(abi[0].pointer_args, [true, false, true, false, false]);
        assert_eq!(abi[0].output_arg, Some(2));
        assert_eq!(abi[0].output_pointer_fields, vec![1]);
    }

    #[test]
    fn artifact_builders_reproduce_source_bytes() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for source in discover_rust_unit_sources(&root).expect("unit sources") {
            let wasm = fs::read(&source.wasm_path).expect("unit wasm");
            let wasm_sha256 = sha256(&wasm);
            let surface = WasmSurface::parse(&wasm).expect("wasm surface");
            let actual_manifest = fs::read(&source.manifest_path).expect("unit manifest");
            let expected_manifest = runtime_unit_manifest_bytes(&source, &surface, &wasm_sha256);
            assert_eq!(actual_manifest, expected_manifest, "{}", source.id);

            let actual_api =
                fs::read(source.wasm_path.with_file_name("api.edm")).expect("unit api");
            let expected_api =
                runtime_api_manifest_bytes(&source.id, &surface).expect("api manifest");
            assert_eq!(actual_api, expected_api, "{}", source.id);
        }
        for manifest in compositions() {
            let actual = fs::read(root.join(manifest.path)).expect("composition bytes");
            let expected = composition_manifest_bytes(manifest).expect("built composition");
            assert_eq!(actual, expected, "{}", manifest.id);
        }
        for manifest in segments() {
            let actual = fs::read(root.join(manifest.path)).expect("segment bytes");
            let expected = segment_manifest_bytes(manifest).expect("built segment");
            assert_eq!(actual, expected, "{}", manifest.id);
        }
        for manifest in chains() {
            let actual = fs::read(root.join(manifest.path)).expect("chain bytes");
            let expected = chain_manifest_bytes(manifest).expect("built chain");
            assert_eq!(actual, expected, "{}", manifest.id);
        }
    }

    #[test]
    fn unit_manifest_verifier_accepts_generated_manifest() {
        let manifest = unit("sha256-fips180").expect("unit");
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let bytes = fs::read(root.join(manifest.manifest_path)).expect("manifest bytes");

        assert!(verify_binary_manifest(manifest, &bytes));
    }

    #[test]
    fn unit_manifest_verifier_rejects_standard_id_change() {
        let manifest = unit("sha256-fips180").expect("unit");
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut bytes = fs::read(root.join(manifest.manifest_path)).expect("manifest bytes");
        bytes[20..24].copy_from_slice(&(manifest.standard_id + 1).to_le_bytes());

        assert!(!verify_binary_manifest(manifest, &bytes));
    }

    #[test]
    fn composition_verifier_rejects_component_hash_change() {
        let mut bytes = HMAC_COMPOSE.to_vec();
        let hash = hex_to_32(hmac_manifest().components[0].wasm_sha256).expect("hash");
        let offset = find_subslice(&bytes, &hash);
        bytes[offset] ^= 0x01;
        assert!(!verify_binary_composition(hmac_manifest(), &bytes));
    }

    #[test]
    fn composition_verifier_rejects_bad_branch_target() {
        let mut composition = hmac_wire_composition();
        composition
            .steps
            .iter_mut()
            .find(|step| step.opcode == 5)
            .expect("branch step")
            .arg2 = 999;
        let bytes = sdk_wire_record_bytes(SdkWireRecord::Composition(composition));
        assert!(!verify_binary_composition(hmac_manifest(), &bytes));
    }

    fn hmac_wire_composition() -> edgerun_wire::CompositionRecord {
        let bytes = HMAC_COMPOSE.to_vec();
        match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&bytes)
            .expect("hmac composition")
        {
            SdkWireRecord::Composition(composition) => composition,
            _ => panic!("not a composition"),
        }
    }

    #[test]
    fn report_verifier_accepts_matching_report() {
        let output_hash = *b"86ea816be859ea16764f6371c1b0e0b5577efb5e6e72b20ed5f683c503f8e80f";
        let bytes =
            execution_report_bytes(hmac_manifest(), &[4, 28], 509, 32, &output_hash).unwrap();
        let report = parse_report(&bytes).expect("report");
        assert!(verify_binary_report(hmac_manifest(), &report));
    }

    #[test]
    fn report_verifier_rejects_cost_change() {
        let output_hash = *b"86ea816be859ea16764f6371c1b0e0b5577efb5e6e72b20ed5f683c503f8e80f";
        let mut bytes =
            execution_report_bytes(hmac_manifest(), &[4, 28], 509, 32, &output_hash).unwrap();
        bytes[22..30].copy_from_slice(&510u64.to_le_bytes());
        let report = parse_report(&bytes).expect("report");
        assert!(!verify_binary_report(hmac_manifest(), &report));
    }

    #[test]
    fn report_verifier_rejects_input_length_change() {
        let output_hash = *b"86ea816be859ea16764f6371c1b0e0b5577efb5e6e72b20ed5f683c503f8e80f";
        let mut bytes =
            execution_report_bytes(hmac_manifest(), &[4, 28], 509, 32, &output_hash).unwrap();
        let input_lengths_offset =
            98 + hmac_manifest().id.len() + hmac_manifest().output_unit.len();
        bytes[input_lengths_offset..input_lengths_offset + 4].copy_from_slice(&5u32.to_le_bytes());
        let report = parse_report(&bytes).expect("report");
        assert!(!verify_binary_report(hmac_manifest(), &report));
    }

    #[test]
    fn report_verifier_rejects_component_hash_change() {
        let output_hash = *b"86ea816be859ea16764f6371c1b0e0b5577efb5e6e72b20ed5f683c503f8e80f";
        let mut bytes =
            execution_report_bytes(hmac_manifest(), &[4, 28], 509, 32, &output_hash).unwrap();
        let hash = hex_to_32(hmac_manifest().components[0].wasm_sha256).expect("hash");
        let offset = find_subslice(&bytes, &hash);
        bytes[offset] ^= 0x01;
        let report = parse_report(&bytes).expect("report");
        assert!(!verify_binary_report(hmac_manifest(), &report));
    }

    #[test]
    fn segment_report_replay_rejects_forged_output_hash() {
        let manifest = segment("hmac-sha256-verify-private-node-v1").expect("segment");
        let inputs = vec![
            b"Jefe".to_vec(),
            b"what do ya want for nothing?".to_vec(),
            hex_to_32("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843")
                .unwrap()
                .to_vec(),
        ];
        let input_lengths: Vec<u32> = inputs.iter().map(|input| input.len() as u32).collect();
        let input_hashes: Vec<[u8; 32]> = inputs.iter().map(|input| sha256(input)).collect();
        let forged_output_hash =
            *b"0000000000000000000000000000000000000000000000000000000000000000";
        let bytes = segment_report_bytes(
            manifest,
            &input_lengths,
            &input_hashes,
            0,
            199,
            1,
            u16::MAX,
            &forged_output_hash,
        )
        .unwrap();
        let report = parse_segment_report(&bytes).expect("report");
        assert!(verify_binary_segment_report(manifest, &report));
        assert!(!replay_binary_segment_report(manifest, &report, &inputs));
    }

    #[test]
    fn segment_report_signature_binds_exact_report_bytes() {
        let manifest = segment("hmac-sha256-verify-private-node-v1").expect("segment");
        let inputs = vec![
            b"Jefe".to_vec(),
            b"what do ya want for nothing?".to_vec(),
            hex_to_32("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843")
                .unwrap()
                .to_vec(),
        ];
        let input_lengths: Vec<u32> = inputs.iter().map(|input| input.len() as u32).collect();
        let input_hashes: Vec<[u8; 32]> = inputs.iter().map(|input| sha256(input)).collect();
        let output_hash = *b"6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d";
        let report_bytes = segment_report_bytes(
            manifest,
            &input_lengths,
            &input_hashes,
            0,
            199,
            1,
            u16::MAX,
            &output_hash,
        )
        .unwrap();
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let esig_bytes = signature_bytes(&report_bytes, &signing_key);
        let esig = parse_artifact_signature_record(&esig_bytes).expect("signature");
        assert!(verify_binary_signature(&report_bytes, &esig));

        let mut tampered = report_bytes.clone();
        tampered[24] ^= 0x01;
        assert!(!verify_binary_signature(&tampered, &esig));
    }

    #[test]
    fn signer_policy_binds_key_to_segment_role_and_capability() {
        let manifest = segment("hmac-sha256-verify-private-node-v1").expect("segment");
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let allowed_key = signing_key.verifying_key().as_bytes().to_vec();
        let policy_bytes = signer_policy_bytes(manifest, &[allowed_key]).expect("policy");
        let policy = parse_signer_policy_record(&policy_bytes).expect("policy");
        let output_hash = *b"6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d";

        let report_bytes = segment_report_bytes(
            manifest,
            &[4, 28, 32],
            &[
                sha256(b"Jefe"),
                sha256(b"what do ya want for nothing?"),
                sha256(
                    &hex_to_32("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843")
                        .unwrap(),
                ),
            ],
            0,
            199,
            1,
            u16::MAX,
            &output_hash,
        )
        .unwrap();
        let esig_bytes = signature_bytes(&report_bytes, &signing_key);
        let signature = parse_artifact_signature_record(&esig_bytes).expect("signature");
        assert!(verify_binary_signer_policy(manifest, &signature, &policy));

        let other_key = SigningKey::from_bytes(&[8u8; 32]);
        let other_signature_bytes = signature_bytes(&report_bytes, &other_key);
        let other_signature =
            parse_artifact_signature_record(&other_signature_bytes).expect("signature");
        assert!(!verify_binary_signer_policy(
            manifest,
            &other_signature,
            &policy
        ));
    }

    #[test]
    fn revocation_requires_trusted_issuer_role() {
        let issuer_key = SigningKey::from_bytes(&[9u8; 32]);
        let issuer = *issuer_key.verifying_key().as_bytes();
        let app_id = sha256(b"test app");
        let target = sha256(b"payment artifact");
        let policy_bytes = trust_policy_bytes(&[OwnedTrustEntry {
            role: TRUST_ROLE_PAYMENT_STORE,
            valid_from: 0,
            valid_until: u64::MAX,
            public_key: issuer,
            app_id,
            developer_id: [0u8; 32],
        }]);
        let policy =
            match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&policy_bytes)
                .expect("policy")
            {
                SdkWireRecord::TrustPolicy(policy) => policy,
                _ => panic!("not a trust policy"),
            };
        let revocations = RevocationSet {
            payment: vec![RevokedTarget {
                target,
                issuer,
                issued_at: 42,
            }],
            ..Default::default()
        };

        assert!(revocations.revoke_payment(&target, &policy, &app_id, &[]));
    }

    #[test]
    fn revocation_ignores_untrusted_issuer() {
        let issuer_key = SigningKey::from_bytes(&[9u8; 32]);
        let trusted_key = SigningKey::from_bytes(&[10u8; 32]);
        let issuer = *issuer_key.verifying_key().as_bytes();
        let trusted = *trusted_key.verifying_key().as_bytes();
        let app_id = sha256(b"test app");
        let target = sha256(b"payment artifact");
        let policy_bytes = trust_policy_bytes(&[OwnedTrustEntry {
            role: TRUST_ROLE_PAYMENT_STORE,
            valid_from: 0,
            valid_until: u64::MAX,
            public_key: trusted,
            app_id,
            developer_id: [0u8; 32],
        }]);
        let policy =
            match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&policy_bytes)
                .expect("policy")
            {
                SdkWireRecord::TrustPolicy(policy) => policy,
                _ => panic!("not a trust policy"),
            };
        let revocations = RevocationSet {
            payment: vec![RevokedTarget {
                target,
                issuer,
                issued_at: 42,
            }],
            ..Default::default()
        };

        assert!(!revocations.revoke_payment(&target, &policy, &app_id, &[]));
    }

    #[test]
    fn revocation_ignores_wrong_trust_role() {
        let issuer_key = SigningKey::from_bytes(&[9u8; 32]);
        let issuer = *issuer_key.verifying_key().as_bytes();
        let app_id = sha256(b"test app");
        let target = sha256(b"payment artifact");
        let policy_bytes = trust_policy_bytes(&[OwnedTrustEntry {
            role: TRUST_ROLE_PRODUCT_STORE,
            valid_from: 0,
            valid_until: u64::MAX,
            public_key: issuer,
            app_id,
            developer_id: [0u8; 32],
        }]);
        let policy =
            match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&policy_bytes)
                .expect("policy")
            {
                SdkWireRecord::TrustPolicy(policy) => policy,
                _ => panic!("not a trust policy"),
            };
        let revocations = RevocationSet {
            payment: vec![RevokedTarget {
                target,
                issuer,
                issued_at: 42,
            }],
            ..Default::default()
        };

        assert!(!revocations.revoke_payment(&target, &policy, &app_id, &[]));
    }

    #[test]
    fn revocation_issuer_trust_is_time_scoped() {
        let issuer_key = SigningKey::from_bytes(&[9u8; 32]);
        let issuer = *issuer_key.verifying_key().as_bytes();
        let app_id = sha256(b"test app");
        let early_target = sha256(b"early payment artifact");
        let active_target = sha256(b"active payment artifact");
        let policy_bytes = trust_policy_bytes(&[OwnedTrustEntry {
            role: TRUST_ROLE_PAYMENT_STORE,
            valid_from: 20,
            valid_until: 30,
            public_key: issuer,
            app_id,
            developer_id: [0u8; 32],
        }]);
        let policy =
            match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&policy_bytes)
                .expect("policy")
            {
                SdkWireRecord::TrustPolicy(policy) => policy,
                _ => panic!("not a trust policy"),
            };
        let revocations = RevocationSet {
            payment: vec![
                RevokedTarget {
                    target: early_target,
                    issuer,
                    issued_at: 19,
                },
                RevokedTarget {
                    target: active_target,
                    issuer,
                    issued_at: 25,
                },
            ],
            ..Default::default()
        };

        assert!(!revocations.revoke_payment(&early_target, &policy, &app_id, &[]));
        assert!(revocations.revoke_payment(&active_target, &policy, &app_id, &[]));
    }

    #[test]
    fn signing_request_response_roundtrip_verifies() {
        let app_id = sha256(b"app");
        let release_id = sha256(b"release");
        let subject = sha256(b"subject");
        let payload = sha256(b"payload");
        let request = sign_request_bytes(SignRequestInput {
            domain: b"edgerun-test-sign",
            app_id: &app_id,
            release_id: &release_id,
            subject_sha256: &subject,
            payload_sha256: &payload,
            algorithm: SIGN_ALGORITHM_ED25519,
            assurance: 2,
            nonce: b"nonce",
        });
        assert!(matches!(
            edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&request)
                .expect("wire request"),
            SdkWireRecord::CapabilityRequest(_)
        ));
        let parsed_request = parse_capability_request_record(&request).expect("request");
        assert_eq!(
            signing_request_algorithm(&parsed_request),
            Some(SIGN_ALGORITHM_ED25519)
        );
        assert_eq!(parsed_request.context, b"edgerun-test-sign");
        assert_eq!(parsed_request.app_id, app_id);
        assert_eq!(parsed_request.release_id, release_id);
        assert_eq!(parsed_request.subject_sha256, subject);
        assert_eq!(parsed_request.payload_sha256, payload);
        assert_eq!(parsed_request.nonce, b"nonce");

        let key = SigningKey::from_bytes(&[11u8; 32]);
        let response = sign_response_bytes(SignResponseInput {
            request_bytes: &request,
            provider: b"software",
            algorithm: SIGN_ALGORITHM_ED25519,
            assurance: 3,
            signer: key.verifying_key().as_bytes(),
            signing_key: &key,
        });
        assert!(matches!(
            edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&response)
                .expect("wire response"),
            SdkWireRecord::CapabilityResponse(_)
        ));
        let parsed_response = parse_capability_response_record(&response).expect("response");
        assert_eq!(
            signing_response_algorithm(&parsed_response),
            Some(SIGN_ALGORITHM_ED25519)
        );
        assert_eq!(parsed_response.provider, b"software");
        assert_eq!(parsed_response.assurance, 3);
        assert!(parsed_response.assurance >= parsed_request.assurance);
        assert!(verify_sign_response_signature(&request, &parsed_response));
    }

    #[test]
    fn signing_response_rejects_request_tamper() {
        let app_id = sha256(b"app");
        let release_id = sha256(b"release");
        let subject = sha256(b"subject");
        let payload = sha256(b"payload");
        let request = sign_request_bytes(SignRequestInput {
            domain: b"edgerun-test-sign",
            app_id: &app_id,
            release_id: &release_id,
            subject_sha256: &subject,
            payload_sha256: &payload,
            algorithm: SIGN_ALGORITHM_ED25519,
            assurance: 2,
            nonce: b"nonce",
        });
        let key = SigningKey::from_bytes(&[11u8; 32]);
        let response = sign_response_bytes(SignResponseInput {
            request_bytes: &request,
            provider: b"software",
            algorithm: SIGN_ALGORITHM_ED25519,
            assurance: 2,
            signer: key.verifying_key().as_bytes(),
            signing_key: &key,
        });
        let parsed_response = parse_capability_response_record(&response).expect("response");
        let mut tampered_request = request.clone();
        tampered_request[20] ^= 0x01;

        assert!(!verify_sign_response_signature(
            &tampered_request,
            &parsed_response
        ));
    }

    #[test]
    fn capability_response_binding_checks_generic_envelope() {
        let app_id = sha256(b"app");
        let release_id = sha256(b"release");
        let subject = sha256(b"subject");
        let payload_hash = sha256(b"payload");
        let request = capability_request_bytes(CapabilityRequestInput {
            kind: 4,
            operation: 6,
            assurance: 2,
            app_id: &app_id,
            release_id: &release_id,
            subject_sha256: &subject,
            payload_sha256: &payload_hash,
            context: b"path",
            payload: b"",
            nonce: b"nonce",
        });
        let parsed_request = parse_capability_request_record(&request).expect("request");
        assert_eq!(parsed_request.capability_kind, 4);
        assert_eq!(parsed_request.operation, 6);
        assert_eq!(parsed_request.context, b"path");

        let response = capability_response_bytes(CapabilityResponseInput {
            request_bytes: &request,
            kind: 4,
            operation: 6,
            status: 0,
            assurance: 3,
            provider: b"local-storage",
            responder: b"runtime",
            payload: b"ok",
            proof: b"proof",
        });
        let parsed_response = parse_capability_response_record(&response).expect("response");
        assert!(capability_response_binding_ok(
            &request,
            &parsed_request,
            &parsed_response
        ));

        let wrong_response = capability_response_bytes(CapabilityResponseInput {
            request_bytes: &request,
            kind: 4,
            operation: 7,
            status: 0,
            assurance: 3,
            provider: b"local-storage",
            responder: b"runtime",
            payload: b"ok",
            proof: b"proof",
        });
        let parsed_wrong = parse_capability_response_record(&wrong_response).expect("response");
        assert!(!capability_response_binding_ok(
            &request,
            &parsed_request,
            &parsed_wrong
        ));
    }

    #[test]
    fn capability_denial_response_binds_request_and_reason() {
        let app_id = sha256(b"app");
        let release_id = sha256(b"release");
        let subject = sha256(b"subject");
        let payload_hash = sha256(b"payload");
        let request = capability_request_bytes(CapabilityRequestInput {
            kind: CAPABILITY_KIND_STORAGE,
            operation: CAPABILITY_OPERATION_READ,
            assurance: 2,
            app_id: &app_id,
            release_id: &release_id,
            subject_sha256: &subject,
            payload_sha256: &payload_hash,
            context: b"path",
            payload: b"",
            nonce: b"nonce",
        });
        let parsed_request = parse_capability_request_record(&request).expect("request");
        let response = capability_denial_response_bytes(
            &request,
            &parsed_request,
            b"profile",
            CAPABILITY_STATUS_POLICY_DENIED,
            b"policy_denied",
        );
        let parsed_response = parse_capability_response_record(&response).expect("response");

        assert_eq!(parsed_response.status, CAPABILITY_STATUS_POLICY_DENIED);
        assert_eq!(parsed_response.payload, b"policy_denied");
        assert!(capability_response_binding_ok(
            &request,
            &parsed_request,
            &parsed_response
        ));
        assert_eq!(
            parsed_response.proof,
            capability_denial_proof(
                &request,
                parsed_response.status,
                &parsed_response.provider,
                &parsed_response.payload
            )
        );
    }

    #[test]
    fn current_capability_artifacts_have_rkyv_wire_records() {
        let app_id = sha256(b"app");
        let release_id = sha256(b"release");
        let subject = sha256(b"subject");
        let payload_hash = sha256(b"payload");
        let request = capability_request_bytes(CapabilityRequestInput {
            kind: CAPABILITY_KIND_STORAGE,
            operation: CAPABILITY_OPERATION_READ,
            assurance: 2,
            app_id: &app_id,
            release_id: &release_id,
            subject_sha256: &subject,
            payload_sha256: &payload_hash,
            context: b"path",
            payload: b"",
            nonce: b"nonce",
        });
        let parsed_request = parse_capability_request_record(&request).expect("request");
        let wire_request = parsed_request.clone();
        assert_eq!(wire_request.capability_kind, CAPABILITY_KIND_STORAGE);
        let first = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(wire_request.clone()));
        let second = sdk_wire_bytes(&SdkWireRecord::CapabilityRequest(wire_request));
        assert_eq!(first, second);

        let response = capability_denial_response_bytes(
            &request,
            &parsed_request,
            b"profile",
            CAPABILITY_STATUS_POLICY_DENIED,
            b"policy_denied",
        );
        let parsed_response = parse_capability_response_record(&response).expect("response");
        let wire_response = parsed_response;
        assert_eq!(wire_response.status, CAPABILITY_STATUS_POLICY_DENIED);
        let first = sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(wire_response.clone()));
        let second = sdk_wire_bytes(&SdkWireRecord::CapabilityResponse(wire_response));
        assert_eq!(first, second);
    }

    #[test]
    fn current_user_profile_artifacts_have_rkyv_wire_records() {
        let owner = SigningKey::from_bytes(&[0x31; 32]);
        let seal_key = SealKey::from_bytes([0x51; 32]);
        let profile_id = user_profile_id(owner.verifying_key().as_bytes(), 7);
        let grant = OwnedUserGrant {
            app_id: sha256(b"app"),
            release_id: sha256(b"release"),
            scope_sha256: sha256(b"user/state"),
            capability_kind: CAPABILITY_KIND_SEALING,
            operation: CAPABILITY_OPERATION_UNSEAL,
            min_assurance: 2,
            flags: 0,
            valid_from: 10,
            valid_until: 20,
        };
        let body = user_profile_body_bytes(&profile_id, &owner, 7, 2, &[grant]);
        let profile_file = user_profile_file_bytes(&body, &seal_key).expect("profile");
        let wire_profile =
            match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&profile_file)
                .expect("profile")
            {
                SdkWireRecord::UserProfile(profile) => profile,
                _ => panic!("not a user profile"),
            };
        let opened = open_user_profile_file(&profile_file, &seal_key).expect("opened");

        let first = sdk_wire_bytes(&SdkWireRecord::UserProfile(wire_profile.clone()));
        let second = sdk_wire_bytes(&SdkWireRecord::UserProfile(wire_profile));
        assert_eq!(first, second);

        let wire_body = opened;
        assert_eq!(wire_body.grants.len(), 1);
        let first = sdk_wire_bytes(&SdkWireRecord::UserProfileBody(wire_body.clone()));
        let second = sdk_wire_bytes(&SdkWireRecord::UserProfileBody(wire_body));
        assert_eq!(first, second);
    }

    #[test]
    fn wire_user_profile_opens_and_authorizes_wire_request() {
        let owner = SigningKey::from_bytes(&[0x32; 32]);
        let seal_key = SealKey::from_bytes([0x52; 32]);
        let profile_id = user_profile_id(owner.verifying_key().as_bytes(), 8);
        let app_id = sha256(b"app");
        let release_id = sha256(b"release");
        let context = b"user/state";
        let grant = OwnedUserGrant {
            app_id,
            release_id,
            scope_sha256: sha256(context),
            capability_kind: CAPABILITY_KIND_STORAGE,
            operation: CAPABILITY_OPERATION_READ,
            min_assurance: 2,
            flags: 0,
            valid_from: 10,
            valid_until: 20,
        };
        let body = wire_user_profile_body_bytes(&profile_id, &owner, 8, 1, &[grant]);
        let parsed_body =
            match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&body)
                .expect("profile body")
            {
                SdkWireRecord::UserProfileBody(body) => body,
                _ => panic!("not a profile body"),
            };
        assert_eq!(parsed_body.profile_id, profile_id);
        let profile_file = wire_user_profile_file_bytes(&body, &seal_key).expect("profile");
        let parsed_file =
            match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&profile_file)
                .expect("profile file")
            {
                SdkWireRecord::UserProfile(profile) => profile,
                _ => panic!("not a user profile"),
            };
        assert_eq!(parsed_file.profile_id, profile_id);
        let opened = open_wire_user_profile_file(&profile_file, &seal_key).expect("opened");
        assert_eq!(opened.profile_id, profile_id);
        assert_eq!(opened.grants.len(), 1);
        assert!(verify_wire_user_profile_body_signature(&opened));

        let request = edgerun_wire::CapabilityRequest::new(
            CAPABILITY_KIND_STORAGE,
            CAPABILITY_OPERATION_READ,
            2,
            app_id,
            release_id,
            sha256(b"subject"),
            [0u8; 32],
            context.to_vec(),
            Vec::new(),
            b"nonce".to_vec(),
        );
        assert!(wire_user_profile_allows_request(&opened, &request, 15));
        assert!(!wire_user_profile_allows_request(&opened, &request, 21));
    }

    #[test]
    fn storage_request_executes_and_verifies_response() {
        let dir = temp_test_dir("storage");
        let request_path = dir.join("request.rkyv");
        let response_path = dir.join("response.rkyv");
        let root = dir.join("objects");
        let payload = b"stored bytes".to_vec();
        let request = edgerun_wire::CapabilityRequest::new(
            CAPABILITY_KIND_STORAGE,
            CAPABILITY_OPERATION_WRITE,
            2,
            sha256(b"app"),
            sha256(b"release"),
            sha256(b"subject"),
            sha256(&payload),
            b"user/state".to_vec(),
            payload.clone(),
            b"nonce".to_vec(),
        );
        let request_bytes =
            sdk_wire_record_bytes(SdkWireRecord::CapabilityRequest(request.clone()));
        fs::write(&request_path, &request_bytes).expect("request file");

        let args = vec![
            request_path.to_string_lossy().into_owned(),
            response_path.to_string_lossy().into_owned(),
            "local-storage".to_owned(),
            root.to_string_lossy().into_owned(),
        ];
        assert_eq!(
            execute_storage_request(&args, CAPABILITY_OPERATION_WRITE, None),
            0
        );

        let response =
            match read_sdk_wire_record(&response_path.to_string_lossy()).expect("response") {
                SdkWireRecord::CapabilityResponse(response) => response,
                _ => panic!("unexpected wire record"),
            };
        assert!(wire_capability_response_binding_ok(
            &request_bytes,
            &request,
            &response
        ));
        assert_eq!(
            response.proof,
            storage_response_proof(
                &request_bytes,
                CAPABILITY_OPERATION_WRITE,
                b"local-storage",
                &response.payload
            )
        );
        assert!(storage_write_receipt_matches(&response.payload, &payload));
        assert_eq!(
            cmd_verify_storage_response(vec![
                request_path.to_string_lossy().into_owned(),
                response_path.to_string_lossy().into_owned()
            ]),
            0
        );
    }

    #[test]
    fn sealing_capability_seals_and_unseals_payload() {
        let app_id = sha256(b"app");
        let release_id = sha256(b"release");
        let subject = sha256(b"state-key");
        let plaintext = b"private app state key";
        let plaintext_hash = sha256(plaintext);
        let key = SealKey::from_bytes([0x42; 32]);
        let request = capability_request_bytes(CapabilityRequestInput {
            kind: CAPABILITY_KIND_SEALING,
            operation: CAPABILITY_OPERATION_SEAL,
            assurance: 2,
            app_id: &app_id,
            release_id: &release_id,
            subject_sha256: &subject,
            payload_sha256: &plaintext_hash,
            context: b"passkey-policy",
            payload: plaintext,
            nonce: b"nonce",
        });
        let parsed_request = parse_capability_request_record(&request).expect("request");
        let sealed = seal_with_key(&parsed_request.payload, &key).expect("sealed");
        let proof = sealing_response_proof(&request, CAPABILITY_OPERATION_SEAL, b"local", &sealed);
        let response = capability_response_bytes(CapabilityResponseInput {
            request_bytes: &request,
            kind: CAPABILITY_KIND_SEALING,
            operation: CAPABILITY_OPERATION_SEAL,
            status: 0,
            assurance: 3,
            provider: b"local",
            responder: &sha256(key.expose_secret()),
            payload: &sealed,
            proof: &proof,
        });
        let parsed_response = parse_capability_response_record(&response).expect("response");

        assert!(capability_response_binding_ok(
            &request,
            &parsed_request,
            &parsed_response
        ));
        assert_eq!(
            parsed_response.proof,
            sealing_response_proof(
                &request,
                parsed_response.operation,
                &parsed_response.provider,
                &parsed_response.payload
            )
        );
        assert_eq!(
            unseal_with_key(&parsed_response.payload, &key).expect("plaintext"),
            plaintext
        );

        let unseal_payload_hash = sha256(&parsed_response.payload);
        let unseal_request = capability_request_bytes(CapabilityRequestInput {
            kind: CAPABILITY_KIND_SEALING,
            operation: CAPABILITY_OPERATION_UNSEAL,
            assurance: 2,
            app_id: &app_id,
            release_id: &release_id,
            subject_sha256: &subject,
            payload_sha256: &unseal_payload_hash,
            context: b"passkey-policy",
            payload: &parsed_response.payload,
            nonce: b"nonce-2",
        });
        let unsealed = unseal_with_key(&parsed_response.payload, &key).expect("unsealed");
        let unseal_proof = sealing_response_proof(
            &unseal_request,
            CAPABILITY_OPERATION_UNSEAL,
            b"local",
            &unsealed,
        );
        let unseal_response = capability_response_bytes(CapabilityResponseInput {
            request_bytes: &unseal_request,
            kind: CAPABILITY_KIND_SEALING,
            operation: CAPABILITY_OPERATION_UNSEAL,
            status: 0,
            assurance: 3,
            provider: b"local",
            responder: &sha256(key.expose_secret()),
            payload: &unsealed,
            proof: &unseal_proof,
        });
        let parsed_unseal_request =
            parse_capability_request_record(&unseal_request).expect("request");
        let parsed_unseal_response =
            parse_capability_response_record(&unseal_response).expect("response");

        assert!(capability_response_binding_ok(
            &unseal_request,
            &parsed_unseal_request,
            &parsed_unseal_response
        ));
        assert_eq!(parsed_unseal_response.payload, plaintext);
    }

    #[test]
    fn encrypted_user_profile_grants_capability_access() {
        let owner = SigningKey::from_bytes(&[0x31; 32]);
        let seal_key = SealKey::from_bytes([0x51; 32]);
        let profile_id = user_profile_id(owner.verifying_key().as_bytes(), 7);
        let app_id = sha256(b"app");
        let release_id = sha256(b"release");
        let context = b"user/state";
        let grant = OwnedUserGrant {
            app_id,
            release_id,
            scope_sha256: sha256(context),
            capability_kind: CAPABILITY_KIND_SEALING,
            operation: CAPABILITY_OPERATION_UNSEAL,
            min_assurance: 2,
            flags: 0,
            valid_from: 10,
            valid_until: 20,
        };
        let body = user_profile_body_bytes(&profile_id, &owner, 7, 1, &[grant]);
        let file = user_profile_file_bytes(&body, &seal_key).expect("profile");
        let parsed = open_user_profile_file(&file, &seal_key).expect("opened");
        assert!(verify_user_profile_body_signature(&parsed));

        let payload = b"sealed-state-key";
        let request = capability_request_bytes(CapabilityRequestInput {
            kind: CAPABILITY_KIND_SEALING,
            operation: CAPABILITY_OPERATION_UNSEAL,
            assurance: 2,
            app_id: &app_id,
            release_id: &release_id,
            subject_sha256: &sha256(b"subject"),
            payload_sha256: &sha256(payload),
            context,
            payload,
            nonce: b"nonce",
        });
        let parsed_request = parse_capability_request_record(&request).expect("request");
        assert!(user_profile_allows_request(&parsed, &parsed_request, 15));
        assert!(!user_profile_allows_request(&parsed, &parsed_request, 21));

        let mut weak_wire =
            match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&request)
                .expect("wire request")
            {
                SdkWireRecord::CapabilityRequest(request) => request,
                _ => panic!("not a capability request"),
            };
        weak_wire.assurance = 1;
        let weak_request = sdk_wire_record_bytes(SdkWireRecord::CapabilityRequest(weak_wire));
        let weak = parse_capability_request_record(&weak_request).expect("request");
        assert!(!user_profile_allows_request(&parsed, &weak, 15));
    }

    #[test]
    fn rust_unit_source_requires_metadata_macro() {
        let source = rust_unit_source_fixture(
            "missing-metadata",
            r#"
#![no_std]

#[edgerun_unit::export]
pub(crate) unsafe fn sha256_digest(_input_ptr: i32, _input_len: i32, _out_ptr: i32) -> i32 {
    0
}
"#,
        );

        let err = validate_rust_unit_source(&source).expect_err("source should fail");
        assert!(err.contains("metadata"));
    }

    #[test]
    fn rust_unit_source_requires_export_macro() {
        let source = rust_unit_source_fixture(
            "missing-export",
            r#"
#![no_std]

edgerun_unit::metadata!(1);

pub(crate) unsafe fn sha256_digest(_input_ptr: i32, _input_len: i32, _out_ptr: i32) -> i32 {
    0
}
"#,
        );

        let err = validate_rust_unit_source(&source).expect_err("source should fail");
        assert!(err.contains("edgerun_unit::export"));
    }

    #[test]
    fn rust_unit_source_rejects_manual_no_mangle() {
        let source = rust_unit_source_fixture(
            "manual-no-mangle",
            r#"
#![no_std]

edgerun_unit::metadata!(1);

#[no_mangle]
#[edgerun_unit::export]
pub(crate) unsafe fn sha256_digest(_input_ptr: i32, _input_len: i32, _out_ptr: i32) -> i32 {
    0
}
"#,
        );

        let err = validate_rust_unit_source(&source).expect_err("source should fail");
        assert!(err.contains("no_mangle"));
    }

    #[test]
    fn rust_unit_standard_id_reads_metadata_literal() {
        let source = rust_unit_source_fixture(
            "standard-id",
            r#"
#![no_std]

edgerun_unit::metadata!(9110);

#[edgerun_unit::export]
pub(crate) fn http_tchar_valid(value: i32) -> i32 {
    value
}
"#,
        );

        assert_eq!(
            read_rust_unit_standard_id(&source.rust_source).unwrap(),
            9110
        );
    }

    #[test]
    fn rust_unit_standard_id_rejects_metadata_expression() {
        let source = rust_unit_source_fixture(
            "standard-id-expression",
            r#"
#![no_std]

pub(crate) const RFC: i32 = 9110;
edgerun_unit::metadata!(RFC);

#[edgerun_unit::export]
pub(crate) fn http_tchar_valid(value: i32) -> i32 {
    value
}
"#,
        );

        let err = read_rust_unit_standard_id(&source.rust_source).expect_err("standard id");
        assert!(err.contains("i32 literal"));
    }

    #[test]
    fn wasm_unit_surface_policy_rejects_imports() {
        let mut surface = valid_test_surface();
        surface.import_count = 1;

        let err = validate_wasm_unit_surface("test-unit", &surface).expect_err("surface");
        assert!(err.contains("imports"));
    }

    #[test]
    fn wasm_unit_surface_policy_requires_owned_memory() {
        let mut surface = valid_test_surface();
        surface
            .exports
            .retain(|export| export.kind != ExternalKind::Memory);

        let err = validate_wasm_unit_surface("test-unit", &surface).expect_err("surface");
        assert!(err.contains("memory"));
    }

    #[test]
    fn wasm_unit_surface_policy_requires_protocol_exports() {
        let mut surface = valid_test_surface();
        surface
            .exports
            .retain(|export| export.name != "proto_abi_version");

        let err = validate_wasm_unit_surface("test-unit", &surface).expect_err("surface");
        assert!(err.contains("proto_abi_version"));
    }

    #[test]
    fn wasm_unit_surface_policy_requires_cost_profile() {
        let mut surface = valid_test_surface();
        surface.exports.push(ExportSurface {
            name: "unpriced_export".to_owned(),
            kind: ExternalKind::Func,
            ty: Some(FuncType::new([ValType::I32], [ValType::I32])),
        });

        let err = validate_wasm_unit_surface("test-unit", &surface).expect_err("surface");
        assert!(err.contains("cost profile"));
    }

    #[test]
    fn api_verifier_rejects_zero_function_cost() {
        let manifest = unit("hmac-sha256-rfc2104").expect("unit");
        let mut api = hmac_wire_api();
        api.functions
            .iter_mut()
            .find(|function| function.name == b"hmac_sha256")
            .expect("hmac function")
            .cost_base = 0;
        let bytes = sdk_wire_record_bytes(SdkWireRecord::UnitApi(api));
        assert!(!verify_binary_api(manifest, &bytes));
    }

    #[test]
    fn api_verifier_rejects_per_byte_cost_change() {
        let manifest = unit("hmac-sha256-rfc2104").expect("unit");
        let mut api = hmac_wire_api();
        api.functions
            .iter_mut()
            .find(|function| function.name == b"hmac_sha256")
            .expect("hmac function")
            .cost_per_byte = 2;
        let bytes = sdk_wire_record_bytes(SdkWireRecord::UnitApi(api));
        assert!(!verify_binary_api(manifest, &bytes));
    }

    #[test]
    fn api_verifier_rejects_signature_change() {
        let manifest = unit("hmac-sha256-rfc2104").expect("unit");
        let mut api = hmac_wire_api();
        api.functions
            .iter_mut()
            .find(|function| function.name == b"hmac_sha256")
            .expect("hmac function")
            .params
            .push(1);
        let bytes = sdk_wire_record_bytes(SdkWireRecord::UnitApi(api));
        assert!(!verify_binary_api(manifest, &bytes));
    }

    fn hmac_wire_api() -> edgerun_wire::UnitApi {
        let bytes = include_bytes!("../units/hmac-sha256-rfc2104/api.edm").to_vec();
        match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&bytes)
            .expect("hmac api")
        {
            SdkWireRecord::UnitApi(api) => api,
            _ => panic!("not a unit api"),
        }
    }

    #[test]
    fn utf8_unit_validates_rfc3629_sequences() {
        let mut unit = instantiate_test_unit("utf8-rfc3629");

        unit.write(0, "hello \u{03c0}".as_bytes());
        assert_eq!(unit.call("utf8_validate", [0, 8, 0, 0, 0]), 0);

        unit.write(0, &[0xc0, 0x80]);
        assert_eq!(unit.call("utf8_validate", [0, 2, 0, 0, 0]), 2);

        unit.write(0, &[0xe2, 0x82]);
        assert_eq!(unit.call("utf8_validate", [0, 2, 0, 0, 0]), 1);
    }

    #[test]
    fn base64url_unit_roundtrips_rfc4648_vector() {
        let mut unit = instantiate_test_unit("base64url-rfc4648");

        unit.write(0, b"foobar");
        assert_eq!(unit.call("base64url_encoded_len", [6, 0, 0, 0, 0]), 8);
        assert_eq!(unit.call("base64url_encode", [0, 6, 128, 0, 0]), 0);
        let encoded = unit.read::<8>(128);
        assert_eq!(&encoded, b"Zm9vYmFy");

        assert_eq!(unit.call("base64url_decode", [128, 8, 256, 300, 0]), 0);
        let decoded_len = unit.read::<4>(300);
        assert_eq!(u32::from_le_bytes(decoded_len), 6);
        let decoded = unit.read::<6>(256);
        assert_eq!(&decoded, b"foobar");
    }

    #[test]
    fn tftp_unit_parses_rfc1350_messages() {
        let mut unit = instantiate_test_unit("tftp-rfc1350");

        assert_eq!(unit.call("tftp_opcode_valid", [6, 0, 0, 0, 0]), 1);
        assert_eq!(unit.call("tftp_opcode_valid", [7, 0, 0, 0, 0]), 0);

        unit.write(16, &[0, 4, 0, 1]);
        assert_eq!(unit.call("tftp_parse", [16, 4, 64, 0, 0]), 0);
        let mut fields = unit.read::<12>(64);
        assert_eq!(u32::from_le_bytes(fields[0..4].try_into().unwrap()), 4);
        assert_eq!(u32::from_le_bytes(fields[4..8].try_into().unwrap()), 18);
        assert_eq!(u32::from_le_bytes(fields[8..12].try_into().unwrap()), 2);

        unit.write(16, &[0, 3, 0, 1, b'h', b'i']);
        assert_eq!(unit.call("tftp_parse", [16, 6, 64, 0, 0]), 0);
        fields = unit.read::<12>(64);
        assert_eq!(u32::from_le_bytes(fields[0..4].try_into().unwrap()), 3);
        assert_eq!(u32::from_le_bytes(fields[4..8].try_into().unwrap()), 18);
        assert_eq!(u32::from_le_bytes(fields[8..12].try_into().unwrap()), 4);

        unit.write(16, &[0]);
        assert_eq!(unit.call("tftp_parse", [16, 1, 64, 0, 0]), 1);

        unit.write(16, &[0, 7]);
        assert_eq!(unit.call("tftp_parse", [16, 2, 64, 0, 0]), 2);

        unit.write(16, &[0, 4, 0, 1, 0]);
        assert_eq!(unit.call("tftp_parse", [16, 5, 64, 0, 0]), 3);

        unit.write(16, &[0, 3, 0]);
        assert_eq!(unit.call("tftp_parse", [16, 3, 64, 0, 0]), 4);
    }

    #[test]
    fn byte_tools_unit_compares_and_normalizes_bytes() {
        let mut unit = instantiate_test_unit("byte-tools-v1");

        unit.write(0, b"Content-Type");
        unit.write(64, b"content-type");
        assert_eq!(unit.call("byte_ascii_case_eq", [0, 12, 64, 12, 0]), 1);
        assert_eq!(unit.call("byte_prefix", [0, 12, 64, 7, 0]), 0);
        assert_eq!(unit.call("byte_find", [0, 12, 45, 0, 0]), 7);
        assert_eq!(unit.call("byte_ascii_lower", [0, 12, 128, 0, 0]), 0);
        assert_eq!(unit.call("byte_eq", [64, 12, 128, 12, 0]), 1);
    }

    #[test]
    fn constant_time_eq_unit_reports_status_bytes() {
        let mut unit = instantiate_test_unit("constant-time-eq-v1");

        unit.write(0, b"abcdef");
        unit.write(64, b"abcdef");
        assert_eq!(unit.call("constant_time_eq", [0, 6, 64, 6, 128]), 0);
        let mut status = unit.read::<1>(128);
        assert_eq!(status, [0]);

        unit.write(64, b"abcdeg");
        assert_eq!(unit.call("constant_time_eq", [0, 6, 64, 6, 128]), 0);
        status = unit.read::<1>(128);
        assert_eq!(status, [1]);

        assert_eq!(unit.call("constant_time_eq", [0, 6, 64, 5, 128]), 0);
        status = unit.read::<1>(128);
        assert_eq!(status, [2]);
    }

    #[test]
    fn http_field_unit_parses_rfc9110_field_line() {
        let mut unit = instantiate_test_unit("http-field-rfc9110");

        unit.write(0, b"Content-Type: text/plain \t");
        assert_eq!(unit.call("http_field_line_parse", [0, 26, 128, 0, 0]), 0);
        let fields = unit.read::<16>(128);
        assert_eq!(u32::from_le_bytes(fields[0..4].try_into().unwrap()), 12);
        assert_eq!(u32::from_le_bytes(fields[4..8].try_into().unwrap()), 14);
        assert_eq!(u32::from_le_bytes(fields[8..12].try_into().unwrap()), 10);
        assert_eq!(u32::from_le_bytes(fields[12..16].try_into().unwrap()), 12);

        unit.write(0, b"Bad Name: value");
        assert_eq!(unit.call("http_field_line_parse", [0, 15, 128, 0, 0]), 2);
    }

    #[test]
    fn cbor_unit_parses_rfc8949_item_heads() {
        let mut unit = instantiate_test_unit("cbor-rfc8949");

        unit.write(0, &[0x18, 0x2a]);
        assert_eq!(unit.call("cbor_head_parse", [0, 2, 128, 0, 0]), 0);
        let mut fields = unit.read::<20>(128);
        assert_eq!(u32::from_le_bytes(fields[0..4].try_into().unwrap()), 0);
        assert_eq!(u32::from_le_bytes(fields[4..8].try_into().unwrap()), 24);
        assert_eq!(u32::from_le_bytes(fields[8..12].try_into().unwrap()), 2);
        assert_eq!(u32::from_le_bytes(fields[12..16].try_into().unwrap()), 42);

        unit.write(0, &[0x64]);
        assert_eq!(unit.call("cbor_head_parse", [0, 1, 128, 0, 0]), 0);
        fields = unit.read::<20>(128);
        assert_eq!(u32::from_le_bytes(fields[0..4].try_into().unwrap()), 3);
        assert_eq!(u32::from_le_bytes(fields[12..16].try_into().unwrap()), 4);

        unit.write(0, &[0x9f]);
        assert_eq!(unit.call("cbor_head_parse", [0, 1, 128, 0, 0]), 3);
    }
}
