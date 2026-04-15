//! Git-aware persistence with .edgekeep support

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use ignore::gitignore::GitignoreBuilder;

/// Manages git-aware persistence
pub struct GitAwarePersist {
    gitignore: Option<ignore::gitignore::Gitignore>,
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

    fn load_gitignore(root: &Path) -> Option<ignore::gitignore::Gitignore> {
        let mut builder = GitignoreBuilder::new(root);

        // Add .gitignore if it exists
        let gitignore_path = root.join(".gitignore");
        if gitignore_path.exists() {
            builder.add(&gitignore_path);
        }

        builder.build().ok()
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
            let matched = gitignore.matched_path_or_any_parents(rel_path, false);
            return !matched.is_ignore();
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
            if let Ok(glob) = glob::Pattern::new(&pattern_str) {
                return glob.matches(&path_str);
            }
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
    use tempfile::TempDir;

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
        let workspace = Path::new("/home/ken/edgerun_reference_core");
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
