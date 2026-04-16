//! Coding agent optimized for 32K context — shell tool, project-aware, token-efficient.

use crate::client::{ChatRequest, ChatResponse, TabbyClient};
use crate::context::ConversationHistory;
use crate::tools::ToolExecutor;
use std::sync::{Arc, Mutex};

const MAX_TOOL_ITERATIONS: usize = 6;
const MAX_HISTORY_TURNS: usize = 4;
const MAX_TOOL_OUTPUT_CHARS: usize = 1500;
const MAX_TURNS_TO_SHOW: usize = 2;

#[derive(Clone)]
pub struct Agent {
    inner: Arc<AgentInner>,
}

struct AgentInner {
    client: TabbyClient,
    history: Mutex<ConversationHistory>,
    executor: ToolExecutor,
    project_map: String,
    temperature: f32,
    max_tokens: u32,
}

impl Clone for AgentInner {
    fn clone(&self) -> Self {
        Self {
            client: self.client.clone(),
            history: Mutex::new(ConversationHistory::new(MAX_HISTORY_TURNS * 2)),
            executor: self.executor.clone(),
            project_map: self.project_map.clone(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }
}

impl Agent {
    pub fn new(tabby_url: &str, model: &str, project_root: &str) -> Self {
        let project_map = build_project_map(project_root);
        Self {
            inner: Arc::new(AgentInner {
                client: TabbyClient::new(tabby_url, model),
                history: Mutex::new(ConversationHistory::new(MAX_HISTORY_TURNS * 2)),
                executor: ToolExecutor::new(project_root),
                project_map,
                temperature: 0.7,
                max_tokens: 2048,
            }),
        }
    }

    pub fn with_allowed_commands(self, commands: Vec<String>) -> Self {
        let executor = self.inner.executor.clone().with_allowed_commands(commands);
        Self {
            inner: Arc::new(AgentInner {
                client: self.inner.client.clone(),
                history: Mutex::new(ConversationHistory::new(MAX_HISTORY_TURNS * 2)),
                executor,
                project_map: self.inner.project_map.clone(),
                temperature: self.inner.temperature,
                max_tokens: self.inner.max_tokens,
            }),
        }
    }
    
    pub fn with_temperature(self, temperature: f32) -> Self {
        Self {
            inner: Arc::new(AgentInner {
                temperature,
                ..(*self.inner).clone()
            }),
        }
    }
    
    pub fn with_max_tokens(self, max_tokens: u32) -> Self {
        Self {
            inner: Arc::new(AgentInner {
                max_tokens,
                ..(*self.inner).clone()
            }),
        }
    }

    pub async fn chat_with_tools(&self, message: &str) -> Result<ChatResponse, String> {
        {
            let mut hist = self.inner.history.lock().unwrap();
            hist.add(crate::context::MessageRole::User, message.to_string());
        }

        let system = Prompts::system_with_context(&self.inner.project_map, self.inner.executor.get_allowed_commands());
        let mut continuation = message.to_string();

        for iteration in 0..MAX_TOOL_ITERATIONS {
            let response = self
                .inner
                .client
                .chat(ChatRequest {
                    message: continuation.clone(),
                    context: if iteration == 0 { Some(system.clone()) } else { None },
                    system_prompt: None,
                    max_tokens: Some(self.inner.max_tokens),
                    temperature: Some(self.inner.temperature),
                })
                .await?;

            let tool_commands = extract_shell_commands(&response.reply);

            if tool_commands.is_empty() {
                {
                    let mut hist = self.inner.history.lock().unwrap();
                    hist.add(crate::context::MessageRole::Assistant, response.reply.clone());
                }
                return Ok(response);
            }

            let mut results = Vec::new();
            for cmd in &tool_commands {
                let raw = self.inner.executor.execute(cmd).await;
                let output = self.inner.executor.format_output(cmd, &raw);
                let truncated = if output.len() > MAX_TOOL_OUTPUT_CHARS {
                    crate::context::truncate_str(&output, MAX_TOOL_OUTPUT_CHARS)
                } else {
                    output
                };
                results.push(format!("$ {}\n{}", cmd, truncated));
            }

            let tool_output = results.join("\n\n");

            if iteration + 1 >= MAX_TOOL_ITERATIONS {
                continuation = format!(
                    "Results:\n{}\n\nFinal answer only — no more commands.",
                    tool_output
                );
            } else {
                continuation = format!(
                    "Results:\n{}\n\nContinue ($ cmd) or give final answer.",
                    tool_output
                );
            }
        }

        Ok(ChatResponse {
            reply: "Command limit reached. Try a more specific question.".to_string(),
            usage: None,
            error: Some("Max tool iterations exceeded".to_string()),
        })
    }

    pub fn clear_history(&self) {
        let mut hist = self.inner.history.lock().unwrap();
        hist.clear();
    }

    pub fn get_allowed_commands(&self) -> Vec<String> {
        self.inner.executor.get_allowed_commands()
    }
}

struct Prompts;

impl Prompts {
    fn system_with_context(project_map: &str, commands: Vec<String>) -> String {
        let cmd_list = commands.join(", ");
        format!(
            "You are a coding assistant. Run shell commands with $ prefix.\n\
Commands: {}\n\
Project:\n{}",
            cmd_list, project_map
        )
    }
}

fn build_project_map(project_root: &str) -> String {
    let mut parts = Vec::new();

    // Language
    if std::path::Path::new(&format!("{}/Cargo.toml", project_root)).exists() {
        let cargo_info = std::fs::read_to_string(&format!("{}/Cargo.toml", project_root))
            .ok()
            .and_then(|c| {
                let name = c.lines()
                    .find(|l| l.trim().starts_with("name = "))
                    .map(|l| l.split('"').nth(1).unwrap_or("?").to_string())
                    .unwrap_or_else(|| "unknown".to_string());
                let deps = c.lines().filter(|l| l.contains(" = {")).count();
                Some((name, deps))
            });

        if let Some((name, deps)) = cargo_info {
            parts.push(format!("Language: Rust | Project: {} | Dependencies: {}", name, deps));
        }
    }

    // File count
    let file_count = count_source_files(project_root);
    parts.push(format!("Files: {} source files", file_count));

    // Git branch
    let git_branch = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().to_string().into())
        .filter(|s| !s.is_empty() && s != "HEAD");

    if let Some(branch) = git_branch {
        parts.push(format!("Git branch: {}", branch));
    }

    if parts.is_empty() {
        "Project: (unknown)".to_string()
    } else {
        parts.join(" | ")
    }
}

fn count_source_files(project_root: &str) -> usize {
    let exts = ["rs", "toml", "md", "txt"];
    let mut count = 0;

    fn walk(dir: &std::path::Path, exts: &[&str], count: &mut usize, depth: usize) {
        if depth > 4 { return; }

        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(_) => return,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.starts_with('.')
                || name_str == "target"
                || name_str == "node_modules"
                || name_str == "dist"
                || name_str == "build"
            {
                continue;
            }

            if path.is_dir() {
                walk(&path, exts, count, depth + 1);
            } else if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                if exts.contains(&ext) {
                    *count += 1;
                }
            }
        }
    }

    walk(std::path::Path::new(project_root), &exts, &mut count, 0);
    count
}


