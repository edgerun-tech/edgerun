//! Tool system for agent actions.
//!
//! Dynamic tool set based on context to prevent the agent from "shooting itself in the foot".
//! All file operations use the in-memory VFS for maximum performance.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process;
use crate::vfs::SharedVFS;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ToolCatalog {
    safe_tools: HashMap<String, bool>,
    allowed_dirs: Vec<String>,
}

impl ToolCatalog {
    pub fn new() -> Self {
        let mut safe_tools = HashMap::new();

        safe_tools.insert("read_file".to_string(), true);
        safe_tools.insert("list_directory".to_string(), true);
        safe_tools.insert("search_files".to_string(), true);
        safe_tools.insert("run_cargo_check".to_string(), true);
        safe_tools.insert("run_cargo_test".to_string(), true);
        safe_tools.insert("run_cargo_build".to_string(), true);
        safe_tools.insert("grep".to_string(), true);

        safe_tools.insert("write_file".to_string(), false);
        safe_tools.insert("delete_file".to_string(), false);
        safe_tools.insert("run_shell".to_string(), false);
        safe_tools.insert("run_git".to_string(), true);

        Self {
            safe_tools,
            allowed_dirs: vec![],
        }
    }

    pub fn with_allowed_dir(mut self, dir: &str) -> Self {
        self.allowed_dirs.push(dir.to_string());
        self
    }

    pub fn enable_tool(&mut self, name: &str) {
        self.safe_tools.insert(name.to_string(), true);
    }

    pub fn disable_tool(&mut self, name: &str) {
        self.safe_tools.insert(name.to_string(), false);
    }

    pub fn is_enabled(&self, name: &str) -> bool {
        self.safe_tools.get(name).copied().unwrap_or(false)
    }

    pub fn get_enabled(&self) -> Vec<ToolDefinition> {
        self.safe_tools
            .iter()
            .map(|(name, enabled)| ToolDefinition {
                name: name.clone(),
                description: Self::tool_description(name),
                enabled: *enabled,
            })
            .collect()
    }

    fn tool_description(name: &str) -> String {
        match name {
            "read_file" => "rf(path)".to_string(),
            "list_directory" => "ls(path)".to_string(),
            "search_files" => "sf(pattern,path)".to_string(),
            "run_cargo_check" => "cc".to_string(),
            "run_cargo_test" => "ct".to_string(),
            "run_cargo_build" => "cb".to_string(),
            "grep" => "gr(pattern,path)".to_string(),
            "write_file" => "wf(path,content)".to_string(),
            "delete_file" => "df(path)".to_string(),
            "run_shell" => "sh(cmd)".to_string(),
            "run_git" => "git(subcommand)".to_string(),
            _ => "unknown".to_string(),
        }
    }
}

impl Default for ToolCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct ToolExecutor {
    catalog: ToolCatalog,
    project_root: String,
    vfs: Option<SharedVFS>,
}

impl ToolExecutor {
    pub fn new(project_root: &str) -> Self {
        Self {
            catalog: ToolCatalog::new().with_allowed_dir(project_root),
            project_root: project_root.to_string(),
            vfs: None,
        }
    }

    pub fn with_vfs(mut self, vfs: SharedVFS) -> Self {
        self.vfs = Some(vfs);
        self
    }

    pub fn with_catalog(mut self, catalog: ToolCatalog) -> Self {
        self.catalog = catalog;
        self
    }

    pub fn execute(&self, tool_name: &str, args: HashMap<String, String>) -> ToolResult {
        if !self.catalog.is_enabled(tool_name) {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Tool '{}' is not enabled", tool_name)),
            };
        }

