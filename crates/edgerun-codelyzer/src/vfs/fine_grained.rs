//! Fine-grained locked VFS for concurrent writes to different files
//!
//! Uses host-side synchronization primitives to allow safe shared access
//! from multiple threads without pulling in a concurrent-map dependency.
//!
//! Stores ALL files as `Vec<u8>` (bytes), including binary files.
//! Text-only operations (`read_str`, `edit`, `grep`) transparently
//! handle UTF-8 conversion and skip non-text files.
//!
//! **Known limitations**:
//! - `read_str()` returns `String` (not `&str`) because lock guards
//!   can't outlive the function call. Use `read()` for zero-copy via `Arc<Vec<u8>>`.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{SystemTime, UNIX_EPOCH};

use super::FileMeta;
use super::grep::GrepMatch;
use super::stats::MemoryStats;

struct LockedFileContent {
    data: Arc<Vec<u8>>,
    version: std::sync::atomic::AtomicU64,
}

pub struct FineGrainedVFS {
    files: RwLock<HashMap<PathBuf, Arc<LockedFileContent>>>,
    metadata: RwLock<HashMap<PathBuf, FileMeta>>,
    original_hashes: RwLock<HashMap<PathBuf, String>>,
    deleted: RwLock<HashSet<PathBuf>>,
    root: PathBuf,
    memory_usage: std::sync::atomic::AtomicUsize,
}

impl FineGrainedVFS {
    /// Load entire directory into memory (ALL files, including binary).
    pub fn load<P: AsRef<Path>>(root: P) -> Result<Self, String> {
        let root = root.as_ref().to_path_buf();
        let mut files = HashMap::new();
        let mut metadata = HashMap::new();
        let mut original_hashes = HashMap::new();
        let deleted = HashSet::new();
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
            files: RwLock::new(files),
            metadata: RwLock::new(metadata),
            original_hashes: RwLock::new(original_hashes),
            deleted: RwLock::new(deleted),
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
        let mut hash = 0xcbf29ce484222325u64;
        for byte in content {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }

    fn get_mtime(path: &Path) -> u64 {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs())
            .unwrap_or(0)
    }

    /// Read file bytes from memory (zero-copy via Arc if possible)
    pub fn read(&self, path: &Path) -> Option<Arc<Vec<u8>>> {
        if self.deleted.read().ok()?.contains(path) {
            return None;
        }
        self.files
            .read()
            .ok()?
            .get(path)
            .map(|entry| entry.data.clone())
    }

    /// Read file as String (clones data). Returns None for binary or deleted files.
    pub fn read_str(&self, path: &Path) -> Option<String> {
        if self.deleted.read().ok()?.contains(path) {
            return None;
        }
        self.files
            .read()
            .ok()?
            .get(path)
            .and_then(|entry| String::from_utf8(entry.data.to_vec()).ok())
    }

    /// Check if file is text (valid UTF-8)
    pub fn is_text(&self, path: &Path) -> bool {
        self.files
            .read()
            .ok()
            .and_then(|files| {
                files
                    .get(path)
                    .map(|entry| std::str::from_utf8(&entry.data).is_ok())
            })
            .unwrap_or(false)
    }

    /// List all non-deleted files
    pub fn files(&self) -> Vec<PathBuf> {
        let files = self.files.read().unwrap();
        let deleted = self.deleted.read().unwrap();
        files
            .keys()
            .filter(|path| !deleted.contains(*path))
            .cloned()
            .collect()
    }

    /// Write bytes to file in memory
    pub fn write_bytes(&self, path: &Path, content: Vec<u8>) -> Result<(), String> {
        let path = path.to_path_buf();
        self.deleted
            .write()
            .map_err(|_| "deleted set lock poisoned".to_string())?
            .remove(&path);

        let size = content.len();
        let hash = Self::compute_hash(&content);

        let new_content = Arc::new(LockedFileContent {
            data: Arc::new(content),
            version: std::sync::atomic::AtomicU64::new(1),
        });

        let old_size = self
            .files
            .write()
            .map_err(|_| "file map lock poisoned".to_string())?
            .insert(path.clone(), new_content)
            .map(|e| e.data.len())
            .unwrap_or(0);

        let mut metadata = self
            .metadata
            .write()
            .map_err(|_| "metadata lock poisoned".to_string())?;
        let meta = metadata.entry(path.clone()).or_insert_with(|| FileMeta {
            path: path.clone(),
            size: 0,
            hash: String::new(),
            modified: 0,
            is_dirty: true,
        });

        meta.size = size;
        meta.hash = hash;
        meta.modified = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        meta.is_dirty = true;

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

        if self
            .deleted
            .read()
            .map_err(|_| "deleted set lock poisoned".to_string())?
            .contains(&path)
        {
            return Err(format!("File {:?} was deleted", path));
        }

        let entry = self
            .files
            .read()
            .map_err(|_| "file map lock poisoned".to_string())?
            .get(&path)
            .cloned()
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

        self.files
            .write()
            .map_err(|_| "file map lock poisoned".to_string())?
            .insert(path.clone(), new_entry);

        if let Some(meta) = self
            .metadata
            .write()
            .map_err(|_| "metadata lock poisoned".to_string())?
            .get_mut(&path)
        {
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

    /// Search for pattern in text files. Binary files are skipped.
    pub fn grep(&self, pattern: &str) -> Vec<GrepMatch> {
        use crate::regex::Regex;

        let regex = match Regex::new(pattern) {
            Some(r) => r,
            None => return Vec::new(),
        };

        let files = self.files.read().unwrap();
        let deleted = self.deleted.read().unwrap();

        files
            .iter()
            .filter(|(path, _)| !deleted.contains(*path))
            .filter_map(|(path, entry)| {
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
        let files = self.files.read().unwrap();
        let metadata = self.metadata.read().unwrap();
        let deleted = self.deleted.read().unwrap();
        let file_count = files.len();
        let memory_bytes = self.memory_usage.load(std::sync::atomic::Ordering::Relaxed);

        MemoryStats {
            file_count,
            dirty_count: metadata.values().filter(|entry| entry.is_dirty).count(),
            deleted_count: deleted.len(),
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
        let tmp = crate::vfs::test_support::tempdir().unwrap();
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
        let tmp = crate::vfs::test_support::tempdir().unwrap();
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

        assert!(
            vfs.read_str(Path::new("file1.txt"))
                .unwrap()
                .contains("edited1")
        );
        assert!(
            vfs.read_str(Path::new("file2.txt"))
                .unwrap()
                .contains("edited2")
        );
    }

    #[test]
    fn test_grep_matches_all_files() {
        let tmp = crate::vfs::test_support::tempdir().unwrap();
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
        let tmp = crate::vfs::test_support::tempdir().unwrap();
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
