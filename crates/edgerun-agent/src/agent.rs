//! AI-powered coding agent that connects to a TabbyAPI server.
//!
//! Design notes:
//! - Conversation state uses std::sync::Mutex (short hold times, never across await)
//! - Shell commands run on spawn_blocking with a timeout
//! - Prompt is rebuilt from scratch each iteration to avoid unbounded growth
//! - Shell metacharacters are rejected before execution

use crate::client::{ChatRequest, ChatResponse, TabbyClient};
use crate::context::ConversationHistory;
use crate::prompts::Prompts;
use crate::tools::ToolExecutor;
use std::sync::{Arc, Mutex};

const MAX_TOOL_ITERATIONS: usize = 10;
const MAX_PROMPT_CHARS: usize = 60000;

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
                history: Mutex::new(ConversationHistory::new(50)),
                executor: ToolExecutor::new(project_root),
            }),
        }
    }

    pub fn with_allowed_commands(self, commands: Vec<String>) -> Self {
        let executor = self.inner.executor.clone().with_allowed_commands(commands);
        Self {
            inner: Arc::new(AgentInner {
                client: self.inner.client.clone(),
                history: Mutex::new(ConversationHistory::new(50)),
                executor,
            }),
        }
    }

    fn build_prompt(&self, user_message: &str) -> String {
        let history_str = {
            let hist = self.inner.history.lock().unwrap();
            hist.to_string()
        };
        let system = Prompts::system();
        let allowed = self.inner.executor.get_allowed_commands();
        let tools_section = format!(
            "You have shell access. Available commands: {}\n\
             To run a command, put it on a line starting with $ like:\n\
             $ cargo check\n\
             $ grep -r \"pattern\" src/\n\n\
             You can also put commands inside ```bash code blocks.\n\n\
             Rules:\n\
             - One command per $ line (no chaining with && or ;)\n\
             - No pipes, redirections, or command substitution\n\
             - After seeing output, provide your final answer\n",
            allowed.join(", ")
        );

        let mut prompt = format!("{}\n\n{}", system, tools_section);

        if !history_str.is_empty() {
            let history_budget = MAX_PROMPT_CHARS / 3;
            let truncated = if history_str.len() > history_budget {
                crate::context::truncate_str(&history_str, history_budget)
            } else {
                history_str
            };
            prompt.push_str(&format!("\n\nConversation so far:\n{}", truncated));
        }

        prompt.push_str(&format!("\n\nUser: {}", user_message));
        prompt
    }

    pub async fn chat(&self, message: &str) -> Result<ChatResponse, String> {
        let prompt = self.build_prompt(message);

        let response = self
            .inner
            .client
            .chat(ChatRequest {
                message: message.to_string(),
                context: Some(prompt),
                system_prompt: None,
                max_tokens: Some(4096),
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

        let mut current_message = message.to_string();

        for iteration in 0..MAX_TOOL_ITERATIONS {
            let prompt = self.build_prompt(&current_message);

            let response = self
                .inner
                .client
                .chat(ChatRequest {
                    message: current_message.clone(),
                    context: Some(prompt),
                    system_prompt: None,
                    max_tokens: Some(4096),
                    temperature: Some(0.3),
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
                let result = self.inner.executor.execute_shell(cmd).await;
                let output = if result.timed_out {
                    format!("$ {}\n[TIMEOUT: command exceeded time limit]", cmd)
                } else if !result.success {
                    format!("$ {}\n[EXIT ERROR] {}", cmd, result.error.unwrap_or_default())
                } else {
                    let output = result.output;
                    let error_note = result.error.map(|e| format!("\n[stderr]: {}", e)).unwrap_or_default();
                    format!("$ {}\n{}{}", cmd, output, error_note)
                };
                results.push(output);
            }

            let tool_output = results.join("\n\n");

            let reply_preview = crate::context::truncate_str(&response.reply, 4000);
            let tool_preview = if tool_output.len() > 16000 {
                crate::context::truncate_str(&tool_output, 16000)
            } else {
                tool_output
            };

            current_message = if iteration + 1 >= MAX_TOOL_ITERATIONS {
                format!(
                    "Previous response:\n{}\n\nTool output:\n{}\n\n\
                     This is the last iteration. Provide your final answer now without running more commands.",
                    reply_preview, tool_preview
                )
            } else {
                format!(
                    "Previous response:\n{}\n\nTool output:\n{}\n\n\
                     Continue your analysis. Provide your final answer or run more commands with $ prefix.",
                    reply_preview, tool_preview
                )
            };
        }

        Ok(ChatResponse {
            reply: "Tool execution limit reached. Please try a simpler request.".to_string(),
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
        let response = "Let me check:\n$ cargo check\n\nDone.";
        let cmds = extract_shell_commands(response);
        assert_eq!(cmds, vec!["cargo check"]);
    }

    #[test]
    fn test_extract_code_block_commands() {
        let response = "Run this:\n```bash\ncargo test\nls src/\n```\nDone.";
        let cmds = extract_shell_commands(response);
        assert_eq!(cmds, vec!["cargo test", "ls src/"]);
    }

    #[test]
    fn test_extract_skips_comments() {
        let response = "```sh\n# this is a comment\ncargo build\n```";
        let cmds = extract_shell_commands(response);
        assert_eq!(cmds, vec!["cargo build"]);
    }

    #[test]
    fn test_extract_skips_other_code_blocks() {
        let response = "```rust\nfn main() {}\n```";
        let cmds = extract_shell_commands(response);
        assert!(cmds.is_empty());
    }

    #[test]
    fn test_extract_mixed() {
        let response = "Check this:\n$ ls\n\nOr:\n```bash\ncargo check\n```\n\n$ git status";
        let cmds = extract_shell_commands(response);
        assert_eq!(cmds, vec!["ls", "cargo check", "git status"]);
    }
}
