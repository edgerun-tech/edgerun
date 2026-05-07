#![cfg(feature = "std")]

use edgerun_crypto::ed25519_dalek::{Signature, Verifier, VerifyingKey};
use edgerun_crypto::{Ed25519SigningKey as SigningKey, Signer};
use edgerun_protocols::seal::{seal_with_key, unseal_with_key, SealKey};
use edgerun_protocols::wire as edgerun_wire;
use edgerun_protocols::wire::{
    sdk_wire_bytes, CapabilityResponseProofRecord, SdkWireRecord, SigningAlgorithmRecord,
    StorageWriteReceiptRecord, UserProfileIdSeedRecord,
};
use edgerun_sdk::{
    sha256, sha256_hex, str_eq, ApiFunction, ChainManifest, CompositionComponent,
    CompositionManifest, Determinism, SegmentManifest, UnitManifest, SDK_ABI_NAME,
};
use edgerun_ssh::SshTarget;
use formats::{
    bytes_to_hex, parse_api, parse_chain, parse_composition, parse_report, parse_segment,
    parse_segment_report,
};
use std::env;
use std::ffi::{CStr, CString};
use std::fs;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
mod formats;

struct RuntimeIndex {
    units: Vec<UnitManifest>,
    compositions: Vec<CompositionManifest>,
    segments: Vec<SegmentManifest>,
    chains: Vec<ChainManifest>,
}

static INDEX: OnceLock<RuntimeIndex> = OnceLock::new();

fn main() {
    let mut args = env::args().skip(1);
    let code = match args.next().as_deref() {
        Some("list") => cmd_list(),
        Some("verify") => cmd_verify(args.next().as_deref()),
        Some("verify-chain") => cmd_verify_chain(args.next().as_deref()),
        Some("verify-segment") => cmd_verify_segment(args.next().as_deref()),
        Some("explain") => cmd_explain(args.next().as_deref()),
        Some("run-segment") => cmd_run_segment(args.collect()),
        Some("run-composition") => cmd_run_composition(args.collect()),
        Some("quote-composition") => cmd_quote_composition(args.collect()),
        Some("preflight-composition") => cmd_preflight_composition(args.collect()),
        Some("verify-report") => cmd_verify_report(args.next().as_deref()),
        Some("verify-segment-report") => cmd_verify_segment_report(args.next().as_deref()),
        Some("replay-segment-report") => cmd_replay_segment_report(args.collect()),
        Some("sign-segment-report") => cmd_sign_segment_report(args.collect()),
        Some("write-signer-policy") => cmd_write_signer_policy(args.collect()),
        Some("verify-signed-segment-report") => cmd_verify_signed_segment_report(args.collect()),
        Some("verify-chain-reports") => cmd_verify_chain_reports(args.collect()),
        Some("bench-unit") => cmd_bench_unit(args.collect()),
        Some("package-app") => cmd_package_app(args.collect()),
        Some("sign-app") => cmd_sign_app(args.collect()),
        Some("verify-signed-app") => cmd_verify_signed_app(args.collect()),
        Some("issue-product") => cmd_issue_product(args.collect()),
        Some("verify-product") => cmd_verify_product(args.collect()),
        Some("issue-entitlement") => cmd_issue_entitlement(args.collect()),
        Some("verify-entitlement") => cmd_verify_entitlement(args.collect()),
        Some("issue-settlement") => cmd_issue_settlement(args.collect()),
        Some("verify-settlement") => cmd_verify_settlement(args.collect()),
        Some("issue-payment-intent") => cmd_issue_payment_intent(args.collect()),
        Some("settle-payment") => cmd_settle_payment(args.collect()),
        Some("verify-payment") => cmd_verify_payment(args.collect()),
        Some("write-trust-policy") => cmd_write_trust_policy(args.collect()),
        Some("verify-trusted-app") => cmd_verify_trusted_app(args.collect()),
        Some("verify-trusted-product") => cmd_verify_trusted_product(args.collect()),
        Some("verify-trusted-entitlement") => cmd_verify_trusted_entitlement(args.collect()),
        Some("verify-trusted-payment") => cmd_verify_trusted_payment(args.collect()),
        Some("verify-trusted-settlement") => cmd_verify_trusted_settlement(args.collect()),
        Some("write-revocation") => cmd_write_revocation(args.collect()),
        Some("verify-revocation") => cmd_verify_revocation(args.collect()),
        Some("write-capability-request") => cmd_write_capability_request(args.collect()),
        Some("verify-capability-response") => cmd_verify_capability_response(args.collect()),
        Some("write-sign-request") => cmd_write_sign_request(args.collect()),
        Some("sign-request") => cmd_sign_request(args.collect()),
        Some("sign-request-authorized") => cmd_sign_request_authorized(args.collect()),
        Some("verify-sign-response") => cmd_verify_sign_response(args.collect()),
        Some("write-seal-request") => cmd_write_seal_request(args.collect()),
        Some("seal-request") => cmd_seal_request(args.collect()),
        Some("seal-request-authorized") => cmd_seal_request_authorized(args.collect()),
        Some("write-unseal-request") => cmd_write_unseal_request(args.collect()),
        Some("unseal-request") => cmd_unseal_request(args.collect()),
        Some("unseal-request-authorized") => cmd_unseal_request_authorized(args.collect()),
        Some("verify-seal-response") => cmd_verify_seal_response(args.collect()),
        Some("write-storage-read-request") => cmd_write_storage_read_request(args.collect()),
        Some("write-storage-write-request") => cmd_write_storage_write_request(args.collect()),
        Some("storage-read-request") => cmd_storage_read_request(args.collect()),
        Some("storage-read-request-authorized") => {
            cmd_storage_read_request_authorized(args.collect())
        }
        Some("storage-write-request") => cmd_storage_write_request(args.collect()),
        Some("storage-write-request-authorized") => {
            cmd_storage_write_request_authorized(args.collect())
        }
        Some("verify-storage-response") => cmd_verify_storage_response(args.collect()),
        Some("create-user-profile") => cmd_create_user_profile(args.collect()),
        Some("grant-profile-capability") => cmd_grant_profile_capability(args.collect()),
        Some("open-user-profile") => cmd_open_user_profile(args.collect()),
        Some("verify-profile-access") => cmd_verify_profile_access(args.collect()),
        Some("generate-unit-metadata") => cmd_generate_unit_metadata(args.next().as_deref()),
        Some("deploy-inventory") => cmd_deploy_inventory(args.collect()),
        Some("deploy-server") => cmd_deploy_server(args.collect()),
        Some("deploy-ssh-probe") => cmd_deploy_ssh_probe(args.collect()),
        Some("deploy-ssh-exec") => cmd_deploy_ssh_exec(args.collect()),
        Some("build-artifacts") => cmd_build_artifacts(),
        _ => {
            eprintln!(
                "usage: edgerun-sdk <...>|write-revocation <out.erev> <kind> <target-hex> <issuer-seed-hex> <issued-at> [reason]|verify-revocation <revocation.erev>|verify-trusted-* ... [revocation.erev]"
            );
            1
        }
    };
    std::process::exit(code);
}

fn cmd_deploy_inventory(args: Vec<String>) -> i32 {
    if args.is_empty() || args.first().map(String::as_str) == Some("--local") {
        let report = edgerun_deploy::host::capability_report_local();
        println!("target=local");
        print!(
            "{}",
            edgerun_machine_report::render_machine_capability_report(&report)
        );
        return 0;
    }
    if args.first().map(String::as_str) == Some("--ssh") {
        let Some(target) = args.get(1) else {
            eprintln!("deploy-inventory --ssh requires user@host[:port]");
            return 1;
        };
        return cmd_deploy_ssh_inventory(target);
    }
    eprintln!("usage: edgerun-sdk deploy-inventory --local|--ssh user@host[:port]");
    1
}

fn cmd_deploy_ssh_inventory(target: &str) -> i32 {
    let target = match SshTarget::parse(target) {
        Ok(target) => target,
        Err(err) => {
            eprintln!("bad ssh target: {err}");
            return 1;
        }
    };
    let key_path = default_ssh_key_path();
    let machine_report = match build_machine_report_for_remote() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("machine-report build failed: {err}");
            return 1;
        }
    };
    let machine_report_bytes = match fs::read(&machine_report) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!(
                "failed to read machine-report binary {}: {err}",
                machine_report.display()
            );
            return 1;
        }
    };
    match edgerun_ssh::host::exec_with_ed25519_key_file_and_input(
        &target,
        &key_path,
        &remote_machine_report_inventory_sh(&target.host),
        &machine_report_bytes,
        Duration::from_secs(30),
    ) {
        Ok(result) => {
            if !result.stderr.is_empty() {
                eprint!("{}", String::from_utf8_lossy(&result.stderr));
            }
            match result.exit_status {
                Some(0) => {
                    let stdout = String::from_utf8_lossy(&result.stdout);
                    println!("target={}@{}:{}", target.user, target.host, target.port);
                    print!("{stdout}");
                    if !stdout.ends_with('\n') {
                        println!();
                    }
                    0
                }
                Some(code) => {
                    eprintln!("remote inventory exited with status {code}");
                    1
                }
                None => 0,
            }
        }
        Err(err) => {
            eprintln!("ssh inventory failed: {err}");
            1
        }
    }
}

fn cmd_deploy_ssh_probe(args: Vec<String>) -> i32 {
    let Some(target) = args.first() else {
        eprintln!("deploy-ssh-probe requires user@host[:port]");
        return 1;
    };
    let target = match SshTarget::parse(target) {
        Ok(target) => target,
        Err(err) => {
            eprintln!("bad ssh target: {err}");
            return 1;
        }
    };
    match edgerun_ssh::host::probe_server(&target, Duration::from_secs(5)) {
        Ok(probe) => {
            println!("target={}@{}:{}", target.user, target.host, target.port);
            println!("transport=ssh");
            println!("client_identification={}", probe.identification.client);
            println!("server_identification={}", probe.identification.server);
            println!(
                "kex_algorithms={}",
                probe.server_kex.kex_algorithms.join(",")
            );
            println!(
                "host_key_algorithms={}",
                probe.server_kex.server_host_key_algorithms.join(",")
            );
            println!(
                "c2s_encryption={}",
                probe
                    .server_kex
                    .encryption_algorithms_client_to_server
                    .join(",")
            );
            println!(
                "s2c_encryption={}",
                probe
                    .server_kex
                    .encryption_algorithms_server_to_client
                    .join(",")
            );
            println!(
                "c2s_mac={}",
                probe.server_kex.mac_algorithms_client_to_server.join(",")
            );
            println!(
                "s2c_mac={}",
                probe.server_kex.mac_algorithms_server_to_client.join(",")
            );
            println!(
                "c2s_compression={}",
                probe
                    .server_kex
                    .compression_algorithms_client_to_server
                    .join(",")
            );
            println!(
                "s2c_compression={}",
                probe
                    .server_kex
                    .compression_algorithms_server_to_client
                    .join(",")
            );
            println!("remote_inventory=requires_ssh_auth_exec");
            0
        }
        Err(err) => {
            eprintln!("ssh probe failed: {err}");
            1
        }
    }
}

