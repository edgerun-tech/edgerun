//! Container process configuration builder.
//!
//! Provides a fluent API for constructing container process configurations,
//! separating the concerns of spec parsing from child process setup.
//!
//! ## Usage
//!
//! ```ignore
//! let config = ContainerConfigBuilder::new("/var/lib/bundle/rootfs")
//!     .args(vec!["/bin/sh".into()])
//!     .cwd("/app".into())
//!     .env(vec!["PATH=/usr/bin".into()])
//!     .hostname("my-container".into())
//!     .namespaces(vec!["mount", "pid"])
//!     .uid(1000)
//!     .gid(1000)
//!     .no_new_privileges(true)
//!     .build()?;
//! ```

use crate::prelude::*;
use alloc::collections::BTreeMap;
use std::io;

use crate::json::{OciMount, OciRlimit, OciSpec};
use crate::validate::{default_process_args, default_process_env};

/// Configuration for a container child process.
///
/// Contains all the flattened parameters needed to set up namespaces,
/// rootfs, security, and execute the workload.
#[derive(Debug, Clone)]
pub struct ContainerProcessConfig {
    /// Root filesystem path.
    pub rootfs: String,
    /// Whether the rootfs should be read-only.
    pub root_readonly: bool,
    /// Process arguments.
    pub args: Vec<String>,
    /// Environment variables (KEY=VALUE format).
    pub env: Vec<String>,
    /// Working directory inside the container.
    pub cwd: String,
    /// Container hostname.
    pub hostname: String,
    /// UID to run as.
    pub uid: u32,
    /// GID to run as.
    pub gid: u32,
    /// Supplementary group IDs.
    pub additional_gids: Vec<u32>,
    /// Umask for the process.
    pub umask: Option<u32>,
    /// Namespace types to create (not path-based joins).
    pub namespace_types: Vec<String>,
    /// Namespace path-based joins (type:path pairs).
    pub namespace_paths: Vec<(String, String)>,
    /// Linux capabilities (effective set).
    pub cap_effective: Vec<String>,
    /// Linux capabilities (permitted set).
    pub cap_permitted: Vec<String>,
    /// Linux capabilities (inheritable set).
    pub cap_inheritable: Vec<String>,
    /// Linux capabilities (bounding set).
    pub cap_bounding: Vec<String>,
    /// Linux capabilities (ambient set).
    pub cap_ambient: Vec<String>,
    /// Resource limits.
    pub rlimits: Vec<OciRlimit>,
    /// OOM score adjustment.
    pub oom_score_adj: i64,
    /// Whether to set PR_SET_NO_NEW_PRIVS.
    pub no_new_privileges: bool,
    /// AppArmor profile name.
    pub apparmor_profile: Option<String>,
    /// Mount points.
    pub mounts: Vec<OciMount>,
    /// Paths to mask with /dev/null.
    pub masked_paths: Vec<String>,
    /// Paths to mount read-only.
    pub readonly_paths: Vec<String>,
    /// Rootfs propagation mode.
    pub rootfs_propagation: Option<String>,
    /// Sysctl parameters.
    pub sysctl: BTreeMap<String, String>,
    /// SELinux mount label.
    pub mount_label: Option<String>,
    /// Whether to allocate a pseudo-terminal (PTY).
    pub terminal: bool,
}

impl ContainerProcessConfig {
    /// Returns the combined namespace flags.
    pub fn namespace_flags(&self) -> i32 {
        crate::namespace_flags_from_types(self.namespace_types.iter().map(String::as_str))
    }

    /// Returns true if PID namespace is being created.
    pub fn has_pid_namespace(&self) -> bool {
        self.namespace_types.iter().any(|t| t == "pid")
    }

