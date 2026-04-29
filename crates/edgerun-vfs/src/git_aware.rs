//! Git-aware persistence with .edgekeep support

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

struct GitignoreRule {
    pattern: String,
    negated: bool,
    directory_only: bool,
    rooted: bool,
}

struct GitignoreRules {
    rules: Vec<GitignoreRule>,
}

impl GitignoreRules {
    fn load(root: &Path) -> Option<Self> {
        let content = fs::read_to_string(root.join(".gitignore")).ok()?;
        let rules = content
            .lines()
            .filter_map(GitignoreRule::parse)
            .collect::<Vec<_>>();

        if rules.is_empty() {
            None
        } else {
            Some(Self { rules })
        }
    }

    fn is_ignored(&self, path: &Path) -> bool {
        let path = path.to_string_lossy().replace('\\', "/");
        let mut ignored = false;

        for rule in &self.rules {
            if rule.matches(&path) {
                ignored = !rule.negated;
            }
        }

        ignored
    }
}

impl GitignoreRule {
    fn parse(line: &str) -> Option<Self> {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            return None;
        }

        let (negated, line) = line
            .strip_prefix('!')
            .map(|rest| (true, rest))
            .unwrap_or((false, line));
        let (rooted, line) = line
            .strip_prefix('/')
            .map(|rest| (true, rest))
            .unwrap_or((false, line));

        let directory_only = line.ends_with('/');
        let pattern = line.trim_end_matches('/').to_string();

        if pattern.is_empty() {
            None
        } else {
            Some(Self {
                pattern,
                negated,
                directory_only,
                rooted,
            })
        }
    }

    fn matches(&self, path: &str) -> bool {
        if self.directory_only {
            return self.matches_directory(path);
        }

        if self.pattern.contains('/') || self.rooted {
            return edgerun_glob::glob_match(&self.pattern, path);
        }

        let basename = path.rsplit('/').next().unwrap_or(path);
        edgerun_glob::glob_match(&self.pattern, basename)
    }

    fn matches_directory(&self, path: &str) -> bool {
        if self.rooted || self.pattern.contains('/') {
            return path == self.pattern || path.starts_with(&format!("{}/", self.pattern));
        }

        path == self.pattern
            || path.starts_with(&format!("{}/", self.pattern))
            || path.contains(&format!("/{}/", self.pattern))
    }
}

/// Manages git-aware persistence
pub struct GitAwarePersist {
    gitignore: Option<GitignoreRules>,
    edgekeep: HashSet<PathBuf>,
    root: PathBuf,
}

impl GitAwarePersist {
    pub fn new(root: &Path) -> Self {
        let gitignore = Self::load_gitignore(root);
        let edgekeep = Self::load_edgekeep(root);

        Self {
            gitignore,
            edgekeep,
            root: root.to_path_buf(),
        }
    }

    fn load_gitignore(root: &Path) -> Option<GitignoreRules> {
        GitignoreRules::load(root)
    }

