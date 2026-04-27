//! Grep and search functionality

use alloc::string::String;

use crate::PathBuf;

/// A single grep match result
#[derive(Debug, Clone)]
pub struct GrepMatch {
    pub path: PathBuf,
    pub line_number: usize,
    pub line: String,
}

impl GrepMatch {
    /// Format as grep-style output: path:line:number:content
    pub fn format(&self) -> String {
        alloc::format!(
            "{}:{}:{}",
            crate::vfs::path_display(&self.path),
            self.line_number,
            self.line
        )
    }
}
