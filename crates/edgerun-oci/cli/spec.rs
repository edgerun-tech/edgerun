//! Spec command implementation — generates a default OCI spec JSON to stdout.

use crate::prelude::*;
use std::io;

pub fn cmd_spec(_opts: &crate::cli::GlobalOpts, _args: &[String]) -> io::Result<()> {
    let spec = default_spec();
    print!("{}", spec);
    Ok(())
}

fn default_spec() -> String {
    use crate::spec::*;

    let spec = OciSpec {
        version: "1.0.2".into(),
        platform: Some(OciPlatform {
            os: Some(crate::host_os().into()),
            arch: Some(crate::host_arch().into()),
            os_version: None,
            os_features: None,
        }),
        process: Some(OciProcess {
            terminal: Some(true),
            user: Some(OciUser {
                uid: Some(0),
                gid: Some(0),
                additional_gids: None,
                umask: None,
            }),
            args: Some(vec!["sh".into()]),
            env: Some(crate::validate::default_process_env()),
            cwd: Some("/".into()),
            capabilities: Some(OciCapabilities {
                bounding: Some(vec![]),
                effective: Some(vec![]),
                inheritable: Some(vec![]),
                permitted: Some(vec![]),
                ambient: Some(vec![]),
            }),
            rlimits: Some(vec![OciRlimit {
                ns_type: "RLIMIT_NOFILE".into(),
                hard: 1024,
                soft: 1024,
            }]),
            no_new_privileges: Some(true),
            oom_score_adj: Some(0),
            apparmor_profile: None,
            selinux_label: None,
            scheduler: None,
            console_size: None,
            io_priority: None,
        }),
        root: Some(OciRoot {
            path: "rootfs".into(),
            readonly: Some(false),
        }),
        hostname: Some("edgerun".into()),
        domainname: None,
        linux: Some(OciLinux {
            namespaces: Some(crate::default_namespaces()),
            masked_paths: Some(crate::default_masked_paths()),
            readonly_paths: Some(crate::default_readonly_paths()),
            rootfs_propagation: Some("private".into()),
            ..Default::default()
        }),
        mounts: Some(vec![
            OciMount {
                destination: "/proc".into(),
                mount_type: Some("proc".into()),
                source: Some("proc".into()),
                options: Some(vec!["nosuid".into(), "nodev".into(), "noexec".into()]),
                label: None,
                recursive: None,
                uid_mappings: None,
                gid_mappings: None,
            },
            OciMount {
                destination: "/dev".into(),
                mount_type: Some("tmpfs".into()),
                source: Some("tmpfs".into()),
                options: Some(vec![
                    "nosuid".into(),
                    "strictatime".into(),
                    "mode=755".into(),
                    "size=65536k".into(),
                ]),
                label: None,
                recursive: None,
                uid_mappings: None,
                gid_mappings: None,
            },
            OciMount {
                destination: "/dev/pts".into(),
                mount_type: Some("devpts".into()),
                source: Some("devpts".into()),
                options: Some(vec![
                    "nosuid".into(),
                    "noexec".into(),
                    "newinstance".into(),
                    "ptmxmode=0666".into(),
                    "mode=0620".into(),
                ]),
                label: None,
                recursive: None,
                uid_mappings: None,
                gid_mappings: None,
            },
            OciMount {
                destination: "/dev/shm".into(),
                mount_type: Some("tmpfs".into()),
                source: Some("shm".into()),
                options: Some(vec![
                    "nosuid".into(),
                    "noexec".into(),
                    "nodev".into(),
                    "mode=1777".into(),
                    "size=65536k".into(),
                ]),
                label: None,
                recursive: None,
                uid_mappings: None,
                gid_mappings: None,
            },
            OciMount {
                destination: "/dev/mqueue".into(),
                mount_type: Some("mqueue".into()),
                source: Some("mqueue".into()),
                options: Some(vec!["nosuid".into(), "nodev".into(), "noexec".into()]),
                label: None,
                recursive: None,
                uid_mappings: None,
                gid_mappings: None,
            },
            OciMount {
                destination: "/sys".into(),
                mount_type: Some("sysfs".into()),
                source: Some("sysfs".into()),
                options: Some(vec![
                    "nosuid".into(),
                    "noexec".into(),
                    "nodev".into(),
                    "ro".into(),
                ]),
                label: None,
                recursive: None,
                uid_mappings: None,
                gid_mappings: None,
            },
            OciMount {
                destination: "/sys/fs/cgroup".into(),
                mount_type: Some("cgroup".into()),
                source: Some("cgroup".into()),
                options: Some(vec![
                    "nosuid".into(),
                    "noexec".into(),
                    "nodev".into(),
                    "relatime".into(),
                    "ro".into(),
                ]),
                label: None,
                recursive: None,
                uid_mappings: None,
                gid_mappings: None,
            },
        ]),
        annotations: None,
    };

    spec.to_json_string_pretty()
}