        match tool_name {
            "read_file" => self.read_file(&args),
            "list_directory" => self.list_directory(&args),
            "search_files" => self.search_files(&args),
            "run_cargo_check" => self.run_cargo_check(&args),
            "run_cargo_test" => self.run_cargo_test(&args),
            "run_cargo_build" => self.run_cargo_build(&args),
            "grep" => self.grep(&args),
            "run_git" => self.run_git(&args),
            _ => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown tool: {}", tool_name)),
            },
        }
    }

    pub async fn execute_async(&self, tool_name: &str, args: HashMap<String, String>) -> ToolResult {
        if !self.catalog.is_enabled(tool_name) {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Tool '{}' is not enabled", tool_name)),
            };
        }

        match tool_name {
            "read_file" => self.read_file(&args),
            "list_directory" => self.list_directory(&args),
            "search_files" => self.search_files(&args),
            "run_cargo_check" => self.run_cargo_check(&args),
            "run_cargo_test" => self.run_cargo_test(&args),
            "run_cargo_build" => self.run_cargo_build(&args),
            "grep" => self.grep(&args),
            "run_git" => self.run_git(&args),
            _ => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Unknown tool: {}", tool_name)),
            },
        }
    }

    pub fn get_enabled_tools(&self) -> Vec<ToolDefinition> {
        self.catalog.get_enabled()
    }

    pub fn is_tool_available(&self, name: &str) -> bool {
        self.catalog.is_enabled(name)
    }

    fn read_file(&self, args: &HashMap<String, String>) -> ToolResult {
        let path = match args.get("path") {
            Some(p) => p,
            None => {
                return ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Missing 'path' argument".to_string()),
                }
            }
        };

        // Use VFS if available, otherwise fall back to disk
        if let Some(vfs) = &self.vfs {
            let vfs_read = vfs.read();
            let rel_path = self.make_relative_path(path);
            match vfs_read.read_str(&rel_path) {
                Some(content) => ToolResult {
                    success: true,
                    output: content.to_string(),
                    error: None,
                },
                None => ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("File not found in VFS: {}", rel_path.display())),
                },
            }
        } else {
            let full_path = self.resolve_path(path);
            match std::fs::read_to_string(&full_path) {
                Ok(content) => ToolResult {
                    success: true,
                    output: content,
                    error: None,
                },
                Err(e) => ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some(format!("Failed to read {}: {}", full_path, e)),
                },
            }
        }
    }

    fn list_directory(&self, args: &HashMap<String, String>) -> ToolResult {
        let path = args.get("path").map(|p| p.as_str()).unwrap_or(".");
        let full_path = self.resolve_path(path);

        match std::fs::read_dir(&full_path) {
            Ok(entries) => {
                let mut files = Vec::new();
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let is_dir = entry.path().is_dir();
                    files.push(if is_dir { format!("{}/", name) } else { name });
                }
                ToolResult {
                    success: true,
                    output: files.join("\n"),
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to list {}: {}", full_path, e)),
            },
        }
    }

    fn search_files(&self, args: &HashMap<String, String>) -> ToolResult {
        let pattern = args.get("pattern").map(|p| p.as_str()).unwrap_or("*");
        let path = args.get("path").map(|p| p.as_str()).unwrap_or(".");
        let full_path = self.resolve_path(path);

        let pattern = glob::Pattern::new(pattern).map_err(|e| format!("Invalid pattern: {}", e));

        match pattern {
            Ok(pattern) => {
                let mut results = Vec::new();
                if let Ok(entries) = std::fs::read_dir(&full_path) {
                    for entry in entries.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if pattern.matches(&name) {
                            let is_dir = entry.path().is_dir();
                            results.push(if is_dir { format!("{}/", name) } else { name });
                        }
                    }
                }
                ToolResult {
                    success: true,
                    output: results.join("\n"),
                    error: None,
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(e),
            },
        }
    }

    fn run_cargo_check(&self, _args: &HashMap<String, String>) -> ToolResult {
        self.run_command("cargo", &["check"], ".")
    }

    fn run_cargo_test(&self, _args: &HashMap<String, String>) -> ToolResult {
        self.run_command("cargo", &["test", "--", "--nocapture"], ".")
    }

    fn run_cargo_build(&self, _args: &HashMap<String, String>) -> ToolResult {
        self.run_command("cargo", &["build"], ".")
    }

    fn grep(&self, args: &HashMap<String, String>) -> ToolResult {
        let pattern = args.get("pattern").map(|p| p.as_str()).unwrap_or("");
        
        // Use VFS grep if available (much faster)
        if let Some(vfs) = &self.vfs {
            let vfs_read = vfs.read();
            let matches = vfs_read.grep(pattern);
            
            if matches.is_empty() {
                return ToolResult {
                    success: true,
                    output: String::new(),
                    error: None,
                };
            }
            
            let output = matches
                .iter()
                .map(|m| format!("{}:{}:{}", m.path.display(), m.line_number, m.line))
                .collect::<Vec<_>>()
                .join("\n");
            
            ToolResult {
                success: true,
                output,
                error: None,
            }
        } else {
            // Fallback to system grep
            let path = args.get("path").map(|p| p.as_str()).unwrap_or(".");
            self.run_command("grep", &["-r", "--line-number", pattern, path], ".")
        }
    }

    fn run_git(&self, args: &HashMap<String, String>) -> ToolResult {
        let subcommand = args
            .get("subcommand")
            .map(|s| s.as_str())
            .unwrap_or("status");
        let mut cmd_args = vec!["-C", &self.project_root];

        match subcommand {
            "status" => cmd_args.push("status"),
            "diff" => cmd_args.push("diff"),
            "log" => {
                cmd_args.push("log");
                cmd_args.push("--oneline");
                cmd_args.push("-10");
            }
            _ => {}
        }

        self.run_command("git", &cmd_args, ".")
    }



    fn run_command(&self, program: &str, args: &[&str], _cwd: &str) -> ToolResult {
        let output = process::Command::new(program)
            .args(args)
            .current_dir(&self.project_root)
            .output();

        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout).to_string();
                let stderr = String::from_utf8_lossy(&o.stderr).to_string();

                if o.status.success() {
                    ToolResult {
                        success: true,
                        output: stdout,
                        error: if stderr.is_empty() {
                            None
                        } else {
                            Some(stderr)
                        },
                    }
                } else {
                    ToolResult {
                        success: false,
                        output: stdout,
                        error: Some(stderr),
                    }
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to run {}: {}", program, e)),
            },
        }
    }

    fn resolve_path(&self, path: &str) -> String {
        if std::path::Path::new(path).is_absolute() {
            path.to_string()
        } else {
            std::path::Path::new(&self.project_root)
                .join(path)
                .to_string_lossy()
                .to_string()
        }
    }

    fn make_relative_path(&self, path: &str) -> std::path::PathBuf {
        let path = std::path::Path::new(path);
        if path.is_absolute() {
            path.strip_prefix(&self.project_root)
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|_| path.to_path_buf())
        } else {
            path.to_path_buf()
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub arguments: HashMap<String, String>,
}

