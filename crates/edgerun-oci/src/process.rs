//! Child process setup — runs in `pre_exec` before `execve`, or as a `clone()` entry point.
//!
//! This module contains all the namespace, rootfs, security, and privilege
//! setup that must happen in the forked child before the container process
//! is exec'd.

use crate::prelude::*;
use std::fs;
use std::io;
use std::os::unix::io::AsRawFd;

use crate::linux_catalog::{namespace_flag, rlimit_number};
pub use crate::process_config::ContainerConfig;
use crate::rootfs::{apply_sysctl, set_rootfs_propagation, setup_rootfs, setup_rootfs_rootless};
#[allow(unused_imports)]
use crate::seccomp::apply_seccomp_from_spec;
use crate::spec::OciSpec;
use crate::syscalls::{do_set_hostname, do_setns, do_setrlimit, do_umask, do_unshare};
use crate::terminal::{send_fd, setup_pty_stdio};
use crate::userns::{
    apply_security_hardening, do_setgid, do_setuid, set_capabilities, set_supplementary_gids,
};

pub use crate::validate::DEFAULT_ENV;

/// Get the host OS string (e.g., "linux").
pub fn host_os() -> &'static str {
    crate::validate::host_os()
}

/// Get the host architecture string (e.g., "amd64").
pub fn host_arch() -> &'static str {
    crate::validate::host_arch()
}

/// Validate an OCI spec for known-invalid values before container creation.
pub fn validate_spec(spec: &OciSpec) -> io::Result<()> {
    crate::validate::validate_spec(spec)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

/// Map an OCI namespace type string to the corresponding CLONE_NEW* flag.
pub fn ns_type_to_flag(ns_type: &str) -> Option<i32> {
    namespace_flag(ns_type)
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
pub fn setup_container_child(
    cfg: &ContainerConfig,
    terminal_socket_fd: Option<i32>,
) -> io::Result<()> {
    do_unshare(cfg.ns_flags)?;
    join_explicit_namespaces(&cfg.ns_paths)?;
    write_uid_map(&cfg.uid_map)?;
    write_gid_map(&cfg.gid_map)?;
    setup_container_child_common(cfg, terminal_socket_fd, RootfsMode::Rooted)
}

enum RootfsMode {
    Rooted,
    Rootless,
}

fn setup_container_child_common(
    cfg: &ContainerConfig,
    terminal_socket_fd: Option<i32>,
    rootfs_mode: RootfsMode,
) -> io::Result<()> {
    let _ = do_set_hostname(&cfg.hostname);
    if let Some(ref domainname) = cfg.domainname {
        let _ = do_set_domainname(domainname);
    }

    // Security hardening and seccomp are applied AFTER rootfs setup because rootfs needs
    // mount/umount2/pivot_root syscalls that are NOT in the workload allow-list.
    // They will be applied just before exec, after runtime-only setup is done.

    for rl in &cfg.rlimits {
        if let Some(resource) = rlimit_number(&rl.ns_type) {
            let _ = do_setrlimit(resource, rl.soft, rl.hard);
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid RLIMIT type: {}", rl.ns_type),
            ));
        }
    }

    if let Some(ref sched) = cfg.scheduler {
        apply_scheduler(sched)?;
    }

    if let Some(ref ioprio) = cfg.io_priority {
        let _ = apply_io_priority(ioprio);
    }

    let _ = fs::write("/proc/self/oom_score_adj", format!("{}", cfg.oom_score_adj));

    if let Some(ref profile) = cfg.apparmor_profile {
        let _ = fs::write("/proc/self/attr/apparmor/exec", format!("exec {}", profile));
    }

    if let Some(ref label) = cfg.selinux_label {
        let _ = fs::write("/proc/self/attr/exec", label.as_bytes());
    }

    if let Some(mask) = cfg.umask {
        do_umask(mask);
    }

    let devices = cfg.devices.as_slice();
    let mount_label = cfg.mount_label.as_deref();
    let devices = if devices.is_empty() {
        None
    } else {
        Some(devices)
    };
    match rootfs_mode {
        RootfsMode::Rooted => setup_rootfs(
            &cfg.root,
            cfg.mounts.as_deref(),
            cfg.masked_paths.as_deref(),
            cfg.readonly_paths.as_deref(),
            devices,
            mount_label,
        )?,
        RootfsMode::Rootless => setup_rootfs_rootless(
            &cfg.root,
            cfg.mounts.as_deref(),
            cfg.masked_paths.as_deref(),
            cfg.readonly_paths.as_deref(),
            devices,
            mount_label,
        )
        .map_err(|error| io::Error::new(error.kind(), format!("setup rootfs failed: {error}")))?,
    }

    set_rootfs_propagation(cfg.rootfs_propagation.as_deref())?;

    apply_sysctl(cfg.sysctl.as_ref())?;

    if cfg.terminal {
        let master_fd = setup_pty_stdio()?;
        if let Some(socket_fd) = terminal_socket_fd {
            send_fd(socket_fd, master_fd)?;
            unsafe { libc::close(master_fd) };
        }
    }

    apply_security_hardening(cfg.no_new_privs)?;
    set_capabilities(
        cfg.cap_effective.as_deref(),
        cfg.cap_permitted.as_deref(),
        cfg.cap_inheritable.as_deref(),
        cfg.cap_bounding.as_deref(),
        cfg.cap_ambient.as_deref(),
    )?;

    if !cfg.additional_gids.is_empty() {
        set_supplementary_gids(&cfg.additional_gids);
    }

    do_setgid(cfg.gid)?;
    do_setuid(cfg.uid)?;

    let _listener_fd =
        apply_seccomp_from_spec(cfg.seccomp.as_ref(), &cfg.bundle_path).map_err(|e| {
            io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "seccomp filter failed to apply: {}. Container startup aborted for security.",
                    e
                ),
            )
        })?;

    if let Some(ref rdt) = cfg.intel_rdt {
        let _ = setup_intel_rdt(rdt);
    }

    Ok(())
}

