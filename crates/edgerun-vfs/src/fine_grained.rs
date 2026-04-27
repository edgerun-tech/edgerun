//! Fine-grained locked VFS for concurrent writes to different files
//!
//! Uses DashMap for per-file locking, allowing concurrent writes
//! to different files without a global RwLock.
//!
//! Stores ALL files as `Vec<u8>` (bytes), including binary files.
//! Text-only operations (`read_str`, `edit`, `grep`) transparently
//! handle UTF-8 conversion and skip non-text files.
//!
//! **Known limitations**:
//! - `read_str()` returns `String` (not `&str`) because DashMap guards
//!   can't outlive the function call. Use `read()` for zero-copy via `Arc<Vec<u8>>`.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use dashmap::DashMap;
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::grep::GrepMatch;
use crate::stats::MemoryStats;
use crate::vfs::FileMeta;

struct LockedFileContent {
    data: Arc<Vec<u8>>,
    version: std::sync::atomic::AtomicU64,
}

pub struct FineGrainedVFS {
    files: DashMap<PathBuf, Arc<LockedFileContent>>,
    metadata: DashMap<PathBuf, FileMeta>,
    original_hashes: DashMap<PathBuf, String>,
    deleted: DashMap<PathBuf, ()>,
    root: PathBuf,
    memory_usage: std::sync::atomic::AtomicUsize,
}

impl FineGrainedVFS {
    /// Load entire directory into memory (ALL files, including binary).
    pub fn load<P: AsRef<Path>>(root: P) -> Result<Self, String> {
        let root = root.as_ref().to_path_buf();
        let files = DashMap::new();
        let metadata = DashMap::new();
        let original_hashes = DashMap::new();
        let deleted = DashMap::new();
        let mut memory_usage = 0usize;

        let file_count = Self::walk_directory(&root, &mut |path, content| {
            let rel_path = path
                .strip_prefix(&root)
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|_| path.to_path_buf());

            let hash = Self::compute_hash(&content);
            let size = content.len();
            memory_usage += size;

            files.insert(
                rel_path.clone(),
                Arc::new(LockedFileContent {
                    data: Arc::new(content),
                    version: std::sync::atomic::AtomicU64::new(1),
                }),
            );

            metadata.insert(
                rel_path.clone(),
                FileMeta {
                    path: rel_path.clone(),
                    size,
                    hash: hash.clone(),
                    modified: Self::get_mtime(path),
                    is_dirty: false,
                },
            );

            original_hashes.insert(rel_path, hash);
        })?;

