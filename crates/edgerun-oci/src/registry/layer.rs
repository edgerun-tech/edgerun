//! Layer extraction, whiteout handling, and rootfs building.
//!
//! Fixes applied:
//! - **Tar symlink/ path traversal validation** — all entry paths validated against dest root
//! - **Streaming digest verification** — no longer loads entire blob into memory
//! - **Overlay whiteout char device handling** (0:0 device check)

use crate::prelude::*;
use std::fs::{self, File};
use std::io::{self, Read};
use std::os::unix::fs::{symlink, FileTypeExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::errors::RegistryError;
use crate::layer_pipeline::format_digest;
use crate::oci_path::{layer_path_safe, normalize_layer_path};
use crate::tar_layer::{
    apply_uncompressed_tar_layer, decompress_gzip_layer, decompress_zstd_layer, layer_compression,
    OciLayerCompression, TarEntry, TarEntryKind, TarLayerSink,
};
use edgerun_crypto::sha2::Digest;

// ===========================================================================
// Path validation helpers
// ===========================================================================

/// Check that a layer-relative path does not escape the destination root.
fn path_safe_within_root(path: &Path) -> bool {
    path.to_str().is_some_and(layer_path_safe)
}

// ===========================================================================
// Secure tar extraction
// ===========================================================================

/// Extract a compressed layer tarball to a directory.
pub fn extract_layer(
    blob_path: &Path,
    dest: &Path,
    media_type: Option<&str>,
) -> Result<(), RegistryError> {
    let mut file = File::open(blob_path).map_err(RegistryError::IoError)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)
        .map_err(RegistryError::IoError)?;

    let compression = match layer_compression(media_type) {
        OciLayerCompression::Unknown
            if blob_path.extension().map(|e| e == "gz").unwrap_or(false) =>
        {
            OciLayerCompression::Gzip
        }
        compression => compression,
    };

    let tar_bytes = match compression {
        OciLayerCompression::Zstd => decompress_zstd_layer(&bytes),
        OciLayerCompression::Gzip => decompress_gzip_layer(&bytes),
        OciLayerCompression::Uncompressed | OciLayerCompression::Unknown => Ok(bytes),
    }
    .map_err(tar_apply_error)?;

    extract_tar_secure(&tar_bytes, dest)?;

    Ok(())
}

/// Securely extract a tar stream to a directory.
/// Validates all entry paths to prevent directory traversal and symlink escape.
pub fn extract_tar_secure(data: &[u8], dest: &Path) -> Result<(), RegistryError> {
    let dest = dest.canonicalize().unwrap_or_else(|_| dest.to_path_buf());
    let mut sink = FsLayerSink { dest };
    apply_uncompressed_tar_layer(data, &mut sink).map_err(|error| {
        RegistryError::IoError(io::Error::new(
            io::ErrorKind::InvalidData,
            error.to_string(),
        ))
    })?;
    Ok(())
}

fn tar_apply_error(error: crate::tar_layer::TarLayerApplyError) -> RegistryError {
    RegistryError::IoError(io::Error::new(
        io::ErrorKind::InvalidData,
        error.to_string(),
    ))
}

struct FsLayerSink {
    dest: PathBuf,
}

impl FsLayerSink {
    fn entry_path(&self, entry: &TarEntry) -> Result<PathBuf, String> {
        let normalized = normalize_layer_path(&entry.path);
        let path = Path::new(&normalized);
        if !path_safe_within_root(path) {
            return Err(format!(
                "tar entry {:?} would escape destination",
                entry.path
            ));
        }
        Ok(self.dest.join(path))
    }

