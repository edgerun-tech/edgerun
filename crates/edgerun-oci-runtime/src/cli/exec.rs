//! Exec command implementation.
//!
//! Run an additional process inside a running container's namespaces.
//! Uses `nsenter` to join the container's namespaces.

use std::fs;
use std::io;
use std::os::raw::c_int;
use std::process::Command;

use crate::state::load_state;

pub fn cmd_exec(_opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if args.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "container ID required"));
    }

    let id = &args[0];
    let exec_args = &args[1..];

    if exec_args.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "command to execute required"));
    }

    let state = load_state(id)?;
    let pid = state.pid.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "container has no PID")
    })?;

    if !crate::cli::is_process_alive(pid) {
        return Err(io::Error::new(io::ErrorKind::InvalidInput,
            format!("container {} is not running", id)));
    }

    // Read the bundle config to get the working directory and environment
    let bundle = &state.bundle;
    let config_path = std::path::Path::new(bundle).join("config.json");
    let (cwd, env_vars) = if let Ok(data) = fs::read(&config_path) {
        if let Ok(spec) = crate::json::parse_oci_spec(&data) {
            let process = spec.process.unwrap_or_default();
            let cwd = process.cwd.unwrap_or_else(|| "/".into());
            let env = process.env.unwrap_or_else(|| crate::process::DEFAULT_ENV.iter().map(|s| s.to_string()).collect());
            (cwd, env)
        } else {
            ("/".to_string(), Vec::new())
        }
    } else {
        ("/".to_string(), Vec::new())
    };

    // Build nsenter command to join the container's namespaces
    let mut cmd = Command::new("nsenter");

    // Join all namespaces of the target PID
    cmd.arg("--target").arg(pid.to_string());
    cmd.arg("--mount");
    cmd.arg("--uts");
    cmd.arg("--ipc");
    cmd.arg("--net");
    cmd.arg("--pid");

    // Set working directory
    cmd.arg("--wd").arg(&cwd);

    // Set environment
    for e in &env_vars {
        if let Some((k, v)) = e.split_once('=') {
            cmd.env(k, v);
        }
    }

    // Execute the command
    cmd.arg("--").arg(&exec_args[0]).args(&exec_args[1..]);
    cmd.stdin(std::process::Stdio::inherit());
    cmd.stdout(std::process::Stdio::inherit());
    cmd.stderr(std::process::Stdio::inherit());

    let status = cmd.status()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("nsenter failed: {}", e)))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}
