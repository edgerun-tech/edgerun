//! Local image removal command.

use crate::prelude::*;
use std::io;
use std::path::PathBuf;

use crate::cli::{default_images_dir, invalid_input, parse_cli_args, required_positional};
use crate::state::state_root_dir;
use crate::ImageRef;
use edgerun_clap::cli::Action;
use edgerun_clap::{Arg, Command};

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
    const USAGE: &str = "Usage: ert rmi [--images-dir DIR] <image>";
    let matches = parse_cli_args(
        Command::new("rmi")
            .arg(
                Arg::new("force")
                    .short('f')
                    .long("force")
                    .action(Action::StoreTrue),
            )
            .arg(Arg::new("images-dir").long("images-dir")),
        args,
        USAGE,
    )?;
    if matches.positional_count() > 1 {
        return Err(invalid_input(USAGE));
    }
    let image = required_positional(&matches, 0, "image is required")?.to_string();
    let images_dir = matches
        .get_one::<PathBuf>("images-dir")
        .unwrap_or_else(default_images_dir);
    Ok((image, images_dir, matches.get_flag("force")))
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
        let Ok(state) = crate::state::load_state_from_str(&data) else {
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
