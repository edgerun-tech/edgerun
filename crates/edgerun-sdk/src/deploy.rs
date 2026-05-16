use super::*;

pub(crate) fn cmd_deploy_inventory(args: Vec<String>) -> i32 {
    if args.is_empty() || args.first().map(String::as_str) == Some("--local") {
        let report = crate::deploy_support::host::capability_report_local();
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

pub(crate) fn cmd_deploy_ssh_inventory(target: &str) -> i32 {
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
    match crate::ssh_support::host::exec_with_ed25519_key_file_and_input(
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

pub(crate) fn cmd_deploy_ssh_probe(args: Vec<String>) -> i32 {
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
    match crate::ssh_support::host::probe_server(&target, Duration::from_secs(5)) {
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

pub(crate) fn cmd_deploy_ssh_exec(args: Vec<String>) -> i32 {
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
    match crate::ssh_support::host::exec_with_ed25519_key_file(
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

pub(crate) fn cmd_deploy_server(args: Vec<String>) -> i32 {
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
    match crate::ssh_support::host::exec_with_ed25519_key_file_and_input(
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

pub(crate) fn default_ssh_key_path() -> PathBuf {
    if let Ok(path) = env::var("EDGERUN_SSH_KEY") {
        return PathBuf::from(path);
    }
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_owned());
    Path::new(&home).join(".ssh/id_ed25519")
}

pub(crate) fn build_edgerun_server_for_remote() -> Result<PathBuf, String> {
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
            "std,http,smtp,imap,dns,tls,acme",
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

pub(crate) fn build_machine_report_for_remote() -> Result<PathBuf, String> {
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

pub(crate) fn remote_machine_report_inventory_sh(host: &str) -> String {
    let mut script = String::from(REMOTE_MACHINE_REPORT_INVENTORY_SH);
    script.push_str("\"$path\" capability-report \"$EDGERUN_REPORT_HOST\"\n");
    script.replace("EDGERUN_REPORT_HOST_PLACEHOLDER", &shell_single_quote(host))
}

pub(crate) fn shell_single_quote(value: &str) -> String {
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

pub(crate) const REMOTE_MACHINE_REPORT_INVENTORY_SH: &str = r#"set -eu
EDGERUN_REPORT_HOST=EDGERUN_REPORT_HOST_PLACEHOLDER
path="/tmp/edgerun-machine-report-$$"
cleanup() { rm -f "$path"; }
trap cleanup EXIT HUP INT TERM
cat > "$path"
chmod 700 "$path"
"#;

pub(crate) const REMOTE_DEPLOY_SERVER_SH: &str = r#"set -eu
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
