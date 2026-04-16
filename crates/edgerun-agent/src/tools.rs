//! Shell-based tool system with dynamic command allowlist.
//!
//! The agent executes commands through the shell. Available commands
//! are configured at startup from the CLI arguments.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub example: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct ToolExecutor {
    allowed_commands: Vec<String>,
    project_root: String,
}

impl ToolExecutor {
    pub fn new(project_root: &str) -> Self {
        Self {
            allowed_commands: Self::default_commands(),
            project_root: project_root.to_string(),
        }
    }

    pub fn with_allowed_commands(mut self, commands: Vec<String>) -> Self {
        self.allowed_commands = commands;
        self
    }

    fn default_commands() -> Vec<String> {
        [
            "cat",
            "head",
            "tail",
            "ls",
            "find",
            "grep",
            "rg",
            "wc",
            "cargo",
            "rustc",
            "rustfmt",
            "clippy-driver",
            "git",
            "diff",
            "echo",
            "mkdir",
            "cp",
            "mv",
            "curl",
            "jq",
            "sort",
            "uniq",
            "awk",
            "sed",
            "tree",
            "which",
            "env",
            "pwd",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    pub fn execute(&self, command: &str, args: &[&str]) -> ToolResult {
        let Some(cmd) = command.split_whitespace().next() else {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("Empty command".to_string()),
            };
        };

        let base_cmd = if cmd.contains('/') {
            cmd.rsplit('/').next().unwrap_or(cmd)
        } else {
            cmd
        };

        if !self.allowed_commands.contains(&base_cmd.to_string()) {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Command '{}' not allowed. Allowed: {}",
                    base_cmd,
                    self.allowed_commands.join(", ")
                )),
            };
        }

        let output = process::Command::new(command)
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
                        error: Some(format!(
                            "Exit {}: {}",
                            o.status.code().unwrap_or(-1),
                            stderr
                        )),
                    }
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to execute '{}': {}", command, e)),
            },
        }
    }

    pub fn execute_shell(&self, shell_command: &str) -> ToolResult {
        let Some(cmd) = shell_command.split_whitespace().next() else {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some("Empty command".to_string()),
            };
        };

        let base_cmd = if cmd.contains('/') {
            cmd.rsplit('/').next().unwrap_or(cmd)
        } else {
            cmd
        };

        if !self.allowed_commands.contains(&base_cmd.to_string()) {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Command '{}' not allowed. Allowed: {}",
                    base_cmd,
                    self.allowed_commands.join(", ")
                )),
            };
        }

        let output = process::Command::new("sh")
            .arg("-c")
            .arg(shell_command)
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
                        error: Some(format!(
                            "Exit {}: {}",
                            o.status.code().unwrap_or(-1),
                            stderr
                        )),
                    }
                }
            }
            Err(e) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Failed to execute shell command: {}", e)),
            },
        }
    }

    pub fn get_allowed_commands(&self) -> &[String] {
        &self.allowed_commands
    }

    pub fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.allowed_commands
            .iter()
            .map(|cmd| ToolDefinition {
                name: cmd.clone(),
                description: format!("Execute '{}' command", cmd),
                example: format!("{} ...", cmd),
            })
            .collect()
    }
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new(".")
    }
}
