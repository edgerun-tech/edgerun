use std::{
    collections::HashMap,
    env,
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{SystemTime, UNIX_EPOCH, Instant},
};

/// Repository Registry: discovers, indexes, and manages multiple code
/// repositories.
///
/// Scans configured directories for git repositories, indexes them on demand,
/// and provides fast switching between indexed repos in the viewer.
use edgerun_json::{self, ToJson, FromJson, JsonValue, Map};

/// Default directories to scan for repositories.
const DEFAULT_SCAN_ROOTS: &[&str] = &["/home", "/Users", "/opt", "/var/src", "/srv"];

/// Directory names to always exclude during discovery.
const EXCLUDED_DIRS: &[&str] = &[
    "node_modules",
    "target",
    ".git",
    "__pycache__",
    ".venv",
    "venv",
    "vendor",
    ".cargo",
    ".rustup",
    ".cache",
    ".local",
    ".config",
    ".vscode",
    ".idea",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".tox",
    ".eggs",
    "*.egg-info",
    "site-packages",
    "test-results",
    "coverage",
    ".nyc_output",
    ".parcel-cache",
    ".serverless",
    ".terraform",
    ".gradle",
    ".m2",
    ".sbt",
    "Pods",
    ".swiftpm",
    "DerivedData",
];

/// File extensions that indicate a directory is not a code repo.
#[allow(dead_code)]
const NON_REPO_EXTENSIONS: &[&str] = &[];

/// Maximum directory depth for repo discovery.
const MAX_DISCOVERY_DEPTH: usize = 8;

/// Minimum files to consider a directory a "repository".
#[allow(dead_code)]
const MIN_REPO_FILES: usize = 1;

/// State of a repository's index.
#[derive(Debug, Clone, PartialEq)]
pub enum IndexState {
    NotIndexed,
    Indexing {
        progress: f32,
        files_scanned: usize,
        total_files: usize,
    },
    Indexed {
        functions: usize,
        edges: usize,
        index_time_ms: u64,
    },
    Stale {
        functions: usize,
        edges: usize,
        last_indexed: u64,
    },
    Error {
        message: String,
    },
}

impl ToJson for IndexState {
    fn to_json(&self) -> JsonValue {
        let mut m = Map::new();
        match self {
            IndexState::NotIndexed => {
                m.push_field("type", "NotIndexed");
            }
            IndexState::Indexing { progress, files_scanned, total_files } => {
                m.push_field("type", "Indexing");
                m.push_field("progress", progress);
                m.push_field("files_scanned", &(*files_scanned as u64));
                m.push_field("total_files", &(*total_files as u64));
            }
            IndexState::Indexed { functions, edges, index_time_ms } => {
                m.push_field("type", "Indexed");
                m.push_field("functions", &(*functions as u64));
                m.push_field("edges", &(*edges as u64));
                m.push_field("index_time_ms", index_time_ms);
            }
            IndexState::Stale { functions, edges, last_indexed } => {
                m.push_field("type", "Stale");
                m.push_field("functions", &(*functions as u64));
                m.push_field("edges", &(*edges as u64));
                m.push_field("last_indexed", last_indexed);
            }
            IndexState::Error { message } => {
                m.push_field("type", "Error");
                m.push_field("message", message);
            }
        }
        JsonValue::Object(m)
    }
}

impl FromJson for IndexState {
    fn from_json(value: &JsonValue) -> Option<Self> {
        let obj = value.as_object()?;
        let type_str = obj.get_str("type")?;
        match type_str {
            "NotIndexed" => Some(IndexState::NotIndexed),
            "Indexing" => Some(IndexState::Indexing {
                progress: obj.get_f64("progress")? as f32,
                files_scanned: obj.get_u64("files_scanned")? as usize,
                total_files: obj.get_u64("total_files")? as usize,
            }),
            "Indexed" => Some(IndexState::Indexed {
                functions: obj.get_u64("functions")? as usize,
                edges: obj.get_u64("edges")? as usize,
                index_time_ms: obj.get_u64("index_time_ms")?,
            }),
            "Stale" => Some(IndexState::Stale {
                functions: obj.get_u64("functions")? as usize,
                edges: obj.get_u64("edges")? as usize,
                last_indexed: obj.get_u64("last_indexed")?,
            }),
            "Error" => Some(IndexState::Error {
                message: obj.get_str("message")?.to_string(),
            }),
            _ => None,
        }
    }
}

