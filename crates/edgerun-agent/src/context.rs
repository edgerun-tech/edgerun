//! Context management for 32k token budget.
//!
//! Devstral-small-2:24b has 32k context window (~24k chars conservative).
//! We need to be extremely efficient with context allocation.

use edgerun_json::{from_str, json, Value};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

const MAX_TOKENS: usize = 32768;
const TOKEN_TO_CHAR_RATIO: usize = 4;
const MAX_CHARS: usize = MAX_TOKENS * TOKEN_TO_CHAR_RATIO;

#[derive(Debug, Clone, Default)]
pub struct ContextBudget {
    pub system: String,
    pub code_context: String,
    pub task: String,
    pub history: String,
}

impl ContextBudget {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_system(mut self, system: &str) -> Self {
        self.system = Self::truncate(system, 2000);
        self
    }

    pub fn with_code_context(mut self, context: &str) -> Self {
        self.code_context = Self::truncate(context, 15000);
        self
    }

    pub fn with_task(mut self, task: &str) -> Self {
        self.task = Self::truncate(task, 2000);
        self
    }

    pub fn with_history(mut self, history: &str) -> Self {
        self.history = Self::truncate(history, 4000);
        self
    }

    fn truncate(s: &str, max_chars: usize) -> String {
        if s.len() > max_chars {
            format!(
                "{}...[truncated {} chars]",
                &s[..max_chars - 20],
                s.len() - max_chars + 20
            )
        } else {
            s.to_string()
        }
    }

    pub fn total_chars(&self) -> usize {
        self.system.len() + self.code_context.len() + self.task.len() + self.history.len()
    }

    pub fn fits(&self) -> bool {
        self.total_chars() <= MAX_CHARS
    }

    pub fn to_prompt(&self) -> String {
        let mut parts = Vec::new();

        if !self.system.is_empty() {
            parts.push(self.system.clone());
        }

        if !self.code_context.is_empty() {
            parts.push(format!("## Code Context\n{}", self.code_context));
        }

        if !self.task.is_empty() {
            parts.push(format!("## Task\n{}", self.task));
        }

        if !self.history.is_empty() {
            parts.push(format!("## Conversation History\n{}", self.history));
        }

        parts.join("\n\n")
    }

    pub fn compress(&mut self) {
        self.code_context = Self::truncate(&self.code_context, 8000);
        self.history = Self::truncate(&self.history, 2000);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeSummary {
    pub file_count: usize,
    pub func_count: usize,
    pub edge_count: usize,
    pub languages: Vec<(String, usize)>,
    pub top_functions: Vec<(String, usize)>,
    pub top_files: Vec<(String, usize)>,
}

impl CodeSummary {
    pub fn from_graph_json(graph_json: &str) -> Self {
        let graph: edgerun_json::Value = match from_str(graph_json) {
            Ok(v) => v,
            Err(_) => return Self::default(),
        };

        let nodes = graph["nodes"]
            .as_array()
            .map(|a| a.as_slice())
            .unwrap_or(&[]);
        let edges = graph["edges"]
            .as_array()
            .map(|a| a.as_slice())
            .unwrap_or(&[]);

        let file_count = {
            let mut files = std::collections::HashSet::new();
            for n in nodes {
                if let Some(f) = n["file"].as_str() {
                    files.insert(f);
                }
            }
            files.len()
        };
        let func_count = nodes.len();
        let edge_count = edges.len();

        let mut lang_counts = std::collections::HashMap::new();
        for n in nodes {
            if let Some(lang) = n["language"].as_str() {
                *lang_counts.entry(lang.to_string()).or_insert(0) += 1;
            }
        }
        let mut languages: Vec<_> = lang_counts.into_iter().collect();
        languages.sort_by_key(|b| std::cmp::Reverse(b.1));
        languages.truncate(10);

        let mut conn_counts = std::collections::HashMap::new();
        for e in edges {
            if let Some(source) = e["source"].as_str() {
                *conn_counts.entry(source.to_string()).or_insert(0) += 1;
            }
            if let Some(target) = e["target"].as_str() {
                *conn_counts.entry(target.to_string()).or_insert(0) += 1;
            }
        }
        let mut conn_vec: Vec<_> = conn_counts.into_iter().collect();
        conn_vec.sort_by_key(|b| std::cmp::Reverse(b.1));
        let top_functions: Vec<_> = conn_vec.into_iter().take(10).collect();

        let mut file_func_counts = std::collections::HashMap::new();
        for n in nodes {
            if let Some(f) = n["file"].as_str() {
                *file_func_counts.entry(f.to_string()).or_insert(0) += 1;
            }
        }
        let mut file_stats: Vec<_> = file_func_counts.into_iter().collect();
        file_stats.sort_by_key(|b| std::cmp::Reverse(b.1));
        let top_files: Vec<_> = file_stats.into_iter().take(10).collect();

        Self {
            file_count,
            func_count,
            edge_count,
            languages,
            top_functions,
            top_files,
        }
    }

    pub fn to_context(&self) -> String {
        let mut ctx = format!(
            "## Codebase Overview\n- Files: {}\n- Functions: {}\n- Relationships: {}\n\n",
            self.file_count, self.func_count, self.edge_count
        );

        ctx.push_str("## Languages\n");
        for (lang, count) in &self.languages {
            ctx.push_str(&format!("- {}: {} functions\n", lang, count));
        }
        ctx.push('\n');

        if !self.top_functions.is_empty() {
            ctx.push_str("## Top 10 Most-Connected Functions\n");
            for (func, connections) in &self.top_functions {
                let display_name = func.rsplit("::").next().unwrap_or(func);
                ctx.push_str(&format!(
                    "- {} ({} connections)\n",
                    display_name, connections
                ));
            }
            ctx.push('\n');
        }

        if !self.top_files.is_empty() {
            ctx.push_str("## Files with Most Functions\n");
            for (file, count) in &self.top_files {
                ctx.push_str(&format!("- {} ({} functions)\n", file, count));
            }
        }

        ctx
    }
}

impl Default for CodeSummary {
    fn default() -> Self {
        Self {
            file_count: 0,
            func_count: 0,
            edge_count: 0,
            languages: vec![],
            top_functions: vec![],
            top_files: vec![],
        }
    }
}

pub struct ConversationHistory {
    messages: VecDeque<Message>,
    max_messages: usize,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

impl ConversationHistory {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            max_messages,
        }
    }

    pub fn add(&mut self, role: MessageRole, content: String) {
        self.messages.push_back(Message { role, content });
        while self.messages.len() > self.max_messages {
            self.messages.pop_front();
        }
    }

    pub fn to_context(&self) -> String {
        self.messages
            .iter()
            .map(|m| {
                let role = match m.role {
                    MessageRole::User => "User",
                    MessageRole::Assistant => "Assistant",
                    MessageRole::System => "System",
                };
                format!("{}: {}", role, m.content)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn clear(&mut self) {
        self.messages.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_budget_truncation() {
        let budget = ContextBudget::new()
            .with_system("a".repeat(3000).as_str())
            .with_code_context("b".repeat(20000).as_str())
            .with_task("c".repeat(3000).as_str());

        assert!(budget.total_chars() <= MAX_CHARS);
    }
}

// Benchmark comment