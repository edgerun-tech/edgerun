//! # codelyzer VFS
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
//! use edgerun_codelyzer::vfs::VirtualFileSystem;
//!
//! let vfs = VirtualFileSystem::load("/path/to/project").unwrap();
//!
//! if let Some(content) = vfs.read_str(std::path::Path::new("src/main.rs")) {
//!     println!("File size: {} bytes", content.len());
//! }
//!
//! let matches = vfs.find_files_containing("fn main");
//! println!("Found {} matches", matches.len());
//! ```
//!
//! ## Quick Start (mount binary)
//!
//! ```sh
//! sudo codelyzer-vfs-mount /path/to/project /mnt/ram --ram-size=32G
//! # Now /mnt/ram serves files from RAM
//! # cargo build works at RAM speed
//! # target/ stays in RAM only (not synced to disk)
//! ```

#[cfg(not(target_os = "none"))]
mod fine_grained;
#[cfg(not(target_os = "none"))]
mod git_aware;
mod grep;
#[cfg(all(feature = "mount", not(target_os = "none")))]
pub mod inotify;
mod stats;
#[cfg(test)]
mod test_support;
mod virtual_fs;

#[cfg(not(target_os = "none"))]
pub use fine_grained::FineGrainedVFS;
#[cfg(not(target_os = "none"))]
pub use git_aware::GitAwarePersist;
pub use stats::MemoryStats;
pub use virtual_fs::{Changeset, FileContent, FileMeta, PersistResult, VirtualFileSystem};

#[cfg(not(target_os = "none"))]
pub type SharedVFS = std::sync::Arc<std::sync::RwLock<VirtualFileSystem>>;

#[cfg(target_os = "none")]
pub type SharedVFS = alloc::sync::Arc<sync::RwLock<VirtualFileSystem>>;

#[cfg(target_os = "none")]
mod sync {
    use core::cell::UnsafeCell;
    use core::ops::{Deref, DerefMut};
    use core::sync::atomic::{AtomicUsize, Ordering};

    pub struct RwLock<T> {
        data: UnsafeCell<T>,
        state: AtomicUsize,
    }

    unsafe impl<T: Send> Send for RwLock<T> {}
    unsafe impl<T: Send> Sync for RwLock<T> {}

    impl<T> RwLock<T> {
        pub fn new(data: T) -> Self {
            Self {
                data: UnsafeCell::new(data),
                state: AtomicUsize::new(0),
            }
        }

        pub fn read(&self) -> RwLockReadGuard<'_, T> {
            loop {
                let state = self.state.load(Ordering::Acquire);
                if state & 1 != 0 {
                    core::hint::spin_loop();
                    continue;
                }
                if self
                    .state
                    .compare_exchange_weak(state, state + 2, Ordering::Acquire, Ordering::Relaxed)
                    .is_ok()
                {
                    return RwLockReadGuard { lock: self };
                }
            }
        }

        pub fn write(&self) -> RwLockWriteGuard<'_, T> {
            while self
                .state
                .compare_exchange(0, 1, Ordering::Acquire, Ordering::Relaxed)
                .is_err()
            {
                core::hint::spin_loop();
            }
            RwLockWriteGuard { lock: self }
        }
    }

    pub struct RwLockReadGuard<'a, T> {
        lock: &'a RwLock<T>,
    }

    impl<T> Deref for RwLockReadGuard<'_, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            unsafe { &*self.lock.data.get() }
        }
    }

    impl<T> Drop for RwLockReadGuard<'_, T> {
        fn drop(&mut self) {
            self.lock.state.fetch_sub(2, Ordering::Release);
        }
    }

    pub struct RwLockWriteGuard<'a, T> {
        lock: &'a RwLock<T>,
    }

    impl<T> Deref for RwLockWriteGuard<'_, T> {
        type Target = T;

        fn deref(&self) -> &Self::Target {
            unsafe { &*self.lock.data.get() }
        }
    }

    impl<T> DerefMut for RwLockWriteGuard<'_, T> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            unsafe { &mut *self.lock.data.get() }
        }
    }

    impl<T> Drop for RwLockWriteGuard<'_, T> {
        fn drop(&mut self) {
            self.lock.state.fetch_and(!1, Ordering::Release);
        }
    }
}

#[cfg(target_os = "none")]
pub type Path = str;
#[cfg(target_os = "none")]
pub type PathBuf = alloc::string::String;

#[cfg(not(target_os = "none"))]
pub use std::path::{Path, PathBuf};
