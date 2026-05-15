use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Debug)]
struct Options {
    apk: String,
    all: bool,
    json: bool,
    summary: bool,
    profile: bool,
    spec: bool,
    scaffold: bool,
    emit_scaffold: Option<String>,
    external_tools: Option<String>,
    top: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct ApiCall {
    class_name: String,
    method_name: String,
    args: Vec<String>,
    return_type: String,
}

#[derive(Clone, Debug)]
struct ZipEntry {
    name: String,
    compression: u16,
    compressed_size: usize,
    uncompressed_size: usize,
    local_header_offset: usize,
}

#[derive(Clone, Debug)]
struct DexHeader {
    string_ids_size: usize,
    string_ids_off: usize,
    type_ids_size: usize,
    type_ids_off: usize,
    proto_ids_size: usize,
    proto_ids_off: usize,
    method_ids_size: usize,
    method_ids_off: usize,
    class_defs_size: usize,
    class_defs_off: usize,
}

#[derive(Clone, Debug)]
struct MethodRef {
    class_name: String,
    method_name: String,
    args: Vec<String>,
    return_type: String,
}

#[derive(Clone, Debug, Default)]
struct CodeScan {
    method_indices: Vec<u32>,
    string_indices: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct SourceNetworkFinding {
    kind: String,
    file: String,
    line: usize,
    detail: String,
    context: String,
}

#[derive(Clone, Debug)]
struct Signal {
    name: &'static str,
    prefixes: &'static [&'static str],
    keywords: &'static [&'static str],
}

#[derive(Clone, Debug, Default)]
struct ApkProfile {
    manifest: ManifestProfile,
    native_libs: BTreeSet<String>,
    domains: BTreeSet<String>,
    uris: BTreeSet<String>,
    uri_schemes: BTreeMap<String, u64>,
    urls: BTreeSet<String>,
    dex_packages: BTreeMap<String, u64>,
    dex_classes: BTreeSet<String>,
    class_summaries: BTreeMap<String, ClassSummary>,
    method_summaries: BTreeMap<String, MethodSummary>,
    signal_callers: BTreeMap<(String, &'static str), u64>,
    resource_files: BTreeMap<String, u64>,
    ui_elements: BTreeMap<String, u64>,
    ui_texts: BTreeSet<String>,
    asset_files: BTreeSet<String>,
    uri_string_count: usize,
    url_count: usize,
}

#[derive(Clone, Debug, Default)]
struct ClassSummary {
    package_name: String,
    method_count: u64,
    internal_calls: u64,
    platform_calls: u64,
    callee_packages: BTreeMap<String, u64>,
    callee_classes: BTreeMap<String, u64>,
    signals: BTreeMap<&'static str, u64>,
}

#[derive(Clone, Debug, Default)]
struct MethodSummary {
    class_name: String,
    method_name: String,
    platform_calls: u64,
    internal_calls: u64,
    callee_packages: BTreeMap<String, u64>,
    callee_classes: BTreeMap<String, u64>,
    signals: BTreeMap<&'static str, u64>,
    string_constants: BTreeMap<String, u64>,
    uri_constants: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, Default)]
struct ManifestProfile {
    package_name: Option<String>,
    permissions: BTreeSet<String>,
    activities: Vec<ComponentProfile>,
    services: Vec<ComponentProfile>,
    receivers: Vec<ComponentProfile>,
    providers: Vec<ComponentProfile>,
}

#[derive(Clone, Debug, Default)]
struct ComponentProfile {
    name: String,
    exported: Option<bool>,
    actions: BTreeSet<String>,
    categories: BTreeSet<String>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let opts = parse_args()?;
    let apk = fs::read(&opts.apk)?;
    let entries = read_zip_entries(&apk)?;
    let mut counts: BTreeMap<ApiCall, u64> = BTreeMap::new();
    let mut profile = ApkProfile::default();
    let mut dex_count = 0usize;

    for entry in &entries {
        if entry.name == "AndroidManifest.xml" {
            let manifest = extract_zip_entry(&apk, entry)?;
            match parse_android_manifest(&manifest) {
                Ok(manifest) => profile.manifest = manifest,
                Err(err) => eprintln!("warning: failed to parse AndroidManifest.xml: {err}"),
            }
        } else if is_native_lib(&entry.name) {
            profile.native_libs.insert(entry.name.clone());
        } else if is_resource_entry(&entry.name) {
            collect_resource_entry(&apk, entry, &mut profile);
        } else if is_classes_dex(&entry.name) {
            dex_count += 1;
            let dex = extract_zip_entry(&apk, entry)?;
            let strings = extract_dex_strings(&dex)?;
            collect_domains(&strings, &mut profile);
            collect_dex_packages(&dex, &mut profile)?;
            let dex_calls = extract_dex_calls(&dex, &mut profile)?;
            for (api, count) in dex_calls {
                if opts.all || is_android_api(&api.class_name) {
                    *counts.entry(api).or_insert(0) += count;
                }
            }
        }
    }

    if dex_count == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "APK does not contain classes*.dex",
        ));
    }

    if let Some(out_dir) = opts.external_tools.as_deref() {
        run_external_tools(&opts.apk, Path::new(out_dir))?;
        eprintln!("wrote external tool output: {out_dir}");
    }

    if let Some(out_dir) = opts.emit_scaffold.as_deref() {
        emit_edgerun_scaffold(Path::new(out_dir), &profile, &counts, opts.top)?;
        println!("wrote scaffold: {out_dir}");
    } else if opts.scaffold {
        print_edgerun_scaffold(&profile, &counts, opts.top);
    } else if opts.spec {
        print_edgerun_app_spec(&profile, &counts, opts.top);
    } else if opts.profile {
        print_profile(&profile, &counts, opts.top);
    } else if opts.summary {
        print_summary(&counts, opts.top);
    } else if opts.json {
        print_json(&counts);
    } else {
        for (api, count) in counts {
            println!(
                "{count}\t{}->{}({}){}",
                api.class_name,
                api.method_name,
                api.args.join(","),
                api.return_type
            );
        }
    }

    Ok(())
}

fn parse_args() -> io::Result<Options> {
    let mut apk = None;
    let mut all = false;
    let mut json = false;
    let mut summary = false;
    let mut profile = false;
    let mut spec = false;
    let mut scaffold = false;
    let mut emit_scaffold = None;
    let mut external_tools = None;
    let mut top = 20usize;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--all" => all = true,
            "--json" => json = true,
            "--summary" => summary = true,
            "--profile" => profile = true,
            "--spec" => spec = true,
            "--scaffold" => scaffold = true,
            "--emit-scaffold" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--emit-scaffold requires a path",
                    )
                })?;
                emit_scaffold = Some(value);
            }
            "--external-tools" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "--external-tools requires a directory",
                    )
                })?;
                external_tools = Some(value);
            }
            "--top" => {
                let value = args.next().ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "--top requires a number")
                })?;
                top = value.parse::<usize>().map_err(|_| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        format!("invalid --top value: {value}"),
                    )
                })?;
            }
            "-h" | "--help" => {
                print_usage();
                std::process::exit(0);
            }
            _ if arg.starts_with('-') => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown option: {arg}"),
                ));
            }
            _ => {
                if apk.replace(arg).is_some() {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "expected one APK path",
                    ));
                }
            }
        }
    }

    let Some(apk) = apk else {
        print_usage();
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "missing APK path",
        ));
    };

    Ok(Options {
        apk,
        all,
        json,
        summary,
        profile,
        spec,
        scaffold,
        emit_scaffold,
        external_tools,
        top,
    })
}

fn print_usage() {
    eprintln!(
        "usage: edgerun-apk-api-calls [--all] [--json] [--summary] [--profile] [--spec] [--scaffold] [--emit-scaffold DIR] [--external-tools DIR] [--top N] <app.apk>\n\
         \n\
         Lists DEX invoke targets with count, class, method, argument types, and return type.\n\
         Default output includes Android platform APIs only. Use --all for every invoke target.\n\
         Use --summary to group APIs into top packages/classes/methods and capability signals.\n\
         Use --profile to add manifest permissions, components, domains, and native libs.\n\
         Use --spec to emit an Edgerun app replacement skeleton.\n\
         Use --scaffold to emit a concrete Edgerun replacement blueprint.\n\
         Use --emit-scaffold DIR to write draft replacement files.\n\
         Use --external-tools DIR to run installed external decompilers/extractors into DIR."
    );
}

fn run_external_tools(apk: &str, out_dir: &Path) -> io::Result<()> {
    fs::create_dir_all(out_dir)?;
    let mut report = String::new();
    report.push_str("external_tools\n");
    report.push_str(&format!("  apk: {apk}\n"));

    let tools = [
        ("androguard", &["--version"][..]),
        ("jadx", &["--version"][..]),
        ("apktool", &["--version"][..]),
        ("strings", &["--version"][..]),
        ("unzip", &["-v"][..]),
    ];
    for (tool, version_args) in tools {
        match command_output(tool, version_args) {
            Ok(output) => {
                let first = first_output_line(&output);
                report.push_str(&format!("  {tool}: available {first}\n"));
            }
            Err(_) => report.push_str(&format!("  {tool}: missing\n")),
        }
    }

    if command_available("jadx") {
        let jadx_dir = out_dir.join("jadx");
        let args = vec![
            "--show-bad-code".to_string(),
            "--deobf".to_string(),
            "-d".to_string(),
            jadx_dir.display().to_string(),
            apk.to_string(),
        ];
        run_external_command(out_dir, "jadx", &args)?;
        report.push_str("  jadx_output: jadx/\n");
        let findings = scan_jadx_network_findings(&jadx_dir)?;
        fs::write(
            out_dir.join("source-network.md"),
            render_source_network_findings(&findings),
        )?;
        report.push_str("  source_network_output: source-network.md\n");
    }

    if command_available("apktool") {
        let apktool_dir = out_dir.join("apktool");
        let args = vec![
            "d".to_string(),
            "-f".to_string(),
            "-o".to_string(),
            apktool_dir.display().to_string(),
            apk.to_string(),
        ];
        run_external_command(out_dir, "apktool", &args)?;
        report.push_str("  apktool_output: apktool/\n");
        let findings = scan_apktool_evidence(&apktool_dir)?;
        fs::write(
            out_dir.join("apktool-evidence.md"),
            render_evidence_findings(
                "Apktool Resource and Smali Evidence",
                "These findings come from decoded resources and smali. They survive many Java decompiler failures, but are still static evidence.",
                &findings,
            ),
        )?;
        report.push_str("  apktool_evidence_output: apktool-evidence.md\n");
    }

    if command_available("androguard") {
        let apkid_args = vec!["apkid".to_string(), apk.to_string()];
        run_external_command_named(out_dir, "androguard", "androguard-apkid", &apkid_args)?;
        report.push_str("  androguard_apkid_output: androguard-apkid.stdout.txt\n");

        let axml_args = vec!["axml".to_string(), apk.to_string()];
        run_external_command_named(out_dir, "androguard", "androguard-axml", &axml_args)?;
        report.push_str("  androguard_axml_output: androguard-axml.stdout.txt\n");

        if command_available("python3") || command_available("python") {
            run_androguard_python_report(apk, out_dir)?;
            report.push_str("  androguard_summary_output: androguard-summary.txt\n");
        }
    }

    if command_available("strings") {
        let output = command_output("strings", &["-a", apk])?;
        fs::write(out_dir.join("strings.txt"), &output.stdout)?;
        fs::write(out_dir.join("strings.stderr.txt"), &output.stderr)?;
        report.push_str("  strings_output: strings.txt\n");
    }

    fs::write(out_dir.join("tools.txt"), report)?;
    Ok(())
}

fn scan_apktool_evidence(apktool_dir: &Path) -> io::Result<Vec<SourceNetworkFinding>> {
    let mut files = Vec::new();
    collect_apktool_evidence_files(apktool_dir, &mut files)?;
    let mut findings = BTreeSet::new();

    for file in files {
        let Ok(text) = fs::read_to_string(&file) else {
            continue;
        };
        let relative = file
            .strip_prefix(apktool_dir)
            .unwrap_or(file.as_path())
            .display()
            .to_string();
        let lines = text.lines().collect::<Vec<_>>();
        let ext = file.extension().and_then(|ext| ext.to_str()).unwrap_or("");
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if ext == "smali" {
                if let Some((kind, detail)) = smali_evidence_pattern(trimmed) {
                    findings.insert(SourceNetworkFinding {
                        kind: kind.to_string(),
                        file: relative.clone(),
                        line: idx + 1,
                        detail,
                        context: source_context(&lines, idx),
                    });
                }
            } else if let Some((kind, detail)) = resource_evidence_pattern(&relative, trimmed) {
                findings.insert(SourceNetworkFinding {
                    kind: kind.to_string(),
                    file: relative.clone(),
                    line: idx + 1,
                    detail,
                    context: source_context(&lines, idx),
                });
            }
        }
    }

    Ok(findings.into_iter().collect())
}

fn collect_apktool_evidence_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_apktool_evidence_files(&path, out)?;
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| matches!(ext, "smali" | "xml"))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn smali_evidence_pattern(line: &str) -> Option<(&'static str, String)> {
    let patterns = [
        ("smali_http_client", "Lokhttp3/"),
        ("smali_http_client", "Lretrofit2/"),
        ("smali_url_connection", "Ljava/net/HttpURLConnection;"),
        ("smali_url", "Ljava/net/URL;"),
        ("smali_websocket", "WebSocket"),
        ("smali_uri_parse", "Landroid/net/Uri;->parse"),
        ("smali_webview", "Landroid/webkit/WebView;"),
        ("smali_javascript_bridge", "addJavascriptInterface"),
        ("smali_tls", "Ljavax/net/ssl/"),
        ("smali_crypto", "Ljavax/crypto/"),
        ("smali_keystore", "KeyStore"),
        ("smali_reflection", "Ljava/lang/reflect/"),
        ("smali_dynamic_load", "Ldalvik/system/DexClassLoader;"),
        ("smali_dynamic_load", "Ldalvik/system/PathClassLoader;"),
        ("smali_process", "Ljava/lang/Runtime;->exec"),
        ("smali_process", "Ljava/lang/ProcessBuilder;"),
        ("smali_file_io", "Ljava/io/File;"),
        ("smali_preferences", "Landroid/content/SharedPreferences;"),
        ("smali_sqlite", "Landroid/database/sqlite/"),
        ("smali_clipboard", "Landroid/content/ClipboardManager;"),
        ("smali_camera", "Landroid/hardware/Camera;"),
        ("smali_camera", "Landroid/hardware/camera2/"),
        ("smali_location", "Landroid/location/"),
        ("smali_bluetooth", "Landroid/bluetooth/"),
        ("smali_nfc", "Landroid/nfc/"),
        ("smali_sms", "Landroid/telephony/SmsManager;"),
        ("smali_contacts", "Landroid/provider/ContactsContract;"),
        ("smali_calendar", "Landroid/provider/CalendarContract;"),
        ("smali_biometric", "Landroid/hardware/biometrics/"),
        ("smali_biometric", "Landroidx/biometric/"),
        ("smali_firebase", "Lcom/google/firebase/"),
        ("smali_play_services", "Lcom/google/android/gms/"),
        ("smali_braze", "Lcom/braze/"),
        ("smali_amplitude", "Lamplitude/"),
        ("smali_segment", "Lcom/segment/analytics/"),
        ("smali_sentry", "Lio/sentry/"),
        ("smali_datadog", "Lcom/datadog/"),
        ("smali_crashlytics", "Lcom/google/firebase/crashlytics/"),
    ];
    if let Some((kind, _)) = patterns.iter().find(|(_, pattern)| line.contains(pattern)) {
        return Some((*kind, line.to_string()));
    }
    if line.starts_with("const-string") {
        if let Some(value) = smali_const_string_value(line) {
            let lower = value.to_ascii_lowercase();
            if has_uri_scheme(&lower) {
                return Some(("smali_uri_string", value));
            }
            if lower.contains("api")
                || lower.contains("auth")
                || lower.contains("login")
                || lower.contains("token")
                || lower.contains("graphql")
                || lower.contains("socket")
                || lower.contains("analytics")
            {
                return Some(("smali_interesting_string", value));
            }
        }
    }
    None
}

fn smali_const_string_value(line: &str) -> Option<String> {
    let start = line.find('"')?;
    let end = line.rfind('"')?;
    (end > start).then(|| line[start + 1..end].to_string())
}

fn resource_evidence_pattern(relative: &str, line: &str) -> Option<(&'static str, String)> {
    let lower = line.to_ascii_lowercase();
    if relative == "AndroidManifest.xml" {
        if lower.contains("<data ")
            && (lower.contains("android:scheme") || lower.contains("android:host"))
        {
            return Some(("manifest_deep_link", line.to_string()));
        }
        if lower.contains("networksecurityconfig") {
            return Some(("manifest_network_security_config", line.to_string()));
        }
        if lower.contains("usescleartexttraffic") {
            return Some(("manifest_cleartext_traffic", line.to_string()));
        }
    }
    if relative.contains("network_security") || lower.contains("cleartexttrafficpermitted") {
        if lower.contains("<domain") {
            return Some(("network_security_domain", line.to_string()));
        }
        if lower.contains("cleartexttrafficpermitted") {
            return Some(("network_security_cleartext", line.to_string()));
        }
        if lower.contains("trust-anchors")
            || lower.contains("certificates")
            || lower.contains("pin-set")
        {
            return Some(("network_security_trust", line.to_string()));
        }
    }
    if lower.contains("http://")
        || lower.contains("https://")
        || lower.contains("ws://")
        || lower.contains("wss://")
        || lower.contains("ftp://")
        || lower.contains("sftp://")
        || lower.contains("content://")
        || lower.contains("intent://")
    {
        return Some(("resource_uri", line.to_string()));
    }
    if relative.contains("/res/values/") {
        let interesting = [
            "api",
            "auth",
            "login",
            "token",
            "client_id",
            "redirect",
            "graphql",
            "socket",
            "analytics",
        ];
        if interesting.iter().any(|token| lower.contains(token)) {
            return Some(("resource_interesting_value", line.to_string()));
        }
    }
    None
}

fn has_uri_scheme(value: &str) -> bool {
    [
        "http://",
        "https://",
        "ws://",
        "wss://",
        "ftp://",
        "ftps://",
        "sftp://",
        "mqtt://",
        "mqtts://",
        "rtsp://",
        "rtmp://",
        "tcp://",
        "udp://",
        "content://",
        "file://",
        "android-app://",
        "market://",
        "intent://",
        "geo:",
        "mailto:",
        "tel:",
        "sms:",
    ]
    .iter()
    .any(|scheme| value.contains(scheme))
}

fn scan_jadx_network_findings(jadx_dir: &Path) -> io::Result<Vec<SourceNetworkFinding>> {
    let mut files = Vec::new();
    collect_source_files(jadx_dir, &mut files)?;
    let mut findings = BTreeSet::new();

    for file in files {
        let Ok(text) = fs::read_to_string(&file) else {
            continue;
        };
        let relative = file
            .strip_prefix(jadx_dir)
            .unwrap_or(file.as_path())
            .display()
            .to_string();
        let lines = text.lines().collect::<Vec<_>>();
        let mut pending_annotations: Vec<(usize, String)> = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if let Some(annotation) = retrofit_annotation(trimmed) {
                pending_annotations.push((idx + 1, annotation));
                findings.insert(SourceNetworkFinding {
                    kind: "retrofit_annotation".to_string(),
                    file: relative.clone(),
                    line: idx + 1,
                    detail: trimmed.to_string(),
                    context: source_context(&lines, idx),
                });
                continue;
            }

            if !pending_annotations.is_empty()
                && (trimmed.contains(")")
                    || trimmed.contains("Call<")
                    || trimmed.contains("suspend ")
                    || trimmed.contains("Observable<")
                    || trimmed.contains("Single<"))
            {
                for (line_no, annotation) in pending_annotations.drain(..) {
                    findings.insert(SourceNetworkFinding {
                        kind: "retrofit_route".to_string(),
                        file: relative.clone(),
                        line: line_no,
                        detail: format!("{annotation} => {trimmed}"),
                        context: source_context(&lines, idx),
                    });
                }
            }

            if let Some(detail) = network_source_pattern(trimmed) {
                findings.insert(SourceNetworkFinding {
                    kind: detail.0.to_string(),
                    file: relative.clone(),
                    line: idx + 1,
                    detail: detail.1,
                    context: source_context(&lines, idx),
                });
            }
        }
    }

    Ok(findings.into_iter().collect())
}

