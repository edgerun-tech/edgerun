//! OCI-compatible CLI wrapper for edgerun-oci-runtime.
//!
//! Implements the commands needed by the `opencontainers/runtime-tools` conformance suite:
//! - `create` — Set up container, clone namespaces, prepare rootfs, fork but don't exec
//! - `start` — Resume exec the container process
//! - `state` — Output container state JSON to stdout
//! - `kill` — Send signal to container process
//! - `delete` — Stop and cleanup container state

use std::fs;
use std::io::{self, Write};
use std::os::fd::AsRawFd;
use std::os::raw::c_int;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use edgerun_oci_runtime::json::{OciSpec, parse_oci_spec, OciLinuxDevice};
use edgerun_oci_runtime::process::ContainerConfig;
use edgerun_oci_runtime::rootfs::{setup_rootfs, apply_sysctl, set_rootfs_propagation};
use edgerun_oci_runtime::seccomp::apply_seccomp_from_spec;
use edgerun_oci_runtime::syscalls::{
    do_unshare, do_setns, do_set_hostname, do_setrlimit, do_umask,
    rlimit_name_to_int,
};
use edgerun_oci_runtime::userns::{
    apply_security_hardening, set_capabilities, do_setgid, do_setuid,
    set_supplementary_gids,
};

const STATE_DIR: &str = "/run/edgerun-oci";

/// Container state matching the OCI runtime spec JSON format.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ContainerState {
    #[serde(rename = "ociVersion")]
    oci_version: String,
    id: String,
    status: String, // "creating" | "created" | "running" | "stopped"
    #[serde(skip_serializing_if = "Option::is_none")]
    pid: Option<u32>,
    bundle: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    annotations: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Default)]
struct GlobalOpts {
    bundle: Option<PathBuf>,
    id: Option<String>,
    pid_file: Option<PathBuf>,
}

fn parse_global_args(args: &[String]) -> GlobalOpts {
    let mut opts = GlobalOpts::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--bundle" => {
                i += 1;
                if i < args.len() {
                    opts.bundle = Some(PathBuf::from(&args[i]));
                }
            }
            s if s.starts_with("--bundle=") => {
                opts.bundle = Some(PathBuf::from(&s["--bundle=".len()..]));
            }
            "--pid-file" => {
                i += 1;
                if i < args.len() {
                    opts.pid_file = Some(PathBuf::from(&args[i]));
                }
            }
            s if s.starts_with("--pid-file=") => {
                opts.pid_file = Some(PathBuf::from(&s["--pid-file=".len()..]));
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => {
                if opts.id.is_none() && !args[i].starts_with('-') {
                    opts.id = Some(args[i].clone());
                }
            }
        }
        i += 1;
    }
    opts
}

fn print_usage() {
    eprintln!("Usage: edgerun-oci [global-options] <command> [command-options]");
    eprintln!();
    eprintln!("Commands:");
    eprintln!("  create <container-id>  Create a container");
    eprintln!("  start <container-id>   Start a created container");
    eprintln!("  state <container-id>   Output state of a container");
    eprintln!("  kill <container-id>    Send signal to container");
    eprintln!("  delete <container-id>  Delete container resources");
    eprintln!();
    eprintln!("Global options:");
    eprintln!("  --bundle <path>    Path to bundle directory");
    eprintln!("  --pid-file <path>  Path to write container PID");
}

fn state_dir(id: &str) -> PathBuf {
    Path::new(STATE_DIR).join(id)
}

fn state_file(id: &str) -> PathBuf {
    state_dir(id).join("state.json")
}

fn fifo_path(id: &str) -> PathBuf {
    state_dir(id).join("start.fifo")
}

fn save_state(state: &ContainerState, id: &str) -> io::Result<()> {
    let dir = state_dir(id);
    fs::create_dir_all(&dir)?;
    let json = serde_json::to_string_pretty(state)?;
    fs::write(state_file(id), json)?;
    Ok(())
}

