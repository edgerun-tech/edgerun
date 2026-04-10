//! Child process setup — runs in `pre_exec` before `execve`, or as a `clone()` entry point.
//!
//! This module contains all the namespace, rootfs, security, and privilege
//! setup that must happen in the forked child before the container process
//! is exec'd.

use std::ffi::CString;
use std::fs;
use std::io;
use std::os::unix::io::AsRawFd;

use crate::json::{OciIdMapping, OciLinuxDevice, OciRoot, OciSpec};
use crate::rootfs::{setup_rootfs, apply_sysctl, set_rootfs_propagation};
use crate::seccomp::apply_seccomp_from_spec;
use crate::syscalls::{
    do_set_hostname, do_unshare, do_setns, do_setrlimit, do_umask, rlimit_name_to_int,
};
use crate::userns::{
    apply_security_hardening, set_capabilities, do_setgid, do_setuid,
    set_supplementary_gids,
};

/// Default environment variables when none are specified in the OCI spec.
pub const DEFAULT_ENV: &[&str] = &[
    "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
    "TERM=xterm",
];

/// Extract all container-relevant config from a spec into a flat struct
/// that can be cloned into a `pre_exec` closure.
pub struct ContainerConfig {
    pub ns_flags: i32,
    pub ns_paths: String,
    pub uid_map: String,
    pub gid_map: String,
    pub hostname: String,
    pub no_new_privs: bool,
    pub cap_effective: Option<Vec<String>>,
    pub cap_permitted: Option<Vec<String>>,
    pub cap_inheritable: Option<Vec<String>>,
    pub cap_bounding: Option<Vec<String>>,
    pub cap_ambient: Option<Vec<String>>,
    pub rlimits: Vec<crate::json::OciRlimit>,
    pub oom_score_adj: i64,
    pub apparmor_profile: Option<String>,
    pub umask: Option<u32>,
    pub root: OciRoot,
    pub mounts: Option<Vec<crate::json::OciMount>>,
    pub masked_paths: Option<Vec<String>>,
    pub readonly_paths: Option<Vec<String>>,
    pub devices_json: String,
    pub rootfs_propagation: Option<String>,
    pub sysctl: Option<std::collections::HashMap<String, String>>,
    pub additional_gids: Vec<u32>,
    pub uid: u32,
    pub gid: u32,
    pub seccomp: Option<crate::json::OciLinuxSeccomp>,
    pub mount_label: Option<String>,
}

impl ContainerConfig {
    /// Extract all config needed for pre_exec from an OCI spec.
    pub fn from_spec(spec: &OciSpec) -> io::Result<Self> {
        let root = spec.root.clone()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no root in OCI spec"))?;

        let linux = spec.linux.clone().unwrap_or_default();
        let process = spec.process.clone().unwrap_or_default();
        let user = process.user.clone().unwrap_or_default();

        let ns_list = linux.namespaces.clone().unwrap_or_else(crate::default_namespaces);
        let ns_flags = crate::namespace_flags(&ns_list);

        // Validate namespace types — reject unknown types
        for ns in &ns_list {
            if ns.path.is_none() {
                match ns.ns_type.as_str() {
                    "mount" | "cgroup" | "uts" | "ipc" | "user" | "pid" | "network" => {}
                    _ => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidInput,
                            format!("unknown namespace type: {}", ns.ns_type),
                        ));
                    }
                }
            }
        }

        let ns_paths = serialize_ns_paths(linux.namespaces.as_deref());
        let uid_map = format_mapping(linux.uid_mappings.as_deref());
        let gid_map = format_mapping(linux.gid_mappings.as_deref());

        let caps = process.capabilities.clone().unwrap_or_default();

        Ok(Self {
            ns_flags,
            ns_paths,
            uid_map,
            gid_map,
            hostname: spec.hostname.clone().unwrap_or_else(|| "edgerun".into()),
            no_new_privs: process.no_new_privileges.unwrap_or(true),
            cap_effective: caps.effective,
            cap_permitted: caps.permitted,
            cap_inheritable: caps.inheritable,
            cap_bounding: caps.bounding,
            cap_ambient: caps.ambient,
            rlimits: process.rlimits.clone().unwrap_or_default(),
            oom_score_adj: process.oom_score_adj.unwrap_or(0),
            apparmor_profile: process.apparmor_profile,
            umask: user.umask,
            root,
            mounts: spec.mounts.clone(),
            masked_paths: linux.masked_paths.clone(),
            readonly_paths: linux.readonly_paths.clone(),
            devices_json: serialize_devices(linux.devices.as_deref()),
            rootfs_propagation: linux.rootfs_propagation.clone(),
            sysctl: linux.sysctl.clone(),
            additional_gids: user.additional_gids.unwrap_or_default(),
            uid: user.uid.unwrap_or(0),
            gid: user.gid.unwrap_or(0),
            seccomp: linux.seccomp,
            mount_label: linux.mount_label.clone(),
        })
    }

    /// Returns true if PID namespace is unshared (not joined via path).
    pub fn has_pid_ns(&self) -> bool {
        (self.ns_flags & crate::syscalls::ns::NEWPID) != 0
    }
}

