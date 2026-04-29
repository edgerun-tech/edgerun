//! OCI spec generation from image config — produces JSON via edgerun-json.

use crate::prelude::*;
use crate::validate::{default_process_args, default_process_env};
use crate::{
    default_masked_paths, default_namespaces, default_readonly_paths, OciLinux, OciMount,
    OciProcess, OciRoot, OciSpec, OciUser,
};

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
    generate_oci_spec_model(image_config, rootfs).to_json_string_pretty()
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
#[path = "../../tests/unit_src/src/registry/oci_spec_tests.rs"]
mod tests;
