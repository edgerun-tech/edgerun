use super::*;

#[test]
fn parses_image_config_with_edgerun_json() {
    let config = parse_image_config(
        br#"{
                "architecture":"amd64",
                "os":"linux",
                "config":{
                    "User":"1000:1001",
                    "Env":["PATH=/bin","A=B"],
                    "Entrypoint":["/init"],
                    "Cmd":["--serve"],
                    "WorkingDir":"/app",
                    "Volumes":{"/data":{}},
                    "Labels":{"org.opencontainers.image.title":"demo"},
                    "StopSignal":"SIGTERM"
                },
                "rootfs":{"type":"layers","diff_ids":["sha256:a"]},
                "history":[{"created_by":"test","empty_layer":true}]
            }"#,
    )
    .unwrap();

    assert_eq!(config.architecture.as_deref(), Some("amd64"));
    let inner = config.config.unwrap();
    assert_eq!(inner.user.as_deref(), Some("1000:1001"));
    assert_eq!(inner.entrypoint.unwrap(), vec!["/init"]);
    assert_eq!(inner.cmd.unwrap(), vec!["--serve"]);
    assert!(inner.volumes.unwrap().contains_key("/data"));
    assert_eq!(config.rootfs.unwrap().diff_ids, vec!["sha256:a"]);
    assert_eq!(config.history.unwrap()[0].empty_layer, Some(true));
}

#[test]
fn parses_single_manifest_descriptor_config_with_edgerun_json() {
    let manifest = parse_single_manifest(
            br#"{
                "schemaVersion":2,
                "config":{"mediaType":"application/vnd.oci.image.config.v1+json","digest":"sha256:cfg","size":42},
                "layers":[{"mediaType":"application/vnd.oci.image.layer.v1.tar","digest":"sha256:layer","size":7}]
            }"#,
        )
        .unwrap();

    assert_eq!(manifest.config_digest, "sha256:cfg");
    assert_eq!(manifest.config_size, Some(42));
    assert_eq!(
        manifest.config_media_type.as_deref(),
        Some("application/vnd.oci.image.config.v1+json")
    );
    assert_eq!(manifest.layers[0].digest, "sha256:layer");
}

#[test]
fn parses_image_index_with_edgerun_json() {
    let manifest = parse_manifest(
        br#"{
                "schemaVersion":2,
                "mediaType":"application/vnd.oci.image.index.v1+json",
                "manifests":[{
                    "mediaType":"application/vnd.oci.image.manifest.v1+json",
                    "digest":"sha256:m",
                    "size":10,
                    "platform":{"architecture":"amd64","os":"linux"}
                }]
            }"#,
    )
    .unwrap();

    match manifest {
        ImageManifest::Index(index) => {
            assert_eq!(index.manifests[0].digest, "sha256:m");
            assert_eq!(
                index.manifests[0]
                    .platform
                    .as_ref()
                    .unwrap()
                    .architecture
                    .as_deref(),
                Some("amd64")
            );
        }
        ImageManifest::Single(_) => panic!("expected image index"),
    }
}