// ===========================================================================
// Individual setup helpers
// ===========================================================================

/// Apply real-time scheduling policy via sched_setattr syscall.
fn apply_scheduler(sched: &crate::spec::OciScheduler) -> io::Result<()> {
    // Use sched_setattr syscall (x86_64=314, aarch64=274)
    // sched_attr struct layout:
    //   size: u32
    //   sched_policy: u32
    //   sched_flags: u64
    //   sched_nice: s32
    //   sched_priority: u32
    //   sched_runtime: u64
    //   sched_deadline: u64
    //   sched_period: u64

    const SCHED_OTHER: u32 = 0;
    const SCHED_FIFO: u32 = 1;
    const SCHED_RR: u32 = 2;
    const SCHED_BATCH: u32 = 3;
    const SCHED_IDLE: u32 = 5;
    const SCHED_DEADLINE: u32 = 6;

    let policy = match sched.policy.as_str() {
        "SCHED_OTHER" => SCHED_OTHER,
        "SCHED_FIFO" => SCHED_FIFO,
        "SCHED_RR" => SCHED_RR,
        "SCHED_BATCH" => SCHED_BATCH,
        "SCHED_IDLE" => SCHED_IDLE,
        "SCHED_DEADLINE" => SCHED_DEADLINE,
        _ => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown scheduler policy: {}", sched.policy),
            ))
        }
    };

    let nice = sched.nice.unwrap_or(0);
    let priority = sched.priority.unwrap_or(0) as u32;
    let runtime = sched
        .deadline
        .as_ref()
        .and_then(|d| d.runtime_ns)
        .unwrap_or(0);
    let deadline = sched
        .deadline
        .as_ref()
        .and_then(|d| d.deadline_ns)
        .unwrap_or(0);
    let period = sched
        .deadline
        .as_ref()
        .and_then(|d| d.period_ns)
        .unwrap_or(0);

    // Build sched_attr struct (48 bytes)
    let mut data = [0u8; 48];
    // size (u32)
    data[0..4].copy_from_slice(&48u32.to_le_bytes());
    // sched_policy (u32)
    data[4..8].copy_from_slice(&policy.to_le_bytes());
    // sched_flags (u64)
    data[8..16].copy_from_slice(&0u64.to_le_bytes());
    // sched_nice (s32)
    data[16..20].copy_from_slice(&nice.to_le_bytes());
    // sched_priority (u32)
    data[20..24].copy_from_slice(&priority.to_le_bytes());
    // sched_runtime (u64)
    data[24..32].copy_from_slice(&runtime.to_le_bytes());
    // sched_deadline (u64)
    data[32..40].copy_from_slice(&deadline.to_le_bytes());
    // sched_period (u64)
    data[40..48].copy_from_slice(&period.to_le_bytes());

    #[cfg(target_arch = "x86_64")]
    let ret = unsafe {
        libc::syscall(
            314,
            0i32, /* self */
            &data as *const _ as *const u8,
            0u64, /* flags */
        ) as i32
    };
    #[cfg(target_arch = "aarch64")]
    let ret = unsafe {
        libc::syscall(
            274,
            0i32, /* self */
            &data as *const _ as *const u8,
            0u64, /* flags */
        ) as i32
    };

    if ret != 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// Apply Intel RDT configuration via resctrl filesystem.
