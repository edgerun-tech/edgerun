//! Zero-dependency glob pattern matching.
//!
//! Supports: `*`, `?`, `[abc]`, `**`

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

pub fn glob_match(pattern: &str, path: &str) -> bool {
    glob_match_with_separator(pattern, path, '/')
}

pub fn glob_match_with_separator(pattern: &str, path: &str, sep: char) -> bool {
    if pattern == "**" {
        return true;
    }
    if let Some(rest) = pattern.strip_prefix("**") {
        // ** matches zero or more path segments
        if rest.is_empty() {
            return true;
        }
        // rest starts with sep? Skip it for matching
        let match_pat = if rest.starts_with(sep) {
            &rest[1..]
        } else {
            rest
        };
        if match_pat.is_empty() {
            return true;
        }
        // Try matching from each position in path (including zero length = ** matches empty)
        for start in 0..=path.len() {
            if matchGlob(match_pat, 0, &path[start..], 0, sep) {
                return true;
            }
            // Stop after trying without separator if next pattern has no more
            if start < path.len() && !path[start..].contains(sep) {
                break;
            }
        }
        return false;
    }
    matchGlob(pattern, 0, path, 0, sep)
}

fn matchGlob(pat: &str, pi: usize, path: &str, ti: usize, sep: char) -> bool {
    let mut p_idx = pi;
    let mut t_idx = ti;

    while p_idx < pat.len() || t_idx < path.len() {
        let pc = pat.get(p_idx..p_idx + 1);
        let tc = path.get(t_idx..t_idx + 1);

        match (pc, tc) {
            (Some("*"), _) if p_idx + 1 >= pat.len() => {
                return true;
            }
            (Some("*"), Some(tc)) => {
                let rest_pat = &pat[p_idx + 1..];
                if rest_pat.is_empty() {
                    return true;
                }
                let next_is_sep = rest_pat.starts_with(sep);

                if next_is_sep {
                    // * followed by /: only match at directory boundary
                    // Either no more path, or find /
                    if let Some(pos) = path[t_idx..].find(sep) {
                        let remaining = &path[t_idx + pos + 1..];
                        if matchGlob(rest_pat, 0, remaining, 0, sep) {
                            return true;
                        }
                    }
                    return false;
                } else {
                    // No sep after *, can match any chars but NOT across /
                    // Try empty first (for * matching nothing), then find / to stop
                    if rest_pat.is_empty() || matchGlob(rest_pat, 0, &path[t_idx..], 0, sep) {
                        return true;
                    }
                    // Try one char at a time, but STOP at first /
                    let mut i = t_idx + 1;
                    while i < path.len() {
                        if path.chars().nth(i) == Some(sep) {
                            return false; // Can't cross separator
                        }
                        if matchGlob(rest_pat, 0, &path[i..], 0, sep) {
                            return true;
                        }
                        i += 1;
                    }
                    return false;
                }
            }
            (Some("?"), Some(_)) => {
                p_idx += 1;
                t_idx += 1;
            }
            (Some("["), Some(t)) => {
                let ch = t.chars().next().unwrap();
                if !matchCharClass(pat, p_idx, ch) {
                    return false;
                }
                if let Some(end) = pat[p_idx..].find(']') {
                    p_idx += end + 1;
                } else {
                    p_idx += 1;
                }
                t_idx += 1;
            }
            (Some(p), Some(t)) if p == t => {
                p_idx += 1;
                t_idx += 1;
            }
            (None, Some(_)) | (Some(_), None) => {
                return false;
            }
            _ => return false,
        }
    }
    p_idx == pat.len() && t_idx == path.len()
}

fn matchCharClass(pat: &str, start: usize, ch: char) -> bool {
    let rest = &pat[start..];
    let Some(end) = rest.find(']') else {
        return false;
    };
    let class = &rest[1..end];

    if class.starts_with('!') || class.starts_with('^') {
        let inner = &class[1..];
        !classContains(inner, ch)
    } else {
        classContains(class, ch)
    }
}

fn classContains(class: &str, ch: char) -> bool {
    let mut i = 0;
    while i < class.len() {
        let c = class[i..].chars().next().unwrap();
        if i + 2 <= class.len() {
            let next_chars: String = class.chars().skip(i).take(2).collect();
            if next_chars.contains('-') && i + 3 <= class.len() {
                let range_end = class.chars().nth(i + 2).unwrap();
                if ch >= c && ch <= range_end {
                    return true;
                }
                i += 3;
                continue;
            }
        }
        if c == ch {
            return true;
        }
        i += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_star() {
        assert!(glob_match("*.rs", "lib.rs"));
        assert!(!glob_match("*.rs", "dir/lib.rs"));
        assert!(glob_match("*.txt", "a.txt"));
    }

    #[test]
    fn test_double_star() {
        assert!(glob_match("**/*.rs", "lib.rs"));
        assert!(glob_match("**/*.rs", "src/lib.rs"));
        assert!(glob_match("**/*.rs", "a/b/lib.rs"));
    }

    #[test]
    fn test_question() {
        assert!(glob_match("?.rs", "a.rs"));
        assert!(!glob_match("?.rs", "ab.rs"));
    }
}
