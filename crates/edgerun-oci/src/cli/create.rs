//! Create command implementation.
//!
//! Uses the library lifecycle to run hooks with full OCI spec compliance.

use std::fs;
use std::io;
use std::path::Path;

use crate::cli::GlobalOpts;
use crate::json::{OciSpec, parse_oci_spec};
use crate::lifecycle::{run_prestart_hooks, run_create_runtime_hooks, fork_container_child, save_created_state};
use crate::process::validate_spec;

fn host_os() -> &'static str {
    if cfg!(target_os = "linux") { "linux" }
    else { "unknown" }
}

fn host_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") { "amd64" }
    else if cfg!(target_arch = "aarch64") { "arm64" }
    else { "unknown" }
}

pub fn cmd_create(opts: &GlobalOpts, args: &[String]) -> io::Result<()> {
    let bundle = opts.bundle.as_deref().unwrap_or(Path::new("."));
    let id = crate::cli::require_container_id(args)?;

    // Set state directory if --root is provided
    if let Some(ref root) = opts.root {
        crate::state::set_state_dir(root.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "--root path is not valid UTF-8")
        })?);
    }

    // Chdir to bundle so relative root.path resolves correctly
    std::env::set_current_dir(bundle).map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("cannot chdir to bundle: {}", e))
    })?;

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

    // Validate platform compatibility
    if let Some(ref platform) = spec.platform {
        if !platform.matches_host() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput,
                format!("platform mismatch: spec targets {}/{} but host is {}/{}",
                    platform.os.as_deref().unwrap_or("unknown"),
                    platform.arch.as_deref().unwrap_or("unknown"),
                    host_os(), host_arch())));
        }
    }

    // Validate spec properties (reject invalid values early)
    validate_spec(&spec)?;

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
    let bundle_abs = bundle.canonicalize().map_err(|e| {
        io::Error::new(io::ErrorKind::InvalidInput, format!("cannot resolve bundle path: {}", e))
    })?;
    save_created_state(&spec, id, child_pid, bundle_abs.to_str().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "bundle path is not valid UTF-8")
    })?)?;

    // Write PID to pid-file if requested
    if let Some(ref pid_file) = opts.pid_file {
        fs::write(pid_file, format!("{}", child_pid))?;
    }

    Ok(())
}
