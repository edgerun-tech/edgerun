//! Create command implementation.
//!
//! Sets up a container in the 'created' state, ready to be started.
//! Delegates to the library `create_container_from_spec()` for full hook support.

use std::fs;
use std::io;
use std::path::Path;

use crate::cli::GlobalOpts;
use crate::json::{OciSpec, parse_oci_spec};
use crate::lifecycle::create_container_from_spec;
use crate::state::{ContainerState, save_state, fifo_path, container_state_dir};

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

    // Create the FIFO for start synchronization (library doesn't handle this)
    let dir = container_state_dir(id);
    fs::create_dir_all(&dir)?;
    let fifo = fifo_path(id);
    let _ = fs::remove_file(&fifo);

    // Use mkfifo — this is the only external dependency in the runtime
    let mkfifo_output = std::process::Command::new("mkfifo").arg(&fifo).output()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("mkfifo failed: {}", e)))?;
    if !mkfifo_output.status.success() {
        return Err(io::Error::new(io::ErrorKind::Other,
            format!("mkfifo failed: {}", String::from_utf8_lossy(&mkfifo_output.stderr))));
    }

    // Use library — runs prestart + createRuntime + createContainer hooks
    let created = create_container_from_spec(&spec, id)?;
    let child_pid = created.pid();

    // Drop the CreatedContainer (it holds the Child handle)
    // The child is already running in the background
    std::mem::forget(created);

    // Write PID to pid-file if requested
    if let Some(ref pid_file) = opts.pid_file {
        fs::write(pid_file, format!("{}", child_pid))?;
    }

    // Save state as 'created'
    let state = ContainerState {
        oci_version: spec.version.clone(),
        id: id.to_string(),
        status: "created".to_string(),
        pid: Some(child_pid),
        bundle: bundle.to_string_lossy().to_string(),
        annotations: spec.annotations.clone(),
    };
    save_state(&state, id)?;
    Ok(())
}