fn collect_source_files(dir: &Path, out: &mut Vec<PathBuf>) -> io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_source_files(&path, out)?;
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| matches!(ext, "java" | "kt"))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn retrofit_annotation(line: &str) -> Option<String> {
    let annotations = [
        "@GET",
        "@POST",
        "@PUT",
        "@PATCH",
        "@DELETE",
        "@HEAD",
        "@OPTIONS",
        "@HTTP",
        "@Headers",
        "@Url",
        "@Body",
        "@Query",
        "@QueryMap",
        "@Path",
        "@Field",
        "@FieldMap",
        "@Part",
        "@Multipart",
        "@FormUrlEncoded",
    ];
    annotations
        .iter()
        .any(|annotation| line.starts_with(annotation))
        .then(|| line.to_string())
}

fn network_source_pattern(line: &str) -> Option<(&'static str, String)> {
    let patterns = [
        ("okhttp_request", "Request.Builder"),
        ("okhttp_url", ".url("),
        ("okhttp_header", ".addHeader("),
        ("okhttp_header", ".header("),
        ("okhttp_client", "OkHttpClient"),
        ("websocket", "newWebSocket"),
        ("websocket", "WebSocketListener"),
        ("retrofit_builder", "Retrofit.Builder"),
        ("retrofit_base_url", ".baseUrl("),
        ("url_connection", "openConnection("),
        ("url_connection", "HttpURLConnection"),
        ("uri_parse", "Uri.parse("),
        ("json_key", "JSONObject"),
        ("graphql", "graphql"),
        ("graphql", "GraphQL"),
    ];
    patterns
        .iter()
        .find(|(_, pattern)| line.contains(pattern))
        .map(|(kind, _)| (*kind, line.to_string()))
}

fn source_context(lines: &[&str], idx: usize) -> String {
    let start = idx.saturating_sub(1);
    let end = (idx + 2).min(lines.len());
    lines[start..end]
        .iter()
        .map(|line| line.trim())
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_source_network_findings(findings: &[SourceNetworkFinding]) -> String {
    render_evidence_findings(
        "Decompiled Source Network Findings",
        "These findings come from decompiled source and are static evidence, not runtime proof.",
        findings,
    )
}

fn render_evidence_findings(title: &str, note: &str, findings: &[SourceNetworkFinding]) -> String {
    let mut out = String::new();
    out.push_str(&format!("# {title}\n\n"));
    out.push_str(note);
    out.push_str("\n\n");
    let mut by_kind: BTreeMap<&str, u64> = BTreeMap::new();
    for finding in findings {
        *by_kind.entry(&finding.kind).or_insert(0) += 1;
    }
    out.push_str("## Summary\n\n");
    for (kind, count) in ranked_u64(&by_kind) {
        out.push_str(&format!("- `{kind}`: {count}\n"));
    }
    out.push('\n');
    out.push_str("## Findings\n\n");
    for finding in findings.iter().take(1000) {
        out.push_str(&format!(
            "- `{}` [{}:{}] {}\n",
            finding.kind,
            markdown_escape_inline(&finding.file),
            finding.line,
            markdown_escape_inline(&finding.detail)
        ));
        if !finding.context.is_empty() {
            out.push_str(&format!(
                "  - context: `{}`\n",
                markdown_escape_inline(&finding.context)
            ));
        }
    }
    if findings.len() > 1000 {
        out.push_str(&format!("\n... {} more findings\n", findings.len() - 1000));
    }
    out
}

fn command_available(tool: &str) -> bool {
    Command::new(tool).arg("--version").output().is_ok()
}

fn command_output(tool: &str, args: &[&str]) -> io::Result<std::process::Output> {
    Command::new(tool).args(args).output()
}

fn run_external_command(out_dir: &Path, tool: &str, args: &[String]) -> io::Result<()> {
    run_external_command_named(out_dir, tool, tool, args)
}

fn run_external_command_named(
    out_dir: &Path,
    command: &str,
    label: &str,
    args: &[String],
) -> io::Result<()> {
    let output = Command::new(command).args(args).output()?;
    fs::write(
        out_dir.join(format!("{label}.stdout.txt")),
        output.stdout.as_slice(),
    )?;
    fs::write(
        out_dir.join(format!("{label}.stderr.txt")),
        output.stderr.as_slice(),
    )?;
    fs::write(
        out_dir.join(format!("{label}.status.txt")),
        format!(
            "success={}\nstatus={:?}\n",
            output.status.success(),
            output.status
        ),
    )?;
    Ok(())
}

fn run_androguard_python_report(apk: &str, out_dir: &Path) -> io::Result<()> {
    let python = if command_available("python3") {
        "python3"
    } else {
        "python"
    };
    let script = r#"
import sys

SCHEMES = (
    "http://", "https://", "ws://", "wss://", "ftp://", "ftps://", "sftp://",
    "mqtt://", "mqtts://", "rtsp://", "rtmp://", "tcp://", "udp://",
    "content://", "file://", "android-app://", "market://", "intent://",
    "geo:", "mailto:", "tel:", "sms:",
)

def safe_call(obj, name, default=None):
    try:
        value = getattr(obj, name)()
        return value if value is not None else default
    except Exception as exc:
        return default

def print_list(title, values, limit=500):
    print(title)
    values = sorted({str(value) for value in values if value is not None})
    for value in values[:limit]:
        print("  " + value)
    if len(values) > limit:
        print("  ... %d more" % (len(values) - limit))

def string_value(value):
    try:
        if hasattr(value, "get_value"):
            return value.get_value()
    except Exception:
        pass
    return str(value)

apk_path = sys.argv[1]
try:
    from loguru import logger
    logger.remove()
except Exception:
    pass
from androguard.core.apk import APK
from androguard.core.dex import DEX

a = APK(apk_path)
print("package: %s" % safe_call(a, "get_package", ""))
print("version_name: %s" % safe_call(a, "get_androidversion_name", ""))
print("version_code: %s" % safe_call(a, "get_androidversion_code", ""))
print("min_sdk: %s" % safe_call(a, "get_min_sdk_version", ""))
print("target_sdk: %s" % safe_call(a, "get_target_sdk_version", ""))

print_list("permissions:", safe_call(a, "get_permissions", []) or [])
print_list("activities:", safe_call(a, "get_activities", []) or [])
print_list("services:", safe_call(a, "get_services", []) or [])
print_list("receivers:", safe_call(a, "get_receivers", []) or [])
print_list("providers:", safe_call(a, "get_providers", []) or [])

uri_strings = set()
interesting_strings = set()
class_count = 0
method_count = 0
for dex_bytes in a.get_all_dex():
    dex = DEX(dex_bytes)
    try:
        class_count += len(dex.get_classes())
    except Exception:
        pass
    try:
        method_count += len(dex.get_methods())
    except Exception:
        pass
    try:
        strings = dex.get_strings()
    except Exception:
        strings = []
    for raw in strings:
        value = string_value(raw)
        if any(scheme in value for scheme in SCHEMES):
            uri_strings.add(value)
        elif (
            len(value) >= 5
            and len(value) <= 180
            and any(token in value.lower() for token in ("api", "auth", "login", "token", "event", "analytics", "graphql", "socket"))
        ):
            interesting_strings.add(value)

print("dex_classes: %d" % class_count)
print("dex_methods: %d" % method_count)
print_list("uri_strings:", uri_strings, 2000)
print_list("interesting_strings:", interesting_strings, 1000)
"#;
    let output = Command::new(python)
        .arg("-c")
        .arg(script)
        .arg(apk)
        .output()?;
    fs::write(
        out_dir.join("androguard-summary.txt"),
        output.stdout.as_slice(),
    )?;
    fs::write(
        out_dir.join("androguard-summary.stderr.txt"),
        output.stderr.as_slice(),
    )?;
    fs::write(
        out_dir.join("androguard-summary.status.txt"),
        format!(
            "success={}\nstatus={:?}\n",
            output.status.success(),
            output.status
        ),
    )?;
    Ok(())
}

fn first_output_line(output: &std::process::Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    stdout
        .lines()
        .chain(stderr.lines())
        .next()
        .unwrap_or("")
        .trim()
        .to_string()
}

fn is_classes_dex(name: &str) -> bool {
    if name == "classes.dex" {
        return true;
    }
    let Some(rest) = name
        .strip_prefix("classes")
        .and_then(|s| s.strip_suffix(".dex"))
    else {
        return false;
    };
    !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
}

fn is_android_api(class_name: &str) -> bool {
    class_name.starts_with("android.")
        || class_name.starts_with("dalvik.")
        || class_name.starts_with("java.")
        || class_name.starts_with("javax.")
        || class_name.starts_with("org.apache.http.")
        || class_name.starts_with("org.json.")
        || class_name.starts_with("org.w3c.dom.")
        || class_name.starts_with("org.xml.sax.")
}

fn is_native_lib(name: &str) -> bool {
    name.starts_with("lib/") && name.ends_with(".so")
}

fn collect_domains(strings: &[String], profile: &mut ApkProfile) {
    for value in strings {
        let uris = uris_from_string(value);
        if !uris.is_empty() {
            profile.uri_string_count += 1;
        }
        if uris.iter().any(|uri| is_http_url(uri)) {
            profile.url_count += 1;
        }
        for uri in uris {
            if let Some(scheme) = uri_scheme(&uri) {
                *profile.uri_schemes.entry(scheme.to_string()).or_insert(0) += 1;
            }
            if is_http_url(&uri) {
                profile.urls.insert(uri.clone());
            }
            if let Some(domain) = domain_from_uri(&uri) {
                profile.domains.insert(domain);
            }
            profile.uris.insert(uri);
        }
    }
}

fn uris_from_string(value: &str) -> Vec<String> {
    let mut uris = Vec::new();
    for scheme in uri_scheme_prefixes() {
        let mut rest = value;
        while let Some(idx) = rest.find(scheme) {
            let after = &rest[idx..];
            let url_end = after
                .find(|ch: char| {
                    ch.is_whitespace()
                        || matches!(ch, '"' | '\'' | '<' | '>' | '\\' | ')' | '(' | '[' | ']')
                })
                .unwrap_or(after.len());
            let uri = after[..url_end]
                .trim_end_matches(|ch: char| matches!(ch, '.' | ',' | ';' | ':' | '!' | '?'))
                .to_string();
            if looks_like_uri(&uri) {
                uris.push(uri);
            }
            rest = &after[url_end..];
        }
    }
    uris
}

fn uri_scheme_prefixes() -> &'static [&'static str] {
    &[
        "https://",
        "http://",
        "wss://",
        "ws://",
        "ftps://",
        "ftp://",
        "sftp://",
        "mqtts://",
        "mqtt://",
        "rtsp://",
        "rtmp://",
        "tcp://",
        "udp://",
        "mailto:",
        "tel:",
        "sms:",
        "geo:",
        "content://",
        "file://",
        "android-app://",
        "market://",
        "intent://",
    ]
}

fn domain_from_uri(uri: &str) -> Option<String> {
    for scheme in uri_scheme_prefixes()
        .iter()
        .copied()
        .filter(|scheme| network_host_scheme_prefix(scheme))
    {
        if let Some(after) = uri.strip_prefix(scheme) {
            let host_end = after
                .find(|ch: char| matches!(ch, '/' | ':' | '?' | '#' | '"' | '\'' | '<' | '>'))
                .unwrap_or(after.len());
            let host = after[..host_end].trim_matches('.');
            if looks_like_domain(host) {
                return Some(host.to_ascii_lowercase());
            }
        }
    }
    None
}

fn network_host_scheme_prefix(scheme_prefix: &str) -> bool {
    matches!(
        scheme_prefix,
        "https://"
            | "http://"
            | "wss://"
            | "ws://"
            | "ftps://"
            | "ftp://"
            | "sftp://"
            | "mqtts://"
            | "mqtt://"
            | "rtsp://"
            | "rtmp://"
            | "tcp://"
            | "udp://"
    )
}

fn looks_like_uri(uri: &str) -> bool {
    if uri.len() > 2048 {
        return false;
    }
    let Some(scheme) = uri_scheme(uri) else {
        return false;
    };
    if matches!(
        scheme,
        "mailto" | "tel" | "sms" | "geo" | "content" | "file" | "android-app" | "market" | "intent"
    ) {
        return uri.len() > scheme.len() + 1;
    }
    domain_from_uri(uri).is_some()
}

fn is_http_url(uri: &str) -> bool {
    uri.starts_with("https://") || uri.starts_with("http://")
}

fn uri_scheme(uri: &str) -> Option<&str> {
    let split = uri.find(':')?;
    let scheme = &uri[..split];
    if scheme.is_empty()
        || !scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
    {
        return None;
    }
    Some(scheme)
}

fn likely_endpoint_urls(profile: &ApkProfile) -> Vec<&String> {
    let mut urls = profile
        .urls
        .iter()
        .filter(|url| is_likely_endpoint_url(url))
        .collect::<Vec<_>>();
    urls.sort_by(|left, right| {
        endpoint_url_score(right)
            .cmp(&endpoint_url_score(left))
            .then_with(|| left.cmp(right))
    });
    urls
}

fn likely_endpoint_uris(profile: &ApkProfile) -> Vec<&String> {
    let mut uris = profile
        .uris
        .iter()
        .filter(|uri| is_likely_endpoint_uri(uri))
        .collect::<Vec<_>>();
    uris.sort_by(|left, right| {
        endpoint_uri_score(right)
            .cmp(&endpoint_uri_score(left))
            .then_with(|| left.cmp(right))
    });
    uris
}

fn is_likely_endpoint_url(url: &str) -> bool {
    if !is_http_url(url) {
        return false;
    }
    is_likely_endpoint_uri(url)
}

fn is_likely_endpoint_uri(uri: &str) -> bool {
    let Some(scheme) = uri_scheme(uri) else {
        return false;
    };
    if !matches!(
        scheme,
        "http"
            | "https"
            | "ws"
            | "wss"
            | "ftp"
            | "ftps"
            | "sftp"
            | "mqtt"
            | "mqtts"
            | "rtsp"
            | "rtmp"
            | "tcp"
            | "udp"
    ) {
        return false;
    }
    let Some(domain) = domain_from_uri(uri) else {
        return false;
    };
    let lower = uri.to_ascii_lowercase();
    !matches!(
        domain.as_str(),
        "schemas.android.com"
            | "ns.adobe.com"
            | "www.bouncycastle.org"
            | "www.ccil.org"
            | "www.w3.org"
            | "www.slf4j.org"
            | "xml.org"
            | "xmlpull.org"
            | "g.co"
            | "goo.gl"
            | "localhost"
    ) && !uri.starts_with("http://schemas.")
        && !uri.contains("/apk/res/")
        && !uri.contains("/xap/")
        && !uri.contains("/tagsoup/")
        && !lower.contains("/codes.html")
        && !lower.contains("/features/")
        && !lower.contains("/properties/")
        && !lower.contains("packagevisibility")
        && !lower.ends_with(".dtd")
        && !lower.ends_with(".xsd")
}

fn endpoint_url_score(url: &str) -> u64 {
    endpoint_uri_score(url)
}

fn endpoint_uri_score(uri: &str) -> u64 {
    let mut score = 0;
    let lower = uri.to_ascii_lowercase();
    if lower.starts_with("https://") || lower.starts_with("wss://") || lower.starts_with("mqtts://")
    {
        score += 100;
    }
    if lower.starts_with("ws://") || lower.starts_with("wss://") {
        score += 75;
    }
    if lower.starts_with("ftp://") || lower.starts_with("ftps://") || lower.starts_with("sftp://") {
        score += 55;
    }
    if lower.starts_with("mqtt://") || lower.starts_with("mqtts://") {
        score += 55;
    }
    if let Some(domain) = domain_from_uri(uri) {
        if domain.starts_with("api.") {
            score += 80;
        }
        if domain.contains(".api.") || domain.contains("api-") || domain.contains("-api") {
            score += 50;
        }
        if domain.starts_with("auth.") || domain.contains("login") || domain.contains("account") {
            score += 35;
        }
        if domain.contains("usercentrics")
            || domain.contains("onfido")
            || domain.contains("adjust")
            || domain.contains("app-measurement")
            || domain.contains("deliveryhero")
            || domain.contains("foodpanda")
            || domain.contains("firebase")
        {
            score += 25;
        }
    }
    if lower.contains("/api/") || lower.contains("graphql") || lower.contains("oauth") {
        score += 30;
    }
    score
}

fn looks_like_domain(host: &str) -> bool {
    if host.len() < 4 || !host.contains('.') || host.contains('/') {
        return false;
    }
    host.bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
        && host
            .rsplit('.')
            .next()
            .is_some_and(|tld| tld.len() >= 2 && tld.bytes().all(|b| b.is_ascii_alphabetic()))
}

fn read_zip_entries(data: &[u8]) -> io::Result<Vec<ZipEntry>> {
    let eocd = find_eocd(data)?;
    let total_entries = read_u16(data, eocd + 10)? as usize;
    let central_dir_size = read_u32(data, eocd + 12)? as usize;
    let central_dir_off = read_u32(data, eocd + 16)? as usize;
    checked_range(data, central_dir_off, central_dir_size)?;

    let mut pos = central_dir_off;
    let mut entries = Vec::with_capacity(total_entries);
    for _ in 0..total_entries {
        if read_u32(data, pos)? != 0x0201_4b50 {
            return Err(invalid("bad ZIP central directory header"));
        }
        let flags = read_u16(data, pos + 8)?;
        let compression = read_u16(data, pos + 10)?;
        let compressed_size = read_u32(data, pos + 20)? as usize;
        let uncompressed_size = read_u32(data, pos + 24)? as usize;
        let name_len = read_u16(data, pos + 28)? as usize;
        let extra_len = read_u16(data, pos + 30)? as usize;
        let comment_len = read_u16(data, pos + 32)? as usize;
        let local_header_offset = read_u32(data, pos + 42)? as usize;
        let name_start = pos + 46;
        let name = read_string(data, name_start, name_len)?;
        if flags & 0x0001 != 0 {
            return Err(invalid("encrypted APK entries are not supported"));
        }
        entries.push(ZipEntry {
            name,
            compression,
            compressed_size,
            uncompressed_size,
            local_header_offset,
        });
        pos = name_start
            .checked_add(name_len)
            .and_then(|p| p.checked_add(extra_len))
            .and_then(|p| p.checked_add(comment_len))
            .ok_or_else(|| invalid("ZIP central directory overflow"))?;
    }
    Ok(entries)
}

fn find_eocd(data: &[u8]) -> io::Result<usize> {
    let min = data.len().saturating_sub(22 + 65_535);
    for pos in (min..=data.len().saturating_sub(22)).rev() {
        if data.get(pos..pos + 4) == Some(&[0x50, 0x4b, 0x05, 0x06]) {
            return Ok(pos);
        }
    }
    Err(invalid("ZIP end of central directory not found"))
}

fn extract_zip_entry(data: &[u8], entry: &ZipEntry) -> io::Result<Vec<u8>> {
    let pos = entry.local_header_offset;
    if read_u32(data, pos)? != 0x0403_4b50 {
        return Err(invalid("bad ZIP local file header"));
    }
    let name_len = read_u16(data, pos + 26)? as usize;
    let extra_len = read_u16(data, pos + 28)? as usize;
    let data_start = pos
        .checked_add(30)
        .and_then(|p| p.checked_add(name_len))
        .and_then(|p| p.checked_add(extra_len))
        .ok_or_else(|| invalid("ZIP local file header overflow"))?;
    checked_range(data, data_start, entry.compressed_size)?;
    let compressed = &data[data_start..data_start + entry.compressed_size];
    match entry.compression {
        0 => {
            if compressed.len() != entry.uncompressed_size {
                return Err(invalid("stored ZIP entry size mismatch"));
            }
            Ok(compressed.to_vec())
        }
        8 => {
            let out = edgerun_encoding::compression::deflate_raw_decompress_with_limit(
                compressed,
                entry.uncompressed_size,
            )
            .map_err(|_| invalid("failed to inflate ZIP entry"))?;
            if out.len() != entry.uncompressed_size {
                return Err(invalid("inflated ZIP entry size mismatch"));
            }
            Ok(out)
        }
        method => Err(invalid(format!(
            "unsupported ZIP compression method {method} for {}",
            entry.name
        ))),
    }
}

