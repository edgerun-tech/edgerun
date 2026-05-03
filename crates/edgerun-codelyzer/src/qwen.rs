//! Qwen CLI integration.
//! Spawns the host `qwen` CLI with the given prompt and returns the output.

use std::{
    collections::HashMap,
    io::Read,
    process::{Command, Stdio},
    time::Duration,
};

use serde::{Deserialize, Serialize};

// use crate::server::GraphData;  // Old server - now using server_http
use crate::server_http::GraphData;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatRequest {
    pub message: String,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub reply: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolUseResponse {
    pub reply: String,
    pub tools: Vec<ToolCallInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ToolCallInfo {
    pub name: String,
    pub args: String,
    pub output: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// Call the qwen CLI with a prompt.
/// The CLI is already authenticated on the host.
#[allow(dead_code)]
pub fn chat(message: &str, context: Option<&str>) -> ChatResponse {
    let full_prompt = match context {
        Some(ctx) => format!(
            "You are a code analysis assistant. Here is the code context:\n\n{}\n\nUser question: \
             {}",
            ctx, message
        ),
        None => message.to_string(),
    };

    // Try common qwen locations
    let qwen_path = std::env::var("QWEN_BIN")
        .ok()
        .or_else(|| {
            ["/home/ken/.npm-global/bin/qwen", "/usr/local/bin/qwen", "/usr/bin/qwen"]
                .iter()
                .find(|p| std::path::Path::new(p).exists())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "qwen".to_string());

    let mut child = match Command::new(&qwen_path)
        .args(["-p", &full_prompt])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return ChatResponse {
                reply: String::new(),
                error: Some(format!("Failed to start qwen CLI: {}", e)),
            }
        }
    };

    let mut stdout = String::new();
    let mut stderr = String::new();

    if let Some(ref mut out) = child.stdout {
        let _ = out.read_to_string(&mut stdout);
    }
    if let Some(ref mut err) = child.stderr {
        let _ = err.read_to_string(&mut stderr);
    }

    match child.wait() {
        Ok(status) if status.success() => {
            // Strip the initial emoji/decorative lines, keep the core reply
            let reply = stdout.trim().to_string();
            ChatResponse {
                reply,
                error: if stderr.is_empty() { None } else { Some(stderr.trim().to_string()) },
            }
        }
        Ok(status) => ChatResponse {
            reply: String::new(),
            error: Some(format!("qwen exited with status {}: {}", status, stderr.trim())),
        },
        Err(e) => ChatResponse {
            reply: String::new(),
            error: Some(format!("Failed to wait for qwen: {}", e)),
        },
    }
}

/// Call qwen with a timeout. Kills the process if it takes too long.
pub fn chat_with_timeout(message: &str, context: Option<&str>, timeout_secs: u64) -> ChatResponse {
    let full_prompt = match context {
        Some(ctx) => format!(
            "You are a code analysis assistant. Here is the code context:\n\n{}\n\nUser question: \
             {}",
            ctx, message
        ),
        None => message.to_string(),
    };

    let qwen_path = std::env::var("QWEN_BIN")
        .ok()
        .or_else(|| {
            ["/home/ken/.npm-global/bin/qwen", "/usr/local/bin/qwen", "/usr/bin/qwen"]
                .iter()
                .find(|p| std::path::Path::new(p).exists())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "qwen".to_string());

    let mut child = match Command::new(&qwen_path)
        .args(["-p", &full_prompt])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return ChatResponse {
                reply: String::new(),
                error: Some(format!("Failed to start qwen CLI: {}", e)),
            }
        }
    };

    // Wait with timeout
    let deadline = std::time::Instant::now() + Duration::from_secs(timeout_secs);
    loop {
        if let Ok(Some(status)) = child.try_wait() {
            let mut stdout = String::new();
            let mut stderr = String::new();
            if let Some(ref mut out) = child.stdout {
                let _ = out.read_to_string(&mut stdout);
            }
            if let Some(ref mut err) = child.stderr {
                let _ = err.read_to_string(&mut stderr);
            }
            if status.success() {
                return ChatResponse {
                    reply: stdout.trim().to_string(),
                    error: if stderr.is_empty() { None } else { Some(stderr.trim().to_string()) },
                };
            } else {
                return ChatResponse {
                    reply: String::new(),
                    error: Some(format!("qwen exited {}: {}", status, stderr.trim())),
                };
            }
        }
        if std::time::Instant::now() >= deadline {
            let _ = child.kill();
            return ChatResponse {
                reply: String::new(),
                error: Some(format!("qwen timed out after {}s", timeout_secs)),
            };
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// Build a concise codebase summary from the graph data.
/// Includes file count, function count, edge count, language breakdown,
/// top 10 most-connected functions, and file-level stats.
/// Keeps output under ~2000 chars.
pub fn build_codebase_summary(graph: &GraphData) -> String {
    let file_count: usize = {
        let mut files = std::collections::HashSet::new();
        for n in &graph.nodes {
            files.insert(n.file.as_str());
        }
        files.len()
    };
    let func_count = graph.nodes.len();
    let edge_count = graph.edges.len();

    let mut lang_counts: HashMap<String, usize> = HashMap::new();
    for n in &graph.nodes {
        *lang_counts.entry(n.language.clone()).or_insert(0) += 1;
    }
    let mut lang_breakdown: Vec<_> = lang_counts.into_iter().collect();
    lang_breakdown.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut conn_counts: HashMap<String, usize> = HashMap::new();
    for e in &graph.edges {
        *conn_counts.entry(e.source.clone()).or_insert(0) += 1;
        *conn_counts.entry(e.target.clone()).or_insert(0) += 1;
    }
    let mut conn_vec: Vec<_> = conn_counts.into_iter().collect();
    conn_vec.sort_by_key(|b| std::cmp::Reverse(b.1));
    let top_connections: Vec<_> = conn_vec.into_iter().take(10).collect();

    let mut file_func_counts: HashMap<String, usize> = HashMap::new();
    for n in &graph.nodes {
        *file_func_counts.entry(n.file.clone()).or_insert(0) += 1;
    }
    let mut file_stats: Vec<_> = file_func_counts.into_iter().collect();
    file_stats.sort_by_key(|b| std::cmp::Reverse(b.1));
    let top_files: Vec<_> = file_stats.into_iter().take(10).collect();

    let mut summary = String::new();
    summary.push_str(&format!(
        "## Codebase Overview\n- Files: {}\n- Functions: {}\n- Relationships: {}\n\n",
        file_count, func_count, edge_count
    ));

    summary.push_str("## Languages\n");
    for (lang, count) in &lang_breakdown {
        summary.push_str(&format!("- {}: {} functions\n", lang, count));
    }
    summary.push('\n');

    if !top_connections.is_empty() {
        summary.push_str("## Top 10 Most-Connected Functions\n");
        for (func, connections) in &top_connections {
            let display_name = func.rsplit("::").next().unwrap_or(func);
            summary.push_str(&format!("- {} ({} connections)\n", display_name, connections));
        }
        summary.push('\n');
    }

    if !top_files.is_empty() {
        summary.push_str("## Files with Most Functions\n");
        for (file, count) in &top_files {
            summary.push_str(&format!("- {} ({} functions)\n", file, count));
        }
    }

    if summary.len() > 2000 {
        summary.truncate(2000);
        summary.push_str("...[truncated]");
    }
    summary
}

/// Context-aware chat. Includes codebase summary and diagnostics as context.
/// Single call to qwen — no tool loop (all context is pre-collected).
pub fn chat_with_tools(message: &str, summary: Option<&str>) -> ToolUseResponse {
    let system_context = r#"You are a code analysis assistant. You have full context of the codebase including:
- Codebase structure (files, functions, call graph)
- Available linters and their results
- Test files found
- Formatting issues
- Missing/unresolved function references
Answer concisely. If asked about code, reference specific files and functions."#;

    let full_prompt = if let Some(s) = summary {
        format!("{}\n\n## Context\n{}\n\n## Question\n{}", system_context, s, message)
    } else {
        format!("{}\n\n## Question\n{}", system_context, message)
    };

    let response = chat_with_timeout(&full_prompt, None, 30);

    ToolUseResponse { reply: response.reply, tools: vec![], error: response.error }
}

/// Parse tool calls from a response string.
/// Looks for patterns like [TOOL: tool_name(arg="value")]
#[allow(dead_code)]
fn parse_tool_calls(text: &str) -> Vec<crate::tools::ToolCall> {
    let mut calls = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if let Some(start) = line.find("[TOOL: ") {
            let rest = &line[start + 7..];
            if let Some(end) = rest.find(']') {
                let tool_spec = &rest[..end];
                if let Some(tc) = crate::tools::ToolCall::from_spec(tool_spec) {
                    calls.push(tc);
                }
            }
        }
    }
    calls
}