fn load_state(id: &str) -> io::Result<ContainerState> {
    let data = fs::read_to_string(state_file(id))?;
    serde_json::from_str(&data).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn delete_state(id: &str) {
    let _ = fs::remove_dir_all(state_dir(id));
}

/// Retry reading a file that may still be being written to.
fn retry_read(path: &Path, max_retries: u32) -> io::Result<Vec<u8>> {
    for i in 0..max_retries {
        if i > 0 {
            std::thread::sleep(std::time::Duration::from_millis(100 * i as u64));
        }
        let data = fs::read(path)?;
        let trimmed = data.iter()
            .skip_while(|b| **b == b' ' || **b == b'\t' || **b == b'\n' || **b == b'\r')
            .copied()
            .collect::<Vec<_>>();
        if !trimmed.is_empty() && (trimmed[0] == b'{' || trimmed[0] == b'[') {
            return Ok(trimmed);
        }
    }
    fs::read(path)
}

fn cmd_create(args: &[String]) -> io::Result<()> {
    let opts = parse_global_args(args);
    let bundle = opts.bundle.as_deref().unwrap_or(Path::new("."));
    let id = opts.id.as_deref().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "container ID is required for create")
    })?;

    // Check for duplicate ID
    if load_state(id).is_ok() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists,
            format!("container ID {} already exists", id)));
    }

    let config_path = bundle.join("config.json");
    let config_data = retry_read(&config_path, 5)?;
    let spec: OciSpec = parse_oci_spec(&config_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid OCI config: {}", e)))?;

    // Create the FIFO for start synchronization
    let dir = state_dir(id);
    fs::create_dir_all(&dir)?;
    let fifo = fifo_path(id);
    let _ = fs::remove_file(&fifo);
    let mkfifo_output = Command::new("mkfifo").arg(&fifo).output()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("mkfifo failed: {}", e)))?;
    if !mkfifo_output.status.success() {
        return Err(io::Error::new(io::ErrorKind::Other,
            format!("mkfifo failed: {}", String::from_utf8_lossy(&mkfifo_output.stderr))));
    }

    // Fork: the child enters namespaces and sets up rootfs, then blocks on FIFO read
    let child_pid = unsafe { libc::fork() };
    if child_pid < 0 {
        return Err(io::Error::last_os_error());
    }

    if child_pid == 0 {
        // Child process
        let fifo_str = fifo.to_string_lossy().to_string();
        let result = setup_child_for_create(&spec, &fifo_str);
        match result {
            Ok(()) => {
                // Exec the container process — but the parent hasn't called start yet.
                // We need to block on the FIFO first, then exec.
                // setup_child_for_create already opened the FIFO for reading which will block
                // until the parent (start command) opens it for writing.
                // After the FIFO is opened by start, we exec here.
                exec_process(&spec);
                std::process::exit(1);
            }
            Err(e) => {
                eprintln!("edgerun-oci: create failed in child: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Parent: write PID to pid-file if requested
    if let Some(ref pid_file) = opts.pid_file {
        fs::write(pid_file, format!("{}", child_pid))?;
    }

    // Save state as 'created'
    let state = ContainerState {
        oci_version: spec.version.clone(),
        id: id.to_string(),
        status: "created".to_string(),
        pid: Some(child_pid as u32),
        bundle: bundle.to_string_lossy().to_string(),
        annotations: spec.annotations.clone(),
    };
    save_state(&state, id)?;
    Ok(())
}

fn setup_child_for_create(spec: &OciSpec, fifo_path: &str) -> io::Result<()> {
    let cfg = ContainerConfig::from_spec(spec)?;

    // Unshare namespaces
    do_unshare(cfg.ns_flags)?;

    // Join explicit namespace paths
    join_explicit_namespaces(&cfg.ns_paths)?;

    // UID/GID mapping
    write_uid_map(&cfg.uid_map)?;
    write_gid_map(&cfg.gid_map)?;

    // Hostname
    let _ = do_set_hostname(&cfg.hostname);

    // Security: no_new_privs + non-dumpable
    apply_security_hardening(cfg.no_new_privs)?;

    // Capabilities
    set_capabilities(
        cfg.cap_effective.as_deref(),
        cfg.cap_permitted.as_deref(),
        cfg.cap_inheritable.as_deref(),
        cfg.cap_bounding.as_deref(),
        cfg.cap_ambient.as_deref(),
    )?;

    // Seccomp — fail-closed
    apply_seccomp_from_spec(cfg.seccomp.as_ref()).map_err(|e| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("seccomp filter failed to apply: {}", e),
        )
    })?;

    // Resource limits
    for rl in &cfg.rlimits {
        if let Some(resource) = rlimit_name_to_int(&rl.ns_type) {
            let _ = do_setrlimit(resource, rl.soft, rl.hard);
        } else {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("invalid RLIMIT type: {}", rl.ns_type)));
        }
    }

    // OOM score
    if cfg.oom_score_adj != 0 {
        let _ = fs::write("/proc/self/oom_score_adj", format!("{}", cfg.oom_score_adj));
    }

    // AppArmor
    if let Some(ref profile) = cfg.apparmor_profile {
        let _ = fs::write("/proc/self/attr/apparmor/exec", format!("exec {}", profile));
    }

    // Umask
    if let Some(mask) = cfg.umask {
        do_umask(mask);
    }

    // Rootfs
    let devices = deserialize_devices(&cfg.devices_json);
    let mount_label = cfg.mount_label.as_deref();
    setup_rootfs(
        &cfg.root,
        cfg.mounts.as_deref(),
        cfg.masked_paths.as_deref(),
        cfg.readonly_paths.as_deref(),
        if devices.is_empty() { None } else { Some(&devices) },
        mount_label,
        true,
        true,
    )?;

    // Rootfs propagation
    set_rootfs_propagation(cfg.rootfs_propagation.as_deref())?;

    // Sysctl
    apply_sysctl(cfg.sysctl.as_ref())?;

    // Supplementary groups
    if !cfg.additional_gids.is_empty() {
        set_supplementary_gids(&cfg.additional_gids);
    }

    // Drop GID then UID
    do_setgid(cfg.gid)?;
    do_setuid(cfg.uid)?;

    // Block waiting for start signal via FIFO
    // Opening the FIFO for read blocks until someone opens it for write
    let _fifo_fd = fs::File::open(fifo_path).map_err(|e| {
        io::Error::new(io::ErrorKind::Other, format!("failed to open start FIFO: {}", e))
    })?;

    Ok(())
}

