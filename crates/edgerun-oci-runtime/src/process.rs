//! Child process setup — runs in `pre_exec` before `execve`, or as a `clone()` entry point.
//!
//! This module contains all the namespace, rootfs, security, and privilege
//! setup that must happen in the forked child before the container process
//! is exec'd.

use std::fs;
use std::io;
use std::os::unix::io::AsRawFd;

use crate::json::{OciIdMapping, OciLinuxDevice, OciRoot, OciSpec};
use crate::rootfs::{setup_rootfs, apply_sysctl, set_rootfs_propagation};
#[allow(unused_imports)]
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

/// Get the host OS string (e.g., "linux").
pub fn host_os() -> &'static str {
    if cfg!(target_os = "linux") { "linux" }
    else if cfg!(target_os = "windows") { "windows" }
    else if cfg!(target_os = "solaris") { "solaris" }
    else { "unknown" }
}

/// Get the host architecture string (e.g., "amd64").
pub fn host_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") { "amd64" }
    else if cfg!(target_arch = "aarch64") { "arm64" }
    else if cfg!(target_arch = "riscv64") { "riscv64" }
    else if cfg!(target_arch = "arm") { "arm" }
    else { "unknown" }
}

/// Known Linux capability names (all 5 sets).
/// Based on Linux capability numbers 0-40.
const KNOWN_CAPABILITIES: &[&str] = &[
    "CAP_CHOWN", "CAP_DAC_OVERRIDE", "CAP_DAC_READ_SEARCH", "CAP_FOWNER",
    "CAP_FSETID", "CAP_KILL", "CAP_SETGID", "CAP_SETUID",
    "CAP_SETPCAP", "CAP_LINUX_IMMUTABLE", "CAP_NET_BIND_SERVICE", "CAP_NET_BROADCAST",
    "CAP_NET_ADMIN", "CAP_NET_RAW", "CAP_IPC_LOCK", "CAP_IPC_OWNER",
    "CAP_SYS_MODULE", "CAP_SYS_RAWIO", "CAP_SYS_CHROOT", "CAP_SYS_PTRACE",
    "CAP_SYS_PACCT", "CAP_SYS_ADMIN", "CAP_SYS_BOOT", "CAP_SYS_NICE",
    "CAP_SYS_RESOURCE", "CAP_SYS_TIME", "CAP_SYS_TTY_CONFIG", "CAP_MKNOD",
    "CAP_LEASE", "CAP_AUDIT_WRITE", "CAP_AUDIT_CONTROL", "CAP_SETFCAP",
    "CAP_MAC_OVERRIDE", "CAP_MAC_ADMIN", "CAP_SYSLOG", "CAP_WAKE_ALARM",
    "CAP_BLOCK_SUSPEND", "CAP_AUDIT_READ", "CAP_PERFMON", "CAP_BPF",
    "CAP_CHECKPOINT_RESTORE",
];

/// Known namespace types for Linux.
const KNOWN_NAMESPACES: &[&str] = &[
    "mount", "pid", "network", "ipc", "uts", "user", "cgroup",
];

/// Validate an OCI spec for known-invalid values before container creation.
///
/// Returns an error for:
/// - Unknown capability names in any capability set
/// - Unknown namespace types (for path-less namespaces)
/// - Missing required fields (root, process)
/// - Negative resource limits (except where allowed)
/// - Empty executable path
pub fn validate_spec(spec: &OciSpec) -> io::Result<()> {
    // Root is required
    if spec.root.is_none() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "spec missing root"));
    }

    // Process is required
    if spec.process.is_none() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "spec missing process"));
    }

    let proc = spec.process.as_ref().unwrap();

    // Process args must not be empty
    if let Some(ref args) = proc.args {
        if args.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "process args must not be empty"));
        }
        // First arg (executable) should be absolute or resolvable
        if args[0].is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "process executable path must not be empty"));
        }
    }

    // Validate capability names
    if let Some(ref caps) = proc.capabilities {
        let cap_sets = [
            ("bounding", caps.bounding.as_ref()),
            ("effective", caps.effective.as_ref()),
            ("permitted", caps.permitted.as_ref()),
            ("inheritable", caps.inheritable.as_ref()),
            ("ambient", caps.ambient.as_ref()),
        ];
        for (name, cap_set) in &cap_sets {
            if let Some(caps_list) = cap_set {
                for cap in caps_list.iter() {
                    if !KNOWN_CAPABILITIES.contains(&cap.as_str()) {
                        return Err(io::Error::new(io::ErrorKind::InvalidInput,
                            format!("unknown capability: {}", cap)));
                    }
                }
            }
        }
    }

    // Validate namespace types
    if let Some(ref linux) = spec.linux {
        if let Some(ref namespaces) = linux.namespaces {
            for ns in namespaces {
                if ns.path.is_none() && !KNOWN_NAMESPACES.contains(&ns.ns_type.as_str()) {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput,
                        format!("unknown namespace type: {}", ns.ns_type)));
                }
            }
        }

        // Validate rlimit types
        if let Some(ref proc) = spec.process {
            if let Some(ref rlimits) = proc.rlimits {
                for rl in rlimits.iter() {
                    let known = [
                        "RLIMIT_CPU", "RLIMIT_FSIZE", "RLIMIT_DATA", "RLIMIT_STACK",
                        "RLIMIT_CORE", "RLIMIT_RSS", "RLIMIT_NPROC", "RLIMIT_NOFILE",
                        "RLIMIT_MEMLOCK", "RLIMIT_AS", "RLIMIT_LOCKS",
                        "RLIMIT_SIGPENDING", "RLIMIT_MSGQUEUE", "RLIMIT_NICE",
                        "RLIMIT_RTPRIO", "RLIMIT_RTTIME",
                    ];
                    if !known.contains(&rl.ns_type.as_str()) {
                        return Err(io::Error::new(io::ErrorKind::InvalidInput,
                            format!("unknown rlimit type: {}", rl.ns_type)));
                    }
                }
            }
        }

        // Validate seccomp action
        if let Some(ref seccomp) = linux.seccomp {
            if let Some(ref default_action) = seccomp.default_action {
                let action_str: String = default_action.clone().into();
                let valid_actions = ["SCMP_ACT_ALLOW", "SCMP_ACT_ERRNO", "SCMP_ACT_KILL",
                    "SCMP_ACT_KILL_PROCESS", "SCMP_ACT_KILL_THREAD", "SCMP_ACT_TRAP",
                    "SCMP_ACT_LOG", "SCMP_ACT_TRACE", "SCMP_ACT_NOTIFY"];
                if !valid_actions.contains(&action_str.as_str()) {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput,
                        format!("unknown seccomp action: {}", action_str)));
                }
            }
            // Validate seccomp architectures
            if let Some(ref archs) = seccomp.architectures {
                let valid_archs = ["SCMP_ARCH_X86", "SCMP_ARCH_X86_64", "SCMP_ARCH_X32",
                    "SCMP_ARCH_ARM", "SCMP_ARCH_AARCH64", "SCMP_ARCH_MIPS",
                    "SCMP_ARCH_MIPS64", "SCMP_ARCH_MIPS64N32", "SCMP_ARCH_MIPSEL",
                    "SCMP_ARCH_MIPSEL64", "SCMP_ARCH_MIPSEL64N32", "SCMP_ARCH_PPC",
                    "SCMP_ARCH_PPC64", "SCMP_ARCH_PPC64LE", "SCMP_ARCH_S390",
                    "SCMP_ARCH_S390X", "SCMP_ARCH_RISCV64"];
                for arch in archs.iter() {
                    if !valid_archs.contains(&arch.as_str()) {
                        return Err(io::Error::new(io::ErrorKind::InvalidInput,
                            format!("unknown seccomp architecture: {}", arch)));
                    }
                }
            }
        }
    }

    Ok(())
}

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
    pub selinux_label: Option<String>,
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
            selinux_label: process.selinux_label,
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

