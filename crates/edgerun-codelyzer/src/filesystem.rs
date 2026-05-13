#![allow(dead_code)]

/// Filesystem tracking layer.
///
/// Scans directories for tracked source files, computes a fast content hash,
/// and diffs snapshots to detect new / modified / deleted files.
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    time::SystemTime,
};

/// Extensions we care about.
const TRACKED_EXTS: &[&str] = &[
    ".c", ".h", ".rs", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".py", ".go", ".java",
];

const IGNORED_DIRS: &[&str] = &[
    ".git",
    ".next",
    ".turbo",
    "build",
    "coverage",
    "dist",
    "node_modules",
    "out",
    "target",
    "third_party",
    "vendor",
];

const IGNORED_GENERATED_PREFIXES: &[&str] = &["zerrors_", "zsyscall_", "zsysnum_", "ztypes_"];

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
        self.added
            .iter()
            .chain(self.modified.iter())
            .map(|f| f.path.as_str())
            .collect()
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
    let Ok(entries) = std::fs::read_dir(current) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if should_skip_dir_or_file(name) {
            continue;
        }

        if path.is_dir() {
            scan_recursive(root, &path, out);
        } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let dot_ext = format!(".{ext}");
            if TRACKED_EXTS.contains(&dot_ext.as_str()) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .to_string();
                let metadata = path.metadata().ok();
                let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
                let modified_ts = metadata
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let hash = compute_file_hash(&path);
                out.push(FileInfo {
                    path: rel,
                    language: ext_to_lang(ext),
                    size,
                    modified_ts,
                    hash,
                });
            }
        }
    }
}

fn should_skip_dir_or_file(name: &str) -> bool {
    if IGNORED_DIRS.contains(&name) {
        return true;
    }
    if IGNORED_GENERATED_PREFIXES
        .iter()
        .any(|p| name.starts_with(p))
    {
        return true;
    }
    false
}

fn ext_to_lang(ext: &str) -> String {
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
    .to_string()
}

fn compute_file_hash(path: &Path) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    if let Ok(bytes) = std::fs::read(path) {
        bytes.hash(&mut hasher);
    }
    hasher.finish()
}

pub fn rolling_hash(bytes: &[u8]) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    hasher.finish()
}

#[cfg(feature = "vfs")]
pub fn load_vfs(root_dir: &str) -> Result<crate::vfs::SharedVFS, String> {
    let vfs = crate::vfs::VirtualFileSystem::load_excluding(root_dir, IGNORED_DIRS)?;
    Ok(std::sync::Arc::new(std::sync::RwLock::new(vfs)))
}

// ─── Parse Cache ──────────────────────────────────────────────

/// A cache entry for a single parsed file.
#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize)]
pub struct CacheEntry {
    pub file_hash: u64,
    pub mtime: u64,
    pub functions: Vec<crate::parser::RawFunctionOwned>,
    pub calls: Vec<crate::parser::RawCallOwned>,
}

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize)]
struct ParseCacheEntries {
    entries: HashMap<String, CacheEntry>,
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
    /// <cache_dir>/<hash(root_dir)>.bin.
    pub fn load(root_dir: &str) -> Self {
        let cache_dir = cache_dir();
        let cache_file = cache_file_path(&cache_dir, root_dir);

        let entries = if cache_file.exists() {
            std::fs::read(&cache_file)
                .ok()
                .and_then(|bytes| {
                    let archived =
                        rkyv::access::<ArchivedParseCacheEntries, rkyv::rancor::Error>(&bytes)
                            .ok()?;
                    rkyv::deserialize::<ParseCacheEntries, rkyv::rancor::Error>(archived).ok()
                })
                .map(|cache| cache.entries)
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        ParseCache {
            root_dir: root_dir.to_string(),
            entries,
            dirty: false,
        }
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
        let cache = ParseCacheEntries {
            entries: self.entries.clone(),
        };
        let bytes = match rkyv::to_bytes::<rkyv::rancor::Error>(&cache) {
            Ok(bytes) => bytes,
            Err(e) => {
                eprintln!("warn: failed to encode rkyv cache: {e}");
                return;
            }
        };
        if let Err(e) = std::fs::write(&cache_file, bytes) {
            eprintln!("warn: failed to write cache: {e}");
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
            CacheEntry {
                file_hash: hash,
                mtime,
                functions: result.functions,
                calls: result.calls,
            },
        );
        self.dirty = true;
    }
}

/// Return the cache directory, creating it if needed.
fn cache_dir() -> PathBuf {
    std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".cache"))
                .unwrap_or_else(|_| PathBuf::from("/tmp"))
        })
        .join("codeanalyzer")
}

/// Compute the cache file path for a given root directory.
fn cache_file_path(cache_dir: &Path, root_dir: &str) -> PathBuf {
    use std::collections::hash_map::DefaultHasher;
    let mut hasher = DefaultHasher::new();
    root_dir.hash(&mut hasher);
    let hash = hasher.finish();
    cache_dir.join(format!("{hash:x}.bin"))
}