    /// Returns the namespace paths as a newline-separated string.
    pub fn namespace_paths_string(&self) -> String {
        self.namespace_paths
            .iter()
            .map(|(t, p)| format!("{}:{}", t, p))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// Builder for [`ContainerProcessConfig`].
pub struct ContainerConfigBuilder {
    rootfs: String,
    root_readonly: bool,
    args: Vec<String>,
    env: Vec<String>,
    cwd: String,
    hostname: String,
    uid: u32,
    gid: u32,
    additional_gids: Vec<u32>,
    umask: Option<u32>,
    namespace_types: Vec<String>,
    namespace_paths: Vec<(String, String)>,
    cap_effective: Vec<String>,
    cap_permitted: Vec<String>,
    cap_inheritable: Vec<String>,
    cap_bounding: Vec<String>,
    cap_ambient: Vec<String>,
    rlimits: Vec<OciRlimit>,
    oom_score_adj: i64,
    no_new_privileges: bool,
    apparmor_profile: Option<String>,
    mounts: Vec<OciMount>,
    masked_paths: Vec<String>,
    readonly_paths: Vec<String>,
    rootfs_propagation: Option<String>,
    sysctl: BTreeMap<String, String>,
    mount_label: Option<String>,
    terminal: bool,
}

impl ContainerConfigBuilder {
    /// Create a new builder with the given rootfs path.
    pub fn new(rootfs: &str) -> Self {
        Self {
            rootfs: rootfs.into(),
            root_readonly: false,
            args: default_process_args(),
            env: default_process_env(),
            cwd: "/".into(),
            hostname: "edgerun".into(),
            uid: 0,
            gid: 0,
            additional_gids: Vec::new(),
            umask: None,
            namespace_types: vec!["mount".into()],
            namespace_paths: Vec::new(),
            cap_effective: Vec::new(),
            cap_permitted: Vec::new(),
            cap_inheritable: Vec::new(),
            cap_bounding: Vec::new(),
            cap_ambient: Vec::new(),
            rlimits: Vec::new(),
            oom_score_adj: 0,
            no_new_privileges: true,
            apparmor_profile: None,
            mounts: Vec::new(),
            masked_paths: Vec::new(),
            readonly_paths: Vec::new(),
            rootfs_propagation: Some("private".into()),
            sysctl: BTreeMap::new(),
            mount_label: None,
            terminal: false,
        }
    }

    /// Create a builder from a full OCI spec.
    pub fn from_spec(spec: &OciSpec) -> io::Result<Self> {
        let root = spec
            .root
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no root in OCI spec"))?;

        let linux = spec.linux.clone().unwrap_or_default();
        let process = spec.process.clone().unwrap_or_default();
        let user = process.user.clone().unwrap_or_default();
        let caps = process.capabilities.clone().unwrap_or_default();

        let ns_list = linux
            .namespaces
            .clone()
            .unwrap_or_else(crate::default_namespaces);
        let ns_types: Vec<String> = ns_list
            .iter()
            .filter(|ns| ns.path.is_none())
            .map(|ns| ns.ns_type.clone())
            .collect();
        let ns_path_joins: Vec<(String, String)> = ns_list
            .iter()
            .filter_map(|ns| ns.path.as_ref().map(|p| (ns.ns_type.clone(), p.clone())))
            .collect();

        Ok(Self {
            rootfs: root.path.clone(),
            root_readonly: root.readonly.unwrap_or(false),
            args: process.args.unwrap_or_else(default_process_args),
            env: process.env.unwrap_or_else(default_process_env),
            cwd: process.cwd.unwrap_or_else(|| "/".into()),
            hostname: spec.hostname.clone().unwrap_or_else(|| "edgerun".into()),
            uid: user.uid.unwrap_or(0),
            gid: user.gid.unwrap_or(0),
            additional_gids: user.additional_gids.unwrap_or_default(),
            umask: user.umask,
            namespace_types: ns_types,
            namespace_paths: ns_path_joins,
            cap_effective: caps.effective.unwrap_or_default(),
            cap_permitted: caps.permitted.unwrap_or_default(),
            cap_inheritable: caps.inheritable.unwrap_or_default(),
            cap_bounding: caps.bounding.unwrap_or_default(),
            cap_ambient: caps.ambient.unwrap_or_default(),
            rlimits: process.rlimits.unwrap_or_default(),
            oom_score_adj: process.oom_score_adj.unwrap_or(0),
            no_new_privileges: process.no_new_privileges.unwrap_or(true),
            apparmor_profile: process.apparmor_profile,
            mounts: spec.mounts.clone().unwrap_or_default(),
            masked_paths: linux.masked_paths.unwrap_or_default(),
            readonly_paths: linux.readonly_paths.unwrap_or_default(),
            rootfs_propagation: linux.rootfs_propagation.or(Some("private".into())),
            sysctl: linux.sysctl.unwrap_or_default(),
            mount_label: linux.mount_label,
            terminal: process.terminal.unwrap_or(false),
        })
    }

    /// Set the rootfs path.
    pub fn rootfs(mut self, path: &str) -> Self {
        self.rootfs = path.into();
        self
    }

    /// Set whether the rootfs should be read-only.
    pub fn root_readonly(mut self, readonly: bool) -> Self {
        self.root_readonly = readonly;
        self
    }

    /// Set the process arguments.
    pub fn args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    /// Set the environment variables.
    pub fn env(mut self, env: Vec<String>) -> Self {
        self.env = env;
        self
    }

    /// Set the working directory.
    pub fn cwd(mut self, cwd: String) -> Self {
        self.cwd = cwd;
        self
    }

    /// Set the hostname.
    pub fn hostname(mut self, hostname: String) -> Self {
        self.hostname = hostname;
        self
    }

    /// Set the UID and GID.
    pub fn user(mut self, uid: u32, gid: u32) -> Self {
        self.uid = uid;
        self.gid = gid;
        self
    }

    /// Set supplementary group IDs.
    pub fn additional_gids(mut self, gids: Vec<u32>) -> Self {
        self.additional_gids = gids;
        self
    }

    /// Set the umask.
    pub fn umask(mut self, mask: u32) -> Self {
        self.umask = Some(mask);
        self
    }

    /// Set the namespaces to create.
    pub fn namespaces(mut self, types: Vec<String>) -> Self {
        self.namespace_types = types;
        self
    }

    /// Set namespace path-based joins.
    pub fn namespace_paths(mut self, paths: Vec<(String, String)>) -> Self {
        self.namespace_paths = paths;
        self
    }

    /// Set all capability sets at once.
    pub fn capabilities(
        mut self,
        effective: Vec<String>,
        permitted: Vec<String>,
        inheritable: Vec<String>,
        bounding: Vec<String>,
        ambient: Vec<String>,
    ) -> Self {
        self.cap_effective = effective;
        self.cap_permitted = permitted;
        self.cap_inheritable = inheritable;
        self.cap_bounding = bounding;
        self.cap_ambient = ambient;
        self
    }

    /// Set resource limits.
    pub fn rlimits(mut self, limits: Vec<OciRlimit>) -> Self {
        self.rlimits = limits;
        self
    }

    /// Set OOM score adjustment.
    pub fn oom_score_adj(mut self, adj: i64) -> Self {
        self.oom_score_adj = adj;
        self
    }

    /// Set no_new_privileges flag.
    pub fn no_new_privileges(mut self, flag: bool) -> Self {
        self.no_new_privileges = flag;
        self
    }

    /// Set AppArmor profile.
    pub fn apparmor(mut self, profile: String) -> Self {
        self.apparmor_profile = Some(profile);
        self
    }

    /// Set mounts.
    pub fn mounts(mut self, mounts: Vec<OciMount>) -> Self {
        self.mounts = mounts;
        self
    }

    /// Set masked paths.
    pub fn masked_paths(mut self, paths: Vec<String>) -> Self {
        self.masked_paths = paths;
        self
    }

    /// Set readonly paths.
    pub fn readonly_paths(mut self, paths: Vec<String>) -> Self {
        self.readonly_paths = paths;
        self
    }

    /// Set rootfs propagation mode.
    pub fn rootfs_propagation(mut self, mode: String) -> Self {
        self.rootfs_propagation = Some(mode);
        self
    }

    /// Set sysctl parameters.
    pub fn sysctl(mut self, params: BTreeMap<String, String>) -> Self {
        self.sysctl = params;
        self
    }

    /// Set mount label.
    pub fn mount_label(mut self, label: String) -> Self {
        self.mount_label = Some(label);
        self
    }

    /// Build the configuration.
    pub fn build(self) -> ContainerProcessConfig {
        ContainerProcessConfig {
            rootfs: self.rootfs,
            root_readonly: self.root_readonly,
            args: self.args,
            env: self.env,
            cwd: self.cwd,
            hostname: self.hostname,
            uid: self.uid,
            gid: self.gid,
            additional_gids: self.additional_gids,
            umask: self.umask,
            namespace_types: self.namespace_types,
            namespace_paths: self.namespace_paths,
            cap_effective: self.cap_effective,
            cap_permitted: self.cap_permitted,
            cap_inheritable: self.cap_inheritable,
            cap_bounding: self.cap_bounding,
            cap_ambient: self.cap_ambient,
            rlimits: self.rlimits,
            oom_score_adj: self.oom_score_adj,
            no_new_privileges: self.no_new_privileges,
            apparmor_profile: self.apparmor_profile,
            mounts: self.mounts,
            masked_paths: self.masked_paths,
            readonly_paths: self.readonly_paths,
            rootfs_propagation: self.rootfs_propagation,
            sysctl: self.sysctl,
            mount_label: self.mount_label,
            terminal: self.terminal,
        }
    }

    /// Request a pseudo-terminal (PTY) allocation for the container.
    pub fn terminal(mut self) -> Self {
        self.terminal = true;
        self
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;

    // ===========================================================================
    // ContainerConfigBuilder defaults
    // ===========================================================================

    #[test]
    fn builder_default_rootfs() {
        let config = ContainerConfigBuilder::new("/rootfs").build();
        assert_eq!(config.rootfs, "/rootfs");
    }

    #[test]
    fn builder_default_args_is_bin_sh() {
        let config = ContainerConfigBuilder::new("/rootfs").build();
        assert_eq!(config.args, vec!["/bin/sh"]);
    }

    #[test]
    fn builder_default_env_has_path() {
        let config = ContainerConfigBuilder::new("/rootfs").build();
        assert!(config.env.iter().any(|e| e.starts_with("PATH=")));
    }

    #[test]
    fn builder_default_cwd_is_root() {
        let config = ContainerConfigBuilder::new("/rootfs").build();
        assert_eq!(config.cwd, "/");
    }

    #[test]
    fn builder_default_hostname_is_edgerun() {
        let config = ContainerConfigBuilder::new("/rootfs").build();
        assert_eq!(config.hostname, "edgerun");
    }

    #[test]
    fn builder_default_user_is_root() {
        let config = ContainerConfigBuilder::new("/rootfs").build();
        assert_eq!(config.uid, 0);
        assert_eq!(config.gid, 0);
    }

    #[test]
    fn builder_default_no_new_privileges_is_true() {
        let config = ContainerConfigBuilder::new("/rootfs").build();
        assert!(config.no_new_privileges);
    }

    #[test]
    fn builder_default_namespaces_is_mount_only() {
        let config = ContainerConfigBuilder::new("/rootfs").build();
        assert_eq!(config.namespace_types, vec!["mount"]);
    }

    #[test]
    fn builder_default_rootfs_propagation_is_private() {
        let config = ContainerConfigBuilder::new("/rootfs").build();
        assert_eq!(config.rootfs_propagation, Some("private".into()));
    }

    #[test]
    fn builder_default_caps_are_empty() {
        let config = ContainerConfigBuilder::new("/rootfs").build();
        assert!(config.cap_effective.is_empty());
        assert!(config.cap_permitted.is_empty());
        assert!(config.cap_inheritable.is_empty());
        assert!(config.cap_bounding.is_empty());
        assert!(config.cap_ambient.is_empty());
    }

    // ===========================================================================
    // Builder fluent setters
    // ===========================================================================

    #[test]
    fn builder_custom_args() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .args(vec!["/bin/foo".into(), "-x".into()])
            .build();
        assert_eq!(config.args, vec!["/bin/foo", "-x"]);
    }

    #[test]
    fn builder_custom_env() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .env(vec!["FOO=bar".into(), "BAZ=qux".into()])
            .build();
        assert_eq!(config.env, vec!["FOO=bar", "BAZ=qux"]);
    }

    #[test]
    fn builder_custom_user() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .user(1000, 1000)
            .build();
        assert_eq!(config.uid, 1000);
        assert_eq!(config.gid, 1000);
    }