// ===========================================================================
// Serialization helpers
// ===========================================================================

fn serialize_devices(devices: Option<&[OciLinuxDevice]>) -> String {
    match devices {
        Some(devs) => edgerun_json::to_string(devs).unwrap_or_default(),
        None => String::new(),
    }
}

fn serialize_ns_paths(namespaces: Option<&[crate::json::OciNamespace]>) -> String {
    match namespaces {
        Some(ns) => ns.iter()
            .filter_map(|n| n.path.as_ref().map(|p| format!("{}:{}", n.ns_type, p)))
            .collect::<Vec<_>>()
            .join("\n"),
        None => String::new(),
    }
}

fn format_mapping(mappings: Option<&[OciIdMapping]>) -> String {
    if let Some(maps) = mappings {
        if maps.is_empty() {
            return "0 65534 1\n".to_string();
        }
        maps.iter()
            .map(|m| format!("{} {} {}\n", m.container_id, m.host_id, m.size))
            .collect()
    } else {
        "0 65534 1\n".to_string()
    }
}

fn deserialize_devices(json: &str) -> Vec<OciLinuxDevice> {
    if json.is_empty() { return Vec::new(); }
    edgerun_json::from_slice::<Vec<OciLinuxDevice>>(json.as_bytes())
        .unwrap_or_default()
}

fn ns_type_to_flag(ns_type: &str) -> Option<i32> {
    use crate::syscalls::ns;
    match ns_type {
        "mount"   => Some(ns::NEWNS),
        "cgroup"  => Some(ns::NEWCGROUP),
        "uts"     => Some(ns::NEWUTS),
        "ipc"     => Some(ns::NEWIPC),
        "user"    => Some(ns::NEWUSER),
        "pid"     => Some(ns::NEWPID),
        "network" => Some(ns::NEWNET),
        _ => None,
    }
}

// ===========================================================================
// pre_exec closure — the actual container setup
// =========================================================================///

