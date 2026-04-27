//! # edgerun-vfs
//!
//! In-memory virtual filesystem with copy-on-write semantics and
//! git-aware write-back for transparent RAM acceleration.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │                 Programs (cargo, etc.)           │
//! │                         │                        │
//! │                         ▼                        │
//! │            ┌──────────────────────┐              │
//! │            │  bind mount (tmpfs)  │  ← reads/writes go to RAM
//! │            └──────────┬───────────┘              │
//! │                       │                           │
//! │            ┌──────────▼───────────┐              │
//! │            │  notify watcher      │  ← detects changes
//! │            └──────────┬───────────┘              │
//! │                       │                           │
//! │            ┌──────────▼───────────┐              │
//! │            │  GitAwarePersist     │  ← filters gitignored files
//! │            └──────────┬───────────┘              │
//! │                       │                           │
//! │            ┌──────────▼───────────┐              │
//! │            │  async write-back    │  ← syncs to disk
//! │            └──────────┬───────────┘              │
//! │                       │                           │
//! │            ┌──────────▼───────────┐              │
//! │            │  original filesystem │  ← only git-tracked files
//! │            └──────────────────────┘              │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! - **ALL files** are loaded into RAM (not just git-tracked)
//! - Only **non-gitignored** files are written back to disk
//! - `target/`, `node_modules/`, etc. live entirely in RAM
//! - Write-back is async and batched — transparent to programs
//!
//! ## Quick Start (library)
//!
//! ```rust,no_run
//! use edgerun_vfs::VirtualFileSystem;
//!
//! let vfs = VirtualFileSystem::load("/path/to/project").unwrap();
//!
//! if let Some(content) = vfs.read_str(std::path::Path::new("src/main.rs")) {
//!     println!("File size: {} bytes", content.len());
//! }
//!
//! let matches = vfs.grep("fn main");
//! println!("Found {} matches", matches.len());
//! ```
//!
//! ## Quick Start (mount binary)
//!
//! ```sh
//! sudo edgerun-vfs-mount /path/to/project /mnt/ram --ram-size=32G
//! # Now /mnt/ram serves files from RAM
//! # cargo build works at RAM speed
//! # target/ stays in RAM only (not synced to disk)
//! ```

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(not(target_os = "none"))]
mod fine_grained;
#[cfg(not(target_os = "none"))]
mod git_aware;
mod grep;
mod stats;
mod vfs;

#[cfg(not(target_os = "none"))]
pub use fine_grained::FineGrainedVFS;
#[cfg(not(target_os = "none"))]
pub use git_aware::GitAwarePersist;
pub use stats::MemoryStats;
pub use vfs::{Changeset, FileContent, FileMeta, PersistResult, VirtualFileSystem};

#[cfg(not(target_os = "none"))]
pub type SharedVFS = std::sync::Arc<std::sync::RwLock<VirtualFileSystem>>;

#[cfg(target_os = "none")]
pub type SharedVFS = alloc::sync::Arc<edgerun_bare_rt::RwLock<VirtualFileSystem>>;

#[cfg(target_os = "none")]
pub type Path = str;
#[cfg(target_os = "none")]
pub type PathBuf = alloc::string::String;

#[cfg(not(target_os = "none"))]
pub use std::path::{Path, PathBuf};
