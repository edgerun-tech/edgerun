use edgerun_http::{HttpClient, HttpVersion, Method, Request};
use edgerun_json::{from_slice, json, to_string};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

struct Client {
    http: HttpClient,
}

impl Client {
    fn new() -> Self {
        Self {
            http: HttpClient::new().version(HttpVersion::Http1),
        }
    }

    async fn chat(&self, messages: &[Message]) -> Result<String, String> {
        let body = json!({
            "model": "big-pickle",
            "messages": messages.iter().map(|m| json!({
                "role": m.role,
                "content": m.content.clone()
            })).collect::<Vec<_>>(),
            "stream": true
        });

        let body_str = to_string(&body).map_err(|e| format!("json serialize: {}", e))?;

        let req = Request::builder()
            .method(Method::POST)
            .uri("https://opencode.ai/zen/v1/chat/completions")
            .header("Content-Type", "application/json")
            .body(body_str.into_bytes())
            .build()
            .map_err(|e| format!("request build: {}", e))?;

        let mut full_response = String::new();
        let mut buffer = String::new();
        let mut done = false;

        self.http
            .execute_http1_body_chunks(&req, |chunk| {
                if done {
                    return Ok(());
                }

                if let Ok(text) = String::from_utf8(chunk.to_vec()) {
                    buffer.push_str(&text);

                    while let Some(pos) = buffer.find("\n\n") {
                        let message = buffer[..pos].to_string();
                        buffer.drain(..pos + 2);

                        for line in message.lines() {
                            if line.starts_with("data: ") {
                                let data = &line[6..];
                                if data == "[DONE]" {
                                    done = true;
                                    return Ok(());
                                }
                                // Skip SSE comments
                                if data.starts_with(':') {
                                    continue;
                                }
                                if let Ok(json) = from_slice(data.as_bytes()) {
                                    if let Some(content) =
                                        json["choices"][0]["delta"]["content"].as_str()
                                    {
                                        print!("{}", content);
                                        io::stdout().flush().ok();
                                        full_response.push_str(content);
                                    }
                                }
                            }
                        }
                    }
                }
                Ok(())
            })
            .await
            .map_err(|e| format!("http stream: {}", e))?;

        // Process any remaining buffer content
        if !buffer.is_empty() && !done {
            for line in buffer.lines() {
                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        break;
                    }
                    if data.starts_with(':') {
                        continue;
                    }
                    if let Ok(json) = from_slice(data.as_bytes()) {
                        if let Some(content) = json["choices"][0]["delta"]["content"].as_str() {
                            print!("{}", content);
                            io::stdout().flush().ok();
                            full_response.push_str(content);
                        }
                    }
                }
            }
        }

        println!();
        Ok(full_response)
    }
}

#[derive(Clone)]
struct Message {
    role: &'static str,
    content: String,
}

fn parse_response(text: &str) -> (String, Vec<Action>) {
    let mut actions = Vec::new();
    let mut clean = String::new();
    let mut idx = 0;

    while idx < text.len() {
        let rest = &text[idx..];
        if rest.starts_with("[action:") {
            let start = idx + 8;
            if let Some(end) = text[start..].find(']') {
                let inner = &text[start..start + end];
                if let Some(colon) = inner.find(':') {
                    actions.push(Action {
                        action: inner[..colon].to_string(),
                        payload: inner[colon + 1..].to_string(),
                    });
                }
                idx = start + end + 1;
                continue;
            }
        }

        let ch = rest.chars().next().expect("idx is within text");
        clean.push(ch);
        idx += ch.len_utf8();
    }

    (clean.trim().to_string(), actions)
}

#[derive(Debug, Clone)]
struct Action {
    action: String,
    payload: String,
}

#[derive(Debug, Clone)]
struct ToolResult {
    action: String,
    ok: bool,
    output: String,
}

impl ToolResult {
    fn success(action: &Action, output: String) -> Self {
        Self {
            action: action.action.clone(),
            ok: true,
            output,
        }
    }

    fn failure(action: &Action, output: String) -> Self {
        Self {
            action: action.action.clone(),
            ok: false,
            output,
        }
    }

