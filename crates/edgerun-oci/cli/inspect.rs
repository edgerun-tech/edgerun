//! Inspect command implementation.

use crate::prelude::*;
use std::io;

use crate::cli::json;
use crate::spec::OciSpec;
use crate::state::{ContainerState, container_state_dir, load_state, save_state};

pub fn cmd_inspect(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    let id = crate::cli::parse_container_id_args(args, "inspect")?;
    let id = id.as_str();
    let mut state = load_state(id)?;
    refresh_state(id, &mut state);
    let spec = crate::cli::load_runtime_or_bundle_spec(&state.id, &state.bundle);
    println!("{}", inspect_json(&state, spec.as_ref()));
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

fn inspect_json(state: &ContainerState, spec: Option<&OciSpec>) -> String {
    let pid = state.pid.unwrap_or(0);
    let state_dir = container_state_dir(&state.id);
    let process = spec.and_then(|spec| spec.process.as_ref());
    let root = spec.and_then(|spec| spec.root.as_ref());
    let args = process
        .and_then(|process| process.args.as_ref())
        .cloned()
        .unwrap_or_default();
    let path = args.first().cloned().unwrap_or_default();
    let rest_args = args.into_iter().skip(1).collect::<Vec<_>>();
    let env = process
        .and_then(|process| process.env.clone())
        .unwrap_or_default();
    let binds = bind_summaries(spec);
    let dns = annotation_list(spec, "run.edgerun.io/dns");
    let extra_hosts = annotation_list(spec, "run.edgerun.io/add-host");
    let log_stdout = state_dir.join("stdout.log").to_string_lossy().to_string();
    let log_stderr = state_dir.join("stderr.log").to_string_lossy().to_string();

    let mut out = String::new();
    let mut first = true;
    out.push('{');
    json::push_string_field(&mut out, &mut first, "Id", &state.id);
    json::push_field_prefix(&mut out, &mut first, "State");
    out.push('{');
    let mut state_first = true;
    json::push_string_field(&mut out, &mut state_first, "Status", &state.status);
    json::push_bool_field(
        &mut out,
        &mut state_first,
        "Running",
        state.status == "running",
    );
    json::push_u64_field(&mut out, &mut state_first, "Pid", u64::from(pid));
    out.push('}');
    json::push_string_field(&mut out, &mut first, "Path", &path);
    json::push_field_prefix(&mut out, &mut first, "Args");
    json::push_string_array(&mut out, &rest_args);
    json::push_field_prefix(&mut out, &mut first, "Config");
    out.push('{');
    let mut config_first = true;
    json::push_field_prefix(&mut out, &mut config_first, "Env");
    json::push_string_array(&mut out, &env);
    json::push_string_field(
        &mut out,
        &mut config_first,
        "WorkingDir",
        &process
            .and_then(|process| process.cwd.clone())
            .unwrap_or_default(),
    );
    json::push_bool_field(
        &mut out,
        &mut config_first,
        "Tty",
        process
            .and_then(|process| process.terminal)
            .unwrap_or(false),
    );
    json::push_bool_field(
        &mut out,
        &mut config_first,
        "NoNewPrivileges",
        process
            .and_then(|process| process.no_new_privileges)
            .unwrap_or(true),
    );
    json::push_string_field(
        &mut out,
        &mut config_first,
        "Hostname",
        &spec
            .and_then(|spec| spec.hostname.clone())
            .unwrap_or_default(),
    );
    json::push_string_field(
        &mut out,
        &mut config_first,
        "User",
        &process
            .and_then(|process| process.user.as_ref())
            .map(user_string)
            .unwrap_or_default(),
    );
    out.push('}');
    json::push_field_prefix(&mut out, &mut first, "HostConfig");
    out.push('{');
    let mut host_first = true;
    json::push_bool_field(
        &mut out,
        &mut host_first,
        "Privileged",
        process
            .and_then(|process| process.no_new_privileges)
            .map(|no_new| !no_new)
            .unwrap_or(false),
    );
    json::push_field_prefix(&mut out, &mut host_first, "Binds");
    json::push_string_array(&mut out, &binds);
    json::push_field_prefix(&mut out, &mut host_first, "Dns");
    json::push_string_array(&mut out, &dns);
    json::push_field_prefix(&mut out, &mut host_first, "ExtraHosts");
    json::push_string_array(&mut out, &extra_hosts);
    out.push('}');
    json::push_field_prefix(&mut out, &mut first, "Mounts");
    write_mounts_json(&mut out, spec);
    json::push_field_prefix(&mut out, &mut first, "GraphDriver");
    out.push_str("{\"Name\":\"edgerun-rootfs\",\"Data\":{");
    let mut graph_first = true;
    json::push_string_field(
        &mut out,
        &mut graph_first,
        "RootDir",
        &root.map(|root| root.path.clone()).unwrap_or_default(),
    );
    json::push_bool_field(
        &mut out,
        &mut graph_first,
        "ReadonlyRootfs",
        root.and_then(|root| root.readonly).unwrap_or(false),
    );
    out.push_str("}}");
    json::push_string_field(&mut out, &mut first, "LogPath", &log_stdout);
    json::push_field_prefix(&mut out, &mut first, "LogPaths");
    out.push('{');
    let mut log_first = true;
    json::push_string_field(&mut out, &mut log_first, "Stdout", &log_stdout);
    json::push_string_field(&mut out, &mut log_first, "Stderr", &log_stderr);
    out.push('}');
    json::push_string_field(&mut out, &mut first, "Bundle", &state.bundle);
    json::push_string_field(&mut out, &mut first, "OciVersion", &state.oci_version);
    json::push_field_prefix(&mut out, &mut first, "Annotations");
    out.push('{');
    if let Some(annotations) = &state.annotations {
        for (index, (key, value)) in annotations.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            json::write_string(&mut out, key);
            out.push(':');
            json::write_string(&mut out, value);
        }
    }
    out.push_str("}}");
    out
}

fn user_string(user: &crate::spec::OciUser) -> String {
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

fn write_mounts_json(out: &mut String, spec: Option<&OciSpec>) {
    out.push('[');
    if let Some(mounts) = spec.and_then(|spec| spec.mounts.as_ref()) {
        for (index, mount) in mounts.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            let mode = if mount
                .options
                .as_ref()
                .is_some_and(|opts| opts.iter().any(|opt| opt == "ro"))
            {
                "ro"
            } else {
                "rw"
            };
            let mut first = true;
            out.push('{');
            json::push_string_field(
                out,
                &mut first,
                "Type",
                &mount.mount_type.clone().unwrap_or_default(),
            );
            json::push_string_field(
                out,
                &mut first,
                "Source",
                &mount.source.clone().unwrap_or_default(),
            );
            json::push_string_field(out, &mut first, "Destination", &mount.destination);
            json::push_string_field(out, &mut first, "Mode", mode);
            json::push_bool_field(out, &mut first, "RW", mode == "rw");
            out.push('}');
        }
    }
    out.push(']');
}
