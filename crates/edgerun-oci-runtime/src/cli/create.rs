//! Create command implementation.
//!
//! Uses the library lifecycle to run hooks with full OCI spec compliance.

use std::fs;
use std::io;
use std::path::Path;

use crate::cli::GlobalOpts;
use crate::json::{OciSpec, parse_oci_spec};
use crate::lifecycle::{run_prestart_hooks, run_create_runtime_hooks, fork_container_child, save_created_state};

pub fn cmd_create(opts: &GlobalOpts, args: &[String]) -> io::Result<()> {
    let bundle = opts.bundle.as_deref().unwrap_or(Path::new("."));
    let id = crate::cli::require_container_id(args)?;

    // Check for duplicate ID
    if crate::state::state_file_path(id).exists() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists,
            format!("container ID {} already exists", id)));
    }

    let config_path = bundle.join("config.json");
    let config_data = fs::read(&config_path)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("cannot read config: {}", e)))?;
    let spec: OciSpec = parse_oci_spec(&config_data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("invalid OCI config: {}", e)))?;

    // Step 1: prestart hooks (runtime namespace)
    run_prestart_hooks(&spec, id)?;

    // Step 2: createRuntime hooks (runtime namespace)
    run_create_runtime_hooks(&spec, id)?;

    // Step 3: fork child (runs setup + createContainer + FIFO wait + startContainer in container namespace)
    let forked = fork_container_child(&spec, id)?;
    let child_pid = forked.pid();

    // Drop the ForkedChild handle — the child is running in the background, blocked on FIFO
    // We don't hold the Child handle; start will signal the FIFO
    std::mem::forget(forked);

    // Step 4: save state as "created"
    save_created_state(&spec, id, child_pid)?;

    // Write PID to pid-file if requested
    if let Some(ref pid_file) = opts.pid_file {
        fs::write(pid_file, format!("{}", child_pid))?;
    }

    Ok(())
}