/// Metadata about a discovered repository.
#[derive(Debug, Clone)]
pub struct RepoInfo {
    pub path: String,
    pub name: String,
    pub is_git_repo: bool,
    pub git_remote: Option<String>,
    pub file_count: usize,
    pub total_size_bytes: u64,
    pub last_modified: u64,
    pub index_state: IndexState,
    pub languages: Vec<String>,
    pub added_at: u64, // epoch seconds when added to registry
}

impl ToJson for RepoInfo {
    fn to_json(&self) -> JsonValue {
        let mut m = Map::new();
        m.push_field("path", &self.path);
        m.push_field("name", &self.name);
        m.push_field("is_git_repo", &self.is_git_repo);
        m.push_field("git_remote", &self.git_remote);
        m.push_field("file_count", &(self.file_count as u64));
        m.push_field("total_size_bytes", &self.total_size_bytes);
        m.push_field("last_modified", &self.last_modified);
        m.push_field("index_state", &self.index_state);
        m.push_field("languages", &self.languages);
        m.push_field("added_at", &self.added_at);
        JsonValue::Object(m)
    }
}

impl FromJson for RepoInfo {
    fn from_json(value: &JsonValue) -> Option<Self> {
        let obj = value.as_object()?;
        Some(Self {
            path: obj.get_str("path")?.to_string(),
            name: obj.get_str("name")?.to_string(),
            is_git_repo: obj.get_bool("is_git_repo")?,
            git_remote: obj.get_str("git_remote").map(|s| s.to_string()),
            file_count: obj.get_u64("file_count")? as usize,
            total_size_bytes: obj.get_u64("total_size_bytes")?,
            last_modified: obj.get_u64("last_modified")?,
            index_state: FromJson::from_json(obj.get("index_state")?)?,
            languages: obj.get_obj_vec("languages").unwrap_or_default(),
            added_at: obj.get_u64("added_at")?,
        })
    }
}

/// Configuration for the repository registry.
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Directories to scan for repositories.
    pub scan_roots: Vec<String>,
    /// Additional paths to exclude from scanning.
    pub excluded_paths: Vec<String>,
    /// Maximum concurrent indexing jobs.
    pub max_concurrent_index: usize,
    /// Auto-discover repos on startup.
    pub auto_discover: bool,
}

impl ToJson for RegistryConfig {
    fn to_json(&self) -> JsonValue {
        let mut m = Map::new();
        m.push_field("scan_roots", &self.scan_roots);
        m.push_field("excluded_paths", &self.excluded_paths);
        m.push_field("max_concurrent_index", &(self.max_concurrent_index as u64));
        m.push_field("auto_discover", &self.auto_discover);
        JsonValue::Object(m)
    }
}

impl FromJson for RegistryConfig {
    fn from_json(value: &JsonValue) -> Option<Self> {
        let obj = value.as_object()?;
        Some(Self {
            scan_roots: obj.get_obj_vec("scan_roots").unwrap_or_else(|| {
                DEFAULT_SCAN_ROOTS.iter().map(|s| s.to_string()).collect()
            }),
            excluded_paths: obj.get_obj_vec("excluded_paths").unwrap_or_default(),
            max_concurrent_index: obj.get_u64("max_concurrent_index").unwrap_or(2) as usize,
            auto_discover: obj.get_bool("auto_discover").unwrap_or(true),
        })
    }
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            scan_roots: DEFAULT_SCAN_ROOTS.iter().map(|s| s.to_string()).collect(),
            excluded_paths: Vec::new(),
            max_concurrent_index: 2,
            auto_discover: true,
        }
    }
}