/// Map an OCI namespace type string to the corresponding CLONE_NEW* flag.
pub fn ns_type_to_flag(ns_type: &str) -> Option<i32> {
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

    // 5. Capabilities (MUST come before no_new_privs — capset can only reduce caps after nnp)
    set_capabilities(
        cfg.cap_effective.as_deref(),
        cfg.cap_permitted.as_deref(),
        cfg.cap_inheritable.as_deref(),
        cfg.cap_bounding.as_deref(),
        cfg.cap_ambient.as_deref(),
    )?;

    // 6. Security: no_new_privs + non-dumpable (after caps, before seccomp)
    apply_security_hardening(cfg.no_new_privs)?;

    // 7. Seccomp (requires no_new_privs set; works without CAP_SYS_ADMIN)
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

    // 11. SELinux label
    if let Some(ref label) = cfg.selinux_label {
        let _ = fs::write("/proc/self/attr/exec", label.as_bytes());
    }

    // 12. Umask
    if let Some(mask) = cfg.umask {
        do_umask(mask);
    }

    // 13. Rootfs
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

    // 14. Rootfs propagation
    set_rootfs_propagation(cfg.rootfs_propagation.as_deref())?;

    // 15. Sysctl
    apply_sysctl(cfg.sysctl.as_ref())?;

    // 16. Supplementary groups
    if !cfg.additional_gids.is_empty() {
        set_supplementary_gids(&cfg.additional_gids);
    }

    // 17. Drop GID then UID
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
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::{OciProcess, OciRoot, OciCapabilities, OciNamespace, OciRlimit, OciLinuxSeccomp, OciSeccompAction};

    fn minimal_spec() -> OciSpec {
        OciSpec {
            version: "1.0.2".into(),
            platform: None,
            process: Some(OciProcess {
                args: Some(vec!["/bin/true".into()]),
                ..Default::default()
            }),
            root: Some(OciRoot { path: "/rootfs".into(), readonly: None }),
            hostname: None,
            linux: None,
            mounts: None,
            annotations: None,
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
        assert!(validate_spec(&spec).unwrap_err().to_string().contains("missing root"));
    }

    #[test]
    fn validate_spec_rejects_missing_process() {
        let mut spec = minimal_spec();
        spec.process = None;
        assert!(validate_spec(&spec).unwrap_err().to_string().contains("missing process"));
    }

    #[test]
    fn validate_spec_rejects_empty_args() {
        let mut spec = minimal_spec();
        spec.process.as_mut().unwrap().args = Some(vec![]);
        assert!(validate_spec(&spec).unwrap_err().to_string().contains("args must not be empty"));
    }

    #[test]
    fn validate_spec_rejects_unknown_capability() {
        let mut spec = minimal_spec();
        spec.process.as_mut().unwrap().capabilities = Some(OciCapabilities {
            effective: Some(vec!["CAP_BOGUS".into()]),
            ..Default::default()
        });
        assert!(validate_spec(&spec).unwrap_err().to_string().contains("unknown capability: CAP_BOGUS"));
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
        assert!(validate_spec(&spec).unwrap_err().to_string().contains("unknown namespace type: bogus"));
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
        assert!(validate_spec(&spec).unwrap_err().to_string().contains("unknown rlimit type: RLIMIT_BOGUS"));
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
        assert_eq!(host_os(), "linux");
    }

    #[test]
    fn host_arch_is_known() {
        let arch = host_arch();
        assert!(matches!(arch, "amd64" | "arm64" | "riscv64" | "arm" | "unknown"));
    }
}
