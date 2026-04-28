//! Ps command implementation — lists processes in the container.

use crate::prelude::*;
use std::fs;
use std::io;

use crate::state::{load_state, save_state, state_root_dir, ContainerState};

pub fn cmd_ps(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--root path is not valid UTF-8",
            )
        })?);
    }

    if container_id_arg(args).is_none() {
        return list_containers(args);
    }

    let id = crate::cli::require_container_id(args)?;

    let state = load_state(id)?;
    let pid = state
        .pid
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "container has no PID"))?;

    // Find all processes in the container's PID namespace
    // We do this by scanning /proc and checking if the NSpid field contains
    // a value that matches our container's init PID namespace
    let mut pids: Vec<u32> = Vec::new();
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if let Ok(pid_str) = name.clone().into_string() {
                if let Ok(n) = pid_str.parse::<u32>() {
                    if n > 0 {
                        // Check NSpid to find PID namespace ID
                        let status_path = format!("/proc/{}/status", n);
                        if let Ok(content) = fs::read_to_string(&status_path) {
                            for line in content.lines() {
                                if line.starts_with("NSpid:") {
                                    let parts: Vec<&str> = line.split_whitespace().collect();
                                    // NSpid format: "NSpid: <host-pid> <ns-pid> ..."
                                    if parts.len() >= 3 {
                                        // The last PID in NSpid is the deepest namespace PID
                                        if let Ok(ns_pid) = parts[parts.len() - 1].parse::<u32>() {
                                            if !pids.contains(&ns_pid) {
                                                pids.push(ns_pid);
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Filter: only include PIDs that are descendants of the container init PID
    let container_pids = find_descendants(pid);

    // Check if --format json
    let is_json = args.iter().any(|a| a == "--format" || a == "-f");

    // Use container PID namespace PIDs
    let display_pids = if container_pids.is_empty() {
        // Fallback: show all PIDs found
        pids.sort_unstable();
        pids
    } else {
        container_pids
    };

    if is_json {
        print_ps_json(&display_pids);
    } else {
        print_ps_table(&display_pids);
    }

    Ok(())
}

fn container_id_arg(args: &[String]) -> Option<&str> {
    let mut skip_next = false;
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        match arg.as_str() {
            "--all" | "-a" => {}
            "--format" | "-f" => skip_next = true,
            _ if !arg.starts_with('-') => return Some(arg.as_str()),
            _ => {}
        }
    }
    None
}

fn list_containers(args: &[String]) -> io::Result<()> {
    let all = args.iter().any(|arg| arg == "-a" || arg == "--all");
    let json = args
        .iter()
        .any(|arg| arg == "--format=json" || arg == "json");
    let mut states = Vec::new();
    let root = state_root_dir();
    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path().join("state.json");
            let Ok(data) = fs::read_to_string(path) else {
                continue;
            };
            let Ok(mut state) = edgerun_json::from_str::<ContainerState>(&data) else {
                continue;
            };
            if let Some(pid) = state.pid {
                if state.status == "running" && !crate::cli::is_process_alive(pid) {
                    state.status = "stopped".to_string();
                    let _ = save_state(&state, &state.id);
                }
            }
            if all || state.status == "running" {
                states.push(state);
            }
        }
    }
    states.sort_by(|a, b| a.id.cmp(&b.id));

    if json {
        print_containers_json(&states);
    } else {
        print_containers_table(&states);
    }
    Ok(())
}

fn print_containers_table(states: &[ContainerState]) {
    println!(
        "{:<28} {:<10} {:<10} BUNDLE",
        "CONTAINER ID", "STATUS", "PID"
    );
    println!("{:-<80}", "");
    for state in states {
        let pid = state
            .pid
            .map(|pid| pid.to_string())
            .unwrap_or_else(|| "-".to_string());
        println!(
            "{:<28} {:<10} {:<10} {}",
            state.id, state.status, pid, state.bundle
        );
    }
}

fn print_containers_json(states: &[ContainerState]) {
    let mut entries = Vec::new();
    for state in states {
        let pid = state
            .pid
            .map(|pid| pid.to_string())
            .unwrap_or_else(|| "null".to_string());
        entries.push(format!(
            "{{\"id\":{},\"status\":{},\"pid\":{},\"bundle\":{}}}",
            json_string(&state.id),
            json_string(&state.status),
            pid,
            json_string(&state.bundle)
        ));
    }
    println!("[{}]", entries.join(","));
}

fn json_string(value: &str) -> String {
    edgerun_json::to_string(value).unwrap_or_else(|_| "\"\"".to_string())
}

/// Find all PIDs that are descendants of the given init PID.
fn find_descendants(init_pid: u32) -> Vec<u32> {
    let mut result = Vec::new();
    result.push(init_pid);

    // Scan /proc for children of init_pid
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if let Ok(pid_str) = name.into_string() {
                if let Ok(n) = pid_str.parse::<u32>() {
                    if n > 0 {
                        let status_path = format!("/proc/{}/status", n);
                        if let Ok(content) = fs::read_to_string(&status_path) {
                            for line in content.lines() {
                                if line.starts_with("PPid:") {
                                    if let Some(ppid_str) = line.split_whitespace().nth(1) {
                                        if let Ok(ppid) = ppid_str.parse::<u32>() {
                                            if (ppid == init_pid || result.contains(&ppid))
                                                && !result.contains(&n)
                                            {
                                                result.push(n);
                                            }
                                        }
                                    }
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    result.sort_unstable();
    result
}

fn print_ps_table(pids: &[u32]) {
    println!("{:<10} {:<10} {:<10}", "PID", "PPID", "STATE");
    println!("{:-<32}", "");
    for &pid in pids {
        let status_path = format!("/proc/{}/status", pid);
        let (state, ppid) = if let Ok(content) = fs::read_to_string(&status_path) {
            let mut s = "?".to_string();
            let mut p = "0".to_string();
            for line in content.lines() {
                if line.starts_with("State:") {
                    if let Some(val) = line.split_whitespace().nth(1) {
                        s = val.to_string();
                    }
                }
                if line.starts_with("PPid:") {
                    if let Some(val) = line.split_whitespace().nth(1) {
                        p = val.to_string();
                    }
                }
            }
            (s, p)
        } else {
            ("?".into(), "0".into())
        };
        println!("{:<10} {:<10} {:<10}", pid, ppid, state);
    }
}

fn print_ps_json(pids: &[u32]) {
    let mut entries: Vec<String> = Vec::new();
    for &pid in pids {
        entries.push(format!("{{\"pid\":{}}}", pid));
    }
    println!("[{}]", entries.join(", "));
}
