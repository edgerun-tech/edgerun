//! AI-powered coding agent that connects to a TabbyAPI server.

use crate::client::{ChatRequest, ChatResponse, TabbyClient};
use crate::context::ConversationHistory;
use crate::prompts::Prompts;
use crate::tools::ToolExecutor;
use std::sync::Arc;
use edgerun_rt::sync::RwLock;

const MAX_TOOL_ITERATIONS: usize = 10;

#[derive(Clone)]
pub struct Agent {
    client: Arc<TabbyClient>,
    context: Arc<RwLock<AgentContext>>,
    executor: Arc<ToolExecutor>,
}

struct AgentContext {
    history: ConversationHistory,
    system_prompt: String,
}

impl AgentContext {
    fn new() -> Self {
        Self {
            history: ConversationHistory::new(50),
            system_prompt: Prompts::system().to_string(),
        }
    }
}

impl Agent {
    pub fn new(tabby_url: &str, model: &str, project_root: &str) -> Self {
        let client = Arc::new(TabbyClient::new(tabby_url, model));
        let context = Arc::new(RwLock::new(AgentContext::new()));
        let executor = Arc::new(ToolExecutor::new(project_root));

        Self {
            client,
            context,
            executor,
        }
    }

    pub fn with_allowed_commands(self, commands: Vec<String>) -> Self {
        let executor = Arc::new(
            Arc::try_unwrap(self.executor)
                .unwrap_or_else(|arc| (*arc).clone())
                .with_allowed_commands(commands),
        );
        Self {
            client: self.client,
            context: self.context,
            executor,
        }
    }

    pub async fn chat(&self, message: &str) -> Result<ChatResponse, String> {
        let (history, system) = {
            let ctx = self.context.read();
            (ctx.history.to_string(), ctx.system_prompt.clone())
        };

        let prompt = format!("{}\n\n{}", system, history);

        let response = self
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
            let mut ctx = self.context.write();
            ctx.history.add(crate::context::MessageRole::User, message.to_string());
            ctx.history.add(
                crate::context::MessageRole::Assistant,
                response.reply.clone(),
            );
        }

        Ok(response)
    }

    pub async fn chat_with_tools(&self, message: &str) -> Result<ChatResponse, String> {
        let (history, system) = {
            let ctx = self.context.read();
            (ctx.history.to_string(), ctx.system_prompt.clone())
        };

        let allowed = self.executor.get_allowed_commands();
        let tools_prompt = format!(
            "You have shell access. You can run these commands: {}\n\
             When you need to run a command, respond with:\n\
             $ command args...\n\n\
             You can run multiple commands. The output will be shown to you.\n\
             After seeing the results, provide your final answer.\n",
            allowed.join(", ")
        );

        let mut current_message = format!(
            "{}\n\n{}\n\nConversation history:\n{}\n\nUser: {}",
            system, tools_prompt, history, message
        );

        for _ in 0..MAX_TOOL_ITERATIONS {
            let response = self
                .client
                .chat(ChatRequest {
                    message: current_message.clone(),
                    context: None,
                    system_prompt: None,
                    max_tokens: Some(4096),
                    temperature: Some(0.3),
                })
                .await?;

            let tool_commands = self.extract_shell_commands(&response.reply);

            if tool_commands.is_empty() {
                {
                    let mut ctx = self.context.write();
                    ctx.history.add(crate::context::MessageRole::User, message.to_string());
                    ctx.history.add(
                        crate::context::MessageRole::Assistant,
                        response.reply.clone(),
                    );
                }
                return Ok(response);
            }

            let mut results = Vec::new();
            for cmd in &tool_commands {
                let result = self.executor.execute_shell(cmd);
                results.push(format!(
                    "$ {}\n{}{}",
                    cmd,
                    result.output,
                    result
                        .error
                        .map(|e| format!("\n[stderr]: {}", e))
                        .unwrap_or_default()
                ));
            }

            let tool_output = results.join("\n\n");

            if tool_output.len() > 8000 {
                current_message = format!(
                    "Previous response:\n{}\n\nTool output (truncated):\n{}\n\nContinue with your answer.",
                    &response.reply[..response.reply.len().min(2000)],
                    &tool_output[..tool_output.len().min(6000)]
                );
            } else {
                current_message = format!(
                    "Previous response:\n{}\n\nTool output:\n{}\n\nContinue your analysis. Provide your final answer or run more commands.",
                    response.reply, tool_output
                );
            }
        }

        Ok(ChatResponse {
            reply: "Tool execution limit reached. Please try a simpler request.".to_string(),
            usage: None,
            error: Some("Max tool iterations exceeded".to_string()),
        })
    }

    fn extract_shell_commands(&self, response: &str) -> Vec<String> {
        let mut commands = Vec::new();

        for line in response.lines() {
            let trimmed = line.trim();
            let cmd = if trimmed.starts_with("$ ") {
                trimmed[2..].to_string()
            } else if trimmed.starts_with("```sh\n") || trimmed.starts_with("```bash\n") {
                continue;
            } else if trimmed.starts_with("```") {
                continue;
            } else {
                continue;
            };

            if !cmd.is_empty() {
                commands.push(cmd);
            }
        }

        commands
    }

    pub fn clear_history(&self) {
        let mut ctx = self.context.write();
        ctx.history.clear();
    }

    pub fn get_allowed_commands(&self) -> Vec<String> {
        self.executor.get_allowed_commands().to_vec()
    }
}

impl Default for Agent {
    fn default() -> Self {
        Self::new("http://10.10.10.1:5001", "devstral-small-2:24b", ".")
    }
}