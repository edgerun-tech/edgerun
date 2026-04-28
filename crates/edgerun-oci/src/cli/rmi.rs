//! Local image removal command.

use crate::prelude::*;
use std::io;
use std::path::PathBuf;

use crate::cli::default_images_dir;
use crate::state::state_root_dir;
use crate::ImageRef;

pub fn cmd_rmi(opts: &crate::cli::GlobalOpts, args: &[String]) -> io::Result<()> {
    crate::cli::apply_global_opts(opts)?;

    let (image_ref, images_dir, force) = parse_rmi_args(args)?;
    let image: ImageRef = image_ref
        .parse()
        .map_err(|e: String| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let bundle = images_dir.join(&image.repository).join(&image.tag);
    if !bundle.join("config.json").is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("image not found: {image_ref}"),
        ));
    }
    if !force {
        if let Some(container) = referencing_container(&bundle)? {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("image is used by container {container}; use --force"),
            ));
        }
    }
    std::fs::remove_dir_all(&bundle)?;
    prune_empty_parents(&images_dir, bundle.parent());
    println!("Removed {image_ref}");
    Ok(())
}

fn parse_rmi_args(args: &[String]) -> io::Result<(String, PathBuf, bool)> {
    let mut images_dir = default_images_dir();
    let mut force = false;
    let mut image = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "--force" | "-f" => {
                force = true;
                i += 1;
            }
            "--images-dir" if i + 1 < args.len() => {
                images_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            }
            _ if !args[i].starts_with('-') && image.is_none() => {
                image = Some(args[i].clone());
                i += 1;
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Usage: ert rmi [--images-dir DIR] <image>",
                ))
            }
        }
    }
    image
        .map(|image| (image, images_dir, force))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "image is required"))
}

fn referencing_container(bundle: &std::path::Path) -> io::Result<Option<String>> {
    let bundle = bundle
        .canonicalize()
        .unwrap_or_else(|_| bundle.to_path_buf());
    let root = state_root_dir();
    let Ok(entries) = std::fs::read_dir(root) else {
        return Ok(None);
    };
    for entry in entries {
        let entry = entry?;
        let state_path = entry.path().join("state.json");
        let Ok(data) = std::fs::read_to_string(state_path) else {
            continue;
        };
        let Ok(state) = edgerun_json::from_str::<crate::state::ContainerState>(&data) else {
            continue;
        };
        let state_bundle = std::path::PathBuf::from(&state.bundle);
        let state_bundle = state_bundle.canonicalize().unwrap_or(state_bundle);
        if state_bundle == bundle {
            return Ok(Some(state.id));
        }
    }
    Ok(None)
}

fn prune_empty_parents(root: &std::path::Path, mut dir: Option<&std::path::Path>) {
    while let Some(path) = dir {
        if path == root {
            break;
        }
        match std::fs::remove_dir(path) {
            Ok(()) => dir = path.parent(),
            Err(_) => break,
        }
    }
}