fn setup_intel_rdt(rdt: &crate::spec::OciLinuxIntelRdt) -> io::Result<()> {
    // The resctrl filesystem is mounted at /sys/fs/resctrl
    let resctrl = std::path::Path::new("/sys/fs/resctrl");
    if !resctrl.exists() {
        // Try to mount it
        let _ = std::fs::create_dir_all(resctrl);
        let _ = crate::syscalls::do_mount("resctrl", "/sys/fs/resctrl", "resctrl", 0, "");
    }

    let clos_id = rdt.clos_id.as_deref().unwrap_or("edgerun");
    let clos_path = resctrl.join(clos_id);

    // Create the clos directory
    std::fs::create_dir_all(&clos_path)?;

    // OCI 1.3.0: combined schemata field overrides individual fields
    if let Some(ref schemata) = rdt.schemata {
        std::fs::write(clos_path.join("schemata"), schemata)?;
    } else {
        // Write L3 cache schema
        if let Some(ref l3) = rdt.l3_cache_schema {
            std::fs::write(clos_path.join("schemata"), l3)?;
        }

        // Write memory bandwidth schema
        if let Some(ref mb) = rdt.mem_bw_schema {
            let mut existing = std::fs::read_to_string(clos_path.join("schemata"))?;
            if !existing.ends_with('\n') {
                existing.push('\n');
            }
            existing.push_str(mb);
            std::fs::write(clos_path.join("schemata"), &existing)?;
        }
    }

    // OCI 1.3.0: enable CMT/MBM monitoring
    if rdt.enable_monitoring == Some(true) {
        // Write "1" to the monitor directory to enable monitoring
        let monitor_path = clos_path.join("monitors");
        let _ = std::fs::create_dir_all(&monitor_path);
        // The kernel enables monitoring automatically when the clos is used.
        // We signal intent by creating the monitors directory.
    }

    // Move current process to this clos
    std::fs::write(clos_path.join("tasks"), format!("{}", std::process::id()))?;

    Ok(())
}

pub(crate) fn join_explicit_namespaces(ns_paths: &str) -> io::Result<()> {
    join_explicit_namespaces_where(ns_paths, |_| true)
}

fn write_uid_map(content: &str) -> io::Result<()> {
    // Only write uid_map if we're in a user namespace.
    // Writing to /proc/self/uid_map outside a user namespace fails with EPERM.
    if let Ok(ns) = fs::read_link("/proc/self/ns/user") {
        if let Ok(init_ns) = fs::read_link("/proc/1/ns/user") {
            if ns != init_ns {
                fs::write("/proc/self/uid_map", content)?;
                return Ok(());
            }
        }
    }
    // Not in a user namespace — skip uid_map writing (we keep current uid)
    Ok(())
}

fn write_setgroups_deny() -> io::Result<()> {
    // Must be written BEFORE gid_map in a user namespace.
    // The kernel requires this to prevent privilege escalation via group mapping.
    // In rootless re-exec mode, the parent already wrote setgroups deny to the
    // child's /proc/<pid>/setgroups before the re-exec, so this is a no-op.
    if let Ok(ns) = fs::read_link("/proc/self/ns/user") {
        if let Ok(init_ns) = fs::read_link("/proc/1/ns/user") {
            if ns != init_ns {
                // Check if setgroups is already "deny" (rootless re-exec case)
                if let Ok(current) = fs::read_to_string("/proc/self/setgroups") {
                    if current.trim() == "deny" {
                        return Ok(()); // Already denied by parent
                    }
                }
                let _ = fs::write("/proc/self/setgroups", "deny");
            }
        }
    }
    Ok(())
}

