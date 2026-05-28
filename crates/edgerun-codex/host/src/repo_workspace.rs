use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

const MAX_FILE_BYTES: usize = 256 * 1024;
const MAX_TOTAL_BYTES: usize = 32 * 1024 * 1024;
const MAX_FILES: usize = 8192;
const MAX_SEARCH_RESULTS: usize = 12;
const MAX_SNIPPET_BYTES: usize = 1200;

#[derive(Clone, Debug)]
pub struct RepoWorkspace {
    root: PathBuf,
    files: Vec<RepoFile>,
    skipped_files: usize,
    skipped_bytes: usize,
}

#[derive(Clone, Debug)]
pub struct RepoFile {
    path: String,
    bytes: Vec<u8>,
    text: Option<String>,
}

#[derive(Clone, Debug)]
pub struct SearchHit {
    pub path: String,
    pub score: usize,
    pub snippet: String,
}

impl RepoWorkspace {
    pub fn load(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        let mut workspace = Self {
            root,
            files: Vec::new(),
            skipped_files: 0,
            skipped_bytes: 0,
        };
        let root = workspace.root.clone();
        workspace.load_dir(&root)?;
        workspace.files.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(workspace)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    pub fn total_bytes(&self) -> usize {
        self.files.iter().map(|file| file.bytes.len()).sum()
    }

    pub fn summary(&self) -> String {
        let mut out = String::new();
        out.push_str("RepoWorkspace:\n");
        out.push_str(&format!("Root: {}\n", self.root.display()));
        out.push_str(&format!("LoadedFiles: {}\n", self.file_count()));
        out.push_str(&format!("LoadedBytes: {}\n", self.total_bytes()));
        out.push_str(&format!("SkippedFiles: {}\n", self.skipped_files));
        out.push_str(&format!("SkippedBytes: {}\n", self.skipped_bytes));
        out.push_str("ImportantFiles:\n");
        for path in self.important_paths().into_iter().take(80) {
            out.push_str("- ");
            out.push_str(path);
            out.push('\n');
        }
        out
    }

    pub fn context_for_request(&self, request: &str) -> String {
        let mut out = self.summary();
        let hits = self.search(request);
        if !hits.is_empty() {
            out.push_str("\nInMemorySearchHits:\n");
            for hit in hits {
                out.push_str("--- ");
                out.push_str(&hit.path);
                out.push_str(" score=");
                out.push_str(&hit.score.to_string());
                out.push_str(" ---\n");
                out.push_str(&hit.snippet);
                if !hit.snippet.ends_with('\n') {
                    out.push('\n');
                }
            }
        }
        out
    }

    pub fn read_text(&self, path: &str) -> Option<&str> {
        self.files
            .iter()
            .find(|file| file.path == normalize_repo_path(path))
            .and_then(|file| file.text.as_deref())
    }

    pub fn replace_text(&mut self, path: &str, new_text: String) -> bool {
        let normalized = normalize_repo_path(path);
        let Some(file) = self.files.iter_mut().find(|file| file.path == normalized) else {
            return false;
        };
        file.bytes = new_text.as_bytes().to_vec();
        file.text = Some(new_text);
        true
    }

    pub fn write_back(&self, path: &str) -> io::Result<()> {
        let normalized = normalize_repo_path(path);
        let Some(file) = self.files.iter().find(|file| file.path == normalized) else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "repo workspace path not loaded"));
        };
        let disk_path = self.root.join(&file.path);
        if let Some(parent) = disk_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(disk_path, &file.bytes)
    }

    pub fn search(&self, query: &str) -> Vec<SearchHit> {
        let terms = query_terms(query);
        if terms.is_empty() {
            return Vec::new();
        }

        let mut hits = Vec::new();
        for file in &self.files {
            let Some(text) = file.text.as_deref() else {
                continue;
            };
            let score = score_file(&file.path, text, &terms);
            if score == 0 {
                continue;
            }
            hits.push(SearchHit {
                path: file.path.clone(),
                score,
                snippet: snippet_for(text, &terms),
            });
        }
        hits.sort_by(|left, right| right.score.cmp(&left.score).then_with(|| left.path.cmp(&right.path)));
        hits.truncate(MAX_SEARCH_RESULTS);
        hits
    }

    fn load_dir(&mut self, dir: &Path) -> io::Result<()> {
        if self.files.len() >= MAX_FILES || self.total_bytes() >= MAX_TOTAL_BYTES {
            return Ok(());
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if should_ignore_name(name) {
                continue;
            }
            let metadata = entry.metadata()?;
            if metadata.is_dir() {
                self.load_dir(&path)?;
            } else if metadata.is_file() {
                self.load_file(&path, metadata.len() as usize)?;
            }
        }
        Ok(())
    }

    fn load_file(&mut self, path: &Path, size: usize) -> io::Result<()> {
        if self.files.len() >= MAX_FILES {
            self.skipped_files += 1;
            self.skipped_bytes += size;
            return Ok(());
        }
        if size > MAX_FILE_BYTES || self.total_bytes().saturating_add(size) > MAX_TOTAL_BYTES {
            self.skipped_files += 1;
            self.skipped_bytes += size;
            return Ok(());
        }
        if should_ignore_path(path) {
            return Ok(());
        }
        let bytes = fs::read(path)?;
        let rel = path.strip_prefix(&self.root).unwrap_or(path);
        let repo_path = normalize_repo_path(&rel.to_string_lossy());
        let text = String::from_utf8(bytes.clone()).ok();
        self.files.push(RepoFile {
            path: repo_path,
            bytes,
            text,
        });
        Ok(())
    }

    fn important_paths(&self) -> Vec<&str> {
        let mut paths: Vec<&str> = self.files.iter().map(|file| file.path.as_str()).collect();
        paths.sort_by_key(|path| important_path_rank(path));
        paths
    }
}