    fn as_prompt_text(&self) -> String {
        let status = if self.ok { "success" } else { "failed" };
        format!(
            "tool: {}\nstatus: {}\noutput:\n{}\n---END TOOL RESULT---",
            self.action, status, self.output
        )
    }
}

fn execute_action(action: &Action) -> Result<String, String> {
    match action.action.as_str() {
        "read-file" => {
            let path = Path::new(&action.payload);
            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("reading {}: {}", path.display(), e))?;
            Ok(format!(
                "Contents of {}:\n{}\n---END---",
                path.display(),
                content
            ))
        }
        "edit-file" => {
            let parts: Vec<&str> = action.payload.splitn(3, '|').collect();
            if parts.len() != 3 {
                return Err("edit-file: expected path|old|new".to_string());
            }
            let path = Path::new(parts[0]);
            let old = parts[1];
            let new = parts[2];

            let content = std::fs::read_to_string(path)
                .map_err(|e| format!("reading {}: {}", path.display(), e))?;

            if !content.contains(old) {
                return Err(format!("old text not found in {}", path.display()));
            }

            let new_content = content.replace(old, new);
            std::fs::write(path, new_content)
                .map_err(|e| format!("writing {}: {}", path.display(), e))?;
            Ok(format!("Edited {}", path.display()))
        }
        #[cfg(feature = "ast-edit")]
        "add-use" => {
            let parts: Vec<&str> = action.payload.splitn(2, '|').collect();
            if parts.len() != 2 {
                return Err("add-use: expected path|use_path".to_string());
            }
            let path = Path::new(parts[0]);
            let use_path = parts[1];
            let mut file = edgerun_edit::edit_ops::parse_file(path)
                .map_err(|e| format!("parse {}: {}", path.display(), e))?;
            edgerun_edit::edit_ops::add_use(&mut file, use_path)
                .map_err(|e| format!("add-use: {}", e))?;
            edgerun_edit::edit_ops::write_file(path, &file)
                .map_err(|e| format!("write {}: {}", path.display(), e))?;
            Ok(format!("Added use {} to {}", use_path, path.display()))
        }
        #[cfg(not(feature = "ast-edit"))]
        "add-use" => ast_edit_disabled("add-use"),
        #[cfg(feature = "ast-edit")]
        "rename-type" => {
            let parts: Vec<&str> = action.payload.splitn(3, '|').collect();
            if parts.len() != 3 {
                return Err("rename-type: expected path|old|new".to_string());
            }
            let path = Path::new(parts[0]);
            let mut file = edgerun_edit::edit_ops::parse_file(path)
                .map_err(|e| format!("parse {}: {}", path.display(), e))?;
            edgerun_edit::edit_ops::rename_type_in_file(&mut file, parts[1], parts[2]);
            edgerun_edit::edit_ops::write_file(path, &file)
                .map_err(|e| format!("write {}: {}", path.display(), e))?;
            Ok(format!("Renamed type in {}", path.display()))
        }
        #[cfg(not(feature = "ast-edit"))]
        "rename-type" => ast_edit_disabled("rename-type"),
        #[cfg(feature = "ast-edit")]
        "add-fn" => {
            let parts: Vec<&str> = action.payload.splitn(3, '|').collect();
            if parts.len() != 3 {
                return Err("add-fn: expected path|fn_name|fn_body".to_string());
            }
            let path = Path::new(parts[0]);
            let mut file = edgerun_edit::edit_ops::parse_file(path)
                .map_err(|e| format!("parse {}: {}", path.display(), e))?;
            edgerun_edit::edit_ops::add_fn(&mut file, parts[1], "", "", parts[2])
                .map_err(|e| format!("add fn: {}", e))?;
            edgerun_edit::edit_ops::write_file(path, &file)
                .map_err(|e| format!("write {}: {}", path.display(), e))?;
            Ok(format!("Added fn {} to {}", parts[1], path.display()))
        }
        #[cfg(not(feature = "ast-edit"))]
        "add-fn" => ast_edit_disabled("add-fn"),
        #[cfg(feature = "ast-edit")]
        "remove-fn" => {
            let parts: Vec<&str> = action.payload.splitn(2, '|').collect();
            if parts.len() != 2 {
                return Err("remove-fn: expected path|fn_name".to_string());
            }
            let path = Path::new(parts[0]);
            let mut file = edgerun_edit::edit_ops::parse_file(path)
                .map_err(|e| format!("parse {}: {}", path.display(), e))?;
            if edgerun_edit::edit_ops::remove_fn(&mut file, parts[1]) {
                edgerun_edit::edit_ops::write_file(path, &file)
                    .map_err(|e| format!("write {}: {}", path.display(), e))?;
                Ok(format!("Removed fn {} from {}", parts[1], path.display()))
            } else {
                Err(format!("fn {} not found", parts[1]))
            }
        }
        #[cfg(not(feature = "ast-edit"))]
        "remove-fn" => ast_edit_disabled("remove-fn"),
        #[cfg(feature = "ast-edit")]
        "list-files" => {
            let path = Path::new(&action.payload);
            let files = edgerun_edit::edit_ops::list_file(path)
                .map_err(|e| format!("list {}: {}", path.display(), e))?;
            Ok(format!("Files in {}: {:?}", path.display(), files))
        }
        #[cfg(not(feature = "ast-edit"))]
        "list-files" => {
            let path = Path::new(&action.payload);
            let entries = std::fs::read_dir(path)
                .map_err(|e| format!("list {}: {}", path.display(), e))?
                .filter_map(Result::ok)
                .map(|entry| entry.path().display().to_string())
                .collect::<Vec<_>>();
            Ok(format!("Files in {}: {:?}", path.display(), entries))
        }
        #[cfg(feature = "ast-edit")]
        "find-fn" => {
            let parts: Vec<&str> = action.payload.splitn(2, '|').collect();
            if parts.len() != 2 {
                return Err("find-fn: expected path|fn_name".to_string());
            }
            let path = Path::new(parts[0]);
            let file = edgerun_edit::edit_ops::parse_file(path)
                .map_err(|e| format!("parse {}: {}", path.display(), e))?;
            if let Some(sig) = edgerun_edit::edit_ops::find_fn(&file, parts[1]) {
                Ok(format!("Found: {}", sig))
            } else {
                Err(format!("fn {} not found", parts[1]))
            }
        }
        #[cfg(not(feature = "ast-edit"))]
        "find-fn" => ast_edit_disabled("find-fn"),
        #[cfg(feature = "ast-edit")]
        "replace-fn-body" => {
            let parts: Vec<&str> = action.payload.splitn(3, '|').collect();
            if parts.len() != 3 {
                return Err("replace-fn-body: expected path|fn_name|new_body".to_string());
            }
            let path = Path::new(parts[0]);
            let mut file = edgerun_edit::edit_ops::parse_file(path)
                .map_err(|e| format!("parse {}: {}", path.display(), e))?;
            edgerun_edit::edit_ops::replace_fn_body(&mut file, parts[1], parts[2])
                .map_err(|e| format!("replace fn body: {}", e))?;
            edgerun_edit::edit_ops::write_file(path, &file)
                .map_err(|e| format!("write {}: {}", path.display(), e))?;
            Ok(format!(
                "Replaced body of fn {} in {}",
                parts[1],
                path.display()
            ))
        }
        #[cfg(not(feature = "ast-edit"))]
        "replace-fn-body" => ast_edit_disabled("replace-fn-body"),
        #[cfg(feature = "ast-edit")]
        "add-derive" => {
            let parts: Vec<&str> = action.payload.splitn(3, '|').collect();
            if parts.len() != 3 {
                return Err("add-derive: expected path|type_name|derive_name".to_string());
            }
            let path = Path::new(parts[0]);
            let mut file = edgerun_edit::edit_ops::parse_file(path)
                .map_err(|e| format!("parse {}: {}", path.display(), e))?;
            if edgerun_edit::edit_ops::add_derive(&mut file, parts[1], parts[2])? {
                edgerun_edit::edit_ops::write_file(path, &file)
                    .map_err(|e| format!("write {}: {}", path.display(), e))?;
                Ok(format!(
                    "Added derive {} to {} in {}",
                    parts[2],
                    parts[1],
                    path.display()
                ))
            } else {
                Err(format!("type {} not found in {}", parts[1], path.display()))
            }
        }
        #[cfg(not(feature = "ast-edit"))]
        "add-derive" => ast_edit_disabled("add-derive"),
        "spawn-agent" => {
            let exe = std::env::current_exe().map_err(|e| format!("locate zen-client: {}", e))?;
            let output = Command::new(exe)
                .arg(&action.payload)
                .output()
                .map_err(|e| format!("spawn agent: {}", e))?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(format!("Agent output: {}", stdout))
        }
        _ => Err(format!("unknown action: {}", action.action)),
    }
}

