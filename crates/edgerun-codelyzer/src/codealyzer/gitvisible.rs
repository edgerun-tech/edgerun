use crate::glob::glob_match;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct VisibilityPolicy {
    pub visible_patterns: Vec<String>,
    pub hidden_patterns: Vec<String>,
}

impl Default for VisibilityPolicy {
    fn default() -> Self {
        Self {
            visible_patterns: vec!["src/**/*.rs".into(), "Cargo.toml".into(), "README*".into()],
            hidden_patterns: vec![
                ".git/**".into(),
                "target/**".into(),
                "*.secret".into(),
                "*.key".into(),
                "*.pem".into(),
                ".env*".into(),
                "**/secrets/**".into(),
                "**/private/**".into(),
            ],
        }
    }
}

pub fn load_visibility_policy(crate_dir: &Path) -> VisibilityPolicy {
    let gitvisible_path = crate_dir.join(".gitvisible");
    if !gitvisible_path.exists() {
        return VisibilityPolicy::default();
    }

    let content = match fs::read_to_string(&gitvisible_path) {
        Ok(c) => c,
        Err(_) => return VisibilityPolicy::default(),
    };

    parse_gitvisible(&content)
}

fn parse_gitvisible(content: &str) -> VisibilityPolicy {
    let mut policy = VisibilityPolicy::default();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.starts_with('!') {
            policy.hidden_patterns.push(line[1..].trim().to_string());
        } else {
            policy.visible_patterns.push(line.to_string());
        }
    }
    policy
}

pub fn is_path_visible(path: &Path, policy: &VisibilityPolicy, crate_dir: &Path) -> bool {
    let rel = path.strip_prefix(crate_dir).unwrap_or(path);
    let rel_str = rel.to_string_lossy();

    for hidden in &policy.hidden_patterns {
        if match_glob(&rel_str, hidden) {
            return false;
        }
    }
    for visible in &policy.visible_patterns {
        if match_glob(&rel_str, visible) {
            return true;
        }
    }
    false
}

fn match_glob(path: &str, pattern: &str) -> bool {
    let p = pattern.replace("**/", "**");
    glob_match(&p, path)
}

pub fn collect_visible_files(crate_dir: &Path, policy: &VisibilityPolicy) -> (Vec<PathBuf>, usize) {
    let mut visible = Vec::new();
    let mut hidden_count = 0;

    if let Ok(_entries) = std::fs::read_dir(crate_dir) {
        collect_files_recursive(crate_dir, policy, &mut visible, &mut hidden_count, 0);
    }

    (visible, hidden_count)
}

fn collect_files_recursive(
    dir: &Path,
    policy: &VisibilityPolicy,
    visible: &mut Vec<PathBuf>,
    hidden_count: &mut usize,
    depth: u32,
) {
    if depth > 10 {
        return;
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(&path, policy, visible, hidden_count, depth + 1);
            } else if is_path_visible(&path, policy, dir.parent().unwrap_or(dir)) {
                visible.push(path);
            } else {
                *hidden_count += 1;
            }
        }
    }
}
