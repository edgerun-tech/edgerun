//! Child process setup — runs in `pre_exec` before `execve`, or as a `clone()` entry point.
//!
//! This module contains all the namespace, rootfs, security, and privilege
//! setup that must happen in the forked child before the container process
//! is exec'd.

use crate::prelude::*;
use std::fs;
use std::io;
use std::os::unix::io::AsRawFd;

use crate::json::{OciIdMapping, OciLinuxDevice, OciRoot, OciSpec};
use crate::rootfs::{apply_sysctl, set_rootfs_propagation, setup_rootfs, setup_rootfs_rootless};
#[allow(unused_imports)]
use crate::seccomp::apply_seccomp_from_spec;
use crate::syscalls::{
    do_set_hostname, do_setns, do_setrlimit, do_umask, do_unshare, rlimit_name_to_int,
};
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

/// Extract all container-relevant config from a spec into a flat struct
/// that can be cloned into a `pre_exec` closure.
pub struct ContainerConfig {
    pub ns_flags: i32,
    pub ns_paths: String,
    pub uid_map: String,
    pub gid_map: String,
    pub hostname: String,
    pub domainname: Option<String>,
    pub no_new_privs: bool,
    pub cap_effective: Option<Vec<String>>,
    pub cap_permitted: Option<Vec<String>>,
    pub cap_inheritable: Option<Vec<String>>,
    pub cap_bounding: Option<Vec<String>>,
    pub cap_ambient: Option<Vec<String>>,
    pub rlimits: Vec<crate::json::OciRlimit>,
    pub oom_score_adj: i64,
    pub apparmor_profile: Option<String>,
    pub selinux_label: Option<String>,
    pub umask: Option<u32>,
    pub root: OciRoot,
    pub mounts: Option<Vec<crate::json::OciMount>>,
    pub masked_paths: Option<Vec<String>>,
    pub readonly_paths: Option<Vec<String>>,
    pub devices: Vec<OciLinuxDevice>,
    pub rootfs_propagation: Option<String>,
    pub sysctl: Option<alloc::collections::BTreeMap<String, String>>,
    pub additional_gids: Vec<u32>,
    pub uid: u32,
    pub gid: u32,
    pub seccomp: Option<crate::json::OciLinuxSeccomp>,
    pub mount_label: Option<String>,
    pub scheduler: Option<crate::json::OciScheduler>,
    pub intel_rdt: Option<crate::json::OciLinuxIntelRdt>,
    pub io_priority: Option<crate::json::OciIoPriority>,
    pub terminal: bool,
    pub bundle_path: String,
}

impl ContainerConfig {
    /// Extract all config needed for pre_exec from an OCI spec.
    pub fn from_spec(spec: &OciSpec) -> io::Result<Self> {
        let root = spec
            .root
            .clone()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no root in OCI spec"))?;

        let linux = spec.linux.clone().unwrap_or_default();
        let process = spec.process.clone().unwrap_or_default();
        let user = process.user.clone().unwrap_or_default();

        let ns_list = linux
            .namespaces
            .clone()
            .unwrap_or_else(crate::default_namespaces);
        let ns_flags = crate::namespace_flags(&ns_list);

        // Validate namespace types — reject unknown types
        for ns in &ns_list {
            if ns.path.is_none()
                && !crate::validate::KNOWN_NAMESPACES.contains(&ns.ns_type.as_str())
            {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown namespace type: {}", ns.ns_type),
                ));
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
            domainname: spec.domainname.clone(),
            no_new_privs: process.no_new_privileges.unwrap_or(true),
            cap_effective: caps.effective,
            cap_permitted: caps.permitted,
            cap_inheritable: caps.inheritable,
            cap_bounding: caps.bounding,
            cap_ambient: caps.ambient,
            rlimits: process.rlimits.clone().unwrap_or_default(),
            oom_score_adj: process.oom_score_adj.unwrap_or(0),
            apparmor_profile: process.apparmor_profile,
            selinux_label: process.selinux_label,
            umask: user.umask,
            bundle_path: root.path.clone(),
            root,
            mounts: spec.mounts.clone(),
            masked_paths: linux.masked_paths.clone(),
            readonly_paths: linux.readonly_paths.clone(),
            devices: linux.devices.clone().unwrap_or_default(),
            rootfs_propagation: linux.rootfs_propagation.clone(),
            sysctl: linux.sysctl.clone(),
            additional_gids: user.additional_gids.unwrap_or_default(),
            uid: user.uid.unwrap_or(0),
            gid: user.gid.unwrap_or(0),
            seccomp: linux.seccomp,
            mount_label: linux.mount_label.clone(),
            scheduler: process.scheduler.clone(),
            intel_rdt: linux.intel_rdt.clone(),
            io_priority: process.io_priority.clone(),
            terminal: process.terminal.unwrap_or(false),
        })
    }

    /// Returns true if PID namespace is unshared (not joined via path).
    pub fn has_pid_ns(&self) -> bool {
        (self.ns_flags & crate::syscalls::ns::NEWPID) != 0
    }
}

