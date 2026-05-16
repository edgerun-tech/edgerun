/// Minimal Git integration via CLI.
///
/// Parses recent commit history to build a file → last-modified-commit map.
/// No libgit2, no full git implementation — just process::Command.
use std::collections::{HashMap, HashSet};
use std::process::Command;

/// A single commit with the files it touched.
#[derive(Debug, Clone)]
pub struct Commit {
    pub hash: String,
    #[allow(dead_code)]
    pub timestamp: u64,
    pub files: Vec<String>,
}

/// Fetch recent commits from a git repository.
/// Returns commits in reverse chronological order (newest first).
/// `depth` limits how many commits to fetch (None = all).
///
/// Only commits that touch files under `scan_root` are returned.
/// File paths are made relative to `scan_root` to match the
/// filesystem scanner output.
pub fn fetch_commits(scan_root: &str, depth: Option<usize>) -> Vec<Commit> {
    // Find the git repo root
    let repo_root = match find_git_root(scan_root) {
        Some(r) => r,
        None => return Vec::new(),
    };

    // Compute the repo-relative subdirectory to filter on
    let repo = repo_root.trim_end_matches('/');
    let scan = scan_root.trim_end_matches('/');
    let repo_rel_path = scan.strip_prefix(&format!("{}/", repo)).unwrap_or("");

    let mut cmd = Command::new("git");
    cmd.arg("-C")
        .arg(&repo_root)
        .arg("log")
        .arg("--name-only")
        .arg("--pretty=format:%H %ct")
        .arg("--no-merges");

    if let Some(n) = depth {
        cmd.arg(format!("-{}", n));
    }

    // Only show commits that touched files under our scan directory
    if !repo_rel_path.is_empty() {
        cmd.arg("--").arg(repo_rel_path);
    }

    let output = match cmd.output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    if !output.status.success() {
        return Vec::new();
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let commits = parse_git_log(&text);

    // Make file paths relative to scan_root by stripping the repo_rel_path prefix
    let strip_prefix = if repo_rel_path.is_empty() {
        String::new()
    } else {
        format!("{}/", repo_rel_path)
    };

    commits
        .into_iter()
        .map(|mut c| {
            c.files = c
                .files
                .into_iter()
                .filter_map(|f| strip_git_path(&f, &strip_prefix))
                .collect();
            c
        })
        .collect()
}

/// Find the git repository root containing `path`.
fn find_git_root(path: &str) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .arg("rev-parse")
        .arg("--show-toplevel")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Strip the repo-relative prefix from a git log path.
fn strip_git_path(path: &str, strip_prefix: &str) -> Option<String> {
    if strip_prefix.is_empty() {
        return Some(path.to_string());
    }
    path.strip_prefix(strip_prefix).map(|s| s.to_string())
}

/// Check if a line looks like a commit header:
/// 40 hex chars, space, then a number (unix timestamp).
fn is_commit_header(line: &str) -> bool {
    // Minimum: 40 hex + 1 space + 1 digit = 42
    if line.len() < 42 {
        return false;
    }
    let bytes = line.as_bytes();
    // First 40 chars must be hex
    if !bytes.iter().take(40).all(|b| b.is_ascii_hexdigit()) {
        return false;
    }
    // Char 40 must be space
    if bytes[40] != b' ' {
        return false;
    }
    // Rest must be digits
    if !bytes.iter().skip(41).all(|b| b.is_ascii_digit()) {
        return false;
    }
    true
}

/// Parse the output of `git log --name-only --pretty=format:"%H %ct"`.
///
/// Format:
///   <40-char-hash> <unix-timestamp>
///   file1
///   file2
///
///   <40-char-hash> <unix-timestamp>
///   file3
fn parse_git_log(text: &str) -> Vec<Commit> {
    let mut commits = Vec::new();
    let mut current_hash: Option<String> = None;
    let mut current_ts: u64 = 0;
    let mut current_files: Vec<String> = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            // Blank line = end of current commit's file list
            if let Some(hash) = current_hash.take() {
                commits.push(Commit {
                    hash,
                    timestamp: current_ts,
                    files: std::mem::take(&mut current_files),
                });
            }
            continue;
        }

        if is_commit_header(trimmed) {
            // Save previous commit if any
            if let Some(hash) = current_hash.take() {
                commits.push(Commit {
                    hash,
                    timestamp: current_ts,
                    files: std::mem::take(&mut current_files),
                });
            }
            // Start new commit
            current_hash = Some(trimmed[..40].to_string());
            current_ts = trimmed[41..].parse().unwrap_or(0);
        } else if current_hash.is_some() {
            // File path belonging to current commit
            current_files.push(trimmed.to_string());
        }
    }

    // Flush last commit (may not have trailing newline)
    if let Some(hash) = current_hash.take() {
        commits.push(Commit {
            hash,
            timestamp: current_ts,
            files: std::mem::take(&mut current_files),
        });
    }

    commits
}

/// Build a mapping from file path → last commit hash that modified it.
/// Iterates commits newest-first; first occurrence wins.
pub fn build_file_commit_map(commits: &[Commit]) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
    for commit in commits {
        for file in &commit.files {
            map.entry(file.clone())
                .or_insert_with(|| commit.hash.clone());
        }
    }
    map
}

/// Get commits that touched a specific file (newest first).
#[allow(dead_code)]
pub fn file_history<'a>(commits: &'a [Commit], file_path: &str) -> Vec<&'a Commit> {
    commits
        .iter()
        .filter(|c| c.files.iter().any(|f| f == file_path))
        .collect()
}

/// Get all files changed in the last N commits.
#[allow(dead_code)]
pub fn recent_changed_files(commits: &[Commit]) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    for commit in commits {
        for file in &commit.files {
            seen.insert(file.clone());
        }
    }
    seen.into_iter().collect()
}
