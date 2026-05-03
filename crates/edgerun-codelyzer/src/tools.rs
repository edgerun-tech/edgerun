//! Tool implementations for context-aware chat.
//! Provides search_codebase, read_file, list_files, and lint_file.

#![allow(clippy::unwrap_used)]

use std::collections::HashMap;

use grep::{
    regex::RegexMatcher,
    searcher::{Searcher, SinkMatch},
};
use edgerun_glob::glob_match;

use crate::diagnostics;

#[allow(dead_code)]
pub struct ToolResult {
    pub tool_name: String,
    pub args: String,
    pub output: String,
    pub content: Option<String>,
}

#[allow(dead_code)]
pub enum ToolCall {
    SearchCodebase { query: String },
    ReadFile { path: String },
    ListFiles { pattern: String },
    EditFile { path: String, content: String },
    LintFile { path: String },
}

#[allow(dead_code)]
impl ToolCall {
    /// Parse a tool specification from a string like: tool_name(arg="value",
    /// arg2="value2")
    pub fn from_spec(spec: &str) -> Option<Self> {
        let spec = spec.trim();
        if let Some(paren_pos) = spec.find('(') {
            let name = &spec[..paren_pos];
            let args_str = &spec[paren_pos + 1..];
            // Remove trailing ) if present
            let args_str = args_str.strip_suffix(')').unwrap_or(args_str);

            let args = parse_key_value_args(args_str);

            match name {
                "search_codebase" => {
                    let query = args.get("query").cloned().unwrap_or_default();
                    if query.is_empty() {
                        return None;
                    }
                    Some(ToolCall::SearchCodebase { query })
                }
                "read_file" => {
                    let path = args.get("path").cloned().unwrap_or_default();
                    if path.is_empty() {
                        return None;
                    }
                    Some(ToolCall::ReadFile { path })
                }
                "list_files" => {
                    let pattern = args.get("pattern").cloned().unwrap_or_default();
                    if pattern.is_empty() {
                        return None;
                    }
                    Some(ToolCall::ListFiles { pattern })
                }
                "edit_file" => {
                    let path = args.get("path").cloned().unwrap_or_default();
                    let content = args.get("content").cloned().unwrap_or_default();
                    if path.is_empty() {
                        return None;
                    }
                    Some(ToolCall::EditFile { path, content })
                }
                "lint_file" => {
                    let path = args.get("path").cloned().unwrap_or_default();
                    if path.is_empty() {
                        return None;
                    }
                    Some(ToolCall::LintFile { path })
                }
                _ => None,
            }
        } else {
            None
        }
    }

    pub fn execute(&self) -> ToolResult {
        match self {
            ToolCall::SearchCodebase { query } => {
                let output = search_codebase(query);
                ToolResult {
                    tool_name: "search_codebase".to_string(),
                    args: format!("query=\"{}\"", query),
                    output,
                    content: None,
                }
            }
            ToolCall::ReadFile { path } => {
                let output = read_file(path);
                ToolResult {
                    tool_name: "read_file".to_string(),
                    args: format!("path=\"{}\"", path),
                    output,
                    content: None,
                }
            }
            ToolCall::ListFiles { pattern } => {
                let output = list_files(pattern);
                ToolResult {
                    tool_name: "list_files".to_string(),
                    args: format!("pattern=\"{}\"", pattern),
                    output,
                    content: None,
                }
            }
            ToolCall::EditFile { path, content } => {
                let output = edit_file(path, content);
                ToolResult {
                    tool_name: "edit_file".to_string(),
                    args: format!("path=\"{}\"", path),
                    output,
                    content: Some(content.clone()),
                }
            }
            ToolCall::LintFile { path } => {
                let output = lint_file(path);
                ToolResult {
                    tool_name: "lint_file".to_string(),
                    args: format!("path=\"{}\"", path),
                    output,
                    content: None,
                }
            }
        }
    }
}

