//! Minimal codelyzer → Xray bridge.
//!
//! Serves repository analysis as JSON that the frontend Xray graph can consume.
//!
//! Usage:
//!   cargo run -p edgerun-codelyzer --bin xray-server -- /path/to/repo
//!   cargo run -p edgerun-codelyzer --bin xray-server -- /path/to/repo --listen 127.0.0.1:13337
//!
//! Endpoints:
//!   GET /graph
//!   GET /graph?path=/path/to/repo
//!   GET /health

use std::{
    collections::HashMap,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::Path,
    time::Instant,
};

use edgerun_codelyzer::analyzer::analyze_full;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct XrayGraphNode {
    id: String,
    name: String,
    file: String,
    language: String,
    kind: String,
    is_static: bool,
    connections: u32,
    tags: Vec<String>,
    commit: Option<String>,
}

#[derive(Debug, Serialize)]
struct XrayGraphEdge {
    source: String,
    target: String,
    kind: String,
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct XrayGraphData {
    nodes: Vec<XrayGraphNode>,
    edges: Vec<XrayGraphEdge>,
    total_bytes: u64,
    node_count: u32,
    edge_count: u32,
    elapsed_ms: u64,
    source: String,
}

#[derive(Debug, Serialize)]
struct GraphUpdateEnvelope<'a> {
    r#type: &'a str,
    data: XrayGraphData,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut root: Option<String> = None;
    let mut listen = "127.0.0.1:13337".to_string();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--listen" => {
                i += 1;
                if let Some(value) = args.get(i) {
                    listen = value.clone();
                }
            }
            "--help" | "-h" => {
                print_usage(&args[0]);
                return;
            }
            value if root.is_none() => root = Some(value.to_string()),
            _ => {}
        }
        i += 1;
    }

    let root = root.unwrap_or_else(|| ".".to_string());
    if !Path::new(&root).is_dir() {
        eprintln!("error: '{}' is not a directory", root);
        std::process::exit(1);
    }

    let listener = TcpListener::bind(&listen).expect("failed to bind codelyzer xray server");
    println!("[xray-server] listening on http://{}", listen);
    println!("[xray-server] default repo: {}", root);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream, &root),
            Err(err) => eprintln!("[xray-server] connection error: {err}"),
        }
    }
}

fn print_usage(program: &str) {
    eprintln!("Usage: {program} <repo-path> [--listen 127.0.0.1:13337]");
}

fn handle_client(mut stream: TcpStream, default_root: &str) {
    let mut buf = [0u8; 8192];
    let n = match stream.read(&mut buf) {
        Ok(n) => n,
        Err(err) => {
            eprintln!("[xray-server] read error: {err}");
            return;
        }
    };

    let request = String::from_utf8_lossy(&buf[..n]);
    let first_line = request.lines().next().unwrap_or_default();
    let path = first_line.split_whitespace().nth(1).unwrap_or("/");

    if path == "/health" {
        respond_json(&mut stream, 200, r#"{"ok":true}"#);
        return;
    }

    if !path.starts_with("/graph") {
        respond_json(&mut stream, 404, r#"{"error":"not found"}"#);
        return;
    }

    let root = query_param(path, "path").unwrap_or_else(|| default_root.to_string());
    if !Path::new(&root).is_dir() {
        respond_json(
            &mut stream,
            400,
            &serde_json::json!({ "error": "path is not a directory", "path": root }).to_string(),
        );
        return;
    }

    match analyze_repo_for_xray(&root) {
        Ok(data) => {
            let envelope = GraphUpdateEnvelope { r#type: "graph_update", data };
            match serde_json::to_string(&envelope) {
                Ok(body) => respond_json(&mut stream, 200, &body),
                Err(err) => respond_json(
                    &mut stream,
                    500,
                    &serde_json::json!({ "error": err.to_string() }).to_string(),
                ),
            }
        }
        Err(err) => respond_json(
            &mut stream,
            500,
            &serde_json::json!({ "error": err }).to_string(),
        ),
    }
}

fn analyze_repo_for_xray(root: &str) -> Result<XrayGraphData, String> {
    let started = Instant::now();
    let (result, _changes) = analyze_full(root);
    let program = result.program;

    let mut connectivity: HashMap<String, u32> = HashMap::new();
    for edge in &program.edges {
        *connectivity.entry(edge.caller.clone()).or_default() += 1;
        *connectivity.entry(edge.callee.clone()).or_default() += 1;
    }

    let mut nodes: Vec<XrayGraphNode> = program
        .functions_ordered()
        .into_iter()
        .map(|func| XrayGraphNode {
            id: func.id.clone(),
            name: func.name.clone(),
            file: func.file.clone(),
            language: func.language.clone(),
            kind: "function".to_string(),
            is_static: func.is_static,
            connections: connectivity.get(&func.id).copied().unwrap_or(0),
            tags: tags_for_function(func),
            commit: func.last_modified_commit.clone(),
        })
        .collect();

    let mut edges: Vec<XrayGraphEdge> = program
        .edges
        .iter()
        .map(|edge| XrayGraphEdge {
            source: edge.caller.clone(),
            target: edge.callee.clone(),
            kind: edge.kind.label().to_string(),
            tags: vec![edge.source.clone(), format!("confidence:{:.1}", edge.confidence)],
        })
        .collect();

    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    edges.sort_by(|a, b| a.source.cmp(&b.source).then(a.target.cmp(&b.target)));

    let total_bytes = result.file_snapshot.iter().map(|file| file.size).sum();

    Ok(XrayGraphData {
        node_count: nodes.len() as u32,
        edge_count: edges.len() as u32,
        nodes,
        edges,
        total_bytes,
        elapsed_ms: started.elapsed().as_millis() as u64,
        source: root.to_string(),
    })
}

fn tags_for_function(func: &edgerun_codelyzer::uir::Function) -> Vec<String> {
    let mut tags = vec![func.language.clone()];
    if func.is_static {
        tags.push("static".to_string());
    }
    if let Some(commit) = &func.last_modified_commit {
        tags.push(format!("commit:{commit}"));
    }
    if func.file.contains("components") || func.file.contains("/ui/") {
        tags.push("ui".to_string());
    }
    if func.file.contains("runtime") {
        tags.push("runtime".to_string());
    }
    if func.file.contains("storage") || func.file.contains("store") {
        tags.push("storage".to_string());
    }
    tags
}

fn respond_json(stream: &mut TcpStream, status: u16, body: &str) {
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };

    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: content-type\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = stream.write_all(response.as_bytes());
}

fn query_param(path: &str, key: &str) -> Option<String> {
    let query = path.split_once('?')?.1;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k == key {
            return Some(percent_decode(v));
        }
    }
    None
}

fn percent_decode(value: &str) -> String {
    let mut bytes = Vec::with_capacity(value.len());
    let mut chars = value.as_bytes().iter().copied();
    while let Some(byte) = chars.next() {
        if byte == b'%' {
            let hi = chars.next().unwrap_or(b'0');
            let lo = chars.next().unwrap_or(b'0');
            if let Ok(hex) = std::str::from_utf8(&[hi, lo]) {
                if let Ok(decoded) = u8::from_str_radix(hex, 16) {
                    bytes.push(decoded);
                    continue;
                }
            }
        }
        if byte == b'+' {
            bytes.push(b' ');
        } else {
            bytes.push(byte);
        }
    }
    String::from_utf8_lossy(&bytes).to_string()
}
