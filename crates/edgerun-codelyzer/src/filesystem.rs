#![allow(dead_code)]

/// Filesystem tracking layer.
///
/// Scans directories for tracked source files, computes a fast content hash,
/// and diffs snapshots to detect new / modified / deleted files.
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
#[derive(Debug, Clone)]
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
    /// <cache_dir>/<hash(root_dir)>.bin.
    pub fn load(root_dir: &str) -> Self {
        let cache_dir = cache_dir();
        let cache_file = cache_file_path(&cache_dir, root_dir);

        let entries = if cache_file.exists() {
            std::fs::read_to_string(&cache_file)
                .ok()
                .and_then(|cache| decode_parse_cache(&cache))
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
        let cache = encode_parse_cache(&self.entries);
        if let Err(e) = std::fs::write(&cache_file, cache) {
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
    cache_dir.join(format!("{hash:x}.cache"))
}

fn encode_parse_cache(entries: &HashMap<String, CacheEntry>) -> String {
    let mut out = String::from("edgerun-codelyzer-cache-v1\n");
    let mut keys: Vec<&String> = entries.keys().collect();
    keys.sort();
    for path in keys {
        let entry = &entries[path];
        out.push_str("file\t");
        push_escaped(&mut out, path);
        out.push('\t');
        out.push_str(&entry.file_hash.to_string());
        out.push('\t');
        out.push_str(&entry.mtime.to_string());
        out.push('\t');
        out.push_str(&entry.functions.len().to_string());
        out.push('\t');
        out.push_str(&entry.calls.len().to_string());
        out.push('\n');
        for function in &entry.functions {
            out.push_str("fn\t");
            push_escaped(&mut out, &function.name);
            out.push('\t');
            out.push_str(&function.start_byte.to_string());
            out.push('\t');
            out.push_str(&function.end_byte.to_string());
            out.push('\t');
            out.push_str(if function.is_static { "1" } else { "0" });
            out.push('\t');
            out.push_str(if function.is_macro_def { "1" } else { "0" });
            out.push('\n');
        }
        for call in &entry.calls {
            out.push_str("call\t");
            push_escaped(&mut out, &call.callee_name);
            out.push('\t');
            out.push_str(&call.start_byte.to_string());
            out.push('\t');
            out.push_str(&call.end_byte.to_string());
            out.push('\t');
            out.push_str(call.kind.label());
            out.push('\n');
        }
    }
    out
}

fn decode_parse_cache(input: &str) -> Option<HashMap<String, CacheEntry>> {
    let mut lines = input.lines();
    if lines.next()? != "edgerun-codelyzer-cache-v1" {
        return None;
    }
    let mut entries = HashMap::new();
    while let Some(line) = lines.next() {
        let fields = split_escaped_fields(line)?;
        if fields.len() != 6 || fields[0] != "file" {
            return None;
        }
        let path = fields[1].clone();
        let file_hash = fields[2].parse().ok()?;
        let mtime = fields[3].parse().ok()?;
        let function_count: usize = fields[4].parse().ok()?;
        let call_count: usize = fields[5].parse().ok()?;
        let mut functions = Vec::with_capacity(function_count);
        let mut calls = Vec::with_capacity(call_count);
        for _ in 0..function_count {
            let fields = split_escaped_fields(lines.next()?)?;
            if fields.len() != 6 || fields[0] != "fn" {
                return None;
            }
            functions.push(crate::parser::RawFunctionOwned {
                name: fields[1].clone(),
                start_byte: fields[2].parse().ok()?,
                end_byte: fields[3].parse().ok()?,
                is_static: fields[4] == "1",
                is_macro_def: fields[5] == "1",
            });
        }
        for _ in 0..call_count {
            let fields = split_escaped_fields(lines.next()?)?;
            if fields.len() != 5 || fields[0] != "call" {
                return None;
            }
            calls.push(crate::parser::RawCallOwned {
                callee_name: fields[1].clone(),
                start_byte: fields[2].parse().ok()?,
                end_byte: fields[3].parse().ok()?,
                kind: parse_call_kind(&fields[4])?,
            });
        }
        entries.insert(
            path,
            CacheEntry {
                file_hash,
                mtime,
                functions,
                calls,
            },
        );
    }
    Some(entries)
}

fn push_escaped(out: &mut String, value: &str) {
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
}

fn split_escaped_fields(line: &str) -> Option<Vec<String>> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\t' => {
                fields.push(current);
                current = String::new();
            }
            '\\' => match chars.next()? {
                '\\' => current.push('\\'),
                'n' => current.push('\n'),
                't' => current.push('\t'),
                'r' => current.push('\r'),
                _ => return None,
            },
            _ => current.push(ch),
        }
    }
    fields.push(current);
    Some(fields)
}

fn parse_call_kind(value: &str) -> Option<crate::uir::CallKind> {
    match value {
        "direct" => Some(crate::uir::CallKind::Direct),
        "indirect" => Some(crate::uir::CallKind::Indirect),
        "macro" => Some(crate::uir::CallKind::Macro),
        "unknown" => Some(crate::uir::CallKind::Unknown),
        _ => None,
    }
}
