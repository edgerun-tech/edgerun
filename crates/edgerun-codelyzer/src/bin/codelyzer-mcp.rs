//! MCP stdio server for edgerun-codelyzer.
//!
//! MCP is an external JSON-RPC protocol. Internal codelyzer wire, cache, and
//! bridge payloads remain rkyv-only.
//!
//! Usage:
//!   cargo run -p edgerun-codelyzer --bin codelyzer-mcp -- /home/ken/edgerun
//!
//! Tools cover graph inspection, Rust AST edits via edgerun-edit, conservative
//! non-Rust text edits gated by explicit user approval, and Xray viewport
//! control commands.

use std::{
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use edgerun_codelyzer::{
    analyzer::analyze_full,
    filesystem,
    mcp_permission,
    mcp_rust_ast,
    uir::Program,
    WIRE_PROTOCOL,
};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize)]
enum ViewportCommandType {
    FocusNode,
    SetCamera,
    Filter,
}

#[derive(Debug, Clone, Archive, RkyvSerialize, RkyvDeserialize)]
struct ViewportCommand {
    command_type: ViewportCommandType,
    ts: u128,
    id: Option<String>,
    zoom: Option<f64>,
    yaw: Option<f64>,
    pitch: Option<f64>,
    pan_x: Option<f64>,
    pan_y: Option<f64>,
    hide: Vec<String>,
    show: Vec<String>,
}

struct ServerState {
    root: PathBuf,
    graph: Option<CachedGraph>,
}

#[derive(Debug)]
struct CachedGraph {
    program: Program,
    files: Vec<filesystem::FileInfo>,
    fingerprint: String,
    elapsed_ms: u128,
}

#[derive(Debug, Serialize)]
struct ToolGraphSummary {
    root: String,
    file_count: usize,
    node_count: usize,
    edge_count: usize,
    total_bytes: u64,
    elapsed_ms: u128,
    fingerprint: String,
}

fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| ".".to_string());
    let root = canonicalize_existing_dir(&root).unwrap_or_else(|err| {
        eprintln!("codelyzer-mcp: {err}");
        std::process::exit(1);
    });

    let mut state = ServerState { root, graph: None };
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(request) => handle_request(&mut state, request),
            Err(err) => Some(error_response(Value::Null, -32700, &format!("parse error: {err}"))),
        };
        if let Some(response) = response {
            let _ = writeln!(stdout, "{}", response);
            let _ = stdout.flush();
        }
    }
}

fn handle_request(state: &mut ServerState, request: Value) -> Option<Value> {
    let id = request.get("id").cloned().unwrap_or(Value::Null);
    let method = request.get("method").and_then(Value::as_str).unwrap_or_default();

    match method {
        "initialize" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "serverInfo": { "name": "edgerun-codelyzer", "version": env!("CARGO_PKG_VERSION") },
                "capabilities": { "tools": {}, "resources": {} }
            }
        })),
        "notifications/initialized" => None,
        "tools/list" => Some(json!({ "jsonrpc": "2.0", "id": id, "result": { "tools": tool_definitions() } })),
        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
            let name = params.get("name").and_then(Value::as_str).unwrap_or_default();
            let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
            Some(call_tool_response(state, id, name, args))
        }
        "resources/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "resources": [
                    { "uri": "codelyzer://graph/summary", "name": "Codelyzer graph summary", "description": "Current repository graph summary", "mimeType": "application/json" },
                    { "uri": "codelyzer://graph/nodes", "name": "Codelyzer graph nodes", "description": "Function nodes extracted from the repository", "mimeType": "application/json" },
                    { "uri": "codelyzer://graph/edges", "name": "Codelyzer graph edges", "description": "Call edges extracted from the repository", "mimeType": "application/json" }
                ]
            }
        })),
        "resources/read" => {
            let uri = request.get("params").and_then(|p| p.get("uri")).and_then(Value::as_str).unwrap_or_default();
            Some(read_resource_response(state, id, uri))
        }
        _ => Some(error_response(id, -32601, &format!("unknown method: {method}"))),
    }
}

