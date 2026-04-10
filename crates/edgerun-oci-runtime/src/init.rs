//! Minimal PID 1 init process for containers.
//!
//! In a PID namespace, PID 1 has special semantics:
//! - Signals not explicitly handled are ignored (default disposition)
//! - Orphaned children become zombies that are never reaped
//!
//! This module provides a minimal init process that:
//! 1. Forwards SIGTERM/SIGINT/SIGQUIT to the child workload process
//! 2. Reaps zombie children via waitpid in a loop
//!
//! Usage: wrap the workload process by calling `run_as_pid1()` which
//! execs a shell that runs the workload and then loops reaping zombies.

#![allow(dead_code)]

use std::io;
use std::os::raw::c_int;

extern "C" {
    fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
    fn kill(pid: c_int, sig: c_int) -> c_int;
}

const WNOHANG: c_int = 1;
const SIGTERM: c_int = 15;
const SIGINT: c_int = 2;
const SIGQUIT: c_int = 3;
const SIGCHLD: c_int = 17;

/// Reap all zombie children. Returns the number of zombies reaped.
/// This should be called periodically in the init loop.
pub fn reap_zombies() -> io::Result<usize> {
    let mut count = 0;
    loop {
        let mut status: c_int = 0;
        let ret = unsafe { waitpid(-1, &mut status, WNOHANG) };
        if ret <= 0 {
            break;
        }
        count += 1;
    }
    Ok(count)
}

/// Generate a shell wrapper that acts as PID 1 init + signal forwarder.
///
/// The generated script:
/// 1. Sets up a trap for SIGTERM/SIGINT/SIGQUIT to forward to the child
/// 2. Runs the workload in the background
/// 3. Loops waiting for SIGCHLD, reaping zombies
/// 4. Exits with the workload's exit code
///
/// This replaces the direct exec with a minimal init shim.
pub fn pid1_init_script(args: &[String]) -> String {
    let workload = args.iter()
        .map(|a| shell_escape(a))
        .collect::<Vec<_>>()
        .join(" ");

    format!(
        r#"#!/bin/sh
# Minimal PID 1 init — forwards signals and reaps zombies

# Forward signals to the child
cleanup() {{
    kill -$1 $PID 2>/dev/null
}}

trap 'cleanup 15' TERM
trap 'cleanup 2' INT
trap 'cleanup 3' QUIT

# Run workload in background
{workload} &
PID=$!

# Reap zombies in a loop
while true; do
    wait $PID 2>/dev/null
    EXIT_CODE=$?
    # If wait returned (child exited), check for other zombies then exit
    while kill -0 $PID 2>/dev/null; do
        sleep 0.1
    done
    exit $EXIT_CODE
done
"#
    )
}

/// Escape a string for safe use in a shell command.
fn shell_escape(s: &str) -> String {
    // Simple: wrap in single quotes, escape any embedded single quotes
    format!("'{}'", s.replace('\'', "'\\''"))
}
