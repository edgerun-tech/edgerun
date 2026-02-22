use std::fs;
use std::path::{Path, PathBuf};

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
        let replace = if tok.starts_with('-')
            || tok.starts_with('~')
            || tok.contains('$')
            || tok.starts_with('/')
            || !looks_like_path(tok)
        {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn guess_command_paths_leaves_existing_path() {
        let current = tempfile::tempdir().unwrap();
        let path = current.path().join("data/input.txt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"hi").unwrap();

        let cmd = "cat data/input.txt";
        let fixed = guess_command_paths(cmd, Some(current.path()), &[]);
        assert_eq!(fixed, cmd);
    }

    #[test]
    fn guess_command_paths_fills_relative_path() {
        let current = tempfile::tempdir().unwrap();
        let path = current.path().join("data/input.txt");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"hi").unwrap();

        let cmd = "cat data/input.txt";
        let fixed = guess_command_paths(cmd, None, &[path.parent().unwrap().to_path_buf()]);
        assert!(fixed.contains(path.to_string_lossy().as_ref()));
    }
}