fn cmd_deploy_ssh_exec(args: Vec<String>) -> i32 {
    let Some(target) = args.first() else {
        eprintln!("deploy-ssh-exec requires user@host[:port] command...");
        return 1;
    };
    if args.len() < 2 {
        eprintln!("deploy-ssh-exec requires user@host[:port] command...");
        return 1;
    }
    let target = match SshTarget::parse(target) {
        Ok(target) => target,
        Err(err) => {
            eprintln!("bad ssh target: {err}");
            return 1;
        }
    };
    let command = args[1..].join(" ");
    let key_path = default_ssh_key_path();
    match edgerun_ssh::host::exec_with_ed25519_key_file(
        &target,
        &key_path,
        &command,
        Duration::from_secs(10),
    ) {
        Ok(result) => {
            print!("{}", String::from_utf8_lossy(&result.stdout));
            eprint!("{}", String::from_utf8_lossy(&result.stderr));
            result.exit_status.unwrap_or(0) as i32
        }
        Err(err) => {
            eprintln!("ssh exec failed: {err}");
            1
        }
    }
}

fn cmd_deploy_server(args: Vec<String>) -> i32 {
    let Some(target) = args.first() else {
        eprintln!("deploy-server requires user@host[:port]");
        return 1;
    };
    let target = match SshTarget::parse(target) {
        Ok(target) => target,
        Err(err) => {
            eprintln!("bad ssh target: {err}");
            return 1;
        }
    };
    let key_path = default_ssh_key_path();
    let server = match build_edgerun_server_for_remote() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("server build failed: {err}");
            return 1;
        }
    };
    let server_bytes = match fs::read(&server) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("failed to read server binary {}: {err}", server.display());
            return 1;
        }
    };
    match edgerun_ssh::host::exec_with_ed25519_key_file_and_input(
        &target,
        &key_path,
        REMOTE_DEPLOY_SERVER_SH,
        &server_bytes,
        Duration::from_secs(60),
    ) {
        Ok(result) => {
            print!("{}", String::from_utf8_lossy(&result.stdout));
            eprint!("{}", String::from_utf8_lossy(&result.stderr));
            result.exit_status.unwrap_or(1) as i32
        }
        Err(err) => {
            eprintln!("deploy-server failed: {err}");
            1
        }
    }
}

fn default_ssh_key_path() -> PathBuf {
    if let Ok(path) = env::var("EDGERUN_SSH_KEY") {
        return PathBuf::from(path);
    }
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_owned());
    Path::new(&home).join(".ssh/id_ed25519")
}

fn build_edgerun_server_for_remote() -> Result<PathBuf, String> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "cannot resolve workspace root".to_owned())?;
    let target = "x86_64-unknown-linux-musl";
    let status = Command::new("cargo")
        .current_dir(workspace_root)
        .args([
            "build",
            "-q",
            "-p",
            "edgerun-node",
            "--bin",
            "edgerun-server",
            "--no-default-features",
            "--features",
            "std,smtp,imap,dns,tls,acme",
            "--release",
            "--target",
            target,
        ])
        .status()
        .map_err(|err| err.to_string())?;
    if !status.success() {
        return Err(format!("cargo build exited with {status}"));
    }
    let relative = Path::new(target).join("release").join("edgerun-server");
    let mut candidates = Vec::new();
    if let Ok(target_dir) = env::var("CARGO_TARGET_DIR") {
        candidates.push(PathBuf::from(target_dir).join(&relative));
    }
    candidates.push(workspace_root.join("target").join(&relative));
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| "built edgerun-server artifact not found".to_owned())
}

fn build_machine_report_for_remote() -> Result<PathBuf, String> {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "cannot resolve workspace root".to_owned())?;
    let target = "x86_64-unknown-linux-musl";
    let status = Command::new("cargo")
        .current_dir(workspace_root)
        .args([
            "build",
            "-q",
            "-p",
            "edgerun-machine-report",
            "--features",
            "linux-bin",
            "--release",
            "--target",
            target,
        ])
        .status()
        .map_err(|err| err.to_string())?;
    if !status.success() {
        return Err(format!("cargo build exited with {status}"));
    }
    let target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("target"));
    Ok(target_dir
        .join(target)
        .join("release")
        .join("edgerun-machine-report"))
}

fn remote_machine_report_inventory_sh(host: &str) -> String {
    let mut script = String::from(REMOTE_MACHINE_REPORT_INVENTORY_SH);
    script.push_str("\"$path\" capability-report \"$EDGERUN_REPORT_HOST\"\n");
    script.replace("EDGERUN_REPORT_HOST_PLACEHOLDER", &shell_single_quote(host))
}

