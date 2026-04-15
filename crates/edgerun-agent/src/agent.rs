//! Main coding agent orchestrator.
//!
//! Combines TabbyClient, ContextManager, and edgerun-edit for AI-powered code operations.

use crate::client::{ChatRequest, ChatResponse, TabbyClient};
use crate::context::{CodeSummary, ContextBudget, ConversationHistory, MessageRole};
use crate::prompts::Prompts;
use crate::tools::{ToolCall, ToolExecutor, ToolResult};
use crate::vfs::SharedVFS;
use edgerun_rt::sync::RwLock;
use std::sync::Arc;

const MAX_TOOL_ITERATIONS: usize = 5;

#[derive(Clone)]
pub struct Agent {
    client: Arc<TabbyClient>,
    context: Arc<RwLock<ContextManager>>,
    executor: Arc<ToolExecutor>,
}

struct ContextManager {
    budget: ContextBudget,
    history: ConversationHistory,
    code_summary: Option<CodeSummary>,
}

impl ContextManager {
    fn new() -> Self {
        Self {
            budget: ContextBudget::new(),
            history: ConversationHistory::new(20),
            code_summary: None,
        }
    }

    fn set_code_summary(&mut self, graph_json: &str) {
        self.code_summary = Some(CodeSummary::from_graph_json(graph_json));
        self.budget = self.budget.clone().with_code_context(
            self.code_summary.as_ref().map(|s| s.to_context()).unwrap_or_default().as_str()
        );
    }

    fn add_message(&mut self, role: MessageRole, content: String) {
        self.history.add(role, content);
        self.budget = self.budget.clone().with_history(self.history.to_context().as_str());
    }

    fn clear(&mut self) {
        self.history.clear();
        self.budget = self.budget.clone().with_history("");
    }
}

impl Agent {
    pub fn new(tabby_url: &str, model: &str, project_root: &str) -> Self {
        let client = Arc::new(TabbyClient::new(tabby_url, model));
        let context = Arc::new(RwLock::new(ContextManager::new()));
        let executor = Arc::new(ToolExecutor::new(project_root));

        Self { client, context, executor }
    }

    pub fn with_vfs(mut self, vfs: SharedVFS) -> Self {
        let executor = Arc::new(
            Arc::try_unwrap(self.executor)
                .unwrap_or_else(|arc| (*arc).clone())
                .with_vfs(vfs)
        );
        Self {
            client: self.client,
            context: self.context,
            executor,
        }
    }

    pub fn with_code_summary(&self, graph_json: &str) {
        let mut ctx = self.context.write();
        ctx.set_code_summary(graph_json);
    }

    pub async fn chat(&self, message: &str) -> Result<ChatResponse, String> {
        let (code_ctx, history) = {
            let ctx = self.context.read();
            (ctx.budget.code_context.clone(), ctx.budget.history.clone())
        };

        let response = self.client.chat(ChatRequest {
            message: message.to_string(),
            context: Some(code_ctx),
            system_prompt: Some(Prompts::system().to_string()),
            max_tokens: Some(4096),
            temperature: Some(0.7),
        }).await?;

        let mut ctx = self.context.write();
        ctx.add_message(MessageRole::User, message.to_string());
        ctx.add_message(MessageRole::Assistant, response.reply.clone());

        Ok(response)
    }

    pub async fn chat_with_tools(&self, message: &str) -> Result<ChatResponse, String> {
        let (code_ctx, _history) = {
            let ctx = self.context.read();
            (ctx.budget.code_context.clone(), ctx.budget.history.clone())
        };

        let tools_json = self.get_tools_prompt();
        let full_message = format!(
            "{}\n\n{}\n\nUser: {}",
            tools_json, code_ctx, message
        );

        let mut current_message = full_message;
        let mut iteration = 0;

        loop {
            iteration += 1;
            if iteration > MAX_TOOL_ITERATIONS {
                return Ok(ChatResponse {
                    reply: "Tool execution limit reached. Please try a simpler request.".to_string(),
                    usage: None,
                    error: Some("Max tool iterations exceeded".to_string()),
                });
            }

            let response = self.client.chat(ChatRequest {
                message: current_message.clone(),
                context: None,
                system_prompt: Some(Prompts::system().to_string()),
                max_tokens: Some(4096),
                temperature: Some(0.7),
            }).await?;

            let reply = &response.reply;

            if let Some(tool_calls) = self.extract_tool_calls(reply) {
                let mut tool_results = Vec::new();
                for call in tool_calls {
                    let result = self.executor.execute(&call.name, call.arguments.clone());
                    tool_results.push(format!(
                        "Tool '{}' result: {}",
                        call.name,
                        if result.success {
                            result.output
                        } else {
                            result.error.unwrap_or_else(|| "Unknown error".to_string())
                        }
                    ));
                }

                current_message = format!(
                    "Previous: {}\n\nTool results:\n{}",
                    reply,
                    tool_results.join("\n")
                );
            } else {
                let mut ctx = self.context.write();
                ctx.add_message(MessageRole::User, message.to_string());
                ctx.add_message(MessageRole::Assistant, response.reply.clone());
                return Ok(response);
            }
        }
    }

    fn get_tools_prompt(&self) -> String {
        let tools = self.executor.get_enabled_tools();
        let mut prompt = String::from("You have access to a set of tools. When you need to use a tool, respond with the tool call in the following format:\n");
        prompt.push_str("tool_name(arg1=\"value1\", arg2=\"value2\")\n\n");
        prompt.push_str("Available tools:\n");
        for tool in tools {
            if tool.enabled {
                prompt.push_str(&format!("- {}: {}\n", tool.name, tool.description));
            }
        }
        prompt.push_str("\nUse tool calls in your response like: read_file(path=\"src/main.rs\")");
        prompt
    }

    fn extract_tool_calls(&self, response: &str) -> Option<Vec<ToolCall>> {
        let mut calls = Vec::new();
        for line in response.lines() {
            let line = line.trim();
            if let Some(call) = ToolCall::from_spec(line) {
                if self.executor.is_tool_available(&call.name) {
                    calls.push(call);
                }
            }
        }
        if calls.is_empty() { None } else { Some(calls) }
    }

    pub async fn generate_code(&self, description: &str) -> Result<ChatResponse, String> {
        self.chat(&format!("Generate Rust code:\n{}", description)).await
    }

    pub async fn debug_code(&self, code: &str, error: &str) -> Result<ChatResponse, String> {
        let prompt = format!(
            "Debug this Rust code. Error: {}\n\nCode:\n```rust\n{}\n```",
            error, code
        );
        self.chat(&prompt).await
    }

    pub async fn generate_tests(&self, code: &str) -> Result<ChatResponse, String> {
        let prompt = format!(
            "Generate unit tests for this Rust code:\n```rust\n{}\n```",
            code
        );
        self.chat(&prompt).await
    }

    pub fn execute_tool(&self, tool_name: &str, args: std::collections::HashMap<String, String>) -> ToolResult {
        self.executor.execute(tool_name, args)
    }

    pub fn get_available_tools(&self) -> Vec<crate::tools::ToolDefinition> {
        self.executor.get_enabled_tools()
    }

    pub fn clear_history(&self) {
        let mut ctx = self.context.write();
        ctx.clear();
    }
}

impl Default for Agent {
    fn default() -> Self {
        Self::new("http://10.10.10.1:5001", "devstral-small-2:24b", ".")
    }
}

#[cfg(test)]
mod tests {
    use super::*;


}
// Benchmark comment