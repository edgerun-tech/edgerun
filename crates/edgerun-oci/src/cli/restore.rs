//! Restore command implementation.
//!
//! Restores a container from a checkpoint using CRIU.

use crate::prelude::*;
use std::fs;
use std::io;
use std::path::PathBuf;

use crate::state::{save_state, ContainerState as StateContainerState};

pub fn cmd_restore(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    if let Some(ref root) = opts.root {
        let root_str = root.to_str().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "--root path is not valid UTF-8",
            )
        })?;
        crate::state::set_state_dir(root_str);
    }

    let mut id = None;
    let mut image_path = None;
    let mut work_path = None;
    let mut bundle_path = None;

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
            "--bundle" => {
                if i + 1 < args.len() {
                    bundle_path = Some(PathBuf::from(&args[i + 1]));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            s if s.starts_with("--bundle=") => {
                bundle_path = Some(PathBuf::from(&s["--bundle=".len()..]));
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

    let id =
        id.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "container ID required"))?;

    let image_path = image_path
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "--image-path is required"))?;

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
