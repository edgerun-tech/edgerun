//! Inspect command implementation.

use crate::prelude::*;
use std::io;

use crate::json::OciSpec;
use crate::state::{container_state_dir, load_state, save_state, ContainerState};

pub fn cmd_inspect(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    let id = crate::cli::require_container_id(args)?;
    let mut state = load_state(id)?;
    refresh_state(id, &mut state);
    let spec = crate::cli::load_runtime_or_bundle_spec(&state.id, &state.bundle);
    let output = inspect_json(&state, spec.as_ref());
    println!(
        "{}",
        edgerun_json::to_string_pretty(&output)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
    );
    Ok(())
}

fn refresh_state(id: &str, state: &mut ContainerState) {
    if let Some(pid) = state.pid {
        if state.status == "running" && !crate::cli::is_process_alive(pid) {
            state.status = "stopped".to_string();
            let _ = save_state(state, id);
        }
    }
}

fn inspect_json(state: &ContainerState, spec: Option<&OciSpec>) -> edgerun_json::JsonValue {
    let pid = state.pid.unwrap_or(0);
    let state_dir = container_state_dir(&state.id);
    let process = spec.and_then(|spec| spec.process.as_ref());
    let root = spec.and_then(|spec| spec.root.as_ref());
    let annotations = state
        .annotations
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|(k, v)| (k, edgerun_json::JsonValue::String(v)))
        .collect::<Vec<_>>();

    edgerun_json::json!({
        "Id": state.id.clone(),
        "State": {
            "Status": state.status.clone(),
            "Running": state.status == "running",
            "Pid": pid,
        },
        "Path": process
            .and_then(|process| process.args.as_ref())
            .and_then(|args| args.first())
            .cloned()
            .unwrap_or_default(),
        "Args": process
            .and_then(|process| process.args.as_ref())
            .map(|args| args.iter().skip(1).cloned().collect::<Vec<_>>())
            .unwrap_or_default(),
        "Config": {
            "Env": process
                .and_then(|process| process.env.clone())
                .unwrap_or_default(),
            "WorkingDir": process
                .and_then(|process| process.cwd.clone())
                .unwrap_or_default(),
            "Tty": process
                .and_then(|process| process.terminal)
                .unwrap_or(false),
            "NoNewPrivileges": process
                .and_then(|process| process.no_new_privileges)
                .unwrap_or(true),
            "Hostname": spec
                .and_then(|spec| spec.hostname.clone())
                .unwrap_or_default(),
            "User": process
                .and_then(|process| process.user.as_ref())
                .map(user_string)
                .unwrap_or_default(),
        },
        "HostConfig": {
            "Privileged": process
                .and_then(|process| process.no_new_privileges)
                .map(|no_new| !no_new)
                .unwrap_or(false),
            "Binds": bind_summaries(spec),
            "Dns": annotation_list(spec, "run.edgerun.io/dns"),
            "ExtraHosts": annotation_list(spec, "run.edgerun.io/add-host"),
        },
        "Mounts": mount_json(spec),
        "GraphDriver": {
            "Name": "edgerun-rootfs",
            "Data": {
                "RootDir": root.map(|root| root.path.clone()).unwrap_or_default(),
                "ReadonlyRootfs": root.and_then(|root| root.readonly).unwrap_or(false),
            }
        },
        "LogPath": state_dir.join("stdout.log").to_string_lossy().to_string(),
        "LogPaths": {
            "Stdout": state_dir.join("stdout.log").to_string_lossy().to_string(),
            "Stderr": state_dir.join("stderr.log").to_string_lossy().to_string(),
        },
        "Bundle": state.bundle.clone(),
        "OciVersion": state.oci_version.clone(),
        "Annotations": edgerun_json::JsonValue::Object(edgerun_json::Map::from(annotations)),
    })
}

fn user_string(user: &crate::json::OciUser) -> String {
    match (user.uid, user.gid) {
        (Some(uid), Some(gid)) => format!("{uid}:{gid}"),
        (Some(uid), None) => uid.to_string(),
        _ => String::new(),
    }
}

fn annotation_list(spec: Option<&OciSpec>, key: &str) -> Vec<String> {
    spec.and_then(|spec| spec.annotations.as_ref())
        .and_then(|annotations| annotations.get(key))
        .map(|value| {
            value
                .split(',')
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn bind_summaries(spec: Option<&OciSpec>) -> Vec<String> {
    spec.and_then(|spec| spec.mounts.as_ref())
        .map(|mounts| {
            mounts
                .iter()
                .filter(|mount| mount.mount_type.as_deref() == Some("bind"))
                .map(|mount| {
                    let mode = if mount
                        .options
                        .as_ref()
                        .is_some_and(|opts| opts.iter().any(|opt| opt == "ro"))
                    {
                        "ro"
                    } else {
                        "rw"
                    };
                    format!(
                        "{}:{}:{}",
                        mount.source.as_deref().unwrap_or(""),
                        mount.destination,
                        mode
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

fn mount_json(spec: Option<&OciSpec>) -> Vec<edgerun_json::JsonValue> {
    spec.and_then(|spec| spec.mounts.as_ref())
        .map(|mounts| {
            mounts
                .iter()
                .map(|mount| {
                    let mode = if mount
                        .options
                        .as_ref()
                        .is_some_and(|opts| opts.iter().any(|opt| opt == "ro"))
                    {
                        "ro"
                    } else {
                        "rw"
                    };
                    edgerun_json::json!({
                        "Type": mount.mount_type.clone().unwrap_or_default(),
                        "Source": mount.source.clone().unwrap_or_default(),
                        "Destination": mount.destination.clone(),
                        "Mode": mode,
                        "RW": mode == "rw",
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}
