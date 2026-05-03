/// Filesystem tracking layer.
///
/// Scans directories for tracked source files, computes a fast content hash,
/// and diffs snapshots to detect new / modified / deleted files.
use std::collections::HashMap;
use std::{fs, path::Path, time::UNIX_EPOCH};

/// Extensions we care about.
const TRACKED_EXTS: &[&str] = &[".c", ".h", ".rs", ".ts", ".js", ".py", ".go", ".java"];

/// Metadata about a single tracked source file.
#[derive(Debug, Clone)]
pub struct FileInfo {
    pub path: String, // relative to scan root
    #[allow(dead_code)]
    pub language: String, // "c", "rust", "typescript", "javascript"
    #[allow(dead_code)]
    pub size: u64,
    #[allow(dead_code)]
    pub modified_ts: u64, // seconds since epoch
    #[allow(dead_code)]
    pub hash: u64, // fast rolling hash of file contents
}

/// Result of comparing two filesystem snapshots.
#[derive(Debug, Default)]
#[allow(dead_code)]
pub struct FileChanges {
    pub added: Vec<FileInfo>,
    pub modified: Vec<FileInfo>,
    pub deleted: Vec<FileInfo>,
    pub unchanged: Vec<FileInfo>,
}

impl FileChanges {
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.modified.is_empty() && self.deleted.is_empty()
    }

    /// All files that need re-parsing (added + modified).
    #[allow(dead_code)]
    pub fn changed_paths(&self) -> Vec<&str> {
        self.added.iter().chain(self.modified.iter()).map(|f| f.path.as_str()).collect()
    }
}

/// Recursively scan a directory for tracked source files.
/// Returns FileInfo with paths relative to `root`.
pub fn scan_dir(root: &str) -> Vec<FileInfo> {
    let mut results = Vec::new();
    let root_path = Path::new(root);
    scan_recursive(root_path, root_path, &mut results);
    results
}

fn scan_recursive(root: &Path, current: &Path, out: &mut Vec<FileInfo>) {
    let Ok(entries) = fs::read_dir(current) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        // Skip noise
        if name.starts_with('.') || name == "target" || name == "node_modules" || name == ".git" {
            continue;
        }

        if path.is_dir() {
            scan_recursive(root, &path, out);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let dot_ext = format!(".{}", ext);
            if TRACKED_EXTS.contains(&dot_ext.as_str()) {
                if let Some(info) = file_info(&path, root) {
                    out.push(info);
                }
            }
        }
    }
}

fn file_info(path: &Path, root: &Path) -> Option<FileInfo> {
    let metadata = fs::metadata(path).ok()?;
    let size = metadata.len();

    let modified_ts = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?.as_secs();

    let content = fs::read_to_string(path).ok()?;
    let hash = rolling_hash(content.as_bytes());

    let language = lang_from_ext(path.extension()?.to_str()?);
    let rel = make_relative(path, root);

    Some(FileInfo { path: rel, language: language.to_string(), size, modified_ts, hash })
}

/// Diff two snapshots. Uses path as the key.
#[allow(dead_code)]
pub fn diff_files(old: &[FileInfo], new: &[FileInfo]) -> FileChanges {
    let old_map: HashMap<&str, &FileInfo> = old.iter().map(|f| (f.path.as_str(), f)).collect();
    let new_map: HashMap<&str, &FileInfo> = new.iter().map(|f| (f.path.as_str(), f)).collect();

    let mut changes = FileChanges::default();

    // Detect added and modified
    for (path, &new_info) in &new_map {
        if let Some(&old_info) = old_map.get(path) {
            if old_info.hash != new_info.hash {
                changes.modified.push(new_info.clone());
            } else {
                changes.unchanged.push(new_info.clone());
            }
        } else {
            changes.added.push(new_info.clone());
        }
    }

    // Detect deleted
    for (path, &old_info) in &old_map {
        if !new_map.contains_key(path) {
            changes.deleted.push(old_info.clone());
        }
    }

    changes
}

/// Fast non-cryptographic rolling hash (FNV-1a variant on bytes).
/// Good enough for content-change detection; very fast.
pub fn rolling_hash(data: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325; // FNV offset basis
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3); // FNV prime
    }
    hash
}

fn lang_from_ext(ext: &str) -> &'static str {
    match ext {
        "c" | "h" => "c",
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        _ => "unknown",
    }
}

fn make_relative(path: &Path, root: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

// ─── Parse Cache ──────────────────────────────────────────────────────

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A cache entry for a single parsed file.
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheEntry {
    pub file_hash: u64,
    pub mtime: u64,
    pub functions: Vec<crate::parser::RawFunctionOwned>,
    pub calls: Vec<crate::parser::RawCallOwned>,
}

/// Persistent parse cache that stores parsed results on disk.
///
/// The cache avoids re-parsing files whose content hash and mtime
/// have not changed since the last parse.
pub struct ParseCache {
    root_dir: String,
    entries: HashMap<String, CacheEntry>,
    dirty: bool,
}

impl ParseCache {
    /// Load the cache from disk. The cache file is stored under
    /// ~/.cache/codeanalyzer/<hash(root_dir)>.json.
    pub fn load(root_dir: &str) -> Self {
        let cache_dir = cache_dir();
        let cache_file = cache_file_path(&cache_dir, root_dir);

        let entries = if cache_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&cache_file) {
                serde_json::from_str(&content).unwrap_or_default()
            } else {
                HashMap::new()
            }
        } else {
            HashMap::new()
        };

        ParseCache { root_dir: root_dir.to_string(), entries, dirty: false }
    }

    /// Save the cache to disk if it has been modified.
    pub fn save(&mut self) {
        if !self.dirty {
            return;
        }
        let cache_dir = cache_dir();
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            eprintln!("warn: failed to create cache dir: {e}");
            return;
        }
        let cache_file = cache_file_path(&cache_dir, &self.root_dir);
        if let Ok(content) = serde_json::to_string(&self.entries) {
            if let Err(e) = std::fs::write(&cache_file, content) {
                eprintln!("warn: failed to write cache: {e}");
            }
        }
        self.dirty = false;
    }

    /// Get a cached entry if the file hash and mtime match.
    #[allow(dead_code)]
    pub fn get(&self, path: &str, current_hash: u64, current_mtime: u64) -> Option<&CacheEntry> {
        let entry = self.entries.get(path)?;
        if entry.file_hash == current_hash && entry.mtime == current_mtime {
            Some(entry)
        } else {
            None
        }
    }

    /// Insert a parsed result into the cache.
    pub fn insert(
        &mut self,
        path: String,
        hash: u64,
        mtime: u64,
        result: crate::parser::CachedParseResult,
    ) {
        self.entries.insert(
            path,
            CacheEntry { file_hash: hash, mtime, functions: result.functions, calls: result.calls },
        );
        self.dirty = true;
    }
}

/// Return the cache directory, creating it if needed.
fn cache_dir() -> PathBuf {
    dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("codeanalyzer")
}

/// Compute the cache file path for a given root directory.
fn cache_file_path(cache_dir: &Path, root_dir: &str) -> PathBuf {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };
    let mut hasher = DefaultHasher::new();
    root_dir.hash(&mut hasher);
    let hash = hasher.finish();
    cache_dir.join(format!("{hash:x}.json"))
}
