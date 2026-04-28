//! Ps command implementation — lists processes in the container.

use crate::prelude::*;
use std::fs;
use std::io;
use std::io::Write;
use std::os::unix::io::AsRawFd;

use crate::cli::exec::{enter_container_root, join_container_namespaces, open_exec_root};
use crate::cli::json_string;
use crate::cli::process_tree;
use crate::state::{load_state, save_state, state_root_dir, ContainerState};

pub fn cmd_ps(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    if container_id_arg(args).is_none() {
        return list_containers(args);
    }

    let id = container_id_arg(args)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "container ID required"))?;

    let state = load_state(id)?;
    let pid = state
        .pid
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "container has no PID"))?;

    if !crate::cli::is_process_alive(pid) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("container {id} is not running"),
        ));
    }

    let spec = crate::cli::load_runtime_or_bundle_spec(&state.id, &state.bundle);
    let (root_fd, root_path) = open_exec_root(pid, spec.as_ref())?;
    let json = args
        .iter()
        .any(|arg| arg == "--format=json" || arg == "json");
    if !root_path.join("proc/self").exists() {
        let processes = read_host_process_tree(pid);
        if json {
            print_processes_json(&processes);
        } else {
            print_processes_table(&processes);
        }
        return Ok(());
    }
    print_container_processes(pid, root_fd.as_raw_fd(), json)
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
            arg if arg.starts_with("--format=") => {}
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
            let Ok(mut state) = crate::state::load_state_from_str(&data) else {
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

fn print_container_processes(init_pid: u32, root_fd: i32, json: bool) -> io::Result<()> {
    let child = unsafe { libc::fork() };
    if child < 0 {
        return Err(io::Error::last_os_error());
    }
    if child == 0 {
        let needs_pid_fork = match join_container_namespaces(init_pid) {
            Ok(needs_pid_fork) => needs_pid_fork,
            Err(_) => unsafe { libc::_exit(126) },
        };
        if needs_pid_fork {
            let inner = unsafe { libc::fork() };
            if inner < 0 {
                unsafe { libc::_exit(126) };
            }
            if inner > 0 {
                unsafe { libc::_exit(0) };
            }
        }
        if enter_container_root(root_fd).is_err() {
            unsafe { libc::_exit(126) };
        }
        let processes = read_proc_processes();
        if json {
            print_processes_json(&processes);
        } else {
            print_processes_table(&processes);
        }
        let _ = io::stdout().flush();
        unsafe { libc::_exit(0) };
    }

    let mut status = 0i32;
    unsafe { libc::waitpid(child, &mut status, 0) };
    if libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0 {
        Ok(())
    } else {
        Err(io::Error::other("failed to list container processes"))
    }
}

#[derive(Debug)]
struct ProcEntry {
    pid: u32,
    ppid: u32,
    state: String,
    name: String,
}

fn read_proc_processes() -> Vec<ProcEntry> {
    let mut processes = Vec::new();
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            let Ok(pid) = name.parse::<u32>() else {
                continue;
            };
            let status_path = format!("/proc/{pid}/status");
            if let Ok(content) = fs::read_to_string(status_path) {
                processes.push(parse_proc_status(pid, &content));
            }
        }
    }
    processes.sort_by_key(|process| process.pid);
    processes
}

fn read_host_process_tree(init_pid: u32) -> Vec<ProcEntry> {
    let mut pids = process_tree::descendants(init_pid);
    pids.push(init_pid);
    pids.sort_unstable();
    pids.dedup();
    let mut processes = Vec::new();
    for pid in pids {
        let status_path = format!("/proc/{pid}/status");
        if let Ok(content) = fs::read_to_string(status_path) {
            processes.push(parse_proc_status(pid, &content));
        }
    }
    processes.sort_by_key(|process| process.pid);
    processes
}

fn parse_proc_status(pid: u32, content: &str) -> ProcEntry {
    let mut name = String::new();
    let mut state = "?".to_string();
    let mut ppid = 0u32;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("Name:") {
            name = rest.trim().to_string();
        }
        if line.starts_with("State:") {
            if let Some(val) = line.split_whitespace().nth(1) {
                state = val.to_string();
            }
        }
        if line.starts_with("PPid:") {
            if let Some(val) = line.split_whitespace().nth(1) {
                ppid = val.parse().unwrap_or(0);
            }
        }
    }
    ProcEntry {
        pid,
        ppid,
        state,
        name,
    }
}

fn print_processes_table(processes: &[ProcEntry]) {
    println!("{:<10} {:<10} {:<10} COMMAND", "PID", "PPID", "STATE");
    println!("{:-<48}", "");
    for process in processes {
        println!(
            "{:<10} {:<10} {:<10} {}",
            process.pid, process.ppid, process.state, process.name
        );
    }
}

fn print_processes_json(processes: &[ProcEntry]) {
    let mut entries = Vec::new();
    for process in processes {
        entries.push(format!(
            "{{\"pid\":{},\"ppid\":{},\"state\":{},\"command\":{}}}",
            process.pid,
            process.ppid,
            json_string(&process.state),
            json_string(&process.name)
        ));
    }
    println!("[{}]", entries.join(","));
}