#[cfg(not(feature = "ast-edit"))]
fn ast_edit_disabled(action: &str) -> Result<String, String> {
    Err(format!(
        "{action} requires building zen-client with the `ast-edit` feature"
    ))
}

fn gather_system_info() -> String {
    let mut info = String::new();

    if let Ok(cpu) = std::fs::read_to_string("/proc/cpuinfo") {
        for line in cpu.lines().take(5) {
            info.push_str(line);
            info.push('\n');
        }
    }

    if let Ok(mem) = std::fs::read_to_string("/proc/meminfo") {
        for line in mem.lines().take(3) {
            info.push_str(line);
            info.push('\n');
        }
    }

    if let Ok(output) = Command::new("ps").args(&["aux", "--sort=-%cpu"]).output() {
        if let Ok(ps) = String::from_utf8(output.stdout) {
            let lines: Vec<&str> = ps.lines().take(6).collect();
            info.push_str("\nTop processes:\n");
            info.push_str(&lines.join("\n"));
        }
    }

    info
}

fn build_repo_context() -> String {
    let mut ctx = String::new();
    ctx.push_str("### File Tree (filtered)\n```\n");

    let output = Command::new("git").args(&["ls-files"]).output();
    match output {
        Ok(o) if o.status.success() => {
            let files = String::from_utf8_lossy(&o.stdout);
            for line in files.lines() {
                if should_include_file(line) {
                    ctx.push_str(line);
                    ctx.push('\n');
                }
            }
        }
        _ => {
            let output = Command::new("find")
                .args(&["crates", "-name", "*.rs", "-not", "-path", "*/target/*"])
                .output();
            if let Ok(o) = output {
                let files = String::from_utf8_lossy(&o.stdout);
                for line in files.lines() {
                    if should_include_file(line) {
                        ctx.push_str(line);
                        ctx.push('\n');
                    }
                }
            }
        }
    }

    ctx.push_str("```\n");
    ctx
}

