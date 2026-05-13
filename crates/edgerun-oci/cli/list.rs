//! List command implementation.

use crate::prelude::*;
use std::fs;
use std::io;

use crate::clap::cli::Action;
use crate::clap::{Arg, Command};
use crate::cli::{invalid_input, parse_cli_args};

enum ListFormat {
    Table,
    Json,
    Quiet,
}

struct ListedContainer {
    oci_version: String,
    id: String,
    pid: Option<u32>,
    status: String,
    bundle: String,
    rootfs: String,
    created: u64,
}

pub fn cmd_list(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;
    let format = parse_list_args(args)?;
    let containers = load_containers()?;

    match format {
        ListFormat::Quiet => {
            for container in containers {
                println!("{}", container.id);
            }
        }
        ListFormat::Table => {
            println!(
                "{:<20} {:<10} {:<10} {:<40} CREATED",
                "ID", "PID", "STATUS", "BUNDLE"
            );
            for container in containers {
                println!(
                    "{:<20} {:<10} {:<10} {:<40} {}",
                    container.id,
                    container
                        .pid
                        .map(|pid| pid.to_string())
                        .unwrap_or_else(|| "-".to_string()),
                    container.status,
                    container.bundle,
                    container.created
                );
            }
        }
        ListFormat::Json => {
            print!("[");
            for (index, container) in containers.iter().enumerate() {
                if index > 0 {
                    print!(",");
                }
                print!(
                    "{{\"ociVersion\":\"{}\",\"id\":\"{}\",\"pid\":{},\"status\":\"{}\",\"bundle\":\"{}\",\"rootfs\":\"{}\",\"created\":{}}}",
                    json_escape(&container.oci_version),
                    json_escape(&container.id),
                    container
                        .pid
                        .map(|pid| pid.to_string())
                        .unwrap_or_else(|| "0".to_string()),
                    json_escape(&container.status),
                    json_escape(&container.bundle),
                    json_escape(&container.rootfs),
                    container.created
                );
            }
            println!("]");
        }
    }

    Ok(())
}

fn parse_list_args(args: &[String]) -> io::Result<ListFormat> {
    const USAGE: &str = "Usage: ert list [-q|--format table|json]";
    let matches = parse_cli_args(
        Command::new("list")
            .arg(Arg::new("quiet").short('q').action(Action::StoreTrue))
            .arg(Arg::new("format").long("format")),
        args,
        USAGE,
    )?;
    if matches.positional_count() > 0 {
        return Err(invalid_input(USAGE));
    }
    if matches.get_flag("quiet") {
        return Ok(ListFormat::Quiet);
    }
    match matches.get_one::<String>("format").as_deref() {
        None | Some("table") => Ok(ListFormat::Table),
        Some("json") => Ok(ListFormat::Json),
        Some(other) => Err(invalid_input(format!(
            "{USAGE}: unsupported format {other}"
        ))),
    }
}

fn load_containers() -> io::Result<Vec<ListedContainer>> {
    let root = crate::state::state_root_dir();
    let mut containers = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return Ok(containers);
    };

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().to_string();
        let Ok(state) = crate::state::load_state(&id) else {
            continue;
        };
        let created = entry
            .metadata()
            .and_then(|metadata| metadata.created().or_else(|_| metadata.modified()))
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let rootfs = crate::cli::load_runtime_or_bundle_spec(&state.id, &state.bundle)
            .and_then(|spec| spec.root.map(|root| root.path))
            .unwrap_or_default();
        containers.push(ListedContainer {
            oci_version: state.oci_version,
            id: state.id,
            pid: state.pid,
            status: state.status,
            bundle: state.bundle,
            rootfs,
            created,
        });
    }

    containers.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(containers)
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}
