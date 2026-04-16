//! AI-powered coding agent for 32K-context models.
//!
//! Token budget (~32K tokens ≈ ~100K chars, 1 token ≈ 3.2 chars):
//!   System + tools:  ~300 chars   (~100 tokens)
//!   Last exchange:   ~3000 chars  (~900 tokens)
//!   Tool output:     ~3000 chars  (~900 tokens)
//!   Model response:  ~8000 chars  (~2500 tokens)
//!   Overhead/buffer: ~2000 chars
//!   Total per turn:  ~16K chars   (~5K tokens) — leaves headroom for 2-3 turns
//!
//! Key design: never repeat what the model already said. Each iteration
//! only sends fresh information (tool output), not the model's own reply.

use crate::client::{ChatRequest, ChatResponse, TabbyClient};
use crate::context::ConversationHistory;
use crate::tools::ToolExecutor;
use std::sync::{Arc, Mutex};

const MAX_TOOL_ITERATIONS: usize = 6;
const CONTEXT_CHAR_BUDGET: usize = 24000; // ~7.5K tokens, leaves 24.5K for model output
const MAX_TOOL_OUTPUT_CHARS: usize = 3000;
const MAX_HISTORY_CHARS: usize = 6000; // ~2K tokens
const MAX_MODEL_REPLY_CHARS: usize = 4000; // ~1.2K tokens max to echo back

#[derive(Clone)]
pub struct Agent {
    inner: Arc<AgentInner>,
}

struct AgentInner {
    client: TabbyClient,
    history: Mutex<ConversationHistory>,
    executor: ToolExecutor,
}

impl Agent {
    pub fn new(tabby_url: &str, model: &str, project_root: &str) -> Self {
        Self {
            inner: Arc::new(AgentInner {
                client: TabbyClient::new(tabby_url, model),
                history: Mutex::new(ConversationHistory::new(8)),
                executor: ToolExecutor::new(project_root),
            }),
        }
    }

    pub fn with_allowed_commands(self, commands: Vec<String>) -> Self {
        let executor = self.inner.executor.clone().with_allowed_commands(commands);
        Self {
            inner: Arc::new(AgentInner {
                client: self.inner.client.clone(),
                history: Mutex::new(ConversationHistory::new(8)),
                executor,
            }),
        }
    }

    pub async fn chat(&self, message: &str) -> Result<ChatResponse, String> {
        let system = Prompts::system();
        let history = {
            let hist = self.inner.history.lock().unwrap();
            let s = hist.to_string();
            if s.len() > MAX_HISTORY_CHARS {
                crate::context::truncate_str(&s, MAX_HISTORY_CHARS)
            } else {
                s
            }
        };

        let mut prompt = system.to_string();
        if !history.is_empty() {
            prompt.push_str(&format!("\n\n{}", history));
        }
        prompt.push_str(&format!("\n\nUser: {}", message));

        let response = self
            .inner
            .client
            .chat(ChatRequest {
                message: message.to_string(),
                context: Some(prompt),
                system_prompt: None,
                max_tokens: Some(2048),
                temperature: Some(0.7),
            })
            .await?;

        {
            let mut hist = self.inner.history.lock().unwrap();
            hist.add(crate::context::MessageRole::User, message.to_string());
            hist.add(crate::context::MessageRole::Assistant, response.reply.clone());
        }

        Ok(response)
    }

