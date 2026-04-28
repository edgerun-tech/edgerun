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