fn extract_dex_calls(data: &[u8], profile: &mut ApkProfile) -> io::Result<BTreeMap<ApiCall, u64>> {
    let header = read_dex_header(data)?;
    let strings = read_string_ids(data, &header)?;
    let types = read_type_ids(data, &header, &strings)?;
    let protos = read_proto_ids(data, &header, &types)?;
    let methods = read_method_ids(data, &header, &strings, &types, &protos)?;
    let mut counts = BTreeMap::new();

    for class_idx in 0..header.class_defs_size {
        let class_def = header.class_defs_off + class_idx * 32;
        let class_type_idx = read_u32(data, class_def)? as usize;
        let caller_class = types.get(class_type_idx).cloned().unwrap_or_default();
        let caller_package = package_name(&caller_class);
        let class_data_off = read_u32(data, class_def + 24)? as usize;
        if class_data_off == 0 {
            continue;
        }
        for (defined_method_idx, code_off) in read_class_methods(data, class_data_off)? {
            record_class_method(profile, &caller_class, &caller_package);
            let defined_method_name = methods
                .get(defined_method_idx as usize)
                .map(|method| method.method_name.as_str())
                .unwrap_or("<unknown>");
            let code_scan = scan_code_item(data, code_off)?;
            record_method_string_constants(
                profile,
                &caller_class,
                defined_method_name,
                &strings,
                &code_scan.string_indices,
            );
            for method_idx in code_scan.method_indices {
                if let Some(method) = methods.get(method_idx as usize) {
                    let api = ApiCall {
                        class_name: method.class_name.clone(),
                        method_name: method.method_name.clone(),
                        args: method.args.clone(),
                        return_type: method.return_type.clone(),
                    };
                    record_signal_caller(profile, &caller_package, &api, 1);
                    record_class_call(profile, &caller_class, &caller_package, &api);
                    record_method_call(profile, &caller_class, defined_method_name, &api);
                    *counts.entry(api).or_insert(0) += 1;
                }
            }
        }
    }

    Ok(counts)
}

fn record_class_method(profile: &mut ApkProfile, caller_class: &str, caller_package: &str) {
    if caller_class.is_empty() {
        return;
    }
    let summary = profile
        .class_summaries
        .entry(caller_class.to_string())
        .or_insert_with(|| ClassSummary {
            package_name: caller_package.to_string(),
            ..ClassSummary::default()
        });
    summary.method_count += 1;
}

fn record_class_call(
    profile: &mut ApkProfile,
    caller_class: &str,
    caller_package: &str,
    api: &ApiCall,
) {
    if caller_class.is_empty() {
        return;
    }
    let callee_package = package_name(&api.class_name);
    let is_platform = is_android_api(&api.class_name);
    let summary = profile
        .class_summaries
        .entry(caller_class.to_string())
        .or_insert_with(|| ClassSummary {
            package_name: caller_package.to_string(),
            ..ClassSummary::default()
        });
    if is_platform {
        summary.platform_calls += 1;
        for signal in signals() {
            if matches_signal(api, signal) {
                *summary.signals.entry(signal.name).or_insert(0) += 1;
            }
        }
    } else {
        summary.internal_calls += 1;
        if !callee_package.is_empty() {
            *summary.callee_packages.entry(callee_package).or_insert(0) += 1;
        }
        if profile.dex_classes.contains(&api.class_name) {
            *summary
                .callee_classes
                .entry(api.class_name.clone())
                .or_insert(0) += 1;
        }
    }
}

fn record_method_call(
    profile: &mut ApkProfile,
    class_name: &str,
    method_name: &str,
    api: &ApiCall,
) {
    if class_name.is_empty() {
        return;
    }
    let callee_package = package_name(&api.class_name);
    let is_platform = is_android_api(&api.class_name);
    let dex_classes = &profile.dex_classes;
    let summary = method_summary_mut(&mut profile.method_summaries, class_name, method_name);
    if is_platform {
        summary.platform_calls += 1;
        for signal in signals() {
            if matches_signal(api, signal) {
                *summary.signals.entry(signal.name).or_insert(0) += 1;
            }
        }
    } else {
        summary.internal_calls += 1;
        if !callee_package.is_empty() {
            *summary.callee_packages.entry(callee_package).or_insert(0) += 1;
        }
        if dex_classes.contains(&api.class_name) {
            *summary
                .callee_classes
                .entry(api.class_name.clone())
                .or_insert(0) += 1;
        }
    }
}

fn record_method_string_constants(
    profile: &mut ApkProfile,
    class_name: &str,
    method_name: &str,
    strings: &[String],
    string_indices: &[u32],
) {
    if class_name.is_empty() {
        return;
    }
    let summary = method_summary_mut(&mut profile.method_summaries, class_name, method_name);
    for string_idx in string_indices {
        let Some(value) = strings.get(*string_idx as usize) else {
            continue;
        };
        if !is_interesting_method_constant(value) {
            continue;
        }
        *summary.string_constants.entry(value.clone()).or_insert(0) += 1;
        for uri in uris_from_string(value) {
            *summary.uri_constants.entry(uri).or_insert(0) += 1;
        }
    }
}

fn method_summary_mut<'a>(
    method_summaries: &'a mut BTreeMap<String, MethodSummary>,
    class_name: &str,
    method_name: &str,
) -> &'a mut MethodSummary {
    let key = method_key_for_class(class_name, method_name);
    method_summaries
        .entry(key)
        .or_insert_with(|| MethodSummary {
            class_name: class_name.to_string(),
            method_name: method_name.to_string(),
            ..MethodSummary::default()
        })
}

fn is_interesting_method_constant(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.len() < 3 || trimmed.len() > 300 {
        return false;
    }
    if !uris_from_string(trimmed).is_empty() {
        return true;
    }
    trimmed.chars().any(|ch| ch.is_ascii_alphabetic())
        && !trimmed.starts_with('L')
        && !trimmed.ends_with(';')
        && !trimmed.starts_with('[')
}

fn method_key_for_class(class_name: &str, method_name: &str) -> String {
    format!("{class_name}->{method_name}")
}

fn record_signal_caller(profile: &mut ApkProfile, caller_package: &str, api: &ApiCall, count: u64) {
    if caller_package.is_empty() {
        return;
    }
    for signal in signals() {
        if matches_signal(api, signal) {
            *profile
                .signal_callers
                .entry((caller_package.to_string(), signal.name))
                .or_insert(0) += count;
        }
    }
}

fn extract_dex_strings(data: &[u8]) -> io::Result<Vec<String>> {
    let header = read_dex_header(data)?;
    read_string_ids(data, &header)
}

fn is_resource_entry(name: &str) -> bool {
    name.starts_with("res/") || name.starts_with("assets/")
}

fn collect_resource_entry(apk: &[u8], entry: &ZipEntry, profile: &mut ApkProfile) {
    if entry.name.starts_with("assets/") {
        if !entry.name.ends_with('/') {
            profile.asset_files.insert(entry.name.clone());
        }
        return;
    }

    if let Some(category) = resource_category(&entry.name) {
        *profile
            .resource_files
            .entry(category.to_string())
            .or_insert(0) += 1;
    }

    if is_binary_xml_resource(&entry.name) {
        let Ok(data) = extract_zip_entry(apk, entry) else {
            return;
        };
        let _ = collect_binary_xml_hints(&data, profile);
    }
}

fn resource_category(name: &str) -> Option<&str> {
    let rest = name.strip_prefix("res/")?;
    if !rest.contains('/') {
        return None;
    }
    rest.split('/').next()
}

fn is_binary_xml_resource(name: &str) -> bool {
    if !name.ends_with(".xml") {
        return false;
    }
    name.starts_with("res/layout")
        || name.starts_with("res/menu")
        || name.starts_with("res/navigation")
        || name.starts_with("res/xml")
}

fn collect_binary_xml_hints(data: &[u8], profile: &mut ApkProfile) -> io::Result<()> {
    if data.len() < 8 || read_u16(data, 0)? != 0x0003 {
        return Ok(());
    }

    let mut strings = Vec::new();
    let mut pos = 8usize;
    while pos + 8 <= data.len() {
        let chunk_type = read_u16(data, pos)?;
        let header_size = read_u16(data, pos + 2)? as usize;
        let chunk_size = read_u32(data, pos + 4)? as usize;
        if chunk_size < header_size || chunk_size == 0 {
            return Err(invalid("bad binary XML chunk size"));
        }

        match chunk_type {
            0x0001 => strings = parse_string_pool(data, pos)?,
            0x0102 => {
                let elem = parse_start_element(data, pos, &strings)?;
                *profile.ui_elements.entry(elem.name.clone()).or_insert(0) += 1;
                for (name, value) in elem.attrs {
                    if is_user_visible_xml_attr(&name) && is_literal_ui_text(&value) {
                        profile.ui_texts.insert(value.trim().to_string());
                    }
                }
            }
            _ => {}
        }

        pos = pos
            .checked_add(chunk_size)
            .ok_or_else(|| invalid("binary XML chunk overflow"))?;
    }
    Ok(())
}

fn is_user_visible_xml_attr(name: &str) -> bool {
    matches!(
        name,
        "text"
            | "android:text"
            | "hint"
            | "android:hint"
            | "label"
            | "android:label"
            | "title"
            | "android:title"
            | "contentDescription"
            | "android:contentDescription"
    )
}

fn is_literal_ui_text(value: &str) -> bool {
    let trimmed = value.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with('@')
        && !trimmed.starts_with('?')
        && !trimmed.starts_with("0x")
        && !trimmed.contains('.')
        && !trimmed.ends_with("Activity")
        && !trimmed.ends_with("Fragment")
        && !trimmed.ends_with("Service")
        && !trimmed.ends_with("Receiver")
        && trimmed.len() <= 120
        && trimmed.chars().any(|ch| ch.is_ascii_alphabetic())
}

fn collect_dex_packages(data: &[u8], profile: &mut ApkProfile) -> io::Result<()> {
    let header = read_dex_header(data)?;
    let strings = read_string_ids(data, &header)?;
    let types = read_type_ids(data, &header, &strings)?;
    checked_range(data, header.class_defs_off, header.class_defs_size * 32)?;
    for class_idx in 0..header.class_defs_size {
        let class_def = header.class_defs_off + class_idx * 32;
        let class_type_idx = read_u32(data, class_def)? as usize;
        if let Some(class_name) = types.get(class_type_idx) {
            profile.dex_classes.insert(class_name.clone());
            let package = package_name(class_name);
            if !package.is_empty() {
                *profile.dex_packages.entry(package).or_insert(0) += 1;
            }
        }
    }
    Ok(())
}