fn tool_definitions() -> Value {
    json!([
        { "name": "graph_summary", "description": "Analyze or read cached repository graph summary.", "inputSchema": { "type": "object", "properties": { "rescan": { "type": "boolean" } } } },
        { "name": "search_symbols", "description": "Search functions/symbols by name, file, language, or tag.", "inputSchema": { "type": "object", "properties": { "query": { "type": "string" }, "limit": { "type": "integer", "minimum": 1, "maximum": 200 } }, "required": ["query"] } },
        { "name": "get_node", "description": "Get one graph node/function by exact id.", "inputSchema": { "type": "object", "properties": { "id": { "type": "string" } }, "required": ["id"] } },
        { "name": "related_nodes", "description": "List callers/callees for a graph node/function.", "inputSchema": { "type": "object", "properties": { "id": { "type": "string" }, "limit": { "type": "integer", "minimum": 1, "maximum": 500 } }, "required": ["id"] } },
        { "name": "list_files", "description": "List indexed source files from the current repository.", "inputSchema": { "type": "object", "properties": { "query": { "type": "string" }, "limit": { "type": "integer", "minimum": 1, "maximum": 500 } } } },
        { "name": "read_file", "description": "Read a file under the repository root.", "inputSchema": { "type": "object", "properties": { "path": { "type": "string" }, "max_bytes": { "type": "integer", "minimum": 1, "maximum": 200000 } }, "required": ["path"] } },
        { "name": "grep_files", "description": "Search text in indexed files under the repository root.", "inputSchema": { "type": "object", "properties": { "query": { "type": "string" }, "limit": { "type": "integer", "minimum": 1, "maximum": 200 } }, "required": ["query"] } },
        { "name": "rust_ast", "description": "AST-safe Rust edits powered by edgerun-edit. Ops: list_file, find_fn, replace_fn_body, add_fn, remove_fn, add_use, add_derive, rename_type, new_file, remove_file, incoming_refs.", "inputSchema": { "type": "object", "properties": { "op": { "type": "string" }, "path": { "type": "string" }, "name": { "type": "string" }, "body": { "type": "string" }, "args": { "type": "string" }, "ret": { "type": "string" }, "use_path": { "type": "string" }, "derive": { "type": "string" }, "old": { "type": "string" }, "new": { "type": "string" }, "content": { "type": "string" } }, "required": ["op"] } },
        { "name": "write_file", "description": "Create or overwrite a non-Rust repo file. Requires text edit approval unless EDGERUN_MCP_ALLOW_TEXT_EDITS=1. Rust files must use rust_ast.", "inputSchema": { "type": "object", "properties": { "path": { "type": "string" }, "content": { "type": "string" }, "create": { "type": "boolean" }, "backup": { "type": "boolean" }, "expected_contains": { "type": "string" }, "approval_token": { "type": "string" }, "allow_text_edit_on_rust": { "type": "boolean" } }, "required": ["path", "content"] } },
        { "name": "replace_text", "description": "Replace exact text in a non-Rust repo file. Requires text edit approval unless EDGERUN_MCP_ALLOW_TEXT_EDITS=1. Rust files must use rust_ast unless allow_text_edit_on_rust=true.", "inputSchema": { "type": "object", "properties": { "path": { "type": "string" }, "old": { "type": "string" }, "new": { "type": "string" }, "all": { "type": "boolean" }, "backup": { "type": "boolean" }, "approval_token": { "type": "string" }, "allow_text_edit_on_rust": { "type": "boolean" } }, "required": ["path", "old", "new"] } },
        { "name": "replace_lines", "description": "Replace 1-indexed inclusive line range in a non-Rust repo file. Requires text edit approval unless EDGERUN_MCP_ALLOW_TEXT_EDITS=1. Rust files must use rust_ast unless allow_text_edit_on_rust=true.", "inputSchema": { "type": "object", "properties": { "path": { "type": "string" }, "start": { "type": "integer", "minimum": 1 }, "end": { "type": "integer", "minimum": 1 }, "replacement": { "type": "string" }, "backup": { "type": "boolean" }, "approval_token": { "type": "string" }, "allow_text_edit_on_rust": { "type": "boolean" } }, "required": ["path", "start", "end", "replacement"] } },
        { "name": "delete_file", "description": "Delete a repo file, creating a backup by default. Requires text edit approval unless EDGERUN_MCP_ALLOW_TEXT_EDITS=1.", "inputSchema": { "type": "object", "properties": { "path": { "type": "string" }, "backup": { "type": "boolean" }, "approval_token": { "type": "string" } }, "required": ["path"] } },
        { "name": "xray_focus_node", "description": "Command the browser Xray viewport to select/focus a node visually.", "inputSchema": { "type": "object", "properties": { "id": { "type": "string" }, "zoom": { "type": "number" } }, "required": ["id"] } },
        { "name": "xray_set_camera", "description": "Command the browser Xray viewport camera.", "inputSchema": { "type": "object", "properties": { "yaw": { "type": "number" }, "pitch": { "type": "number" }, "zoom": { "type": "number" }, "panX": { "type": "number" }, "panY": { "type": "number" } } } },
        { "name": "xray_filter", "description": "Command the browser Xray viewport to hide/show filter keys.", "inputSchema": { "type": "object", "properties": { "hide": { "type": "array", "items": { "type": "string" } }, "show": { "type": "array", "items": { "type": "string" } } } } },
        { "name": "xray_show_related", "description": "Command Xray to focus a node and visually show its closest related graph neighborhood.", "inputSchema": { "type": "object", "properties": { "id": { "type": "string" } }, "required": ["id"] } }
    ])
}

