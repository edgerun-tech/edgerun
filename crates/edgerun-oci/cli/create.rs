//! Create command implementation.
//!
//! Uses the library lifecycle to run hooks with full OCI spec compliance.

use crate::cli::process_tree::{signal_tree, wait_tree_dead};
use crate::cli::{parse_cli_args, required_positional, split_cli_prefix, GlobalOpts};
use crate::libc;
use crate::prelude::*;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::lifecycle::{
    fork_container_child, run_create_runtime_hooks, run_prestart_hooks, save_created_state,
};
use crate::process::validate_spec;
use crate::spec::{parse_oci_spec, OciSpec};
use crate::state::delete_state_with_result;

pub fn cmd_create(opts: &GlobalOpts, args: &[String]) -> io::Result<()> {
    let (id, bundle_override, pid_file_override) = parse_create_args(args)?;
    let id = id.as_str();
    let bundle = bundle_override
        .as_ref()
        .or_else(|| opts.bundle.as_ref())
        .map(PathBuf::as_path)
        .unwrap_or_else(|| Path::new("."));
    let pid_file = pid_file_override.or_else(|| opts.pid_file.clone());

    crate::cli::apply_global_opts(opts)?;

    // Chdir to bundle so relative root.path resolves correctly
    std::env::set_current_dir(bundle).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("cannot chdir to bundle: {}", e),
        )
    })?;

    // Check for duplicate ID
    if crate::state::state_file_path(id).exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("container ID {} already exists", id),
        ));
    }

    let config_path = bundle.join("config.json");
    let config_data = fs::read(&config_path).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("cannot read config: {}", e),
        )
    })?;
    let spec: OciSpec = parse_oci_spec(&config_data).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid OCI config: {}", e),
        )
    })?;

    // Validate platform compatibility
    if let Some(ref platform) = spec.platform {
        if !platform.matches_host() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "platform mismatch: spec targets {}/{} but host is {}/{}",
                    platform.os.as_deref().unwrap_or("unknown"),
                    platform.arch.as_deref().unwrap_or("unknown"),
                    crate::host_os(),
                    crate::host_arch()
                ),
            ));
        }
    }

    // Validate spec properties (reject invalid values early)
    validate_spec(&spec)?;

    // Step 1: prestart hooks (runtime namespace)
    run_prestart_hooks(&spec, id)?;

    // Step 2: createRuntime hooks (runtime namespace)
    run_create_runtime_hooks(&spec, id)?;

    let bundle_abs = bundle.canonicalize().map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("cannot resolve bundle path: {}", e),
        )
    })?;

    // Step 3: fork child (runs setup + createContainer + FIFO wait + startContainer in container namespace)
    let forked = fork_container_child(&spec, id)?;
    let child_pid = forked.pid();

    if let Err(error) = save_created_state(
        &spec,
        id,
        child_pid,
        bundle_abs.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "bundle path is not valid UTF-8",
            )
        })?,
    ) {
        signal_tree(child_pid, libc::SIGKILL);
        let _ = wait_tree_dead(child_pid, Duration::from_secs(2));
        let _ = delete_state_with_result(id);
        return Err(error);
    }

    // Drop the ForkedChild handle — the child is running in the background, blocked on FIFO
    // We don't hold the Child handle; start will signal the FIFO
    std::mem::forget(forked);

    // Step 4: state is persisted as "created" and runtime spec snapshot is recorded.

    // Write PID to pid-file if requested
    if let Some(ref pid_file) = pid_file {
        fs::write(pid_file, format!("{}", child_pid))?;
    }

    Ok(())
}

fn parse_create_args(args: &[String]) -> io::Result<(String, Option<PathBuf>, Option<PathBuf>)> {
    const USAGE: &str = "Usage: ert create [options] <container-id>";
    let (prefix, id, remaining) =
        split_cli_prefix(args, &["--pid-file", "--bundle", "--console-socket", "-b"]);
    let matches = parse_cli_args(
        edgerun_clap::Command::new("create")
            .arg(edgerun_clap::Arg::new("pid-file").long("pid-file"))
            .arg(edgerun_clap::Arg::new("bundle").short('b').long("bundle"))
            .arg(edgerun_clap::Arg::new("console-socket").long("console-socket")),
        &prefix,
        USAGE,
    )?;
    let id = required_positional(&matches, 0, "container ID required")?;
    if !remaining.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "unexpected additional arguments after container id",
        ));
    }
    crate::cli::validate_container_id(&id)?;

    Ok((
        id.to_string(),
        matches.get_one::<PathBuf>("bundle"),
        matches.get_one::<PathBuf>("pid-file"),
    ))
}
