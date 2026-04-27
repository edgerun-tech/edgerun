//! Git integration for automatic rollback and staging.

use std::format;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::string::{String, ToString};
use std::vec;
use std::vec::Vec;

/// Run a git command in the given directory. Returns (stdout, stderr, success).
fn run_git(dir: &Path, args: &[&str]) -> (String, String, bool) {
    let output = Command::new("git").args(args).current_dir(dir).output();
    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout).to_string();
            let stderr = String::from_utf8_lossy(&o.stderr).to_string();
            (stdout, stderr, o.status.success())
        }
        Err(e) => (String::new(), e.to_string(), false),
    }
}

/// Check if a directory is a git repo (or has a parent that is).
pub fn find_git_root(dir: &Path) -> Option<PathBuf> {
    let mut cur = if dir.is_dir() {
        dir.to_path_buf()
    } else {
        dir.parent()?.to_path_buf()
    };
    loop {
        if cur.join(".git").exists() {
            return Some(cur);
        }
        if !cur.pop() {
            return None;
        }
    }
}

/// Create a git worktree-style snapshot: stash everything, apply edits,
/// and roll back on failure.
///
/// Strategy: before edits, `git add` all target files (or `git add -u` for
/// existing tracked files). After edits, if cargo check fails,
/// `git checkout -- <files>` to restore originals.
pub struct GitSafety {
    root: PathBuf,
    /// Files modified since we started this batch (for rollback).
    pub modified: Vec<String>,
}

impl GitSafety {
    pub fn new(dir: &Path) -> Option<Self> {
        let root = find_git_root(dir)?;
        Some(Self {
            root,
            modified: Vec::new(),
        })
    }

    /// Record a file as modified (for potential rollback).
    pub fn track(&mut self, path: &Path) {
        // Convert to relative path from git root
        if let Ok(rel) = path.strip_prefix(&self.root) {
            self.modified.push(rel.to_string_lossy().to_string());
        }
    }

    /// Stage only the files tracked by this GitSafety instance.
    pub fn stage_all(&self) -> (bool, String) {
        if self.modified.is_empty() {
            return (true, "no files modified".to_string());
        }
        let files: Vec<&str> = self.modified.iter().map(|s| s.as_str()).collect();
        // Try staging specific files first
        let mut args = vec!["add", "--"];
        for f in &files {
            args.push(f);
        }
        let (_, stderr, ok) = run_git(&self.root, &args);
        if !ok {
            // For untracked new files, add without --
            let mut args = vec!["add"];
            for f in &files {
                args.push(f);
            }
            let (_, stderr, ok2) = run_git(&self.root, &args);
            if !ok2 {
                return (false, format!("git add failed: {stderr}"));
            }
        }
        (true, format!("staged {} file(s)", self.modified.len()))
    }

    /// Rollback all tracked files to their pre-edit state.
    pub fn rollback(&self) -> (bool, String) {
        if self.modified.is_empty() {
            return (true, "nothing to rollback".to_string());
        }
        let files: Vec<&str> = self.modified.iter().map(|s| s.as_str()).collect();
        let mut args = vec!["checkout", "HEAD", "--"];
        for f in &files {
            args.push(f);
        }
        let (_, stderr, ok) = run_git(&self.root, &args);
        if ok {
            (true, format!("rolled back {} file(s)", files.len()))
        } else {
            (false, stderr)
        }
    }

    /// Reset the staging area (unstage without discarding working tree changes).
    pub fn unstage(&self) {
        run_git(&self.root, &["reset", "HEAD", "--"]);
    }
}

/// Get the current git diff (for debugging/logging).
pub fn diff(root: &Path) -> String {
    let (stdout, _, _) = run_git(root, &["diff", "--stat"]);
    stdout
}

/// Commit the staged changes with a message.
pub fn commit(root: &Path, message: &str) -> (bool, String) {
    let (_, _, ok) = run_git(root, &["commit", "-m", message]);
    if ok {
        (true, "committed".to_string())
    } else {
        (false, "commit failed (may be nothing staged)".to_string())
    }
}
