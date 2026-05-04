//! MCP stdio server for edgerun-codelyzer.
//!
//! Usage:
//!   cargo run -p edgerun-codelyzer --bin codelyzer-mcp -- /home/ken/edgerun
//!
//! This intentionally uses only stdio + JSON-RPC so it stays dependency-light.

use std::{
    collections::HashMap,
    fs,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use edgerun_codelyzer::{analyzer::analyze_full, filesystem, uir::Program};
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Debug)]
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
                "capabilities": {
                    "tools": {},
                    "resources": {}
                }
            }
        })),
        "notifications/initialized" => None,
        "tools/list" => Some(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": { "tools": tool_definitions() }
        })),
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
                    {
                        "uri": "codelyzer://graph/summary",
                        "name": "Codelyzer graph summary",
                        "description": "Current repository graph summary",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "codelyzer://graph/nodes",
                        "name": "Codelyzer graph nodes",
                        "description": "Function nodes extracted from the repository",
                        "mimeType": "application/json"
                    },
                    {
                        "uri": "codelyzer://graph/edges",
                        "name": "Codelyzer graph edges",
                        "description": "Call edges extracted from the repository",
                        "mimeType": "application/json"
                    }
                ]
            }
        })),
        "resources/read" => {
            let uri = request
                .get("params")
                .and_then(|p| p.get("uri"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            Some(read_resource_response(state, id, uri))
        }
        _ => Some(error_response(id, -32601, &format!("unknown method: {method}"))),
    }
}

fn tool_definitions() -> Value {
    json!([
        {
            "name": "graph_summary",
            "description": "Analyze or read cached repository graph summary.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "rescan": { "type": "boolean", "description": "Force a fresh scan." }
                }
            }
        },
        {
            "name": "search_symbols",
            "description": "Search functions/symbols by name, file, language, or tag.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 200 }
                },
                "required": ["query"]
            }
        },
        {
            "name": "get_node",
            "description": "Get one graph node/function by exact id.",
            "inputSchema": {
                "type": "object",
                "properties": { "id": { "type": "string" } },
                "required": ["id"]
            }
        },
        {
            "name": "related_nodes",
            "description": "List callers/callees for a graph node/function.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 500 }
                },
                "required": ["id"]
            }
        },
        {
            "name": "list_files",
            "description": "List indexed source files from the current repository.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 500 }
                }
            }
        },
        {
            "name": "read_file",
            "description": "Read a file under the repository root.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "max_bytes": { "type": "integer", "minimum": 1, "maximum": 200000 }
                },
                "required": ["path"]
            }
        },
        {
            "name": "grep_files",
            "description": "Search text in indexed files under the repository root.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "limit": { "type": "integer", "minimum": 1, "maximum": 200 }
                },
                "required": ["query"]
            }
        }
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
        _ => Err(format!("unknown tool: {name}")),
    };

    match result {
        Ok(value) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "content": [{ "type": "text", "text": pretty_json(&value) }],
                "structuredContent": value
            }
        }),
        Err(err) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "isError": true,
                "content": [{ "type": "text", "text": err }]
            }
        }),
    }
}

