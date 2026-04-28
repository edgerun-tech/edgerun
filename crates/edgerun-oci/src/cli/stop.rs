//! Stop command implementation.

use crate::prelude::*;
use std::io;
use std::thread;
use std::time::{Duration, Instant};

use crate::cli::is_process_alive;
use crate::state::{load_state, save_state};

pub fn cmd_stop(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--root path is not valid UTF-8",
            )
        })?);
    }

    let (timeout, id) = parse_stop_args(args)?;
    let mut state = load_state(id)?;
    let Some(pid) = state.pid else {
        state.status = "stopped".to_string();
        save_state(&state, id)?;
        return Ok(());
    };

    if !is_process_alive(pid) {
        state.status = "stopped".to_string();
        save_state(&state, id)?;
        return Ok(());
    }

    signal_tree(pid, libc::SIGTERM);
    let deadline = Instant::now() + Duration::from_secs(timeout);
    while Instant::now() < deadline {
        if !is_process_alive(pid) {
            state.status = "stopped".to_string();
            save_state(&state, id)?;
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    if is_process_alive(pid) {
        signal_tree(pid, libc::SIGKILL);
    }
    state.status = "stopped".to_string();
    save_state(&state, id)
}

fn parse_stop_args(args: &[String]) -> io::Result<(u64, &str)> {
    let mut timeout = 10u64;
    let mut id = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-t" | "--time" if i + 1 < args.len() => {
                timeout = args[i + 1].parse::<u64>().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "stop timeout must be seconds")
                })?;
                i += 2;
            }
            arg if arg.starts_with("--time=") => {
                timeout = arg["--time=".len()..].parse::<u64>().map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "stop timeout must be seconds")
                })?;
                i += 1;
            }
            arg if id.is_none() => {
                id = Some(arg);
                i += 1;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Usage: ert stop [-t seconds] <container-id>",
                ))
            }
        }
    }

    id.map(|id| (timeout, id)).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Usage: ert stop [-t seconds] <container-id>",
        )
    })
}

fn signal(pid: u32, signal: libc::c_int) -> io::Result<()> {
    if unsafe { libc::kill(pid as libc::c_int, signal) } == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::NotFound {
        Ok(())
    } else {
        Err(error)
    }
}

fn signal_tree(root_pid: u32, signal_number: libc::c_int) {
    let mut pids = descendants(root_pid);
    pids.push(root_pid);
    pids.sort_unstable();
    pids.dedup();
    for pid in pids.into_iter().rev() {
        let _ = signal(pid, signal_number);
    }
}

fn descendants(root_pid: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut changed = true;
    while changed {
        changed = false;
        let Ok(entries) = std::fs::read_dir("/proc") else {
            break;
        };
        for entry in entries.flatten() {
            let Ok(name) = entry.file_name().into_string() else {
                continue;
            };
            let Ok(pid) = name.parse::<u32>() else {
                continue;
            };
            if pid == root_pid || out.contains(&pid) {
                continue;
            }
            let status = format!("/proc/{pid}/status");
            let Ok(content) = std::fs::read_to_string(status) else {
                continue;
            };
            let Some(ppid) = parent_pid(&content) else {
                continue;
            };
            if ppid == root_pid || out.contains(&ppid) {
                out.push(pid);
                changed = true;
            }
        }
    }
    out
}

fn parent_pid(status: &str) -> Option<u32> {
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("PPid:") {
            return rest.trim().parse().ok();
        }
    }
    None
}
