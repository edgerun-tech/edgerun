//! OCI spec generation from image config — produces JSON via edgerun-json.

use crate::prelude::*;
use crate::validate::{default_process_args, default_process_env};
use crate::{
    default_masked_paths, default_namespaces, default_readonly_paths, OciLinux, OciMount,
    OciProcess, OciRoot, OciSpec, OciUser, DEFAULT_MASKED_PATHS, DEFAULT_NAMESPACES,
    DEFAULT_READONLY_PATHS,
};
use edgerun_json::{json, to_string_pretty};

use super::config::ImageConfig;

pub fn generate_oci_spec_model(image_config: &ImageConfig, rootfs: &str) -> OciSpec {
    let process_config = image_config.config.as_ref();
    let args = image_process_args(process_config);
    let env = process_config
        .and_then(|c| c.env.as_ref())
        .cloned()
        .unwrap_or_else(default_process_env);
    let cwd = process_config
        .and_then(|c| c.working_dir.as_ref())
        .cloned()
        .unwrap_or_else(|| "/".into());
    let (uid, gid) = parse_user(process_config.and_then(|c| c.user.as_ref()));

    OciSpec {
        version: "1.0.2".into(),
        platform: None,
        process: Some(OciProcess {
            args: Some(if args.is_empty() {
                default_process_args()
            } else {
                args
            }),
            env: Some(env),
            cwd: Some(cwd),
            user: Some(OciUser {
                uid: Some(uid),
                gid: Some(gid),
                ..OciUser::default()
            }),
            no_new_privileges: Some(true),
            ..OciProcess::default()
        }),
        root: Some(OciRoot {
            path: rootfs.into(),
            readonly: None,
        }),
        hostname: Some("edgerun".into()),
        domainname: None,
        linux: Some(OciLinux {
            namespaces: Some(default_namespaces()),
            masked_paths: Some(default_masked_paths()),
            readonly_paths: Some(default_readonly_paths()),
            ..OciLinux::default()
        }),
        mounts: volume_mounts_model(image_config),
        annotations: None,
    }
}

pub fn generate_oci_spec(image_config: &ImageConfig, rootfs: &str) -> String {
    let process_config = image_config.config.as_ref();

    let args = image_process_args(process_config);

    let env = process_config
        .and_then(|c| c.env.as_ref())
        .cloned()
        .unwrap_or_else(default_process_env);

    let cwd = process_config
        .and_then(|c| c.working_dir.as_ref())
        .cloned()
        .unwrap_or_else(|| "/".into());

    let (uid, gid) = parse_user(process_config.and_then(|c| c.user.as_ref()));

    let volume_mounts: Vec<edgerun_json::JsonValue> = image_config
        .config
        .as_ref()
        .and_then(|c| c.volumes.as_ref())
        .map(|volumes| {
            volumes
                .keys()
                .map(|dest| {
                    let dest_str = dest.as_str();
                    json!({
                        "destination": dest_str,
                        "type": "tmpfs",
                        "source": "tmpfs",
                        "options": ["nosuid", "nodev", "noexec"]
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let process_args: Vec<edgerun_json::JsonValue> = if args.is_empty() {
        vec![json!("/bin/sh")]
    } else {
        args.iter().map(|a| json!(a.as_str())).collect()
    };

    let env_json: Vec<edgerun_json::JsonValue> = env.iter().map(|e| json!(e.as_str())).collect();
    let namespaces_json: Vec<edgerun_json::JsonValue> = DEFAULT_NAMESPACES
        .iter()
        .map(|(ns_type, path)| {
            json!({
                "type": *ns_type,
                "path": path.unwrap_or("")
            })
        })
        .collect();
    let masked_paths_json: Vec<edgerun_json::JsonValue> = DEFAULT_MASKED_PATHS
        .iter()
        .map(|path| json!(*path))
        .collect();
    let readonly_paths_json: Vec<edgerun_json::JsonValue> = DEFAULT_READONLY_PATHS
        .iter()
        .map(|path| json!(*path))
        .collect();

    let mut spec = json!({
        "ociVersion": "1.0.2",
        "process": {
            "args": process_args,
            "env": env_json,
            "cwd": cwd,
            "noNewPrivileges": true,
            "user": {
                "uid": uid,
                "gid": gid
            }
        },
        "root": {
            "path": rootfs
        },
        "hostname": "edgerun",
        "linux": {
            "namespaces": namespaces_json,
            "maskedPaths": masked_paths_json,
            "readonlyPaths": readonly_paths_json
        }
    });

    // Insert mounts if there are volume mounts
    if !volume_mounts.is_empty() {
        if let edgerun_json::JsonValue::Object(ref mut fields) = spec {
            fields.push_field("mounts", edgerun_json::JsonValue::Array(volume_mounts));
        }
    }

    to_string_pretty(&spec).unwrap_or_default()
}

fn parse_user(user_str: Option<&String>) -> (u32, u32) {
    if let Some(user) = user_str {
        let (uid, gid) = user.split_once(':').unwrap_or((user.as_str(), "0"));
        (uid.parse().unwrap_or(0), gid.parse().unwrap_or(0))
    } else {
        (0, 0)
    }
}

fn image_process_args(process_config: Option<&super::config::ImageConfigInner>) -> Vec<String> {
    process_config
        .and_then(|c| c.entrypoint.as_ref())
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .chain(
            process_config
                .and_then(|c| c.cmd.as_ref())
                .cloned()
                .unwrap_or_default(),
        )
        .collect()
}

fn volume_mounts_model(image_config: &ImageConfig) -> Option<Vec<OciMount>> {
    image_config
        .config
        .as_ref()
        .and_then(|config| config.volumes.as_ref())
        .map(|volumes| {
            volumes
                .keys()
                .map(|dest| OciMount {
                    destination: dest.clone(),
                    mount_type: Some("tmpfs".into()),
                    source: Some("tmpfs".into()),
                    options: Some(vec!["nosuid".into(), "nodev".into(), "noexec".into()]),
                    label: None,
                    recursive: None,
                    uid_mappings: None,
                    gid_mappings: None,
                })
                .collect()
        })
        .filter(|mounts: &Vec<OciMount>| !mounts.is_empty())
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
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
}