fn call_tool_response(state: &mut ServerState, id: Value, name: &str, args: Value) -> Value {
    let result = match name {
        "graph_summary" => tool_graph_summary(state, args),
        "search_symbols" => tool_search_symbols(state, args),
        "get_node" => tool_get_node(state, args),
        "related_nodes" => tool_related_nodes(state, args),
        "list_files" => tool_list_files(state, args),
        "read_file" => tool_read_file(state, args),
        "grep_files" => tool_grep_files(state, args),
        "rust_ast" => {
            let result = mcp_rust_ast::call(&state.root, args);
            if result.is_ok() { invalidate_graph(state); }
            result
        }
        "write_file" => tool_write_file(state, args),
        "replace_text" => tool_replace_text(state, args),
        "replace_lines" => tool_replace_lines(state, args),
        "delete_file" => tool_delete_file(state, args),
        "xray_focus_node" => tool_xray_command(args, "focus_node"),
        "xray_set_camera" => tool_xray_command(args, "set_camera"),
        "xray_filter" => tool_xray_command(args, "filter"),
        "xray_show_related" => tool_xray_command(args, "show_related"),
        _ => Err(format!("unknown tool: {name}")),
    };

    match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": { "content": [{ "type": "text", "text": pretty_mcp_json(&value) }], "structuredContent": value } }),
        Err(err) => json!({ "jsonrpc": "2.0", "id": id, "result": { "isError": true, "content": [{ "type": "text", "text": err }] } }),
    }
}

fn ensure_graph(state: &mut ServerState, rescan: bool) -> Result<&CachedGraph, String> {
    let files = filesystem::scan_dir(state.root.to_str().unwrap_or("."));
    let fingerprint = snapshot_fingerprint(&files);
    let should_scan = rescan || state.graph.as_ref().map(|g| g.fingerprint.as_str()) != Some(fingerprint.as_str());
    if should_scan {
        let started = Instant::now();
        let (result, _changes) = analyze_full(state.root.to_str().unwrap_or("."));
        state.graph = Some(CachedGraph { program: result.program, files, fingerprint, elapsed_ms: started.elapsed().as_millis() });
    }
    state.graph.as_ref().ok_or_else(|| "graph unavailable".to_string())
}

