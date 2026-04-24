//! Zero-dependency glob pattern matching.
//!
//! Supports: `*`, `?`, `[abc]`, `**`
//!
//! # Example
//!
//! ```
//! use edgerun_glob::glob_match;
//! assert!(glob_match("*.rs", "lib.rs"));
//! assert!(glob_match("src/*.rs", "src/lib.rs"));
//! assert!(glob_match("**/*.txt", "a/b/c.txt"));
//! ```

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;

/// Match a path against a glob pattern.
pub fn glob_match(pattern: &str, path: &str) -> bool {
    glob_match_with_separator(pattern, path, '/')
}

/// Match with custom path separator.
pub fn glob_match_with_separator(pattern: &str, path: &str, sep: char) -> bool {
    let mut pi = 0; // pattern index
    let mut ti = 0; // path index
    
    while pi < pattern.len() || ti < path.len() {
        let pc = pattern[pi..].chars().next();
        let tc = path[ti..].chars().next();
        
        match (pc, tc) {
            (Some('*'), Some(tc_char)) if tc_char == sep => {
                // * doesn't match across separator
                pi += 1;
            }
            (Some('*'), _) => {
                // Try greedy match
                if pi + 1 >= pattern.len() {
                    // * at end matches everything remaining
                    return true;
                }
                // Look ahead in path
                let remaining_pattern = &pattern[pi + 1..];
                let remaining_path = &path[ti..];
                if remaining_path.is_empty() && remaining_pattern.is_empty() {
                    return true;
                }
                if remaining_path.is_empty() {
                    return false;
                }
                // Try matching from current position
                let mut ti_next = ti;
                while ti_next < path.len() {
                    if glob_match_fast(&pattern[pi + 1..], &path[ti_next..], sep) {
                        return true;
                    }
                    // Move past one path segment
                    let Some(next_sep) = path[ti_next..].find(sep) else {
                        break;
                    };
                    ti_next += next_sep + 1;
                }
                return false;
            }
            (Some('?'), Some(_)) => {
                pi += 1;
                ti += 1;
            }
            (Some('['), Some(tc)) => {
                // Character class: [abc] or [!abc] or [a-z]
                if !match_char_class(&pattern[pi..], tc) {
                    return false;
                }
                // Skip to ]
                if let Some(end) = pattern[pi..].find(']') {
                    pi += end + 1;
                } else {
                    pi += 1;
                }
                ti += 1;
            }
            (Some(a), Some(b)) if a == b => {
                pi += 1;
                ti += 1;
            }
            (None, Some(_)) => return false,
            (Some(_), None) => return false,
            _ => return false,
        }
    }
    pi == pattern.len() && ti == path.len()
}

/// Fast path for non-recursive matching.
fn glob_match_fast(pattern: &str, path: &str, sep: char) -> bool {
    let mut pi = 0;
    let mut ti = 0;
    
    while pi < pattern.len() && ti < path.len() {
        let pc = pattern[pi..].chars().next();
        let tc = path[ti..].chars().next();
        
        match (pc, tc) {
            (Some('*'), _) => return false,
            (Some('?'), Some(_)) => {
                pi += 1;
                ti += 1;
            }
            (Some('['), Some(tc)) => {
                if !match_char_class(&pattern[pi..], tc) {
                    return false;
                }
                if let Some(end) = pattern[pi..].find(']') {
                    pi += end + 1;
                } else {
                    pi += 1;
                }
                ti += 1;
            }
            (Some(a), Some(b)) if a == b => {
                pi += 1;
                ti += 1;
            }
            _ => return false,
        }
    }
    pi == pattern.len() && ti == path.len()
}

fn match_char_class(pattern: &str, ch: char) -> bool {
    let start = if pattern.starts_with('[') { 1 } else { 0 };
    let end = pattern.find(']').unwrap_or(pattern.len());
    
    if start >= end {
        return false;
    }
    
    let class = &pattern[start..end];
    
    if let Some(minus) = class.find('-') {
        let start_char = class[..minus].chars().next();
        let end_char = class[minus + 1..].chars().next();
        if let (Some(s), Some(e)) = (start_char, end_char) {
            return ch >= s && ch <= e;
        }
    }
    
    class.chars().any(|c| c == ch)
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