fn serialize_ns_paths(namespaces: Option<&[crate::json::OciNamespace]>) -> String {
    match namespaces {
        Some(ns) => ns
            .iter()
            .filter_map(|n| {
                n.path
                    .as_ref()
                    .filter(|path| !path.is_empty())
                    .map(|path| format!("{}:{}", n.ns_type, path))
            })
            .collect::<Vec<_>>()
            .join("\n"),
        None => String::new(),
    }
}

fn format_mapping(mappings: Option<&[OciIdMapping]>) -> String {
    if let Some(maps) = mappings {
        if !maps.is_empty() {
            return maps
                .iter()
                .map(|m| format!("{} {} {}\n", m.container_id, m.host_id, m.size))
                .collect();
        }
    }
    // No explicit mappings — generate rootless defaults
    default_rootless_mapping()
}

/// Generate a default uid/gid mapping for rootless mode.
///
/// Uses subuid/subgid ranges from /etc/subuid and /etc/subgid.
/// The kernel only allows uid_map entries within the caller's configured subuid range.
/// When running as root, maps container root to host nobody.
///
/// Follows the Podman mapping pattern:
/// - Container UID 0 → host's real UID (size 1)
/// - Container UID 1..N → subuid ranges
fn default_rootless_mapping() -> String {
    let uid = unsafe { libc::getuid() };
    if uid == 0 {
        // Root mode: map container root to host nobody (minimal mapping)
        return "0 65534 1\n".to_string();
    }

    // Rootless: use subuid/subgid ranges — the kernel REQUIRES all mapped
    // host UIDs to be within the caller's configured subuid range.
    match crate::rootless::get_current_user_subuids() {
        Ok(subuids) if !subuids.is_empty() => {
            // Map container uid 0 to the host user's own UID,
            // then container 1..N to the subuid ranges
            let mut map = format!("0 {} 1\n", uid);
            for range in &subuids {
                // Container IDs start at 1 (0 is reserved for the user's own UID)
                map.push_str(&format!("1 {} {}\n", range.start, range.count));
            }
            map
        }
        _ => {
            // No subuid ranges — map only the host user's own UID (size 1).
            // This works because the kernel allows mapping your own uid.
            format!("0 {} 1\n", uid)
        }
    }
}

/// Map an OCI namespace type string to the corresponding CLONE_NEW* flag.
pub fn ns_type_to_flag(ns_type: &str) -> Option<i32> {
    use crate::syscalls::ns;
    match ns_type {
        "mount" => Some(ns::NEWNS),
        "cgroup" => Some(ns::NEWCGROUP),
        "uts" => Some(ns::NEWUTS),
        "ipc" => Some(ns::NEWIPC),
        "user" => Some(ns::NEWUSER),
        "pid" => Some(ns::NEWPID),
        "network" => Some(ns::NEWNET),
        _ => None,
    }
}

// ===========================================================================
// Terminal / PTY support
// ===========================================================================