    pub async fn chat_with_tools(&self, message: &str) -> Result<ChatResponse, String> {
        {
            let mut hist = self.inner.history.lock().unwrap();
            hist.add(crate::context::MessageRole::User, message.to_string());
        }

        let system = Prompts::system();
        let allowed = self.inner.executor.get_allowed_commands();
        let mut current_prompt = format!(
            "{}\n\nAvailable commands: {}\n\nUser: {}",
            system,
            allowed.join(", "),
            message,
        );

        for iteration in 0..MAX_TOOL_ITERATIONS {
            // Trim prompt to budget
            if current_prompt.len() > CONTEXT_CHAR_BUDGET {
                current_prompt = crate::context::truncate_str(&current_prompt, CONTEXT_CHAR_BUDGET);
            }

            let response = self
                .inner
                .client
                .chat(ChatRequest {
                    message: current_prompt.clone(),
                    context: None,
                    system_prompt: None,
                    max_tokens: Some(2048),
                    temperature: Some(if iteration == 0 { 0.3 } else { 0.2 }),
                })
                .await?;

            let tool_commands = extract_shell_commands(&response.reply);

            if tool_commands.is_empty() {
                {
                    let mut hist = self.inner.history.lock().unwrap();
                    hist.add(crate::context::MessageRole::User, message.to_string());
                    hist.add(crate::context::MessageRole::Assistant, response.reply.clone());
                }
                return Ok(response);
            }

            let mut tool_sections = Vec::new();
            for cmd in &tool_commands {
                let result = self.inner.executor.execute_shell(cmd).await;
                let output = if result.timed_out {
                    format!("[TIMEOUT after {}s]", self.inner.executor.timeout_secs())
                } else if !result.success {
                    truncate_first_lines(&result.error.unwrap_or_default(), 15)
                } else if result.output.is_empty() {
                    "(no output)".to_string()
                } else {
                    truncate_first_lines(&result.output, 40)
                };
                tool_sections.push(format!("$ {}\n{}", cmd, output));
            }

            let tool_output = tool_sections.join("\n\n");

            if iteration + 1 >= MAX_TOOL_ITERATIONS {
                current_prompt = format!(
                    "Tool output:\n{}\n\nProvide your final answer now.",
                    tool_output
                );
            } else {
                current_prompt = format!(
                    "Tool output:\n{}\n\nContinue ($ command) or give final answer.",
                    tool_output
                );
            }
        }

        Ok(ChatResponse {
            reply: "I reached the tool execution limit. Please ask again with a simpler task.".to_string(),
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
    fn system() -> &'static str {
        "You are a coding assistant with shell access. Run commands with $ prefix or in ```bash blocks. Rules: one command per line, no chaining/pipes/substitution. Be brief."
    }
}

/// Truncate output to first N non-empty lines, keeping totals short.
fn truncate_first_lines(output: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = output.lines().take(max_lines).collect();
    let mut result = lines.join("\n");
    let total_lines = output.lines().count();
    if total_lines > max_lines {
        result.push_str(&format!("\n... ({} more lines)", total_lines - max_lines));
    }
    if result.len() > MAX_TOOL_OUTPUT_CHARS {
        result = crate::context::truncate_str(&result, MAX_TOOL_OUTPUT_CHARS);
    }
    result
}

fn extract_shell_commands(response: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut in_code_block = false;
    let mut code_block_lang = String::new();

    for line in response.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            if in_code_block {
                in_code_block = false;
                code_block_lang.clear();
            } else {
                in_code_block = true;
                code_block_lang = trimmed.trim_start_matches('`').trim().to_lowercase();
            }
            continue;
        }

        if in_code_block && (code_block_lang.is_empty() || code_block_lang == "sh" || code_block_lang == "bash") {
            let cmd = trimmed.trim();
            if !cmd.is_empty() && !cmd.starts_with('#') {
                commands.push(cmd.to_string());
            }
            continue;
        }

        if !in_code_block && trimmed.starts_with("$ ") {
            let cmd = trimmed[2..].trim().to_string();
            if !cmd.is_empty() {
                commands.push(cmd);
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
    fn test_extract_dollar_commands() {
        assert_eq!(extract_shell_commands("$ cargo check\nDone"), vec!["cargo check"]);
    }

    #[test]
    fn test_extract_code_block_commands() {
        assert_eq!(
            extract_shell_commands("```bash\ncargo test\nls src/\n```"),
            vec!["cargo test", "ls src/"]
        );
    }

    #[test]
    fn test_extract_skips_comments() {
        assert_eq!(
            extract_shell_commands("```sh\n# comment\ncargo build\n```"),
            vec!["cargo build"]
        );
    }

    #[test]
    fn test_extract_skips_other_code_blocks() {
        assert!(extract_shell_commands("```rust\nfn main() {}\n```").is_empty());
    }

    #[test]
    fn test_truncate_first_lines() {
        let long = (1..=100).map(|i| format!("line {}", i)).collect::<Vec<_>>().join("\n");
        let truncated = truncate_first_lines(&long, 10);
        assert!(truncated.contains("line 10"));
        assert!(truncated.contains("90 more lines"));
    }
}
