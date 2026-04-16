//! Shell tool executor — strict allowlist, no shell metacharacters, bounded output.

use std::collections::HashSet;
use std::process;
use std::time::Duration;

use serde::{Deserialize, Serialize};

const DEFAULT_COMMAND_TIMEOUT_SECS: u64 = 30;
const MAX_OUTPUT_CHARS: usize = 3000;
const MAX_STDERR_CHARS: usize = 500;

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
    pub timed_out: bool,
}

#[derive(Clone)]
pub struct ToolExecutor {
    allowed_commands: HashSet<String>,
    project_root: String,
    timeout: Duration,
}

const SHELL_META_CHARS: &[char] = &[';', '&', '|', '$', '`', '(', ')', '{', '}', '<', '>', '!', '\n'];

impl ToolExecutor {
    pub fn new(project_root: &str) -> Self {
        Self {
            allowed_commands: Self::default_commands(),
            project_root: project_root.to_string(),
            timeout: Duration::from_secs(DEFAULT_COMMAND_TIMEOUT_SECS),
        }
    }

    pub fn with_allowed_commands(mut self, commands: Vec<String>) -> Self {
        self.allowed_commands = commands.into_iter().collect();
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn timeout_secs(&self) -> u64 {
        self.timeout.as_secs()
    }

    fn default_commands() -> HashSet<String> {
        [
            "cat", "head", "tail", "ls", "find", "grep", "rg", "wc",
            "cargo", "rustc", "rustfmt", "clippy-driver",
            "git", "diff", "echo", "mkdir", "cp", "mv",
            "curl", "jq", "sort", "uniq", "awk", "sed",
            "tree", "which", "env", "pwd", "test",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn extract_base_cmd(cmd: &str) -> &str {
        let first = cmd.split_whitespace().next().unwrap_or("");
        if first.contains('/') {
            first.rsplit('/').next().unwrap_or(first)
        } else {
            first
        }
    }

    fn validate_command(&self, command: &str) -> Result<(), String> {
        if command.trim().is_empty() {
            return Err("Empty command".to_string());
        }

        if command.contains(SHELL_META_CHARS) {
            return Err("Shell metacharacters forbidden. Use one command per line.".to_string());
        }

        let base_cmd = Self::extract_base_cmd(command);
        if !self.allowed_commands.contains(base_cmd) {
            return Err(format!("Command '{}' not allowed.", base_cmd));
        }

        Ok(())
    }

    pub async fn execute_shell(&self, shell_command: &str) -> ToolResult {
        if let Err(e) = self.validate_command(shell_command) {
            return ToolResult {
                success: false,
                output: String::new(),
                error: Some(e),
                timed_out: false,
            };
        }

        let cmd = shell_command.to_string();
        let project_root = self.project_root.clone();
        let timeout = self.timeout;

        let handle = edgerun_rt::spawn_blocking(move || {
            run_command_sync(&cmd, &project_root)
        });

        let timeout_result = edgerun_rt::timeout(timeout, handle).await;

        match timeout_result {
            Ok(result) => match result {
                Ok(tool_result) => tool_result,
                Err(_) => ToolResult {
                    success: false,
                    output: String::new(),
                    error: Some("Task panicked".to_string()),
                    timed_out: false,
                },
            },
            Err(_) => ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!("Timed out ({}s)", timeout.as_secs())),
                timed_out: true,
            },
        }
    }

    pub fn get_allowed_commands(&self) -> Vec<String> {
        let mut cmds: Vec<String> = self.allowed_commands.iter().cloned().collect();
        cmds.sort();
        cmds
    }

    pub fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.get_allowed_commands()
            .iter()
            .map(|cmd| ToolDefinition {
                name: cmd.clone(),
                description: format!("Run {}", cmd),
                example: format!("$ {} ...", cmd),
            })
            .collect()
    }
}

fn run_command_sync(command: &str, project_root: &str) -> ToolResult {
    let output = process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(project_root)
        .stdout(process::Stdio::piped())
        .stderr(process::Stdio::piped())
        .stdin(process::Stdio::null())
        .output();

    match output {
        Ok(o) => {
            let stdout = truncate_output(&String::from_utf8_lossy(&o.stdout), MAX_OUTPUT_CHARS);
            let success = o.status.success();

            if success {
                let stderr = String::from_utf8_lossy(&o.stderr);
                ToolResult {
                    success: true,
                    output: stdout,
                    error: if stderr.is_empty() { None } else { Some(truncate_output(&stderr, MAX_STDERR_CHARS)) },
                    timed_out: false,
                }
            } else {
                let stderr = truncate_output(&String::from_utf8_lossy(&o.stderr), MAX_STDERR_CHARS);
                ToolResult {
                    success: false,
                    output: stdout,
                    error: Some(format!("exit {}: {}", o.status.code().unwrap_or(-1), stderr)),
                    timed_out: false,
                }
            }
        }
        Err(e) => ToolResult {
            success: false,
            output: String::new(),
            error: Some(format!("Failed: {}", e)),
            timed_out: false,
        },
    }
}

fn truncate_output(s: &str, max_chars: usize) -> String {
    if s.len() <= max_chars {
        return s.to_string();
    }
    // Take first N lines that fit, then report total
    let mut result = String::new();
    let mut lines_taken = 0;
    for line in s.lines() {
        if result.len() + line.len() + 1 > max_chars - 30 {
            break;
        }
        if !result.is_empty() {
            result.push('\n');
        }
        result.push_str(line);
        lines_taken += 1;
    }
    let total_lines = s.lines().count();
    if lines_taken < total_lines {
        result.push_str(&format!("\n... ({} more lines)", total_lines - lines_taken));
    }
    result
}

impl Default for ToolExecutor {
    fn default() -> Self {
        Self::new(".")
    }
}
