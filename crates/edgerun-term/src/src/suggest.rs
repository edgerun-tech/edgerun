use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use crate::tab::{CwdHistoryEntry, Tab};
use crate::tab_current_dir;

pub fn path_completion_suggestions(
    prefix: &str,
    current_dir: Option<&Path>,
    limit: usize,
) -> Vec<String> {
    let token = match prefix.split_whitespace().last() {
        Some(t) if looks_like_path(t) => t,
        _ => return Vec::new(),
    };

    let (base_input, name_prefix) = if let Some(idx) = token.rfind('/') {
        (&token[..=idx], &token[idx + 1..])
    } else {
        return Vec::new();
    };

    let base_path = Path::new(base_input);
    let resolved = if base_path.is_absolute() {
        base_path.to_path_buf()
    } else if let Some(cwd) = current_dir {
        cwd.join(base_path)
    } else {
        return Vec::new();
    };

    if !resolved.is_dir() {
        return Vec::new();
    }

    let mut results = Vec::new();
    if let Ok(entries) = fs::read_dir(&resolved) {
        for entry in entries.flatten() {
            if results.len() >= limit {
                break;
            }
            let name = match entry.file_name().into_string() {
                Ok(n) => n,
                Err(_) => continue,
            };
            if !name.starts_with(name_prefix) {
                continue;
            }
            let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
            let mut suggestion = String::new();
            suggestion.push_str(base_input);
            suggestion.push_str(&name);
            if is_dir {
                suggestion.push('/');
            }
            results.push(suggestion);
        }
    }

    results
}

pub fn recent_dirs_for_tab(
    active: usize,
    tabs: &[Tab],
    history: &[CwdHistoryEntry],
) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();

    if let Some(tab) = tabs.get(active) {
        if let Some(cwd) = tab_current_dir(tab) {
            seen.insert(cwd.clone());
            dirs.push(cwd);
        }
    }

    for entry in history.iter().rev().filter(|h| h.tab == active) {
        if seen.insert(entry.path.clone()) {
            dirs.push(entry.path.clone());
        }
    }

    dirs
}

pub fn looks_like_path(token: &str) -> bool {
    token.contains('/') || token.starts_with("./") || token.starts_with("../")
}

pub fn guess_command_paths(
    entry: &str,
    current_dir: Option<&Path>,
    recent_dirs: &[PathBuf],
) -> String {
    let mut changed = false;
    let mut tokens = Vec::new();

    for tok in entry.split_whitespace() {
        let replace = if tok.starts_with('-') || tok.starts_with('~') || tok.contains('$') {
            None
        } else if tok.starts_with('/') {
            None
        } else if !looks_like_path(tok) {
            None
        } else {
            let rel = Path::new(tok);
            if current_dir
                .map(|cwd| cwd.join(rel).exists())
                .unwrap_or(false)
            {
                None
            } else {
                recent_dirs.iter().find_map(|dir| {
                    let candidate = dir.join(rel);
                    if candidate.exists() {
                        return candidate.to_str().map(|s| s.to_string());
                    }
                    if let Some(parent) = dir.parent() {
                        let sibling = parent.join(rel);
                        if sibling.exists() {
                            return sibling.to_str().map(|s| s.to_string());
                        }
                    }
                    None
                })
            }
        };

        if let Some(new_tok) = replace {
            tokens.push(new_tok);
            changed = true;
        } else {
            tokens.push(tok.to_string());
        }
    }

    if changed {
        tokens.join(" ")
    } else {
        entry.to_string()
    }
}

pub fn fix_command_paths(
    entry: &str,
    active: usize,
    tabs: &[Tab],
    history: &[CwdHistoryEntry],
) -> String {
    let current_dir_buf = tabs.get(active).and_then(tab_current_dir);
    let current_dir = current_dir_buf.as_deref();
    let recent_dirs = recent_dirs_for_tab(active, tabs, history);
    guess_command_paths(entry, current_dir, &recent_dirs)
}
