use super::*;
use crate::spec::{
    OciCapabilities, OciLinux, OciLinuxSeccomp, OciNamespace, OciProcess, OciRlimit, OciRoot,
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
        domainname: None,
        linux: None,
        mounts: None,
        annotations: None,
    }
}

#[test]
fn accepts_minimal() {
    assert!(validate_spec(&minimal_spec()).is_ok());
}

#[test]
fn rejects_missing_root() {
    let mut spec = minimal_spec();
    spec.root = None;
    assert!(validate_spec(&spec)
        .unwrap_err()
        .to_string()
        .contains("missing root"));
}

#[test]
fn rejects_missing_process() {
    let mut spec = minimal_spec();
    spec.process = None;
    assert!(validate_spec(&spec)
        .unwrap_err()
        .to_string()
        .contains("missing process"));
}

#[test]
fn rejects_empty_args() {
    let mut spec = minimal_spec();
    spec.process.as_mut().unwrap().args = Some(vec![]);
    assert!(validate_spec(&spec)
        .unwrap_err()
        .to_string()
        .contains("args must not be empty"));
}

#[test]
fn rejects_unknown_capability() {
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
fn accepts_valid_capability() {
    let mut spec = minimal_spec();
    spec.process.as_mut().unwrap().capabilities = Some(OciCapabilities {
        effective: Some(vec!["CAP_NET_BIND_SERVICE".into()]),
        ..Default::default()
    });
    assert!(validate_spec(&spec).is_ok());
}

#[test]
fn rejects_unknown_namespace() {
    let mut spec = minimal_spec();
    spec.linux = Some(OciLinux {
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
fn allows_path_based_namespace() {
    let mut spec = minimal_spec();
    spec.linux = Some(OciLinux {
        namespaces: Some(vec![OciNamespace {
            ns_type: "custom".into(),
            path: Some("/var/run/ns/custom".into()),
        }]),
        ..Default::default()
    });
    assert!(validate_spec(&spec).is_ok());
}

#[test]
fn rejects_unknown_rlimit() {
    let mut spec = minimal_spec();
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
fn accepts_valid_seccomp_action() {
    let mut spec = minimal_spec();
    spec.linux = Some(OciLinux {
        seccomp: Some(OciLinuxSeccomp {
            default_action: Some(OciSeccompAction::Allow),
            ..Default::default()
        }),
        ..Default::default()
    });
    assert!(validate_spec(&spec).is_ok());
}