impl ToolCall {
    pub fn from_spec(spec: &str) -> Option<Self> {
        let spec = spec.trim();
        // Find the opening parenthesis
        let open_paren = spec.find('(')?;
        let close_paren = spec.rfind(')')?;
        if open_paren >= close_paren {
            return None;
        }
        let name = spec[..open_paren].trim();
        let args_str = spec[open_paren + 1..close_paren].trim();

        if name.is_empty() {
            return None;
        }

        let mut arguments = HashMap::new();
        if !args_str.is_empty() {
            // Split by commas, but we assume no commas inside values for simplicity
            for arg in args_str.split(',') {
                let arg = arg.trim();
                if arg.is_empty() {
                    continue;
                }
                let eq_pos = arg.find('=')?;
                let key = arg[..eq_pos].trim();
                let mut value = arg[eq_pos + 1..].trim();
                // Remove surrounding quotes if present
                if value.starts_with('\"') && value.ends_with('\"') && value.len() >= 2 {
                    value = &value[1..value.len() - 1];
                } else if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
                    value = &value[1..value.len() - 1];
                }
                if key.is_empty() || value.is_empty() {
                    return None;
                }
                arguments.insert(key.to_string(), value.to_string());
            }
        }

        Some(Self {
            name: name.to_string(),
            arguments,
        })
    }
}

// Benchmark comment