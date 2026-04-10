//! Layer extraction, whiteout handling, and rootfs building.
//!
//! Fixes applied:
//! - **Tar symlink/ path traversal validation** — all entry paths validated against dest root
//! - **Streaming digest verification** — no longer loads entire blob into memory
//! - **Overlay whiteout char device handling** (0:0 device check)

use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

use crate::errors::RegistryError;

// ===========================================================================
// Secure tar extraction
// ===========================================================================

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
        extract_tar_secure(&mut decoder, dest)?;
    } else if is_gzip {
        let mut decoder = flate2::read::GzDecoder::new(file);
        extract_tar_secure(&mut decoder, dest)?;
    } else {
        extract_tar_secure(&mut BufReader::new(file), dest)?;
    }

    Ok(())
}

/// Securely extract a tar stream to a directory.
/// Validates all entry paths to prevent directory traversal and symlink escape.
pub fn extract_tar_secure<R: Read>(reader: R, dest: &Path) -> Result<(), RegistryError> {
    let mut archive = tar::Archive::new(reader);
    let dest = dest.canonicalize().unwrap_or(dest.to_path_buf());

    for entry in archive.entries().map_err(|e| RegistryError::IoError(e))? {
        let entry = entry.map_err(|e| RegistryError::IoError(e))?;
        let path = entry.path().map_err(|e| RegistryError::IoError(e))?;

        // Validate: entry must resolve within dest
        let entry_path = dest.join(&path);
        let entry_path = entry_path.canonicalize().unwrap_or(entry_path);
        if !entry_path.starts_with(&dest) {
            return Err(RegistryError::IoError(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("tar entry {:?} escapes destination {:?}", path, dest),
            )));
        }

        // Validate symlinks: target must resolve within dest
        if entry.header().entry_type() == tar::EntryType::Symlink {
            if let Some(link_target) = entry.link_name().map_err(|e| RegistryError::IoError(e))? {
                // If absolute, check it's within dest; if relative, resolve from entry's parent
                let resolved = if link_target.is_absolute() {
                    link_target.to_path_buf()
                } else {
                    entry_path.parent().unwrap_or(&dest).join(&link_target)
                };
                let resolved = resolved.canonicalize().unwrap_or(resolved);
                if !resolved.starts_with(&dest) {
                    return Err(RegistryError::IoError(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!(
                            "symlink {:?} -> {:?} escapes destination",
                            path, link_target
                        ),
                    )));
                }
            }
        }

        // Extract the entry
        // Use unpack_in which validates paths, but we already validated above
        let mut entry = entry;
        entry.unpack_in(&dest).map_err(|e| RegistryError::IoError(e))?;
    }

    Ok(())
}

// ===========================================================================
// Streaming digest verification
// ===========================================================================

/// Verify that a blob matches the expected digest.
/// Streams the file through a SHA-256 hasher — never loads the entire file into memory.
pub fn verify_blob_digest(
    blob_path: &Path,
    expected_digest: &str,
) -> Result<(), RegistryError> {
    let mut file = File::open(blob_path).map_err(|e| RegistryError::IoError(e))?;
    let mut hasher = edgerun_core::crypto::Sha256Hasher::new();
    let mut buf = [0u8; 65536]; // 64KB buffer
    loop {
        let n = file.read(&mut buf).map_err(|e| RegistryError::IoError(e))?;
        if n == 0 { break; }
        hasher.update(&buf[..n]);
    }
    let computed_hash = hasher.finalize();
    let computed = format!("sha256:{}", edgerun_core::util::bytes_to_hex(&computed_hash));

    if computed != expected_digest {
        return Err(RegistryError::DigestMismatch {
            expected: expected_digest.to_string(),
            computed,
        });
    }

    Ok(())
}

// ===========================================================================
// Whiteout handling
// ===========================================================================

/// Apply whiteout files across layers (reverse order, top layer first).
pub fn apply_whiteouts(layer_dirs: &[PathBuf]) -> Result<(), RegistryError> {
    for layer_dir in layer_dirs.iter().rev() {
        remove_whiteout_files(layer_dir)?;
    }
    Ok(())
}

/// Remove whiteout files from a directory tree.
/// Handles both OCI-style `.wh.` prefix and overlayfs char device whiteouts (0:0).
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
            // Skip the opaque whiteout marker itself
            if name == ".wh..wh..opq" {
                continue;
            }
            // OCI-style whiteout: .wh.<name> → delete <name>
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

        // Overlayfs char device whiteout (0:0 character device)
        if let Ok(metadata) = path.metadata() {
            if metadata.file_type().is_char_device() && metadata.rdev() == 0 {
                let _ = fs::remove_file(&path);
            }
        }

        if path.is_dir() {
            remove_whiteout_files(&path)?;
        }
    }

    Ok(())
}

// ===========================================================================
// Rootfs building
// ===========================================================================

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

        // Skip whiteout files
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
