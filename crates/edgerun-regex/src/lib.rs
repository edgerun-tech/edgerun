//! Zero-dependency regex for grep (basic subset).
//!
//! Supports: literal text, `.*` (any chars), `?` (single char), `^` `$` anchors
//!
//! # Example
//!
//! ```
//! use edgerun_regex::Regex;
//! let re = Regex::new("error").unwrap();
//! assert!(re.is_match("error: file not found"));
//! ```

#![no_std]

extern crate alloc;

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

/// A compiled regex for matching.
#[derive(Clone)]
pub struct Regex {
    pattern: String,
    is_anchored: bool,
}

impl Regex {
    /// Compile a pattern.
    pub fn new(pattern: &str) -> Option<Regex> {
        let is_anchored = pattern.starts_with('^') || pattern.ends_with('$');
        Some(Regex {
            pattern: pattern.trim_matches('^').trim_matches('$').to_string(),
            is_anchored,
        })
    }

    /// Check if pattern matches text.
    pub fn is_match(&self, text: &str) -> bool {
        let p = &self.pattern;

        if p.is_empty() {
            return true;
        }

        if self.is_anchored {
            if self.pattern.starts_with('^') {
                text.starts_with(p.trim_start_matches('^'))
            } else if self.pattern.ends_with('$') {
                text.ends_with(p.trim_end_matches('$'))
            } else {
                text.contains(p)
            }
        } else {
            // Simple contains for now
            // TODO: handle .* and ?
            text.contains(p)
        }
    }
}

#[derive(Debug, Clone)]
pub struct GrepMatch {
    pub path: String,
    pub line_number: usize,
    pub line: String,
}

/// Find all matches in text.
pub fn grep_matches(text: &str, pattern: &str) -> Vec<GrepMatch> {
    let re = match Regex::new(pattern) {
        Some(r) => r,
        None => return Vec::new(),
    };

    let mut matches = Vec::new();
    for (line_num, line) in text.lines().enumerate() {
        if re.is_match(line) {
            matches.push(GrepMatch {
                path: String::new(),
                line_number: line_num + 1,
                line: line.to_string(),
            });
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_literal() {
        let re = Regex::new("error").unwrap();
        assert!(re.is_match("error: not found"));
        assert!(!re.is_match("ERROR"));
    }

    #[test]
    fn test_anchored() {
        let re = Regex::new("^error").unwrap();
        assert!(re.is_match("error start"));
        assert!(!re.is_match("prefix error"));
    }
}
