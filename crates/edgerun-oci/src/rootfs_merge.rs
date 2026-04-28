//! Layer directory merge helpers for building rootfs trees.

use crate::prelude::*;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub fn build_rootfs_from_layers(layer_dirs: &[PathBuf], dest: &Path) -> io::Result<()> {
    for layer_dir in layer_dirs {
        copy_dir_contents(layer_dir, dest)?;
    }
    Ok(())
}

fn copy_dir_contents(src: &Path, dest: &Path) -> io::Result<()> {
    use std::os::unix::fs::symlink;

    if !src.is_dir() {
        return Ok(());
    }

    let entries = match fs::read_dir(src) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        let file_name = entry.file_name();

        if let Some(name) = file_name.to_str() {
            if name.starts_with(".wh.") {
                continue;
            }
        }

        if src_path.is_dir() {
            let _ = fs::create_dir_all(&dest_path);
            copy_dir_contents(&src_path, &dest_path)?;
        } else if src_path.is_symlink() {
            let target = fs::read_link(&src_path)?;
            let temp_path = temp_path(dest);
            let _ = fs::remove_file(&temp_path);
            symlink(&target, &temp_path)?;
            fs::rename(&temp_path, &dest_path)?;
        } else {
            let temp_path = temp_path(dest);
            fs::copy(&src_path, &temp_path)?;
            fs::rename(&temp_path, &dest_path)?;
        }
    }

    Ok(())
}

fn temp_path(dest: &Path) -> PathBuf {
    let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    dest.join(format!(".tmp.{:x}-{n:x}", std::process::id()))
}