fn write_gid_map(content: &str) -> io::Result<()> {
    // Same as uid_map — only write if in a user namespace
    if let Ok(ns) = fs::read_link("/proc/self/ns/user") {
        if let Ok(init_ns) = fs::read_link("/proc/1/ns/user") {
            if ns != init_ns {
                // setgroups MUST be denied before writing gid_map
                write_setgroups_deny()?;
                fs::write("/proc/self/gid_map", content)?;
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Set the NIS domain name via setdomainname(2) syscall.
fn do_set_domainname(name: &str) -> io::Result<()> {
    let n =
        std::ffi::CString::new(name).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let ret = unsafe { libc::setdomainname(n.as_ptr(), n.as_bytes().len()) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Apply I/O priority via ioprio_set(2) syscall (OCI 1.1.0).
///
/// Linux ioprio_set interface:
/// - class 0: none (use existing priority)
/// - class 1: realtime (highest priority)
/// - class 2: best-effort (default, priority 0-7)
/// - class 3: idle (lowest priority, runs only when nobody else needs disk)
///
/// The encoded value is: (class << 13) | priority
fn apply_io_priority(ioprio: &crate::spec::OciIoPriority) -> io::Result<()> {
    // ioprio_set(which, who, ioprio)
    // which=1 = PRIO_PROCESS (current process), who=0 = self
    let priority = ioprio.priority.unwrap_or(4);
    let encoded = (ioprio.class << 13) | (priority & 7);

    #[cfg(target_arch = "x86_64")]
    let ret = unsafe {
        libc::syscall(251, 1i32 /* PRIO_PROCESS */, 0i32, encoded) as i32
    };
    #[cfg(target_arch = "aarch64")]
    let ret = unsafe {
        libc::syscall(31, 1i32 /* PRIO_PROCESS */, 0i32, encoded) as i32
    };

    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

// ===========================================================================
// Rootless setup — skips user namespace unshare (already created by parent)
// ===========================================================================

/// Run container setup in rootless mode (user namespace already created by parent).
///
/// This is called AFTER the parent has written uid/gid maps for the child.
/// It unshares remaining namespaces and does all the rootfs/caps/seccomp setup,
/// but skips uid/gid map writing (the parent already did that).
pub fn setup_container_child_rootless(
    cfg: &ContainerConfig,
    terminal_socket_fd: Option<i32>,
) -> io::Result<()> {
    use crate::syscalls::ns;

    // 1. Unshare remaining namespaces (exclude user and mount — clone already created these)
    let remaining_flags = cfg.ns_flags & !(ns::NEWUSER | ns::NEWNS);
    if remaining_flags != 0 {
        do_unshare(remaining_flags).map_err(|error| {
            io::Error::new(
                error.kind(),
                format!("unshare remaining namespaces 0x{remaining_flags:x} failed: {error}"),
            )
        })?;
    }
    if remaining_flags & ns::NEWPID != 0 {
        fork_into_pid_namespace()?;
    }

    // 2. Join explicit namespace paths (skip user namespace — already joined via parent)
    join_explicit_namespaces_non_user(&cfg.ns_paths).map_err(|error| {
        io::Error::new(error.kind(), format!("join namespaces failed: {error}"))
    })?;

    // Skip uid/gid map writing — parent already wrote these via /proc/<pid>/
    setup_container_child_common(cfg, terminal_socket_fd, RootfsMode::Rootless)
}

fn fork_into_pid_namespace() -> io::Result<()> {
    let child_pid = unsafe { libc::fork() };
    if child_pid < 0 {
        return Err(io::Error::last_os_error());
    }

    if child_pid > 0 {
        let mut status = 0i32;
        let waited = unsafe { libc::waitpid(child_pid, &mut status as *mut i32, 0) };
        if waited < 0 {
            unsafe { libc::_exit(1) };
        }
        let code = if libc::WIFEXITED(status) {
            libc::WEXITSTATUS(status)
        } else {
            128
        };
        unsafe { libc::_exit(code) };
    }

    std::env::set_var("_ERT_PIDNS_READY", "1");
    Ok(())
}

/// Join explicit namespace paths, skipping user namespace.
///
/// In rootless mode, the user namespace was already created by the parent,
/// so we skip joining it (and can't join it anyway — setns on user ns is restricted).
fn join_explicit_namespaces_non_user(ns_paths: &str) -> io::Result<()> {
    join_explicit_namespaces_where(ns_paths, |ns_type| ns_type != "user")
}

fn join_explicit_namespaces_where(
    ns_paths: &str,
    mut should_join: impl FnMut(&str) -> bool,
) -> io::Result<()> {
    if ns_paths.is_empty() {
        return Ok(());
    }
    for entry in ns_paths.split('\n') {
        if let Some((ns_type, path)) = entry.split_once(':') {
            if !should_join(ns_type) {
                continue;
            }
            if let Some(flag) = ns_type_to_flag(ns_type) {
                let fd = fs::File::open(path).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("cannot open namespace {}: {}", path, e),
                    )
                })?;
                do_setns(fd.as_raw_fd(), flag)?;
            }
        }
    }
    Ok(())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(all(test, not(target_os = "none")))]
#[path = "../tests/unit_src/src/process_tests.rs"]
mod tests;
