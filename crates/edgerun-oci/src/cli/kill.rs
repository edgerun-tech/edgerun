//! Kill command implementation.
//!
//! Sends a signal to a container process.
//! For TERM/INT/QUIT on running containers, signal goes to PID 1 init
//! which forwards to the child workload. Other signals go directly to PID.
//! No-op for stopped containers (OCI kill_no_effect behavior).

use crate::prelude::*;
use std::io;
use std::os::raw::c_int;

use crate::cli::{is_process_alive, parse_kill_args};
use crate::state::load_state;

pub fn cmd_kill(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--root path is not valid UTF-8",
            )
        })?);
    }

    let (sig_str, id) = parse_kill_args(args);
    let id = if id.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "container ID required",
        ));
    } else {
        id
    };
    let sig_str = sig_str.unwrap_or("TERM");

    let state = load_state(id)?;
    let pid = state
        .pid
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "container has no PID"))?;

    // No-op for stopped/created containers — OCI spec: kill should be safe on non-running
    if !is_process_alive(pid) {
        return Ok(());
    }

    let sig = parse_signal(sig_str)?;
    let ret = unsafe { libc::kill(pid as c_int, sig) };
    if ret != 0 {
        // ESRCH = process doesn't exist — not an error for kill
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::NotFound {
            return Ok(());
        }
        return Err(err);
    }
    Ok(())
}

fn parse_signal(s: &str) -> io::Result<c_int> {
    // Strip optional "SIG" prefix
    let name = s.strip_prefix("SIG").unwrap_or(s);
    if let Ok(n) = s.parse::<c_int>() {
        return Ok(n);
    }
    match name {
        "HUP" | "SIGHUP" => Ok(1),
        "INT" | "SIGINT" => Ok(2),
        "QUIT" | "SIGQUIT" => Ok(3),
        "KILL" | "SIGKILL" => Ok(9),
        "TERM" | "SIGTERM" => Ok(15),
        "CONT" | "SIGCONT" => Ok(18),
        "STOP" | "SIGSTOP" => Ok(19),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown signal: {}", s),
        )),
    }
}
