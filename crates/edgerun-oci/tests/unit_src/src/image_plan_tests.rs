use super::*;
use crate::registry::config::{ImageConfigInner, RootFs};

fn image_config() -> ImageConfig {
    ImageConfig {
        architecture: Some("amd64".into()),
        os: Some("linux".into()),
        config: Some(ImageConfigInner {
            user: None,
            env: Some(vec!["PATH=/bin".into()]),
            entrypoint: Some(vec!["/init".into()]),
            cmd: None,
            working_dir: None,
            exposed_ports: None,
            volumes: None,
            labels: None,
            stop_signal: None,
        }),
        rootfs: Some(RootFs {
            r#type: "layers".into(),
            diff_ids: vec!["sha256:diff".into()],
        }),
        history: None,
    }
}

fn manifest() -> SingleManifest {
    SingleManifest {
        config_digest: "sha256:config".into(),
        layers: vec![LayerDescriptor {
            media_type: Some("application/vnd.oci.image.layer.v1.tar".into()),
            digest: "sha256:layer".into(),
            size: 12,
        }],
    }
}

#[test]
fn builds_bare_image_plan() {
    let plan =
        BareImagePlan::from_manifest_config(&manifest(), &image_config(), "/rootfs").unwrap();

    assert_eq!(plan.config_digest, "sha256:config");
    assert_eq!(
        plan.layer_digests().collect::<Vec<_>>(),
        vec!["sha256:layer"]
    );
    assert_eq!(plan.diff_ids, vec!["sha256:diff"]);
    assert_eq!(plan.runtime.rootfs, "/rootfs");
    assert_eq!(plan.runtime.args, vec!["/init"]);
    assert!(!plan.has_layer_count_mismatch());
}

#[test]
fn validates_digest_references() {
    assert!(validate_digest_reference(
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    ));
    assert!(!validate_digest_reference("sha256:layer"));
    assert!(!validate_digest_reference(
        "SHA256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
    ));
    assert!(!validate_digest_reference("missing-separator"));
}

#[test]
fn validates_plan_descriptors() {
    let mut manifest = manifest();
    manifest.config_digest =
        "sha256:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into();
    manifest.layers[0].digest =
        "sha256:abcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcdefabcd".into();
    let mut config = image_config();
    config.rootfs.as_mut().unwrap().diff_ids[0] =
        "sha256:1111111111111111111111111111111111111111111111111111111111111111".into();
    let plan = BareImagePlan::from_manifest_config(&manifest, &config, "/rootfs").unwrap();

    assert!(plan.validate_descriptors().is_ok());
}

#[test]
fn rejects_bad_plan_digest() {
    let plan =
        BareImagePlan::from_manifest_config(&manifest(), &image_config(), "/rootfs").unwrap();

    let error = plan.validate_descriptors().unwrap_err();

    assert_eq!(
        error,
        ImagePlanError::InvalidDigest {
            field: "config.digest".into(),
            digest: "sha256:config".into()
        }
    );
}

#[test]
fn detects_layer_count_mismatch() {
    let mut config = image_config();
    config
        .rootfs
        .as_mut()
        .unwrap()
        .diff_ids
        .push("sha256:extra".into());
    let plan = BareImagePlan::from_manifest_config(&manifest(), &config, "/rootfs").unwrap();

    assert!(plan.has_layer_count_mismatch());
}

#[test]
fn selects_manifest_for_platform() {
    let index = ImageIndex {
        media_type: None,
        manifests: vec![
            ManifestDescriptor {
                media_type: None,
                digest: "sha256:arm".into(),
                size: 1,
                platform: Some(PlatformDescriptor {
                    architecture: Some("arm64".into()),
                    os: Some("linux".into()),
                }),
            },
            ManifestDescriptor {
                media_type: None,
                digest: "sha256:amd".into(),
                size: 1,
                platform: Some(PlatformDescriptor {
                    architecture: Some("amd64".into()),
                    os: Some("linux".into()),
                }),
            },
        ],
    };

    let selected = select_manifest_for_target(&index, "linux", "amd64").unwrap();
    assert_eq!(selected.digest, "sha256:amd");
}

#[test]
fn builds_plan_from_manifest_and_config_bytes() {
    let manifest_json = br#"{
            "schemaVersion":2,
            "config":{"digest":"sha256:config","size":10},
            "layers":[{"digest":"sha256:layer","size":12}]
        }"#;
    let config_json = br#"{
            "architecture":"amd64",
            "os":"linux",
            "config":{"Entrypoint":["/init"],"Env":["PATH=/bin"]},
            "rootfs":{"type":"layers","diff_ids":["sha256:diff"]}
        }"#;

    let plan =
        BareImagePlan::from_manifest_config_bytes(manifest_json, config_json, "/rootfs").unwrap();

    assert_eq!(plan.config_digest, "sha256:config");
    assert_eq!(plan.layer_count(), 1);
    assert_eq!(plan.runtime.args, vec!["/init"]);
}

#[test]
fn selects_manifest_digest_from_index_bytes() {
    let index_json = br#"{
            "schemaVersion":2,
            "manifests":[
                {"digest":"sha256:arm","size":1,"platform":{"architecture":"arm64","os":"linux"}},
                {"digest":"sha256:amd","size":1,"platform":{"architecture":"amd64","os":"linux"}}
            ]
        }"#;

    let digest = selected_manifest_digest_from_index_bytes(index_json, "linux", "amd64").unwrap();

    assert_eq!(digest, "sha256:amd");
}

#[test]
fn missing_platform_reports_target() {
    let index_json = br#"{
            "schemaVersion":2,
            "manifests":[
                {"digest":"sha256:arm","size":1,"platform":{"architecture":"arm64","os":"linux"}}
            ]
        }"#;

    let error =
        selected_manifest_digest_from_index_bytes(index_json, "linux", "amd64").unwrap_err();

    assert_eq!(
        error,
        ImagePlanError::PlatformNotFound {
            os: "linux".into(),
            arch: "amd64".into()
        }
    );
}
