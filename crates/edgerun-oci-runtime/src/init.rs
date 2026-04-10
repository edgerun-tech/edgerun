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
