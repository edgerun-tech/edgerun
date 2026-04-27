//! Layer extraction, whiteout handling, and rootfs building.
//!
//! Fixes applied:
//! - **Tar symlink/ path traversal validation** — all entry paths validated against dest root
//! - **Streaming digest verification** — no longer loads entire blob into memory
//! - **Overlay whiteout char device handling** (0:0 device check)

use crate::prelude::*;
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};

use super::errors::RegistryError;
use crate::layer_pipeline::bytes_to_hex;
use crate::oci_path::layer_path_safe;
use crate::tar_layer::{layer_compression, parse_oci_whiteout, OciLayerCompression, OciWhiteout};
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
    let file = File::open(blob_path).map_err(RegistryError::IoError)?;

    let compression = match layer_compression(media_type) {
        OciLayerCompression::Unknown
            if blob_path.extension().map(|e| e == "gz").unwrap_or(false) =>
        {
            OciLayerCompression::Gzip
        }
        compression => compression,
    };

    match compression {
        OciLayerCompression::Zstd => {
            let mut decoder = zstd::Decoder::new(file).map_err(RegistryError::IoError)?;
            extract_tar_secure(&mut decoder, dest)?;
        }
        OciLayerCompression::Gzip => {
            let mut decoder = flate2::read::GzDecoder::new(file);
            extract_tar_secure(&mut decoder, dest)?;
        }
        OciLayerCompression::Uncompressed | OciLayerCompression::Unknown => {
            extract_tar_secure(&mut BufReader::new(file), dest)?;
        }
    }

    Ok(())
}

/// Securely extract a tar stream to a directory.
/// Validates all entry paths to prevent directory traversal and symlink escape.
pub fn extract_tar_secure<R: Read>(reader: R, dest: &Path) -> Result<(), RegistryError> {
    let mut archive = tar::Archive::new(reader);
    let dest = dest.canonicalize().unwrap_or(dest.to_path_buf());

    for entry in archive.entries().map_err(RegistryError::IoError)? {
        let entry = entry.map_err(RegistryError::IoError)?;
        let path = entry.path().map_err(RegistryError::IoError)?;

        // Validate: entry must resolve within dest.
        // First try canonicalize (resolves symlinks). If that fails (path doesn't
        // exist yet), do a manual component check to reject ".." escape attempts.
        let entry_path = dest.join(&path);
        let entry_resolved = entry_path.canonicalize().ok();
        if let Some(ref resolved) = entry_resolved {
            if !resolved.starts_with(&dest) {
                return Err(RegistryError::IoError(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("tar entry {:?} escapes destination {:?}", path, dest),
                )));
            }
        } else {
            // Path doesn't exist yet — check components manually
            if !path_safe_within_root(&path) {
                return Err(RegistryError::IoError(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    format!("tar entry {:?} would escape destination", path),
                )));
            }
        }

        // Validate symlinks: target must resolve within dest
        if entry.header().entry_type() == tar::EntryType::Symlink {
            if let Some(link_target) = entry.link_name().map_err(RegistryError::IoError)? {
                // If absolute, check it's within dest; if relative, resolve from entry's parent
                let resolved = if link_target.is_absolute() {
                    dest.join(link_target.strip_prefix("/").unwrap_or(&link_target))
                } else {
                    entry_path.parent().unwrap_or(&dest).join(&link_target)
                };
                // Try canonicalize first; if target doesn't exist, check manually
                if let Ok(canonical) = resolved.canonicalize() {
                    if !canonical.starts_with(&dest) {
                        return Err(RegistryError::IoError(io::Error::new(
                            io::ErrorKind::PermissionDenied,
                            format!(
                                "symlink {:?} -> {:?} escapes destination",
                                path, link_target
                            ),
                        )));
                    }
                } else if !path_safe_within_root(&link_target) {
                    return Err(RegistryError::IoError(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        format!(
                            "symlink {:?} -> {:?} would escape destination",
                            path, link_target
                        ),
                    )));
                }
            }
        }

        // Extract the entry
        // Use unpack_in which validates paths, but we already validated above
        let mut entry = entry;
        entry.unpack_in(&dest).map_err(RegistryError::IoError)?;
    }

    Ok(())
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