fn should_include_file(path: &str) -> bool {
    let exclude_patterns = [
        "target/",
        "node_modules/",
        ".git/",
        "Cargo.lock",
        "*.swp",
        "*.swo",
        ".DS_Store",
        "*.o",
        "*.a",
        "*.so",
        "*.dylib",
    ];

    for pat in &exclude_patterns {
        if pat.contains('*') {
            let suffix = &pat[1..];
            if path.ends_with(suffix) {
                return false;
            }
        } else if path.contains(pat) {
            return false;
        }
    }
    true
}

fn load_prompt_from_file(path: &str) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("Failed to read prompt file {}: {}", path, e))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <prompt-or-file>", args[0]);
        eprintln!("  If argument is a file that exists, loads prompt from it.");
        eprintln!("  Otherwise, uses the argument as the prompt.");
        eprintln!("");
        eprintln!("Agentic loop: will execute edit actions returned by the model");
        eprintln!("Actions: read-file, edit-file, add-use, rename-type, add-fn, remove-fn, find-fn, list-files, spawn-agent");
        std::process::exit(1);
    }

    let prompt = if Path::new(&args[1]).exists() {
        match load_prompt_from_file(&args[1]) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("{}", e);
                std::process::exit(1);
            }
        }
    } else {
        args[1..].join(" ")
    };

    let result: Result<(), String> = edgerun_rt::block_on(async {
        let client = Client::new();

        let self_source = std::fs::read_to_string("zen-client/src/main.rs")
            .or_else(|_| std::fs::read_to_string("crates/zen-client/src/main.rs"))
            .unwrap_or_else(|_| "Failed to read self source".to_string());

        let system_info = gather_system_info();
        let repo_context = build_repo_context();

        let mut iteration: u32 = 0;

        let system_prompt = format!(
            r#"You are an autonomous Rust coding agent running inside `zen-client`.

## Your Source Code
```
{}
```

## Execution Context
- **Iteration**: You are on iteration {} of the agentic loop
- **Repository**: Edgerun Core (workspace with 110+ crates)
- **System**: {}
- **Memory**: You will NOT remember anything after execution stops. Write important info down.

## Repository File Tree
```
{}
```

## Available Tools
Call tools by emitting one or more exact action tokens in your response:
`[action:tool-name:payload]`

| Tool | Format | Purpose |
|------|--------|---------|
| read-file | `[action:read-file:path]` | Read file contents |
| edit-file | `[action:edit-file:path|old|new]` | Exact text replacement |
| add-use | `[action:add-use:path|use_stmt]` | Add import |
| rename-type | `[action:rename-type:path|old|new]` | Rename type |
| add-fn | `[action:add-fn:path|name|body]` | Add function |
| remove-fn | `[action:remove-fn:path|name]` | Remove function |
| replace-fn-body | `[action:replace-fn-body:path|name|new_body]` | Replace function body |
| add-derive | `[action:add-derive:path|type|derive]` | Add derive attribute |
| find-fn | `[action:find-fn:path|name]` | Find function |
| list-files | `[action:list-files:path]` | List directory |
| spawn-agent | `[action:spawn-agent:prompt]` | Spawn new agent |

## Your Purpose
Figure out what you were built for and do what you think is best:
1. Explore the repo (use `list-files`, `read-file`)
2. Understand your own code
3. Spawn more agents if needed (via `spawn-agent`)
4. Keep notes by writing to files
5. Be proactive — you have agency

## Workflow
1. Read files first to understand structure
2. Use `edit-file` with exact old/new code
3. Keep responses concise
4. After finishing a task, explain what you did"#,
            self_source, iteration, system_info, repo_context
        );

        let mut messages = vec![
            Message {
                role: "system",
                content: system_prompt,
            },
            Message {
                role: "user",
                content: prompt,
            },
        ];

        loop {
            iteration += 1;

            let response = client.chat(&messages).await?;
            let (text, actions) = parse_response(&response);

            if !text.is_empty() {
                println!("\nAgent (iter {}): {}", iteration, text);
            }

            if actions.is_empty() {
                println!("\n[Enter new prompt, or 'exit' to quit]");
                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_ok() {
                    let trimmed = input.trim();
                    if trimmed.is_empty() || trimmed == "exit" {
                        break Ok(());
                    }
                    messages.push(Message {
                        role: "user",
                        content: trimmed.to_string(),
                    });
                    continue;
                }
                break Ok(());
            }

            let mut results = Vec::new();
            for action in &actions {
                match execute_action(action) {
                    Ok(msg) => {
                        println!("  [executed] {}", msg.lines().next().unwrap_or(&msg));
                        results.push(ToolResult::success(action, msg));
                    }
                    Err(e) => {
                        eprintln!("  [failed] {}: {}", action.action, e);
                        results.push(ToolResult::failure(action, e));
                    }
                }
            }

            messages.push(Message {
                role: "assistant",
                content: format!(
                    "{}\nActions: {:?}",
                    text,
                    actions.iter().map(|a| &a.action).collect::<Vec<_>>()
                ),
            });
            messages.push(Message {
                role: "user",
                content: format!(
                    "Tool results:\n{}",
                    results
                        .iter()
                        .map(ToolResult::as_prompt_text)
                        .collect::<Vec<_>>()
                        .join("\n\n")
                ),
            });

            if messages.len() > 22 {
                messages.drain(1..3);
            }
        }
    });

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_response_removes_action_without_eating_following_text() {
        let (text, actions) =
            parse_response("Read this [action:read-file:zen-client/src/main.rs] then continue.");

        assert_eq!(text, "Read this  then continue.");
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action, "read-file");
        assert_eq!(actions[0].payload, "zen-client/src/main.rs");
    }

    #[test]
    fn tool_result_prompt_includes_real_output() {
        let action = Action {
            action: "read-file".to_string(),
            payload: "foo.rs".to_string(),
        };

        let result = ToolResult::success(&action, "fn main() {}".to_string());
        let prompt = result.as_prompt_text();

        assert!(prompt.contains("tool: read-file"));
        assert!(prompt.contains("status: success"));
        assert!(prompt.contains("fn main() {}"));
    }
}
