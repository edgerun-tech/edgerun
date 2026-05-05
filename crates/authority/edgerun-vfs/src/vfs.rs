//! Core Virtual Filesystem implementation
//!
//! Stores ALL files as `Vec<u8>` (bytes), including binary files.
//! Text-only operations (`read_str`, `edit`, `grep`) transparently
//! handle UTF-8 conversion and skip non-text files.

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use core::str;

use crate::{Path, PathBuf};

use crate::grep::GrepMatch;
use crate::stats::MemoryStats;

type HashMap<K, V> = BTreeMap<K, V>;
type HashSet<T> = BTreeSet<T>;

pub(crate) fn path_display(path: &Path) -> String {
    #[cfg(not(target_os = "none"))]
    {
        path.display().to_string()
    }

    #[cfg(target_os = "none")]
    {
        path.to_string()
    }
}

fn path_buf_display(path: &PathBuf) -> String {
    #[cfg(not(target_os = "none"))]
    {
        path.display().to_string()
    }

    #[cfg(target_os = "none")]
    {
        path.clone()
    }
}

fn path_to_path_buf(path: &Path) -> PathBuf {
    #[cfg(not(target_os = "none"))]
    {
        path.to_path_buf()
    }

    #[cfg(target_os = "none")]
    {
        path.to_string()
    }
}

#[cfg(not(target_os = "none"))]
fn join_path(root: &Path, path: &PathBuf) -> PathBuf {
    root.join(path)
}

#[cfg(target_os = "none")]
fn join_path(root: &Path, path: &PathBuf) -> PathBuf {
    if root.is_empty() {
        return path.clone();
    }
    let mut joined = root.to_string();
    if !joined.ends_with('/') {
        joined.push('/');
    }
    joined.push_str(path);
    joined
}

fn path_extension(path: &PathBuf) -> Option<&str> {
    #[cfg(not(target_os = "none"))]
    {
        path.extension().and_then(|e| e.to_str())
    }

    #[cfg(target_os = "none")]
    {
        path.rsplit('/')
            .next()?
            .rsplit_once('.')
            .map(|(_, ext)| ext)
    }
}

fn path_as_str(path: &PathBuf) -> Option<&str> {
    #[cfg(not(target_os = "none"))]
    {
        path.to_str()
    }

    #[cfg(target_os = "none")]
    {
        Some(path.as_str())
    }
}

fn now_secs() -> u64 {
    #[cfg(not(target_os = "none"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    #[cfg(target_os = "none")]
    {
        0
    }
}

/// File metadata
#[derive(Debug, Clone)]
pub struct FileMeta {
    pub path: PathBuf,
    pub size: usize,
    pub hash: String,
    pub modified: u64,
    pub is_dirty: bool,
}

/// File content with copy-on-write semantics
#[derive(Debug, Clone)]
pub struct FileContent {
    data: Arc<Vec<u8>>,
    version: u64,
}

impl FileContent {
    fn new(content: Vec<u8>) -> Self {
        Self {
            data: Arc::new(content),
            version: 1,
        }
    }

    /// Get content as raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get content as string slice (returns None for binary files)
    pub fn as_str(&self) -> Option<&str> {
        str::from_utf8(&self.data).ok()
    }

    /// Check if content is valid UTF-8 text
    pub fn is_text(&self) -> bool {
        str::from_utf8(&self.data).is_ok()
    }

    /// Check if content is shared (COW)
    pub fn is_shared(&self) -> bool {
        Arc::strong_count(&self.data) > 1
    }

    pub fn version(&self) -> u64 {
        self.version
    }
}

/// Changeset describing modifications
#[derive(Debug)]
pub struct Changeset<'a> {
    pub added: Vec<&'a PathBuf>,
    pub modified: Vec<&'a PathBuf>,
    pub removed: Vec<&'a PathBuf>,
}

/// Virtual filesystem state
#[derive(Debug)]
pub struct VirtualFileSystem {
    files: BTreeMap<PathBuf, FileContent>,
    metadata: HashMap<PathBuf, FileMeta>,
    original_hashes: HashMap<PathBuf, String>,
    deleted: HashSet<PathBuf>,
    root: PathBuf,
    memory_usage: usize,
}

