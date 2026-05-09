//! Checkpoint command implementation.
//!
//! Checkpoints a running container using CRIU.

use crate::prelude::*;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::cli::{invalid_input, parse_cli_args, required_positional};
use crate::state::{load_state, save_state};
use edgerun_clap::cli::Action;
use edgerun_clap::{Arg, Command};

pub fn cmd_checkpoint(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    const USAGE: &str = "Usage: ert checkpoint [--image-path DIR] [--work-path DIR] [--leave-running] [--pre-dump] <container-id>";
    let matches = parse_cli_args(
        Command::new("checkpoint")
            .arg(Arg::new("image-path").long("image-path"))
            .arg(Arg::new("work-path").long("work-path"))
            .arg(
                Arg::new("leave-running")
                    .long("leave-running")
                    .action(Action::StoreTrue),
            )
            .arg(
                Arg::new("pre-dump")
                    .long("pre-dump")
                    .action(Action::StoreTrue),
            ),
        args,
        USAGE,
    )?;
    if matches.positional_count() > 1 {
        return Err(invalid_input(USAGE));
    }

    let id = required_positional(&matches, 0, "container ID required")?.to_string();
    crate::cli::validate_container_id(&id)?;
    let image_path = matches.get_one::<PathBuf>("image-path");
    let work_path = matches.get_one::<PathBuf>("work-path");
    let leave_running = matches.get_flag("leave-running");
    let need_pre_dump = matches.get_flag("pre-dump");

    let image_path =
        image_path.unwrap_or_else(|| PathBuf::from(format!("/var/lib/edgerun/checkpoint/{}", id)));
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

    let state = load_state(&id)?;

    if state.status != "running" && state.status != "created" && state.status != "paused" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("container {} is not running (status: {})", id, state.status),
        ));
    }

    let pid = state
        .pid
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "container has no PID"))?;
    if !crate::cli::is_process_alive(pid) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("container {} has no live process (pid {})", id, pid),
        ));
    }

    if let Err(err) = crate::criu::validate_criu_image_path(&image_path) {
        return Err(err);
    }
    if let Err(err) = crate::criu::validate_criu_image_path(&work_path) {
        return Err(err);
    }

    fs::create_dir_all(&image_path)?;
    fs::create_dir_all(&work_path)?;

    let flags = if need_pre_dump {
        crate::criu::DumpFlags::PreDump as u32
    } else {
        crate::criu::DumpFlags::Empty as u32
    };

    let dump_opts = crate::criu::CriuDumpOpts {
        pid: pid as i32,
        img: &image_path,
        work: Some(&work_path),
        flags,
        status_fd: None,
    };

    crate::criu::criu_dump(&dump_opts)?;

    let should_stop_container = !leave_running && !need_pre_dump;
    if should_stop_container {
        let mut new_state = state.clone();
        new_state.status = "stopped".to_string();
        new_state.pid = None;
        save_state(&new_state, &id)?;
    }

    eprintln!("Checkpoint saved to {}", image_path.display());
    Ok(())
}