fn deserialize_devices(json: &str) -> Vec<OciLinuxDevice> {
    if json.is_empty() { return Vec::new(); }
    edgerun_json::from_slice::<Vec<OciLinuxDevice>>(json.as_bytes())
        .unwrap_or_default()
}

fn join_explicit_namespaces(ns_paths: &str) -> io::Result<()> {
    if ns_paths.is_empty() { return Ok(()); }
    for entry in ns_paths.split('\n') {
        if let Some((ns_type, path)) = entry.split_once(':') {
            if let Some(flag) = ns_type_to_flag(ns_type) {
                let fd = fs::File::open(path)
                    .map_err(|e| io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("cannot open namespace {}: {}", path, e)
                    ))?;
                do_setns(fd.as_raw_fd(), flag)?;
            }
        }
    }
    Ok(())
}

fn ns_type_to_flag(ns_type: &str) -> Option<i32> {
    match ns_type {
        "mount"   => Some(0x00020000),  // CLONE_NEWNS
        "cgroup"  => Some(0x02000000),  // CLONE_NEWCGROUP
        "uts"     => Some(0x04000000),  // CLONE_NEWUTS
        "ipc"     => Some(0x08000000),  // CLONE_NEWIPC
        "user"    => Some(0x10000000),  // CLONE_NEWUSER
        "pid"     => Some(0x20000000),  // CLONE_NEWPID
        "network" => Some(0x40000000),  // CLONE_NEWNET
        _ => None,
    }
}