impl VirtualFileSystem {
    /// Create new VFS and load entire directory into memory.
    ///
    /// Loads ALL files including binary files and files in target/, .git/, etc.
    /// No directories are skipped. Binary files are stored as-is.
    pub fn load<P: AsRef<Path>>(root: P) -> Result<Self, String> {
        #[cfg(target_os = "none")]
        {
            let _ = root;
            Err("filesystem loading is unavailable on bare targets".to_string())
        }

        #[cfg(not(target_os = "none"))]
        {
            let root = path_to_path_buf(root.as_ref());
            let mut files = BTreeMap::new();
            let mut metadata = HashMap::new();
            let mut original_hashes = HashMap::new();
            let mut memory_usage = 0usize;

            edgerun_log::info!(
                "Loading filesystem into memory: {}",
                path_buf_display(&root)
            );

            let start = std::time::Instant::now();
            let file_count = Self::walk_directory(&root, &mut |path, content| {
                let rel_path = path
                    .strip_prefix(&root)
                    .map(|p| p.to_path_buf())
                    .unwrap_or_else(|_| path.to_path_buf());

                let hash = Self::compute_hash(&content);
                let size = content.len();
                let modified = Self::get_mtime(path);

                memory_usage += size;

                files.insert(rel_path.clone(), FileContent::new(content));
                metadata.insert(
                    rel_path.clone(),
                    FileMeta {
                        path: rel_path.clone(),
                        size,
                        hash: hash.clone(),
                        modified,
                        is_dirty: false,
                    },
                );
                original_hashes.insert(rel_path, hash);
            })?;

            let elapsed = start.elapsed();
            let memory_mb = memory_usage as f64 / (1024.0 * 1024.0);

            edgerun_log::info!(
                "Loaded {} files ({:.2} MB) in {:.2}s",
                file_count,
                memory_mb,
                elapsed.as_secs_f64()
            );

            Ok(Self {
                files,
                metadata,
                original_hashes,
                deleted: HashSet::new(),
                root,
                memory_usage,
            })
        }
    }

    /// Load the full repo into a VFS, excluding build/output dirs.
    /// Uses VFS (load_excluding) when the `vfs` feature is enabled,
    /// falling back to scan_dir otherwise.
    pub fn load_excluding<P: AsRef<Path>>(root: P, exclude_dirs: &[&str]) -> Result<Self, String> {
        #[cfg(target_os = "none")]
        {
            let _ = (root, exclude_dirs);
            Err("filesystem loading is unavailable on bare targets".to_string())
        }

        #[cfg(not(target_os = "none"))]
        {
            let root = path_to_path_buf(root.as_ref());
            let mut files = BTreeMap::new();
            let mut metadata = HashMap::new();
            let mut original_hashes = HashMap::new();
            let mut memory_usage = 0usize;

            edgerun_log::info!(
                "Loading filesystem into memory (excluding {:?}): {}",
                exclude_dirs,
                path_buf_display(&root)
            );

            let start = std::time::Instant::now();
            let file_count =
                Self::walk_directory_excluding(&root, exclude_dirs, &mut |path, content| {
                    let rel_path = path
                        .strip_prefix(&root)
                        .map(|p| p.to_path_buf())
                        .unwrap_or_else(|_| path.to_path_buf());

                    let hash = Self::compute_hash(&content);
                    let size = content.len();
                    let modified = Self::get_mtime(path);

                    memory_usage += size;

                    files.insert(rel_path.clone(), FileContent::new(content));
                    metadata.insert(
                        rel_path.clone(),
                        FileMeta {
                            path: rel_path.clone(),
                            size,
                            hash: hash.clone(),
                            modified,
                            is_dirty: false,
                        },
                    );
                    original_hashes.insert(rel_path, hash);
                })?;

            let elapsed = start.elapsed();
            let memory_mb = memory_usage as f64 / (1024.0 * 1024.0);

            edgerun_log::info!(
                "Loaded {} files ({:.2} MB) in {:.2}s",
                file_count,
                memory_mb,
                elapsed.as_secs_f64()
            );

            Ok(Self {
                files,
                metadata,
                original_hashes,
                deleted: HashSet::new(),
                root,
                memory_usage,
            })
        }
    }

