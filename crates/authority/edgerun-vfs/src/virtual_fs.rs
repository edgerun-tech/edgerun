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

use super::{Path, PathBuf, hash_hex};

type HashMap<K, V> = BTreeMap<K, V>;
type HashSet<T> = BTreeSet<T>;

pub(crate) fn path_display(path: &Path) -> String {
    path.to_string()
}

fn path_buf_display(path: &PathBuf) -> String {
    path.clone()
}

fn path_extension(path: &PathBuf) -> Option<&str> {
    path.rsplit('/')
        .next()?
        .rsplit_once('.')
        .map(|(_, ext)| ext)
}

fn path_as_str(path: &PathBuf) -> Option<&str> {
    Some(path.as_str())
}

fn now_secs() -> u64 {
    0
}

fn normalize_relative_path(path: &str) -> Result<String, String> {
    let path = path.trim_matches('/');
    if path.is_empty() {
        return Err("path must not be empty".to_string());
    }
    if path.starts_with('\\') || path.contains('\\') {
        return Err("path must use forward slashes".to_string());
    }
    if path
        .split('/')
        .any(|part| part.is_empty() || part == "." || part == "..")
    {
        return Err("path must be normalized and relative".to_string());
    }
    Ok(path.to_string())
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
    /// Create an empty byte-fed VFS.
    pub fn empty(root: impl Into<String>) -> Self {
        Self {
            files: BTreeMap::new(),
            metadata: HashMap::new(),
            original_hashes: HashMap::new(),
            deleted: HashSet::new(),
            root: root.into(),
            memory_usage: 0,
        }
    }

    /// Create a VFS from already-loaded bytes.
    pub fn from_entries<I, P>(entries: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = (P, Vec<u8>)>,
        P: Into<String>,
    {
        Self::from_entries_with_root("", entries)
    }

    /// Create a VFS from already-loaded bytes and a source label/root.
    pub fn from_entries_with_root<I, P>(root: impl Into<String>, entries: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = (P, Vec<u8>)>,
        P: Into<String>,
    {
        let mut vfs = Self::empty(root);
        for (path, content) in entries {
            vfs.insert_clean(path.into(), content)?;
        }
        Ok(vfs)
    }

    fn insert_clean(&mut self, path: String, content: Vec<u8>) -> Result<(), String> {
        let path = normalize_relative_path(&path)?;
        let hash = Self::compute_hash(&content);
        let size = content.len();
        self.memory_usage = self.memory_usage.saturating_add(size);
        self.files.insert(path.clone(), FileContent::new(content));
        self.metadata.insert(
            path.clone(),
            FileMeta {
                path: path.clone(),
                size,
                hash: hash.clone(),
                modified: 0,
                is_dirty: false,
            },
        );
        self.original_hashes.insert(path, hash);
        Ok(())
    }

    /// Compute domain-separated BLAKE3 hash of bytes.
    fn compute_hash(content: &[u8]) -> String {
        hash_hex(&crate::packet::vfs_object_id(content))
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
        let path = normalize_relative_path(path)?;
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
        let path = normalize_relative_path(path)?;
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
        let Ok(path) = normalize_relative_path(path) else {
            return false;
        };
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
                        .map(|s| crate::glob::glob_match(pattern, s))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_vfs() -> VirtualFileSystem {
        VirtualFileSystem::from_entries([
            ("file1.txt", b"content1".to_vec()),
            ("file2.txt", b"content2".to_vec()),
            ("src/main.rs", b"fn main() {}\n".to_vec()),
            ("src/lib.rs", b"fn foo() {}\n".to_vec()),
            ("assets/binary.dat", vec![0u8, 159, 146, 150]),
        ])
        .expect("sample vfs")
    }

    #[test]
    fn test_from_entries_loads_bytes() {
        let vfs = sample_vfs();
        assert_eq!(vfs.files().count(), 5);
        assert_eq!(vfs.read_str("file1.txt"), Some("content1"));
        assert_eq!(vfs.read_str("file2.txt"), Some("content2"));
    }

    #[test]
    fn test_binary_file() {
        let vfs = sample_vfs();
        let bytes = vfs.read("assets/binary.dat").unwrap();
        assert_eq!(&*bytes, &[0u8, 159, 146, 150]);
        assert!(vfs.read_str("assets/binary.dat").is_none());
    }

    #[test]
    fn test_read_write() {
        let mut vfs =
            VirtualFileSystem::from_entries([("test.txt", b"initial".to_vec())]).expect("vfs");
        assert_eq!(vfs.read_str("test.txt"), Some("initial"));

        vfs.write("test.txt", "modified".to_string()).unwrap();
        assert_eq!(vfs.read_str("test.txt"), Some("modified"));

        vfs.write_bytes("test.txt", vec![0u8, 159, 146, 150])
            .unwrap();
        assert!(vfs.read_str("test.txt").is_none());
        assert_eq!(&*vfs.read("test.txt").unwrap(), &[0u8, 159, 146, 150]);
    }

    #[test]
    fn test_edit() {
        let mut vfs = VirtualFileSystem::from_entries([("test.txt", b"line1\nline2\n".to_vec())])
            .expect("vfs");
        vfs.edit("test.txt", |content| {
            content.push_str("line3\n");
            Ok(())
        })
        .unwrap();

        assert_eq!(vfs.read_str("test.txt"), Some("line1\nline2\nline3\n"));
    }

    #[test]
    fn test_edit_binary_file_fails() {
        let mut vfs =
            VirtualFileSystem::from_entries([("binary.dat", vec![0u8, 255, 128])]).expect("vfs");
        let result = vfs.edit("binary.dat", |_c| Ok(()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("binary"));
    }

    #[test]
    fn test_delete_and_recover() {
        let mut vfs =
            VirtualFileSystem::from_entries([("test.txt", b"content".to_vec())]).expect("vfs");
        assert!(vfs.read_str("test.txt").is_some());

        vfs.delete("test.txt");
        assert!(vfs.read_str("test.txt").is_none());
        assert_eq!(vfs.files().count(), 0);
    }

    #[test]
    fn test_metadata() {
        let vfs =
            VirtualFileSystem::from_entries([("test.txt", b"content".to_vec())]).expect("vfs");
        let meta = vfs.metadata.get("test.txt").unwrap();
        assert_eq!(meta.size, 7);
        assert!(!meta.is_dirty);
    }

    #[test]
    fn test_dirty_tracking() {
        let mut vfs =
            VirtualFileSystem::from_entries([("test.txt", b"initial".to_vec())]).expect("vfs");
        assert!(!vfs.metadata.values().any(|m| m.is_dirty));

        vfs.write("test.txt", "modified".to_string()).unwrap();
        assert!(vfs.metadata.get("test.txt").unwrap().is_dirty);
    }

    #[test]
    fn test_glob() {
        let vfs = VirtualFileSystem::from_entries([
            ("file1.txt", b"content".to_vec()),
            ("file2.txt", b"content".to_vec()),
            ("file3.md", b"content".to_vec()),
        ])
        .expect("vfs");
        let matches = vfs.glob("*.txt");
        assert_eq!(matches.len(), 2);

        let matches = vfs.glob("*.md");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_count_by_language() {
        let vfs = sample_vfs();
        let counts = vfs.count_by_language();
        assert_eq!(*counts.get("rs").unwrap(), 2);
    }

    #[test]
    fn test_edit_nonexistent_file() {
        let mut vfs = VirtualFileSystem::empty("");

        let result = vfs.edit("nonexistent.txt", |_c| Ok(()));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_memory_usage() {
        let vfs = sample_vfs();
        assert!(vfs.memory_usage > 0);
    }

    #[test]
    fn test_changes() {
        let mut vfs =
            VirtualFileSystem::from_entries([("test.txt", b"initial".to_vec())]).expect("vfs");
        assert!(!vfs.metadata.values().any(|m| m.is_dirty));

        vfs.write("test.txt", "modified".to_string()).unwrap();
        assert!(vfs.metadata.get("test.txt").unwrap().is_dirty);
    }

    #[test]
    fn rejects_non_normalized_paths() {
        assert!(VirtualFileSystem::from_entries([("../x", b"bad".to_vec())]).is_err());
        assert!(VirtualFileSystem::from_entries([("a//b", b"bad".to_vec())]).is_err());
        assert!(VirtualFileSystem::from_entries([("a\\b", b"bad".to_vec())]).is_err());
    }
}