    #[test]
    fn builder_custom_hostname() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .hostname("my-container".into())
            .build();
        assert_eq!(config.hostname, "my-container");
    }

    #[test]
    fn builder_custom_namespaces() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .namespaces(vec!["mount".into(), "pid".into(), "network".into()])
            .build();
        assert_eq!(config.namespace_types.len(), 3);
        assert!(config.namespace_types.contains(&"mount".into()));
        assert!(config.namespace_types.contains(&"pid".into()));
    }

    #[test]
    fn builder_custom_caps() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .capabilities(
                vec!["CAP_NET_BIND_SERVICE".into()],
                vec!["CAP_NET_BIND_SERVICE".into()],
                vec![],
                vec!["CAP_NET_BIND_SERVICE".into()],
                vec![],
            )
            .build();
        assert_eq!(config.cap_effective, vec!["CAP_NET_BIND_SERVICE"]);
        assert_eq!(config.cap_bounding, vec!["CAP_NET_BIND_SERVICE"]);
    }

    #[test]
    fn builder_masked_paths() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .masked_paths(vec!["/proc/kcore".into(), "/proc/keys".into()])
            .build();
        assert_eq!(config.masked_paths.len(), 2);
    }

    #[test]
    fn builder_readonly_paths() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .readonly_paths(vec!["/proc/sys".into()])
            .build();
        assert_eq!(config.readonly_paths, vec!["/proc/sys"]);
    }

    #[test]
    fn builder_sysctl_params() {
        let mut sysctl = BTreeMap::new();
        sysctl.insert("net.ipv4.ip_forward".into(), "1".into());
        let config = ContainerConfigBuilder::new("/rootfs")
            .sysctl(sysctl)
            .build();
        assert_eq!(
            config.sysctl.get("net.ipv4.ip_forward"),
            Some(&"1".to_string())
        );
    }

    // ===========================================================================
    // from_spec
    // ===========================================================================

    #[test]
    fn from_spec_minimal() {
        let spec = crate::create_bundle("/rootfs", vec!["/bin/true".into()], None, None);
        let builder = ContainerConfigBuilder::from_spec(&spec).unwrap();
        let config = builder.build();

        assert_eq!(config.rootfs, "/rootfs");
        assert_eq!(config.args, vec!["/bin/true"]);
        assert_eq!(config.hostname, "edgerun");
    }

    #[test]
    fn from_spec_with_hostname() {
        let spec = crate::create_bundle(
            "/rootfs",
            vec!["/bin/true".into()],
            None,
            Some("myhost".into()),
        );
        let builder = ContainerConfigBuilder::from_spec(&spec).unwrap();
        let config = builder.build();

        assert_eq!(config.hostname, "myhost");
    }

    #[test]
    fn from_spec_with_env() {
        let mut spec = crate::create_bundle("/rootfs", vec!["/bin/true".into()], None, None);
        if let Some(ref mut proc) = spec.process {
            proc.env = Some(vec!["FOO=bar".into()]);
        }
        let builder = ContainerConfigBuilder::from_spec(&spec).unwrap();
        let config = builder.build();

        assert_eq!(config.env, vec!["FOO=bar"]);
    }

    #[test]
    fn from_spec_missing_root_errors() {
        let spec = crate::json::OciSpec {
            version: "1.0.2".into(),
            root: None,
            process: None,
            hostname: None,
            linux: None,
            mounts: None,
            platform: None,
            annotations: None,
            domainname: None,
        };
        let result = ContainerConfigBuilder::from_spec(&spec);
        assert!(result.is_err());
    }

    // ===========================================================================
    // ContainerProcessConfig methods
    // ===========================================================================

    #[test]
    fn config_namespace_flags_empty() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .namespaces(vec![])
            .build();
        assert_eq!(config.namespace_flags(), 0);
    }

    #[test]
    fn config_namespace_flags_mount() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .namespaces(vec!["mount".into()])
            .build();
        let flags = config.namespace_flags();
        assert_ne!(flags, 0);
        assert_eq!(flags, crate::syscalls::ns::NEWNS);
    }

    #[test]
    fn config_namespace_flags_multiple() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .namespaces(vec!["mount".into(), "pid".into()])
            .build();
        let flags = config.namespace_flags();
        assert_eq!(
            flags,
            crate::syscalls::ns::NEWNS | crate::syscalls::ns::NEWPID
        );
    }

    #[test]
    fn config_has_pid_namespace() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .namespaces(vec!["mount".into(), "pid".into()])
            .build();
        assert!(config.has_pid_namespace());
    }

    #[test]
    fn config_no_pid_namespace() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .namespaces(vec!["mount".into()])
            .build();
        assert!(!config.has_pid_namespace());
    }

    #[test]
    fn config_namespace_paths_string() {
        let config = ContainerConfigBuilder::new("/rootfs")
            .namespace_paths(vec![
                ("mount".into(), "/proc/1/ns/mnt".into()),
                ("pid".into(), "/proc/1/ns/pid".into()),
            ])
            .build();
        let s = config.namespace_paths_string();
        assert!(s.contains("mount:/proc/1/ns/mnt"));
        assert!(s.contains("pid:/proc/1/ns/pid"));
    }

    #[test]
    fn config_namespace_paths_empty_string() {
        let config = ContainerConfigBuilder::new("/rootfs").build();
        assert!(config.namespace_paths_string().is_empty());
    }

    // ===========================================================================
    // Edge cases
    // ===========================================================================

    #[test]
    fn builder_empty_args() {
        let config = ContainerConfigBuilder::new("/rootfs").args(vec![]).build();
        assert!(config.args.is_empty());
    }

    #[test]
    fn builder_chaining_is_fluent() {
        // Ensure all setters return Self for chaining
        let config = ContainerConfigBuilder::new("/rootfs")
            .rootfs("/new/rootfs")
            .root_readonly(true)
            .args(vec!["/bin/sh".into()])
            .env(vec!["PATH=/usr/bin".into()])
            .cwd("/app".into())
            .hostname("test".into())
            .user(1000, 1000)
            .umask(0o022)
            .namespaces(vec!["mount".into()])
            .no_new_privileges(true)
            .rootfs_propagation("slave".into())
            .build();

        assert_eq!(config.rootfs, "/new/rootfs");
        assert!(config.root_readonly);
        assert_eq!(config.cwd, "/app");
        assert_eq!(config.hostname, "test");
        assert_eq!(config.uid, 1000);
        assert_eq!(config.gid, 1000);
        assert_eq!(config.umask, Some(0o022));
        assert_eq!(config.rootfs_propagation, Some("slave".into()));
    }

    #[test]
    fn config_is_cloneable() {
        let config = ContainerConfigBuilder::new("/rootfs").build();
        let _clone = config.clone();
    }

    #[test]
    fn config_is_debuggable() {
        let config = ContainerConfigBuilder::new("/rootfs").build();
        let debug = format!("{:?}", config);
        assert!(debug.contains("ContainerProcessConfig"));
    }
}