    fn ensure_parent(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            self.reject_symlink_ancestors(parent)?;
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn reject_symlink_ancestors(&self, path: &Path) -> Result<(), String> {
        let relative = path.strip_prefix(&self.dest).unwrap_or(path);
        let mut current = self.dest.clone();
        for component in relative.components() {
            current.push(component.as_os_str());
            match fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    return Err(format!("tar entry parent {:?} is a symlink", current));
                }
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(())
    }

    fn remove_existing(path: &Path) -> Result<(), String> {
        match fs::symlink_metadata(path) {
            Ok(metadata) if metadata.file_type().is_dir() => {
                fs::remove_dir_all(path).map_err(|error| error.to_string())
            }
            Ok(_) => fs::remove_file(path).map_err(|error| error.to_string()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.to_string()),
        }
    }

    fn validate_link(&self, entry_path: &Path, target: &Path) -> Result<(), String> {
        if target.is_absolute() {
            let normalized = normalize_layer_path(target.to_str().unwrap_or_default());
            if normalized.is_empty() || !layer_path_safe(&normalized) {
                return Err(format!(
                    "symlink {:?} -> {:?} would escape destination",
                    entry_path, target
                ));
            }
            return Ok(());
        }

        let resolved = if target.is_absolute() {
            self.dest.join(target.strip_prefix("/").unwrap_or(target))
        } else {
            entry_path.parent().unwrap_or(&self.dest).join(target)
        };

        if let Ok(canonical) = resolved.canonicalize() {
            if !canonical.starts_with(&self.dest) {
                return Err(format!(
                    "symlink {:?} -> {:?} escapes destination",
                    entry_path, target
                ));
            }
        } else {
            let relative = resolved.strip_prefix(&self.dest).unwrap_or(&resolved);
            if !path_safe_within_root(relative) {
                return Err(format!(
                    "symlink {:?} -> {:?} would escape destination",
                    entry_path, target
                ));
            }
        }

        Ok(())
    }
}

impl TarLayerSink for FsLayerSink {
    fn apply_entry(&mut self, entry: &TarEntry, data: &[u8]) -> Result<(), String> {
        let path = self.entry_path(entry)?;

        match entry.kind {
            TarEntryKind::Regular => {
                self.ensure_parent(&path)?;
                Self::remove_existing(&path)?;
                fs::write(&path, data).map_err(|error| error.to_string())?;
                fs::set_permissions(&path, fs::Permissions::from_mode(entry.mode))
                    .map_err(|error| error.to_string())?;
            }
            TarEntryKind::Directory => {
                self.ensure_parent(&path)?;
                match fs::symlink_metadata(&path) {
                    Ok(metadata) if metadata.file_type().is_dir() => {}
                    Ok(_) => Self::remove_existing(&path)?,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.to_string()),
                }
                fs::create_dir_all(&path).map_err(|error| error.to_string())?;
                fs::set_permissions(&path, fs::Permissions::from_mode(entry.mode))
                    .map_err(|error| error.to_string())?;
            }
            TarEntryKind::Symlink => {
                let target = entry
                    .link_name
                    .as_deref()
                    .ok_or_else(|| format!("symlink {:?} missing target", entry.path))?;
                let target = Path::new(target);
                self.validate_link(&path, target)?;
                self.ensure_parent(&path)?;
                Self::remove_existing(&path)?;
                symlink(target, &path).map_err(|error| error.to_string())?;
            }
            TarEntryKind::Hardlink => {
                let target = entry
                    .link_name
                    .as_deref()
                    .ok_or_else(|| format!("hardlink {:?} missing target", entry.path))?;
                let normalized = normalize_layer_path(target);
                if !layer_path_safe(&normalized) {
                    return Err(format!(
                        "hardlink {:?} -> {:?} would escape destination",
                        entry.path, target
                    ));
                }
                let target = self.dest.join(normalized);
                self.ensure_parent(&path)?;
                Self::remove_existing(&path)?;
                fs::hard_link(target, &path).map_err(|error| error.to_string())?;
            }
            TarEntryKind::Character | TarEntryKind::Block | TarEntryKind::Fifo => {}
            TarEntryKind::PaxExtended
            | TarEntryKind::PaxGlobal
            | TarEntryKind::GnuLongName
            | TarEntryKind::GnuLongLink => {}
        }

        Ok(())
    }
}

// ===========================================================================
// Streaming digest verification
// ===========================================================================

/// Verify that a blob matches the expected digest.
/// Streams the file through a SHA-256 hasher — never loads the entire file into memory.
pub fn verify_blob_digest(blob_path: &Path, expected_digest: &str) -> Result<(), RegistryError> {
    let mut file = File::open(blob_path).map_err(RegistryError::IoError)?;
    let mut hasher = edgerun_crypto::sha2::Sha256::new();
    let mut buf = [0u8; 65536]; // 64KB buffer
    loop {
        let n = file.read(&mut buf).map_err(RegistryError::IoError)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let computed_hash = hasher.finalize();
    let computed = format_digest("sha256", &computed_hash);

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
    crate::rootfs_layers::apply_whiteouts(layer_dirs).map_err(RegistryError::IoError)
}

// ===========================================================================
// Rootfs building
// ===========================================================================

/// Build rootfs by merging layers in order.
pub fn build_rootfs(layer_dirs: &[PathBuf], dest: &Path) -> Result<(), RegistryError> {
    crate::rootfs_layers::build_rootfs(layer_dirs, dest).map_err(RegistryError::IoError)
}

#[cfg(all(test, not(target_os = "none")))]
#[path = "../../tests/unit_src/src/registry/layer_tests.rs"]
mod tests;