/// Run all container setup in the forked child before execve.
///
/// This implements OCI lifecycle steps:
/// - Create namespaces, join explicit paths
/// - UID/GID mapping, hostname
/// - Security: no_new_privs, capabilities, seccomp, rlimits, OOM, AppArmor, umask
/// - Rootfs: pivot_root, mounts, devices, sysctl, propagation
/// - Drop privileges (gid, uid)
pub fn setup_container_child(cfg: &ContainerConfig) -> io::Result<()> {
    // 1. Unshare namespaces
    do_unshare(cfg.ns_flags)?;

    // 2. Join explicit namespace paths
    join_explicit_namespaces(&cfg.ns_paths)?;

    // 3. UID/GID mapping
    write_uid_map(&cfg.uid_map)?;
    write_gid_map(&cfg.gid_map)?;

    // 4. Hostname
    let _ = do_set_hostname(&cfg.hostname);

    // 5. Security: no_new_privs + non-dumpable
    apply_security_hardening(cfg.no_new_privs)?;

    // 6. Capabilities
    set_capabilities(
        cfg.cap_effective.as_deref(),
        cfg.cap_permitted.as_deref(),
        cfg.cap_inheritable.as_deref(),
        cfg.cap_bounding.as_deref(),
        cfg.cap_ambient.as_deref(),
    )?;

    // 7. Seccomp — fail-closed (spec-driven or fallback allow-list)
    apply_seccomp_from_spec(cfg.seccomp.as_ref()).map_err(|e| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("seccomp filter failed to apply: {}. Container startup aborted for security.", e),
        )
    })?;

    // 8. Resource limits
    for rl in &cfg.rlimits {
        if let Some(resource) = rlimit_name_to_int(&rl.ns_type) {
            let _ = do_setrlimit(resource, rl.soft, rl.hard);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid RLIMIT type: {}", rl.ns_type),
            ));
        }
    }

    // 9. OOM score
    if cfg.oom_score_adj != 0 {
        let _ = fs::write("/proc/self/oom_score_adj", format!("{}", cfg.oom_score_adj));
    }

    // 10. AppArmor
    if let Some(ref profile) = cfg.apparmor_profile {
        let _ = fs::write("/proc/self/attr/apparmor/exec", format!("exec {}", profile));
    }

    // 11. Umask
    if let Some(mask) = cfg.umask {
        do_umask(mask);
    }

    // 12. Rootfs
    let devices = deserialize_devices(&cfg.devices_json);
    let mount_label = cfg.mount_label.as_deref();
    setup_rootfs(
        &cfg.root,
        cfg.mounts.as_deref(),
        cfg.masked_paths.as_deref(),
        cfg.readonly_paths.as_deref(),
        if devices.is_empty() { None } else { Some(&devices) },
        mount_label,
    )?;

    // 13. Rootfs propagation
    set_rootfs_propagation(cfg.rootfs_propagation.as_deref())?;

    // 14. Sysctl
    apply_sysctl(cfg.sysctl.as_ref())?;

    // 15. Supplementary groups
    if !cfg.additional_gids.is_empty() {
        set_supplementary_gids(&cfg.additional_gids);
    }

    // 16. Drop GID then UID
    do_setgid(cfg.gid)?;
    do_setuid(cfg.uid)?;

    Ok(())
}

// ===========================================================================
// Individual setup helpers
// ===========================================================================

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

fn write_uid_map(content: &str) -> io::Result<()> {
    // Only write uid_map if we're in a user namespace.
    // Writing to /proc/self/uid_map outside a user namespace fails with EPERM.
    if let Ok(ns) = fs::read_link("/proc/self/ns/user") {
        if let Ok(init_ns) = fs::read_link("/proc/1/ns/user") {
            if ns != init_ns {
                fs::write("/proc/self/uid_map", content)?;
                let _ = fs::write("/proc/self/setgroups", "deny");
                return Ok(());
            }
        }
    }
    // Not in a user namespace — skip uid_map writing (we keep current uid)
    Ok(())
}

fn write_gid_map(content: &str) -> io::Result<()> {
    // Same as uid_map — only write if in a user namespace
    if let Ok(ns) = fs::read_link("/proc/self/ns/user") {
        if let Ok(init_ns) = fs::read_link("/proc/1/ns/user") {
            if ns != init_ns {
                fs::write("/proc/self/gid_map", content)?;
                return Ok(());
            }
        }
    }
    Ok(())
}

// ===========================================================================
// clone() entry point and setup
// ===========================================================================

/// Data passed to the clone() child entry point.
pub struct CloneChildData {
    pub bundle: CString,
    pub fifo: CString,
    pub id: CString,
}