        Ok(Self {
            files,
            metadata,
            original_hashes,
            deleted,
            root,
            memory_usage: std::sync::atomic::AtomicUsize::new(memory_usage),
        })
    }

    fn walk_directory<F>(root: &Path, callback: &mut F) -> Result<usize, String>
    where
        F: FnMut(&PathBuf, Vec<u8>),
    {
        fn walk_impl(
            dir: &Path,
            callback: &mut dyn FnMut(&PathBuf, Vec<u8>),
        ) -> Result<usize, String> {
            let mut local_count = 0;
            for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();
                if path.is_dir() {
                    local_count += walk_impl(&path, callback)?;
                } else if let Ok(content) = std::fs::read(&path) {
                    callback(&path, content);
                    local_count += 1;
                }
            }
            Ok(local_count)
        }
        walk_impl(root, callback)
    }

    fn compute_hash(content: &[u8]) -> String {
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    fn get_mtime(path: &Path) -> u64 {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs())
            .unwrap_or(0)
    }

    /// Read file bytes from memory (zero-copy via Arc if possible)
    pub fn read(&self, path: &Path) -> Option<Arc<Vec<u8>>> {
        if self.deleted.contains_key(path) {
            return None;
        }
        self.files.get(path).map(|entry| entry.data.clone())
    }

    /// Read file as String (clones data). Returns None for binary or deleted files.
    pub fn read_str(&self, path: &Path) -> Option<String> {
        if self.deleted.contains_key(path) {
            return None;
        }
        self.files
            .get(path)
            .and_then(|entry| String::from_utf8(entry.data.to_vec()).ok())
    }

    /// Check if file is text (valid UTF-8)
    pub fn is_text(&self, path: &Path) -> bool {
        self.files
            .get(path)
            .map(|entry| std::str::from_utf8(&entry.data).is_ok())
            .unwrap_or(false)
    }

    /// List all non-deleted files
    pub fn files(&self) -> Vec<PathBuf> {
        self.files
            .iter()
            .filter(|entry| !self.deleted.contains_key(entry.key()))
            .map(|entry| entry.key().clone())
            .collect()
    }

    /// Write bytes to file in memory
    pub fn write_bytes(&self, path: &Path, content: Vec<u8>) -> Result<(), String> {
        let path = path.to_path_buf();
        self.deleted.remove(&path);

        let size = content.len();
        let old_size = self.files.get(&path).map(|e| e.data.len()).unwrap_or(0);
        let hash = Self::compute_hash(&content);

        let new_content = Arc::new(LockedFileContent {
            data: Arc::new(content),
            version: std::sync::atomic::AtomicU64::new(1),
        });

        self.files.insert(path.clone(), new_content);

        self.metadata
            .entry(path.clone())
            .or_insert_with(|| FileMeta {
                path: path.clone(),
                size: 0,
                hash: String::new(),
                modified: 0,
                is_dirty: true,
            });

        if let Some(mut meta) = self.metadata.get_mut(&path) {
            meta.size = size;
            meta.hash = hash;
            meta.modified = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            meta.is_dirty = true;
        }

        self.memory_usage
            .fetch_sub(old_size, std::sync::atomic::Ordering::Relaxed);
        self.memory_usage
            .fetch_add(size, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    /// Write string to file in memory (convenience wrapper)
    pub fn write(&self, path: &Path, content: String) -> Result<(), String> {
        self.write_bytes(path, content.into_bytes())
    }

    /// Edit text file in memory (returns error for binary files)
    pub fn edit<F>(&self, path: &Path, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut String) -> Result<(), String>,
    {
        let path = path.to_path_buf();

        if self.deleted.contains_key(&path) {
            return Err(format!("File {:?} was deleted", path));
        }

        let entry = self
            .files
            .get(&path)
            .ok_or_else(|| format!("File not found: {:?}", path))?;

        let mut text = String::from_utf8(entry.data.to_vec())
            .map_err(|_| format!("Cannot edit binary file: {:?}", path))?;

        f(&mut text)?;

        let new_version = entry.version.load(std::sync::atomic::Ordering::Relaxed) + 1;
        drop(entry);

        let hash = Self::compute_hash(text.as_bytes());
        let size = text.len();

        let new_entry = Arc::new(LockedFileContent {
            data: Arc::new(text.into_bytes()),
            version: std::sync::atomic::AtomicU64::new(new_version),
        });

        self.files.insert(path.clone(), new_entry);

        if let Some(mut meta) = self.metadata.get_mut(&path) {
            meta.size = size;
            meta.hash = hash;
            meta.modified = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            meta.is_dirty = true;
        }

        Ok(())
    }

    /// Search for pattern in text files (parallel). Binary files are skipped.
    pub fn grep(&self, pattern: &str) -> Vec<GrepMatch> {
        use edgerun_regex::Regex;

        let regex = match Regex::new(pattern) {
            Some(r) => r,
            None => return Vec::new(),
        };

        self.files
            .par_iter()
            .filter(|entry| !self.deleted.contains_key(entry.key()))
            .filter_map(|entry| {
                let path = entry.key();
                let text = String::from_utf8(entry.data.to_vec()).ok()?;
                let mut matches = Vec::new();
                for (line_num, line) in text.lines().enumerate() {
                    if regex.is_match(line) {
                        matches.push(GrepMatch {
                            path: path.clone(),
                            line_number: line_num + 1,
                            line: line.to_string(),
                        });
                    }
                }
                Some(matches)
            })
            .flatten()
            .collect()
    }

    /// Get memory usage statistics
    pub fn memory_stats(&self) -> MemoryStats {
        let file_count = self.files.len();
        let memory_bytes = self.memory_usage.load(std::sync::atomic::Ordering::Relaxed);

        MemoryStats {
            file_count,
            dirty_count: self.metadata.iter().filter(|entry| entry.is_dirty).count(),
            deleted_count: self.deleted.len(),
            memory_bytes,
            memory_mb: memory_bytes as f64 / (1024.0 * 1024.0),
            avg_file_size: memory_bytes.checked_div(file_count).unwrap_or(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concurrent_writes_different_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("file1.txt"), "initial1").unwrap();
        std::fs::write(tmp.path().join("file2.txt"), "initial2").unwrap();

        let vfs = FineGrainedVFS::load(tmp.path()).unwrap();

        let vfs = Arc::new(vfs);
        let vfs1 = vfs.clone();
        let vfs2 = vfs.clone();

        let handle1 =
            std::thread::spawn(move || vfs1.write(Path::new("file1.txt"), "modified1".to_string()));
        let handle2 =
            std::thread::spawn(move || vfs2.write(Path::new("file2.txt"), "modified2".to_string()));

        assert!(handle1.join().unwrap().is_ok());
        assert!(handle2.join().unwrap().is_ok());

        assert_eq!(
            vfs.read_str(Path::new("file1.txt")),
            Some("modified1".to_string())
        );
        assert_eq!(
            vfs.read_str(Path::new("file2.txt")),
            Some("modified2".to_string())
        );
    }

    #[test]
    fn test_concurrent_edits_different_files() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(tmp.path().join("file2.txt"), "content2").unwrap();

        let vfs = FineGrainedVFS::load(tmp.path()).unwrap();

        vfs.edit(Path::new("file1.txt"), |c| {
            c.push_str(" - edited1");
            Ok(())
        })
        .unwrap();

        vfs.edit(Path::new("file2.txt"), |c| {
            c.push_str(" - edited2");
            Ok(())
        })
        .unwrap();

        assert!(vfs
            .read_str(Path::new("file1.txt"))
            .unwrap()
            .contains("edited1"));
        assert!(vfs
            .read_str(Path::new("file2.txt"))
            .unwrap()
            .contains("edited2"));
    }

    #[test]
    fn test_grep_is_parallel() {
        let tmp = tempfile::tempdir().unwrap();
        for i in 0..100 {
            let content = format!("fn test_{}() {{ let x = {}; }}\n", i, i);
            std::fs::write(tmp.path().join(format!("file_{}.rs", i)), content).unwrap();
        }

        let vfs = FineGrainedVFS::load(tmp.path()).unwrap();
        let matches = vfs.grep("fn test_");
        assert_eq!(matches.len(), 100);
    }

    #[test]
    fn test_binary_file_handling() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("text.txt"), "hello world").unwrap();
        std::fs::write(tmp.path().join("binary.dat"), std::vec![0u8, 255, 128]).unwrap();

        let vfs = FineGrainedVFS::load(tmp.path()).unwrap();

        assert!(vfs.is_text(Path::new("text.txt")));
        assert!(!vfs.is_text(Path::new("binary.dat")));

        assert_eq!(
            vfs.read_str(Path::new("text.txt")),
            Some("hello world".to_string())
        );
        assert_eq!(vfs.read_str(Path::new("binary.dat")), None);

        let bytes = vfs.read(Path::new("binary.dat")).unwrap();
        assert_eq!(&*bytes, &[0u8, 255, 128]);
    }
}
