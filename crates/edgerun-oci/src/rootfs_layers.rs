use crate::prelude::*;
use std::io;
use std::path::{Path, PathBuf};

pub fn apply_whiteouts(layer_dirs: &[PathBuf]) -> io::Result<()> {
    let _ = layer_dirs;
    Ok(())
}

pub fn build_rootfs(layer_dirs: &[PathBuf], dest: &Path) -> io::Result<()> {
    crate::rootfs_copy::merge_layer_dirs(layer_dirs, dest)
}