fn invalidate_graph(state: &mut ServerState) { state.graph = None; }

fn tool_graph_summary(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let rescan = args.get("rescan").and_then(Value::as_bool).unwrap_or(false);
    let root = state.root.display().to_string();
    let graph = ensure_graph(state, rescan)?;
    let total_bytes: u64 = graph.files.iter().map(|f| f.size).sum();
    Ok(json!(ToolGraphSummary { root, file_count: graph.files.len(), node_count: graph.program.functions.len(), edge_count: graph.program.edges.len(), total_bytes, elapsed_ms: graph.elapsed_ms, fingerprint: graph.fingerprint.clone() }))
}

fn tool_search_symbols(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let query = args.get("query").and_then(Value::as_str).unwrap_or_default().to_lowercase();
    let limit = limit_arg(&args, 50, 200);
    let graph = ensure_graph(state, false)?;
    let mut results = Vec::new();
    for func in graph.program.functions_ordered() {
        let haystack = format!("{} {} {} {}", func.id, func.name, func.file, func.language).to_lowercase();
        if haystack.contains(&query) {
            results.push(json!({ "id": func.id, "name": func.name, "file": func.file, "language": func.language, "is_static": func.is_static, "commit": func.last_modified_commit }));
            if results.len() >= limit { break; }
        }
    }
    Ok(json!({ "query": query, "count": results.len(), "results": results }))
}

fn tool_get_node(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let node_id = args.get("id").and_then(Value::as_str).ok_or("missing id")?;
    let graph = ensure_graph(state, false)?;
    let func = graph.program.get_by_legacy(node_id).ok_or_else(|| format!("node not found: {node_id}"))?;
    Ok(json!({ "id": func.id, "name": func.name, "file": func.file, "language": func.language, "is_static": func.is_static, "commit": func.last_modified_commit, "incoming": graph.program.get_callers(node_id).len(), "outgoing": graph.program.get_callees(node_id).len() }))
}

fn tool_related_nodes(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let node_id = args.get("id").and_then(Value::as_str).ok_or("missing id")?;
    let limit = limit_arg(&args, 100, 500);
    let graph = ensure_graph(state, false)?;
    let mut related = Vec::new();
    for edge in &graph.program.edges {
        if edge.caller == node_id || edge.callee == node_id {
            let other = if edge.caller == node_id { &edge.callee } else { &edge.caller };
            let direction = if edge.caller == node_id { "out" } else { "in" };
            let func = graph.program.get_by_legacy(other);
            related.push(json!({ "direction": direction, "edge_kind": edge.kind.label(), "confidence": edge.confidence, "id": other, "name": func.map(|f| f.name.clone()).unwrap_or_else(|| other.clone()), "file": func.map(|f| f.file.clone()), "language": func.map(|f| f.language.clone()) }));
            if related.len() >= limit { break; }
        }
    }
    Ok(json!({ "id": node_id, "count": related.len(), "related": related }))
}

fn tool_list_files(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let query = args.get("query").and_then(Value::as_str).unwrap_or_default().to_lowercase();
    let limit = limit_arg(&args, 100, 500);
    let graph = ensure_graph(state, false)?;
    let mut files = graph.files.clone();
    files.sort_by(|a, b| b.size.cmp(&a.size));
    let results: Vec<_> = files.into_iter().filter(|f| query.is_empty() || f.path.to_lowercase().contains(&query) || f.language.to_lowercase().contains(&query)).take(limit).map(|f| json!({ "path": f.path, "language": f.language, "size": f.size, "modified_ts": f.modified_ts })).collect();
    Ok(json!({ "query": query, "count": results.len(), "files": results }))
}