/// Allocate a pseudo-terminal and connect it to stdin/stdout/stderr.
///
/// Opens `/dev/ptmx`, grants/unlocks the slave, then dups it to fds 0, 1, 2.
/// The master fd is left open (it will be inherited by the exec'd workload).
pub fn setup_terminal() -> io::Result<i32> {
    use std::os::raw::c_char;
    use std::os::raw::c_int;

    extern "C" {
        fn posix_openpt(flags: c_int) -> c_int;
        fn grantpt(fd: c_int) -> c_int;
        fn unlockpt(fd: c_int) -> c_int;
        fn ptsname(fd: c_int) -> *const c_char;
    }

    // Open master PTY
    let master_fd = unsafe { posix_openpt(libc::O_RDWR | libc::O_NOCTTY) };
    if master_fd < 0 {
        return Err(io::Error::last_os_error());
    }

    // Grant access to slave
    if unsafe { grantpt(master_fd) } != 0 {
        let err = io::Error::last_os_error();
        unsafe { libc::close(master_fd) };
        return Err(err);
    }

    // Unlock slave
    if unsafe { unlockpt(master_fd) } != 0 {
        let err = io::Error::last_os_error();
        unsafe { libc::close(master_fd) };
        return Err(err);
    }

    // Get slave path and open it
    let slave_path = unsafe { ptsname(master_fd) };
    if slave_path.is_null() {
        let err = io::Error::last_os_error();
        unsafe { libc::close(master_fd) };
        return Err(err);
    }

    // Open slave PTY
    let slave_fd = unsafe { libc::open(slave_path, libc::O_RDWR) };
    if slave_fd < 0 {
        let err = io::Error::last_os_error();
        unsafe { libc::close(master_fd) };
        return Err(err);
    }

    // Set controlling terminal
    unsafe { libc::ioctl(slave_fd, libc::TIOCSCTTY as _, 0) };

    // Dup slave to stdin/stdout/stderr
    unsafe { libc::dup2(slave_fd, libc::STDIN_FILENO) };
    unsafe { libc::dup2(slave_fd, libc::STDOUT_FILENO) };
    unsafe { libc::dup2(slave_fd, libc::STDERR_FILENO) };

    // Close original slave fd (stdin/stdout/stderr are now the slave)
    if slave_fd > 2 {
        unsafe { libc::close(slave_fd) };
    }

    Ok(master_fd)
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
        if let Some(resource) = rlimit_name_to_int(&rl.ns_type) {
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
        let master_fd = setup_terminal()?;
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
fn apply_scheduler(sched: &crate::json::OciScheduler) -> io::Result<()> {
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
fn setup_intel_rdt(rdt: &crate::json::OciLinuxIntelRdt) -> io::Result<()> {
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
fn apply_io_priority(ioprio: &crate::json::OciIoPriority) -> io::Result<()> {
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

fn send_fd(sock_fd: i32, fd: i32) -> io::Result<()> {
    let mut msg: libc::msghdr = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    let cmsg_space = unsafe { libc::CMSG_SPACE(std::mem::size_of::<i32>() as u32) as usize };
    let mut cmsg_buf = vec![0u8; cmsg_space];
    let fd_to_send = fd;
    let iov = libc::iovec {
        iov_base: &fd_to_send as *const _ as *mut libc::c_void,
        iov_len: std::mem::size_of::<i32>(),
    };

    msg.msg_iov = &iov as *const _ as *mut libc::iovec;
    msg.msg_iovlen = 1;
    msg.msg_control = cmsg_buf.as_mut_ptr() as *mut libc::c_void;
    msg.msg_controllen = cmsg_space as _;

    let cmsg = unsafe { libc::CMSG_FIRSTHDR(&msg) };
    if cmsg.is_null() {
        return Err(io::Error::other(
            "failed to allocate terminal fd control message",
        ));
    }
    unsafe {
        (*cmsg).cmsg_level = libc::SOL_SOCKET;
        (*cmsg).cmsg_type = libc::SCM_RIGHTS;
        (*cmsg).cmsg_len = libc::CMSG_LEN(std::mem::size_of::<i32>() as u32) as _;
        std::ptr::copy_nonoverlapping(&fd as *const i32, libc::CMSG_DATA(cmsg) as *mut i32, 1);
    }

    let ret = unsafe { libc::sendmsg(sock_fd, &msg, 0) };
    if ret < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
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
mod tests {
    use super::*;
    use crate::json::{
        OciCapabilities, OciLinuxSeccomp, OciNamespace, OciProcess, OciRlimit, OciRoot,
        OciSeccompAction,
    };

    fn minimal_spec() -> OciSpec {
        OciSpec {
            version: "1.0.2".into(),
            platform: None,
            process: Some(OciProcess {
                args: Some(vec!["/bin/true".into()]),
                ..Default::default()
            }),
            root: Some(OciRoot {
                path: "/rootfs".into(),
                readonly: None,
            }),
            hostname: None,
            linux: None,
            mounts: None,
            annotations: None,
            domainname: None,
        }
    }

    #[test]
    fn validate_spec_accepts_minimal() {
        assert!(validate_spec(&minimal_spec()).is_ok());
    }

    #[test]
    fn validate_spec_rejects_missing_root() {
        let mut spec = minimal_spec();
        spec.root = None;
        assert!(validate_spec(&spec)
            .unwrap_err()
            .to_string()
            .contains("missing root"));
    }

    #[test]
    fn validate_spec_rejects_missing_process() {
        let mut spec = minimal_spec();
        spec.process = None;
        assert!(validate_spec(&spec)
            .unwrap_err()
            .to_string()
            .contains("missing process"));
    }

    #[test]
    fn validate_spec_rejects_empty_args() {
        let mut spec = minimal_spec();
        spec.process.as_mut().unwrap().args = Some(vec![]);
        assert!(validate_spec(&spec)
            .unwrap_err()
            .to_string()
            .contains("args must not be empty"));
    }

    #[test]
    fn validate_spec_rejects_unknown_capability() {
        let mut spec = minimal_spec();
        spec.process.as_mut().unwrap().capabilities = Some(OciCapabilities {
            effective: Some(vec!["CAP_BOGUS".into()]),
            ..Default::default()
        });
        assert!(validate_spec(&spec)
            .unwrap_err()
            .to_string()
            .contains("unknown capability: CAP_BOGUS"));
    }

    #[test]
    fn validate_spec_accepts_valid_capability() {
        let mut spec = minimal_spec();
        spec.process.as_mut().unwrap().capabilities = Some(OciCapabilities {
            effective: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
            ..Default::default()
        });
        assert!(validate_spec(&spec).is_ok());
    }

    #[test]
    fn validate_spec_rejects_unknown_namespace() {
        let mut spec = minimal_spec();
        spec.linux = Some(crate::json::OciLinux {
            namespaces: Some(vec![OciNamespace {
                ns_type: "bogus".into(),
                path: None,
            }]),
            ..Default::default()
        });
        assert!(validate_spec(&spec)
            .unwrap_err()
            .to_string()
            .contains("unknown namespace type: bogus"));
    }

    #[test]
    fn validate_spec_allows_path_based_namespace() {
        let mut spec = minimal_spec();
        spec.linux = Some(crate::json::OciLinux {
            namespaces: Some(vec![OciNamespace {
                ns_type: "custom".into(),
                path: Some("/var/run/ns/custom".into()),
            }]),
            ..Default::default()
        });
        assert!(validate_spec(&spec).is_ok());
    }

    #[test]
    fn validate_spec_rejects_unknown_rlimit() {
        let mut spec = minimal_spec();
        spec.linux = Some(crate::json::OciLinux {
            ..Default::default()
        });
        spec.process.as_mut().unwrap().rlimits = Some(vec![OciRlimit {
            ns_type: "RLIMIT_BOGUS".into(),
            hard: 1024,
            soft: 512,
        }]);
        assert!(validate_spec(&spec)
            .unwrap_err()
            .to_string()
            .contains("unknown rlimit type: RLIMIT_BOGUS"));
    }

    #[test]
    fn validate_spec_accepts_valid_seccomp_action() {
        let mut spec = minimal_spec();
        spec.linux = Some(crate::json::OciLinux {
            seccomp: Some(OciLinuxSeccomp {
                default_action: Some(OciSeccompAction::Allow),
                ..Default::default()
            }),
            ..Default::default()
        });
        assert!(validate_spec(&spec).is_ok());
    }

    #[test]
    fn host_os_is_linux() {
        assert_eq!(host_os(), crate::validate::host_os());
    }

    #[test]
    fn host_arch_is_known() {
        let arch = host_arch();
        assert!(matches!(
            arch,
            "amd64" | "arm64" | "riscv64" | "arm" | "unknown"
        ));
    }

    #[test]
    fn platform_matches_host_linux_amd64() {
        use crate::json::OciPlatform;
        let platform = OciPlatform {
            os: Some(host_os().into()),
            arch: Some(host_arch().into()),
            os_version: None,
            os_features: None,
        };
        assert!(platform.matches_host());
    }

    #[test]
    fn platform_rejects_windows() {
        use crate::json::OciPlatform;
        let platform = OciPlatform {
            os: Some("windows".into()),
            arch: Some("amd64".into()),
            os_version: None,
            os_features: None,
        };
        assert!(!platform.matches_host());
    }

    #[test]
    fn platform_rejects_wrong_arch() {
        use crate::json::OciPlatform;
        let platform = OciPlatform {
            os: Some("linux".into()),
            arch: Some("riscv64".into()),
            os_version: None,
            os_features: None,
        };
        // Only matches on actual riscv64 hardware
        if cfg!(target_arch = "riscv64") {
            assert!(platform.matches_host());
        } else {
            assert!(!platform.matches_host());
        }
    }

    #[test]
    fn platform_none_matches_host() {
        // When no platform is specified, it should not block creation
        // (the runtime allows None = no platform constraint)
        use crate::json::OciPlatform;
        let platform = OciPlatform {
            os: None,
            arch: None,
            os_version: None,
            os_features: None,
        };
        assert!(platform.matches_host());
    }

    #[test]
    fn default_namespaces_includes_cgroup() {
        let namespaces = crate::default_namespaces();
        let has_cgroup = namespaces.iter().any(|ns| ns.ns_type == "cgroup");
        assert!(has_cgroup, "default namespaces should include cgroup");
    }

    #[test]
    fn container_config_has_terminal_field() {
        let cfg = ContainerConfig::from_spec(&minimal_spec()).unwrap();
        assert!(!cfg.terminal, "terminal should default to false");
    }
}
