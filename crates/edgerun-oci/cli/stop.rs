//! Stop command implementation.

use crate::libc;
use crate::prelude::*;
use std::io;
use std::thread;
use std::time::{Duration, Instant};

use crate::clap::{Arg, Command};
use crate::cli::process_tree::{signal_tree, wait_tree_dead};
use crate::cli::{invalid_input, is_process_alive, parse_cli_args, required_positional};
use crate::state::{load_state, save_state};

pub fn cmd_stop(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    let (timeout, id) = parse_stop_args(args)?;
    let mut state = load_state(&id)?;
    let Some(pid) = state.pid else {
        state.status = "stopped".to_string();
        save_state(&state, &id)?;
        return Ok(());
    };

    if !is_process_alive(pid) {
        state.status = "stopped".to_string();
        save_state(&state, &id)?;
        return Ok(());
    }

    signal_tree(pid, libc::SIGTERM);
    let deadline = Instant::now() + Duration::from_secs(timeout);
    while Instant::now() < deadline {
        if !is_process_alive(pid) {
            state.status = "stopped".to_string();
            save_state(&state, &id)?;
            return Ok(());
        }
        thread::sleep(Duration::from_millis(100));
    }

    if is_process_alive(pid) {
        signal_tree(pid, libc::SIGKILL);
        if !wait_tree_dead(pid, Duration::from_secs(2)) {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("container {id} process tree did not exit after SIGKILL"),
            ));
        }
    }
    state.status = "stopped".to_string();
    save_state(&state, &id)
}

fn parse_stop_args(args: &[String]) -> io::Result<(u64, String)> {
    const USAGE: &str = "Usage: ert stop [-t seconds] <container-id>";
    let matches = parse_cli_args(
        Command::new("stop").arg(Arg::new("time").short('t').long("time")),
        args,
        USAGE,
    )?;
    if matches.positional_count() > 1 {
        return Err(invalid_input(USAGE));
    }
    let timeout = matches
        .get_one::<String>("time")
        .map(|value| parse_timeout(&value))
        .transpose()?
        .unwrap_or(10);
    let id = required_positional(&matches, 0, USAGE)?.to_string();
    crate::cli::validate_container_id(&id)?;
    Ok((timeout, id))
}

fn parse_timeout(value: &str) -> io::Result<u64> {
    value
        .parse::<u64>()
        .map_err(|_| invalid_input("stop timeout must be seconds"))
}
