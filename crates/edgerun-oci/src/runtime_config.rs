//! no_std runtime-facing OCI configuration.

use crate::prelude::*;
use crate::spec::{
    OciCapabilities, OciLinuxDevice, OciLinuxResources, OciLinuxSeccomp, OciMount, OciNamespace,
    OciRlimit, OciSpec,
};
use crate::validate::{
    default_process_args, default_process_env, validate_spec, OciValidationError,
};
use alloc::collections::BTreeMap;

/// Flattened OCI configuration for non-host runtimes.
///
/// This deliberately avoids Linux syscall flags, filesystem handles, and `std`
/// types. Bare-metal code can decide how to interpret mounts, namespaces, and
/// resource hints for its own boot path.
#[derive(Debug, Clone)]
pub struct BareRuntimeConfig {
    pub rootfs: String,
    pub root_readonly: bool,
    pub args: Vec<String>,
    pub env: Vec<String>,
    pub cwd: String,
    pub hostname: String,
    pub domainname: Option<String>,
    pub uid: u32,
    pub gid: u32,
    pub additional_gids: Vec<u32>,
    pub umask: Option<u32>,
    pub no_new_privileges: bool,
    pub capabilities: OciCapabilities,
    pub rlimits: Vec<OciRlimit>,
    pub oom_score_adj: i64,
    pub apparmor_profile: Option<String>,
    pub selinux_label: Option<String>,
    pub terminal: bool,
    pub namespaces: Vec<BareNamespace>,
    pub mounts: Vec<OciMount>,
    pub masked_paths: Vec<String>,
    pub readonly_paths: Vec<String>,
    pub rootfs_propagation: Option<String>,
    pub sysctl: BTreeMap<String, String>,
    pub devices: Vec<OciLinuxDevice>,
    pub resources: Option<OciLinuxResources>,
    pub seccomp: Option<OciLinuxSeccomp>,
    pub mount_label: Option<String>,
    pub annotations: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BareNamespace {
    pub kind: BareNamespaceKind,
    pub path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BareNamespaceKind {
    Mount,
    Pid,
    Network,
    Ipc,
    Uts,
    User,
    Cgroup,
    Time,
    Other(String),
}

impl BareRuntimeConfig {
    pub fn from_spec(spec: &OciSpec) -> Result<Self, OciValidationError> {
        validate_spec(spec)?;

        let root = spec
            .root
            .as_ref()
            .ok_or_else(|| OciValidationError::new("spec missing root"))?;
        let process = spec
            .process
            .as_ref()
            .ok_or_else(|| OciValidationError::new("spec missing process"))?;
        let user = process.user.clone().unwrap_or_default();
        let linux = spec.linux.clone().unwrap_or_default();

        let namespaces = linux
            .namespaces
            .clone()
            .unwrap_or_else(crate::default_namespaces)
            .into_iter()
            .map(BareNamespace::from)
            .collect();

        Ok(Self {
            rootfs: root.path.clone(),
            root_readonly: root.readonly.unwrap_or(false),
            args: process.args.clone().unwrap_or_else(default_process_args),
            env: process.env.clone().unwrap_or_else(default_process_env),
            cwd: process.cwd.clone().unwrap_or_else(|| "/".into()),
            hostname: spec.hostname.clone().unwrap_or_else(|| "edgerun".into()),
            domainname: spec.domainname.clone(),
            uid: user.uid.unwrap_or(0),
            gid: user.gid.unwrap_or(0),
            additional_gids: user.additional_gids.unwrap_or_default(),
            umask: user.umask,
            no_new_privileges: process.no_new_privileges.unwrap_or(true),
            capabilities: process.capabilities.clone().unwrap_or_default(),
            rlimits: process.rlimits.clone().unwrap_or_default(),
            oom_score_adj: process.oom_score_adj.unwrap_or(0),
            apparmor_profile: process.apparmor_profile.clone(),
            selinux_label: process.selinux_label.clone(),
            terminal: process.terminal.unwrap_or(false),
            namespaces,
            mounts: spec.mounts.clone().unwrap_or_default(),
            masked_paths: linux.masked_paths.unwrap_or_default(),
            readonly_paths: linux.readonly_paths.unwrap_or_default(),
            rootfs_propagation: linux.rootfs_propagation,
            sysctl: linux.sysctl.unwrap_or_default(),
            devices: linux.devices.unwrap_or_default(),
            resources: linux.resources,
            seccomp: linux.seccomp,
            mount_label: linux.mount_label,
            annotations: spec.annotations.clone().unwrap_or_default(),
        })
    }

    pub fn namespace_paths(&self) -> impl Iterator<Item = (&BareNamespaceKind, &str)> {
        self.namespaces.iter().filter_map(|namespace| {
            namespace
                .path
                .as_deref()
                .map(|path| (&namespace.kind, path))
        })
    }

    pub fn creates_namespace(&self, kind: BareNamespaceKind) -> bool {
        self.namespaces
            .iter()
            .any(|namespace| namespace.kind == kind && namespace.path.is_none())
    }
}

impl From<OciNamespace> for BareNamespace {
    fn from(namespace: OciNamespace) -> Self {
        Self {
            kind: BareNamespaceKind::from(namespace.ns_type),
            path: namespace.path,
        }
    }
}

impl From<String> for BareNamespaceKind {
    fn from(value: String) -> Self {
        match value.as_str() {
            "mount" => Self::Mount,
            "pid" => Self::Pid,
            "network" => Self::Network,
            "ipc" => Self::Ipc,
            "uts" => Self::Uts,
            "user" => Self::User,
            "cgroup" => Self::Cgroup,
            "time" => Self::Time,
            _ => Self::Other(value),
        }
    }
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;
    use crate::spec::{OciLinux, OciProcess, OciRoot, OciUser};

    fn minimal_spec() -> OciSpec {
        OciSpec {
            version: "1.0.2".into(),
            platform: None,
            process: Some(OciProcess {
                args: Some(vec!["/init".into()]),
                user: Some(OciUser {
                    uid: Some(1000),
                    gid: Some(1001),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            root: Some(OciRoot {
                path: "/rootfs".into(),
                readonly: Some(true),
            }),
            hostname: Some("node".into()),
            domainname: None,
            linux: Some(OciLinux {
                namespaces: Some(vec![
                    OciNamespace {
                        ns_type: "pid".into(),
                        path: None,
                    },
                    OciNamespace {
                        ns_type: "network".into(),
                        path: Some("/proc/1/ns/net".into()),
                    },
                ]),
                ..Default::default()
            }),
            mounts: None,
            annotations: None,
        }
    }

    #[test]
    fn flattens_minimal_spec_for_bare_runtime() {
        let config = BareRuntimeConfig::from_spec(&minimal_spec()).unwrap();

        assert_eq!(config.rootfs, "/rootfs");
        assert!(config.root_readonly);
        assert_eq!(config.args, vec!["/init"]);
        assert_eq!(config.cwd, "/");
        assert_eq!(config.hostname, "node");
        assert_eq!(config.uid, 1000);
        assert_eq!(config.gid, 1001);
        assert!(config.no_new_privileges);
        assert!(config.creates_namespace(BareNamespaceKind::Pid));
        assert_eq!(
            config.namespace_paths().collect::<Vec<_>>(),
            vec![(&BareNamespaceKind::Network, "/proc/1/ns/net")]
        );
    }

    #[test]
    fn applies_process_defaults() {
        let mut spec = minimal_spec();
        spec.process = Some(OciProcess::default());

        let config = BareRuntimeConfig::from_spec(&spec).unwrap();

        assert_eq!(config.args, vec!["/bin/sh"]);
        assert!(config.env.iter().any(|value| value.starts_with("PATH=")));
    }
}