/// Parse key=value arguments from a string like: key="value", key2="value2"
#[allow(dead_code)]
fn parse_key_value_args(s: &str) -> HashMap<String, String> {
    let mut args = HashMap::new();
    let mut chars = s.chars().peekable();

    loop {
        // Skip whitespace
        while chars.peek().is_some_and(|c| c.is_whitespace()) {
            chars.next();
        }

        // Read key
        let mut key = String::new();
        while let Some(&c) = chars.peek() {
            if c == '=' || c.is_whitespace() {
                break;
            }
            key.push(c);
            chars.next();
        }

        if key.is_empty() {
            break;
        }

        // Skip to '='
        while chars.peek().is_some_and(|c| *c == ' ') {
            chars.next();
        }

        if chars.peek() != Some(&'=') {
            break;
        }
        chars.next(); // consume '='

        // Skip whitespace after '='
        while chars.peek().is_some_and(|c| *c == ' ') {
            chars.next();
        }

        // Read value (quoted or unquoted)
        let value = if chars.peek() == Some(&'"') {
            chars.next(); // consume opening quote
            let mut val = String::new();
            loop {
                match chars.next() {
                    Some('"') => break,
                    Some(c) => val.push(c),
                    None => break,
                }
            }
            val
        } else {
            let mut val = String::new();
            while let Some(&c) = chars.peek() {
                if c == ',' || c.is_whitespace() {
                    break;
                }
                val.push(c);
                chars.next();
            }
            val
        };

        args.insert(key, value);

        // Skip comma and whitespace
        while let Some(&c) = chars.peek() {
            if c == ',' || c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }

        if chars.peek().is_none() {
            break;
        }
    }

    args
}

/// Search all source files for code matching the query.
#[allow(dead_code)]
fn search_codebase(query: &str) -> String {
    let root_dir = match get_project_root() {
        Some(dir) => dir,
        None => return "Error: No project root available.".to_string(),
    };

    let matcher = match RegexMatcher::new_line_matcher(query) {
        Ok(m) => m,
        Err(e) => return format!("Error compiling regex: {}", e),
    };

    let mut results: Vec<String> = Vec::new();
    let extensions = &["c", "h", "rs", "ts", "tsx", "js", "jsx", "mjs", "py", "go", "java"];

    // Walk directory tree manually since we're using edgerun-glob for matching
    let mut dirs_to_visit = vec![std::path::PathBuf::from(&root_dir)];
    let skip_dirs: &[&str] = &["target", "node_modules", ".git", "__pycache__", ".venv"];

    while let Some(dir) = dirs_to_visit.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if skip_dirs.contains(&name) {
                            continue;
                        }
                    }
                    dirs_to_visit.push(path);
                } else if path.is_file() {
                    // Check extension
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if !extensions.contains(&ext) {
                        continue;
                    }

                    // Search the file with grep
                    let mut searcher = Searcher::new();
                    let mut sink = LineSink {
                        path: path.clone(),
                        root_dir: root_dir.clone(),
                        results: Vec::new(),
                        limit: 20, // Limit matches per file
                    };

                    if searcher.search_path(&matcher, &path, &mut sink).is_ok()
                        && !sink.results.is_empty()
                    {
                        results.extend(sink.results);
                        if results.len() >= 100 {
                            break; // Overall limit
                        }
                    }
                }
            }
        }
        if results.len() >= 100 {
            break;
        }
    }

    if results.is_empty() {
        format!("No results found for query: {}", query)
    } else {
        let mut output = format!("Found {} results for '{}':\n", results.len(), query);
        for (i, line) in results.iter().enumerate().take(50) {
            output.push_str(&format!("{}. {}\n", i + 1, line));
        }
        if results.len() > 50 {
            output.push_str(&format!("... and {} more results", results.len() - 50));
        }
        output
    }
}

#[allow(dead_code)]
struct LineSink {
    path: std::path::PathBuf,
    root_dir: String,
    results: Vec<String>,
    limit: usize,
}

impl grep::searcher::Sink for LineSink {
    type Error = std::io::Error;

    fn matched(
        &mut self,
        _searcher: &Searcher,
        mat: &SinkMatch<'_>,
    ) -> Result<bool, Self::Error> {
        if self.results.len() >= self.limit {
            return Ok(false);
        }

        let line = String::from_utf8_lossy(mat.bytes()).to_string();
        let line_num = mat.line_number().unwrap_or(0);
        let rel_path = self
            .path
            .strip_prefix(&self.root_dir)
            .unwrap_or(&self.path)
            .to_string_lossy();

        self.results
            .push(format!("{}:{}: {}", rel_path, line_num, line.trim()));
        Ok(true)
    }
}

