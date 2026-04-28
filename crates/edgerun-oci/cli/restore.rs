//! Restore command implementation.
//!
//! Restores a container from a checkpoint using CRIU.

use crate::prelude::*;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::cli::{invalid_input, parse_cli_args, required_positional};
use crate::state::{save_state, ContainerState as StateContainerState};
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
    let image_path = matches
        .get_one::<PathBuf>("image-path")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "--image-path is required"))?;
    let bundle_path = matches.get_one::<PathBuf>("bundle");

    let bundle_path = bundle_path.unwrap_or_else(|| PathBuf::from("/var/lib/edgerun/bundle"));

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