fn ensure_graph(state: &mut ServerState, rescan: bool) -> Result<&CachedGraph, String> {
    let files = filesystem::scan_dir(state.root.to_str().unwrap_or("."));
    let fingerprint = snapshot_fingerprint(&files);
    let should_scan = rescan || state.graph.as_ref().map(|g| g.fingerprint.as_str()) != Some(fingerprint.as_str());

    if should_scan {
        let started = Instant::now();
        let (result, _changes) = analyze_full(state.root.to_str().unwrap_or("."));
        state.graph = Some(CachedGraph {
            program: result.program,
            files,
            fingerprint,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    state.graph.as_ref().ok_or_else(|| "graph unavailable".to_string())
}

fn tool_graph_summary(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let rescan = args.get("rescan").and_then(Value::as_bool).unwrap_or(false);
    let root = state.root.display().to_string();
    let graph = ensure_graph(state, rescan)?;
    let total_bytes: u64 = graph.files.iter().map(|f| f.size).sum();
    Ok(json!(ToolGraphSummary {
        root,
        file_count: graph.files.len(),
        node_count: graph.program.functions.len(),
        edge_count: graph.program.edges.len(),
        total_bytes,
        elapsed_ms: graph.elapsed_ms,
        fingerprint: graph.fingerprint.clone(),
    }))
}

fn tool_search_symbols(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let query = args.get("query").and_then(Value::as_str).unwrap_or_default().to_lowercase();
    let limit = limit_arg(&args, 50, 200);
    let graph = ensure_graph(state, false)?;
    let mut results = Vec::new();

    for func in graph.program.functions_ordered() {
        let haystack = format!("{} {} {} {}", func.id, func.name, func.file, func.language).to_lowercase();
        if haystack.contains(&query) {
            results.push(json!({
                "id": func.id,
                "name": func.name,
                "file": func.file,
                "language": func.language,
                "is_static": func.is_static,
                "commit": func.last_modified_commit,
            }));
            if results.len() >= limit { break; }
        }
    }

    Ok(json!({ "query": query, "count": results.len(), "results": results }))
}

fn tool_get_node(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let node_id = args.get("id").and_then(Value::as_str).ok_or("missing id")?;
    let graph = ensure_graph(state, false)?;
    let func = graph.program.get_by_legacy(node_id).ok_or_else(|| format!("node not found: {node_id}"))?;
    let incoming = graph.program.get_callers(node_id).len();
    let outgoing = graph.program.get_callees(node_id).len();
    Ok(json!({
        "id": func.id,
        "name": func.name,
        "file": func.file,
        "language": func.language,
        "is_static": func.is_static,
        "commit": func.last_modified_commit,
        "incoming": incoming,
        "outgoing": outgoing,
    }))
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
            related.push(json!({
                "direction": direction,
                "edge_kind": edge.kind.label(),
                "confidence": edge.confidence,
                "id": other,
                "name": func.map(|f| f.name.clone()).unwrap_or_else(|| other.clone()),
                "file": func.map(|f| f.file.clone()),
                "language": func.map(|f| f.language.clone()),
            }));
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

    let results: Vec<_> = files
        .into_iter()
        .filter(|f| query.is_empty() || f.path.to_lowercase().contains(&query) || f.language.to_lowercase().contains(&query))
        .take(limit)
        .map(|f| json!({ "path": f.path, "language": f.language, "size": f.size, "modified_ts": f.modified_ts }))
        .collect();

    Ok(json!({ "query": query, "count": results.len(), "files": results }))
}

fn tool_read_file(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let path = args.get("path").and_then(Value::as_str).ok_or("missing path")?;
    let max_bytes = limit_arg_named(&args, "max_bytes", 50_000, 200_000);
    let full_path = resolve_under_root(&state.root, path)?;
    let mut content = fs::read_to_string(&full_path).map_err(|err| format!("read failed: {err}"))?;
    let truncated = content.len() > max_bytes;
    if truncated {
        content.truncate(max_bytes);
    }
    Ok(json!({ "path": path, "absolute_path": full_path, "truncated": truncated, "content": content }))
}

fn tool_grep_files(state: &mut ServerState, args: Value) -> Result<Value, String> {
    let query = args.get("query").and_then(Value::as_str).ok_or("missing query")?;
    let needle = query.to_lowercase();
    let limit = limit_arg(&args, 80, 200);
    let graph = ensure_graph(state, false)?;
    let mut results = Vec::new();

    for file in &graph.files {
        let path = resolve_under_root(&state.root, &file.path)?;
        let Ok(content) = fs::read_to_string(&path) else { continue };
        for (line_no, line) in content.lines().enumerate() {
            if line.to_lowercase().contains(&needle) {
                results.push(json!({
                    "path": file.path,
                    "line": line_no + 1,
                    "text": line.trim(),
                }));
                if results.len() >= limit {
                    return Ok(json!({ "query": query, "count": results.len(), "matches": results }));
                }
            }
        }
    }

    Ok(json!({ "query": query, "count": results.len(), "matches": results }))
}

fn read_resource_response(state: &mut ServerState, id: Value, uri: &str) -> Value {
    let result = match uri {
        "codelyzer://graph/summary" => tool_graph_summary(state, json!({})),
        "codelyzer://graph/nodes" => {
            let graph = match ensure_graph(state, false) {
                Ok(graph) => graph,
                Err(err) => return error_response(id, -32603, &err),
            };
            Ok(json!(graph.program.functions_ordered().into_iter().map(|func| json!({
                "id": func.id,
                "name": func.name,
                "file": func.file,
                "language": func.language,
                "is_static": func.is_static,
            })).collect::<Vec<_>>()))
        }
        "codelyzer://graph/edges" => {
            let graph = match ensure_graph(state, false) {
                Ok(graph) => graph,
                Err(err) => return error_response(id, -32603, &err),
            };
            Ok(json!(graph.program.edges.iter().map(|edge| json!({
                "source": edge.caller,
                "target": edge.callee,
                "kind": edge.kind.label(),
                "confidence": edge.confidence,
            })).collect::<Vec<_>>()))
        }
        _ => Err(format!("unknown resource: {uri}")),
    };

    match result {
        Ok(value) => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "contents": [{
                    "uri": uri,
                    "mimeType": "application/json",
                    "text": pretty_json(&value)
                }]
            }
        }),
        Err(err) => error_response(id, -32603, &err),
    }
}

fn canonicalize_existing_dir(path: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(path);
    let canonical = path.canonicalize().map_err(|err| format!("invalid root: {err}"))?;
    if !canonical.is_dir() {
        return Err(format!("root is not a directory: {}", canonical.display()));
    }
    Ok(canonical)
}

fn resolve_under_root(root: &Path, path: &str) -> Result<PathBuf, String> {
    let joined = root.join(path.trim_start_matches('/'));
    let canonical = joined.canonicalize().map_err(|err| format!("invalid path: {err}"))?;
    if !canonical.starts_with(root) {
        return Err("path escapes repository root".to_string());
    }
    Ok(canonical)
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
        for byte in file.path.as_bytes() {
            hash ^= *byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
    }
    format!("{:x}:{}", hash, files.len())
}

fn limit_arg(args: &Value, default: usize, max: usize) -> usize {
    limit_arg_named(args, "limit", default, max)
}

fn limit_arg_named(args: &Value, name: &str, default: usize, max: usize) -> usize {
    args.get(name)
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(default)
        .clamp(1, max)
}

fn pretty_json(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}
