//! Async file I/O — delegates to `spawn_blocking` for non-blocking semantics.
//!
//! All operations are thin wrappers around `std::fs` run on the blocking
//! thread pool, so they never block the async reactor.

use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};

use crate::blocking_pool::{JoinError, JoinHandle};
use crate::runtime::spawn_blocking;

// ===========================================================================
// read
// ===========================================================================

/// Read the entire contents of a file into a `Vec<u8>`.
pub async fn read<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    let path = path.as_ref().to_path_buf();
    spawn_blocking(move || std::fs::read(&path))
        .await
        .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}

/// Read the entire contents of a file into a `String`.
pub async fn read_to_string<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let path = path.as_ref().to_path_buf();
    spawn_blocking(move || std::fs::read_to_string(&path))
        .await
        .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}

// ===========================================================================
// write
// ===========================================================================

/// Write a slice of bytes to a file, creating it if it doesn't exist.
pub async fn write<P: AsRef<Path>, C: AsRef<[u8]>>(path: P, contents: C) -> io::Result<()> {
    let path = path.as_ref().to_path_buf();
    let contents = contents.as_ref().to_vec();
    spawn_blocking(move || std::fs::write(&path, &contents))
        .await
        .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}

// ===========================================================================
// metadata
// ===========================================================================

/// Query the metadata of a file.
pub async fn metadata<P: AsRef<Path>>(path: P) -> io::Result<std::fs::Metadata> {
    let path = path.as_ref().to_path_buf();
    spawn_blocking(move || std::fs::metadata(&path))
        .await
        .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}

// ===========================================================================
// remove_file
// ===========================================================================

/// Remove a file from the filesystem.
pub async fn remove_file<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref().to_path_buf();
    spawn_blocking(move || std::fs::remove_file(&path))
        .await
        .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}

// ===========================================================================
// rename
// ===========================================================================

/// Rename (move) a file or directory.
pub async fn rename<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<()> {
    let from = from.as_ref().to_path_buf();
    let to = to.as_ref().to_path_buf();
    spawn_blocking(move || std::fs::rename(&from, &to))
        .await
        .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}

// ===========================================================================
// create_dir / create_dir_all
// ===========================================================================

/// Create a new, empty directory.
pub async fn create_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref().to_path_buf();
    spawn_blocking(move || std::fs::create_dir(&path))
        .await
        .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}

/// Recursively create a directory and all of its parents.
pub async fn create_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref().to_path_buf();
    spawn_blocking(move || std::fs::create_dir_all(&path))
        .await
        .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}

// ===========================================================================
// remove_dir / remove_dir_all
// ===========================================================================

/// Remove an existing empty directory.
pub async fn remove_dir<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref().to_path_buf();
    spawn_blocking(move || std::fs::remove_dir(&path))
        .await
        .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}

/// Remove a directory at `path` after removing all its contents.
pub async fn remove_dir_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref().to_path_buf();
    spawn_blocking(move || std::fs::remove_dir_all(&path))
        .await
        .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}

// ===========================================================================
// copy
// ===========================================================================

/// Copy the contents of one file to another.
pub async fn copy<P: AsRef<Path>, Q: AsRef<Path>>(from: P, to: Q) -> io::Result<u64> {
    let from = from.as_ref().to_path_buf();
    let to = to.as_ref().to_path_buf();
    spawn_blocking(move || std::fs::copy(&from, &to))
        .await
        .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}

// ===========================================================================
// exists
// ===========================================================================

/// Check if a path exists.
pub async fn exists<P: AsRef<Path>>(path: P) -> bool {
    let path = path.as_ref().to_path_buf();
    spawn_blocking(move || path.exists()).await.unwrap_or(false)
}

// ===========================================================================
// canonicalize
// ===========================================================================

/// Return the canonical, absolute form of a path.
pub async fn canonicalize<P: AsRef<Path>>(path: P) -> io::Result<PathBuf> {
    let path = path.as_ref().to_path_buf();
    spawn_blocking(move || std::fs::canonicalize(&path))
        .await
        .unwrap_or_else(|_| Err(io::Error::other("blocking task panicked")))
}
