//! Filesystem tree copy helpers for materializing writable container rootfs trees.

use crate::prelude::*;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{symlink, FileTypeExt, MetadataExt, PermissionsExt};
use std::path::Path;

pub fn copy_rootfs_tree(src: &Path, dest: &Path) -> io::Result<()> {
    if !src.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("image rootfs not found: {}", src.display()),
        ));
    }
    fs::create_dir_all(dest)?;
    copy_dir_contents(src, dest)
}

fn copy_dir_contents(src: &Path, dest: &Path) -> io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        copy_rootfs_entry(&src_path, &dest_path)?;
    }
    Ok(())
}

fn copy_rootfs_entry(src: &Path, dest: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(src)?;
    let file_type = metadata.file_type();
    if file_type.is_dir() {
        fs::create_dir_all(dest)?;
        copy_dir_contents(src, dest)?;
        fs::set_permissions(
            dest,
            fs::Permissions::from_mode(metadata.permissions().mode()),
        )?;
    } else if file_type.is_symlink() {
        let target = fs::read_link(src)?;
        let _ = fs::remove_file(dest);
        symlink(target, dest)?;
    } else if file_type.is_file() {
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dest)?;
        fs::set_permissions(
            dest,
            fs::Permissions::from_mode(metadata.permissions().mode()),
        )?;
    } else if file_type.is_fifo() {
        create_special_file(dest, libc::S_IFIFO, metadata.permissions().mode(), 0)?;
    } else if file_type.is_char_device() || file_type.is_block_device() {
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
