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
    let spec = crate::spec::OciSpec {
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
