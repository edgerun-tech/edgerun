use crate::prelude::*;
use std::fs;
use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

use crate::tar_layer::{parse_oci_whiteout, OciWhiteout};

pub fn apply_whiteouts(layer_dirs: &[PathBuf]) -> io::Result<()> {
    for layer_dir in layer_dirs.iter().rev() {
        remove_whiteout_files(layer_dir)?;
    }
    Ok(())
}

fn remove_whiteout_files(dir: &Path) -> io::Result<()> {
    remove_whiteout_files_in(dir, dir)
}

fn remove_whiteout_files_in(root: &Path, dir: &Path) -> io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(|entry| entry.ok()).collect(),
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let path = entry.path();
        if let Some(relative_path) = path.strip_prefix(root).ok().and_then(|path| path.to_str()) {
            match parse_oci_whiteout(relative_path) {
                Some(OciWhiteout::RemovePath(target_path)) => {
                    remove_path(&root.join(target_path));
                    let _ = fs::remove_file(&path);
                    continue;
                }
                Some(OciWhiteout::OpaqueDirectory(_)) => {
                    let _ = fs::remove_file(&path);
                    continue;
                }
                None => {}
            }
        }

        if let Ok(metadata) = path.metadata() {
            if metadata.file_type().is_char_device() && metadata.rdev() == 0 {
                let _ = fs::remove_file(&path);
            }
        }

        if path.is_dir() {
            remove_whiteout_files_in(root, &path)?;
        }
    }

    Ok(())
}

fn remove_path(path: &Path) {
    if path.exists() {
        if path.is_dir() {
            let _ = fs::remove_dir_all(path);
        } else {
            let _ = fs::remove_file(path);
        }
    }
}

pub fn build_rootfs(layer_dirs: &[PathBuf], dest: &Path) -> io::Result<()> {
    crate::rootfs_copy::merge_layer_dirs(layer_dirs, dest)
}