/// Entry point for the cloned child process.
/// 
/// This runs in the new namespaces from the start (no unshare needed).
/// Reads config from bundle, runs setup, waits on FIFO for start signal, then execs.
/// 
/// Returns 0 on success, 1 on failure.
pub fn cloned_child_main(data: CloneChildData) -> i32 {
    // Open FIFO for reading+writing (O_RDWR prevents EOF when no writers)
    let fifo_fd = unsafe {
        libc::open(data.fifo.as_ptr(), libc::O_RDWR)
    };
    if fifo_fd < 0 {
        return 1;
    }

    // Read the config.json from the bundle
    let config_path = format!("{}/config.json", data.bundle.to_string_lossy());
    let config_cstr = match CString::new(config_path.clone()) {
        Ok(c) => c,
        Err(_) => return 1,
    };
    let fd = unsafe { libc::open(config_cstr.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return 1;
    }

    // Read config data
    let mut buf = Vec::new();
    loop {
        let mut tmp = [0u8; 4096];
        let n = unsafe { libc::read(fd, tmp.as_mut_ptr() as *mut libc::c_void, tmp.len()) };
        if n <= 0 { break; }
        buf.extend_from_slice(&tmp[..n as usize]);
    }
    unsafe { libc::close(fd) };

    // Parse spec
    let spec = match crate::json::parse_oci_spec(&buf) {
        Ok(s) => s,
        Err(_) => return 1,
    };

    // Run the setup
    let bundle_lossy = data.bundle.to_string_lossy();
    let bundle_path = std::path::Path::new(bundle_lossy.as_ref());
    if setup_child_for_create(&spec, bundle_path).is_err() {
        return 1;
    }

    // Wait for start signal
    let mut start_buf = [0u8; 4];
    let n = unsafe { libc::read(fifo_fd, start_buf.as_mut_ptr() as *mut libc::c_void, start_buf.len()) };
    if n <= 0 {
        return 1;
    }

    // Exec the container
    exec_container_process(&spec);
    1 // Should not reach here if exec succeeds
}

/// Setup the container for the create command (clone path).
/// This runs setup without waiting for FIFO — the caller handles FIFO.
pub fn setup_child_for_create(spec: &OciSpec, bundle: &std::path::Path) -> io::Result<()> {
    // Change to bundle directory so relative rootfs paths work
    std::env::set_current_dir(bundle)?;

    let cfg = ContainerConfig::from_spec(spec)?;

    join_explicit_namespaces(&cfg.ns_paths)?;
    write_uid_map(&cfg.uid_map)?;
    write_gid_map(&cfg.gid_map)?;

    let _ = do_set_hostname(&cfg.hostname);

    apply_security_hardening(cfg.no_new_privs)?;

    // Capabilities — non-fatal, may fail in certain namespace configurations
    let _ = set_capabilities(
        cfg.cap_effective.as_deref(),
        cfg.cap_permitted.as_deref(),
        cfg.cap_inheritable.as_deref(),
        cfg.cap_bounding.as_deref(),
        cfg.cap_ambient.as_deref(),
    );

    // Seccomp — fail-closed (spec-driven or fallback allow-list)
    apply_seccomp_from_spec(cfg.seccomp.as_ref()).map_err(|e| {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("seccomp filter failed to apply: {}. Container startup aborted for security.", e),
        )
    })?;

    // Resource limits
    for rl in &cfg.rlimits {
        if let Some(resource) = rlimit_name_to_int(&rl.ns_type) {
            let _ = do_setrlimit(resource, rl.soft, rl.hard);
        }
    }

    // OOM score
    if cfg.oom_score_adj != 0 {
        let _ = fs::write("/proc/self/oom_score_adj", format!("{}", cfg.oom_score_adj));
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
    )?;

    set_rootfs_propagation(cfg.rootfs_propagation.as_deref())?;
    apply_sysctl(cfg.sysctl.as_ref())?;

    if !cfg.additional_gids.is_empty() {
        set_supplementary_gids(&cfg.additional_gids);
    }

    do_setgid(cfg.gid)?;
    do_setuid(cfg.uid)?;

    Ok(())
}

/// Exec the container process.
fn exec_container_process(spec: &OciSpec) {
    use crate::init::pid1_init_script;
    use std::process::{Command, Stdio};
    use std::os::unix::process::CommandExt;

    let process = spec.process.clone().unwrap_or_default();
    let args = process.args.clone().unwrap_or_else(|| vec!["/bin/sh".into()]);
    let env = process.env.clone().unwrap_or_else(|| DEFAULT_ENV.iter().map(|s| s.to_string()).collect());
    let cwd = process.cwd.clone().unwrap_or("/".into());

    let cfg = ContainerConfig::from_spec(spec).unwrap_or_else(|e| {
        eprintln!("failed to parse container config: {}", e);
        std::process::exit(1)
    });
    let init_script = if cfg.has_pid_ns() {
        pid1_init_script(&args)
    } else {
        String::new()
    };

    let mut cmd = if cfg.has_pid_ns() {
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
