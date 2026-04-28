//! Stop command implementation.

use crate::prelude::*;
use std::io;
use std::thread;
use std::time::{Duration, Instant};

use crate::cli::is_process_alive;
use crate::cli::process_tree::{signal_tree, wait_tree_dead};
use crate::state::{load_state, save_state};

pub fn cmd_stop(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

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
        let _ = wait_tree_dead(pid, Duration::from_secs(2));
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
