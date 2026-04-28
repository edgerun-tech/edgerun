use super::*;
use crate::validate_spec;
use alloc::collections::BTreeMap;

#[test]
fn generates_valid_oci_spec_model_with_edgerun_json() {
    let mut volumes = BTreeMap::new();
    volumes.insert("/data".into(), edgerun_json::JsonValue::Null);
    let image_config = ImageConfig {
        architecture: Some("amd64".into()),
        os: Some("linux".into()),
        config: Some(super::super::config::ImageConfigInner {
            user: Some("1000:1001".into()),
            env: Some(vec!["PATH=/bin".into()]),
            entrypoint: Some(vec!["/init".into()]),
            cmd: Some(vec!["--serve".into()]),
            working_dir: Some("/app".into()),
            exposed_ports: None,
            volumes: Some(volumes),
            labels: None,
            stop_signal: None,
        }),
        rootfs: None,
        history: None,
    };

    let spec = generate_oci_spec_model(&image_config, "/rootfs");

    validate_spec(&spec).unwrap();
    let process = spec.process.unwrap();
    assert_eq!(process.args.unwrap(), vec!["/init", "--serve"]);
    assert_eq!(process.cwd.as_deref(), Some("/app"));
    assert_eq!(process.user.unwrap().uid, Some(1000));
    assert_eq!(spec.root.unwrap().path, "/rootfs");
    assert_eq!(spec.mounts.unwrap()[0].destination, "/data");
}