/// The registry manages all known repositories.
pub struct RepositoryRegistry {
    pub config: RegistryConfig,
    pub repos: HashMap<String, RepoInfo>, // path → info
    active_repo: Option<String>,          // currently active repo path
    cancel_flag: Arc<AtomicBool>,
}

impl RepositoryRegistry {
    pub fn new(config: RegistryConfig) -> Self {
        Self {
            config,
            repos: HashMap::new(),
            active_repo: None,
            cancel_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Load the registry from disk.
    pub fn load() -> Self {
        let config = load_config().unwrap_or_default();
        let repos = load_repo_index().unwrap_or_default();
        let active = load_active_repo();

        let mut registry = Self::new(config);
        let active_valid = active.filter(|p| repos.contains_key(p.as_str()));
        registry.repos = repos;
        registry.active_repo = active_valid;
        registry
    }

    /// Save the registry to disk.
    pub fn save(&self) {
        save_config(&self.config);
        save_repo_index(&self.repos);
        if let Some(ref path) = self.active_repo {
            save_active_repo(path);
        }
    }

    /// Get all known repositories.
    pub fn list_repos(&self) -> Vec<&RepoInfo> {
        let mut repos: Vec<_> = self.repos.values().collect();
        repos.sort_by(|a, b| a.name.cmp(&b.name));
        repos
    }

    /// Get a specific repo by path.
    #[allow(dead_code)]
    pub fn get_repo(&self, path: &str) -> Option<&RepoInfo> {
        self.repos.get(path)
    }

    /// Get the currently active repo path.
    pub fn active_repo(&self) -> Option<&str> {
        self.active_repo.as_deref()
    }

    /// Manually add a repository by path.
    pub fn add_repo(&mut self, path: &str) -> Result<(), String> {
        let path = Path::new(path)
            .canonicalize()
            .map_err(|e| format!("Invalid path: {}", e))?;
        let path_str = path.to_string_lossy().to_string();

        if self.repos.contains_key(&path_str) {
            return Err(format!("Repository already added: {}", path_str));
        }

        let info = scan_repo_info(&path_str, &self.config.excluded_paths);
        self.repos.insert(path_str.clone(), info);
        self.save();
        Ok(())
    }

    /// Remove a repository from the registry.
    pub fn remove_repo(&mut self, path: &str) -> bool {
        let removed = self.repos.remove(path).is_some();
        if self.active_repo.as_deref() == Some(path) {
            self.active_repo = None;
        }
        if removed {
            self.save();
        }
        removed
    }

    /// Discover repositories by scanning configured roots.
    pub fn discover_repos(&mut self) -> Vec<RepoInfo> {
        let mut found = Vec::new();

        for root in &self.config.scan_roots {
            if !Path::new(root).exists() {
                continue;
            }
            discover_git_repos(root, &self.config.excluded_paths, &mut found);
        }

        // Add newly discovered repos
        let before = self.repos.len();
        for repo in &found {
            self.repos
                .entry(repo.path.clone())
                .or_insert_with(|| repo.clone());
        }

        if self.repos.len() > before {
            self.save();
        }

        found
    }

    /// Index a repository (build the call graph).
    /// Returns immediately; indexing runs in background.
    pub fn start_indexing(&self, path: &str) -> Result<(), String> {
        let repo = self
            .repos
            .get(path)
            .ok_or_else(|| format!("Repository not found: {}", path))?;

        if matches!(repo.index_state, IndexState::Indexing { .. }) {
            return Err("Repository is already being indexed".to_string());
        }

        let path = path.to_string();
        let cancel = self.cancel_flag.clone();

        std::thread::spawn(move || {
            // Import here to avoid circular deps in analysis
            let start = Instant::now();
            let cancel_clone = cancel.clone();

            // Set indexing state via a callback would be ideal, but for now
            // we just run the analysis and update state on completion
            let result = std::panic::catch_unwind(|| crate::analyzer::analyze_full(&path));

            match result {
                Ok((analysis_result, _changes)) => {
                    let elapsed = start.elapsed();
                    let func_count = analysis_result.program.functions.len();
                    let edge_count = analysis_result.program.edges.len();

                    // Update the repo's index state
                    // This requires mutable access to the registry, which we don't have
                    // in this thread. The server will poll for results.
                    println!(
                        "[registry] Indexed {} in {:.2}s: {} functions, {} edges",
                        path,
                        elapsed.as_secs_f64(),
                        func_count,
                        edge_count
                    );
                }
                Err(e) => {
                    eprintln!("[registry] Indexing failed for {}: {:?}", path, e);
                }
            }

            // Reset cancel flag
            cancel_clone.store(false, Ordering::SeqCst);
        });

        Ok(())
    }

    /// Switch the active repository.
    pub fn switch_repo(&mut self, path: &str) -> Result<(), String> {
        if !self.repos.contains_key(path) {
            return Err(format!("Repository not found: {}", path));
        }
        self.active_repo = Some(path.to_string());
        self.save();
        Ok(())
    }

    /// Set the active repository (used during initialization).
    pub fn set_active_repo(&mut self, path: String) {
        self.active_repo = Some(path);
    }

    /// Get the cancel flag for aborting indexing.
    #[allow(dead_code)]
    pub fn cancel_index(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// Check if indexing was cancelled.
    #[allow(dead_code)]
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.swap(false, Ordering::SeqCst)
    }
}

/// Scan a directory for basic repository information.
pub fn scan_repo_info(path: &str, excluded: &[String]) -> RepoInfo {
    let path_buf = Path::new(path);
    let name = path_buf
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string());

    let is_git = path_buf.join(".git").exists();
    let git_remote = if is_git { read_git_remote(path) } else { None };

    let (file_count, total_size, last_modified, languages) = count_files(path, excluded);

    RepoInfo {
        path: path.to_string(),
        name,
        is_git_repo: is_git,
        git_remote,
        file_count,
        total_size_bytes: total_size,
        last_modified,
        index_state: IndexState::NotIndexed,
        languages,
        added_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
    }
}

/// Read the git remote URL for a repository.
fn read_git_remote(path: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["-C", path, "remote", "get-url", "origin"])
        .output()
        .ok()?;
    if output.status.success() {
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        // Try 'git remote -v'
        let output = std::process::Command::new("git")
            .args(["-C", path, "remote", "-v"])
            .output()
            .ok()?;
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            text.lines()
                .next()
                .and_then(|l| l.split_whitespace().nth(1))
                .map(String::from)
        } else {
            None
        }
    }
}

