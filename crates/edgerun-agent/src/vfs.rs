//! In-memory virtual filesystem with copy-on-write semantics.
//!
//! All files are loaded into RAM on initialization, including binary files.
//! Changes are tracked with copy-on-write and persisted only on explicit commits.
//!
//! Designed for 64GB RAM systems - can easily handle 100k+ file codebases.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use edgerun_rt::sync::RwLock;

/// File metadata
#[derive(Debug, Clone)]
pub struct FileMeta {
    pub path: PathBuf,
    pub size: usize,
    pub hash: String,
    pub modified: u64,
    pub is_dirty: bool,
}

/// File content with copy-on-write semantics.
/// Stores bytes to handle both text and binary files.
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
        std::str::from_utf8(&self.data).ok()
    }

    /// Check if content is valid UTF-8 text
    pub fn is_text(&self) -> bool {
        std::str::from_utf8(&self.data).is_ok()
    }

    /// Check if content is shared (COW)
    pub fn is_shared(&self) -> bool {
        Arc::strong_count(&self.data) > 1
    }

    pub fn version(&self) -> u64 {
        self.version
    }
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
    /// Loads ALL files including binary; no directories are skipped.
    pub fn load<P: AsRef<Path>>(root: P) -> Result<Self, String> {
        let root = root.as_ref().to_path_buf();
        let mut files = BTreeMap::new();
        let mut metadata = HashMap::new();
        let mut original_hashes = HashMap::new();
        let mut memory_usage = 0usize;

        println!("Loading filesystem into memory: {}", root.display());
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

        println!(
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

    /// Walk directory and load ALL files (no skipping, including binary).
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
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        hasher.update(content);
        format!("{:x}", hasher.finalize())
    }

    /// Get file modification time
    fn get_mtime(path: &Path) -> u64 {
        std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map(|t| t.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs())
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
        let path = path.to_path_buf();
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
        meta.modified = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        meta.is_dirty = true;

        self.memory_usage = self.memory_usage.saturating_sub(old_size);
        self.memory_usage += size;

        Ok(())
    }

    /// Write string to file (convenience wrapper)
    pub fn write(&mut self, path: &Path, content: String) -> Result<(), String> {
        self.write_bytes(path, content.into_bytes())
    }

    /// Edit text file in memory (returns error for binary files)
    pub fn edit<F>(&mut self, path: &Path, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut String) -> Result<(), String>,
    {
        let path = path.to_path_buf();

        if self.deleted.contains(&path) {
            return Err(format!("File {:?} was deleted", path));
        }

        let file = self
            .files
            .get(&path)
            .ok_or_else(|| format!("File not found: {:?}", path))?;

        let old_size = file.data.len();
        let mut text = String::from_utf8(file.data.to_vec())
            .map_err(|_| format!("Cannot edit binary file: {:?}", path))?;

        let _ = file;
        f(&mut text)?;

        let new_size = text.len();
        let new_data = text.into_bytes();
        let hash = Self::compute_hash(&new_data);

        if let Some(existing) = self.files.get_mut(&path) {
            existing.data = Arc::new(new_data);
            existing.version += 1;
        }

        if let Some(meta) = self.metadata.get_mut(&path) {
            meta.size = new_size;
            meta.hash = hash;
            meta.modified = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            meta.is_dirty = true;
        }

        self.memory_usage = self.memory_usage.saturating_sub(old_size);
        self.memory_usage += new_size;

        Ok(())
    }

    /// Delete file (tracked until commit)
    pub fn delete(&mut self, path: &Path) -> bool {
        let path = path.to_path_buf();
        if self.files.contains_key(&path) {
            self.deleted.insert(path);
            true
        } else {
            false
        }
    }

    /// Check if file exists (not deleted)
    pub fn exists(&self, path: &Path) -> bool {
        self.files.contains_key(path) && !self.deleted.contains(path)
    }

    /// Check if file is text (valid UTF-8)
    pub fn is_text(&self, path: &Path) -> bool {
        self.files.get(path).map(|fc| fc.is_text()).unwrap_or(false)
    }

    /// Get file metadata
    pub fn metadata(&self, path: &Path) -> Option<&FileMeta> {
        self.metadata.get(path)
    }

    /// List all files
    pub fn files(&self) -> impl Iterator<Item = &PathBuf> {
        self.files.keys().filter(|p| !self.deleted.contains(*p))
    }

    /// Get all dirty files (need persistence)
    pub fn dirty_files(&self) -> impl Iterator<Item = &PathBuf> {
        self.metadata
            .iter()
            .filter(|(_, meta)| meta.is_dirty)
            .map(|(path, _)| path)
    }

    /// Get memory usage statistics
    pub fn memory_stats(&self) -> MemoryStats {
        let file_count = self.files.len();
        let dirty_count = self.dirty_files().count();
        let deleted_count = self.deleted.len();

        MemoryStats {
            file_count,
            dirty_count,
            deleted_count,
            memory_bytes: self.memory_usage,
            memory_mb: self.memory_usage as f64 / (1024.0 * 1024.0),
            avg_file_size: if file_count > 0 {
                self.memory_usage / file_count
            } else {
                0
            },
        }
    }

    /// Persist dirty files to disk (writes bytes, preserving binary content)
    pub fn persist(&mut self) -> Result<PersistResult, String> {
        let mut persisted = 0;
        let mut deleted = 0;
        let mut errors = Vec::new();

        for path in self.dirty_files().cloned().collect::<Vec<_>>() {
            if let Some(content) = self.files.get(&path) {
                let full_path = self.root.join(&path);

                if let Some(parent) = full_path.parent() {
                    if let Err(e) = std::fs::create_dir_all(parent) {
                        errors.push(format!("Failed to create directory {:?}: {}", parent, e));
                        continue;
                    }
                }

                match std::fs::write(&full_path, content.as_bytes()) {
                    Ok(_) => {
                        persisted += 1;
                        if let Some(meta) = self.metadata.get_mut(&path) {
                            meta.is_dirty = false;
                        }
                    }
                    Err(e) => {
                        errors.push(format!("Failed to write {:?}: {}", path, e));
                    }
                }
            }
        }

        for path in self.deleted.drain() {
            let full_path = self.root.join(&path);
            match std::fs::remove_file(&full_path) {
                Ok(_) => {
                    deleted += 1;
                    self.files.remove(&path);
                    self.metadata.remove(&path);
                    self.original_hashes.remove(&path);
                }
                Err(e) => {
                    errors.push(format!("Failed to delete {:?}: {}", path, e));
                }
            }
        }

        Ok(PersistResult {
            persisted,
            deleted,
            errors,
        })
    }

    /// Get changed files since load
    pub fn changes(&self) -> Changeset {
        let mut added = Vec::new();
        let mut modified = Vec::new();
        let removed: Vec<&PathBuf> = self.deleted.iter().collect();

        for (path, meta) in &self.metadata {
            if let Some(original_hash) = self.original_hashes.get(path) {
                if original_hash != &meta.hash {
                    modified.push(path);
                }
            } else {
                added.push(path);
            }
        }

        Changeset {
            added,
            modified,
            removed,
        }
    }

    /// Get root directory
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Search for pattern in text files (parallel, in-memory).
    /// Binary files are automatically skipped.
    pub fn grep(&self, pattern: &str) -> Vec<GrepMatch> {
        use rayon::prelude::*;
        use regex::Regex;

        let regex = match Regex::new(pattern) {
            Ok(r) => r,
            Err(_) => return Vec::new(),
        };

        self.files
            .par_iter()
            .filter(|(path, _)| !self.deleted.contains(*path))
            .filter_map(|(path, content)| {
                let text = content.as_str()?;
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

    /// Search for files matching glob pattern
    pub fn glob(&self, pattern: &str) -> Vec<&PathBuf> {
        let glob_pattern = match glob::Pattern::new(pattern) {
            Ok(p) => p,
            Err(_) => return Vec::new(),
        };

        self.files
            .keys()
            .filter(|path| {
                !self.deleted.contains(*path)
                    && path
                        .to_str()
                        .map(|s| glob_pattern.matches(s))
                        .unwrap_or(false)
            })
            .collect()
    }

    /// Get file count by language (extension)
    pub fn count_by_language(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();

        for path in self.files() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
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

/// A single grep match result
#[derive(Debug, Clone)]
pub struct GrepMatch {
    pub path: PathBuf,
    pub line_number: usize,
    pub line: String,
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub file_count: usize,
    pub dirty_count: usize,
    pub deleted_count: usize,
    pub memory_bytes: usize,
    pub memory_mb: f64,
    pub avg_file_size: usize,
}

impl std::fmt::Display for MemoryStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Memory Statistics:")?;
        writeln!(f, "  Files: {}", self.file_count)?;
        writeln!(f, "  Dirty files: {}", self.dirty_count)?;
        writeln!(f, "  Deleted files: {}", self.deleted_count)?;
        writeln!(f, "  Memory usage: {:.2} MB", self.memory_mb)?;
        writeln!(f, "  Avg file size: {} bytes", self.avg_file_size)?;
        Ok(())
    }
}

/// Persist operation result
#[derive(Debug)]
pub struct PersistResult {
    pub persisted: usize,
    pub deleted: usize,
    pub errors: Vec<String>,
}

/// Changeset describing modifications
#[derive(Debug)]
pub struct Changeset<'a> {
    pub added: Vec<&'a PathBuf>,
    pub modified: Vec<&'a PathBuf>,
    pub removed: Vec<&'a PathBuf>,
}

/// Thread-safe wrapper for VFS
pub type SharedVFS = Arc<RwLock<VirtualFileSystem>>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_dir() -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("test.rs"), "fn main() {}").unwrap();
        fs::write(tmp.path().join("lib.rs"), "pub mod test;").unwrap();
        tmp
    }

    #[test]
    fn test_load_vfs() {
        let tmp = create_test_dir();
        let vfs = VirtualFileSystem::load(tmp.path()).unwrap();

        assert_eq!(vfs.files.len(), 2);
        assert!(vfs.read_str(Path::new("test.rs")).is_some());
    }

    #[test]
    fn test_read_write() {
        let tmp = create_test_dir();
        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();

        // Read
        let content = vfs.read_str(Path::new("test.rs")).unwrap();
        assert_eq!(content, "fn main() {}");

        // Write string
        vfs.write(
            Path::new("test.rs"),
            "fn main() { println!(\"Hi\"); }".to_string(),
        )
        .unwrap();

        let content = vfs.read_str(Path::new("test.rs")).unwrap();
        assert_eq!(content, "fn main() { println!(\"Hi\"); }");

        // Write bytes
        vfs.write_bytes(Path::new("test.rs"), b"binary data".to_vec())
            .unwrap();
        let bytes = vfs.read(Path::new("test.rs")).unwrap();
        assert_eq!(&*bytes, b"binary data");
    }

    #[test]
    fn test_binary_file() {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("data.bin"), vec![0u8, 255, 128]).unwrap();

        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();
        assert_eq!(vfs.files.len(), 1);

        let bytes = vfs.read(Path::new("data.bin")).unwrap();
        assert_eq!(&*bytes, &[0u8, 255, 128]);

        assert!(!vfs.is_text(Path::new("data.bin")));
        assert!(vfs.read_str(Path::new("data.bin")).is_none());

        let result = vfs.edit(Path::new("data.bin"), |_| Ok(()));
        assert!(result.is_err());
    }

    #[test]
    fn test_copy_on_write() {
        let tmp = create_test_dir();
        let mut vfs = VirtualFileSystem::load(tmp.path()).unwrap();

        // Get shared reference
        let content1 = vfs.read(Path::new("test.rs")).unwrap();

        // Edit (should create new Arc)
        vfs.edit(Path::new("test.rs"), |s| {
            s.push_str("\n// Comment");
            Ok(())
        })
        .unwrap();

        // Original reference should be unchanged
        assert_eq!(String::from_utf8_lossy(&content1), "fn main() {}");

        // New content should have edit
        let content2 = vfs.read_str(Path::new("test.rs")).unwrap();
        assert!(content2.contains("// Comment"));
    }

    #[test]
    fn test_memory_stats() {
        let tmp = create_test_dir();
        let vfs = VirtualFileSystem::load(tmp.path()).unwrap();

        let stats = vfs.memory_stats();
        assert!(stats.file_count >= 2);
        assert!(stats.memory_mb > 0.0);
    }
}