fn tool_read_file(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let path = args.get("path").and_then(Value::as_str).ok_or("missing path")?;
    let max_bytes = limit_arg_named(&args, "max_bytes", 50_000, 200_000);
    let full_path = resolve_under_root(&state.root, path)?;
    let mut content = fs::read_to_string(&full_path).map_err(|err| format!("read failed: {err}"))?;
    let truncated = content.len() > max_bytes;
    if truncated { content.truncate(max_bytes); }
    Ok(json!({ "path": path, "absolute_path": full_path, "truncated": truncated, "content": content }))
}

fn tool_grep_files(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let query = args.get("query").and_then(Value::as_str).ok_or("missing query")?;
    let needle = query.to_lowercase();
    let limit = limit_arg(&args, 80, 200);
    let root = state.root.clone();
    let graph = ensure_graph(state, false)?;
    let files = graph.files.clone();
    let mut results = Vec::new();
    for file in &files {
        let path = resolve_under_root(&root, &file.path)?;
        let Ok(content) = fs::read_to_string(&path) else { continue };
        for (line_no, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                results.push(json!({ "path": file.path, "line": line_no + 1, "text": line.trim() }));
                if results.len() >= limit { return Ok(json!({ "query": query, "count": results.len(), "matches": results })); }
            }
        }
    }
    Ok(json!({ "query": query, "count": results.len(), "matches": results }))
}

fn require_text_permission(state: &ServerState, operation: &str, path: &str, args: &Value) -> Result<(), String> {
    let token = args.get("approval_token").and_then(Value::as_str);
    mcp_permission::require_text_edit_permission(&state.root, operation, path, token).map(|grant| {
        let _ = grant.approved;
        let _ = grant.token;
        let _ = grant.request_path;
    })
}

fn forbid_text_rust(path: &str, args: &Value) -> Result<(), String> {
    let allow = args.get("allow_text_edit_on_rust").and_then(Value::as_bool).unwrap_or(false);
    if path.ends_with(".rs") && !allow {
        return Err("Refusing text edit on Rust file. Use rust_ast for AST-safe Rust changes, or pass allow_text_edit_on_rust=true explicitly.".to_string());
    }
    Ok(())
}

fn tool_write_file(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let path = args.get("path").and_then(Value::as_str).ok_or("missing path")?;
    forbid_text_rust(path, &args)?;
    require_text_permission(state, "write_file", path, &args)?;
    let content = args.get("content").and_then(Value::as_str).ok_or("missing content")?;
    let create = args.get("create").and_then(Value::as_bool).unwrap_or(false);
    let backup = args.get("backup").and_then(Value::as_bool).unwrap_or(true);
    let full_path = state.root.join(path.trim_start_matches('/'));
    if full_path.exists() {
        let full_path = resolve_under_root(&state.root, path)?;
        if let Some(expected) = args.get("expected_contains").and_then(Value::as_str) {
            let existing = fs::read_to_string(&full_path).map_err(|err| format!("read failed: {err}"))?;
            if !existing.contains(expected) { return Err("expected_contains was not found; refusing write".to_string()); }
        }
        if backup { backup_file(&full_path)?; }
        fs::write(&full_path, content).map_err(|err| format!("write failed: {err}"))?;
    } else {
        if !create { return Err("file does not exist; pass create=true".to_string()); }
        let parent = full_path.parent().ok_or("invalid path")?;
        fs::create_dir_all(parent).map_err(|err| format!("mkdir failed: {err}"))?;
        let canonical_parent = parent.canonicalize().map_err(|err| format!("invalid parent: {err}"))?;
        if !canonical_parent.starts_with(&state.root) { return Err("path escapes repository root".to_string()); }
        fs::write(&full_path, content).map_err(|err| format!("write failed: {err}"))?;
    }
    invalidate_graph(state);
    Ok(json!({ "path": path, "bytes": content.len(), "changed": true }))
}

