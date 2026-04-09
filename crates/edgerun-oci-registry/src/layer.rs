//! Layer extraction, whiteout handling, and rootfs building.

use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

use crate::errors::RegistryError;

/// Extract a compressed layer tarball to a directory.
pub fn extract_layer(
    blob_path: &Path,
    dest: &Path,
    media_type: Option<&str>,
) -> Result<(), RegistryError> {
    let file = File::open(blob_path).map_err(|e| RegistryError::IoError(e))?;

    let is_gzip = media_type
        .map(|mt| mt.contains("gzip"))
        .unwrap_or(false)
        || blob_path.extension().map(|e| e == "gz").unwrap_or(false);
    let is_zstd = media_type
        .map(|mt| mt.contains("zstd"))
        .unwrap_or(false);

    if is_zstd {
        let mut decoder =
            zstd::Decoder::new(file).map_err(|e| RegistryError::IoError(e))?;
        extract_tar(&mut decoder, dest)?;
    } else if is_gzip {
        let mut decoder = flate2::read::GzDecoder::new(file);
        extract_tar(&mut decoder, dest)?;
    } else {
        extract_tar(&mut BufReader::new(file), dest)?;
    }

    Ok(())
}

/// Extract a tar stream to a directory.
pub fn extract_tar<R: Read>(reader: R, dest: &Path) -> Result<(), RegistryError> {
    let mut archive = tar::Archive::new(reader);
    archive
        .unpack(dest)
        .map_err(|e| RegistryError::IoError(e))?;
    Ok(())
}

/// Verify that a blob matches the expected digest.
pub fn verify_blob_digest(
    blob_path: &Path,
    expected_digest: &str,
) -> Result<(), RegistryError> {
    let data = fs::read(blob_path).map_err(|e| RegistryError::IoError(e))?;
    let computed_hash = edgerun_core::crypto::sha256(&data);
    let computed =
        format!("sha256:{}", edgerun_core::util::bytes_to_hex(&computed_hash));

    if computed != expected_digest {
        return Err(RegistryError::DigestMismatch {
            expected: expected_digest.to_string(),
            computed,
        });
    }

    Ok(())
}

/// Apply whiteout files across layers (reverse order, top layer first).
pub fn apply_whiteouts(layer_dirs: &[PathBuf]) -> Result<(), RegistryError> {
    for layer_dir in layer_dirs.iter().rev() {
        remove_whiteout_files(layer_dir)?;
    }
    Ok(())
}

/// Remove whiteout files from a directory tree.
fn remove_whiteout_files(dir: &Path) -> Result<(), RegistryError> {
    if !dir.is_dir() {
        return Ok(());
    }

    let entries: Vec<_> = match fs::read_dir(dir) {
        Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
        Err(_) => return Ok(()),
    };

    for entry in entries {
        let path = entry.path();
        let file_name = entry.file_name();

        if let Some(name) = file_name.to_str() {
            if name == ".wh..wh..opq" {
                continue;
            }
            if let Some(rest) = name.strip_prefix(".wh.") {
                let target = path.parent().unwrap().join(rest);
                if target.exists() {
                    if target.is_dir() {
                        let _ = fs::remove_dir_all(&target);
                    } else {
                        let _ = fs::remove_file(&target);
                    }
                }
                let _ = fs::remove_file(&path);
                continue;
            }
        }

        if let Ok(metadata) = path.metadata() {
            if metadata.file_type().is_char_device() {
                // Overlay whiteout (0:0 device) — would need dev_t check
            }
        }

        if path.is_dir() {
            remove_whiteout_files(&path)?;
        }
    }

    Ok(())
}

/// Build rootfs by merging layers in order.
pub fn build_rootfs(layer_dirs: &[PathBuf], dest: &Path) -> Result<(), RegistryError> {
    for layer_dir in layer_dirs {
        copy_dir_contents(layer_dir, dest)?;
    }
    Ok(())
}

/// Copy all contents from src to dest, overwriting existing files.
fn copy_dir_contents(src: &Path, dest: &Path) -> Result<(), RegistryError> {
    use std::os::unix::fs::symlink;

    if !src.is_dir() {
        return Ok(());
    }

    let entries: Vec<_> = match fs::read_dir(src) {
        Ok(entries) => entries.filter_map(|e| e.ok()).collect(),
        Err(_) => return Ok(()),
    };

    for entry in entries {
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
            let target =
                fs::read_link(&src_path).map_err(|e| RegistryError::IoError(e))?;
            let _ = fs::remove_file(&dest_path);
            let _ = symlink(&target, &dest_path);
        } else {
            let _ = fs::remove_file(&dest_path);
            fs::copy(&src_path, &dest_path)
                .map_err(|e| RegistryError::IoError(e))?;
        }
    }

    Ok(())
}