/// Count source files and gather language info.
fn count_files(path: &str, excluded: &[String]) -> (usize, u64, u64, Vec<String>) {
    use std::collections::HashSet;

    let mut file_count = 0;
    let mut total_size = 0u64;
    let mut last_modified = 0u64;
    let mut langs: HashSet<String> = HashSet::new();

    let source_ext = [
        "c", "h", "rs", "ts", "tsx", "js", "jsx", "mjs", "py", "go", "java", "cpp", "cc", "cxx",
        "hpp", "cs", "rb", "php", "swift", "kt", "kts", "scala", "r", "R", "lua", "sh", "bash",
        "zsh", "fish",
    ];

    count_files_inner(
        Path::new(path),
        excluded,
        &source_ext,
        &mut file_count,
        &mut total_size,
        &mut last_modified,
        &mut langs,
        0,
    );

    let mut langs_vec: Vec<_> = langs.into_iter().collect();
    langs_vec.sort();
    (file_count, total_size, last_modified, langs_vec)
}

#[allow(clippy::too_many_arguments)]
fn count_files_inner(
    current: &Path,
    excluded: &[String],
    source_ext: &[&str],
    file_count: &mut usize,
    total_size: &mut u64,
    last_modified: &mut u64,
    langs: &mut std::collections::HashSet<String>,
    depth: usize,
) {
    if depth > MAX_DISCOVERY_DEPTH {
        return;
    }

    let Ok(entries) = fs::read_dir(current) else {
        return;
    };

    let mut dirs = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if is_excluded(name, excluded) || path.is_symlink() {
            continue;
        }

        if path.is_dir() {
            if should_descend(&path, excluded) {
                dirs.push(path);
            }
        } else if let Ok(meta) = entry.metadata() {
            if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if source_ext.contains(&ext) {
                    *file_count += 1;
                    *total_size += meta.len();
                    let mtime = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    if mtime > *last_modified {
                        *last_modified = mtime;
                    }
                    langs.insert(lang_from_ext(ext).to_string());
                }
            }
        }
    }

    for dir in dirs {
        count_files_inner(
            &dir,
            excluded,
            source_ext,
            file_count,
            total_size,
            last_modified,
            langs,
            depth + 1,
        );
    }
}

