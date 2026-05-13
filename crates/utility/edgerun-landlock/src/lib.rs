//! EdgeRun-owned Landlock facade.
//!
//! Codex currently carries Landlock failures through protocol error variants,
//! while sandbox command construction lives in `codex_sandboxing`. This crate
//! keeps that protocol surface owned without pulling the upstream landlock
//! crate into the native binary.

extern crate std;

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RulesetError {
    message: String,
}

impl RulesetError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for RulesetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RulesetError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathFdError {
    message: String,
}

impl PathFdError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PathFdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for PathFdError {}
