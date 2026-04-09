//! OCI spec generation from image config — produces JSON directly.

use std::collections::HashMap;
use std::path::Path;

use crate::config::ImageConfig;

/// A minimal JSON value for OCI spec output.
#[derive(Clone, Debug)]
enum Json {
    Null,
    Bool(bool),
    Num(u64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn obj(pairs: Vec<(&str, Json)>) -> Json {
        Json::Obj(pairs.into_iter().map(|(k, v)| (k.to_string(), v)).collect())
    }

    fn arr(items: Vec<Json>) -> Json {
        Json::Arr(items)
    }

    fn str(s: &str) -> Json {
        Json::Str(s.to_string())
    }

    fn to_string_pretty(&self) -> String {
        format_json(self, 0)
    }
}

fn format_json(v: &Json, indent: usize) -> String {
    let pad = "  ".repeat(indent);
    let inner_pad = "  ".repeat(indent + 1);
    match v {
        Json::Null => "null".into(),
        Json::Bool(b) => b.to_string(),
        Json::Num(n) => n.to_string(),
        Json::Str(s) => format!("\"{}\"", escape_str(s)),
        Json::Arr(arr) => {
            if arr.is_empty() {
                return "[]".into();
            }
            let items = arr
                .iter()
                .map(|v| format!("{}{}", inner_pad, format_json(v, indent + 1)))
                .collect::<Vec<_>>()
                .join(",\n");
            format!("[\n{}\n{}]", items, pad)
        }
        Json::Obj(fields) => {
            if fields.is_empty() {
                return "{}".into();
            }
            let items = fields
                .iter()
                .map(|(k, v)| {
                    format!(
                        "{}\"{}\": {}",
                        inner_pad,
                        escape_str(k),
                        format_json(v, indent + 1)
                    )
                })
                .collect::<Vec<_>>()
                .join(",\n");
            format!("{{\n{}\n{}}}", items, pad)
        }
    }
}

fn escape_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

fn default_namespaces() -> Json {
    Json::arr(vec![
        Json::obj(vec![("type", Json::str("pid")), ("path", Json::str(""))]),
        Json::obj(vec![("type", Json::str("network")), ("path", Json::str(""))]),
        Json::obj(vec![("type", Json::str("ipc")), ("path", Json::str(""))]),
        Json::obj(vec![("type", Json::str("uts")), ("path", Json::str(""))]),
        Json::obj(vec![
            ("type", Json::str("mount")),
            ("path", Json::str("")),
        ]),
    ])
}

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
            vec!["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
                .into()]
        });

    let cwd = process_config
        .and_then(|c| c.working_dir.as_ref())
        .cloned()
        .unwrap_or_else(|| "/".into());

    let (uid, gid) = parse_user(process_config.and_then(|c| c.user.as_ref()));

    let volume_mounts: Vec<Json> = image_config
        .config
        .as_ref()
        .and_then(|c| c.volumes.as_ref())
        .map(|volumes| {
            volumes
                .keys()
                .map(|dest| {
                    Json::obj(vec![
                        ("destination", Json::str(dest)),
                        ("type", Json::str("tmpfs")),
                        ("source", Json::str("tmpfs")),
                        (
                            "options",
                            Json::arr(vec![
                                Json::str("nosuid"),
                                Json::str("nodev"),
                                Json::str("noexec"),
                            ]),
                        ),
                    ])
                })
                .collect()
        })
        .unwrap_or_default();

    let process_args = if args.is_empty() {
        vec![Json::str("/bin/sh")]
    } else {
        args.into_iter().map(|a| Json::str(&a)).collect()
    };

    let spec = Json::obj(vec![
        ("ociVersion", Json::str("1.0.2")),
        (
            "process",
            Json::obj(vec![
                ("args", Json::arr(process_args)),
                (
                    "env",
                    Json::arr(env.into_iter().map(|e| Json::str(&e)).collect()),
                ),
                ("cwd", Json::str(&cwd)),
                ("noNewPrivileges", Json::Bool(true)),
                (
                    "user",
                    Json::obj(vec![
                        ("uid", Json::Num(uid as u64)),
                        ("gid", Json::Num(gid as u64)),
                    ]),
                ),
            ]),
        ),
        (
            "root",
            Json::obj(vec![(
                "path",
                Json::str(rootfs.to_str().unwrap_or("/")),
            )]),
        ),
        ("hostname", Json::str("edgerun")),
        (
            "linux",
            Json::obj(vec![
                ("namespaces", default_namespaces()),
                (
                    "maskedPaths",
                    Json::arr(vec![
                        Json::str("/proc/acpi"),
                        Json::str("/proc/kcore"),
                        Json::str("/proc/keys"),
                        Json::str("/proc/latency_stats"),
                        Json::str("/proc/timer_list"),
                        Json::str("/proc/timer_stats"),
                        Json::str("/proc/sched_debug"),
                        Json::str("/proc/scsi"),
                        Json::str("/sys/firmware"),
                    ]),
                ),
                (
                    "readonlyPaths",
                    Json::arr(vec![
                        Json::str("/proc/asound"),
                        Json::str("/proc/bus"),
                        Json::str("/proc/fs"),
                        Json::str("/proc/irq"),
                        Json::str("/proc/sys"),
                        Json::str("/proc/sysrq-trigger"),
                    ]),
                ),
            ]),
        ),
    ]);

    // Add mounts if there are volume mounts
    if !volume_mounts.is_empty() {
        // We need to rebuild with mounts — simpler to just add it
        let spec_str = spec.to_string_pretty();
        // Just insert mounts before the final }
        let mounts_json = format!(
            ",\n  \"mounts\": [\n{}\n  ]",
            volume_mounts
                .iter()
                .map(|m| format!("    {}", m.to_string_pretty()))
                .collect::<Vec<_>>()
                .join(",\n")
        );
        // Insert before the last }
        if let Some(pos) = spec_str.rfind('}') {
            return format!("{}{}\n}}", &spec_str[..pos], mounts_json);
        }
    }

    spec.to_string_pretty()
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