fn shell_single_quote(value: &str) -> String {
    let mut out = String::from("'");
    for ch in value.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

const REMOTE_MACHINE_REPORT_INVENTORY_SH: &str = r#"set -eu
EDGERUN_REPORT_HOST=EDGERUN_REPORT_HOST_PLACEHOLDER
path="/tmp/edgerun-machine-report-$$"
cleanup() { rm -f "$path"; }
trap cleanup EXIT HUP INT TERM
cat > "$path"
chmod 700 "$path"
"#;

const REMOTE_DEPLOY_SERVER_SH: &str = r#"set -eu
tmp="/tmp/edgerun-server-new-$$"
rollback="/opt/edgerun/rollback/$(date +%Y%m%d%H%M%S)"
cleanup() { rm -f "$tmp"; }
trap cleanup EXIT HUP INT TERM
cat > "$tmp"
chmod 755 "$tmp"
echo "== compiled plan =="
"$tmp" --print-compiled-plan
echo "== backup =="
mkdir -p "$rollback"
[ ! -e /usr/local/bin/edgerun-server ] || cp -p /usr/local/bin/edgerun-server "$rollback/edgerun-server"
[ ! -e /etc/init.d/edgerun-server ] || cp -p /etc/init.d/edgerun-server "$rollback/edgerun-server.initd"
[ ! -e /etc/edgerun/server/server.yaml ] || cp -p /etc/edgerun/server/server.yaml "$rollback/server.yaml"
echo "rollback=$rollback"
echo "== stop old service =="
rc-service edgerun-server stop || true
echo "== install =="
install -d -m 0750 /etc/edgerun/server
install -d -m 0750 /etc/edgerun/server/tls
install -d -m 0750 /var/lib/edgerun/mail/maildirs
install -d -m 0750 /var/lib/edgerun/mail/queue
install -d -m 0750 /var/lib/edgerun/.edgerun
install -m 0755 "$tmp" /usr/local/bin/edgerun-server
cat > /etc/init.d/edgerun-server <<'EDGERUN_INIT'
#!/sbin/openrc-run
name="Edgerun multi-protocol server"
description="Edgerun DNS/HTTP/SMTP/IMAP server"
command="/usr/local/bin/edgerun-server"
command_background="yes"
pidfile="/run/edgerun-server.pid"
output_log="/var/log/edgerun-server.log"
error_log="/var/log/edgerun-server.log"
rc_ulimit="-n 65536"
export EDGERUN_RT_WORKER_THREADS="1"

extra_started_commands="health plan"
description_health="Run edgerun-server health check"
description_plan="Print compiled deployment plan"

depend() {
    need net
    after firewall
}

start_pre() {
    checkpath -d -m 0750 /etc/edgerun/server
    checkpath -d -m 0750 /etc/edgerun/server/tls
    checkpath -d -m 0750 /var/lib/edgerun/mail
    checkpath -d -m 0750 /var/lib/edgerun/mail/maildirs
    checkpath -d -m 0750 /var/lib/edgerun/mail/queue
    checkpath -d -m 0750 /var/lib/edgerun/.edgerun
    checkpath -f -m 0644 /var/log/edgerun-server.log
}

health() {
    /usr/local/bin/edgerun-server --health-check
}

plan() {
    /usr/local/bin/edgerun-server --print-compiled-plan
}
EDGERUN_INIT
chmod 755 /etc/init.d/edgerun-server
rc-update add edgerun-server default || true
echo "== start =="
rc-service edgerun-server start
sleep 2
echo "== status =="
rc-service edgerun-server status || true
echo "== health =="
/usr/local/bin/edgerun-server --health-check || true
echo "== listeners =="
netstat -lntup 2>/dev/null | grep -E ':(25|53|80|143)[[:space:]]' || true
"#;

fn cmd_build_artifacts() -> i32 {
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

fn cmd_generate_unit_metadata(id: Option<&str>) -> i32 {
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

fn cmd_bench_unit(args: Vec<String>) -> i32 {
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

fn cmd_package_app(args: Vec<String>) -> i32 {
    let app_id = args
        .first()
        .map(String::as_str)
        .unwrap_or("hmac-sha256-rfc2104-composed");
    if let Err(err) = build_artifacts() {
        eprintln!("package-app build failed: {err}");
        return 1;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("dist").join(format!("{app_id}-app")));
    let result = match app_id {
        "sha256-fips180" => package_sha256_browser_app(&root, &out_dir),
        "hmac-sha256-rfc2104-composed" => package_hmac_browser_app(&root, &out_dir),
        _ => Err(format!("unsupported app package: {app_id}")),
    };
    match result {
        Ok(()) => {
            println!("app: {app_id}");
            println!("path: {}", out_dir.display());
            println!("entry: {}", out_dir.join("index.html").display());
            println!("artifact_graph: {}", out_dir.join("app.eapp").display());
            0
        }
        Err(err) => {
            eprintln!("package-app failed: {err}");
            1
        }
    }
}

fn package_hmac_browser_app(root: &Path, out_dir: &Path) -> Result<(), String> {
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
        out_dir.join("app.edapp"),
        hmac_browser_app_manifest(composition, &composition_graph, &unit_json),
    )
    .map_err(|err| err.to_string())?;
    write_packaged_app_graph(out_dir, "hmac-sha256-rfc2104-composed")?;
    fs::write(out_dir.join("index.html"), HMAC_BROWSER_INDEX).map_err(|err| err.to_string())?;
    fs::write(
        out_dir.join("edgerun-browser.js"),
        COMPOSITION_BROWSER_RUNNER,
    )
    .map_err(|err| err.to_string())?;
    Ok(())
}

fn package_sha256_browser_app(root: &Path, out_dir: &Path) -> Result<(), String> {
    let unit = unit("sha256-fips180").ok_or_else(|| "missing sha256-fips180 unit".to_owned())?;
    fs::create_dir_all(out_dir).map_err(|err| err.to_string())?;
    let unit_json = package_unit_for_app(root, out_dir, unit)?;
    fs::write(
        out_dir.join("app.edapp"),
        sha256_browser_app_manifest(&unit_json),
    )
    .map_err(|err| err.to_string())?;
    write_packaged_app_graph(out_dir, "sha256-fips180")?;
    fs::write(out_dir.join("index.html"), SHA256_BROWSER_INDEX).map_err(|err| err.to_string())?;
    fs::write(out_dir.join("edgerun-browser.js"), SHA256_BROWSER_RUNNER)
        .map_err(|err| err.to_string())?;
    Ok(())
}

fn package_unit_for_app(
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

fn browser_api_functions_json(api: &edgerun_wire::UnitApi) -> String {
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

fn json_u8_array(bytes: &[u8]) -> String {
    let values = bytes
        .iter()
        .map(|byte| byte.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{values}]")
}

fn json_escape_bytes(bytes: &[u8]) -> String {
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

const EAPP_DOMAIN_APP_ID: &[u8] = b"edgerun-sdk.eapp.v1.app-id";
const EAPP_DEVELOPER_DOMAIN: &[u8] = b"edgerun-sdk.esig.v1.app.developer";
const EAPP_STORE_DOMAIN: &[u8] = b"edgerun-sdk.esig.v1.app.store";
const EPRD_DEVELOPER_DOMAIN: &[u8] = b"edgerun-sdk.eprd.v1.developer-product";
const EPRD_STORE_DOMAIN: &[u8] = b"edgerun-sdk.eprd.v1.store-product";
const EENT_STORE_DOMAIN: &[u8] = b"edgerun-sdk.eent.v1.store-entitlement";
const ESET_STORE_DOMAIN: &[u8] = b"edgerun-sdk.eset.v1.store-settlement";
const EPAY_PAYER_DOMAIN: &[u8] = b"edgerun-sdk.epay.v1.payer-intent";
const EPAY_STORE_DOMAIN: &[u8] = b"edgerun-sdk.epay.v1.store-settlement";
const EREV_DOMAIN: &[u8] = b"edgerun-sdk.erev.v1.revocation";
const CAPABILITY_RESPONSE_DOMAIN: &[u8] = b"edgerun-sdk.rkyv.v1.capability-response";
const EUPB_DOMAIN: &[u8] = b"edgerun-sdk.eupb.v1.user-profile-body";

fn cmd_sign_app(args: Vec<String>) -> i32 {
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

fn cmd_verify_signed_app(args: Vec<String>) -> i32 {
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

fn write_packaged_app_graph(out_dir: &Path, app_slug: &str) -> Result<(), String> {
    let developer_public = [0u8; 32];
    let app_id = app_id_for(app_slug, &developer_public);
    let bytes = packaged_app_graph_bytes(out_dir, app_slug, &developer_public, &app_id)?;
    fs::write(out_dir.join("app.eapp"), bytes).map_err(|err| err.to_string())
}

fn packaged_app_graph_bytes(
    app_dir: &Path,
    app_slug: &str,
    developer_public: &[u8; 32],
    app_id: &[u8; 32],
) -> Result<Vec<u8>, String> {
    let artifacts = collect_app_artifacts(app_dir)?;
    let app_manifest_sha256 = artifacts
        .iter()
        .find(|artifact| artifact.path == "app.edapp")
        .map(|artifact| artifact.sha256)
        .ok_or_else(|| "app.edapp missing from app package".to_owned())?;
    Ok(sdk_wire_record_bytes(SdkWireRecord::AppGraph(
        edgerun_wire::AppGraphRecord {
            abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
            flags: 1,
            app_id: *app_id,
            developer_public_key: *developer_public,
            app_manifest_sha256,
            app_slug: app_slug.as_bytes().to_vec(),
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

struct AppArtifact {
    kind: u16,
    path: String,
    sha256: [u8; 32],
}

fn collect_app_artifacts(app_dir: &Path) -> Result<Vec<AppArtifact>, String> {
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

fn collect_app_files(root: &Path, dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
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

fn verify_packaged_app_graph(app_dir: &Path, graph: &edgerun_wire::AppGraphRecord) -> bool {
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
    actual.iter().all(|expected| {
        graph.artifacts.iter().any(|actual| {
            actual.kind == expected.kind
                && actual.path == expected.path.as_bytes()
                && actual.sha256 == expected.sha256
        })
    })
}

fn app_id_for(app_slug: &str, developer_public: &[u8; 32]) -> [u8; 32] {
    let mut bytes =
        Vec::with_capacity(EAPP_DOMAIN_APP_ID.len() + developer_public.len() + app_slug.len());
    bytes.extend_from_slice(EAPP_DOMAIN_APP_ID);
    bytes.extend_from_slice(developer_public);
    bytes.extend_from_slice(app_slug.as_bytes());
    sha256(&bytes)
}

fn parse_seed(value: &str) -> Option<[u8; 32]> {
    parse_hex(value)?.try_into().ok()
}

fn parse_seal_key(value: &str) -> Option<SealKey> {
    Some(SealKey::from_bytes(parse_hex(value)?.try_into().ok()?))
}

fn user_profile_id(owner_id: &[u8; 32], epoch: u64) -> [u8; 32] {
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

fn parse_scope_hash(value: &str) -> Option<[u8; 32]> {
    if value == "any" {
        return Some([0u8; 32]);
    }
    if let Some(context_hex) = value.strip_prefix("context:") {
        return Some(sha256(&parse_hex(context_hex)?));
    }
    hex_to_32(value).ok()
}

fn cmd_issue_product(args: Vec<String>) -> i32 {
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

fn cmd_verify_product(args: Vec<String>) -> i32 {
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

fn cmd_issue_entitlement(args: Vec<String>) -> i32 {
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

fn cmd_verify_entitlement(args: Vec<String>) -> i32 {
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

fn read_app_graph(app_dir: &Path) -> Result<(Vec<u8>, edgerun_wire::AppGraphRecord), String> {
    let bytes = fs::read(app_dir.join("app.eapp")).map_err(|err| err.to_string())?;
    let graph = parse_app_graph_record(&bytes).ok_or_else(|| "invalid app.eapp".to_owned())?;
    Ok((bytes, graph))
}

fn parse_app_graph_record(bytes: &[u8]) -> Option<edgerun_wire::AppGraphRecord> {
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

fn parse_entitlement_record(bytes: &[u8]) -> Option<edgerun_wire::EntitlementRecord> {
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

fn parse_product_record(bytes: &[u8]) -> Option<edgerun_wire::ProductRecord> {
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

fn parse_settlement_record(bytes: &[u8]) -> Option<edgerun_wire::SettlementRecord> {
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

fn parse_payment_record(bytes: &[u8]) -> Option<edgerun_wire::PaymentRecord> {
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

fn parse_unit_manifest_record(bytes: &[u8]) -> Option<edgerun_wire::UnitManifestRecord> {
    let owned = bytes.to_vec();
    match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned).ok()? {
        SdkWireRecord::UnitManifest(manifest) => Some(manifest),
        _ => None,
    }
}

fn parse_entitlement_kind(value: &str) -> Option<u16> {
    match value {
        "license" => Some(1),
        "subscription" => Some(2),
        "consumable" => Some(3),
        "feature" => Some(4),
        _ => value.parse().ok(),
    }
}

struct EntitlementInput<'a> {
    kind: u16,
    store_fee_bps: u16,
    developer_share_bps: u16,
    valid_from: u64,
    valid_until: u64,
    app_id: &'a [u8],
    developer_id: &'a [u8],
    store_id: &'a [u8],
    release_id: &'a [u8; 32],
    terms_sha256: &'a [u8; 32],
    product_sha256: &'a [u8; 32],
    subject_id: &'a [u8],
    product_id: &'a [u8],
    purchase_id: &'a [u8],
    store_key: &'a SigningKey,
}

fn entitlement_bytes(input: EntitlementInput<'_>) -> Vec<u8> {
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

fn entitlement_unsigned_bytes(entitlement: &edgerun_wire::EntitlementRecord) -> Vec<u8> {
    let mut unsigned = entitlement.clone();
    unsigned.signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::Entitlement(unsigned))
}

fn verify_entitlement_signature(entitlement: &edgerun_wire::EntitlementRecord) -> bool {
    let Ok(signature_bytes) = <[u8; 64]>::try_from(entitlement.signature.as_slice()) else {
        return false;
    };
    let Ok(public_key) = VerifyingKey::from_bytes(&entitlement.store_id) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);
    public_key
        .verify(
            &signature_payload_for_domain(
                EENT_STORE_DOMAIN,
                &sha256(&entitlement_unsigned_bytes(entitlement)),
            ),
            &signature,
        )
        .is_ok()
}

struct ProductBinding {
    sha256: [u8; 32],
    kind: u16,
    store_fee_bps: u16,
    developer_share_bps: u16,
    product_id: Vec<u8>,
    store_id: [u8; 32],
    terms_sha256: [u8; 32],
}

fn read_verified_product(
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

fn cmd_issue_settlement(args: Vec<String>) -> i32 {
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

fn cmd_verify_settlement(args: Vec<String>) -> i32 {
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

fn parse_u64_arg(value: &str, name: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("bad {name}: {value}"))
}

fn parse_bps_arg(value: &str, name: &str) -> Result<u16, String> {
    match value.parse::<u16>() {
        Ok(value) if value <= 10_000 => Ok(value),
        _ => Err(format!("bad {name}: {value}")),
    }
}

struct ProductInput<'a> {
    kind: u16,
    store_fee_bps: u16,
    developer_share_bps: u16,
    price_minor: u64,
    validity_seconds: u64,
    app_id: &'a [u8],
    developer_id: &'a [u8],
    store_id: &'a [u8],
    release_id: &'a [u8; 32],
    terms_sha256: &'a [u8; 32],
    product_id: &'a [u8],
    currency: &'a [u8],
    developer_key: &'a SigningKey,
    store_key: &'a SigningKey,
}

fn product_bytes(input: ProductInput<'_>) -> Vec<u8> {
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

fn product_developer_unsigned_bytes(product: &edgerun_wire::ProductRecord) -> Vec<u8> {
    let mut unsigned = product.clone();
    unsigned.developer_signature.clear();
    unsigned.store_signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::Product(unsigned))
}

fn product_store_unsigned_bytes(product: &edgerun_wire::ProductRecord) -> Vec<u8> {
    let mut unsigned = product.clone();
    unsigned.store_signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::Product(unsigned))
}

fn verify_product_developer_signature(product: &edgerun_wire::ProductRecord) -> bool {
    let Ok(signature_bytes) = <[u8; 64]>::try_from(product.developer_signature.as_slice()) else {
        return false;
    };
    let Ok(public_key) = VerifyingKey::from_bytes(&product.developer_id) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);
    public_key
        .verify(
            &signature_payload_for_domain(
                EPRD_DEVELOPER_DOMAIN,
                &sha256(&product_developer_unsigned_bytes(product)),
            ),
            &signature,
        )
        .is_ok()
}

fn verify_product_store_signature(product: &edgerun_wire::ProductRecord) -> bool {
    let Ok(signature_bytes) = <[u8; 64]>::try_from(product.store_signature.as_slice()) else {
        return false;
    };
    let Ok(public_key) = VerifyingKey::from_bytes(&product.store_id) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);
    public_key
        .verify(
            &signature_payload_for_domain(
                EPRD_STORE_DOMAIN,
                &sha256(&product_store_unsigned_bytes(product)),
            ),
            &signature,
        )
        .is_ok()
}

struct SettlementInput<'a> {
    period_start: u64,
    period_end: u64,
    gross_minor: u64,
    processor_fee_minor: u64,
    store_fee_minor: u64,
    developer_net_minor: u64,
    developer_id: &'a [u8; 32],
    store_id: &'a [u8; 32],
    currency: &'a [u8],
    entitlement_hashes: &'a [[u8; 32]],
    store_key: &'a SigningKey,
}

fn settlement_bytes(input: SettlementInput<'_>) -> Vec<u8> {
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

fn settlement_unsigned_bytes(settlement: &edgerun_wire::SettlementRecord) -> Vec<u8> {
    let mut unsigned = settlement.clone();
    unsigned.signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::Settlement(unsigned))
}

fn verify_settlement_signature(settlement: &edgerun_wire::SettlementRecord) -> bool {
    let Ok(signature_bytes) = <[u8; 64]>::try_from(settlement.signature.as_slice()) else {
        return false;
    };
    let Ok(public_key) = VerifyingKey::from_bytes(&settlement.store_id) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);
    public_key
        .verify(
            &signature_payload_for_domain(
                ESET_STORE_DOMAIN,
                &sha256(&settlement_unsigned_bytes(settlement)),
            ),
            &signature,
        )
        .is_ok()
}

fn verify_settlement_entitlements(
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

fn cmd_issue_payment_intent(args: Vec<String>) -> i32 {
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

fn cmd_settle_payment(args: Vec<String>) -> i32 {
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

fn cmd_verify_payment(args: Vec<String>) -> i32 {
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

fn cmd_write_trust_policy(args: Vec<String>) -> i32 {
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

fn cmd_verify_trusted_app(args: Vec<String>) -> i32 {
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

fn cmd_verify_trusted_product(args: Vec<String>) -> i32 {
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

fn cmd_verify_trusted_entitlement(args: Vec<String>) -> i32 {
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

fn cmd_verify_trusted_payment(args: Vec<String>) -> i32 {
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

fn cmd_verify_trusted_settlement(args: Vec<String>) -> i32 {
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

fn parse_payment_purpose(value: &str) -> Option<u16> {
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

const TRUST_ROLE_DEVELOPER: u16 = 1;
const TRUST_ROLE_APP_STORE: u16 = 2;
const TRUST_ROLE_PRODUCT_STORE: u16 = 3;
const TRUST_ROLE_ENTITLEMENT_STORE: u16 = 4;
const TRUST_ROLE_PAYMENT_STORE: u16 = 5;
const TRUST_ROLE_SETTLEMENT_STORE: u16 = 6;
const TRUST_ROLE_PAYER: u16 = 7;
const TRUST_ROLE_PAYEE: u16 = 8;

fn parse_trust_role(value: &str) -> Option<u16> {
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

struct OwnedTrustEntry {
    role: u16,
    valid_from: u64,
    valid_until: u64,
    public_key: [u8; 32],
    app_id: [u8; 32],
    developer_id: [u8; 32],
}

fn trust_policy_bytes(entries: &[OwnedTrustEntry]) -> Vec<u8> {
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

fn read_trust_policy(path: &str) -> Option<edgerun_wire::TrustPolicy> {
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

fn trust_allows(
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
struct OwnedUserGrant {
    app_id: [u8; 32],
    release_id: [u8; 32],
    scope_sha256: [u8; 32],
    capability_kind: u16,
    operation: u16,
    min_assurance: u16,
    flags: u16,
    valid_from: u64,
    valid_until: u64,
}

impl From<&edgerun_wire::UserCapabilityGrant> for OwnedUserGrant {
    fn from(grant: &edgerun_wire::UserCapabilityGrant) -> Self {
        Self {
            app_id: grant.app_id,
            release_id: grant.release_id,
            scope_sha256: grant.scope_sha256,
            capability_kind: grant.capability_kind,
            operation: grant.operation,
            min_assurance: grant.min_assurance,
            flags: grant.flags,
            valid_from: grant.valid_from,
            valid_until: grant.valid_until,
        }
    }
}

fn user_profile_body_bytes(
    profile_id: &[u8; 32],
    owner_key: &SigningKey,
    epoch: u64,
    monotonic_version: u64,
    grants: &[OwnedUserGrant],
) -> Vec<u8> {
    wire_user_profile_body_bytes(profile_id, owner_key, epoch, monotonic_version, grants)
}

fn user_profile_file_bytes(body: &[u8], seal_key: &SealKey) -> Result<Vec<u8>, String> {
    wire_user_profile_file_bytes(body, seal_key)
}

fn open_user_profile_file(
    bytes: &[u8],
    seal_key: &SealKey,
) -> Result<edgerun_wire::UserProfileBody, String> {
    open_wire_user_profile_file(bytes, seal_key)
}

fn verify_user_profile_body_signature(profile: &edgerun_wire::UserProfileBody) -> bool {
    verify_wire_user_profile_body_signature(profile)
}

fn sdk_wire_record_bytes(record: SdkWireRecord) -> Vec<u8> {
    sdk_wire_bytes(&record)
}

fn read_sdk_wire_record(path: &str) -> Result<SdkWireRecord, String> {
    let bytes = fs::read(path).map_err(|err| format!("cannot read {path}: {err}"))?;
    let owned = bytes.to_vec();
    edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned)
        .map_err(|err| format!("invalid SDK wire record {path}: {err:?}"))
}

fn parse_capability_request_record(bytes: &[u8]) -> Option<edgerun_wire::CapabilityRequest> {
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

fn parse_capability_response_record(bytes: &[u8]) -> Option<edgerun_wire::CapabilityResponse> {
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

fn wire_capability_response_binding_ok(
    request_bytes: &[u8],
    request: &edgerun_wire::CapabilityRequest,
    response: &edgerun_wire::CapabilityResponse,
) -> bool {
    response.request_sha256 == sha256(request_bytes)
        && response.capability_kind == request.capability_kind
        && response.operation == request.operation
        && response.assurance >= request.assurance
}

fn wire_user_grant_from_owned(grant: &OwnedUserGrant) -> edgerun_wire::UserCapabilityGrant {
    edgerun_wire::UserCapabilityGrant {
        capability_kind: grant.capability_kind,
        operation: grant.operation,
        min_assurance: grant.min_assurance,
        flags: grant.flags,
        valid_from: grant.valid_from,
        valid_until: grant.valid_until,
        app_id: grant.app_id,
        release_id: grant.release_id,
        scope_sha256: grant.scope_sha256,
    }
}

fn wire_user_profile_body_unsigned_bytes(body: &edgerun_wire::UserProfileBody) -> Vec<u8> {
    let mut unsigned = body.clone();
    unsigned.signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::UserProfileBody(unsigned))
}

fn wire_user_profile_body_bytes(
    profile_id: &[u8; 32],
    owner_key: &SigningKey,
    epoch: u64,
    monotonic_version: u64,
    grants: &[OwnedUserGrant],
) -> Vec<u8> {
    let mut body = edgerun_wire::UserProfileBody {
        abi_version: edgerun_wire::SDK_WIRE_ABI_VERSION,
        flags: 1,
        epoch,
        monotonic_version,
        profile_id: *profile_id,
        owner_id: *owner_key.verifying_key().as_bytes(),
        grants: grants.iter().map(wire_user_grant_from_owned).collect(),
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

fn wire_user_profile_file_bytes(body_bytes: &[u8], seal_key: &SealKey) -> Result<Vec<u8>, String> {
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
        body_sha256: sha256(body_bytes),
        sealed_body: sealed,
    };
    Ok(sdk_wire_record_bytes(SdkWireRecord::UserProfile(profile)))
}

fn open_wire_user_profile_file(
    bytes: &[u8],
    seal_key: &SealKey,
) -> Result<edgerun_wire::UserProfileBody, String> {
    let owned = bytes.to_vec();
    let profile = match edgerun_wire::from_bytes::<SdkWireRecord, edgerun_wire::WireError>(&owned)
        .map_err(|err| format!("invalid wire user profile: {err:?}"))?
    {
        SdkWireRecord::UserProfile(profile) => profile,
        _ => return Err("wire record is not a user profile".to_owned()),
    };
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

fn verify_wire_user_profile_body_signature(profile: &edgerun_wire::UserProfileBody) -> bool {
    let Ok(public_key) = VerifyingKey::from_bytes(&profile.owner_id) else {
        return false;
    };
    let Ok(signature_bytes) = <[u8; 64]>::try_from(profile.signature.as_slice()) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);
    let unsigned = wire_user_profile_body_unsigned_bytes(profile);
    public_key
        .verify(
            &signature_payload_for_domain(EUPB_DOMAIN, &sha256(&unsigned)),
            &signature,
        )
        .is_ok()
}

fn user_profile_allows_request(
    profile: &edgerun_wire::UserProfileBody,
    request: &edgerun_wire::CapabilityRequest,
    at: u64,
) -> bool {
    wire_user_profile_allows_request(profile, request, at)
}

fn wire_user_profile_allows_request(
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

const REV_KIND_KEY: u16 = 1;
const REV_KIND_APP: u16 = 2;
const REV_KIND_PRODUCT: u16 = 3;
const REV_KIND_ENTITLEMENT: u16 = 4;
const REV_KIND_PAYMENT: u16 = 5;
const REV_KIND_SETTLEMENT: u16 = 6;
const CAPABILITY_KIND_SIGNING: u16 = 1;
const CAPABILITY_KIND_SEALING: u16 = 2;
const CAPABILITY_KIND_STORAGE: u16 = 4;
const CAPABILITY_OPERATION_SIGN: u16 = 1;
const CAPABILITY_OPERATION_SEAL: u16 = 3;
const CAPABILITY_OPERATION_UNSEAL: u16 = 4;
const CAPABILITY_OPERATION_READ: u16 = 6;
const CAPABILITY_OPERATION_WRITE: u16 = 7;
const CAPABILITY_STATUS_OK: u16 = 0;
const CAPABILITY_STATUS_POLICY_DENIED: u16 = 1;
const CAPABILITY_STATUS_INVALID_REQUEST: u16 = 2;
const CAPABILITY_STATUS_PROVIDER_FAILED: u16 = 3;
const SIGN_ALGORITHM_ED25519: u16 = 1;

fn parse_capability_kind(value: &str) -> Option<u16> {
    match value {
        "sign" | "signing" => Some(CAPABILITY_KIND_SIGNING),
        "seal" | "sealing" => Some(CAPABILITY_KIND_SEALING),
        "payment" | "payments" => Some(3),
        "storage" => Some(CAPABILITY_KIND_STORAGE),
        "network" => Some(5),
        _ => value.parse().ok(),
    }
}

fn parse_capability_operation(value: &str) -> Option<u16> {
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

fn parse_sign_algorithm(value: &str) -> Option<u16> {
    match value {
        "ed25519" => Some(SIGN_ALGORITHM_ED25519),
        _ => value.parse().ok(),
    }
}

fn parse_u16_arg(value: &str, name: &str) -> Result<u16, String> {
    value
        .parse()
        .map_err(|_| format!("{name} must be an unsigned 16-bit integer"))
}

fn cmd_write_revocation(args: Vec<String>) -> i32 {
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

fn cmd_verify_revocation(args: Vec<String>) -> i32 {
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
    if signature_ok {
        0
    } else {
        1
    }
}

fn cmd_write_sign_request(args: Vec<String>) -> i32 {
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

fn cmd_sign_request(args: Vec<String>) -> i32 {
    if args.len() < 4 {
        eprintln!("sign-request requires request.rkyv, out.rkyv, provider, and signer seed");
        return 1;
    }
    execute_sign_request(&args, None)
}

fn cmd_sign_request_authorized(args: Vec<String>) -> i32 {
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

fn execute_sign_request(
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

fn cmd_verify_sign_response(args: Vec<String>) -> i32 {
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
    if binding_ok && signature_ok {
        0
    } else {
        1
    }
}

fn parse_revocation_kind(value: &str) -> Option<u16> {
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

struct RevocationInput<'a> {
    kind: u16,
    issued_at: u64,
    target: &'a [u8; 32],
    reason: &'a [u8],
    issuer_key: &'a SigningKey,
}

fn revocation_bytes(input: RevocationInput<'_>) -> Vec<u8> {
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

fn revocation_unsigned_bytes(revocation: &edgerun_wire::Revocation) -> Vec<u8> {
    let mut unsigned = revocation.clone();
    unsigned.signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::Revocation(unsigned))
}

fn parse_revocation_record(bytes: &[u8]) -> Option<edgerun_wire::Revocation> {
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

fn verify_revocation_signature(revocation: &edgerun_wire::Revocation) -> bool {
    let Ok(signature_bytes) = <[u8; 64]>::try_from(revocation.signature.as_slice()) else {
        return false;
    };
    let Ok(public_key) = VerifyingKey::from_bytes(&revocation.issuer) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);
    let body = revocation_unsigned_bytes(revocation);
    public_key
        .verify(
            &signature_payload_for_domain(EREV_DOMAIN, &sha256(&body)),
            &signature,
        )
        .is_ok()
}

fn cmd_write_capability_request(args: Vec<String>) -> i32 {
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

fn cmd_verify_capability_response(args: Vec<String>) -> i32 {
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
    if binding_ok && denial_proof_ok {
        0
    } else {
        1
    }
}

fn cmd_write_seal_request(args: Vec<String>) -> i32 {
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

fn cmd_write_unseal_request(args: Vec<String>) -> i32 {
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

fn write_sealing_request(
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

fn cmd_seal_request(args: Vec<String>) -> i32 {
    if args.len() < 4 {
        eprintln!(
            "seal-request requires request.rkyv, out.rkyv, provider, seal key hex, and optional assurance"
        );
        return 1;
    }
    execute_sealing_request(&args, CAPABILITY_OPERATION_SEAL, None)
}

fn cmd_seal_request_authorized(args: Vec<String>) -> i32 {
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

fn cmd_unseal_request(args: Vec<String>) -> i32 {
    if args.len() < 4 {
        eprintln!(
            "unseal-request requires request.rkyv, out.rkyv, provider, seal key hex, and optional assurance"
        );
        return 1;
    }
    execute_sealing_request(&args, CAPABILITY_OPERATION_UNSEAL, None)
}

fn cmd_unseal_request_authorized(args: Vec<String>) -> i32 {
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

struct ProfileAuthorizationInput<'a> {
    profile_path: &'a str,
    profile_key_hex: &'a str,
    at: u64,
}

fn execute_sealing_request(
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

fn parse_authorized_at(value: &str) -> Option<u64> {
    match parse_u64_arg(value, "at") {
        Ok(value) => Some(value),
        Err(err) => {
            eprintln!("{err}");
            None
        }
    }
}

fn profile_authorizes_request(
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

fn cmd_verify_seal_response(args: Vec<String>) -> i32 {
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

fn cmd_write_storage_read_request(args: Vec<String>) -> i32 {
    if args.len() < 8 {
        eprintln!(
            "write-storage-read-request requires out.rkyv, app id, release id, subject hash, key/context hex, expected sha256|any, assurance, and nonce hex"
        );
        return 1;
    }
    write_storage_request(&args, CAPABILITY_OPERATION_READ)
}

fn cmd_write_storage_write_request(args: Vec<String>) -> i32 {
    if args.len() < 8 {
        eprintln!(
            "write-storage-write-request requires out.rkyv, app id, release id, subject hash, key/context hex, payload hex, assurance, and nonce hex"
        );
        return 1;
    }
    write_storage_request(&args, CAPABILITY_OPERATION_WRITE)
}

fn write_storage_request(args: &[String], operation: u16) -> i32 {
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

fn cmd_storage_read_request(args: Vec<String>) -> i32 {
    if args.len() < 4 {
        eprintln!(
            "storage-read-request requires request.rkyv, out.rkyv, provider, root dir, and optional assurance"
        );
        return 1;
    }
    execute_storage_request(&args, CAPABILITY_OPERATION_READ, None)
}

fn cmd_storage_read_request_authorized(args: Vec<String>) -> i32 {
    if args.len() < 7 {
        eprintln!(
            "storage-read-request-authorized requires profile.eusr, profile key hex, request.rkyv, out.rkyv, provider, root dir, at, and optional assurance"
        );
        return 1;
    }
    execute_storage_authorized(&args, CAPABILITY_OPERATION_READ)
}

fn cmd_storage_write_request(args: Vec<String>) -> i32 {
    if args.len() < 4 {
        eprintln!(
            "storage-write-request requires request.rkyv, out.rkyv, provider, root dir, and optional assurance"
        );
        return 1;
    }
    execute_storage_request(&args, CAPABILITY_OPERATION_WRITE, None)
}

fn cmd_storage_write_request_authorized(args: Vec<String>) -> i32 {
    if args.len() < 7 {
        eprintln!(
            "storage-write-request-authorized requires profile.eusr, profile key hex, request.rkyv, out.rkyv, provider, root dir, at, and optional assurance"
        );
        return 1;
    }
    execute_storage_authorized(&args, CAPABILITY_OPERATION_WRITE)
}

fn execute_storage_authorized(args: &[String], expected_operation: u16) -> i32 {
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

fn execute_storage_request(
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

fn cmd_verify_storage_response(args: Vec<String>) -> i32 {
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

fn cmd_create_user_profile(args: Vec<String>) -> i32 {
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
    let body = user_profile_body_bytes(&profile_id, &owner_key, epoch, monotonic_version, &[]);
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

fn cmd_grant_profile_capability(args: Vec<String>) -> i32 {
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
        capability_kind,
        operation,
        min_assurance,
        flags: 0,
        valid_from,
        valid_until,
    });
    let profile_id = existing.profile_id;
    let new_body = user_profile_body_bytes(
        &profile_id,
        &owner_key,
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

fn cmd_open_user_profile(args: Vec<String>) -> i32 {
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
    println!("grant_count: {}", parsed.grants.len());
    print_check(
        "owner-signature",
        verify_user_profile_body_signature(&parsed),
    );
    0
}

fn cmd_verify_profile_access(args: Vec<String>) -> i32 {
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
    if allowed {
        0
    } else {
        1
    }
}

struct CapabilityRequestInput<'a> {
    kind: u16,
    operation: u16,
    assurance: u16,
    app_id: &'a [u8; 32],
    release_id: &'a [u8; 32],
    subject_sha256: &'a [u8; 32],
    payload_sha256: &'a [u8; 32],
    context: &'a [u8],
    payload: &'a [u8],
    nonce: &'a [u8],
}

fn capability_request_bytes(input: CapabilityRequestInput<'_>) -> Vec<u8> {
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

struct CapabilityResponseInput<'a> {
    request_bytes: &'a [u8],
    kind: u16,
    operation: u16,
    status: u16,
    assurance: u16,
    provider: &'a [u8],
    responder: &'a [u8],
    payload: &'a [u8],
    proof: &'a [u8],
}

fn capability_response_bytes(input: CapabilityResponseInput<'_>) -> Vec<u8> {
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

fn capability_response_binding_ok(
    request_bytes: &[u8],
    request: &edgerun_wire::CapabilityRequest,
    response: &edgerun_wire::CapabilityResponse,
) -> bool {
    response.request_sha256 == sha256(request_bytes)
        && response.capability_kind == request.capability_kind
        && response.operation == request.operation
        && response.assurance >= request.assurance
}

fn capability_denial_response_bytes(
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

fn write_capability_denial_response(
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

fn capability_denial_proof(
    request_bytes: &[u8],
    status: u16,
    provider: &[u8],
    payload: &[u8],
) -> [u8; 32] {
    capability_response_proof(request_bytes, status, provider, payload)
}

fn sealing_response_proof(
    request_bytes: &[u8],
    operation: u16,
    provider: &[u8],
    payload: &[u8],
) -> [u8; 32] {
    capability_response_proof(request_bytes, operation, provider, payload)
}

fn storage_provider_path(root: &Path, context: &[u8]) -> PathBuf {
    root.join(format!("{}.bin", bytes_to_hex(&sha256(context))))
}

fn storage_write_receipt_payload(payload: &[u8]) -> Vec<u8> {
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&StorageWriteReceiptRecord {
        payload_sha256: sha256(payload),
        payload_len: payload.len() as u64,
    })
    .expect("storage write receipt must serialize through rkyv")
    .into_vec()
}

fn storage_write_receipt_matches(receipt: &[u8], payload: &[u8]) -> bool {
    let Ok(receipt) =
        edgerun_wire::from_bytes::<StorageWriteReceiptRecord, edgerun_wire::WireError>(receipt)
    else {
        return false;
    };
    receipt.payload_sha256 == sha256(payload) && receipt.payload_len == payload.len() as u64
}

fn storage_response_proof(
    request_bytes: &[u8],
    operation: u16,
    provider: &[u8],
    payload: &[u8],
) -> [u8; 32] {
    capability_response_proof(request_bytes, operation, provider, payload)
}

fn capability_response_proof(
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

struct SignRequestInput<'a> {
    domain: &'a [u8],
    app_id: &'a [u8; 32],
    release_id: &'a [u8; 32],
    subject_sha256: &'a [u8; 32],
    payload_sha256: &'a [u8; 32],
    algorithm: u16,
    assurance: u16,
    nonce: &'a [u8],
}

fn sign_request_bytes(input: SignRequestInput<'_>) -> Vec<u8> {
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

struct SignResponseInput<'a> {
    request_bytes: &'a [u8],
    provider: &'a [u8],
    algorithm: u16,
    assurance: u16,
    signer: &'a [u8],
    signing_key: &'a SigningKey,
}

fn sign_response_bytes(input: SignResponseInput<'_>) -> Vec<u8> {
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

fn signing_algorithm_payload(algorithm: u16) -> Vec<u8> {
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(&SigningAlgorithmRecord { algorithm })
        .expect("signing algorithm payload must serialize through rkyv")
        .into_vec()
}

fn parse_signing_algorithm_payload(payload: &[u8]) -> Option<u16> {
    let owned = payload.to_vec();
    edgerun_wire::from_bytes::<SigningAlgorithmRecord, edgerun_wire::WireError>(&owned)
        .ok()
        .map(|record| record.algorithm)
}

fn signing_request_algorithm(request: &edgerun_wire::CapabilityRequest) -> Option<u16> {
    (request.capability_kind == CAPABILITY_KIND_SIGNING
        && request.operation == CAPABILITY_OPERATION_SIGN)
        .then(|| parse_signing_algorithm_payload(&request.payload))
        .flatten()
}

fn signing_response_algorithm(response: &edgerun_wire::CapabilityResponse) -> Option<u16> {
    (response.capability_kind == CAPABILITY_KIND_SIGNING
        && response.operation == CAPABILITY_OPERATION_SIGN)
        .then(|| parse_signing_algorithm_payload(&response.payload))
        .flatten()
}

fn verify_sign_response_signature(
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
    let Ok(public_key) = VerifyingKey::from_bytes(&public_key_bytes) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);
    public_key
        .verify(
            &signature_payload_for_domain(CAPABILITY_RESPONSE_DOMAIN, &request_sha256),
            &signature,
        )
        .is_ok()
}

#[derive(Clone, Copy)]
struct RevokedTarget {
    target: [u8; 32],
    issuer: [u8; 32],
    issued_at: u64,
}

#[derive(Default)]
struct RevocationSet {
    key: Vec<RevokedTarget>,
    app: Vec<RevokedTarget>,
    product: Vec<RevokedTarget>,
    entitlement: Vec<RevokedTarget>,
    payment: Vec<RevokedTarget>,
    settlement: Vec<RevokedTarget>,
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
    fn revoke_payment(
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

fn revoked_by_authorized_issuer(
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

fn revocation_authorized(
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

fn read_revocations<'a>(paths: impl Iterator<Item = &'a String>) -> RevocationSet {
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

struct PaymentIntentInput<'a> {
    purpose: u16,
    amount_minor: u64,
    created_at: u64,
    app_id: &'a [u8],
    release_id: &'a [u8; 32],
    payer_id: &'a [u8; 32],
    payee_id: &'a [u8; 32],
    context_sha256: &'a [u8; 32],
    currency: &'a [u8],
    payer_key: &'a SigningKey,
}

fn payment_intent_bytes(input: PaymentIntentInput<'_>) -> Vec<u8> {
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

fn payment_intent_unsigned_bytes(payment: &edgerun_wire::PaymentRecord) -> Vec<u8> {
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

struct PaymentSettlementInput<'a> {
    settled_at: u64,
    store_id: &'a [u8],
    rail_ref_sha256: &'a [u8; 32],
    rail_kind: &'a [u8],
    store_key: &'a SigningKey,
}

fn settle_payment_bytes(
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

fn payment_store_unsigned_bytes(payment: &edgerun_wire::PaymentRecord) -> Vec<u8> {
    let mut unsigned = payment.clone();
    unsigned.store_signature.clear();
    sdk_wire_record_bytes(SdkWireRecord::Payment(unsigned))
}

fn verify_payment_payer_signature(payment: &edgerun_wire::PaymentRecord) -> bool {
    let Ok(signature_bytes) = <[u8; 64]>::try_from(payment.payer_signature.as_slice()) else {
        return false;
    };
    let Ok(public_key) = VerifyingKey::from_bytes(&payment.payer_id) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);
    public_key
        .verify(
            &signature_payload_for_domain(
                EPAY_PAYER_DOMAIN,
                &sha256(&payment_intent_unsigned_bytes(payment)),
            ),
            &signature,
        )
        .is_ok()
}

fn verify_payment_store_signature(payment: &edgerun_wire::PaymentRecord) -> bool {
    if payment.store_id.len() != 32 {
        return false;
    }
    let Ok(public_key_bytes) = <[u8; 32]>::try_from(payment.store_id.as_slice()) else {
        return false;
    };
    let Ok(signature_bytes) = <[u8; 64]>::try_from(payment.store_signature.as_slice()) else {
        return false;
    };
    let Ok(public_key) = VerifyingKey::from_bytes(&public_key_bytes) else {
        return false;
    };
    let signature = Signature::from_bytes(&signature_bytes);
    public_key
        .verify(
            &signature_payload_for_domain(
                EPAY_STORE_DOMAIN,
                &sha256(&payment_store_unsigned_bytes(payment)),
            ),
            &signature,
        )
        .is_ok()
}

fn hmac_browser_app_manifest(
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

fn browser_composition_json(composition: &edgerun_wire::CompositionRecord) -> String {
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

fn copy_file(from: PathBuf, to: PathBuf) -> Result<(), String> {
    let parent = to
        .parent()
        .ok_or_else(|| format!("bad destination path: {}", to.display()))?;
    fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    fs::copy(&from, &to)
        .map(|_| ())
        .map_err(|err| format!("cannot copy {} to {}: {err}", from.display(), to.display()))
}

fn file_sha256_hex(path: PathBuf) -> Result<String, String> {
    let bytes = fs::read(&path).map_err(|err| format!("cannot read {}: {err}", path.display()))?;
    Ok(String::from_utf8_lossy(&sha256_hex(&bytes)).into_owned())
}

struct NativeImplementation {
    target: &'static str,
    source_path: PathBuf,
    app_path: PathBuf,
    sha256: String,
}

fn native_implementation_for_unit(
    root: &Path,
    unit_id: &str,
) -> Result<Option<NativeImplementation>, String> {
    match unit_id {
        "sha256-fips180" => build_native_sha256_implementation(root),
        _ => Ok(None),
    }
}

fn build_native_sha256_implementation(root: &Path) -> Result<Option<NativeImplementation>, String> {
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

fn native_host_target() -> Option<&'static str> {
    match (env::consts::ARCH, env::consts::OS) {
        ("x86_64", "linux") => Some("x86_64-unknown-linux-gnu"),
        ("aarch64", "linux") => Some("aarch64-unknown-linux-gnu"),
        ("x86_64", "macos") => Some("x86_64-apple-darwin"),
        ("aarch64", "macos") => Some("aarch64-apple-darwin"),
        _ => None,
    }
}

fn native_sha256_built_library_name() -> &'static str {
    match env::consts::OS {
        "macos" => "libedgerun_sdk_native_sha256_fips180.dylib",
        _ => "libedgerun_sdk_native_sha256_fips180.so",
    }
}

fn native_build_output_path(target_dir: &Path, target: &str) -> PathBuf {
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

fn native_unit_library_name() -> &'static str {
    match env::consts::OS {
        "macos" => "libunit.dylib",
        _ => "libunit.so",
    }
}

fn native_unit_library_path(root: &Path, unit_id: &str) -> Result<PathBuf, String> {
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

fn build_native_unit_implementation(root: &Path, unit_id: &str) -> Result<(), String> {
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

fn native_cdylib_output_path(target_dir: &Path, target: &str, package: &str) -> PathBuf {
    let lib_name = native_cdylib_library_name(package);
    let direct = target_dir.join("release").join(&lib_name);
    if direct.exists() {
        return direct;
    }
    target_dir.join(target).join("release").join(lib_name)
}

fn native_cdylib_library_name(package: &str) -> String {
    let crate_name = package.replace('-', "_");
    match env::consts::OS {
        "macos" => format!("lib{crate_name}.dylib"),
        _ => format!("lib{crate_name}.so"),
    }
}

fn verify_native_sha256_library(path: &Path) -> Result<(), String> {
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

struct NativeLibrary {
    handle: *mut core::ffi::c_void,
}

impl NativeLibrary {
    #[cfg(unix)]
    unsafe fn open(path: &Path) -> Result<Self, String> {
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
    unsafe fn open(path: &Path) -> Result<Self, String> {
        let _ = path;
        Err("native library verification requires dlopen-compatible host".to_owned())
    }

    #[cfg(unix)]
    unsafe fn symbol<T: Copy>(&self, name: &str) -> Result<T, String> {
        let name = CString::new(name).map_err(|_| format!("symbol contains NUL: {name}"))?;
        let symbol = dlsym(self.handle, name.as_ptr());
        if symbol.is_null() {
            Err(dl_error())
        } else {
            Ok(core::mem::transmute_copy(&symbol))
        }
    }

    #[cfg(not(unix))]
    unsafe fn symbol<T: Copy>(&self, name: &str) -> Result<T, String> {
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
const RTLD_NOW: core::ffi::c_int = 2;

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
    fn munmap(addr: *mut core::ffi::c_void, len: usize) -> core::ffi::c_int;
}

#[cfg(unix)]
unsafe fn dl_error() -> String {
    let err = dlerror();
    if err.is_null() {
        "dynamic loader error".to_owned()
    } else {
        CStr::from_ptr(err).to_string_lossy().into_owned()
    }
}

#[cfg(unix)]
unsafe fn map_low_memory(len: usize) -> Result<*mut u8, String> {
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
unsafe fn map_low_memory(_len: usize) -> Result<*mut u8, String> {
    Err("native unit direct-pointer memory requires Unix mmap".to_owned())
}

fn sha256_browser_app_manifest(unit_json: &str) -> String {
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

const SHA256_BROWSER_INDEX: &str = r#"<!doctype html>
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

const HMAC_BROWSER_INDEX: &str = r#"<!doctype html>
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

const SHA256_BROWSER_RUNNER: &str = r#"const statusEl = document.getElementById("status");
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
  const app = await (await fetch("./app.edapp")).json();
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

const COMPOSITION_BROWSER_RUNNER: &str = r#"const statusEl = document.getElementById("status");
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
  const app = await (await fetch("./app.edapp")).json();
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

struct BenchResult {
    bytes: usize,
    iterations: usize,
    wasmtime_invoke_ns: f64,
    wasmtime_compile_ns: f64,
    native_dylib_ns: f64,
    checksum: u8,
}

fn bench_sha256_unit(bytes: usize, iterations: usize) -> Result<BenchResult, String> {
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

fn bench_sha256_system_wasmtime(
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

fn run_system_wasmtime_invoke(
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

fn run_system_wasmtime_compile(wasmtime: &Path, wasm_path: &Path) -> Result<(), String> {
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

fn system_command_error(command: &str, output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    format!(
        "{command} failed with status {}: {}{}",
        output.status,
        stderr.trim(),
        stdout.trim()
    )
}

fn bench_sha256_native_dylib(
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

fn cmd_verify_segment(id: Option<&str>) -> i32 {
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

fn cmd_verify_chain(id: Option<&str>) -> i32 {
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

fn cmd_verify_chain_reports(args: Vec<String>) -> i32 {
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

fn cmd_verify_report(path: Option<&str>) -> i32 {
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

fn cmd_verify_segment_report(path: Option<&str>) -> i32 {
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

fn cmd_replay_segment_report(args: Vec<String>) -> i32 {
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

fn cmd_sign_segment_report(args: Vec<String>) -> i32 {
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

fn cmd_write_signer_policy(args: Vec<String>) -> i32 {
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

fn cmd_verify_signed_segment_report(args: Vec<String>) -> i32 {
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

fn cmd_quote_composition(args: Vec<String>) -> i32 {
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

fn cmd_preflight_composition(args: Vec<String>) -> i32 {
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

fn cmd_run_segment(args: Vec<String>) -> i32 {
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

fn cmd_run_composition(args: Vec<String>) -> i32 {
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

fn runtime_index() -> &'static RuntimeIndex {
    INDEX.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        load_runtime_index(&root).expect("edgerun-sdk runtime index")
    })
}

fn units() -> &'static [UnitManifest] {
    &runtime_index().units
}

fn compositions() -> &'static [CompositionManifest] {
    &runtime_index().compositions
}

fn segments() -> &'static [SegmentManifest] {
    &runtime_index().segments
}

fn chains() -> &'static [ChainManifest] {
    &runtime_index().chains
}

fn unit(id: &str) -> Option<&'static UnitManifest> {
    units().iter().find(|unit| unit.id == id)
}

fn composition(id: &str) -> Option<&'static CompositionManifest> {
    compositions()
        .iter()
        .find(|composition| composition.id == id)
}

fn segment(id: &str) -> Option<&'static SegmentManifest> {
    segments().iter().find(|segment| segment.id == id)
}

fn chain(id: &str) -> Option<&'static ChainManifest> {
    chains().iter().find(|chain| chain.id == id)
}

fn build_artifacts() -> Result<Vec<PathBuf>, String> {
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
struct RustUnitSource {
    id: String,
    standard: String,
    standard_id: i32,
    rust_manifest: PathBuf,
    rust_source: PathBuf,
    wasm_path: PathBuf,
    manifest_path: PathBuf,
}

fn discover_rust_unit_sources(root: &Path) -> Result<Vec<RustUnitSource>, String> {
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

struct RustUnitMetadata {
    id: String,
    standard: String,
}

fn load_runtime_index(root: &Path) -> Result<RuntimeIndex, String> {
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

fn load_runtime_unit(source: &RustUnitSource) -> Result<UnitManifest, String> {
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

fn runtime_exports_from_surface(surface: &WasmSurface) -> Result<&'static [ApiFunction], String> {
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

fn api_type_string(ty: &FuncType) -> String {
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

fn valtype_name(ty: &ValType) -> &'static str {
    match ty {
        ValType::I32 => "i32",
        ValType::I64 => "i64",
        ValType::F32 => "f32",
        ValType::F64 => "f64",
    }
}

fn discover_compositions(
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

fn discover_segments(
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

fn discover_chains(root: &Path) -> Result<Vec<ChainManifest>, String> {
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

fn leak_str(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn leak_slice<T>(value: Vec<T>) -> &'static [T] {
    Box::leak(value.into_boxed_slice())
}

fn rust_unit_metadata(manifest_path: &Path) -> Result<RustUnitMetadata, String> {
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

fn compile_rust_unit_source(root: &Path, source: &RustUnitSource) -> Result<(), String> {
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

fn generate_rust_unit_metadata(
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

fn validate_rust_unit_source(source: &RustUnitSource) -> Result<(), String> {
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

fn read_rust_unit_standard_id(path: &Path) -> Result<i32, String> {
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

fn validate_wasm_unit_surface(unit_id: &str, surface: &WasmSurface) -> Result<(), String> {
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

fn verify_unit_identity_exports(
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

fn has_i32_zero_arg_export(surface: &WasmSurface, name: &str) -> bool {
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

fn runtime_unit_manifest_bytes(
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

fn runtime_api_manifest_bytes(unit_id: &str, surface: &WasmSurface) -> Result<Vec<u8>, String> {
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

fn rust_package_name(manifest_path: &Path) -> Result<Option<String>, String> {
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
struct BuildStep {
    opcode: u8,
    component_index: u8,
    function_index: u8,
    arg0: u32,
    arg1: u32,
    arg2: u32,
    arg3: u32,
    arg4: u32,
}

const fn build_step(
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

fn composition_manifest_bytes(manifest: &CompositionManifest) -> Result<Vec<u8>, String> {
    composition_manifest_bytes_with_units(manifest, units())
}

fn composition_manifest_bytes_with_units(
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

fn parse_composition_source(
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

fn component_index(
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

fn input_index(inputs: &[String], name: &str, context: &str) -> Result<usize, String> {
    inputs
        .iter()
        .position(|actual| actual == name)
        .ok_or_else(|| format!("{context}: unknown input: {name}"))
}

fn resolve_function(
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

fn parse_ref(token: &str, inputs: &[String], context: &str) -> Result<u32, String> {
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

fn comparison_ref(cmp: &str, reference: u32, context: &str) -> Result<u32, String> {
    let comparison = match cmp {
        "gt" => 1,
        "eq" => 2,
        "ne" => 3,
        _ => return Err(format!("{context}: bad comparison: {cmp}")),
    };
    Ok((comparison << 28) | (reference & 0x0fff_ffff))
}

fn parse_slot(token: &str, context: &str) -> Result<u32, String> {
    let Some(slot) = token.strip_prefix("slot:") else {
        return Err(format!("{context}: bad slot: {token}"));
    };
    parse_u32(slot, context)
}

fn parse_u32(token: &str, context: &str) -> Result<u32, String> {
    token
        .parse::<u32>()
        .map_err(|_| format!("{context}: bad integer: {token}"))
}

fn segment_manifest_bytes(manifest: &SegmentManifest) -> Result<Vec<u8>, String> {
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

fn chain_manifest_bytes(manifest: &ChainManifest) -> Result<Vec<u8>, String> {
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

fn parse_segment_source(manifest: &SegmentManifest, source: &str) -> Result<(), String> {
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

fn parse_chain_source(
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

struct ParsedSegmentSource {
    id: String,
    composition_id: String,
    node_role: String,
    capability: String,
    input_kinds: Vec<u8>,
    output_count: u16,
    component_start: u16,
    component_count: u16,
    step_start: u16,
    step_count: u16,
}

fn parse_segment_source_values(
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

struct ParsedChainSource {
    id: String,
    segments: Vec<String>,
    link_count: u16,
}

fn parse_chain_source_values(
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

fn parse_input_kind(token: &str, context: &str) -> Result<u8, String> {
    match token {
        "public" => Ok(1),
        "linked" => Ok(2),
        "private" => Ok(3),
        _ => Err(format!("{context}: bad input kind: {token}")),
    }
}

fn parse_endpoint(token: &str, context: &str) -> Result<(u16, u16), String> {
    let Some((left, right)) = token.split_once('.') else {
        return Err(format!("{context}: bad chain endpoint: {token}"));
    };
    Ok((
        parse_u32(left, context)? as u16,
        parse_u32(right, context)? as u16,
    ))
}

fn write_execution_report(
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

fn write_segment_report(
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

fn execution_report_bytes(
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

fn segment_report_bytes(
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

fn path_relative_to(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn hex_to_32(hex: &str) -> Result<[u8; 32], String> {
    let bytes = parse_hex(hex).ok_or_else(|| "invalid sha256 hex".to_owned())?;
    let array: [u8; 32] = bytes
        .try_into()
        .map_err(|_| "sha256 hex is not 32 bytes".to_owned())?;
    Ok(array)
}

fn verify_binary_report(
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

fn cmd_list() -> i32 {
    for manifest in units() {
        println!(
            "{}\n  standard: {}\n  wasm: {}\n  sha256: {}",
            manifest.id, manifest.standard, manifest.wasm_path, manifest.wasm_sha256
        );
    }
    0
}

fn cmd_explain(id: Option<&str>) -> i32 {
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

fn cmd_verify(id: Option<&str>) -> i32 {
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

fn verify_composition(manifest: &CompositionManifest) -> bool {
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

fn verify_segment(manifest: &SegmentManifest) -> bool {
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

fn verify_chain(manifest: &ChainManifest) -> bool {
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

fn verify_binary_chain(expected: &ChainManifest, actual: &edgerun_wire::ChainRecord) -> bool {
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

fn verify_binary_segment(expected: &SegmentManifest, actual: &edgerun_wire::SegmentRecord) -> bool {
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

fn verify_binary_segment_report(
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

fn replay_binary_segment_report(
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

const ESIG_ALGORITHM_ED25519: u16 = 1;
const ESIG_DOMAIN: &[u8] = b"edgerun-sdk.esig.v1.segment-report";

fn signature_bytes(report_bytes: &[u8], signing_key: &SigningKey) -> Vec<u8> {
    signature_bytes_for_domain(report_bytes, signing_key, ESIG_DOMAIN)
}

fn signature_bytes_for_domain(
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

fn signer_policy_bytes(
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

fn parse_artifact_signature_record(bytes: &[u8]) -> Option<edgerun_wire::ArtifactSignature> {
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

fn parse_signer_policy_record(bytes: &[u8]) -> Option<edgerun_wire::SignerPolicy> {
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

fn verify_binary_signature(
    report_bytes: &[u8],
    signature: &edgerun_wire::ArtifactSignature,
) -> bool {
    verify_signature_for_domain(report_bytes, signature, ESIG_DOMAIN)
}

fn verify_signature_for_domain(
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

fn verify_binary_signer_policy(
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

fn signature_payload(report_hash: &[u8; 32]) -> Vec<u8> {
    signature_payload_for_domain(ESIG_DOMAIN, report_hash)
}

fn signature_payload_for_domain(domain: &[u8], report_hash: &[u8; 32]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(domain.len() + report_hash.len());
    payload.extend_from_slice(domain);
    payload.extend_from_slice(report_hash);
    payload
}

fn verify_chain_reports(
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

fn verify_http_auth_preflight_composition(manifest: &CompositionManifest, bytes: &[u8]) -> bool {
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

fn verify_auth_decision_private_composition(manifest: &CompositionManifest, bytes: &[u8]) -> bool {
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

fn verify_hmac_sha256_verify_composition(manifest: &CompositionManifest, bytes: &[u8]) -> bool {
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

fn verify_hmac_sha256_composition(manifest: &CompositionManifest, bytes: &[u8]) -> bool {
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

fn verify_hkdf_expand_sha256_l42_composition(manifest: &CompositionManifest, bytes: &[u8]) -> bool {
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

fn verify_hkdf_extract_sha256_composition(manifest: &CompositionManifest, bytes: &[u8]) -> bool {
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

fn verify_unit(manifest: &UnitManifest) -> bool {
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

fn verify_binary_manifest(expected: &UnitManifest, bytes: &[u8]) -> bool {
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

fn verify_binary_api(expected: &UnitManifest, bytes: &[u8]) -> bool {
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

fn api_cost_profile(name: &str) -> Option<(u32, u32)> {
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

fn verify_binary_composition(expected: &CompositionManifest, bytes: &[u8]) -> bool {
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

fn valid_len_source(value: u32) -> bool {
    let kind = value >> 24;
    kind <= 3
}

fn valid_arg_ref(value: u32) -> bool {
    let kind = value >> 24;
    kind <= 3
}

fn valid_comparison(value: u32) -> bool {
    let comparison = value >> 28;
    let kind = (value >> 24) & 0x0f;
    comparison <= 3 && kind <= 3
}

struct ComponentRuntime {
    _library: NativeLibrary,
    memory: NativeMemory,
    pointer_args_are_offsets: bool,
    functions: Vec<FunctionRuntime>,
}

#[derive(Clone)]
struct FunctionRuntime {
    func: NativeFunction,
    name: String,
    param_count: usize,
    pointer_args: [bool; 5],
    output_arg: Option<usize>,
    output_pointer_fields: Vec<usize>,
    cost_base: u32,
    cost_per_byte: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct NativeAbiFunction {
    name: String,
    pointer_args: [bool; 5],
    output_arg: Option<usize>,
    output_pointer_fields: Vec<usize>,
}

#[derive(Clone, Copy)]
enum NativeFunction {
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

struct NativeMemory {
    ptr: *mut u8,
    len: usize,
    owned: bool,
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

struct ExecutionReport {
    output: Vec<u8>,
    cost: u64,
}

struct CostQuote {
    cost: u64,
    output_len: u32,
    steps_executed: usize,
}

struct PreflightReport {
    cost_min: u64,
    cost_max: u64,
    output_len: Option<u32>,
    steps_min: usize,
    steps_max: usize,
    unknowns: Vec<String>,
    possible_failures: Vec<String>,
}

#[derive(Clone)]
struct PreflightState {
    pc: usize,
    cost_min: u64,
    cost_max: u64,
    steps: usize,
    output_len: Option<u32>,
    scalars: [SymVal; 256],
}

#[derive(Clone, Copy)]
enum SymVal {
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

fn execute_composition(
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

fn load_component_runtime(root: &Path, unit_id: &str) -> Result<ComponentRuntime, String> {
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

fn native_abi_for_unit(root: &Path, unit_id: &str) -> Result<Vec<NativeAbiFunction>, String> {
    let path = root.join("units").join(unit_id).join("rust/src/lib.rs");
    let source = fs::read_to_string(&path)
        .map_err(|err| format!("cannot read native abi source {}: {err}", path.display()))?;
    parse_native_abi_source(&source).map_err(|err| format!("{unit_id}: {err}"))
}

fn parse_native_abi_source(source: &str) -> Result<Vec<NativeAbiFunction>, String> {
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

fn parse_export_signature(signature: &str) -> Result<(String, Vec<String>), String> {
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

fn native_arg_is_pointer(name: &str) -> bool {
    name == "ptr" || name.ends_with("_ptr") || name.ends_with("_out")
}

fn native_arg_is_output_pointer(name: &str) -> bool {
    name == "out_ptr"
        || name.ends_with("_out_ptr")
        || name.ends_with("output_ptr")
        || name.ends_with("_out")
}

fn parse_native_output_pointer_fields(lines: &[&str], arg_names: &[String]) -> Vec<usize> {
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

fn parse_out_add_field(line: &str) -> Option<usize> {
    let start = line.find("out.add(")? + "out.add(".len();
    let end = line[start..].find(')')?;
    line[start..start + end].trim().parse().ok()
}

fn quote_composition(
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

fn preflight_composition(
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
struct FunctionCost {
    name: String,
    cost_base: u32,
    cost_per_byte: u32,
}

fn load_function_tables(
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

fn component_mut(
    components: &mut [ComponentRuntime],
    index: u8,
) -> Result<&mut ComponentRuntime, String> {
    components
        .get_mut(index as usize)
        .ok_or_else(|| "component index out of range".to_owned())
}

fn metered_len_u32(name: &str, args: &[u32; 5]) -> u32 {
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

fn metered_len_sym(name: &str, args: &[SymVal; 5]) -> SymVal {
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

fn captured_result_range(name: &str) -> SymVal {
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

fn resolve_sym_ref(
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

fn compare_sym_refs(
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

fn resolve_ref(value: u32, inputs: &[&[u8]], scalars: &[u32; 256]) -> Result<u32, String> {
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

fn resolve_ref_len(
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

fn compare_refs(
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

fn compare_refs_len(
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

fn parse_hex(input: &str) -> Option<Vec<u8>> {
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

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[derive(Default)]
struct WasmSurface {
    valid: bool,
    import_count: usize,
    imports_memory: bool,
    exports: Vec<ExportSurface>,
}

struct ExportSurface {
    name: String,
    kind: ExternalKind,
    ty: Option<FuncType>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExternalKind {
    Func,
    Memory,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct FuncType {
    params: Vec<ValType>,
    results: Vec<ValType>,
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
enum ValType {
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

struct WasmCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
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

fn verify_surface(manifest: &UnitManifest, surface: &WasmSurface) -> bool {
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

fn verify_export(expected: &ApiFunction, surface: &WasmSurface) -> bool {
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

fn parse_api_type(ty: &str) -> Option<(Vec<ValType>, Vec<ValType>)> {
    let (params, results) = ty.split_once("->")?;
    Some((parse_val_types(params), parse_val_types(results)))
}

fn parse_val_types(text: &str) -> Vec<ValType> {
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

fn valtypes_to_api_bytes(types: &[ValType]) -> Vec<u8> {
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

fn print_api(items: &[ApiFunction]) {
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

fn print_check(name: &str, ok: bool) {
    println!("  {}: {}", name, if ok { "ok" } else { "failed" });
}

#[cfg(test)]
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
unsafe fn sha256_digest(_input_ptr: i32, _input_len: i32, _out_ptr: i32) -> i32 {
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

unsafe fn sha256_digest(_input_ptr: i32, _input_len: i32, _out_ptr: i32) -> i32 {
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
unsafe fn sha256_digest(_input_ptr: i32, _input_len: i32, _out_ptr: i32) -> i32 {
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
fn http_tchar_valid(value: i32) -> i32 {
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

const RFC: i32 = 9110;
edgerun_unit::metadata!(RFC);

#[edgerun_unit::export]
fn http_tchar_valid(value: i32) -> i32 {
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