fn write_uid_map(content: &str) -> io::Result<()> {
    fs::write("/proc/self/uid_map", content)?;
    let _ = fs::write("/proc/self/setgroups", "deny");
    Ok(())
}

fn write_gid_map(content: &str) -> io::Result<()> {
    fs::write("/proc/self/gid_map", content)?;
    Ok(())
}

fn exec_process(spec: &OciSpec) {
    let process = spec.process.clone().unwrap_or_default();
    let args = process.args.clone().unwrap_or_else(|| vec!["/bin/sh".into()]);
    let env = process.env.clone().unwrap_or_else(|| vec![
        "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
        "TERM=xterm".into(),
    ]);
    let cwd = process.cwd.clone().unwrap_or_else(|| "/".into());

    let use_pid1 = has_pid_ns(spec);
    let init_script = if use_pid1 {
        pid1_init_script(&args)
    } else {
        String::new()
    };

    let mut cmd = if use_pid1 {
        let mut c = Command::new("/bin/sh");
        c.arg("-c");
        c.arg(&init_script);
        c
    } else {
        let mut c = Command::new(&args[0]);
        c.args(&args[1..]);
        c
    };
    cmd.current_dir(&cwd);
    cmd.env_clear();
    for e in &env {
        if let Some((k, v)) = e.split_once('=') {
            cmd.env(k, v);
        }
    }
    cmd.stdin(Stdio::inherit());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let _ = cmd.exec();
    std::process::exit(1);
}

fn has_pid_ns(spec: &OciSpec) -> bool {
    let linux = spec.linux.as_ref();
    let ns_list = linux.and_then(|l| l.namespaces.as_ref()).map(|x| x.as_slice()).unwrap_or(&[]);
    if ns_list.is_empty() { return true; }
    for ns in ns_list {
        if ns.ns_type == "pid" && ns.path.is_none() { return true; }
    }
    false
}

fn pid1_init_script(args: &[String]) -> String {
    let workload = args.iter()
        .map(|a| a.replace('\'', "'\\''"))
        .map(|a| format!("'{}'", a))
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        r#"#!/bin/sh
cleanup() {{
    kill -$1 $PID 2>/dev/null
}}
trap 'cleanup 15' TERM
trap 'cleanup 2' INT
trap 'cleanup 3' QUIT
{workload} &
PID=$!
while true; do
    wait $PID 2>/dev/null
    EXIT_CODE=$?
    while kill -0 $PID 2>/dev/null; do
        sleep 0.1
    done
    exit $EXIT_CODE
done
"#
    )
}

fn cmd_start(args: &[String]) -> io::Result<()> {
    let id = args.first().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "container ID required")
    })?;

    let mut state = load_state(id)?;
    if state.status != "created" {
        return Err(io::Error::new(io::ErrorKind::InvalidInput,
            format!("container {} is not in 'created' state (status: {})", id, state.status)));
    }

    // Open the FIFO for writing — this unblocks the child's blocking read
    let fifo = fifo_path(id);
    let mut fifo_file = fs::File::create(&fifo)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("failed to open start FIFO: {}", e)))?;
    let _ = fifo_file.write_all(b"go\n");
    let _ = fifo_file.flush();
    // Keep the FIFO open briefly to ensure the reader gets the data
    std::thread::sleep(std::time::Duration::from_millis(100));

    state.status = "running".to_string();
    save_state(&state, id)?;
    Ok(())
}