/// Discover git repositories under a root directory.
fn discover_git_repos(root: &str, excluded: &[String], found: &mut Vec<RepoInfo>) {
    discover_git_repos_inner(Path::new(root), excluded, found, 0);
}

fn discover_git_repos_inner(
    current: &Path,
    excluded: &[String],
    found: &mut Vec<RepoInfo>,
    depth: usize,
) {
    if depth > MAX_DISCOVERY_DEPTH {
        return;
    }

    let Ok(entries) = fs::read_dir(current) else {
        return;
    };

    let mut dirs = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if is_excluded(name, excluded) || path.is_symlink() {
            continue;
        }

        if path.is_dir() {
            // Check if this is a git repo
            if path.join(".git").is_dir() || path.join(".git").is_file() {
                let path_str = path.to_string_lossy().to_string();
                let info = scan_repo_info(&path_str, excluded);
                found.push(info);
                // Don't descend into a git repo
                continue;
            }
            if should_descend(&path, excluded) {
                dirs.push(path);
            }
        }
    }

    for dir in dirs {
        discover_git_repos_inner(&dir, excluded, found, depth + 1);
    }
}

/// Check if a directory name should be excluded.
fn is_excluded(name: &str, extra: &[String]) -> bool {
    EXCLUDED_DIRS.contains(&name)
        || extra
            .iter()
            .any(|p| p.ends_with(name) || name.ends_with(p.as_str()))
}

/// Check if we should descend into a directory.
fn should_descend(path: &Path, excluded: &[String]) -> bool {
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if is_excluded(name, excluded) {
            return false;
        }
    }
    // Skip hidden directories except .git
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        if name.starts_with('.') && name != ".git" {
            return false;
        }
    }
    true
}

/// Map file extension to language name.
fn lang_from_ext(ext: &str) -> &'static str {
    match ext {
        "c" | "h" => "c",
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "cpp" | "cc" | "cxx" | "hpp" => "cpp",
        "cs" => "csharp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "scala" => "scala",
        "r" | "R" => "r",
        "lua" => "lua",
        "sh" | "bash" | "zsh" | "fish" => "shell",
        _ => "other",
    }
}

// ─── Persistence ──────────────────────────────────────────────────────

fn registry_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("codeanalyzer")
}

fn config_path() -> PathBuf {
    registry_dir().join("config.json")
}

fn index_path() -> PathBuf {
    registry_dir().join("repos.json")
}

fn active_path() -> PathBuf {
    registry_dir().join("active.txt")
}

fn load_config() -> Option<RegistryConfig> {
    let path = config_path();
    if path.exists() {
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

fn save_config(config: &RegistryConfig) {
    let dir = registry_dir();
    let _ = fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = fs::write(config_path(), json);
    }
}

fn load_repo_index() -> Option<HashMap<String, RepoInfo>> {
    let path = index_path();
    if path.exists() {
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

fn save_repo_index(repos: &HashMap<String, RepoInfo>) {
    let dir = registry_dir();
    let _ = fs::create_dir_all(&dir);
    if let Ok(json) = serde_json::to_string_pretty(repos) {
        let _ = fs::write(index_path(), json);
    }
}

fn load_active_repo() -> Option<String> {
    let path = active_path();
    if path.exists() {
        fs::read_to_string(&path).ok().map(|s| s.trim().to_string())
    } else {
        None
    }
}

fn save_active_repo(path: &str) {
    let dir = registry_dir();
    let _ = fs::create_dir_all(&dir);
    let _ = fs::write(active_path(), path);
}