    /// Walk directory and load ALL files, skipping excluded directories.
    #[cfg(not(target_os = "none"))]
    fn walk_directory_excluding<F>(
        root: &Path,
        exclude_dirs: &[&str],
        callback: &mut F,
    ) -> Result<usize, String>
    where
        F: FnMut(&PathBuf, Vec<u8>),
    {
        fn walk_impl(
            dir: &Path,
            exclude_dirs: &[&str],
            callback: &mut dyn FnMut(&PathBuf, Vec<u8>),
        ) -> Result<usize, String> {
            let mut local_count = 0;

            for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
                let entry = entry.map_err(|e| e.to_string())?;
                let path = entry.path();

                // Skip excluded directories
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if exclude_dirs.contains(&name) {
                            continue;
                        }
                    }
                    local_count += walk_impl(&path, exclude_dirs, callback)?;
                } else if let Ok(content) = std::fs::read(&path) {
                    callback(&path, content);
                    local_count += 1;
                }
            }

            Ok(local_count)
        }

        let count = walk_impl(root, exclude_dirs, callback)?;
        Ok(count)
    }

    /// Walk directory and load ALL files (no skipping, including binary).
    #[cfg(not(target_os = "none"))]
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

        let count = walk_impl(root, callback)?;
        Ok(count)
    }

    /// Compute SHA1 hash of bytes
    fn compute_hash(content: &[u8]) -> String {
        let mut hash = 0xcbf29ce484222325u64;
        for byte in content {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }

    /// Get file modification time
    #[cfg(not(target_os = "none"))]
    fn get_mtime(path: &Path) -> u64 {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            })
            .unwrap_or(0)
    }

    /// Read file bytes from memory (zero-copy via Arc if possible)
    pub fn read(&self, path: &Path) -> Option<Arc<Vec<u8>>> {
        if self.deleted.contains(path) {
            return None;
        }
        self.files.get(path).map(|fc| fc.data.clone())
    }

    /// Read file as string slice (returns None for binary or deleted files)
    pub fn read_str(&self, path: &Path) -> Option<&str> {
        if self.deleted.contains(path) {
            return None;
        }
        self.files.get(path).and_then(|fc| fc.as_str())
    }

    /// Write bytes to file in memory
    pub fn write_bytes(&mut self, path: &Path, content: Vec<u8>) -> Result<(), String> {
        let path = path_to_path_buf(path);
        self.deleted.remove(&path);

        let size = content.len();
        let old_size = self.files.get(&path).map(|fc| fc.data.len()).unwrap_or(0);

        let hash = Self::compute_hash(&content);

        if let Some(existing) = self.files.get_mut(&path) {
            *existing = FileContent::new(content);
        } else {
            self.files.insert(path.clone(), FileContent::new(content));
        }

        let meta = self
            .metadata
            .entry(path.clone())
            .or_insert_with(|| FileMeta {
                path: path.clone(),
                size: 0,
                hash: String::new(),
                modified: 0,
                is_dirty: true,
            });

        meta.size = size;
        meta.hash = hash;
        meta.modified = now_secs();
        meta.is_dirty = true;

        self.memory_usage = self.memory_usage.saturating_sub(old_size);
        self.memory_usage += size;

        Ok(())
    }

    /// Write string to file in memory (convenience wrapper)
    pub fn write(&mut self, path: &Path, content: String) -> Result<(), String> {
        self.write_bytes(path, content.into_bytes())
    }

    /// Edit text file in memory (returns error for binary files)
    pub fn edit<F>(&mut self, path: &Path, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut String) -> Result<(), String>,
    {
        let path = path_to_path_buf(path);
        if self.deleted.contains(&path) {
            return Err(format!("File {} was deleted", path_buf_display(&path)));
        }

        let file = self
            .files
            .get_mut(&path)
            .ok_or_else(|| format!("File not found: {}", path_buf_display(&path)))?;

        let old_size = file.data.len();
        let mut text = file
            .as_str()
            .ok_or_else(|| format!("Cannot edit binary file: {}", path_buf_display(&path)))?
            .to_string();

        f(&mut text)?;

        let new_content = text.into_bytes();
        let new_hash = Self::compute_hash(&new_content);
        let new_size = new_content.len();

        *file = FileContent::new(new_content);
        if let Some(meta) = self.metadata.get_mut(&path) {
            meta.size = new_size;
            meta.hash = new_hash.clone();
            meta.modified = now_secs();
            meta.is_dirty = true;
        }

        if let Some(original) = self.original_hashes.get(&path) {
            if *original == new_hash {
                if let Some(meta) = self.metadata.get_mut(&path) {
                    meta.is_dirty = false;
                }
            }
        }

        self.memory_usage = self.memory_usage.saturating_sub(old_size);
        self.memory_usage += new_size;

        Ok(())
    }

    /// Iterate over all loaded files as (path, content_bytes).
    pub fn files(&self) -> impl Iterator<Item = (&PathBuf, &[u8])> {
        self.files
            .iter()
            .filter(move |(path, _)| !self.deleted.contains(*path))
            .map(|(path, fc)| (path, &fc.data[..]))
    }

    /// Mark a file as deleted (removed from VFS, but not from disk).
    pub fn delete(&mut self, path: &Path) -> bool {
        let path = path_to_path_buf(path);
        if self.files.contains_key(&path) {
            self.deleted.insert(path);
            true
        } else {
            false
        }
    }

    /// Search for files matching glob pattern
    pub fn glob(&self, pattern: &str) -> Vec<&PathBuf> {
        self.files
            .keys()
            .filter(|path| {
                !self.deleted.contains(*path)
                    && path_as_str(path)
                        .map(|s| edgerun_glob::glob_match(pattern, s))
                        .unwrap_or(false)
            })
            .collect()
    }

    /// Get file count by language (extension)
    pub fn count_by_language(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();

        for (path, _) in self.files() {
            if let Some(ext) = path_extension(path) {
                *counts.entry(ext.to_string()).or_insert(0) += 1;
            }
        }

        counts
    }

    /// Get total lines of code (text files only)
    pub fn total_lines(&self) -> usize {
        self.files
            .values()
            .filter_map(|c| c.as_str())
            .map(|s| s.lines().count())
            .sum()
    }

    /// Find text files containing a specific string (case-insensitive).
    /// Binary files are skipped.
    pub fn find_files_containing(&self, text: &str) -> Vec<&PathBuf> {
        let text_lower = text.to_lowercase();

        self.files
            .iter()
            .filter(|(path, content)| {
                !self.deleted.contains(*path)
                    && content
                        .as_str()
                        .map(|s| s.to_lowercase().contains(&text_lower))
                        .unwrap_or(false)
            })
            .map(|(path, _)| path)
            .collect()
    }

    /// Persist dirty files back to disk.
    /// Returns count of files persisted.
    #[cfg(not(target_os = "none"))]
    pub fn persist(&self, base_path: &Path) -> Result<PersistResult, String> {
        let mut persisted = 0usize;
        let mut errors = Vec::new();

        for (path, meta) in &self.metadata {
            if !meta.is_dirty {
                continue;
            }
            if self.deleted.contains(path) {
                continue;
            }
            if let Some(content) = self.files.get(path) {
                let full_path = base_path.join(path);
                if let Some(parent) = full_path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                if let Err(e) = std::fs::write(&full_path, &*content.data) {
                    errors.push(format!("Failed to write {}: {}", path.display(), e));
                } else {
                    persisted += 1;
                }
            }
        }

        if !errors.is_empty() {
            return Err(errors.join("; "));
        }

        Ok(PersistResult { persisted })
    }
}

