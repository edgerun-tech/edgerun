//! Layer extraction, whiteout handling, and rootfs building.
//!
//! Fixes applied:
//! - **Tar symlink/ path traversal validation** — all entry paths validated against dest root
//! - **Streaming digest verification** — no longer loads entire blob into memory
//! - **Overlay whiteout char device handling** (0:0 device check)

use crate::prelude::*;
use std::fs::{self, File};
use std::io::{self, Read};
use std::os::unix::fs::{symlink, FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::errors::RegistryError;
use crate::layer_pipeline::bytes_to_hex;
use crate::oci_path::{layer_path_safe, normalize_layer_path};
use crate::tar_layer::{
    apply_uncompressed_tar_layer, decompress_gzip_layer, decompress_zstd_layer, layer_compression,
    parse_oci_whiteout, OciLayerCompression, OciWhiteout, TarEntry, TarEntryKind, TarLayerSink,
};
use edgerun_crypto::sha2::Digest;

// ===========================================================================
// Path validation helpers
// ===========================================================================

/// Check that a path component list does not escape the root via "..".
fn path_safe_within_root(path: &std::path::Path) -> bool {
    if let Some(path) = path.to_str() {
        return layer_path_safe(path);
    }

    use std::path::Component;
    let mut depth = 0isize;
    for comp in path.components() {
        match comp {
            Component::RootDir => {}
            Component::Normal(_) => depth += 1,
            Component::ParentDir => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    true
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

    fn ensure_parent(path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        Ok(())
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
                Self::ensure_parent(&path)?;
                fs::write(&path, data).map_err(|error| error.to_string())?;
                fs::set_permissions(&path, fs::Permissions::from_mode(entry.mode))
                    .map_err(|error| error.to_string())?;
            }
            TarEntryKind::Directory => {
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
                Self::ensure_parent(&path)?;
                let _ = fs::remove_file(&path);
                symlink(target, &path).map_err(|error| error.to_string())?;
            }
            TarEntryKind::Hardlink => {
                let target = entry
                    .link_name
                    .as_deref()
                    .ok_or_else(|| format!("hardlink {:?} missing target", entry.path))?;
                let target = self.dest.join(normalize_layer_path(target));
                Self::ensure_parent(&path)?;
                let _ = fs::remove_file(&path);
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
    let computed = format!("sha256:{}", bytes_to_hex(&computed_hash));

    if computed != expected_digest {
        return Err(RegistryError::DigestMismatch {
            expected: expected_digest.to_string(),
            computed,
        });
    }

    Ok(())
}

#[cfg(all(test, not(target_os = "none")))]
mod tests {
    use super::*;
    use crate::test_support::{tar, tar_entry};
    use std::io::Write;

    fn tmp_dir() -> std::path::PathBuf {
        static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("oci_layer_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn extract_layer_basic() {
        let tmp = tmp_dir();
        let dest = tmp.join("rootfs");
        std::fs::create_dir_all(&dest).unwrap();

        let tar_data = tar(vec![tar_entry("file.txt", b'0', b"hello world")]);
        let tar_file = tmp.join("layer.tar");
        std::fs::write(&tar_file, &tar_data).unwrap();

        extract_layer(&tar_file, &dest, None).unwrap();

        let extracted = dest.join("file.txt");
        assert!(extracted.exists());
        assert_eq!(std::fs::read_to_string(&extracted).unwrap(), "hello world");
    }

    #[test]
    fn path_safe_within_root_valid() {
        assert!(path_safe_within_root(std::path::Path::new("foo/bar")));
        assert!(path_safe_within_root(std::path::Path::new("a/b/c")));
    }

    #[test]
    fn path_safe_within_root_rejects_absolute_parent_escape() {
        assert!(!path_safe_within_root(std::path::Path::new("/foo")));
        assert!(!path_safe_within_root(std::path::Path::new("../foo")));
        assert!(!path_safe_within_root(std::path::Path::new("../../../etc")));
    }

    #[test]
    fn verify_blob_digest_valid() {
        let tmp = tmp_dir();
        let blob_path = tmp.join("blob");
        let mut f = std::fs::File::create(&blob_path).unwrap();
        let content = b"hello";
        f.write_all(content).unwrap();
        f.sync_all().unwrap();

        // Calculate expected SHA256
        use edgerun_crypto::sha2::Digest;
        let mut hasher = edgerun_crypto::sha2::Sha256::new();
        hasher.update(content);
        let hash = hasher.finalize();
        let expected = format!("sha256:{}", bytes_to_hex(&hash));

        let result = verify_blob_digest(&blob_path, &expected);
        assert!(result.is_ok(), "digest mismatch: {:?}", result);
    }

    #[test]
    fn verify_blob_digest_mismatch() {
        let tmp = tmp_dir();
        let blob_path = tmp.join("blob");
        std::fs::write(&blob_path, b"hello").unwrap();

        let result = verify_blob_digest(
            &blob_path,
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        );
        assert!(result.is_err());
    }
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
    remove_whiteout_files_in(dir, dir)
}

fn remove_whiteout_files_in(root: &Path, dir: &Path) -> Result<(), RegistryError> {
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

        if let Some(relative_path) = path.strip_prefix(root).ok().and_then(|path| path.to_str()) {
            match parse_oci_whiteout(relative_path) {
                Some(OciWhiteout::RemovePath(target_path)) => {
                    let target = root.join(target_path);
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
                Some(OciWhiteout::OpaqueDirectory(_)) => {
                    let _ = fs::remove_file(&path);
                    continue;
                }
                None => {}
            }
        }

        // Overlayfs char device whiteout (0:0 character device)
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
/// Preserves file permissions via fs::copy (which copies mode bits).
/// Uses atomic rename for regular files: write to temp, then rename.
fn copy_dir_contents(src: &Path, dest: &Path) -> Result<(), RegistryError> {
    use std::os::unix::fs::symlink;
    use std::sync::atomic::{AtomicU64, Ordering};

    if !src.is_dir() {
        return Ok(());
    }

    static COUNTER: AtomicU64 = AtomicU64::new(0);

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
            let target = fs::read_link(&src_path).map_err(RegistryError::IoError)?;
            // Atomic symlink: create temp link, then rename
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let temp_path = dest.join(format!(".tmp.{:x}-{:x}", std::process::id(), n));
            let _ = fs::remove_file(&temp_path);
            symlink(&target, &temp_path).map_err(RegistryError::IoError)?;
            fs::rename(&temp_path, &dest_path).map_err(RegistryError::IoError)?;
        } else {
            // Atomic file copy: write to temp file, then rename
            let n = COUNTER.fetch_add(1, Ordering::Relaxed);
            let temp_path = dest.join(format!(".tmp.{:x}-{:x}", std::process::id(), n));
            fs::copy(&src_path, &temp_path).map_err(RegistryError::IoError)?;
            fs::rename(&temp_path, &dest_path).map_err(RegistryError::IoError)?;
        }
    }

    Ok(())
}