fn cmd_state(args: &[String]) -> io::Result<()> {
    let id = args.first().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "container ID required")
    })?;

    let state = load_state(id)?;
    let mut updated_state = state.clone();

    // Check if process is still alive
    if let Some(pid) = state.pid {
        let alive = unsafe { libc::kill(pid as c_int, 0) == 0 };
        if !alive && state.status == "running" {
            updated_state.status = "stopped".to_string();
            save_state(&updated_state, id)?;
        }
    }

    // Output matching rspecs.State JSON format
    let pid_val = updated_state.pid.unwrap_or(0);
    let output = serde_json::json!({
        "ociVersion": updated_state.oci_version,
        "id": updated_state.id,
        "status": updated_state.status,
        "pid": pid_val,
        "bundle": updated_state.bundle,
        "annotations": updated_state.annotations.unwrap_or_default(),
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn cmd_kill(args: &[String]) -> io::Result<()> {
    let id = args.first().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "container ID required")
    })?;
    let sig_str = args.get(1).map(|s| s.as_str()).unwrap_or("TERM");

    let state = load_state(id)?;
    let pid = state.pid.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "container has no PID")
    })?;

    let sig = parse_signal(sig_str)?;
    let ret = unsafe { libc::kill(pid as c_int, sig) };
    if ret != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn parse_signal(s: &str) -> io::Result<c_int> {
    // Strip optional "SIG" prefix
    let name = s.strip_prefix("SIG").unwrap_or(s);
    if let Ok(n) = s.parse::<c_int>() {
        return Ok(n);
    }
    match name {
        "HUP" | "SIGHUP" => Ok(1),
        "INT" | "SIGINT" => Ok(2),
        "QUIT" | "SIGQUIT" => Ok(3),
        "KILL" | "SIGKILL" => Ok(9),
        "TERM" | "SIGTERM" => Ok(15),
        "CONT" | "SIGCONT" => Ok(18),
        "STOP" | "SIGSTOP" => Ok(19),
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("unknown signal: {}", s))),
    }
}

fn cmd_delete(args: &[String]) -> io::Result<()> {
    let mut force = false;
    let mut id: Option<&str> = None;

    for arg in args {
        match arg.as_str() {
            "--force" => force = true,
            _ => {
                if id.is_none() {
                    id = Some(arg.as_str());
                }
            }
        }
    }

    let id = id.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "container ID required")
    })?;

    if let Ok(state) = load_state(id) {
        if let Some(pid) = state.pid {
            let alive = unsafe { libc::kill(pid as c_int, 0) == 0 };
            if alive && state.status == "running" && !force {
                return Err(io::Error::new(io::ErrorKind::InvalidInput,
                    format!("container {} is still running, use --force", id)));
            }
            if alive {
                unsafe { libc::kill(pid as c_int, libc::SIGKILL) };
                // Wait briefly for exit
                unsafe { libc::usleep(50000) };
            }
        }
    }

    // Clean up state dir and FIFO
    delete_state(id);
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        print_usage();
        std::process::exit(1);
    }

    // Find the command — it's the first non-flag argument after global options
    let mut cmd_idx = None;
    let global_args: Vec<String> = {
        let mut result = Vec::new();
        for (i, arg) in args.iter().enumerate().skip(1) {
            if arg.starts_with('-') {
                result.push(arg.clone());
            } else {
                cmd_idx = Some(i);
                // Collect remaining args as command args
                for remaining in args.iter().skip(i + 1) {
                    result.push(remaining.clone());
                }
                break;
            }
        }
        result
    };

    let command = cmd_idx.map(|i| &args[i]).unwrap_or_else(|| {
        print_usage();
        std::process::exit(1);
    });

    let result = match command.as_str() {
        "create" => cmd_create(&global_args),
        "start" => cmd_start(&global_args),
        "state" => cmd_state(&global_args),
        "kill" => cmd_kill(&global_args),
        "delete" => cmd_delete(&global_args),
        _ => {
            eprintln!("Unknown command: {}", command);
            print_usage();
            std::process::exit(127);
        }
    };

    if let Err(e) = result {
        eprintln!("edgerun-oci: {}: {}", command, e);
        std::process::exit(1);
    }
}
