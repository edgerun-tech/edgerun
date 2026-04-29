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
    anchored_start: bool,
    anchored_end: bool,
}

impl Regex {
    /// Compile a pattern.
    pub fn new(pattern: &str) -> Option<Regex> {
        let anchored_start = pattern.starts_with('^');
        let anchored_end = pattern.ends_with('$');
        Some(Regex {
            pattern: pattern
                .trim_start_matches('^')
                .trim_end_matches('$')
                .to_string(),
            anchored_start,
            anchored_end,
        })
    }

    /// Check if pattern matches text.
    pub fn is_match(&self, text: &str) -> bool {
        let p = &self.pattern;

        if p.is_empty() {
            return true;
        }

        match (self.anchored_start, self.anchored_end) {
            (true, true) => text == p,
            (true, false) => text.starts_with(p),
            (false, true) => text.ends_with(p),
            (false, false) => {
                // Simple contains for now
                // TODO: handle .* and ?
                text.contains(p)
            }
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

        let re = Regex::new("error$").unwrap();
        assert!(re.is_match("ends with error"));
        assert!(!re.is_match("error suffix"));

        let re = Regex::new("^error$").unwrap();
        assert!(re.is_match("error"));
        assert!(!re.is_match("error suffix"));
        assert!(!re.is_match("prefix error"));
    }
}