fn tool_replace_text(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let path = args.get("path").and_then(Value::as_str).ok_or("missing path")?;
    forbid_text_rust(path, &args)?;
    require_text_permission(state, "replace_text", path, &args)?;
    let old = args.get("old").and_then(Value::as_str).ok_or("missing old")?;
    let new = args.get("new").and_then(Value::as_str).ok_or("missing new")?;
    let all = args.get("all").and_then(Value::as_bool).unwrap_or(false);
    let backup = args.get("backup").and_then(Value::as_bool).unwrap_or(true);
    let full_path = resolve_under_root(&state.root, path)?;
    let content = fs::read_to_string(&full_path).map_err(|err| format!("read failed: {err}"))?;
    let matches = content.matches(old).count();
    if matches == 0 { return Err("old text not found".to_string()); }
    if matches > 1 && !all { return Err(format!("old text matched {matches} times; pass all=true or use more context")); }
    let replaced = if all { content.replace(old, new) } else { content.replacen(old, new, 1) };
    if backup { backup_file(&full_path)?; }
    fs::write(&full_path, replaced).map_err(|err| format!("write failed: {err}"))?;
    invalidate_graph(state);
    Ok(json!({ "path": path, "matches": matches, "changed": true }))
}

fn tool_replace_lines(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let path = args.get("path").and_then(Value::as_str).ok_or("missing path")?;
    forbid_text_rust(path, &args)?;
    require_text_permission(state, "replace_lines", path, &args)?;
    let start = args.get("start").and_then(Value::as_u64).ok_or("missing start")? as usize;
    let end = args.get("end").and_then(Value::as_u64).ok_or("missing end")? as usize;
    let replacement = args.get("replacement").and_then(Value::as_str).ok_or("missing replacement")?;
    if start == 0 || end < start { return Err("invalid line range".to_string()); }
    let backup = args.get("backup").and_then(Value::as_bool).unwrap_or(true);
    let full_path = resolve_under_root(&state.root, path)?;
    let content = fs::read_to_string(&full_path).map_err(|err| format!("read failed: {err}"))?;
    let mut lines: Vec<&str> = content.lines().collect();
    if end > lines.len() { return Err(format!("end line {end} exceeds file line count {}", lines.len())); }
    let replacement_lines: Vec<&str> = replacement.lines().collect();
    lines.splice(start - 1..end, replacement_lines);
    let mut next = lines.join("\n");
    if content.ends_with('\n') || replacement.ends_with('\n') { next.push('\n'); }
    if backup { backup_file(&full_path)?; }
    fs::write(&full_path, next).map_err(|err| format!("write failed: {err}"))?;
    invalidate_graph(state);
    Ok(json!({ "path": path, "start": start, "end": end, "changed": true }))
}

fn tool_delete_file(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let path = args.get("path").and_then(Value::as_str).ok_or("missing path")?;
    require_text_permission(state, "delete_file", path, &args)?;
    let backup = args.get("backup").and_then(Value::as_bool).unwrap_or(true);
    let full_path = resolve_under_root(&state.root, path)?;
    if backup { backup_file(&full_path)?; }
    fs::remove_file(&full_path).map_err(|err| format!("delete failed: {err}"))?;
    invalidate_graph(state);
    Ok(json!({ "path": path, "deleted": true }))
}

fn tool_xray_command(args: Value, command_type: &str) -> Result<Value, String> {
    let command_type = match command_type {
        "focus_node" => ViewportCommandType::FocusNode,
        "set_camera" => ViewportCommandType::SetCamera,
        "filter" => ViewportCommandType::Filter,
        _ => return Err(format!("unknown viewport command type: {command_type}")),
    };
    let command = args.as_object().ok_or("arguments must be object")?;
    let string_list = |key: &str| -> Vec<String> {
        command
            .get(key)
            .and_then(Value::as_array)
            .map(|values| values.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default()
    };
    let command = ViewportCommand {
        command_type,
        ts: now_ms(),
        id: command.get("id").and_then(Value::as_str).map(str::to_string),
        zoom: command.get("zoom").and_then(Value::as_f64),
        yaw: command.get("yaw").and_then(Value::as_f64),
        pitch: command.get("pitch").and_then(Value::as_f64),
        pan_x: command.get("panX").and_then(Value::as_f64),
        pan_y: command.get("panY").and_then(Value::as_f64),
        hide: string_list("hide"),
        show: string_list("show"),
    };
    let dir = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("/tmp")).join("edgerun-codelyzer");
    fs::create_dir_all(&dir).map_err(|err| format!("mkdir failed: {err}"))?;
    let path = dir.join("viewport-command.rkyv");
    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&command)
        .map(|bytes| bytes.to_vec())
        .map_err(|err| err.to_string())?;
    fs::write(&path, bytes).map_err(|err| format!("write failed: {err}"))?;
    Ok(json!({ "queued": true, "protocol": WIRE_PROTOCOL, "path": path }))
}

