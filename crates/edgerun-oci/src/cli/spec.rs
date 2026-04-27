//! Spec command implementation — generates a default OCI spec JSON to stdout.

use crate::prelude::*;
use std::io;

pub fn cmd_spec(_opts: &crate::cli::GlobalOpts, _args: &[String]) -> io::Result<()> {
    let spec = default_spec();
    print!("{}", spec);
    Ok(())
}

fn default_spec() -> String {
    use crate::json::*;

    let spec = OciSpec {
        version: "1.0.2".into(),
        platform: Some(OciPlatform {
            os: Some(if cfg!(target_os = "linux") {
                "linux".into()
            } else {
                "unknown".into()
            }),
            arch: Some(if cfg!(target_arch = "x86_64") {
                "amd64".into()
            } else if cfg!(target_arch = "aarch64") {
                "arm64".into()
            } else {
                "unknown".into()
            }),
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
            env: Some(vec![
                "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into(),
                "TERM=xterm".into(),
            ]),
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
            masked_paths: Some(vec![
                "/proc/acpi".into(),
                "/proc/kcore".into(),
                "/proc/keys".into(),
                "/proc/latency_stats".into(),
                "/proc/timer_list".into(),
                "/proc/timer_stats".into(),
                "/proc/sched_debug".into(),
                "/proc/scsi".into(),
                "/sys/firmware".into(),
            ]),
            readonly_paths: Some(vec![
                "/proc/asound".into(),
                "/proc/bus".into(),
                "/proc/fs".into(),
                "/proc/irq".into(),
                "/proc/sys".into(),
                "/proc/sysrq-trigger".into(),
            ]),
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
