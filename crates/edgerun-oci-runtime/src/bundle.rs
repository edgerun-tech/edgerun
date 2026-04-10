//! OCI bundle creation and serialization.
//!
//! Creates minimal OCI bundle directories with `config.json` + rootfs path.

use std::fs;
use std::io;
use std::path::Path;

use crate::json::{
    OciLinux, OciProcess, OciRoot, OciSpec, OciUser,
};
use crate::default_namespaces;

/// Create a minimal OCI bundle from a rootfs directory and command.
pub fn create_bundle(
    rootfs_path: &str,
    args: Vec<String>,
    env: Option<Vec<String>>,
    hostname: Option<String>,
) -> OciSpec {
    OciSpec {
        version: "1.0.2".into(),
        platform: None,
        process: Some(OciProcess {
            args: Some(args),
            env,
            cwd: Some("/".into()),
            no_new_privileges: Some(true),
            user: Some(OciUser {
                uid: Some(0),
                gid: Some(0),
                additional_gids: None,
                umask: None,
            }),
            capabilities: None,
            ..Default::default()
        }),
        root: Some(OciRoot {
            path: rootfs_path.into(),
            readonly: None,
        }),
        hostname: hostname.or_else(|| Some("edgerun".into())),
        linux: Some(OciLinux {
            namespaces: Some(default_namespaces()),
            masked_paths: Some(vec![
                "/proc/acpi".into(), "/proc/kcore".into(), "/proc/keys".into(),
                "/proc/latency_stats".into(), "/proc/timer_list".into(),
                "/proc/timer_stats".into(), "/proc/sched_debug".into(),
                "/proc/scsi".into(), "/sys/firmware".into(),
            ]),
            readonly_paths: Some(vec![
                "/proc/asound".into(), "/proc/bus".into(), "/proc/fs".into(),
                "/proc/irq".into(), "/proc/sys".into(), "/proc/sysrq-trigger".into(),
            ]),
            ..Default::default()
        }),
        mounts: Some(vec![]),
        annotations: None,
    }
}

/// Serialize an OCI spec to config.json in a bundle directory.
pub fn write_bundle(bundle_path: &Path, spec: &OciSpec) -> io::Result<()> {
    fs::create_dir_all(bundle_path)?;
    let json = spec.to_json_string();
    fs::write(bundle_path.join("config.json"), json)?;
    Ok(())
}