fn extract_cmd_from_line(line: &str) -> Option<String> {
    // Match $ followed by a command, with optional leading text
    // e.g. "Try: $ ls src" or "$ cargo test" or "  $ echo hello"
    if let Some(dollar_pos) = line.find("$ ") {
        let after_dollar = &line[dollar_pos + 2..];
        let cmd = after_dollar.trim();
        if !cmd.is_empty() && !cmd.starts_with('#') {
            return Some(cmd.to_string());
        }
    }
    None
}

fn extract_shell_commands(response: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut in_code_block = false;
    let mut lang = String::new();

    for line in response.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_code_block {
                in_code_block = false;
                lang.clear();
            } else {
                in_code_block = true;
                lang = trimmed.trim_start_matches('`').trim().to_lowercase();
            }
            continue;
        }

        if in_code_block && (lang.is_empty() || lang == "sh" || lang == "bash" || lang == "text") {
            let cmd = trimmed.trim();
            if !cmd.is_empty() && !cmd.starts_with('#') {
                commands.push(cmd.to_string());
            }
            continue;
        }

        if !in_code_block {
            let cmd = extract_cmd_from_line(trimmed);
            if let Some(c) = cmd {
                commands.push(c);
            }
        }
    }

    commands
}

impl Default for Agent {
    fn default() -> Self {
        Self::new("http://10.10.10.1:5001", "devstral-small-2:24b", ".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dollar() {
        assert_eq!(extract_shell_commands("$ grep 'fn' src/lib.rs\nDone"), vec!["grep 'fn' src/lib.rs"]);
    }

    #[test]
    fn test_extract_code_block() {
        assert_eq!(
            extract_shell_commands("```bash\ncargo test\nls src/\n```"),
            vec!["cargo test", "ls src/"]
        );
    }

    #[test]
    fn test_extract_mixed() {
        let r = "Try: $ ls src\nOr:\n```bash\ncargo build\n```\nThen $ cargo test";
        assert_eq!(extract_shell_commands(r), vec!["ls src", "cargo build", "cargo test"]);
    }

    #[test]
    fn test_extract_skips_comments() {
        assert_eq!(extract_shell_commands("```sh\n# comment\ncargo build\n```"), vec!["cargo build"]);
    }

    #[test]
    fn test_extract_skips_rust() {
        assert!(extract_shell_commands("```rust\nfn main() {}\n```").is_empty());
    }
}
