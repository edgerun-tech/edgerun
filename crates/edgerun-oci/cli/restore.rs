//! Restore command implementation.
//!
//! Restores a container from a checkpoint using CRIU.

use crate::prelude::*;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::cli::{invalid_input, parse_cli_args, required_positional};
use crate::state::{ContainerState as StateContainerState, load_state, save_state};
use edgerun_clap::{Arg, Command};

pub fn cmd_restore(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    const USAGE: &str =
        "Usage: ert restore --image-path DIR [--work-path DIR] [--bundle DIR] <container-id>";
    let matches = parse_cli_args(
        Command::new("restore")
            .arg(Arg::new("image-path").long("image-path"))
            .arg(Arg::new("work-path").long("work-path"))
            .arg(Arg::new("bundle").long("bundle")),
        args,
        USAGE,
    )?;
    if matches.positional_count() > 1 {
        return Err(invalid_input(USAGE));
    }

    let id = required_positional(&matches, 0, "container ID required")?.to_string();
    crate::cli::validate_container_id(&id)?;
    let image_path = matches
        .get_one::<PathBuf>("image-path")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "--image-path is required"))?;
    let work_path = matches.get_one::<PathBuf>("work-path");
    let bundle_path = matches.get_one::<PathBuf>("bundle");

    let bundle_path = bundle_path.unwrap_or_else(|| PathBuf::from("/var/lib/edgerun/bundle"));
    let work_path = work_path.unwrap_or_else(|| image_path.join("work"));

    if !image_path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--image-path must be an absolute path",
        ));
    }
    if !work_path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--work-path must be an absolute path",
        ));
    }

    if let Err(err) = crate::criu::validate_criu_image_path(&image_path) {
        return Err(err);
    }
    if let Err(err) = crate::criu::validate_criu_image_path(&work_path) {
        return Err(err);
    }
    if !work_path.exists() {
        fs::create_dir_all(&work_path)?;
    }

    let existing_state = load_state(&id).ok();
    if let Some(existing_state) = existing_state.as_ref() {
        if existing_state.status == "running"
            || existing_state.status == "paused"
            || existing_state.status == "created"
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "container {} already has active state ({})",
                    id, existing_state.status
                ),
            ));
        }
    }

    if let Some(pid) = existing_state.and_then(|state| state.pid) {
        if pid != 0 && !crate::cli::is_process_alive(pid) {
            let mut stopped_state = StateContainerState {
                oci_version: "1.0.2".to_string(),
                id: id.clone(),
                status: "stopped".to_string(),
                pid: None,
                bundle: bundle_path.to_string_lossy().to_string(),
                annotations: None,
            };
            save_state(&stopped_state, &id)?;
        }
    }

    let inventory = image_path.join("inventory.img");
    if !inventory.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("checkpoint image not found: {}", inventory.display()),
        ));
    }

    let flags = crate::criu::RestoreFlags::Empty as u32;

    let restore_opts = crate::criu::CriuRestoreOpts {
        img: &image_path,
        flags,
        pid: None,
        status_fd: None,
    };

    let restored_pid = crate::criu::criu_restore(&restore_opts)?;

    let state = StateContainerState {
        oci_version: "1.0.2".to_string(),
        id: id.clone(),
        status: "running".to_string(),
        pid: Some(restored_pid as u32),
        bundle: bundle_path.to_string_lossy().to_string(),
        annotations: None,
    };
    save_state(&state, &id)?;

    eprintln!("Restored container {} (PID: {})", id, restored_pid);
    Ok(())
}
