//! Host process-tree helpers for lifecycle commands.

use crate::libc;
use crate::prelude::*;
use std::io;
use std::os::raw::c_int;
use std::thread;
use std::time::{Duration, Instant};

pub(crate) fn signal(pid: u32, signal: c_int) -> io::Result<()> {
    if unsafe { libc::kill(pid as c_int, signal) } == 0 {
        return Ok(());
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::NotFound {
        Ok(())
    } else {
        Err(error)
    }
}

pub(crate) fn signal_tree(root_pid: u32, signal_number: c_int) {
    let mut pids = descendants(root_pid);
    pids.push(root_pid);
    pids.sort_unstable();
    pids.dedup();
    for pid in pids.into_iter().rev() {
        let _ = signal(pid, signal_number);
    }
}

pub(crate) fn wait_tree_dead(root_pid: u32, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !tree_alive(root_pid) {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    !tree_alive(root_pid)
}

fn tree_alive(root_pid: u32) -> bool {
    if process_alive(root_pid) {
        return true;
    }
    descendants(root_pid).into_iter().any(process_alive)
}

pub(crate) fn process_alive(pid: u32) -> bool {
    if unsafe { libc::kill(pid as c_int, 0) != 0 } {
        return false;
    }
    let status_path = format!("/proc/{pid}/status");
    if let Ok(status) = std::fs::read_to_string(status_path) {
        for line in status.lines() {
            if let Some(rest) = line.strip_prefix("State:") {
                return !rest.trim_start().starts_with('Z');
            }
        }
    }
    true
}

pub(crate) fn descendants(root_pid: u32) -> Vec<u32> {
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
