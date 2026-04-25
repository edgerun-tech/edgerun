//! OCI spec generation from image config — produces JSON via edgerun-json.

use std::path::Path;

use edgerun_json::{json, to_string_pretty};

use super::config::ImageConfig;

pub fn generate_oci_spec(image_config: &ImageConfig, rootfs: &Path) -> String {
    let process_config = image_config.config.as_ref();

    let args = process_config
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
        .collect::<Vec<_>>();

    let env = process_config
        .and_then(|c| c.env.as_ref())
        .cloned()
        .unwrap_or_else(|| {
            vec!["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".into()]
        });

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
            "path": rootfs.to_str().unwrap_or("/")
        },
        "hostname": "edgerun",
        "linux": {
            "namespaces": [
                { "type": "pid", "path": "" },
                { "type": "network", "path": "" },
                { "type": "ipc", "path": "" },
                { "type": "uts", "path": "" },
                { "type": "mount", "path": "" }
            ],
            "maskedPaths": [
                "/proc/acpi", "/proc/kcore", "/proc/keys",
                "/proc/latency_stats", "/proc/timer_list",
                "/proc/timer_stats", "/proc/sched_debug",
                "/proc/scsi", "/sys/firmware"
            ],
            "readonlyPaths": [
                "/proc/asound", "/proc/bus", "/proc/fs",
                "/proc/irq", "/proc/sys", "/proc/sysrq-trigger"
            ]
        }
    });

    // Insert mounts if there are volume mounts
    if !volume_mounts.is_empty() {
        if let edgerun_json::JsonValue::Object(ref mut fields) = spec {
            fields.push((
                "mounts".to_string(),
                edgerun_json::JsonValue::Array(volume_mounts),
            ));
        }
    }

    to_string_pretty(&spec).unwrap_or_default()
}

fn parse_user(user_str: Option<&String>) -> (u32, u32) {
    if let Some(s) = user_str {
        let parts: Vec<&str> = s.split(':').collect();
        let uid = parts[0].parse().unwrap_or(0);
        let gid = if parts.len() > 1 {
            parts[1].parse().unwrap_or(0)
        } else {
            0
        };
        (uid, gid)
    } else {
        (0, 0)
    }
}
