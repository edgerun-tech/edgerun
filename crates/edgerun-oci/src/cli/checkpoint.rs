//! Checkpoint command implementation.
//!
//! Checkpoints a running container using CRIU.

use std::fs;
use std::io;
use std::path::PathBuf;

use crate::state::{load_state, save_state};

pub fn cmd_checkpoint(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        let root_str = root.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "--root path is not valid UTF-8")
        })?;
        crate::state::set_state_dir(root_str);
    }

    let mut id = None;
    let mut image_path = None;
    let mut work_path = None;
    let mut leave_running = false;
    let mut need_pre_dump = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--image-path" => {
                if i + 1 < args.len() {
                    image_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            s if s.starts_with("--image-path=") => {
                image_path = Some(PathBuf::from(&s["--image-path=".len()..]));
                i += 1;
            }
            "--work-path" => {
                if i + 1 < args.len() {
                    work_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            s if s.starts_with("--work-path=") => {
                work_path = Some(PathBuf::from(&s["--work-path=".len()..]));
                i += 1;
            }
            "--leave-running" => {
                leave_running = true;
                i += 1;
            }
            "--pre-dump" => {
                need_pre_dump = true;
                i += 1;
            }
            s if s.starts_with('-') => {
                i += 1;
            }
            _ => {
                if id.is_none() {
                    id = Some(args[i].clone());
                }
                i += 1;
            }
        }
    }

    let id = id.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "container ID required"))?;

    let image_path = image_path.unwrap_or_else(|| PathBuf::from(format!("/var/lib/edgerun/checkpoint/{}", id)));
    let work_path = work_path.unwrap_or_else(|| image_path.join("work"));

    let state = load_state(&id)?;

    if state.status != "running" && state.status != "created" && state.status != "paused" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("container {} is not running (status: {})", id, state.status),
        ));
    }

    let pid = state.pid.ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "container has no PID")
    })?;

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

    if !leave_running {
        let mut new_state = state.clone();
        new_state.status = "stopped".to_string();
        new_state.pid = None;
        save_state(&new_state, &id)?;
    }

    eprintln!("Checkpoint saved to {}", image_path.display());
    Ok(())
}