fn important_path_rank(path: &str) -> usize {
    if path == "Cargo.toml" || path == "Makefile" || path.ends_with("/Cargo.toml") {
        return 0;
    }
    if path.ends_with("main.rs") || path.ends_with("lib.rs") || path.ends_with("mod.rs") {
        return 1;
    }
    if path.contains("codex") || path.contains("agent") || path.contains("host") {
        return 2;
    }
    if path.ends_with(".rs") || path.ends_with(".zig") {
        return 3;
    }
    if path.ends_with(".md") || path.ends_with(".txt") {
        return 4;
    }
    5
}

fn score_file(path: &str, text: &str, terms: &[String]) -> usize {
    let path_lower = path.to_ascii_lowercase();
    let text_lower = text.to_ascii_lowercase();
    let mut score = 0;
    for term in terms {
        if path_lower.contains(term) {
            score += 20;
        }
        score += text_lower.matches(term).take(10).count();
    }
    score
}

fn snippet_for(text: &str, terms: &[String]) -> String {
    let lower = text.to_ascii_lowercase();
    let mut start = 0;
    for term in terms {
        if let Some(index) = lower.find(term) {
            start = index.saturating_sub(MAX_SNIPPET_BYTES / 3);
            break;
        }
    }
    while start > 0 && !text.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (start + MAX_SNIPPET_BYTES).min(text.len());
    while end > start && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[start..end].to_string()
}

fn query_terms(query: &str) -> Vec<String> {
    query
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_' && ch != '-')
        .filter(|term| term.len() >= 3)
        .map(|term| term.to_ascii_lowercase())
        .filter(|term| !STOP_TERMS.contains(&term.as_str()))
        .take(24)
        .collect()
}

fn normalize_repo_path(path: &str) -> String {
    path.replace('\\', "/")
        .trim_start_matches("./")
        .trim_start_matches('/')
        .to_string()
}

fn should_ignore_name(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".build"
            | "target"
            | "node_modules"
            | "vendor"
            | "dist"
            | "build"
            | ".cache"
            | ".zig-cache"
            | "zig-out"
    )
}

fn should_ignore_path(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "webp" | "gif" | "wasm" | "a" | "o" | "so" | "dylib" | "dll" | "rlib" | "zip" | "tar" | "gz" | "xz" | "7z" | "bin"
    )
}

const STOP_TERMS: &[&str] = &[
    "the", "and", "for", "with", "that", "this", "from", "have", "make", "what", "when", "where", "there", "their", "into", "your", "you", "are", "was", "were", "will", "would", "should", "could",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_terms_drop_short_and_stop_words() {
        assert_eq!(query_terms("fix the codex host"), vec!["fix", "codex", "host"]);
    }

    #[test]
    fn normalize_paths_are_repo_relative() {
        assert_eq!(normalize_repo_path("./a\\b.rs"), "a/b.rs");
    }
}
