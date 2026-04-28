//! Filesystem tree copy helpers for rootfs materialization and layer merging.

use crate::prelude::*;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{symlink, FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy)]
struct CopyMode {
    skip_whiteouts: bool,
    atomic_replace: bool,
    preserve_metadata: bool,
    copy_special: bool,
}

const MATERIALIZE: CopyMode = CopyMode {
    skip_whiteouts: false,
    atomic_replace: false,
    preserve_metadata: true,
    copy_special: true,
};

const MERGE_LAYER: CopyMode = CopyMode {
    skip_whiteouts: true,
    atomic_replace: true,
    preserve_metadata: false,
    copy_special: false,
};

pub fn copy_rootfs_tree(src: &Path, dest: &Path) -> io::Result<()> {
    if !src.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("image rootfs not found: {}", src.display()),
        ));
    }
    fs::create_dir_all(dest)?;
    copy_dir_contents(src, dest, MATERIALIZE)
}

pub fn merge_layer_dirs(layer_dirs: &[PathBuf], dest: &Path) -> io::Result<()> {
    for layer_dir in layer_dirs {
        copy_dir_contents(layer_dir, dest, MERGE_LAYER)?;
    }
    Ok(())
}

fn copy_dir_contents(src: &Path, dest: &Path, mode: CopyMode) -> io::Result<()> {
    if !src.is_dir() {
        return Ok(());
    }
    let entries = fs::read_dir(src)?.collect::<io::Result<Vec<_>>>()?;
    if mode.skip_whiteouts
        && entries.iter().any(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| name == ".wh..wh..opq")
                .unwrap_or(false)
        })
    {
        clear_dir_contents(dest)?;
    }
    for entry in entries {
        if mode.skip_whiteouts {
            if let Some(name) = entry.file_name().to_str() {
                if name == ".wh..wh..opq" {
                    continue;
                }
                if name.starts_with(".wh.") {
                    remove_path(&dest.join(name.trim_start_matches(".wh.")))?;
                    continue;
                }
            }
        }
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        copy_rootfs_entry(&src_path, &dest_path, mode)?;
    }
    Ok(())
}

fn clear_dir_contents(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(path)? {
        remove_path(&entry?.path())?;
    }
    Ok(())
}

fn remove_path(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn copy_rootfs_entry(src: &Path, dest: &Path, mode: CopyMode) -> io::Result<()> {
    let metadata = fs::symlink_metadata(src)?;
    let file_type = metadata.file_type();
    if file_type.is_dir() {
        fs::create_dir_all(dest)?;
        copy_dir_contents(src, dest, mode)?;
        if mode.preserve_metadata {
            fs::set_permissions(
                dest,
                fs::Permissions::from_mode(metadata.permissions().mode()),
            )?;
        }
    } else if file_type.is_symlink() {
        let target = fs::read_link(src)?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        if mode.atomic_replace {
            let temp_path = temp_path(dest);
            let _ = fs::remove_file(&temp_path);
            symlink(target, &temp_path)?;
            fs::rename(&temp_path, dest)?;
        } else {
            let _ = fs::remove_file(dest);
            symlink(target, dest)?;
        }
    } else if file_type.is_file() {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        if mode.atomic_replace {
            let temp_path = temp_path(dest);
            fs::copy(src, &temp_path)?;
            fs::rename(&temp_path, dest)?;
        } else {
            fs::copy(src, dest)?;
        }
        if mode.preserve_metadata {
            fs::set_permissions(
                dest,
                fs::Permissions::from_mode(metadata.permissions().mode()),
            )?;
        }
    } else if mode.copy_special && file_type.is_fifo() {
        create_special_file(dest, libc::S_IFIFO, metadata.permissions().mode(), 0)?;
    } else if mode.copy_special && (file_type.is_char_device() || file_type.is_block_device()) {
        let kind = if file_type.is_char_device() {
            libc::S_IFCHR
        } else {
            libc::S_IFBLK
        };
        create_special_file(dest, kind, metadata.permissions().mode(), metadata.rdev())?;
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("unsupported rootfs entry type: {}", src.display()),
        ));
    }
    Ok(())
}

fn create_special_file(dest: &Path, kind: libc::mode_t, mode: u32, dev: u64) -> io::Result<()> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    let path = std::ffi::CString::new(dest.as_os_str().as_bytes())
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    let ret = unsafe { libc::mknod(path.as_ptr(), kind | (mode as libc::mode_t), dev) };
    if ret == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

fn temp_path(dest: &Path) -> PathBuf {
    let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    dest.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".tmp.{:x}-{n:x}", std::process::id()))
}

#[cfg(test)]
#[path = "../tests/unit_src/src/rootfs_copy_tests.rs"]
mod tests;