fn read_resource_response(state: &mut ServerState, id: Value, uri: &str) -> Value {
    let result = match uri {
        "codelyzer://graph/summary" => tool_graph_summary(state, json!({})),
        "codelyzer://graph/nodes" => ensure_graph(state, false).map(|g| json!(g.program.functions_ordered().into_iter().map(|func| json!({ "id": func.id, "name": func.name, "file": func.file, "language": func.language, "is_static": func.is_static })).collect::<Vec<_>>())),
        "codelyzer://graph/edges" => ensure_graph(state, false).map(|g| json!(g.program.edges.iter().map(|edge| json!({ "source": edge.caller, "target": edge.callee, "kind": edge.kind.label(), "confidence": edge.confidence })).collect::<Vec<_>>())),
        _ => Err(format!("unknown resource: {uri}")),
    };
    match result {
        Ok(value) => json!({ "jsonrpc": "2.0", "id": id, "result": { "contents": [{ "uri": uri, "mimeType": "application/json", "text": pretty_mcp_json(&value) }] } }),
        Err(err) => error_response(id, -32603, &err),
    }
}

fn canonicalize_existing_dir(path: &str) -> Result<PathBuf, String> {
    let canonical = PathBuf::from(path).canonicalize().map_err(|err| format!("invalid root: {err}"))?;
    if !canonical.is_dir() { return Err(format!("root is not a directory: {}", canonical.display())); }
    Ok(canonical)
}

fn resolve_under_root(root: &Path, path: &str) -> Result<PathBuf, String> {
    let joined = root.join(path.trim_start_matches('/'));
    let canonical = joined.canonicalize().map_err(|err| format!("invalid path: {err}"))?;
    if !canonical.starts_with(root) { return Err("path escapes repository root".to_string()); }
    Ok(canonical)
}

fn backup_file(path: &Path) -> Result<PathBuf, String> {
    let backup = path.with_extension(format!("{}.bak.{}", path.extension().and_then(|e| e.to_str()).unwrap_or("file"), now_ms()));
    fs::copy(path, &backup).map_err(|err| format!("backup failed: {err}"))?;
    Ok(backup)
}

fn snapshot_fingerprint(files: &[filesystem::FileInfo]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for file in files {
        hash ^= file.hash;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= file.size;
        hash = hash.wrapping_mul(0x100000001b3);
        hash ^= file.modified_ts;
        hash = hash.wrapping_mul(0x100000001b3);
        for byte in file.path.as_bytes() { hash ^= *byte as u64; hash = hash.wrapping_mul(0x100000001b3); }
    }
    format!("{:x}:{}", hash, files.len())
}

fn now_ms() -> u128 { SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() }
fn limit_arg(args: &Value, default: usize, max: usize) -> usize { limit_arg_named(args, "limit", default, max) }
fn limit_arg_named(args: &Value, name: &str, default: usize, max: usize) -> usize {
    args.get(name).and_then(Value::as_u64).map(|n| n as usize).unwrap_or(default).clamp(1, max)
}
fn pretty_mcp_json(value: &Value) -> String { serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string()) }
fn error_response(id: Value, code: i64, message: &str) -> Value { json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } }) }