/// Read the full content of a file from the project root.
#[allow(dead_code)]
fn read_file(path: &str) -> String {
    let root_dir = match get_project_root() {
        Some(dir) => dir,
        None => return "Error: No project root available.".to_string(),
    };

    // Clean the path to prevent directory traversal
    let clean_path = path.trim_start_matches('/');
    let full_path = std::path::Path::new(&root_dir).join(clean_path);

    // Verify the path is within the project root
    if !full_path.starts_with(&root_dir) {
        return "Error: Path is outside the project root.".to_string();
    }

    match std::fs::read_to_string(&full_path) {
        Ok(content) => {
            // Truncate very long files
            if content.len() > 10000 {
                format!(
                    "{}\n\n... (file truncated, showing first 10000 of {} chars)",
                    &content[..10000],
                    content.len()
                )
            } else {
                content
            }
        }
        Err(e) => format!("Error reading file '{}': {}", path, e),
    }
}

/// List files matching a glob pattern.
#[allow(dead_code)]
fn list_files(pattern: &str) -> String {
    let root_dir = match get_project_root() {
        Some(dir) => dir,
        None => return "Error: No project root available.".to_string(),
    };

    let mut files: Vec<String> = Vec::new();
    let mut dirs_to_visit = vec![std::path::PathBuf::from(&root_dir)];

    while let Some(dir) = dirs_to_visit.pop() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs_to_visit.push(path);
                } else if path.is_file() {
                    let rel_path = path
                        .strip_prefix(&root_dir)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .to_string();
                    if glob_match(pattern, &rel_path) {
                        files.push(rel_path);
                    }
                }
            }
        }
        if files.len() >= 100 {
            break;
        }
    }

    if files.is_empty() {
        format!("No files found matching pattern: {}", pattern)
    } else {
        let mut output = format!("Found {} files matching '{}':\n", files.len(), pattern);
        for file in &files {
            output.push_str(&format!("  - {}\n", file));
        }
        output
    }
}

/// Edit a file: validate path, read current content for diff preview, and
/// return confirmation. This does NOT write the file — it returns the proposed
/// edit for UI review.
#[allow(dead_code)]
fn edit_file(path: &str, content: &str) -> String {
    let root_dir = match get_project_root() {
        Some(dir) => dir,
        None => return "Error: No project root available.".to_string(),
    };

    let clean_path = path.trim_start_matches('/');
    let full_path = std::path::Path::new(&root_dir).join(clean_path);

    if !full_path.starts_with(&root_dir) {
        return "Error: Path is outside the project root.".to_string();
    }

    let current_content = match std::fs::read_to_string(&full_path) {
        Ok(c) => c,
        Err(e) => return format!("Error reading current file '{}': {}", path, e),
    };

    let old_len = current_content.len();
    let new_len = content.len();
    let diff_lines = compute_line_diff(&current_content, content);

    format!(
        "Proposed edit for '{}':\n- Current: {} bytes\n- Proposed: {} bytes\n- Changes: {} lines added/modified\n\nUse the UI to accept or reject this edit.",
        path, old_len, new_len, diff_lines
    )
}

/// Compute a simple line-level diff count: how many lines differ between old
/// and new.
#[allow(dead_code)]
fn compute_line_diff(old: &str, new: &str) -> usize {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();
    let max_len = old_lines.len().max(new_lines.len());
    let mut changes = 0;
    for i in 0..max_len {
        let old_line = old_lines.get(i).copied().unwrap_or("");
        let new_line = new_lines.get(i).copied().unwrap_or("");
        if old_line != new_line {
            changes += 1;
        }
    }
    changes
}

/// Get the project root directory.
/// This is a simplified version that looks for common project markers.
#[allow(dead_code)]
fn get_project_root() -> Option<String> {
    // For now, just use current directory
    std::env::current_dir()
        .ok()
        .map(|p| p.to_string_lossy().to_string())
}

/// Run linters on a file and return formatted output.
#[allow(dead_code)]
fn lint_file(path: &str) -> String {
    let root_dir = match get_project_root() {
        Some(dir) => dir,
        None => return "Error: No project root available.".to_string(),
    };

    let diags = diagnostics::lint_single_file(&root_dir, path);
    if diags.is_empty() {
        return format!("No linting issues found for '{}'.", path);
    }

    let mut output = format!("Found {} linting issue(s) in '{}':\n\n", diags.len(), path);
    for diag in &diags {
        let icon = match diag.severity.as_str() {
            "error" => "[ERROR]",
            "warning" => "[WARN]",
            _ => "[INFO]",
        };
        output.push_str(&format!(
            "  {} {}:{}:{} [{}] {}\n",
            icon,
            diag.file,
            diag.line.unwrap_or(0),
            diag.column.unwrap_or(0),
            diag.source,
            diag.message
        ));
    }
    output
}
