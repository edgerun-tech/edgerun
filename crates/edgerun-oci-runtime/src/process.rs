//! Child process setup — runs in `pre_exec` before `execve`.
//!
//! This module contains all the namespace, rootfs, security, and privilege
//! setup that must happen in the forked child before the container process
//! is exec'd.

use std::fs;
use std::io;
use std::os::unix::io::AsRawFd;

use crate::json::{OciIdMapping, OciLinuxDevice, OciRoot, OciSpec};
use crate::default_namespaces;
use crate::rootfs::{setup_rootfs, apply_sysctl, set_rootfs_propagation};
use crate::seccomp::apply_seccomp_from_spec;
use crate::syscalls::{
    do_set_hostname, do_unshare, do_setns, do_setrlimit, do_umask, rlimit_name_to_int,
};
use crate::userns::{
    apply_security_hardening, set_capabilities, do_setgid, do_setuid,
    set_supplementary_gids,
};

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

        let ns_list = linux.namespaces.clone().unwrap_or_else(default_namespaces);
        let ns_flags = {
            use crate::syscalls::ns;
            let mut f: i32 = 0;
            for ns in &ns_list {
                if ns.path.is_some() { continue; }
                f |= match ns.ns_type.as_str() {
                    "mount"   => ns::NEWNS,
                    "cgroup"  => ns::NEWCGROUP,
                    "uts"     => ns::NEWUTS,
                    "ipc"     => ns::NEWIPC,
                    "user"    => ns::NEWUSER,
                    "pid"     => ns::NEWPID,
                    "network" => ns::NEWNET,
                    _ => 0, // Unknown ns type — validation below
                };
            }
            f
        };

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
        (self.ns_flags & 0x20000000) != 0 // CLONE_NEWPID
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
    match ns_type {
        "mount"   => Some(0x00020000),
        "cgroup"  => Some(0x02000000),
        "uts"     => Some(0x04000000),
        "ipc"     => Some(0x08000000),
        "user"    => Some(0x10000000),
        "pid"     => Some(0x20000000),
        "network" => Some(0x40000000),
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
        true,  // strict masked paths
        true,  // strict readonly paths
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
    fs::write("/proc/self/uid_map", content)?;
    let _ = fs::write("/proc/self/setgroups", "deny");
    Ok(())
}

fn write_gid_map(content: &str) -> io::Result<()> {
    fs::write("/proc/self/gid_map", content)?;
    Ok(())
}