fn parse_android_manifest(data: &[u8]) -> io::Result<ManifestProfile> {
    if data.len() < 8 || read_u16(data, 0)? != 0x0003 {
        return Err(invalid("AndroidManifest.xml is not binary XML"));
    }

    let mut strings = Vec::new();
    let mut manifest = ManifestProfile::default();
    let mut pos = 8usize;
    let mut current_component: Option<(String, ComponentProfile)> = None;
    let mut in_intent_filter = false;

    while pos + 8 <= data.len() {
        let chunk_type = read_u16(data, pos)?;
        let header_size = read_u16(data, pos + 2)? as usize;
        let chunk_size = read_u32(data, pos + 4)? as usize;
        if chunk_size < header_size || chunk_size == 0 {
            return Err(invalid("bad binary XML chunk size"));
        }

        match chunk_type {
            0x0001 => {
                strings = parse_string_pool(data, pos)?;
            }
            0x0102 => {
                let elem = parse_start_element(data, pos, &strings)?;
                match elem.name.as_str() {
                    "manifest" => {
                        manifest.package_name = elem.attr("package").map(str::to_string);
                    }
                    "uses-permission" | "uses-permission-sdk-23" => {
                        if let Some(name) = elem.attr("name").or_else(|| elem.attr("android:name"))
                        {
                            manifest.permissions.insert(name.to_string());
                        }
                    }
                    "activity" | "activity-alias" | "service" | "receiver" | "provider" => {
                        if let Some((kind, component)) = current_component.take() {
                            push_component(&mut manifest, &kind, component);
                        }
                        let name = elem
                            .attr("name")
                            .or_else(|| elem.attr("android:name"))
                            .unwrap_or("<unnamed>")
                            .to_string();
                        let exported = elem
                            .attr("exported")
                            .or_else(|| elem.attr("android:exported"))
                            .and_then(parse_bool);
                        current_component = Some((
                            elem.name.clone(),
                            ComponentProfile {
                                name,
                                exported,
                                ..ComponentProfile::default()
                            },
                        ));
                    }
                    "intent-filter" => in_intent_filter = true,
                    "action" if in_intent_filter => {
                        if let Some((_, component)) = current_component.as_mut() {
                            if let Some(name) =
                                elem.attr("name").or_else(|| elem.attr("android:name"))
                            {
                                component.actions.insert(name.to_string());
                            }
                        }
                    }
                    "category" if in_intent_filter => {
                        if let Some((_, component)) = current_component.as_mut() {
                            if let Some(name) =
                                elem.attr("name").or_else(|| elem.attr("android:name"))
                            {
                                component.categories.insert(name.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }
            0x0103 => {
                let name_idx = read_u32(data, pos + 20)? as usize;
                let name = strings.get(name_idx).map(String::as_str).unwrap_or("");
                match name {
                    "intent-filter" => in_intent_filter = false,
                    "activity" | "activity-alias" | "service" | "receiver" | "provider" => {
                        if let Some((kind, component)) = current_component.take() {
                            push_component(&mut manifest, &kind, component);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }

        pos = pos
            .checked_add(chunk_size)
            .ok_or_else(|| invalid("binary XML chunk overflow"))?;
    }

    if let Some((kind, component)) = current_component.take() {
        push_component(&mut manifest, &kind, component);
    }

    Ok(manifest)
}

#[derive(Clone, Debug)]
struct XmlElement {
    name: String,
    attrs: Vec<(String, String)>,
}

impl XmlElement {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

fn parse_string_pool(data: &[u8], off: usize) -> io::Result<Vec<String>> {
    let header_size = read_u16(data, off + 2)? as usize;
    let chunk_size = read_u32(data, off + 4)? as usize;
    let string_count = read_u32(data, off + 8)? as usize;
    let flags = read_u32(data, off + 16)?;
    let strings_start = read_u32(data, off + 20)? as usize;
    let is_utf8 = flags & 0x0000_0100 != 0;
    checked_range(data, off, chunk_size)?;
    checked_range(data, off + header_size, string_count * 4)?;

    let mut strings = Vec::with_capacity(string_count);
    for idx in 0..string_count {
        let string_off = read_u32(data, off + header_size + idx * 4)? as usize;
        let pos = off
            .checked_add(strings_start)
            .and_then(|p| p.checked_add(string_off))
            .ok_or_else(|| invalid("string pool offset overflow"))?;
        strings.push(if is_utf8 {
            read_utf8_pool_string(data, pos)?
        } else {
            read_utf16_pool_string(data, pos)?
        });
    }
    Ok(strings)
}

fn parse_start_element(data: &[u8], off: usize, strings: &[String]) -> io::Result<XmlElement> {
    let name_idx = read_u32(data, off + 20)? as usize;
    let attr_start = read_u16(data, off + 24)? as usize;
    let attr_size = read_u16(data, off + 26)? as usize;
    let attr_count = read_u16(data, off + 28)? as usize;
    let attrs_off = off
        .checked_add(16)
        .and_then(|p| p.checked_add(attr_start))
        .ok_or_else(|| invalid("XML attribute offset overflow"))?;
    checked_range(data, attrs_off, attr_count * attr_size)?;

    let mut attrs = Vec::new();
    for idx in 0..attr_count {
        let attr_off = attrs_off + idx * attr_size;
        let ns_idx = read_u32(data, attr_off)? as usize;
        let attr_name_idx = read_u32(data, attr_off + 4)? as usize;
        let raw_value_idx = read_u32(data, attr_off + 8)?;
        let value_type = *data
            .get(attr_off + 15)
            .ok_or_else(|| invalid("XML attribute type out of range"))?;
        let value_data = read_u32(data, attr_off + 16)?;

        let mut name = strings
            .get(attr_name_idx)
            .cloned()
            .unwrap_or_else(|| format!("#{attr_name_idx}"));
        if let Some(ns) = strings.get(ns_idx) {
            if ns == "http://schemas.android.com/apk/res/android" {
                name = format!("android:{name}");
            }
        }

        let value = if raw_value_idx != u32::MAX {
            strings
                .get(raw_value_idx as usize)
                .cloned()
                .unwrap_or_default()
        } else {
            format_typed_xml_value(value_type, value_data, strings)
        };
        attrs.push((name, value));
    }

    Ok(XmlElement {
        name: strings
            .get(name_idx)
            .cloned()
            .unwrap_or_else(|| format!("#{name_idx}")),
        attrs,
    })
}

fn read_utf8_pool_string(data: &[u8], mut pos: usize) -> io::Result<String> {
    let _utf16_len = read_pool_len8(data, &mut pos)?;
    let byte_len = read_pool_len8(data, &mut pos)?;
    checked_range(data, pos, byte_len + 1)?;
    Ok(String::from_utf8_lossy(&data[pos..pos + byte_len]).into_owned())
}

fn read_utf16_pool_string(data: &[u8], mut pos: usize) -> io::Result<String> {
    let char_len = read_pool_len16(data, &mut pos)?;
    checked_range(data, pos, char_len * 2 + 2)?;
    let mut units = Vec::with_capacity(char_len);
    for idx in 0..char_len {
        units.push(read_u16(data, pos + idx * 2)?);
    }
    Ok(String::from_utf16_lossy(&units))
}

fn read_pool_len8(data: &[u8], pos: &mut usize) -> io::Result<usize> {
    let first = *data
        .get(*pos)
        .ok_or_else(|| invalid("string pool length out of range"))?;
    *pos += 1;
    if first & 0x80 == 0 {
        Ok(first as usize)
    } else {
        let second = *data
            .get(*pos)
            .ok_or_else(|| invalid("string pool length out of range"))?;
        *pos += 1;
        Ok((((first & 0x7f) as usize) << 8) | second as usize)
    }
}

fn read_pool_len16(data: &[u8], pos: &mut usize) -> io::Result<usize> {
    let first = read_u16(data, *pos)?;
    *pos += 2;
    if first & 0x8000 == 0 {
        Ok(first as usize)
    } else {
        let second = read_u16(data, *pos)?;
        *pos += 2;
        Ok((((first & 0x7fff) as usize) << 16) | second as usize)
    }
}

fn format_typed_xml_value(value_type: u8, value_data: u32, strings: &[String]) -> String {
    match value_type {
        0x03 => strings
            .get(value_data as usize)
            .cloned()
            .unwrap_or_default(),
        0x10 | 0x11 => value_data.to_string(),
        0x12 => (value_data != 0).to_string(),
        0x01 => format!("@0x{value_data:08x}"),
        _ => format!("0x{value_data:08x}"),
    }
}

fn push_component(manifest: &mut ManifestProfile, kind: &str, component: ComponentProfile) {
    match kind {
        "activity" | "activity-alias" => manifest.activities.push(component),
        "service" => manifest.services.push(component),
        "receiver" => manifest.receivers.push(component),
        "provider" => manifest.providers.push(component),
        _ => {}
    }
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn read_dex_header(data: &[u8]) -> io::Result<DexHeader> {
    if data.len() < 112 || data.get(0..4) != Some(b"dex\n") {
        return Err(invalid("not a DEX file"));
    }
    Ok(DexHeader {
        string_ids_size: read_u32(data, 56)? as usize,
        string_ids_off: read_u32(data, 60)? as usize,
        type_ids_size: read_u32(data, 64)? as usize,
        type_ids_off: read_u32(data, 68)? as usize,
        proto_ids_size: read_u32(data, 72)? as usize,
        proto_ids_off: read_u32(data, 76)? as usize,
        method_ids_size: read_u32(data, 88)? as usize,
        method_ids_off: read_u32(data, 92)? as usize,
        class_defs_size: read_u32(data, 96)? as usize,
        class_defs_off: read_u32(data, 100)? as usize,
    })
}

fn read_string_ids(data: &[u8], header: &DexHeader) -> io::Result<Vec<String>> {
    checked_range(data, header.string_ids_off, header.string_ids_size * 4)?;
    let mut strings = Vec::with_capacity(header.string_ids_size);
    for idx in 0..header.string_ids_size {
        let off = read_u32(data, header.string_ids_off + idx * 4)? as usize;
        strings.push(read_dex_string(data, off)?);
    }
    Ok(strings)
}

fn read_type_ids(data: &[u8], header: &DexHeader, strings: &[String]) -> io::Result<Vec<String>> {
    checked_range(data, header.type_ids_off, header.type_ids_size * 4)?;
    let mut types = Vec::with_capacity(header.type_ids_size);
    for idx in 0..header.type_ids_size {
        let string_idx = read_u32(data, header.type_ids_off + idx * 4)? as usize;
        let descriptor = strings
            .get(string_idx)
            .ok_or_else(|| invalid("DEX type string index out of range"))?;
        types.push(format_type(descriptor));
    }
    Ok(types)
}

fn read_proto_ids(
    data: &[u8],
    header: &DexHeader,
    types: &[String],
) -> io::Result<Vec<(String, Vec<String>)>> {
    checked_range(data, header.proto_ids_off, header.proto_ids_size * 12)?;
    let mut protos = Vec::with_capacity(header.proto_ids_size);
    for idx in 0..header.proto_ids_size {
        let pos = header.proto_ids_off + idx * 12;
        let return_type_idx = read_u32(data, pos + 4)? as usize;
        let parameters_off = read_u32(data, pos + 8)? as usize;
        let return_type = types
            .get(return_type_idx)
            .ok_or_else(|| invalid("DEX proto return type index out of range"))?
            .clone();
        let args = if parameters_off == 0 {
            Vec::new()
        } else {
            read_type_list(data, parameters_off, types)?
        };
        protos.push((return_type, args));
    }
    Ok(protos)
}

fn read_method_ids(
    data: &[u8],
    header: &DexHeader,
    strings: &[String],
    types: &[String],
    protos: &[(String, Vec<String>)],
) -> io::Result<Vec<MethodRef>> {
    checked_range(data, header.method_ids_off, header.method_ids_size * 8)?;
    let mut methods = Vec::with_capacity(header.method_ids_size);
    for idx in 0..header.method_ids_size {
        let pos = header.method_ids_off + idx * 8;
        let class_idx = read_u16(data, pos)? as usize;
        let proto_idx = read_u16(data, pos + 2)? as usize;
        let name_idx = read_u32(data, pos + 4)? as usize;
        let class_name = types
            .get(class_idx)
            .ok_or_else(|| invalid("DEX method class index out of range"))?
            .clone();
        let (return_type, args) = protos
            .get(proto_idx)
            .ok_or_else(|| invalid("DEX method proto index out of range"))?;
        let method_name = strings
            .get(name_idx)
            .ok_or_else(|| invalid("DEX method name index out of range"))?
            .clone();
        methods.push(MethodRef {
            class_name,
            method_name,
            args: args.clone(),
            return_type: return_type.clone(),
        });
    }
    Ok(methods)
}

fn read_type_list(data: &[u8], off: usize, types: &[String]) -> io::Result<Vec<String>> {
    let size = read_u32(data, off)? as usize;
    checked_range(data, off + 4, size * 2)?;
    let mut out = Vec::with_capacity(size);
    for idx in 0..size {
        let type_idx = read_u16(data, off + 4 + idx * 2)? as usize;
        out.push(
            types
                .get(type_idx)
                .ok_or_else(|| invalid("DEX type list index out of range"))?
                .clone(),
        );
    }
    Ok(out)
}

fn read_class_methods(data: &[u8], class_data_off: usize) -> io::Result<Vec<(u32, usize)>> {
    let mut pos = class_data_off;
    let static_fields = read_uleb128(data, &mut pos)?;
    let instance_fields = read_uleb128(data, &mut pos)?;
    let direct_methods = read_uleb128(data, &mut pos)?;
    let virtual_methods = read_uleb128(data, &mut pos)?;

    for _ in 0..static_fields + instance_fields {
        let _field_idx_diff = read_uleb128(data, &mut pos)?;
        let _access_flags = read_uleb128(data, &mut pos)?;
    }

    let mut methods = Vec::new();
    let mut method_idx = 0u32;
    for _ in 0..direct_methods {
        method_idx = method_idx
            .checked_add(read_uleb128(data, &mut pos)?)
            .ok_or_else(|| invalid("DEX method index overflow"))?;
        let _access_flags = read_uleb128(data, &mut pos)?;
        let code_off = read_uleb128(data, &mut pos)? as usize;
        if code_off != 0 {
            methods.push((method_idx, code_off));
        }
    }
    method_idx = 0;
    for _ in 0..virtual_methods {
        method_idx = method_idx
            .checked_add(read_uleb128(data, &mut pos)?)
            .ok_or_else(|| invalid("DEX method index overflow"))?;
        let _access_flags = read_uleb128(data, &mut pos)?;
        let code_off = read_uleb128(data, &mut pos)? as usize;
        if code_off != 0 {
            methods.push((method_idx, code_off));
        }
    }
    Ok(methods)
}

fn scan_code_item(data: &[u8], code_off: usize) -> io::Result<CodeScan> {
    checked_range(data, code_off, 16)?;
    let insns_size = read_u32(data, code_off + 12)? as usize;
    let insns_off = code_off + 16;
    checked_range(data, insns_off, insns_size * 2)?;
    let mut cursor = 0usize;
    let mut scan = CodeScan::default();
    while cursor < insns_size {
        let unit = read_u16(data, insns_off + cursor * 2)?;
        let opcode = (unit & 0x00ff) as u8;
        if matches!(opcode, 0x6e..=0x72 | 0x74..=0x78 | 0xfa | 0xfb) {
            if cursor + 1 < insns_size {
                scan.method_indices
                    .push(read_u16(data, insns_off + (cursor + 1) * 2)? as u32);
            }
        } else if opcode == 0x1a {
            if cursor + 1 < insns_size {
                scan.string_indices
                    .push(read_u16(data, insns_off + (cursor + 1) * 2)? as u32);
            }
        } else if opcode == 0x1b && cursor + 2 < insns_size {
            let low = read_u16(data, insns_off + (cursor + 1) * 2)? as u32;
            let high = read_u16(data, insns_off + (cursor + 2) * 2)? as u32;
            scan.string_indices.push(low | (high << 16));
        }
        let Ok(width) = instruction_width(data, insns_off, insns_size, cursor) else {
            break;
        };
        cursor += width;
    }
    Ok(scan)
}

fn instruction_width(
    data: &[u8],
    insns_off: usize,
    insns_size: usize,
    cursor: usize,
) -> io::Result<usize> {
    let unit = read_u16(data, insns_off + cursor * 2)?;
    let opcode = (unit & 0x00ff) as u8;
    let high = (unit >> 8) as u8;
    let width = match opcode {
        0x00 if high == 0x01 => {
            let size = payload_u16(data, insns_off, insns_size, cursor + 1)? as usize;
            4 + size * 2
        }
        0x00 if high == 0x02 => {
            let size = payload_u16(data, insns_off, insns_size, cursor + 1)? as usize;
            2 + size * 4
        }
        0x00 if high == 0x03 => {
            let elem_width = payload_u16(data, insns_off, insns_size, cursor + 1)? as usize;
            let size = payload_u32(data, insns_off, insns_size, cursor + 2)? as usize;
            4 + (elem_width * size).div_ceil(2)
        }
        0x00 => 1,
        0x01..=0x0f => 1,
        0x10..=0x12 => 1,
        0x13 => 2,
        0x14 => 3,
        0x15..=0x16 => 2,
        0x17 => 3,
        0x18 => 5,
        0x19..=0x1a => 2,
        0x1b => 3,
        0x1c => 2,
        0x1d..=0x1e => 1,
        0x1f..=0x20 => 2,
        0x21 => 1,
        0x22..=0x23 => 2,
        0x24..=0x26 => 3,
        0x27 => 1,
        0x28 => 1,
        0x29 => 2,
        0x2a => 3,
        0x2b..=0x2c => 3,
        0x2d..=0x31 => 2,
        0x32..=0x37 => 2,
        0x38..=0x3d => 2,
        0x44..=0x51 => 2,
        0x52..=0x5f => 2,
        0x60..=0x6d => 2,
        0x6e..=0x72 => 3,
        0x74..=0x78 => 3,
        0x7b..=0x8f => 1,
        0x90..=0xaf => 2,
        0xb0..=0xcf => 1,
        0xd0..=0xd7 => 2,
        0xd8..=0xe2 => 2,
        0xe3..=0xf9 => 1,
        0xfa..=0xfb => 4,
        0xfc..=0xfd => 3,
        0xfe..=0xff => 2,
        _ => 1,
    };
    if width == 0 || cursor + width > insns_size {
        return Err(invalid("DEX instruction exceeds code item"));
    }
    Ok(width)
}

fn payload_u16(data: &[u8], insns_off: usize, insns_size: usize, cursor: usize) -> io::Result<u16> {
    if cursor >= insns_size {
        return Err(invalid("DEX payload exceeds code item"));
    }
    read_u16(data, insns_off + cursor * 2)
}

fn payload_u32(data: &[u8], insns_off: usize, insns_size: usize, cursor: usize) -> io::Result<u32> {
    if cursor + 1 >= insns_size {
        return Err(invalid("DEX payload exceeds code item"));
    }
    read_u32(data, insns_off + cursor * 2)
}

fn read_dex_string(data: &[u8], off: usize) -> io::Result<String> {
    let mut pos = off;
    let _utf16_size = read_uleb128(data, &mut pos)?;
    let start = pos;
    while pos < data.len() && data[pos] != 0 {
        pos += 1;
    }
    if pos >= data.len() {
        return Err(invalid("unterminated DEX string"));
    }
    Ok(String::from_utf8_lossy(&data[start..pos]).into_owned())
}

fn format_type(descriptor: &str) -> String {
    let (ty, rest) = parse_type_descriptor(descriptor);
    if rest.is_empty() {
        ty
    } else {
        descriptor.to_string()
    }
}

fn parse_type_descriptor(input: &str) -> (String, &str) {
    let Some(first) = input.as_bytes().first().copied() else {
        return (String::new(), input);
    };
    match first {
        b'V' => ("void".to_string(), &input[1..]),
        b'Z' => ("boolean".to_string(), &input[1..]),
        b'B' => ("byte".to_string(), &input[1..]),
        b'S' => ("short".to_string(), &input[1..]),
        b'C' => ("char".to_string(), &input[1..]),
        b'I' => ("int".to_string(), &input[1..]),
        b'J' => ("long".to_string(), &input[1..]),
        b'F' => ("float".to_string(), &input[1..]),
        b'D' => ("double".to_string(), &input[1..]),
        b'L' => {
            let Some(end) = input.find(';') else {
                return (input.to_string(), "");
            };
            (input[1..end].replace('/', "."), &input[end + 1..])
        }
        b'[' => {
            let (inner, rest) = parse_type_descriptor(&input[1..]);
            (format!("{inner}[]"), rest)
        }
        _ => (input.to_string(), ""),
    }
}

fn read_uleb128(data: &[u8], pos: &mut usize) -> io::Result<u32> {
    let mut result = 0u32;
    for shift in (0..35).step_by(7) {
        let byte = *data
            .get(*pos)
            .ok_or_else(|| invalid("ULEB128 exceeds input"))?;
        *pos += 1;
        result |= ((byte & 0x7f) as u32) << shift;
        if byte & 0x80 == 0 {
            return Ok(result);
        }
    }
    Err(invalid("ULEB128 is too large"))
}

fn print_json(counts: &BTreeMap<ApiCall, u64>) {
    println!("[");
    for (idx, (api, count)) in counts.iter().enumerate() {
        let comma = if idx + 1 == counts.len() { "" } else { "," };
        println!(
            "  {{\"count\":{},\"class\":\"{}\",\"method\":\"{}\",\"args\":[{}],\"returns\":\"{}\"}}{}",
            count,
            json_escape(&api.class_name),
            json_escape(&api.method_name),
            api.args
                .iter()
                .map(|arg| format!("\"{}\"", json_escape(arg)))
                .collect::<Vec<_>>()
                .join(","),
            json_escape(&api.return_type),
            comma
        );
    }
    println!("]");
}

fn print_summary(counts: &BTreeMap<ApiCall, u64>, top: usize) {
    let mut total_call_sites = 0u64;
    let mut classes = BTreeSet::new();
    let mut packages: BTreeMap<String, SummaryCount> = BTreeMap::new();
    let mut class_counts: BTreeMap<String, SummaryCount> = BTreeMap::new();
    let mut method_counts: BTreeMap<String, SummaryCount> = BTreeMap::new();
    let mut interesting_packages: BTreeMap<String, SummaryCount> = BTreeMap::new();
    let mut interesting_class_counts: BTreeMap<String, SummaryCount> = BTreeMap::new();
    let mut interesting_method_counts: BTreeMap<String, SummaryCount> = BTreeMap::new();
    let mut signal_counts: BTreeMap<&'static str, SummaryCount> = BTreeMap::new();

    for (api, count) in counts {
        total_call_sites += count;
        classes.insert(api.class_name.as_str());

        add_summary_count(&mut packages, package_name(&api.class_name), *count);
        add_summary_count(&mut class_counts, api.class_name.clone(), *count);
        add_summary_count(&mut method_counts, method_key(api), *count);

        if is_interesting_api(api) {
            add_summary_count(
                &mut interesting_packages,
                package_name(&api.class_name),
                *count,
            );
            add_summary_count(
                &mut interesting_class_counts,
                api.class_name.clone(),
                *count,
            );
            add_summary_count(&mut interesting_method_counts, method_key(api), *count);
        }

        for signal in signals() {
            if matches_signal(api, signal) {
                add_summary_count_by_ref(&mut signal_counts, signal.name, *count);
            }
        }
    }

    println!("summary");
    println!("  unique_signatures: {}", counts.len());
    println!("  total_call_sites: {total_call_sites}");
    println!("  unique_classes: {}", classes.len());
    println!();

    print_ranked("capability_signals", &signal_counts, top);
    print_ranked("top_interesting_packages", &interesting_packages, top);
    print_ranked("top_interesting_classes", &interesting_class_counts, top);
    print_ranked("top_interesting_methods", &interesting_method_counts, top);
    print_ranked("top_packages", &packages, top);
    print_ranked("top_classes", &class_counts, top);
    print_ranked("top_methods", &method_counts, top);
}

fn print_profile(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>, top: usize) {
    println!("profile");
    println!(
        "  package: {}",
        profile
            .manifest
            .package_name
            .as_deref()
            .unwrap_or("<unknown>")
    );
    println!("  unique_signatures: {}", counts.len());
    println!("  total_call_sites: {}", counts.values().sum::<u64>());
    println!();

    println!("permissions");
    for permission in profile.manifest.permissions.iter().take(top) {
        println!("  {permission}");
    }
    if profile.manifest.permissions.len() > top {
        println!("  ... {} more", profile.manifest.permissions.len() - top);
    }
    println!();

    print_components("activities", &profile.manifest.activities, top);
    print_components("services", &profile.manifest.services, top);
    print_components("receivers", &profile.manifest.receivers, top);
    print_components("providers", &profile.manifest.providers, top);

    println!("native_libs");
    println!("  count: {}", profile.native_libs.len());
    for lib in profile.native_libs.iter().take(top) {
        println!("  {lib}");
    }
    if profile.native_libs.len() > top {
        println!("  ... {} more", profile.native_libs.len() - top);
    }
    println!();

    println!("network_indicators");
    let endpoint_urls = likely_endpoint_urls(profile);
    let endpoint_uris = likely_endpoint_uris(profile);
    println!("  uri_strings: {}", profile.uri_string_count);
    println!("  uris: {}", profile.uris.len());
    println!("  endpoint_uris: {}", endpoint_uris.len());
    for uri in endpoint_uris.iter().take(top) {
        println!("  endpoint_uri: {uri}");
    }
    if endpoint_uris.len() > top {
        println!("  ... {} more endpoint uris", endpoint_uris.len() - top);
    }
    println!("  uri_schemes: {}", profile.uri_schemes.len());
    for (scheme, count) in ranked_u64(&profile.uri_schemes).into_iter().take(top) {
        println!("  scheme: {scheme} ({count})");
    }
    println!("  url_strings: {}", profile.url_count);
    println!("  endpoint_urls: {}", endpoint_urls.len());
    for url in endpoint_urls.iter().take(top) {
        println!("  endpoint_url: {url}");
    }
    if endpoint_urls.len() > top {
        println!("  ... {} more endpoint urls", endpoint_urls.len() - top);
    }
    println!("  urls: {}", profile.urls.len());
    for url in profile.urls.iter().take(top) {
        println!("  url: {url}");
    }
    if profile.urls.len() > top {
        println!("  ... {} more urls", profile.urls.len() - top);
    }
    println!("  domains: {}", profile.domains.len());
    for domain in profile.domains.iter().take(top) {
        println!("  domain: {domain}");
    }
    if profile.domains.len() > top {
        println!("  ... {} more", profile.domains.len() - top);
    }
    println!();

    print_summary(counts, top);
    print_capability_candidates(profile, counts, top);
    print_likely_behaviors(profile, counts);
}

fn print_edgerun_app_spec(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>, top: usize) {
    let package = profile
        .manifest
        .package_name
        .as_deref()
        .unwrap_or("<unknown>");
    let app_prefixes = likely_app_prefixes(profile);
    let sdks = detected_sdks(profile);
    let workflows = infer_workflows(profile, counts);

    println!("edgerun_app_spec");
    println!("  identity");
    println!("    android_package: {package}");
    println!("    replacement_id: {}", package.replace('.', "-"));
    println!("    confidence: {}", spec_confidence(profile, counts));
    println!();

    println!("  code_ownership");
    println!("    likely_app_prefixes");
    for prefix in app_prefixes.iter().take(top) {
        let count = profile.dex_packages.get(prefix).copied().unwrap_or(0);
        println!("      {prefix}: {count} classes");
    }
    println!("    detected_sdks");
    for sdk in sdks.iter().take(top) {
        println!("      {sdk}");
    }
    if sdks.is_empty() {
        println!("      <none detected>");
    }
    println!("    third_party_module_policy");
    for sdk in sdks.iter().take(top) {
        println!("      {sdk}: {}", third_party_policy(sdk));
    }
    let reachable = reachable_from_manifest(profile);
    println!("    manifest_reachable");
    println!("      classes: {}", reachable.len());
    for (package, count) in reachable_package_counts(profile, &reachable)
        .into_iter()
        .take(top)
    {
        println!("      {package}: {count} classes");
    }
    println!();

    println!("  entrypoints");
    print_spec_entrypoints("activities", &profile.manifest.activities, top);
    print_spec_entrypoints("services", &profile.manifest.services, top);
    print_spec_entrypoints("receivers", &profile.manifest.receivers, top);
    println!();

    println!("  capabilities");
    print_spec_capabilities(profile, counts, top);
    println!();

    println!("  app_owned_signal_evidence");
    print_app_signal_evidence(profile, top);
    println!();

    println!("  conversion_units");
    print_conversion_units(profile, top);
    println!();

    println!("  workflows");
    for workflow in workflows {
        println!("    - {workflow}");
    }
    println!();

    println!("  reimplementation_plan");
    for step in reimplementation_steps(profile, counts) {
        println!("    - {step}");
    }
    println!();

    println!("  open_questions");
    for question in open_questions(profile, counts) {
        println!("    - {question}");
    }
}

fn print_edgerun_scaffold(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>, top: usize) {
    let package = profile
        .manifest
        .package_name
        .as_deref()
        .unwrap_or("<unknown>");
    let replacement_id = package.replace('.', "-");
    let sdks = detected_sdks(profile);
    let units = selected_conversion_units(profile, top);

    println!("edgerun_replacement_scaffold");
    println!("  app");
    println!("    id: {replacement_id}");
    println!("    source_android_package: {package}");
    println!("    confidence: {}", spec_confidence(profile, counts));
    println!("    generation_status: draft_static_analysis");
    println!();

    println!("  files");
    println!("    - edgerun.toml");
    println!("    - analysis.md");
    println!("    - src/main.rs");
    println!("    - src/events.rs");
    println!("    - src/capabilities.rs");
    println!("    - src/state.rs");
    println!("    - src/network.rs");
    println!("    - src/workflows/mod.rs");
    for unit in &units {
        println!(
            "    - src/workflows/{}.rs",
            workflow_file_stem(&unit.component.name)
        );
    }
    if !sdks.is_empty() {
        println!("    - src/third_party_modules.rs");
    }
    println!();

    println!("  edgerun_toml");
    println!("    [app]");
    println!("    id = \"{replacement_id}\"");
    println!("    source_android_package = \"{package}\"");
    println!("    status = \"draft_static_analysis\"");
    println!();
    println!("    [capabilities]");
    print_toml_capabilities(profile, counts, top);
    println!();

    println!("  third_party_modules");
    if sdks.is_empty() {
        println!("    <none detected>");
    } else {
        for sdk in sdks.iter().take(top) {
            let decision = third_party_decision(sdk);
            println!("    - name: {sdk}");
            println!("      default_action: {}", decision.default_action);
            println!("      breakage_risk: {}", decision.breakage_risk);
            println!("      rationale: {}", decision.rationale);
        }
    }
    println!();

    println!("  workflow_modules");
    if units.is_empty() {
        println!("    <no direct manifest workflow units selected>");
    }
    for unit in units {
        print_scaffold_workflow_module(&unit);
    }
    println!();

    println!("  runtime_observation_needed");
    for item in dynamic_trace_needs(profile, counts) {
        println!("    - {item}");
    }
}

fn emit_edgerun_scaffold(
    out_dir: &Path,
    profile: &ApkProfile,
    counts: &BTreeMap<ApiCall, u64>,
    top: usize,
) -> io::Result<()> {
    let src_dir = out_dir.join("src");
    let workflows_dir = src_dir.join("workflows");
    fs::create_dir_all(&workflows_dir)?;

    let units = selected_conversion_units(profile, top);
    let sdks = detected_sdks(profile);

    fs::write(
        out_dir.join("edgerun.toml"),
        render_edgerun_toml(profile, counts, top),
    )?;
    fs::write(
        out_dir.join("analysis.md"),
        render_analysis_md(profile, counts, &units, top),
    )?;
    fs::write(src_dir.join("main.rs"), render_main_rs())?;
    fs::write(src_dir.join("events.rs"), render_events_rs(&units))?;
    fs::write(
        src_dir.join("capabilities.rs"),
        render_capabilities_rs(profile, counts),
    )?;
    fs::write(src_dir.join("state.rs"), render_state_rs())?;
    fs::write(src_dir.join("network.rs"), render_network_rs(profile, top))?;
    fs::write(
        src_dir.join("third_party_modules.rs"),
        render_third_party_modules_rs(&sdks),
    )?;
    fs::write(
        workflows_dir.join("mod.rs"),
        render_workflows_mod_rs(&units),
    )?;

    for unit in &units {
        let file_name = format!("{}.rs", workflow_file_stem(&unit.component.name));
        fs::write(
            workflows_dir.join(file_name),
            render_workflow_rs(unit, profile),
        )?;
    }

    fs::write(
        out_dir.join("README.md"),
        render_scaffold_readme(profile, counts, &units),
    )?;
    Ok(())
}

fn render_edgerun_toml(
    profile: &ApkProfile,
    counts: &BTreeMap<ApiCall, u64>,
    top: usize,
) -> String {
    let package = profile
        .manifest
        .package_name
        .as_deref()
        .unwrap_or("<unknown>");
    let mut out = String::new();
    out.push_str("[app]\n");
    out.push_str(&format!(
        "id = \"{}\"\n",
        toml_escape(&package.replace('.', "-"))
    ));
    out.push_str(&format!(
        "source_android_package = \"{}\"\n",
        toml_escape(package)
    ));
    out.push_str("status = \"draft_static_analysis\"\n\n");
    out.push_str("[capabilities]\n");
    out.push_str(&render_toml_capabilities(profile, counts, top));
    out.push('\n');
    out.push_str("[analysis]\n");
    out.push_str(&format!("unique_signatures = {}\n", counts.len()));
    out.push_str(&format!(
        "total_call_sites = {}\n",
        counts.values().sum::<u64>()
    ));
    out.push_str(&format!(
        "confidence = \"{}\"\n",
        spec_confidence(profile, counts)
    ));
    out
}

fn render_toml_capabilities(
    profile: &ApkProfile,
    counts: &BTreeMap<ApiCall, u64>,
    top: usize,
) -> String {
    let mut out = String::new();
    if !profile.domains.is_empty() || signal_call_sites(counts, "network_web") > 0 {
        let domains = profile
            .domains
            .iter()
            .take(top)
            .map(|domain| format!("\"{}\"", toml_escape(domain)))
            .collect::<Vec<_>>()
            .join(", ");
        let urls = profile
            .urls
            .iter()
            .take(top)
            .map(|url| format!("\"{}\"", toml_escape(url)))
            .collect::<Vec<_>>()
            .join(", ");
        out.push_str(&format!(
            "network = {{ default = \"deny\", allow_domains = [{domains}], static_urls = [{urls}] }}\n"
        ));
    }
    if profile
        .manifest
        .permissions
        .iter()
        .any(|permission| permission.contains("LOCATION"))
        || signal_call_sites(counts, "location") > 0
    {
        let mode = if profile
            .manifest
            .permissions
            .iter()
            .any(|permission| permission.contains("FINE_LOCATION"))
        {
            "precise_or_coarse"
        } else {
            "coarse"
        };
        out.push_str(&format!(
            "location = {{ mode = \"{mode}\", prompt = \"per_use\" }}\n"
        ));
    }
    if signal_call_sites(counts, "files_storage") > 0 {
        out.push_str("storage = { app_private = true, document_grants = \"prompt\" }\n");
    }
    if signal_call_sites(counts, "database_preferences") > 0 {
        out.push_str("local_state = { backend = \"structured_store\" }\n");
    }
    if signal_call_sites(counts, "notifications") > 0 {
        out.push_str("notifications = { channel = \"app_scoped\", prompt = true }\n");
    }
    if signal_call_sites(counts, "work_background") > 0
        || !profile.manifest.services.is_empty()
        || !profile.manifest.receivers.is_empty()
    {
        out.push_str(
            "background_tasks = { scheduler = \"explicit\", event_subscriptions = \"review\" }\n",
        );
    }
    if signal_call_sites(counts, "camera_media_capture") > 0 {
        out.push_str("camera_media = { sessions = \"explicit_prompt\" }\n");
    }
    if signal_call_sites(counts, "bluetooth_nearby") > 0
        || profile
            .manifest
            .permissions
            .iter()
            .any(|permission| permission.contains("BLUETOOTH"))
    {
        out.push_str("bluetooth_nearby = { scan = \"prompt\", connect = \"allowlist\" }\n");
    }
    if signal_call_sites(counts, "crypto_security") > 0 {
        out.push_str("secrets_crypto = { keystore = \"edgerun\", certificates = \"review\" }\n");
    }
    out
}

fn render_main_rs() -> String {
    let mut out = String::new();
    out.push_str(
        "mod capabilities;\nmod events;\nmod network;\nmod state;\nmod third_party_modules;\nmod workflows;\n\n",
    );
    out.push_str("fn main() {\n");
    out.push_str("    let mut app_state = state::AppState::default();\n");
    out.push_str("    let capabilities = capabilities::Capabilities::default();\n");
    out.push_str("    third_party_modules::configure_defaults();\n");
    out.push_str("    for event in events::SEED_EVENTS {\n");
    out.push_str("        workflows::dispatch(*event, &mut app_state, &capabilities);\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

fn render_events_rs(units: &[SelectedUnit<'_>]) -> String {
    let event_names = unique_event_variant_names(units);
    let mut out = String::new();
    out.push_str("#[derive(Clone, Copy, Debug, Eq, PartialEq)]\n");
    out.push_str("pub enum AppEvent {\n");
    for event_name in &event_names {
        out.push_str(&format!("    {event_name},\n"));
    }
    out.push_str("    AndroidAction(&'static str),\n");
    out.push_str("}\n\n");

    out.push_str("pub const SEED_EVENTS: &[AppEvent] = &[\n");
    for event_name in &event_names {
        out.push_str(&format!("    AppEvent::{event_name},\n"));
    }
    for unit in units {
        for action in unit.component.actions.iter().take(8) {
            out.push_str(&format!(
                "    AppEvent::AndroidAction(\"{}\"),\n",
                rust_escape(action)
            ));
        }
    }
    out.push_str("];\n\n");

    out.push_str("impl AppEvent {\n");
    out.push_str("    pub fn source(self) -> &'static str {\n");
    out.push_str("        match self {\n");
    for (unit, event_name) in units.iter().zip(event_names.iter()) {
        out.push_str(&format!(
            "            AppEvent::{event_name} => \"{}\",\n",
            rust_escape(&unit.component.name)
        ));
    }
    out.push_str("            AppEvent::AndroidAction(action) => action,\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

fn render_capabilities_rs(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>) -> String {
    let mut out = String::new();
    out.push_str("#[derive(Debug, Default)]\npub struct Capabilities {\n");
    if signal_call_sites(counts, "accounts_identity") > 0 {
        out.push_str("    pub accounts_identity: AccountsIdentityCapability,\n");
    }
    if !profile.domains.is_empty() || signal_call_sites(counts, "network_web") > 0 {
        out.push_str("    pub network: NetworkCapability,\n");
    }
    if signal_call_sites(counts, "location") > 0 {
        out.push_str("    pub location: LocationCapability,\n");
    }
    if signal_call_sites(counts, "notifications") > 0 {
        out.push_str("    pub notifications: NotificationCapability,\n");
    }
    if signal_call_sites(counts, "files_storage") > 0 {
        out.push_str("    pub storage: StorageCapability,\n");
    }
    if signal_call_sites(counts, "database_preferences") > 0 {
        out.push_str("    pub local_state: LocalStateCapability,\n");
    }
    if signal_call_sites(counts, "camera_media_capture") > 0 {
        out.push_str("    pub camera_media: CameraMediaCapability,\n");
    }
    if signal_call_sites(counts, "bluetooth_nearby") > 0 {
        out.push_str("    pub bluetooth_nearby: BluetoothNearbyCapability,\n");
    }
    if signal_call_sites(counts, "crypto_security") > 0 {
        out.push_str("    pub secrets_crypto: SecretsCryptoCapability,\n");
    }
    if signal_call_sites(counts, "contacts_calendar") > 0 {
        out.push_str("    pub contacts_calendar: ContactsCalendarCapability,\n");
    }
    if signal_call_sites(counts, "package_intents") > 0 {
        out.push_str("    pub app_events: AppEventsCapability,\n");
    }
    if signal_call_sites(counts, "sensors") > 0 {
        out.push_str("    pub sensors: SensorsCapability,\n");
    }
    if signal_call_sites(counts, "sms_telephony") > 0 {
        out.push_str("    pub sms_telephony: SmsTelephonyCapability,\n");
    }
    if signal_call_sites(counts, "work_background") > 0
        || !profile.manifest.services.is_empty()
        || !profile.manifest.receivers.is_empty()
    {
        out.push_str("    pub background_tasks: BackgroundTasksCapability,\n");
    }
    out.push_str("}\n\n");
    out.push_str("#[derive(Clone, Copy, Debug, Eq, PartialEq)]\n");
    out.push_str("pub struct CapabilityIntent {\n");
    out.push_str("    pub capability: &'static str,\n");
    out.push_str("    pub operation: &'static str,\n");
    out.push_str("    pub source: &'static str,\n");
    out.push_str("}\n\n");
    out.push_str("#[derive(Debug, Default)] pub struct AccountsIdentityCapability;\n");
    out.push_str("#[derive(Debug, Default)] pub struct NetworkCapability;\n");
    out.push_str("#[derive(Debug, Default)] pub struct LocationCapability;\n");
    out.push_str("#[derive(Debug, Default)] pub struct NotificationCapability;\n");
    out.push_str("#[derive(Debug, Default)] pub struct StorageCapability;\n");
    out.push_str("#[derive(Debug, Default)] pub struct LocalStateCapability;\n");
    out.push_str("#[derive(Debug, Default)] pub struct CameraMediaCapability;\n");
    out.push_str("#[derive(Debug, Default)] pub struct BluetoothNearbyCapability;\n");
    out.push_str("#[derive(Debug, Default)] pub struct SecretsCryptoCapability;\n");
    out.push_str("#[derive(Debug, Default)] pub struct ContactsCalendarCapability;\n");
    out.push_str("#[derive(Debug, Default)] pub struct AppEventsCapability;\n");
    out.push_str("#[derive(Debug, Default)] pub struct SensorsCapability;\n");
    out.push_str("#[derive(Debug, Default)] pub struct SmsTelephonyCapability;\n");
    out.push_str("#[derive(Debug, Default)] pub struct BackgroundTasksCapability;\n\n");
    out.push_str("impl AccountsIdentityCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"accounts_identity\", operation, source) } }\n");
    out.push_str("impl NetworkCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"network_web\", operation, source) } }\n");
    out.push_str("impl LocationCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"location\", operation, source) } }\n");
    out.push_str("impl NotificationCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"notifications\", operation, source) } }\n");
    out.push_str("impl StorageCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"files_storage\", operation, source) } }\n");
    out.push_str("impl LocalStateCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"database_preferences\", operation, source) } }\n");
    out.push_str("impl CameraMediaCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"camera_media_capture\", operation, source) } }\n");
    out.push_str("impl BluetoothNearbyCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"bluetooth_nearby\", operation, source) } }\n");
    out.push_str("impl SecretsCryptoCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"crypto_security\", operation, source) } }\n");
    out.push_str("impl ContactsCalendarCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"contacts_calendar\", operation, source) } }\n");
    out.push_str("impl AppEventsCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"package_intents\", operation, source) } }\n");
    out.push_str("impl SensorsCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"sensors\", operation, source) } }\n");
    out.push_str("impl SmsTelephonyCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"sms_telephony\", operation, source) } }\n");
    out.push_str("impl BackgroundTasksCapability { pub fn request(&self, operation: &'static str, source: &'static str) -> CapabilityIntent { intent(\"work_background\", operation, source) } }\n\n");
    out.push_str("fn intent(capability: &'static str, operation: &'static str, source: &'static str) -> CapabilityIntent {\n");
    out.push_str("    CapabilityIntent { capability, operation, source }\n");
    out.push_str("}\n");
    out
}

fn render_state_rs() -> String {
    "use crate::capabilities::CapabilityIntent;\n\n#[derive(Debug, Default)]\npub struct AppState {\n    pub started_workflows: Vec<&'static str>,\n    pub handled_events: Vec<&'static str>,\n    pub capability_intents: Vec<CapabilityIntent>,\n}\n".to_string()
}

fn render_network_rs(profile: &ApkProfile, top: usize) -> String {
    let mut out = String::new();
    out.push_str("pub const STATIC_ENDPOINT_URIS: &[&str] = &[\n");
    for uri in likely_endpoint_uris(profile).into_iter().take(top) {
        out.push_str(&format!("    \"{}\",\n", rust_escape(uri)));
    }
    out.push_str("];\n\n");
    out.push_str("pub const STATIC_URIS: &[&str] = &[\n");
    for uri in profile.uris.iter().take(top) {
        out.push_str(&format!("    \"{}\",\n", rust_escape(uri)));
    }
    out.push_str("];\n\n");
    out.push_str("pub const STATIC_ENDPOINT_URLS: &[&str] = &[\n");
    for url in likely_endpoint_urls(profile).into_iter().take(top) {
        out.push_str(&format!("    \"{}\",\n", rust_escape(url)));
    }
    out.push_str("];\n\n");
    out.push_str("pub const STATIC_URLS: &[&str] = &[\n");
    for url in profile.urls.iter().take(top) {
        out.push_str(&format!("    \"{}\",\n", rust_escape(url)));
    }
    out.push_str("];\n\n");
    out.push_str("pub const ALLOW_DOMAINS: &[&str] = &[\n");
    for domain in profile.domains.iter().take(top) {
        out.push_str(&format!("    \"{}\",\n", rust_escape(domain)));
    }
    out.push_str("];\n\n");
    out.push_str("pub fn is_allowed_domain(domain: &str) -> bool {\n    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)\n}\n");
    out
}

fn render_third_party_modules_rs(sdks: &[String]) -> String {
    let mut out = String::new();
    out.push_str("#[derive(Debug)]\npub struct ThirdPartyModule {\n");
    out.push_str("    pub name: &'static str,\n    pub default_action: &'static str,\n    pub breakage_risk: &'static str,\n    pub rationale: &'static str,\n}\n\n");
    out.push_str("pub const MODULES: &[ThirdPartyModule] = &[\n");
    for sdk in sdks {
        let decision = third_party_decision(sdk);
        out.push_str("    ThirdPartyModule {\n");
        out.push_str(&format!("        name: \"{}\",\n", rust_escape(sdk)));
        out.push_str(&format!(
            "        default_action: \"{}\",\n",
            decision.default_action
        ));
        out.push_str(&format!(
            "        breakage_risk: \"{}\",\n",
            decision.breakage_risk
        ));
        out.push_str(&format!(
            "        rationale: \"{}\",\n",
            rust_escape(decision.rationale)
        ));
        out.push_str("    },\n");
    }
    out.push_str("];\n\n");
    out.push_str("pub fn configure_defaults() {\n    for module in MODULES {\n        let _ = module;\n    }\n}\n");
    out
}

fn render_workflows_mod_rs(units: &[SelectedUnit<'_>]) -> String {
    let mut out = String::new();
    let event_names = unique_event_variant_names(units);
    out.push_str("use crate::capabilities::Capabilities;\n");
    out.push_str("use crate::events::AppEvent;\n");
    out.push_str("use crate::state::AppState;\n\n");
    for unit in units {
        out.push_str(&format!(
            "pub mod {};\n",
            workflow_file_stem(&unit.component.name)
        ));
    }
    out.push('\n');
    out.push_str(
        "pub fn dispatch(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {\n",
    );
    out.push_str("    match event {\n");
    for (unit, event_name) in units.iter().zip(event_names.iter()) {
        out.push_str(&format!(
            "        AppEvent::{event_name} => {}::handle_event(event, state, capabilities),\n",
            workflow_file_stem(&unit.component.name)
        ));
    }
    out.push_str("        AppEvent::AndroidAction(action) => match action {\n");
    for unit in units {
        let module = workflow_file_stem(&unit.component.name);
        for action in &unit.component.actions {
            out.push_str(&format!(
                "            \"{}\" => {module}::handle_event(event, state, capabilities),\n",
                rust_escape(action)
            ));
        }
    }
    out.push_str("            _ => {}\n");
    out.push_str("        },\n");
    out.push_str("    }\n");
    out.push_str("}\n");
    out
}

fn render_workflow_rs(unit: &SelectedUnit<'_>, profile: &ApkProfile) -> String {
    let mut out = String::new();
    let rust_method_names = unique_rust_method_names(&unit.methods);
    out.push_str("use crate::capabilities::Capabilities;\nuse crate::events::AppEvent;\nuse crate::state::AppState;\n\n");
    out.push_str(&format!(
        "pub const SOURCE_CLASS: &str = \"{}\";\n",
        rust_escape(&unit.component.name)
    ));
    out.push_str(&format!("pub const KIND: &str = \"{}\";\n\n", unit.kind));
    out.push_str("pub fn handle_event(event: AppEvent, state: &mut AppState, capabilities: &Capabilities) {\n");
    out.push_str("    state.handled_events.push(event.source());\n");
    out.push_str("    state.started_workflows.push(SOURCE_CLASS);\n");
    for rust_name in &rust_method_names {
        out.push_str(&format!("    {}(state, capabilities);\n", rust_name));
    }
    out.push_str("}\n\n");
    for (method, rust_name) in unit.methods.iter().zip(rust_method_names.iter()) {
        out.push_str(&format!(
            "fn {}(state: &mut AppState, capabilities: &Capabilities) {{\n",
            rust_name
        ));
        out.push_str("    let _ = state;\n    let _ = capabilities;\n");
        out.push_str(&format!(
            "    // Source method: {}->{}\n",
            rust_escape(&method.class_name),
            rust_escape(&method.method_name)
        ));
        out.push_str(&format!(
            "    // Static call sites: platform={}, internal={}\n",
            method.platform_calls, method.internal_calls
        ));
        for (signal, count) in ranked_u64(&method.signals).into_iter().take(4) {
            out.push_str(&format!("    // Signal: {signal} ({count} call sites)\n"));
        }
        for (uri, count) in ranked_u64(&method.uri_constants).into_iter().take(4) {
            out.push_str(&format!(
                "    // Method URI constant: {} ({count} refs)\n",
                rust_escape(uri)
            ));
        }
        for (constant, count) in ranked_u64(&method.string_constants).into_iter().take(4) {
            if !method.uri_constants.contains_key(constant.as_str()) {
                out.push_str(&format!(
                    "    // Method string constant: {} ({count} refs)\n",
                    rust_escape(constant)
                ));
            }
        }
        for (signal, _) in ranked_u64(&method.signals).into_iter().take(4) {
            if let Some((field, operation)) = capability_intent_call(signal) {
                out.push_str(&format!(
                    "    state.capability_intents.push(capabilities.{field}.request(\"{operation}\", SOURCE_CLASS));\n"
                ));
            }
        }
        for task in scaffold_tasks_for_method(method) {
            out.push_str(&format!("    // TODO: {task}\n"));
        }
        out.push_str("}\n\n");
    }
    if unit.methods.is_empty() {
        out.push_str(
            "fn extracted_entrypoint(state: &mut AppState, capabilities: &Capabilities) {\n",
        );
        out.push_str("    let _ = state;\n    let _ = capabilities;\n");
        out.push_str(
            "    // TODO: No method body was directly recovered for this manifest entrypoint.\n",
        );
        out.push_str("}\n");
    }
    let _ = profile;
    out
}

fn render_analysis_md(
    profile: &ApkProfile,
    counts: &BTreeMap<ApiCall, u64>,
    units: &[SelectedUnit<'_>],
    top: usize,
) -> String {
    let package = profile
        .manifest
        .package_name
        .as_deref()
        .unwrap_or("<unknown>");
    let mut out = String::new();
    out.push_str("# APK Behavior Analysis\n\n");
    out.push_str("## Identity\n\n");
    out.push_str(&format!("- Source package: `{package}`\n"));
    out.push_str(&format!(
        "- Confidence: `{}`\n",
        spec_confidence(profile, counts)
    ));
    out.push_str(&format!("- Unique API signatures: `{}`\n", counts.len()));
    out.push_str(&format!(
        "- Static call sites: `{}`\n\n",
        counts.values().sum::<u64>()
    ));

    out.push_str("## Event Model\n\n");
    let event_names = unique_event_variant_names(units);
    if units.is_empty() {
        out.push_str("- No typed workflow events were generated.\n\n");
    } else {
        for (unit, event_name) in units.iter().zip(event_names.iter()) {
            out.push_str(&format!(
                "- `{event_name}` routes to `{}` ({})\n",
                unit.component.name, unit.kind
            ));
            for action in unit.component.actions.iter().take(4) {
                out.push_str(&format!("  - Android action `{action}`\n"));
            }
        }
        out.push('\n');
    }

    out.push_str("## Main Workflows\n\n");
    if units.is_empty() {
        out.push_str("- No manifest entrypoint classes had directly parsed code.\n\n");
    } else {
        for unit in units {
            out.push_str(&format!(
                "- `{}` ({}) exported={}\n",
                unit.component.name,
                unit.kind,
                unit.component
                    .exported
                    .map(|value| value.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ));
            if !unit.component.actions.is_empty() {
                out.push_str(&format!(
                    "  actions: `{}`\n",
                    unit.component
                        .actions
                        .iter()
                        .take(4)
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("`, `")
                ));
            }
            for method in &unit.methods {
                out.push_str(&format!(
                    "  method `{}`: platform={}, internal={}",
                    method.method_name, method.platform_calls, method.internal_calls
                ));
                let signals = ranked_u64(&method.signals)
                    .into_iter()
                    .take(3)
                    .map(|(signal, count)| format!("{signal}:{count}"))
                    .collect::<Vec<_>>();
                if !signals.is_empty() {
                    out.push_str(&format!(" signals={}", signals.join(", ")));
                }
                out.push('\n');
            }
        }
        out.push('\n');
    }

    out.push_str("## Capability Evidence\n\n");
    let capability_rows = signals()
        .iter()
        .filter_map(|signal| {
            let count = signal_call_sites(counts, signal.name);
            (count > 0).then_some((signal.name, count, signal_description(signal.name)))
        })
        .collect::<Vec<_>>();
    if capability_rows.is_empty() {
        out.push_str("- No Android capability signal calls were detected in selected output.\n\n");
    } else {
        for (name, count, description) in capability_rows {
            out.push_str(&format!("- `{name}`: {count} call sites; {description}\n"));
        }
        out.push('\n');
    }

    out.push_str("## Generated Capability Intent Plan\n\n");
    let mut wrote_intents = false;
    for unit in units {
        for method in &unit.methods {
            let intents = ranked_u64(&method.signals)
                .into_iter()
                .filter_map(|(signal, _)| {
                    capability_intent_call(signal)
                        .map(|(field, operation)| (*signal, field, operation))
                })
                .take(5)
                .collect::<Vec<_>>();
            if intents.is_empty() {
                continue;
            }
            wrote_intents = true;
            out.push_str(&format!(
                "- `{}` -> `{}` records:\n",
                unit.component.name, method.method_name
            ));
            for (signal, field, operation) in intents {
                out.push_str(&format!(
                    "  - `{field}.{operation}` from `{signal}` evidence\n"
                ));
            }
        }
    }
    if !wrote_intents {
        out.push_str("- No generated workflow method currently records capability intents.\n");
    }
    out.push('\n');

    out.push_str("## Method Local Constants\n\n");
    let mut wrote_constants = false;
    for unit in units {
        for method in &unit.methods {
            if method.string_constants.is_empty() && method.uri_constants.is_empty() {
                continue;
            }
            wrote_constants = true;
            out.push_str(&format!(
                "- `{}` -> `{}` constants:\n",
                unit.component.name, method.method_name
            ));
            for (uri, count) in ranked_u64(&method.uri_constants).into_iter().take(5) {
                out.push_str(&format!(
                    "  - uri `{}` ({count})\n",
                    markdown_escape_inline(uri)
                ));
            }
            for (constant, count) in ranked_u64(&method.string_constants).into_iter().take(5) {
                if method.uri_constants.contains_key(constant.as_str()) {
                    continue;
                }
                out.push_str(&format!(
                    "  - string `{}` ({count})\n",
                    markdown_escape_inline(constant)
                ));
            }
        }
    }
    if !wrote_constants {
        out.push_str("- No selected workflow methods had static string constants.\n");
    }
    out.push('\n');

    out.push_str("## UI And Resource Hints\n\n");
    if profile.resource_files.is_empty()
        && profile.ui_elements.is_empty()
        && profile.ui_texts.is_empty()
        && profile.asset_files.is_empty()
    {
        out.push_str("- No resource hints were extracted.\n\n");
    } else {
        if !profile.resource_files.is_empty() {
            out.push_str("- Resource buckets:");
            for (bucket, count) in ranked_u64(&profile.resource_files).into_iter().take(top) {
                out.push_str(&format!(" `{}`({})", bucket, count));
            }
            out.push('\n');
        }
        if !profile.ui_elements.is_empty() {
            out.push_str("- UI elements:");
            for (element, count) in ranked_u64(&profile.ui_elements).into_iter().take(top) {
                out.push_str(&format!(" `{}`({})", element, count));
            }
            out.push('\n');
        }
        if !profile.ui_texts.is_empty() {
            out.push_str("- Literal UI text samples:\n");
            for text in profile.ui_texts.iter().take(top) {
                out.push_str(&format!("  - `{}`\n", markdown_escape_inline(text)));
            }
        }
        if !profile.asset_files.is_empty() {
            out.push_str("- Asset samples:\n");
            for asset in profile.asset_files.iter().take(top) {
                out.push_str(&format!("  - `{}`\n", markdown_escape_inline(asset)));
            }
        }
        out.push('\n');
    }

    out.push_str("## Network And Native Surface\n\n");
    let endpoint_uris = likely_endpoint_uris(profile);
    if endpoint_uris.is_empty() {
        out.push_str("- Static endpoint URI candidates: none found\n");
    } else {
        out.push_str("- Static endpoint URI candidates:\n");
        for uri in endpoint_uris.into_iter().take(top) {
            out.push_str(&format!("  - `{}`\n", markdown_escape_inline(uri)));
        }
    }
    if !profile.uri_schemes.is_empty() {
        out.push_str("- Static URI schemes:");
        for (scheme, count) in ranked_u64(&profile.uri_schemes).into_iter().take(top) {
            out.push_str(&format!(" `{scheme}`({count})"));
        }
        out.push('\n');
    }
    if profile.uris.is_empty() {
        out.push_str("- Static URI strings: none found\n");
    } else {
        out.push_str("- Static URI strings:\n");
        for uri in profile.uris.iter().take(top) {
            out.push_str(&format!("  - `{}`\n", markdown_escape_inline(uri)));
        }
    }
    let endpoint_urls = likely_endpoint_urls(profile);
    if endpoint_urls.is_empty() {
        out.push_str("- Static endpoint URL candidates: none found\n");
    } else {
        out.push_str("- Static endpoint URL candidates:\n");
        for url in endpoint_urls.into_iter().take(top) {
            out.push_str(&format!("  - `{}`\n", markdown_escape_inline(url)));
        }
    }
    if profile.urls.is_empty() {
        out.push_str("- Static URL strings: none found\n");
    } else {
        out.push_str("- Static URL strings:\n");
        for url in profile.urls.iter().take(top) {
            out.push_str(&format!("  - `{}`\n", markdown_escape_inline(url)));
        }
    }
    if profile.domains.is_empty() {
        out.push_str("- Static domain strings: none found\n");
    } else {
        out.push_str("- Static domain strings:\n");
        for domain in profile.domains.iter().take(top) {
            out.push_str(&format!("  - `{domain}`\n"));
        }
    }
    if profile.native_libs.is_empty() {
        out.push_str("- Native libraries: none found\n\n");
    } else {
        out.push_str("- Native libraries:\n");
        for lib in profile.native_libs.iter().take(top) {
            out.push_str(&format!("  - `{lib}`\n"));
        }
        out.push('\n');
    }

    out.push_str("## Third-Party Modules\n\n");
    let sdks = detected_sdks(profile);
    if sdks.is_empty() {
        out.push_str("- No known SDK clusters detected.\n\n");
    } else {
        for sdk in sdks.iter().take(top) {
            let decision = third_party_decision(sdk);
            out.push_str(&format!(
                "- `{sdk}`: default=`{}`, breakage=`{}`; {}\n",
                decision.default_action, decision.breakage_risk, decision.rationale
            ));
        }
        out.push('\n');
    }

    out.push_str("## Runtime Observation Needed\n\n");
    for item in dynamic_trace_needs(profile, counts) {
        out.push_str(&format!("- {item}\n"));
    }
    out
}

fn render_scaffold_readme(
    profile: &ApkProfile,
    counts: &BTreeMap<ApiCall, u64>,
    units: &[SelectedUnit<'_>],
) -> String {
    let package = profile
        .manifest
        .package_name
        .as_deref()
        .unwrap_or("<unknown>");
    let mut out = String::new();
    out.push_str("# Edgerun APK Replacement Scaffold\n\n");
    out.push_str(&format!("- Source Android package: `{package}`\n"));
    out.push_str(&format!("- Unique API signatures: `{}`\n", counts.len()));
    out.push_str(&format!(
        "- Total static call sites: `{}`\n",
        counts.values().sum::<u64>()
    ));
    out.push_str(&format!("- Workflow modules: `{}`\n\n", units.len()));
    out.push_str(
        "This scaffold is generated from static APK analysis. It is not a working app yet. Start with `analysis.md` for the extracted behavior map.\n\n",
    );
    out.push_str("Next steps:\n");
    for item in dynamic_trace_needs(profile, counts) {
        out.push_str(&format!("- {item}\n"));
    }
    out
}

struct SelectedUnit<'a> {
    kind: &'static str,
    component: &'a ComponentProfile,
    class_summary: &'a ClassSummary,
    methods: Vec<&'a MethodSummary>,
}

struct ThirdPartyDecision {
    default_action: &'static str,
    breakage_risk: &'static str,
    rationale: &'static str,
}

fn selected_conversion_units<'a>(profile: &'a ApkProfile, top: usize) -> Vec<SelectedUnit<'a>> {
    let mut components = Vec::new();
    collect_component_units(&mut components, "activity", &profile.manifest.activities);
    collect_component_units(&mut components, "service", &profile.manifest.services);
    collect_component_units(&mut components, "receiver", &profile.manifest.receivers);
    let owned_packages = likely_owned_packages(profile);
    let mut units = Vec::new();

    for (kind, component) in components {
        if units.len() >= top {
            break;
        }
        if !owned_packages.is_empty()
            && !owned_packages.iter().any(|prefix| {
                component.name == *prefix || component.name.starts_with(&format!("{prefix}."))
            })
        {
            continue;
        }
        let Some(class_summary) = profile.class_summaries.get(&component.name) else {
            continue;
        };
        let methods = selected_entry_methods(profile, &component.name, 5);
        units.push(SelectedUnit {
            kind,
            component,
            class_summary,
            methods,
        });
    }
    units
}

fn selected_entry_methods<'a>(
    profile: &'a ApkProfile,
    class_name: &str,
    top: usize,
) -> Vec<&'a MethodSummary> {
    let lifecycle_names = [
        "onCreate",
        "onStart",
        "onResume",
        "onNewIntent",
        "onActivityResult",
        "onRequestPermissionsResult",
        "onStartCommand",
        "onBind",
        "onMessageReceived",
        "onReceive",
        "doWork",
        "handleIntent",
        "handleMessage",
    ];
    let mut rows: Vec<_> = profile
        .method_summaries
        .values()
        .filter(|method| method.class_name == class_name)
        .collect();
    rows.sort_by(|left, right| {
        let left_lifecycle = lifecycle_names.contains(&left.method_name.as_str());
        let right_lifecycle = lifecycle_names.contains(&right.method_name.as_str());
        right_lifecycle
            .cmp(&left_lifecycle)
            .then_with(|| {
                (right.platform_calls + right.internal_calls)
                    .cmp(&(left.platform_calls + left.internal_calls))
            })
            .then_with(|| left.method_name.cmp(&right.method_name))
    });
    rows.truncate(top);
    rows
}

fn print_toml_capabilities(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>, top: usize) {
    if !profile.domains.is_empty() || signal_call_sites(counts, "network_web") > 0 {
        let domains = profile
            .domains
            .iter()
            .take(top)
            .map(|domain| format!("\"{domain}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let urls = profile
            .urls
            .iter()
            .take(top)
            .map(|url| format!("\"{url}\""))
            .collect::<Vec<_>>()
            .join(", ");
        println!(
            "    network = {{ default = \"deny\", allow_domains = [{domains}], static_urls = [{urls}] }}"
        );
    }
    if profile
        .manifest
        .permissions
        .iter()
        .any(|permission| permission.contains("LOCATION"))
        || signal_call_sites(counts, "location") > 0
    {
        let mode = if profile
            .manifest
            .permissions
            .iter()
            .any(|permission| permission.contains("FINE_LOCATION"))
        {
            "precise_or_coarse"
        } else {
            "coarse"
        };
        println!("    location = {{ mode = \"{mode}\", prompt = \"per_use\" }}");
    }
    if signal_call_sites(counts, "files_storage") > 0 {
        println!("    storage = {{ app_private = true, document_grants = \"prompt\" }}");
    }
    if signal_call_sites(counts, "database_preferences") > 0 {
        println!("    local_state = {{ backend = \"structured_store\" }}");
    }
    if signal_call_sites(counts, "notifications") > 0 {
        println!("    notifications = {{ channel = \"app_scoped\", prompt = true }}");
    }
    if signal_call_sites(counts, "work_background") > 0
        || !profile.manifest.services.is_empty()
        || !profile.manifest.receivers.is_empty()
    {
        println!(
            "    background_tasks = {{ scheduler = \"explicit\", event_subscriptions = \"review\" }}"
        );
    }
    if signal_call_sites(counts, "camera_media_capture") > 0 {
        println!("    camera_media = {{ sessions = \"explicit_prompt\" }}");
    }
    if signal_call_sites(counts, "bluetooth_nearby") > 0
        || profile
            .manifest
            .permissions
            .iter()
            .any(|permission| permission.contains("BLUETOOTH"))
    {
        println!("    bluetooth_nearby = {{ scan = \"prompt\", connect = \"allowlist\" }}");
    }
    if signal_call_sites(counts, "crypto_security") > 0 {
        println!("    secrets_crypto = {{ keystore = \"edgerun\", certificates = \"review\" }}");
    }
}

fn third_party_decision(sdk: &str) -> ThirdPartyDecision {
    if sdk.contains("Firebase") || sdk.contains("Google Play services") {
        ThirdPartyDecision {
            default_action: "preserve_minimal",
            breakage_risk: "medium",
            rationale: "often carries auth, push, maps, safety, or wearable integrations; analytics should remain opt-out",
        }
    } else if sdk.contains("Braze")
        || sdk.contains("AppsFlyer")
        || sdk.contains("Adjust")
        || sdk.contains("Amplitude")
        || sdk.contains("Segment")
        || sdk.contains("Sentry")
        || sdk.contains("ShakeBugs")
    {
        ThirdPartyDecision {
            default_action: "opt_out",
            breakage_risk: "medium",
            rationale: "developer included it, but it is usually telemetry, attribution, campaigns, or support; user can enable with acknowledgement",
        }
    } else if sdk.contains("Onfido") || sdk.contains("iProov") {
        ThirdPartyDecision {
            default_action: "preserve_if_workflow_requires",
            breakage_risk: "high",
            rationale: "identity verification may be core functionality and may require vendor backend compatibility",
        }
    } else if sdk.contains("Billing") {
        ThirdPartyDecision {
            default_action: "preserve_if_paid_flow_required",
            breakage_risk: "high",
            rationale: "paid/subscription flows can break without store billing integration",
        }
    } else if sdk.contains("Klarna") {
        ThirdPartyDecision {
            default_action: "preserve_if_checkout_required",
            breakage_risk: "high",
            rationale: "payment and auth redirects can be core checkout behavior and need explicit user-visible replacement",
        }
    } else if sdk.contains("Meta/Facebook") {
        ThirdPartyDecision {
            default_action: "preserve_if_login_or_share_required",
            breakage_risk: "medium",
            rationale: "may provide login, sharing, attribution, or web redirect handling; make optional unless a workflow uses it",
        }
    } else if sdk.contains("Flutter") || sdk.contains("Chromium") {
        ThirdPartyDecision {
            default_action: "replace_gradually",
            breakage_risk: "high",
            rationale: "runtime or rendering framework; rebuild screen-by-screen instead of silently dropping it",
        }
    } else {
        ThirdPartyDecision {
            default_action: "preserve_if_reachable",
            breakage_risk: "unknown",
            rationale: "module appears in the app; dynamic trace should decide whether it is required",
        }
    }
}

fn print_scaffold_workflow_module(unit: &SelectedUnit<'_>) {
    println!("    - module: {}", workflow_file_stem(&unit.component.name));
    println!("      kind: {}", unit.kind);
    println!("      source_class: {}", unit.component.name);
    println!(
        "      source_exported: {}",
        unit.component
            .exported
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    );
    println!(
        "      direct_platform_calls: {}",
        unit.class_summary.platform_calls
    );
    println!(
        "      direct_internal_calls: {}",
        unit.class_summary.internal_calls
    );
    if !unit.component.actions.is_empty() {
        println!("      triggers");
        for action in unit.component.actions.iter().take(4) {
            println!("        - {action}");
        }
    }
    println!("      loop_methods");
    for method in &unit.methods {
        println!("        - name: {}", method.method_name);
        println!("          platform_call_sites: {}", method.platform_calls);
        println!("          internal_call_sites: {}", method.internal_calls);
        print_ranked_inline("signals", &method.signals, 3);
        print_ranked_inline("calls_packages", &method.callee_packages, 4);
    }
    println!("      scaffold_tasks");
    for task in scaffold_tasks_for_unit(unit) {
        println!("        - {task}");
    }
}

fn scaffold_tasks_for_unit(unit: &SelectedUnit<'_>) -> Vec<String> {
    let mut tasks = BTreeSet::new();
    tasks.insert("translate lifecycle method into explicit Edgerun workflow function".to_string());
    for method in &unit.methods {
        for signal in method.signals.keys() {
            match *signal {
                "network_web" => {
                    tasks.insert(
                        "route network calls through constrained network capability".to_string(),
                    );
                }
                "package_intents" => {
                    tasks.insert("replace Android intents with typed Edgerun events".to_string());
                }
                "database_preferences" => {
                    tasks.insert("map local persistence to structured store".to_string());
                }
                "notifications" => {
                    tasks.insert("emit notifications through app-scoped channel".to_string());
                }
                "location" => {
                    tasks.insert("request location only inside this workflow".to_string());
                }
                "camera_media_capture" => {
                    tasks
                        .insert("wrap camera/media access in explicit session prompts".to_string());
                }
                "bluetooth_nearby" => {
                    tasks.insert("wrap Bluetooth scan/connect in allowlisted sessions".to_string());
                }
                "crypto_security" => {
                    tasks.insert(
                        "move keys and signatures behind secrets/crypto capability".to_string(),
                    );
                }
                _ => {}
            }
        }
    }
    tasks.into_iter().collect()
}

fn scaffold_tasks_for_method(method: &MethodSummary) -> Vec<String> {
    let mut tasks = BTreeSet::new();
    tasks.insert(
        "translate this lifecycle/body method into explicit Edgerun control flow".to_string(),
    );
    for signal in method.signals.keys() {
        match *signal {
            "network_web" => {
                tasks.insert("replace direct Android/JVM network access with crate::network allowlist checks".to_string());
            }
            "package_intents" => {
                tasks.insert(
                    "replace Android Intent behavior with typed Edgerun events".to_string(),
                );
            }
            "database_preferences" => {
                tasks.insert(
                    "move persistence into crate::state or Edgerun structured storage".to_string(),
                );
            }
            "notifications" => {
                tasks.insert(
                    "emit notifications through app-scoped notification capability".to_string(),
                );
            }
            "location" => {
                tasks.insert("request location only when this workflow runs".to_string());
            }
            "camera_media_capture" => {
                tasks.insert("wrap camera/media access in explicit user sessions".to_string());
            }
            "bluetooth_nearby" => {
                tasks.insert("wrap Bluetooth scan/connect in device allowlist flow".to_string());
            }
            "crypto_security" => {
                tasks.insert(
                    "move keys/certificates/signatures behind secrets/crypto capability"
                        .to_string(),
                );
            }
            _ => {}
        }
    }
    tasks.into_iter().collect()
}

fn dynamic_trace_needs(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>) -> Vec<String> {
    let mut needs = BTreeSet::new();
    needs.insert("capture launch-to-first-use screen flow and selected UI actions".to_string());
    if !profile.domains.is_empty() || signal_call_sites(counts, "network_web") > 0 {
        needs.insert("record actual contacted domains and authentication redirects".to_string());
    }
    if !profile.manifest.services.is_empty() || !profile.manifest.receivers.is_empty() {
        needs.insert("record which services/receivers fire during normal use".to_string());
    }
    if !detected_sdks(profile).is_empty() {
        needs.insert(
            "toggle third-party modules and observe breakage before default opt-out".to_string(),
        );
    }
    if signal_call_sites(counts, "location") > 0 {
        needs.insert("identify exact user action that requests location".to_string());
    }
    if signal_call_sites(counts, "camera_media_capture") > 0 {
        needs.insert(
            "identify camera/media entrypoints and whether they are core workflow or vendor SDK"
                .to_string(),
        );
    }
    needs.into_iter().collect()
}

fn workflow_file_stem(class_name: &str) -> String {
    class_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn rust_method_name(method_name: &str) -> String {
    let mut out = String::new();
    for ch in method_name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    let out = out.trim_matches('_');
    let out = if out.is_empty() { "method" } else { out };
    if out.as_bytes().first().is_some_and(|b| b.is_ascii_digit()) {
        format!("method_{out}")
    } else {
        out.to_string()
    }
}

fn unique_rust_method_names(methods: &[&MethodSummary]) -> Vec<String> {
    let mut seen = BTreeMap::new();
    let mut names = Vec::with_capacity(methods.len());

    for method in methods {
        let base = rust_method_name(&method.method_name);
        let count = seen.entry(base.clone()).or_insert(0usize);
        if *count == 0 {
            names.push(base);
        } else {
            names.push(format!("{base}_{}", *count + 1));
        }
        *count += 1;
    }

    names
}

fn unique_event_variant_names(units: &[SelectedUnit<'_>]) -> Vec<String> {
    let mut seen = BTreeMap::new();
    let mut names = Vec::with_capacity(units.len());

    for unit in units {
        let base = rust_type_name(&workflow_file_stem(&unit.component.name));
        let count = seen.entry(base.clone()).or_insert(0usize);
        if *count == 0 {
            names.push(base);
        } else {
            names.push(format!("{base}{}", *count + 1));
        }
        *count += 1;
    }

    names
}

fn rust_type_name(stem: &str) -> String {
    let mut out = String::new();
    let mut uppercase_next = true;
    for ch in stem.chars() {
        if ch.is_ascii_alphanumeric() {
            if uppercase_next {
                out.push(ch.to_ascii_uppercase());
                uppercase_next = false;
            } else {
                out.push(ch.to_ascii_lowercase());
            }
        } else {
            uppercase_next = true;
        }
    }
    if out.is_empty() {
        out.push_str("WorkflowEvent");
    }
    if out.as_bytes().first().is_some_and(|b| b.is_ascii_digit()) {
        out.insert_str(0, "Workflow");
    }
    out
}

fn rust_escape(input: &str) -> String {
    input
        .chars()
        .flat_map(|ch| match ch {
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            c => vec![c],
        })
        .collect()
}

fn markdown_escape_inline(input: &str) -> String {
    input.replace('\\', "\\\\").replace('`', "\\`")
}

fn toml_escape(input: &str) -> String {
    rust_escape(input)
}

fn ranked_u64<K: AsRef<str> + Ord>(map: &BTreeMap<K, u64>) -> Vec<(&K, u64)> {
    let mut rows: Vec<_> = map.iter().map(|(key, value)| (key, *value)).collect();
    rows.sort_by(|(left_key, left), (right_key, right)| {
        right
            .cmp(left)
            .then_with(|| left_key.as_ref().cmp(right_key.as_ref()))
    });
    rows
}

fn likely_app_prefixes(profile: &ApkProfile) -> Vec<String> {
    let package = profile.manifest.package_name.as_deref().unwrap_or("");
    let mut prefixes = Vec::new();
    if !package.is_empty() {
        for (prefix, count) in &profile.dex_packages {
            if (prefix == package || prefix.starts_with(&format!("{package}."))) && *count > 0 {
                prefixes.push(prefix.clone());
            }
        }
        if prefixes.is_empty() {
            let root = package_root(package, 3);
            for (prefix, count) in &profile.dex_packages {
                if prefix.starts_with(&root) && *count > 0 {
                    prefixes.push(prefix.clone());
                }
            }
        }
    }
    if prefixes.is_empty() {
        let reachable = reachable_from_manifest(profile);
        for (prefix, count) in reachable_package_counts(profile, &reachable) {
            if count >= 2 && !is_known_sdk_package(&prefix) && !is_platform_package(&prefix) {
                prefixes.push(prefix);
            }
        }
    }
    prefixes.sort_by(|left, right| {
        profile
            .dex_packages
            .get(right)
            .cmp(&profile.dex_packages.get(left))
            .then_with(|| left.cmp(right))
    });
    prefixes
}

fn likely_owned_packages(profile: &ApkProfile) -> Vec<String> {
    let mut out = BTreeSet::new();
    for prefix in likely_app_prefixes(profile) {
        out.insert(prefix);
    }
    let reachable = reachable_from_manifest(profile);
    for (package, count) in reachable_package_counts(profile, &reachable) {
        if count >= 2 && !is_known_sdk_package(&package) {
            out.insert(package);
        }
    }
    out.into_iter().collect()
}

fn detected_sdks(profile: &ApkProfile) -> Vec<String> {
    const SDKS: &[(&str, &str)] = &[
        ("com.google.firebase", "Firebase"),
        ("com.google.android.gms", "Google Play services"),
        ("com.google.android.play", "Google Play libraries"),
        ("com.facebook", "Meta/Facebook SDK"),
        ("com.klarna", "Klarna payment/auth SDK"),
        ("com.braze", "Braze engagement SDK"),
        ("bo.app", "Braze engagement SDK"),
        ("com.android.billingclient", "Google Play Billing"),
        ("com.onfido", "Onfido identity verification"),
        ("com.iproov", "iProov identity verification"),
        ("com.appsflyer", "AppsFlyer attribution"),
        ("com.adjust", "Adjust attribution"),
        ("com.segment", "Segment analytics"),
        ("com.amplitude", "Amplitude analytics"),
        ("com.incognia", "Incognia risk/location SDK"),
        ("com.shakebugs", "ShakeBugs feedback SDK"),
        ("io.sentry", "Sentry telemetry SDK"),
        ("io.flutter", "Flutter runtime/plugins"),
        ("org.bouncycastle", "Bouncy Castle crypto"),
        ("org.conscrypt", "Conscrypt TLS"),
        ("com.squareup", "Square/OkHttp/Moshi stack"),
        ("okhttp3", "OkHttp"),
        ("retrofit2", "Retrofit"),
        ("androidx.work", "AndroidX WorkManager"),
        ("androidx.room", "AndroidX Room database"),
        ("androidx.camera", "AndroidX Camera"),
        ("org.chromium", "Chromium/WebView code"),
    ];
    let mut out = BTreeSet::new();
    for prefix in profile.dex_packages.keys() {
        for (sdk_prefix, name) in SDKS {
            if prefix.starts_with(sdk_prefix) {
                out.insert(*name);
            }
        }
    }
    out.into_iter().map(str::to_string).collect()
}

fn third_party_policy(sdk: &str) -> &'static str {
    if sdk.contains("Firebase") || sdk.contains("Google Play services") {
        "preserve only required auth/push/maps APIs; replace analytics by default"
    } else if sdk.contains("Braze")
        || sdk.contains("AppsFlyer")
        || sdk.contains("Adjust")
        || sdk.contains("Amplitude")
        || sdk.contains("Segment")
        || sdk.contains("Sentry")
        || sdk.contains("ShakeBugs")
    {
        "optional module; default opt-out may break campaigns, attribution, telemetry, or support flows"
    } else if sdk.contains("Onfido") || sdk.contains("iProov") {
        "preserve only if identity verification is core to the app workflow"
    } else if sdk.contains("Billing") {
        "preserve only paid/subscription flows explicitly needed by user"
    } else if sdk.contains("Flutter") || sdk.contains("Chromium") {
        "runtime/framework dependency; replace screen-by-screen when rebuilding natively"
    } else {
        "preserve if directly required by an extracted workflow; otherwise make optional"
    }
}

fn reachable_from_manifest(profile: &ApkProfile) -> BTreeSet<String> {
    let mut reached = BTreeSet::new();
    let mut queue = Vec::new();
    for component in profile
        .manifest
        .activities
        .iter()
        .chain(profile.manifest.services.iter())
        .chain(profile.manifest.receivers.iter())
        .chain(profile.manifest.providers.iter())
    {
        if profile.class_summaries.contains_key(&component.name) {
            queue.push((component.name.clone(), 0usize));
        }
    }

    while let Some((class_name, depth)) = queue.pop() {
        if !reached.insert(class_name.clone()) {
            continue;
        }
        if depth >= 4 || reached.len() >= 4_000 {
            continue;
        }
        let Some(summary) = profile.class_summaries.get(&class_name) else {
            continue;
        };
        for callee in summary.callee_classes.keys() {
            if !reached.contains(callee) && !is_known_sdk_package(&package_name(callee)) {
                queue.push((callee.clone(), depth + 1));
            }
        }
    }
    reached
}

fn reachable_package_counts(
    profile: &ApkProfile,
    reachable: &BTreeSet<String>,
) -> Vec<(String, u64)> {
    let mut counts = BTreeMap::new();
    for class_name in reachable {
        let package = profile
            .class_summaries
            .get(class_name)
            .map(|summary| summary.package_name.clone())
            .unwrap_or_else(|| package_name(class_name));
        if package.is_empty() {
            continue;
        }
        *counts.entry(package).or_insert(0) += 1;
    }
    let mut rows: Vec<_> = counts.into_iter().collect();
    rows.sort_by(|(left_pkg, left), (right_pkg, right)| {
        right.cmp(left).then_with(|| left_pkg.cmp(right_pkg))
    });
    rows
}

fn is_platform_package(package: &str) -> bool {
    package.starts_with("android.")
        || package.starts_with("androidx.")
        || package.starts_with("java.")
        || package.starts_with("javax.")
        || package.starts_with("kotlin.")
        || package.starts_with("kotlinx.")
        || package.starts_with("dalvik.")
}

fn is_known_sdk_package(package: &str) -> bool {
    is_platform_package(package)
        || package.starts_with("com.google.")
        || package == "com.facebook"
        || package.starts_with("com.facebook.")
        || package == "com.klarna"
        || package.starts_with("com.klarna.")
        || package == "com.braze"
        || package.starts_with("com.braze.")
        || package == "bo.app"
        || package.starts_with("bo.app.")
        || package == "com.android.billingclient"
        || package.starts_with("com.android.billingclient.")
        || package.starts_with("com.onfido.")
        || package.starts_with("com.iproov.")
        || package.starts_with("com.appsflyer.")
        || package.starts_with("com.adjust.")
        || package.starts_with("com.segment.")
        || package.starts_with("com.amplitude.")
        || package.starts_with("com.incognia.")
        || package.starts_with("com.shakebugs.")
        || package.starts_with("io.sentry.")
        || package.starts_with("io.flutter.")
        || package.starts_with("org.bouncycastle.")
        || package.starts_with("org.conscrypt.")
        || package == "okhttp3"
        || package.starts_with("okhttp3.")
        || package == "retrofit2"
        || package.starts_with("retrofit2.")
        || package.starts_with("org.chromium.")
        || package.starts_with("j$.")
}

fn infer_workflows(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>) -> Vec<String> {
    let mut workflows = BTreeSet::new();
    let package = profile.manifest.package_name.as_deref().unwrap_or("");
    let all_text = manifest_text(profile).to_ascii_lowercase();

    if has_launcher_activity(profile) {
        workflows.insert("launch interactive UI from app launcher".to_string());
    }
    if all_text.contains("firebase.messaging") || all_text.contains("c2dm.intent.receive") {
        workflows.insert("receive push messages and react in background".to_string());
    }
    if all_text.contains("boot_completed") {
        workflows.insert("resume scheduled/background work after device boot".to_string());
    }
    if all_text.contains("view") && all_text.contains("browsable") {
        workflows.insert("handle inbound deep links".to_string());
    }
    if signal_call_sites(counts, "network_web") > 0 || !profile.domains.is_empty() {
        workflows.insert("call remote HTTP/TLS services".to_string());
    }
    if signal_call_sites(counts, "database_preferences") > 0 {
        workflows.insert("persist local app state".to_string());
    }
    if signal_call_sites(counts, "location") > 0
        || profile
            .manifest
            .permissions
            .iter()
            .any(|permission| permission.contains("LOCATION"))
    {
        workflows.insert("request and consume device location".to_string());
    }
    if signal_call_sites(counts, "camera_media_capture") > 0 {
        workflows.insert("capture or process camera/media data".to_string());
    }
    if package.contains("fdroid") || all_text.contains("org.fdroid.action.update_repos") {
        workflows.insert("manage package/repository update flows".to_string());
    }
    if package.contains("homeassistant") || all_text.contains("sensor") {
        workflows.insert("sync device sensors or home automation state".to_string());
    }
    if workflows.is_empty() {
        workflows.insert("unknown main workflow; requires dynamic observation".to_string());
    }
    workflows.into_iter().collect()
}

fn reimplementation_steps(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>) -> Vec<String> {
    let mut steps = vec![
        "build a minimal Edgerun UI shell for launcher and deep-link entrypoints".to_string(),
        "replace broad Android permissions with explicit Edgerun capability requests".to_string(),
    ];
    if !profile.domains.is_empty() {
        steps.push("create a constrained network client for observed backend domains".to_string());
    }
    if signal_call_sites(counts, "database_preferences") > 0 {
        steps.push(
            "map SharedPreferences/SQLite usage to an app-private structured store".to_string(),
        );
    }
    if signal_call_sites(counts, "work_background") > 0
        || !profile.manifest.services.is_empty()
        || !profile.manifest.receivers.is_empty()
    {
        steps.push(
            "model services/receivers as explicit scheduled jobs and event subscriptions"
                .to_string(),
        );
    }
    if signal_call_sites(counts, "notifications") > 0 {
        steps.push("map notification flows to an app-scoped notification channel".to_string());
    }
    if signal_call_sites(counts, "crypto_security") > 0 {
        steps.push("isolate key material behind Edgerun secrets/crypto capabilities".to_string());
    }
    if !profile.native_libs.is_empty() {
        steps.push(
            "audit native libraries and either isolate, stub, or replace their behavior"
                .to_string(),
        );
    }
    steps.push(
        "run in Waydroid once to capture actual network, screens, and permission prompts"
            .to_string(),
    );
    steps
}

fn open_questions(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>) -> Vec<String> {
    let mut questions = Vec::new();
    if !profile.domains.is_empty() {
        questions.push(
            "which observed domains are essential backend APIs versus analytics/docs/CDN noise?"
                .to_string(),
        );
    }
    if has_exported_components(profile) {
        questions.push(
            "which exported components are real user workflows versus SDK/system glue?".to_string(),
        );
    }
    if signal_call_sites(counts, "network_web") > 0 {
        questions.push(
            "what authentication flow and token storage does the app actually use?".to_string(),
        );
    }
    if signal_call_sites(counts, "camera_media_capture") > 0 {
        questions.push(
            "is camera/media usage core product behavior or bundled SDK capability?".to_string(),
        );
    }
    if !profile.native_libs.is_empty() {
        questions.push(
            "what does each native library do, and can it be replaced with an Edgerun-native unit?"
                .to_string(),
        );
    }
    if questions.is_empty() {
        questions.push("main behavior is still ambiguous; collect a dynamic run trace".to_string());
    }
    questions
}

fn print_spec_entrypoints(label: &str, components: &[ComponentProfile], top: usize) {
    let exported = components
        .iter()
        .filter(|component| component.exported == Some(true))
        .count();
    println!(
        "    {label}: {} total, {exported} exported",
        components.len()
    );
    for component in components
        .iter()
        .filter(|component| component.exported == Some(true) || !component.actions.is_empty())
        .take(top)
    {
        println!("      {}", component.name);
        for action in component.actions.iter().take(3) {
            println!("        action: {action}");
        }
    }
}

fn print_spec_capabilities(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>, top: usize) {
    let permissions = &profile.manifest.permissions;
    if permissions.iter().any(|p| p.contains("INTERNET"))
        || !profile.domains.is_empty()
        || signal_call_sites(counts, "network_web") > 0
    {
        println!("    network");
        for uri in likely_endpoint_uris(profile).into_iter().take(top) {
            println!("      static_endpoint_uri: {uri}");
        }
        for url in profile.urls.iter().take(top) {
            println!("      static_url: {url}");
        }
        if profile.urls.len() > top {
            println!("      more_urls: {}", profile.urls.len() - top);
        }
        for domain in profile.domains.iter().take(top) {
            println!("      allow_domain: {domain}");
        }
        if profile.domains.len() > top {
            println!("      more_domains: {}", profile.domains.len() - top);
        }
    }
    if permissions.iter().any(|p| p.contains("LOCATION"))
        || signal_call_sites(counts, "location") > 0
    {
        println!(
            "    location: {}",
            if permissions.iter().any(|p| p.contains("FINE_LOCATION")) {
                "precise_or_coarse"
            } else {
                "coarse"
            }
        );
    }
    if signal_call_sites(counts, "files_storage") > 0 {
        println!("    storage: app_private_plus_document_grants");
    }
    if signal_call_sites(counts, "database_preferences") > 0 {
        println!("    local_state: structured_store");
    }
    if signal_call_sites(counts, "notifications") > 0 {
        println!("    notifications: app_scoped_channel");
    }
    if signal_call_sites(counts, "work_background") > 0
        || !profile.manifest.services.is_empty()
        || !profile.manifest.receivers.is_empty()
    {
        println!("    background_tasks: scheduled_jobs_and_event_subscriptions");
    }
    if signal_call_sites(counts, "camera_media_capture") > 0 {
        println!("    camera_media: explicit_sessions");
    }
    if signal_call_sites(counts, "bluetooth_nearby") > 0
        || permissions.iter().any(|p| p.contains("BLUETOOTH"))
    {
        println!("    bluetooth_nearby: scan_connect_allowlist");
    }
    if signal_call_sites(counts, "crypto_security") > 0 {
        println!("    secrets_crypto: keys_certificates_signatures");
    }
    if !profile.native_libs.is_empty() {
        println!("    native_code: isolate_or_replace");
    }
}

fn print_app_signal_evidence(profile: &ApkProfile, top: usize) {
    let owned_packages = likely_owned_packages(profile);
    if owned_packages.is_empty() {
        println!("    <unknown app code ownership>");
        return;
    }
    let mut signal_counts: BTreeMap<&'static str, u64> = BTreeMap::new();
    for ((caller_package, signal), count) in &profile.signal_callers {
        if owned_packages.iter().any(|prefix| {
            caller_package == prefix || caller_package.starts_with(&format!("{prefix}."))
        }) {
            *signal_counts.entry(*signal).or_insert(0) += count;
        }
    }
    if signal_counts.is_empty() {
        println!("    <no app-owned platform calls classified>");
        return;
    }
    let mut rows: Vec<_> = signal_counts.into_iter().collect();
    rows.sort_by(|(left_signal, left), (right_signal, right)| {
        right.cmp(left).then_with(|| left_signal.cmp(right_signal))
    });
    for (signal, count) in rows.into_iter().take(top) {
        println!("    {signal}: {count} app-owned call_sites");
    }
}

fn print_conversion_units(profile: &ApkProfile, top: usize) {
    let mut units = Vec::new();
    collect_component_units(&mut units, "activity", &profile.manifest.activities);
    collect_component_units(&mut units, "service", &profile.manifest.services);
    collect_component_units(&mut units, "receiver", &profile.manifest.receivers);
    let owned_packages = likely_owned_packages(profile);

    let mut printed = 0usize;
    for (kind, component) in units {
        if printed >= top {
            break;
        }
        if !owned_packages.is_empty()
            && !owned_packages.iter().any(|prefix| {
                component.name == *prefix || component.name.starts_with(&format!("{prefix}."))
            })
        {
            continue;
        }
        let Some(summary) = profile.class_summaries.get(&component.name) else {
            continue;
        };
        printed += 1;
        println!("    - kind: {kind}");
        println!("      class: {}", component.name);
        println!("      package: {}", summary.package_name);
        println!(
            "      exported: {}",
            component
                .exported
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        );
        println!("      methods_with_code: {}", summary.method_count);
        println!("      platform_call_sites: {}", summary.platform_calls);
        println!("      internal_call_sites: {}", summary.internal_calls);
        print_ranked_inline("signals", &summary.signals, 4);
        print_ranked_inline("depends_on_packages", &summary.callee_packages, 5);
        print_entrypoint_methods(profile, &component.name, 5);
        for action in component.actions.iter().take(3) {
            println!("      trigger_action: {action}");
        }
    }
    if printed == 0 {
        println!("    <no manifest entrypoint classes had directly parsed code>");
    }
    print_reachable_clusters(profile, top);
}

fn print_entrypoint_methods(profile: &ApkProfile, class_name: &str, top: usize) {
    let lifecycle_names = [
        "onCreate",
        "onStart",
        "onResume",
        "onPause",
        "onStop",
        "onDestroy",
        "onNewIntent",
        "onActivityResult",
        "onRequestPermissionsResult",
        "onStartCommand",
        "onBind",
        "onMessageReceived",
        "onReceive",
        "doWork",
        "handleIntent",
        "handleMessage",
    ];
    let mut rows: Vec<_> = profile
        .method_summaries
        .values()
        .filter(|method| method.class_name == class_name)
        .collect();
    rows.sort_by(|left, right| {
        let left_lifecycle = lifecycle_names.contains(&left.method_name.as_str());
        let right_lifecycle = lifecycle_names.contains(&right.method_name.as_str());
        right_lifecycle
            .cmp(&left_lifecycle)
            .then_with(|| {
                (right.platform_calls + right.internal_calls)
                    .cmp(&(left.platform_calls + left.internal_calls))
            })
            .then_with(|| left.method_name.cmp(&right.method_name))
    });
    if rows.is_empty() {
        return;
    }
    println!("      entry_methods");
    for method in rows.into_iter().take(top) {
        println!("        - {}", method.method_name);
        println!("          platform_call_sites: {}", method.platform_calls);
        println!("          internal_call_sites: {}", method.internal_calls);
        print_ranked_inline("signals", &method.signals, 3);
        print_ranked_inline("calls_packages", &method.callee_packages, 4);
    }
}

#[derive(Default)]
struct ReachableCluster {
    classes: u64,
    platform_calls: u64,
    internal_calls: u64,
    signals: BTreeMap<&'static str, u64>,
}

fn print_reachable_clusters(profile: &ApkProfile, top: usize) {
    let reachable = reachable_from_manifest(profile);
    if reachable.is_empty() {
        return;
    }
    let mut clusters: BTreeMap<String, ReachableCluster> = BTreeMap::new();
    for class_name in reachable {
        let Some(summary) = profile.class_summaries.get(&class_name) else {
            continue;
        };
        if is_known_sdk_package(&summary.package_name) {
            continue;
        }
        let cluster = clusters.entry(summary.package_name.clone()).or_default();
        cluster.classes += 1;
        cluster.platform_calls += summary.platform_calls;
        cluster.internal_calls += summary.internal_calls;
        for (signal, count) in &summary.signals {
            *cluster.signals.entry(*signal).or_insert(0) += count;
        }
    }
    let mut rows: Vec<_> = clusters.into_iter().collect();
    rows.sort_by(|(left_pkg, left), (right_pkg, right)| {
        let left_score = left.platform_calls + left.internal_calls + left.classes;
        let right_score = right.platform_calls + right.internal_calls + right.classes;
        right_score
            .cmp(&left_score)
            .then_with(|| left_pkg.cmp(right_pkg))
    });
    if rows.is_empty() {
        return;
    }
    println!("    reachable_clusters");
    for (package, cluster) in rows.into_iter().take(top) {
        println!("      - package: {package}");
        println!("        classes: {}", cluster.classes);
        println!("        platform_call_sites: {}", cluster.platform_calls);
        println!("        internal_call_sites: {}", cluster.internal_calls);
        print_ranked_inline("signals", &cluster.signals, 4);
    }
}

fn collect_component_units<'a>(
    units: &mut Vec<(&'static str, &'a ComponentProfile)>,
    kind: &'static str,
    components: &'a [ComponentProfile],
) {
    for component in components {
        if component.exported == Some(true) || !component.actions.is_empty() {
            units.push((kind, component));
        }
    }
}

fn print_ranked_inline<K: AsRef<str> + Ord>(label: &str, map: &BTreeMap<K, u64>, top: usize) {
    let mut rows: Vec<_> = map.iter().collect();
    rows.sort_by(|(left_key, left), (right_key, right)| {
        right
            .cmp(left)
            .then_with(|| left_key.as_ref().cmp(right_key.as_ref()))
    });
    if rows.is_empty() {
        return;
    }
    println!("      {label}");
    for (key, count) in rows.into_iter().take(top) {
        println!("        {}: {}", key.as_ref(), count);
    }
}

fn package_root(package: &str, parts: usize) -> String {
    package.split('.').take(parts).collect::<Vec<_>>().join(".")
}

fn has_launcher_activity(profile: &ApkProfile) -> bool {
    profile.manifest.activities.iter().any(|activity| {
        activity.actions.contains("android.intent.action.MAIN")
            && activity
                .categories
                .contains("android.intent.category.LAUNCHER")
    })
}

fn has_exported_components(profile: &ApkProfile) -> bool {
    profile
        .manifest
        .activities
        .iter()
        .chain(profile.manifest.services.iter())
        .chain(profile.manifest.receivers.iter())
        .chain(profile.manifest.providers.iter())
        .any(|component| component.exported == Some(true))
}

fn manifest_text(profile: &ApkProfile) -> String {
    let mut out = String::new();
    for component in profile
        .manifest
        .activities
        .iter()
        .chain(profile.manifest.services.iter())
        .chain(profile.manifest.receivers.iter())
        .chain(profile.manifest.providers.iter())
    {
        out.push_str(&component.name);
        out.push('\n');
        for action in &component.actions {
            out.push_str(action);
            out.push('\n');
        }
        for category in &component.categories {
            out.push_str(category);
            out.push('\n');
        }
    }
    for permission in &profile.manifest.permissions {
        out.push_str(permission);
        out.push('\n');
    }
    out
}

fn spec_confidence(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>) -> &'static str {
    let has_manifest = profile.manifest.package_name.is_some();
    let has_code = !profile.dex_packages.is_empty() && !counts.is_empty();
    let has_runtime_gaps = !profile.domains.is_empty() || has_exported_components(profile);
    match (has_manifest, has_code, has_runtime_gaps) {
        (true, true, false) => "medium_static",
        (true, true, true) => "medium_static_needs_dynamic_trace",
        (true, false, _) => "low_manifest_only",
        _ => "low",
    }
}

fn print_components(label: &str, components: &[ComponentProfile], top: usize) {
    println!("{label}");
    println!("  count: {}", components.len());
    let exported = components
        .iter()
        .filter(|component| component.exported == Some(true))
        .count();
    if exported > 0 {
        println!("  exported: {exported}");
    }
    for component in components.iter().take(top) {
        let exported = component
            .exported
            .map(|value| value.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        println!("  {} exported={exported}", component.name);
        for action in component.actions.iter().take(3) {
            println!("    action: {action}");
        }
        for category in component.categories.iter().take(2) {
            println!("    category: {category}");
        }
    }
    if components.len() > top {
        println!("  ... {} more", components.len() - top);
    }
    println!();
}

fn print_likely_behaviors(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>) {
    let mut behaviors = BTreeSet::new();
    let permissions = &profile.manifest.permissions;

    if permissions.iter().any(|p| p.contains("LOCATION"))
        || signal_call_sites(counts, "location") > 0
    {
        behaviors.insert("uses device location");
    }
    if permissions.iter().any(|p| p.contains("CAMERA"))
        || signal_call_sites(counts, "camera_media_capture") > 0
    {
        behaviors.insert("uses camera or media capture/playback APIs");
    }
    if permissions.iter().any(|p| p.contains("CONTACTS"))
        || signal_call_sites(counts, "contacts_calendar") > 0
    {
        behaviors.insert("touches contacts or calendar data");
    }
    if permissions.iter().any(|p| p.contains("BLUETOOTH"))
        || signal_call_sites(counts, "bluetooth_nearby") > 0
    {
        behaviors.insert("uses Bluetooth or nearby-device APIs");
    }
    if permissions.iter().any(|p| p.contains("SMS"))
        || signal_call_sites(counts, "sms_telephony") > 0
    {
        behaviors.insert("uses SMS, phone, or telephony APIs");
    }
    if permissions.iter().any(|p| p.contains("INTERNET"))
        || !profile.domains.is_empty()
        || signal_call_sites(counts, "network_web") > 0
    {
        behaviors.insert("talks to network services");
    }
    if permissions.iter().any(|p| p.contains("NOTIFICATION"))
        || signal_call_sites(counts, "notifications") > 0
    {
        behaviors.insert("posts or manages notifications");
    }
    if signal_call_sites(counts, "database_preferences") > 0 {
        behaviors.insert("stores structured local state");
    }
    if signal_call_sites(counts, "work_background") > 0
        || !profile.manifest.services.is_empty()
        || !profile.manifest.receivers.is_empty()
    {
        behaviors.insert("runs background work or reacts to system events");
    }
    if signal_call_sites(counts, "crypto_security") > 0 {
        behaviors.insert("uses cryptography, certificates, or key storage");
    }
    if !profile.native_libs.is_empty() {
        behaviors.insert("loads native code");
    }

    println!("likely_behavior");
    for behavior in behaviors {
        println!("  {behavior}");
    }
}

fn print_capability_candidates(profile: &ApkProfile, counts: &BTreeMap<ApiCall, u64>, top: usize) {
    let permissions = &profile.manifest.permissions;
    println!("edgerun_capability_candidates");

    if permissions.iter().any(|p| p.contains("INTERNET"))
        || !profile.domains.is_empty()
        || signal_call_sites(counts, "network_web") > 0
    {
        let sample = profile
            .domains
            .iter()
            .take(top.min(5))
            .cloned()
            .collect::<Vec<_>>()
            .join(", ");
        if sample.is_empty() {
            println!("  network: constrained outbound HTTP/TLS");
        } else {
            println!("  network: constrained outbound HTTP/TLS to {sample}");
        }
    }
    if permissions.iter().any(|p| p.contains("LOCATION"))
        || signal_call_sites(counts, "location") > 0
    {
        let precise = permissions.iter().any(|p| p.contains("FINE_LOCATION"));
        println!(
            "  location: {}",
            if precise {
                "precise or coarse"
            } else {
                "coarse"
            }
        );
    }
    if permissions.iter().any(|p| p.contains("CAMERA"))
        || signal_call_sites(counts, "camera_media_capture") > 0
    {
        println!("  camera_media: explicit capture/playback sessions");
    }
    if permissions.iter().any(|p| p.contains("BLUETOOTH"))
        || signal_call_sites(counts, "bluetooth_nearby") > 0
    {
        println!("  bluetooth_nearby: scan/connect with device allowlist");
    }
    if permissions.iter().any(|p| p.contains("CONTACTS"))
        || signal_call_sites(counts, "contacts_calendar") > 0
    {
        println!("  contacts_calendar: selected records only");
    }
    if permissions.iter().any(|p| p.contains("NOTIFICATION"))
        || signal_call_sites(counts, "notifications") > 0
    {
        println!("  notifications: app-scoped notification channel");
    }
    if signal_call_sites(counts, "files_storage") > 0 {
        println!("  storage: app-private files plus explicit document grants");
    }
    if signal_call_sites(counts, "database_preferences") > 0 {
        println!("  local_state: app-private structured store");
    }
    if signal_call_sites(counts, "work_background") > 0
        || !profile.manifest.services.is_empty()
        || !profile.manifest.receivers.is_empty()
    {
        println!("  background_tasks: scheduled jobs and declared event triggers");
    }
    if signal_call_sites(counts, "crypto_security") > 0 {
        println!("  secrets_crypto: key store, signatures, certificates");
    }
    if !profile.native_libs.is_empty() {
        println!("  native_code: isolate or replace native library behavior");
    }
    println!();
}

fn signal_call_sites(counts: &BTreeMap<ApiCall, u64>, name: &str) -> u64 {
    let Some(signal) = signals().iter().find(|signal| signal.name == name) else {
        return 0;
    };
    counts
        .iter()
        .filter(|(api, _)| matches_signal(api, signal))
        .map(|(_, count)| *count)
        .sum()
}

#[derive(Clone, Copy, Debug, Default)]
struct SummaryCount {
    signatures: u64,
    call_sites: u64,
}

fn add_summary_count(map: &mut BTreeMap<String, SummaryCount>, key: String, call_sites: u64) {
    let entry = map.entry(key).or_default();
    entry.signatures += 1;
    entry.call_sites += call_sites;
}

fn add_summary_count_by_ref(
    map: &mut BTreeMap<&'static str, SummaryCount>,
    key: &'static str,
    call_sites: u64,
) {
    let entry = map.entry(key).or_default();
    entry.signatures += 1;
    entry.call_sites += call_sites;
}

fn print_ranked<K: AsRef<str> + Ord>(label: &str, map: &BTreeMap<K, SummaryCount>, top: usize) {
    println!("{label}");
    let mut rows: Vec<_> = map.iter().collect();
    rows.sort_by(|(left_key, left), (right_key, right)| {
        right
            .call_sites
            .cmp(&left.call_sites)
            .then_with(|| right.signatures.cmp(&left.signatures))
            .then_with(|| left_key.as_ref().cmp(right_key.as_ref()))
    });
    for (key, count) in rows.into_iter().take(top) {
        println!(
            "  {}\t{} call_sites\t{} signatures",
            key.as_ref(),
            count.call_sites,
            count.signatures
        );
    }
    println!();
}

fn package_name(class_name: &str) -> String {
    let mut parts = class_name.rsplitn(2, '.');
    let _class = parts.next();
    parts.next().unwrap_or(class_name).to_string()
}

fn method_key(api: &ApiCall) -> String {
    format!("{}->{}", api.class_name, api.method_name)
}

fn is_interesting_api(api: &ApiCall) -> bool {
    !is_boilerplate_class(&api.class_name) && !is_boilerplate_method(api)
}

fn is_boilerplate_class(class_name: &str) -> bool {
    const PREFIXES: &[&str] = &[
        "java.lang.",
        "java.util.",
        "kotlin.",
        "kotlinx.",
        "android.animation.",
        "android.graphics.",
        "android.text.",
        "android.util.",
        "android.view.",
        "android.widget.",
        "androidx.compose.",
    ];
    const EXACT: &[&str] = &[
        "android.os.Bundle",
        "android.os.Handler",
        "android.os.Looper",
        "android.os.Message",
        "android.os.Parcel",
        "android.os.Parcelable",
        "android.os.Parcelable$Creator",
        "android.os.SystemClock",
    ];
    PREFIXES.iter().any(|prefix| class_name.starts_with(prefix)) || EXACT.contains(&class_name)
}

fn is_boilerplate_method(api: &ApiCall) -> bool {
    matches!(
        api.method_name.as_str(),
        "<init>" | "equals" | "hashCode" | "toString"
    )
}

fn matches_signal(api: &ApiCall, signal: &Signal) -> bool {
    signal
        .prefixes
        .iter()
        .any(|prefix| api.class_name.starts_with(prefix))
        || signal
            .keywords
            .iter()
            .any(|keyword| api.class_name.contains(keyword) || api.method_name.contains(keyword))
}

fn signals() -> &'static [Signal] {
    &[
        Signal {
            name: "accounts_identity",
            prefixes: &["android.accounts.", "android.credentials."],
            keywords: &["Account", "Credential", "Auth", "Login", "SignIn"],
        },
        Signal {
            name: "activity_ui",
            prefixes: &[
                "android.app.",
                "android.view.",
                "android.widget.",
                "android.window.",
            ],
            keywords: &["Activity", "Dialog", "Fragment", "Window", "View"],
        },
        Signal {
            name: "bluetooth_nearby",
            prefixes: &["android.bluetooth.", "android.companion."],
            keywords: &["Bluetooth", "Ble", "CompanionDevice"],
        },
        Signal {
            name: "camera_media_capture",
            prefixes: &["android.hardware.camera", "android.media."],
            keywords: &["Camera", "MediaRecorder", "AudioRecord", "Microphone"],
        },
        Signal {
            name: "contacts_calendar",
            prefixes: &["android.provider.Contacts", "android.provider.Calendar"],
            keywords: &["Contacts", "Calendar"],
        },
        Signal {
            name: "crypto_security",
            prefixes: &["javax.crypto.", "java.security.", "android.security."],
            keywords: &["Cipher", "KeyStore", "Certificate", "Signature", "Crypto"],
        },
        Signal {
            name: "database_preferences",
            prefixes: &["android.database.", "android.preference."],
            keywords: &["SQLite", "SharedPreferences", "Preference"],
        },
        Signal {
            name: "files_storage",
            prefixes: &[
                "android.provider.Documents",
                "android.provider.MediaStore",
                "java.io.",
                "java.nio.file.",
            ],
            keywords: &["File", "Storage", "Document", "MediaStore"],
        },
        Signal {
            name: "location",
            prefixes: &["android.location."],
            keywords: &["Location", "Gps", "Gnss", "Geocoder"],
        },
        Signal {
            name: "network_web",
            prefixes: &[
                "android.net.",
                "android.webkit.",
                "java.net.",
                "javax.net.",
                "org.apache.http.",
            ],
            keywords: &["Socket", "Http", "Url", "Network", "WebView"],
        },
        Signal {
            name: "notifications",
            prefixes: &["android.app.Notification", "android.service.notification."],
            keywords: &["Notification"],
        },
        Signal {
            name: "package_intents",
            prefixes: &["android.content.", "android.content.pm."],
            keywords: &["Intent", "PackageManager", "BroadcastReceiver"],
        },
        Signal {
            name: "sensors",
            prefixes: &["android.hardware."],
            keywords: &["Sensor", "Vibrator", "Nfc", "Fingerprint", "Biometric"],
        },
        Signal {
            name: "sms_telephony",
            prefixes: &["android.telephony.", "android.provider.Telephony"],
            keywords: &["Sms", "Mms", "Telephony", "PhoneNumber"],
        },
        Signal {
            name: "work_background",
            prefixes: &["android.app.job.", "android.app.usage.", "android.os."],
            keywords: &[
                "JobScheduler",
                "AlarmManager",
                "WakeLock",
                "Work",
                "Service",
            ],
        },
    ]
}

fn signal_description(name: &str) -> &'static str {
    match name {
        "accounts_identity" => "accounts, credentials, login, or authentication APIs",
        "activity_ui" => "Android UI, activity, dialog, window, or view behavior",
        "bluetooth_nearby" => "Bluetooth or nearby-device discovery/connect behavior",
        "camera_media_capture" => "camera, microphone, recorder, or media capture behavior",
        "contacts_calendar" => "contacts or calendar provider access",
        "crypto_security" => "key, certificate, cipher, signature, or keystore behavior",
        "database_preferences" => "local database or preference storage behavior",
        "files_storage" => "file, document, media store, or filesystem behavior",
        "location" => "location, GNSS/GPS, geocoder, or map-position behavior",
        "network_web" => "network, socket, HTTP, TLS, or WebView behavior",
        "notifications" => "notification posting, channels, or listener behavior",
        "package_intents" => "intent, package manager, broadcast, or cross-app behavior",
        "sensors" => "sensors, haptics, NFC, fingerprint, or biometric behavior",
        "sms_telephony" => "SMS, MMS, phone, carrier, or telephony behavior",
        "work_background" => "jobs, alarms, wake locks, services, or background work",
        _ => "matched Android API capability signal",
    }
}

fn capability_intent_call(signal: &str) -> Option<(&'static str, &'static str)> {
    match signal {
        "accounts_identity" => Some(("accounts_identity", "authenticate_or_load_account")),
        "network_web" => Some(("network", "request_network_or_webview")),
        "location" => Some(("location", "request_location")),
        "notifications" => Some(("notifications", "post_or_update_notification")),
        "files_storage" => Some(("storage", "read_or_write_app_file")),
        "database_preferences" => Some(("local_state", "read_or_write_local_state")),
        "camera_media_capture" => Some(("camera_media", "capture_or_process_media")),
        "bluetooth_nearby" => Some(("bluetooth_nearby", "scan_or_connect_device")),
        "crypto_security" => Some(("secrets_crypto", "use_key_or_crypto_operation")),
        "contacts_calendar" => Some(("contacts_calendar", "read_or_write_contact_calendar")),
        "package_intents" => Some(("app_events", "send_or_receive_app_event")),
        "sensors" => Some(("sensors", "read_sensor_or_biometric")),
        "sms_telephony" => Some(("sms_telephony", "use_sms_or_phone_state")),
        "work_background" => Some(("background_tasks", "schedule_or_handle_background_work")),
        _ => None,
    }
}

fn json_escape(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn read_string(data: &[u8], off: usize, len: usize) -> io::Result<String> {
    checked_range(data, off, len)?;
    Ok(String::from_utf8_lossy(&data[off..off + len]).into_owned())
}

fn read_u16(data: &[u8], off: usize) -> io::Result<u16> {
    checked_range(data, off, 2)?;
    Ok(u16::from_le_bytes([data[off], data[off + 1]]))
}

fn read_u32(data: &[u8], off: usize) -> io::Result<u32> {
    checked_range(data, off, 4)?;
    Ok(u32::from_le_bytes([
        data[off],
        data[off + 1],
        data[off + 2],
        data[off + 3],
    ]))
}

fn checked_range(data: &[u8], off: usize, len: usize) -> io::Result<()> {
    if off.checked_add(len).is_some_and(|end| end <= data.len()) {
        Ok(())
    } else {
        Err(invalid("input range is out of bounds"))
    }
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.into())
}
