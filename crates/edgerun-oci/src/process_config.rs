use crate::libc;
use crate::prelude::*;
use std::io;

use crate::linux_catalog::is_oci_namespace_name;
use crate::spec::{OciIdMapping, OciLinuxDevice, OciRoot, OciSpec};

/// Extracted container config cloned into child setup.
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
    pub rlimits: Vec<crate::spec::OciRlimit>,
    pub oom_score_adj: i64,
    pub apparmor_profile: Option<String>,
    pub selinux_label: Option<String>,
    pub umask: Option<u32>,
    pub root: OciRoot,
    pub mounts: Option<Vec<crate::spec::OciMount>>,
    pub masked_paths: Option<Vec<String>>,
    pub readonly_paths: Option<Vec<String>>,
    pub devices: Vec<OciLinuxDevice>,
    pub rootfs_propagation: Option<String>,
    pub sysctl: Option<alloc::collections::BTreeMap<String, String>>,
    pub additional_gids: Vec<u32>,
    pub uid: u32,
    pub gid: u32,
    pub seccomp: Option<crate::spec::OciLinuxSeccomp>,
    pub mount_label: Option<String>,
    pub scheduler: Option<crate::spec::OciScheduler>,
    pub intel_rdt: Option<crate::spec::OciLinuxIntelRdt>,
    pub io_priority: Option<crate::spec::OciIoPriority>,
    pub terminal: bool,
    pub bundle_path: String,
}

impl ContainerConfig {
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

        for ns in &ns_list {
            if ns.path.is_none() && !is_oci_namespace_name(&ns.ns_type) {
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

    pub fn has_pid_ns(&self) -> bool {
        (self.ns_flags & crate::syscalls::ns::NEWPID) != 0
    }
}

fn serialize_ns_paths(namespaces: Option<&[crate::spec::OciNamespace]>) -> String {
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
    default_rootless_mapping()
}

fn default_rootless_mapping() -> String {
    let uid = unsafe { libc::getuid() };
    if uid == 0 {
        return "0 65534 1\n".to_string();
    }

    match crate::rootless::get_current_user_subuids() {
        Ok(subuids) if !subuids.is_empty() => {
            let mut map = format!("0 {} 1\n", uid);
            for range in &subuids {
                map.push_str(&format!("1 {} {}\n", range.start, range.count));
            }
            map
        }
        _ => format!("0 {} 1\n", uid),
    }
}