/// Result of a persist operation.
pub struct PersistResult {
    pub persisted: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::tempdir;
    use std::path::Path;

    #[test]
    fn test_load_vfs() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(tmp.path().join("file2.txt"), "content2").unwrap();

        let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        assert_eq!(vfs.files().count(), 2);
        assert_eq!(vfs.read_str(Path::new("file1.txt")), Some("content1"));
        assert_eq!(vfs.read_str(Path::new("file2.txt")), Some("content2"));
    }

    #[test]
    fn test_load_binary_file() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("binary.dat"), vec![0u8, 159, 146, 150]).unwrap();
        std::fs::write(tmp.path().join("text.txt"), "hello").unwrap();

        let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        assert_eq!(vfs.files().count(), 2);

        let bytes = vfs.read(Path::new("binary.dat")).unwrap();
        assert_eq!(&*bytes, &[0u8, 159, 146, 150]);

        // binary file should not return a string
        assert!(vfs.read_str(Path::new("binary.dat")).is_none());
        assert_eq!(vfs.read_str(Path::new("text.txt")), Some("hello"));
    }

    #[test]
    fn test_read_write() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("test.txt"), "initial").unwrap();

        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        assert_eq!(vfs.read_str(Path::new("test.txt")), Some("initial"));

        vfs.write(Path::new("test.txt"), "modified".to_string())
            .unwrap();
        assert_eq!(vfs.read_str(Path::new("test.txt")), Some("modified"));

        vfs.write_bytes(Path::new("test.txt"), vec![0u8, 159, 146, 150])
            .unwrap();
        assert!(vfs.read_str(Path::new("test.txt")).is_none());
        assert_eq!(
            &*vfs.read(Path::new("test.txt")).unwrap(),
            &[0u8, 159, 146, 150]
        );
    }

    #[test]
    fn test_edit() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("test.txt"), "line1\nline2\n").unwrap();

        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        vfs.edit(Path::new("test.txt"), |content| {
            content.push_str("line3\n");
            Ok(())
        })
        .unwrap();

        assert_eq!(
            vfs.read_str(Path::new("test.txt")),
            Some("line1\nline2\nline3\n")
        );
    }

    #[test]
    fn test_edit_binary_file_fails() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("binary.dat"), vec![0u8, 255, 128]).unwrap();

        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        let result = vfs.edit(Path::new("binary.dat"), |_c| Ok(()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("binary"));
    }

    #[test]
    fn test_delete_and_recover() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("test.txt"), "content").unwrap();

        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        assert!(vfs.read_str(Path::new("test.txt")).is_some());

        vfs.delete(Path::new("test.txt"));
        assert!(vfs.read_str(Path::new("test.txt")).is_none());
        // deleted files should not appear in files()
        assert_eq!(vfs.files().count(), 0);
    }

    #[test]
    fn test_metadata() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("test.txt"), "content").unwrap();

        let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        let meta = vfs.metadata.get(Path::new("test.txt")).unwrap();
        assert_eq!(meta.size, 7);
        assert!(!meta.is_dirty);
    }

    #[test]
    fn test_dirty_tracking() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("test.txt"), "initial").unwrap();

        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        assert!(!vfs.metadata.values().any(|m| m.is_dirty));

        vfs.write(Path::new("test.txt"), "modified".to_string())
            .unwrap();
        assert!(vfs.metadata.get(Path::new("test.txt")).unwrap().is_dirty);
    }

    #[test]
    fn test_persist() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("test.txt"), "initial").unwrap();

        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        vfs.write(Path::new("test.txt"), "modified".to_string())
            .unwrap();

        let result = vfs.persist(tmp.path()).unwrap();
        assert_eq!(result.persisted, 1);

        let disk_content = std::fs::read(tmp.path().join("test.txt")).unwrap();
        assert_eq!(disk_content, b"modified");
    }

    #[test]
    fn test_glob() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("file1.txt"), "content").unwrap();
        std::fs::write(tmp.path().join("file2.txt"), "content").unwrap();
        std::fs::write(tmp.path().join("file3.md"), "content").unwrap();

        let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        let matches = vfs.glob("*.txt");
        assert_eq!(matches.len(), 2);

        let matches = vfs.glob("*.md");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_count_by_language() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("file1.rs"), "fn main() {}").unwrap();
        std::fs::write(tmp.path().join("file2.rs"), "fn foo() {}").unwrap();
        std::fs::write(tmp.path().join("file3.js"), "function bar() {}").unwrap();

        let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        let counts = vfs.count_by_language();
        assert_eq!(*counts.get("rs").unwrap(), 2);
        assert_eq!(*counts.get("js").unwrap(), 1);
    }

    #[test]
    fn test_edit_nonexistent_file() {
        let tmp = tempdir().unwrap();
        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();

        let result = vfs.edit(Path::new("nonexistent.txt"), |_c| Ok(()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_memory_usage() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("test.txt"), "content").unwrap();

        let vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        assert!(vfs.memory_usage > 0);
    }

    #[test]
    fn test_changes() {
        let tmp = tempdir().unwrap();
        std::fs::write(tmp.path().join("test.txt"), "initial").unwrap();

        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        assert!(!vfs.metadata.values().any(|m| m.is_dirty));

        vfs.write(Path::new("test.txt"), "modified".to_string())
            .unwrap();
        assert!(
            vfs.metadata
                .get(&PathBuf::from("test.txt"))
                .unwrap()
                .is_dirty
        );
    }
}
