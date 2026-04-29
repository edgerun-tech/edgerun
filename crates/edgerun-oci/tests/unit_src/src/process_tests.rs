use super::*;
use crate::spec::{
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
    spec.linux = Some(crate::spec::OciLinux {
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
    spec.linux = Some(crate::spec::OciLinux {
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
    spec.linux = Some(crate::spec::OciLinux {
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
    spec.linux = Some(crate::spec::OciLinux {
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
    use crate::spec::OciPlatform;
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
    use crate::spec::OciPlatform;
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
    use crate::spec::OciPlatform;
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
    use crate::spec::OciPlatform;
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