    fn load_edgekeep(root: &Path) -> HashSet<PathBuf> {
        let edgekeep_path = root.join(".edgekeep");
        let mut patterns = HashSet::new();

        if edgekeep_path.exists() {
            if let Ok(content) = fs::read_to_string(&edgekeep_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        patterns.insert(PathBuf::from(line));
                    }
                }
            }
        }

        patterns
    }

    /// Check if a file should be persisted
    pub fn should_persist(&self, rel_path: &Path) -> bool {
        // Check .edgekeep first
        for pattern in &self.edgekeep {
            if self.pattern_matches(pattern, rel_path) {
                return true;
            }
        }

        // If gitignore exists, check if file is ignored
        if let Some(gitignore) = &self.gitignore {
            return !gitignore.is_ignored(rel_path);
        }

        // No gitignore = persist everything
        true
    }

    fn pattern_matches(&self, pattern: &Path, path: &Path) -> bool {
        let pattern_str = pattern.to_string_lossy();
        let path_str = path.to_string_lossy();

        // Exact match
        if pattern == path {
            return true;
        }

        // Glob pattern
        if pattern_str.contains('*') {
            return edgerun_glob::glob_match(&pattern_str, &path_str);
        }

        // Directory prefix
        if path.starts_with(pattern) {
            return true;
        }

        false
    }

    /// Get list of files that should be persisted
    pub fn persist_list<'a, I>(&self, paths: I) -> Vec<&'a PathBuf>
    where
        I: Iterator<Item = &'a PathBuf>,
    {
        paths.filter(|path| self.should_persist(path)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDir;

    #[test]
    fn test_edgekeep_exact_match() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".gitignore"), "*\n").unwrap();
        fs::write(tmp.path().join(".edgekeep"), ".env\n").unwrap();

        let persist = GitAwarePersist::new(tmp.path());
        assert!(persist.should_persist(Path::new(".env")));
        assert!(!persist.should_persist(Path::new("config.toml")));
    }

    #[test]
    fn test_edgekeep_glob_pattern() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".gitignore"), "*\n").unwrap();
        fs::write(tmp.path().join(".edgekeep"), "*.local.*\n").unwrap();

        let persist = GitAwarePersist::new(tmp.path());
        assert!(persist.should_persist(Path::new("config.local.toml")));
        assert!(persist.should_persist(Path::new("test.local.txt")));
        assert!(!persist.should_persist(Path::new("config.toml")));
    }

    #[test]
    fn test_edgekeep_directory() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".gitignore"), "*\n").unwrap();
        fs::write(tmp.path().join(".edgekeep"), "secrets/\n").unwrap();

        let persist = GitAwarePersist::new(tmp.path());
        assert!(persist.should_persist(Path::new("secrets/api_key.txt")));
        assert!(persist.should_persist(Path::new("secrets/sub/password.txt")));
        assert!(!persist.should_persist(Path::new("src/main.rs")));
    }

    #[test]
    fn test_gitignore_respected() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".gitignore"), "target/\n*.log\n").unwrap();

        let persist = GitAwarePersist::new(tmp.path());
        assert!(!persist.should_persist(Path::new("target/debug/main")));
        assert!(!persist.should_persist(Path::new("debug.log")));
        assert!(persist.should_persist(Path::new("src/main.rs")));
    }

    #[test]
    fn test_no_gitignore_persists_all() {
        let tmp = TempDir::new().unwrap();

        let persist = GitAwarePersist::new(tmp.path());
        assert!(persist.should_persist(Path::new("src/main.rs")));
        assert!(persist.should_persist(Path::new(".env")));
        assert!(persist.should_persist(Path::new("config.toml")));
    }

    #[test]
    fn test_edgekeep_overrides_gitignore() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join(".gitignore"), "*.log\n").unwrap();
        fs::write(tmp.path().join(".edgekeep"), "debug.log\n").unwrap();

        let persist = GitAwarePersist::new(tmp.path());
        assert!(persist.should_persist(Path::new("debug.log")));
    }

    #[test]
    fn test_real_workspace_gitignore() {
        let workspace = Path::new("/home/ken/edgerun_core");
        if !workspace.exists() {
            return;
        }

        let persist = GitAwarePersist::new(workspace);

        assert!(persist.should_persist(Path::new("Cargo.toml")));
        assert!(persist.should_persist(Path::new("README.md")));
        assert!(persist.should_persist(Path::new("crates/edgerun-vfs/src/lib.rs")));
        assert!(persist.should_persist(Path::new("crates/edgerun-vfs/Cargo.toml")));

        assert!(!persist.should_persist(Path::new("target/debug/main")));
        assert!(!persist.should_persist(Path::new("target/release/edgerun-agent")));
        assert!(!persist.should_persist(Path::new("target/.rustc_info.json")));
    }
}
