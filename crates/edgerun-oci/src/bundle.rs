//! OCI bundle creation and serialization.
//!
//! Creates minimal OCI bundle directories with `config.json` + rootfs path.

use crate::prelude::*;
use std::fs;
use std::io;
use std::path::Path;

use crate::spec::{OciLinux, OciProcess, OciRoot, OciSpec, OciUser};

/// Create a minimal OCI bundle from a rootfs directory and command.
/// Create a minimal OCI bundle from a rootfs directory and command.
pub fn create_bundle(
    rootfs_path: &str,
    args: Vec<String>,
    env: Option<Vec<String>>,
    hostname: Option<String>,
) -> OciSpec {
    OciSpec {
        version: "1.0.2".into(),
        platform: Some(crate::spec::OciPlatform {
            os: Some(crate::host_os().into()),
            arch: Some(crate::host_arch().into()),
            os_version: None,
            os_features: None,
        }),
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
        domainname: None,
        linux: Some(OciLinux {
            namespaces: Some(crate::default_namespaces()),
            masked_paths: Some(crate::default_masked_paths()),
            readonly_paths: Some(crate::default_readonly_paths()),
            ..Default::default()
        }),
        mounts: Some(vec![]),
        annotations: None,
    }
}

/// Serialize an OCI spec to config.json in a bundle directory.
pub fn write_bundle(bundle_path: &Path, spec: &OciSpec) -> io::Result<()> {
    fs::create_dir_all(bundle_path)?;
    let json = crate::spec::spec_to_json_string(spec);
    fs::write(bundle_path.join("config.json"), json)?;
    Ok(())
}
