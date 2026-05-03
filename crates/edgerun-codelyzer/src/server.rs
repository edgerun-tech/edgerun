#![allow(clippy::unwrap_used)]

use std::{
    collections::{HashMap, HashSet},
    io::{self, Read, Write},
    net::{TcpListener, TcpStream},
    os::unix::net::{UnixListener, UnixStream},
    sync::{Arc, Mutex},
    thread,
};

use sha1::{Digest, Sha1};
use prost::Message;

pub mod generated {
    pub mod codeanalyzer {
        include!("generated/codeanalyzer.rs");
    }
}

pub use generated::codeanalyzer::{
    index_state, ws_message, ws_response, AddRepoResponse, AnalyzeResponse, ApplyEditResponse,
    ChatRequest, ChatResponse, DirListing, DiscoverReposResponse, ErrorResponse, FileData,
    FileEntry, FilesResponse, GetConfigResponse, GetDiagnosticsResponse, GetFileResponse,
    GetFilesResponse, GetIndexStateResponse, GetReposResponse, GetViewResponse,
    GetWsConnectionsResponse, GraphData, GraphEdge, GraphNode, GraphUpdate, IndexState, QueryMatch,
    QueryResult, RegistryConfig, RemoveRepoResponse, RepoInfo, RepoList, StatsResponse,
    StatusResponse, SwitchRepoResponse, TagGroup, ToolCallInfo, ToolUseResponse, ViewData,
    ViewEdge, ViewNode, WsMessage, WsRequest, WsResponse, XrefResponse,
};

use crate::{analyzer, diagnostics, qwen, repo_registry::RepositoryRegistry};

fn derive_tags(file: &str, name: &str, language: &str, is_static: bool) -> Vec<String> {
    let mut tags: HashSet<String> = HashSet::new();

    tags.insert(format!("lang:{}", language));

    if is_static {
        tags.insert("linkage:static".to_string());
    } else {
        tags.insert("linkage:module".to_string());
    }

    if let Some(ext) = file.rsplit('.').next() {
        if !ext.is_empty() && ext.len() < 10 {
            tags.insert(format!("ext:{}", ext));
        }
    }

    let path_parts: Vec<&str> = file.split('/').collect();
    for part in path_parts.iter().take(4) {
        if !part.is_empty() && !part.contains('.') {
            tags.insert(format!("path:{}", part));
            match *part {
                "drivers" | "driver" => {
                    tags.insert("subsystem:drivers".to_string());
                }
                "kernel" | "net" | "fs" | "arch" => {
                    tags.insert(format!("subsystem:{}", part));
                }
                "src" | "lib" | "bin" => {
                    tags.insert(format!("dir:{}", part));
                }
                _ => {}
            }
        }
    }

    if let Some(filename) = file.split('/').next_back() {
        if let Some(name_without_ext) = filename.rsplit('.').next() {
            if !name_without_ext.is_empty() {
                tags.insert(format!("file:{}", name_without_ext));
            }
        }
    }

    if !name.is_empty() {
        if name.contains("handler") || name.contains("_h_") {
            tags.insert("pattern:handler".to_string());
        }
        if name.starts_with("init_") || name.contains("_init") {
            tags.insert("pattern:init".to_string());
        }
        if name.starts_with("test_") || name.ends_with("_test") || name.contains("_tests") {
            tags.insert("pattern:test".to_string());
        }
        if name.contains("ioctl") || name.contains("syscall") || name.contains("_io_") {
            tags.insert("pattern:io".to_string());
        }
        if name.contains("alloc")
            || name.contains("free")
            || name.contains("buf")
            || name.contains("mem")
        {
            tags.insert("pattern:memory".to_string());
        }
    }

    let mut tags: Vec<String> = tags.into_iter().collect();
    tags.sort();
    tags
}

pub static REGISTRY: Mutex<Option<RepositoryRegistry>> = Mutex::new(None);

pub static APP_STATE: Mutex<Option<AppState>> = Mutex::new(None);

static SESSIONS: Mutex<Option<std::collections::HashSet<String>>> = Mutex::new(None);

const MAX_FILE_SIZE: usize = 10 * 1024 * 1024;
const MAX_EDIT_SIZE: usize = 5 * 1024 * 1024;

fn is_new_session(session_id: &str) -> bool {
    let mut sessions = SESSIONS.lock().unwrap();
    let set = sessions.get_or_insert_with(std::collections::HashSet::new);
    let new_session = !set.contains(session_id);
    if new_session {
        set.insert(session_id.to_string());
    }
    new_session
}

pub struct AppState {
    pub graph: GraphData,
    pub graph_proto: Vec<u8>,
    pub root_dir: String,
    pub diagnostics_report: String,
}

impl AppState {
    pub fn graph(&self) -> &GraphData {
        &self.graph
    }
}

impl AppState {
    pub fn new(root_dir: &str) -> Self {
        println!("[server] Analyzing {}...", root_dir);
        let (result, _changes) = analyzer::analyze_full(root_dir);
        let nc = result.program.functions.len();
        let ec = result.program.edges.len();
        println!("[server] {} functions, {} edges", nc, ec);

        let nodes: Vec<GraphNode> = result
            .program
            .functions
            .iter()
            .map(|(id, func)| GraphNode {
                id: id.to_legacy(),
                name: func.name.clone(),
                file: func.file.clone(),
                language: func.language.clone(),
                is_static: func.is_static,
                connections: 0,
                tags: derive_tags(&func.file, &func.name, &func.language, func.is_static),
                commit: func.last_modified_commit.clone(),
            })
            .collect();

        let edges: Vec<GraphEdge> = result
            .program
            .edges
            .iter()
            .map(|e| GraphEdge {
                source: e.caller.clone(),
                target: e.callee.clone(),
                kind: e.kind.label().to_string(),
            })
            .collect();

        let tag_groups = vec![
            TagGroup {
                name: "Drivers".into(),
                tags: vec!["subsystem:drivers".into()],
                color: "#4CAF50".into(),
            },
            TagGroup {
                name: "Kernel".into(),
                tags: vec![
                    "subsystem:kernel".into(),
                    "subsystem:net".into(),
                    "subsystem:fs".into(),
                ],
                color: "#FF9800".into(),
            },
            TagGroup {
                name: "Memory".into(),
                tags: vec!["pattern:memory".into(), "pattern:alloc".into(), "pattern:buf".into()],
                color: "#2196F3".into(),
            },
            TagGroup {
                name: "IO".into(),
                tags: vec!["pattern:io".into(), "pattern:handler".into()],
                color: "#9C27B0".into(),
            },
            TagGroup {
                name: "Init".into(),
                tags: vec!["pattern:init".into()],
                color: "#E91E63".into(),
            },
            TagGroup {
                name: "Tests".into(),
                tags: vec!["pattern:test".into()],
                color: "#00BCD4".into(),
            },
            TagGroup {
                name: "Rust".into(),
                tags: vec!["lang:rust".into()],
                color: "#DEA584".into(),
            },
            TagGroup { name: "C".into(), tags: vec!["lang:c".into()], color: "#555555".into() },
            TagGroup {
                name: "Static".into(),
                tags: vec!["linkage:static".into()],
                color: "#795548".into(),
            },
        ];

        let graph = GraphData {
            nodes,
            edges,
            tag_groups,
            total_bytes: 0,
            node_count: nc as u32,
            edge_count: ec as u32,
        };

        let mut graph_proto = Vec::new();
        prost::Message::encode(&graph, &mut graph_proto).expect("graph serialization failed");

        println!("[server] Graph: {} bytes (proto)", graph_proto.len());

        let diagnostics_graph = diagnostics::GraphData {
            nodes: graph
                .nodes
                .iter()
                .map(|n| diagnostics::GraphNode { id: n.id.clone(), name: n.name.clone() })
                .collect(),
            edges: graph
                .edges
                .iter()
                .map(|e| diagnostics::GraphEdge {
                    source: e.source.clone(),
                    target: e.target.clone(),
                })
                .collect(),
        };

        println!("[server] Scanning for available linters/tools...");
        let tools = diagnostics::scan_tools(root_dir);
        println!("[server] Found tools: {}", tools.join(", "));
        let diagnostics_report = diagnostics::get_diagnostics_summary(root_dir, &diagnostics_graph);

        Self { graph, graph_proto, root_dir: root_dir.to_string(), diagnostics_report }
    }
}

#[allow(dead_code)]
pub fn get_state() -> &'static Mutex<Option<AppState>> {
    &APP_STATE
}

type WsBroadcaster = Arc<Mutex<Vec<std::sync::mpsc::Sender<Vec<u8>>>>>;
static WS_BROADCASTER: Mutex<Option<WsBroadcaster>> = Mutex::new(None);

fn ws_broadcaster() -> Arc<Mutex<Vec<std::sync::mpsc::Sender<Vec<u8>>>>> {
    let mut g = WS_BROADCASTER.lock().unwrap();
    if g.is_none() {
        *g = Some(Arc::new(Mutex::new(Vec::new())));
    }
    g.as_ref().unwrap().clone()
}

fn broadcast_proto(data: &[u8]) {
    let txs = ws_broadcaster();
    let mut senders = txs.lock().unwrap();
    senders.retain(|tx| tx.send(data.to_vec()).is_ok());
}

fn broadcast_json(json: &str) {
    let txs = ws_broadcaster();
    let mut senders = txs.lock().unwrap();
    let msg = WsMessage { msg: Some(ws_message::Msg::RawJson(json.as_bytes().to_vec())) };
    let mut buf = Vec::new();
    if prost::Message::encode(&msg, &mut buf).is_ok() {
        senders.retain(|tx| tx.send(buf.clone()).is_ok());
    }
}

fn ws_write(stream: &mut TcpStream, data: &[u8]) -> io::Result<()> {
    let n = data.len();
    let mut hdr = Vec::with_capacity(14);
    hdr.push(0x82);
    if n < 126 {
        hdr.push(n as u8);
    } else if n < 65536 {
        hdr.push(126);
        hdr.push(((n >> 8) & 0xFF) as u8);
        hdr.push((n & 0xFF) as u8);
    } else {
        hdr.push(127);
        for i in (0..8).rev() {
            hdr.push(((n >> (i * 8)) & 0xFF) as u8);
        }
    }
    stream.write_all(&hdr)?;
    stream.write_all(data)?;
    stream.flush()
}

fn ws_write_str(stream: &mut TcpStream, text: &str) -> io::Result<()> {
    ws_write(stream, text.as_bytes())
}

fn ws_read_frame(r: &mut impl Read) -> io::Result<Option<(Vec<u8>, u8)>> {
    let mut hdr = [0u8; 2];
    r.read_exact(&mut hdr)?;
    let opcode = hdr[0] & 0x0F;
    let masked = (hdr[1] >> 7) & 1 == 1;
    let mut plen = (hdr[1] & 0x7F) as usize;
    if plen == 126 {
        let mut e = [0u8; 2];
        r.read_exact(&mut e)?;
        plen = u16::from_be_bytes(e) as usize;
    } else if plen == 127 {
        let mut e = [0u8; 8];
        r.read_exact(&mut e)?;
        plen = u64::from_be_bytes(e) as usize;
    }
    let mut mask = [0u8; 4];
    if masked {
        r.read_exact(&mut mask)?;
    }
    let mut data = vec![0u8; plen];
    r.read_exact(&mut data)?;
    if masked {
        for i in 0..plen {
            data[i] ^= mask[i % 4];
        }
    }
    if opcode == 8 {
        return Ok(None);
    }
    Ok(Some((data, opcode)))
}

fn ws_handshake_response(key: &str) -> String {
    const MAGIC: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let mut h = Sha1::new();
    h.update(key.as_bytes());
    h.update(MAGIC.as_bytes());
    let accept = base64_encode(&h.finalize());
    format!(
        "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: \
         Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
        accept
    )
}

fn write_proto_binary(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> io::Result<()> {
    let status_str = match status {
        200 => "200 OK",
        400 => "400 Bad Request",
        404 => "404 Not Found",
        413 => "413 Payload Too Large",
        500 => "500 Internal Server Error",
        _ => "500 Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status_str}\r\nContent-Type: {content_type}\r\nContent-Length: \
         {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}

fn write_text(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
) -> io::Result<()> {
    write_proto_binary(stream, status, content_type, body)
}

fn base64_encode(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let t = (b0 << 16) | (b1 << 8) | b2;
        out.push(T[((t >> 18) & 0x3F) as usize] as char);
        out.push(T[((t >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 { T[((t >> 6) & 0x3F) as usize] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[(t & 0x3F) as usize] as char } else { '=' });
    }
    out
}

fn read_line(r: &mut impl Read) -> io::Result<String> {
    let mut line = Vec::new();
    loop {
        let mut byte = [0u8];
        r.read_exact(&mut byte)?;
        if byte[0] == b'\n' {
            break;
        }
        if byte[0] != b'\r' {
            line.push(byte[0]);
        }
    }
    Ok(String::from_utf8_lossy(&line).to_string())
}

fn serve_http(
    mut stream: TcpStream,
    viewer_dir: &str,
    method: &str,
    path: &str,
    query: &str,
    content_length: usize,
) -> io::Result<()> {
    let filepath = if path == "/" || path.is_empty() {
        format!("{}/index.html", viewer_dir)
    } else if path.starts_with("/api/") {
        return handle_api(stream, viewer_dir, method, path, query, content_length);
    } else {
        format!("{}{}", viewer_dir, path)
    };
    match std::fs::read(&filepath) {
        Ok(data) => {
            let mime = if path == "/" || path.ends_with(".html") {
                "text/html; charset=utf-8"
            } else if path.ends_with(".css") {
                "text/css; charset=utf-8"
            } else if path.ends_with(".js") || path.ends_with(".mjs") {
                "application/javascript; charset=utf-8"
            } else {
                "application/octet-stream"
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: \
                 {}\r\nAccess-Control-Allow-Origin: *\r\nCache-Control: no-cache\r\n\r\n",
                mime,
                data.len()
            )?;
            stream.write_all(&data)?;
            stream.flush()
        }
        Err(_) => {
            if path.starts_with("/api/") {
                return handle_api(stream, viewer_dir, method, path, query, content_length);
            }
            let body = b"Not Found";
            write!(stream, "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\n\r\n")?;
            stream.write_all(body)?;
            stream.flush()
        }
    }
}

fn handle_tcp_connection(mut stream: TcpStream, viewer_dir: &str) -> io::Result<()> {
    let req_line = read_line(&mut stream)?;
    let parts: Vec<&str> = req_line.trim_end_matches(&['\r', '\n'][..]).splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Ok(());
    }
    let method = parts[0];
    let full_path = parts[1];
    let path = full_path.split('?').next().unwrap_or("/");
    let query = full_path.split_once('?').map(|x| x.1).unwrap_or("");

    let mut headers = HashMap::new();
    loop {
        let line = read_line(&mut stream)?;
        let line = line.trim_end_matches(&['\r', '\n'][..]).to_string();
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(": ") {
            headers.insert(k.to_lowercase(), v.to_string());
        }
    }

    let content_length: usize =
        headers.get("content-length").and_then(|v| v.parse().ok()).unwrap_or(0);

    if headers.get("upgrade").map(|s| s.to_lowercase()) == Some("websocket".into()) {
        let key = headers.get("sec-websocket-key").cloned().unwrap_or_default();
        let resp = ws_handshake_response(&key);
        stream.write_all(resp.as_bytes())?;
        stream.flush()?;

        let graph_bytes = {
            let guard = APP_STATE.lock().unwrap();
            guard.as_ref().map(|s| s.graph_proto.clone()).unwrap_or_default()
        };
        ws_write(&mut stream, &graph_bytes)?;

        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();
        ws_broadcaster().lock().unwrap().push(tx);

        loop {
            match ws_read_frame(&mut stream) {
                Ok(Some((data, opcode))) => {
                    if opcode == 9 {
                        let _ = stream.write_all(&[0x8A, 0x00]);
                        let _ = stream.flush();
                        continue;
                    }
                    handle_ws_frame(&mut stream, &data, viewer_dir);
                }
                Ok(None) => break,
                Err(_) => break,
            }
            while let Ok(msg) = rx.try_recv() {
                let _ = ws_write(&mut stream, &msg);
            }
        }
        return Ok(());
    }

    serve_http(stream, viewer_dir, method, path, query, content_length)
}

fn handle_ws_frame(stream: &mut TcpStream, data: &[u8], viewer_dir: &str) {
    if data.is_empty() {
        return;
    }

    if data[0] == 0x7B || data[0] == b'{' {
        if let Ok(text) = String::from_utf8(data.to_vec()) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                let msg_type = json["type"].as_str().unwrap_or("");
                match msg_type {
                    "request" => {
                        let id = json["id"].as_u64().unwrap_or(0);
                        let action = json["action"].as_str().unwrap_or("");
                        let payload = data.to_vec();
                        if let Ok(resp) = handle_ws_request(id, action, &payload, viewer_dir) {
                            let _ = ws_write_str(stream, &resp);
                        }
                    }
                    "eval_result" => {
                        broadcast_json(&text);
                    }
                    "debug_log" => {
                        let level = json["level"].as_str().unwrap_or("");
                        let msg = json["message"].as_str().unwrap_or("");
                        println!("[browser:{}] {}", level, msg);
                    }
                    "client_ready" => {}
                    _ => {}
                }
            }
        }
        return;
    }

    use prost::Message;
    if let Ok(msg) = WsMessage::decode(data) {
        handle_ws_message(stream, msg, viewer_dir);
    }
}

fn handle_ws_message(stream: &mut TcpStream, msg: WsMessage, viewer_dir: &str) {
    let msg_field = msg.msg;
    if msg_field.is_none() {
        return;
    }
    let msg_type = msg_field.unwrap();
    match msg_type {
        ws_message::Msg::Request(req) => {
            let id = req.id;
            let action = req.action;
            if let Ok(resp) = handle_ws_request(id, &action, &req.payload, viewer_dir) {
                let _ = ws_write_str(stream, &resp);
            }
        }
        ws_message::Msg::ClientReady(_) => {}
        ws_message::Msg::DebugLog(log) => {
            println!("[browser:{}] {}", log.level, log.message);
        }
        ws_message::Msg::EvalResult(res) => {
            let result_str = res.result.unwrap_or_default();
            let err_str = res.error.unwrap_or_default();
            let msg_json = serde_json::json!({
                "type": "eval_result",
                "id": res.id,
                "result": if result_str.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(result_str) },
                "error": if err_str.is_empty() { serde_json::Value::Null } else { serde_json::Value::String(err_str) },
            });
            broadcast_json(&msg_json.to_string());
        }
        _ => {}
    }
}

fn handle_debug_connection(mut stream: UnixStream) -> io::Result<()> {
    let req_line = read_line(&mut stream)?;
    let parts: Vec<&str> = req_line.trim_end_matches(&['\r', '\n'][..]).splitn(3, ' ').collect();
    let path = if parts.len() >= 2 { parts[1] } else { "/" };

    loop {
        let line = read_line(&mut stream)?;
        if line.trim().is_empty() {
            break;
        }
    }

    if path.starts_with("/eval") {
        let code = if let Some(q) = path.split_once('?') {
            q.1.trim_start_matches("code=").trim_start_matches("code%3D").to_string()
        } else {
            String::new()
        };

        if code.is_empty() {
            let body = br#"{"error":"no code parameter"}"#;
            write!(
                stream,
                "HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: \
                 {}\r\n\r\n",
                body.len()
            )?;
            stream.write_all(body)?;
            return stream.flush();
        }

        let msg = WsMessage {
            msg: Some(ws_message::Msg::EvalCommand(ws_message::EvalCommand { id: 0, code })),
        };
        let mut buf = Vec::new();
        let _ = prost::Message::encode(&msg, &mut buf);
        broadcast_proto(&buf);

        let start = std::time::Instant::now();
        let mut result = Vec::new();
        let (tx, rx) = std::sync::mpsc::channel::<Vec<u8>>();

        let broadcaster = ws_broadcaster();
        broadcaster.lock().unwrap().push(tx);

        while start.elapsed().as_secs() < 10 {
            if let Ok(msg_bytes) = rx.recv_timeout(std::time::Duration::from_millis(100)) {
                let ws_msg = WsMessage::decode(msg_bytes.as_ref());
                if let Ok(ws_msg) = ws_msg {
                    if let Some(ws_message::Msg::EvalResult(res)) = ws_msg.msg {
                        if let Some(result_str) = res.result {
                            result = result_str.into_bytes();
                            break;
                        }
                        if let Some(err) = res.error {
                            result = format!(r#"{{"error":"{}"}}"#, err).into_bytes();
                            break;
                        }
                    }
                }
            }
        }

        if result.is_empty() {
            let body = br#"{"error":"timeout waiting for browser"}"#;
            write!(
                stream,
                "HTTP/1.1 504 Gateway Timeout\r\nContent-Type: \
                 application/json\r\nContent-Length: {}\r\n\r\n",
                body.len()
            )?;
            stream.write_all(body)?;
            return stream.flush();
        }

        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            result.len()
        )?;
        stream.write_all(&result)?;
        return stream.flush();
    }

    if path == "/status" {
        let count = ws_broadcaster().lock().unwrap().len();
        let resp = GetWsConnectionsResponse { count: count as u32 };
        let mut buf = Vec::new();
        let _ = prost::Message::encode(&resp, &mut buf);
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: \
             {}\r\n\r\n",
            buf.len()
        )?;
        stream.write_all(&buf)?;
        return stream.flush();
    }

    let body = br#"{"error":"unknown path"}"#;
    write!(
        stream,
        "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}

pub fn serve(
    tcp_addr: &str,
    viewer_dir: &str,
    root_dir: &str,
    debug_socket: &str,
) -> io::Result<()> {
    {
        let mut reg_guard = REGISTRY.lock().unwrap();
        let mut registry = RepositoryRegistry::load();

        if let Ok(canonical) = std::path::Path::new(root_dir).canonicalize() {
            let path_str = canonical.to_string_lossy().to_string();
            if !registry.repos.contains_key(&path_str) {
                let info = crate::repo_registry::scan_repo_info(
                    &path_str,
                    &registry.config.excluded_paths,
                );
                registry.repos.insert(path_str.clone(), info);
            }
            registry.set_active_repo(path_str);
        }

        if registry.config.auto_discover {
            println!("[registry] Discovering repositories...");
            let found = registry.discover_repos();
            println!("[registry] Found {} repositories", found.len());
        }

        registry.save();
        *reg_guard = Some(registry);
    }

    {
        let active_path = {
            let reg_guard = REGISTRY.lock().unwrap();
            reg_guard.as_ref().and_then(|r| r.active_repo().map(String::from))
        };
        let path_to_analyze = active_path.as_deref().unwrap_or(root_dir);

        let mut state_guard = APP_STATE.lock().unwrap();
        *state_guard = Some(AppState::new(path_to_analyze));
    }

    let viewer = viewer_dir.to_string();

    let _ = std::fs::remove_file(debug_socket);

    let debug_listener = UnixListener::bind(debug_socket)?;
    println!("[debug] Listening on {}", debug_socket);
    println!(
        "[debug] Usage: curl --unix-socket {} http://debug/eval?code=document.title",
        debug_socket
    );

    let _debug_socket = debug_socket.to_string();
    let debug_listener = Arc::new(debug_listener);
    thread::spawn(move || {
        for stream in debug_listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = handle_debug_connection(stream) {
                        eprintln!("[debug] error: {}", e);
                    }
                }
                Err(e) => eprintln!("[debug] accept error: {}", e),
            }
        }
    });

    let tcp = TcpListener::bind(tcp_addr)?;
    println!("[server] Listening on {}", tcp_addr);
    println!("[server] Open http://{}/ in your browser", tcp_addr);

    for stream in tcp.incoming() {
        match stream {
            Ok(stream) => {
                let viewer = viewer.clone();
                thread::spawn(move || {
                    if let Err(e) = handle_tcp_connection(stream, &viewer) {
                        eprintln!("[server] error: {}", e);
                    }
                });
            }
            Err(e) => eprintln!("[server] accept error: {}", e),
        }
    }

    Ok(())
}

fn handle_api(
    mut stream: TcpStream,
    _viewer_dir: &str,
    method: &str,
    path: &str,
    query: &str,
    content_length: usize,
) -> io::Result<()> {
    if path == "/api/chat" && method == "POST" {
        let mut body_bytes = vec![0u8; content_length];
        if content_length > 0 {
            stream.read_exact(&mut body_bytes)?;
        }

        let request: qwen::ChatRequest = if body_bytes.is_empty() {
            qwen::ChatRequest {
                message: String::new(),
                context: None,
                summary: None,
                session_id: None,
            }
        } else {
            match serde_json::from_slice(&body_bytes) {
                Ok(r) => r,
                Err(e) => {
                    let resp = ErrorResponse { error: format!("Invalid JSON: {}", e) };
                    let mut buf = Vec::new();
                    let _ = prost::Message::encode(&resp, &mut buf);
                    return write_proto_binary(&mut stream, 400, "application/octet-stream", &buf);
                }
            }
        };

        let (summary, diagnostics_ctx) = {
            let guard = APP_STATE.lock().unwrap();
            guard
                .as_ref()
                .map(|s| {
                    let include_summary =
                        request.session_id.as_ref().map(|sid| is_new_session(sid)).unwrap_or(true)
                            || request.summary.is_some();
                    let summary = if include_summary {
                        Some(qwen::build_codebase_summary(s.graph()))
                    } else {
                        None
                    };
                    (summary, Some(s.diagnostics_report.clone()))
                })
                .unwrap_or((None, None))
        };

        let mut full_context = summary.unwrap_or_default();
        if let Some(ref diag) = diagnostics_ctx {
            if !diag.is_empty() {
                full_context.push_str("\n\n## Code Quality & Diagnostics\n");
                full_context.push_str(diag);
            }
        }

        let response = qwen::chat_with_tools(
            &request.message,
            if full_context.is_empty() { None } else { Some(&full_context) },
        );
        let resp = ToolUseResponse {
            reply: response.reply,
            tools: response
                .tools
                .into_iter()
                .map(|t| ToolCallInfo {
                    name: t.name,
                    args: t.args,
                    output: t.output,
                    content: t.content,
                })
                .collect(),
            error: response.error,
        };
        let mut buf = Vec::new();
        prost::Message::encode(&resp, &mut buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
    }

    if path == "/api/analyze" && method == "POST" {
        let analyze_path = if content_length > 0 {
            let mut body_bytes = vec![0u8; content_length];
            let _ = stream.read_exact(&mut body_bytes);
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&body_bytes) {
                json.get("path")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .or_else(|| {
                        query.split('&').find_map(|p| p.strip_prefix("path=")).map(url_decode)
                    })
                    .unwrap_or_default()
            } else {
                query
                    .split('&')
                    .find_map(|p| p.strip_prefix("path="))
                    .map(url_decode)
                    .unwrap_or_default()
            }
        } else {
            query
                .split('&')
                .find_map(|p| p.strip_prefix("path="))
                .map(url_decode)
                .unwrap_or_default()
        };

        if analyze_path.is_empty() || !std::path::Path::new(&analyze_path).is_dir() {
            let resp = ErrorResponse { error: "invalid or missing path parameter".into() };
            let mut buf = Vec::new();
            let _ = prost::Message::encode(&resp, &mut buf);
            return write_proto_binary(&mut stream, 400, "application/octet-stream", &buf);
        }

        println!("[analyze] Re-analyzing {}...", analyze_path);
        let state = AppState::new(&analyze_path);
        {
            let mut guard = APP_STATE.lock().unwrap();
            *guard = Some(state);
        }

        let graph_push = {
            let guard = APP_STATE.lock().unwrap();
            guard.as_ref().map(|s| {
                let msg = WsMessage { msg: Some(ws_message::Msg::GraphData(s.graph().clone())) };
                let mut buf = Vec::new();
                let _ = prost::Message::encode(&msg, &mut buf);
                buf
            })
        };
        if let Some(push) = graph_push {
            broadcast_proto(&push);
        }

        let resp = {
            let guard = APP_STATE.lock().unwrap();
            guard
                .as_ref()
                .map(|s| {
                    let graph = s.graph();
                    AnalyzeResponse {
                        status: "ok".into(),
                        path: analyze_path.clone(),
                        functions: graph.node_count,
                        edges: graph.edge_count,
                    }
                })
                .unwrap_or_else(|| AnalyzeResponse {
                    status: "error".into(),
                    path: analyze_path.clone(),
                    functions: 0,
                    edges: 0,
                })
        };
        let mut buf = Vec::new();
        prost::Message::encode(&resp, &mut buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
    }

    if path == "/api/graph.proto" {
        let state_guard = APP_STATE.lock().unwrap();
        if let Some(ref state) = *state_guard {
            let body = &state.graph_proto;
            return write_proto_binary(&mut stream, 200, "application/octet-stream", body);
        }
    }

    if path == "/api/apply-edit" && method == "POST" {
        let mut body_bytes = vec![0u8; content_length];
        if content_length > 0 {
            stream.read_exact(&mut body_bytes)?;
        }

        let edit_req: serde_json::Value = match serde_json::from_slice(&body_bytes) {
            Ok(v) => v,
            Err(e) => {
                let resp = ErrorResponse { error: format!("Invalid JSON: {}", e) };
                let mut buf = Vec::new();
                let _ = prost::Message::encode(&resp, &mut buf);
                return write_proto_binary(&mut stream, 400, "application/octet-stream", &buf);
            }
        };

        let file_path = edit_req["path"].as_str().unwrap_or("");
        let content = edit_req["content"].as_str().unwrap_or("");

        if content.len() > MAX_EDIT_SIZE {
            let resp =
                ErrorResponse { error: format!("content too large (max {} bytes)", MAX_EDIT_SIZE) };
            let mut buf = Vec::new();
            let _ = prost::Message::encode(&resp, &mut buf);
            return write_proto_binary(&mut stream, 413, "application/octet-stream", &buf);
        }

        if file_path.is_empty() {
            let resp = ErrorResponse { error: "missing 'path' parameter".into() };
            let mut buf = Vec::new();
            let _ = prost::Message::encode(&resp, &mut buf);
            return write_proto_binary(&mut stream, 400, "application/octet-stream", &buf);
        }

        let state_guard = APP_STATE.lock().unwrap();
        let result = if let Some(ref state) = *state_guard {
            let clean_path = file_path.trim_start_matches('/');
            let full_path = std::path::Path::new(&state.root_dir).join(clean_path);

            let canonical_root = match std::path::Path::new(&state.root_dir).canonicalize() {
                Ok(r) => r,
                Err(e) => {
                    let resp = ErrorResponse { error: format!("Cannot resolve root path: {}", e) };
                    let mut buf = Vec::new();
                    let _ = prost::Message::encode(&resp, &mut buf);
                    return write_proto_binary(&mut stream, 500, "application/octet-stream", &buf);
                }
            };
            let canonical_full = match full_path.canonicalize() {
                Ok(p) => p,
                Err(e) => {
                    let resp = ErrorResponse { error: format!("Cannot resolve file path: {}", e) };
                    let mut buf = Vec::new();
                    let _ = prost::Message::encode(&resp, &mut buf);
                    return write_proto_binary(&mut stream, 400, "application/octet-stream", &buf);
                }
            };

            if !canonical_full.starts_with(&canonical_root) {
                ApplyEditResponse { status: "error".into(), path: file_path.into(), bytes: 0 }
            } else {
                match std::fs::write(&canonical_full, content) {
                    Ok(()) => ApplyEditResponse {
                        status: "ok".into(),
                        path: file_path.into(),
                        bytes: content.len() as u32,
                    },
                    Err(e) => ApplyEditResponse {
                        status: format!("error: {}", e),
                        path: file_path.into(),
                        bytes: 0,
                    },
                }
            }
        } else {
            ApplyEditResponse {
                status: "error: no codebase analyzed".into(),
                path: file_path.into(),
                bytes: 0,
            }
        };

        let mut buf = Vec::new();
        prost::Message::encode(&result, &mut buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
    }

    if path == "/api/file" {
        let state_guard = APP_STATE.lock().unwrap();
        if let Some(ref state) = *state_guard {
            let file_path = query.split('&').find_map(|p| p.strip_prefix("path=")).unwrap_or("");
            let decoded = url_decode(file_path);
            let full_path = format!("{}/{}", state.root_dir.trim_end_matches('/'), decoded);

            if let Ok(metadata) = std::fs::metadata(&full_path) {
                if metadata.len() as usize > MAX_FILE_SIZE {
                    let resp = ErrorResponse {
                        error: format!("file too large (max {} bytes)", MAX_FILE_SIZE),
                    };
                    let mut buf = Vec::new();
                    let _ = prost::Message::encode(&resp, &mut buf);
                    return write_proto_binary(&mut stream, 413, "application/octet-stream", &buf);
                }
            }

            match std::fs::read_to_string(&full_path) {
                Ok(content) => {
                    let resp = GetFileResponse { content };
                    let mut buf = Vec::new();
                    prost::Message::encode(&resp, &mut buf)
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
                }
                Err(e) => {
                    let resp = ErrorResponse { error: format!("cannot read {}: {}", decoded, e) };
                    let mut buf = Vec::new();
                    let _ = prost::Message::encode(&resp, &mut buf);
                    return write_proto_binary(&mut stream, 404, "application/octet-stream", &buf);
                }
            }
        }
    }

    if path == "/api/stats" {
        let state_guard = APP_STATE.lock().unwrap();
        if let Some(ref state) = *state_guard {
            let graph = state.graph();
            let mut lang_counts: std::collections::HashMap<String, u32> =
                std::collections::HashMap::new();
            let mut file_counts: std::collections::HashMap<String, u32> =
                std::collections::HashMap::new();
            for n in &graph.nodes {
                *lang_counts.entry(n.language.clone()).or_insert(0) += 1;
                *file_counts.entry(n.file.clone()).or_insert(0) += 1;
            }
            let resp = StatsResponse {
                languages: lang_counts,
                files: file_counts,
                total_functions: graph.node_count,
                total_edges: graph.edge_count,
            };
            let mut buf = Vec::new();
            prost::Message::encode(&resp, &mut buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
        }
    }

    if path == "/api/query" {
        let state_guard = APP_STATE.lock().unwrap();
        if let Some(ref state) = *state_guard {
            let query_str = query.split('&').find_map(|p| p.strip_prefix("q=")).unwrap_or("");
            let query_lower = url_decode(query_str).to_lowercase();
            let graph = state.graph();
            let matches: Vec<QueryMatch> = graph
                .nodes
                .iter()
                .filter_map(|n| {
                    if n.name.to_lowercase().contains(&query_lower) {
                        Some(QueryMatch {
                            id: n.id.clone(),
                            name: n.name.clone(),
                            file: n.file.clone(),
                            language: n.language.clone(),
                        })
                    } else {
                        None
                    }
                })
                .collect();
            let total_count = matches.len() as u32;
            let resp = QueryResult { matches, total_count };
            let mut buf = Vec::new();
            prost::Message::encode(&resp, &mut buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
        }
    }

    if path == "/api/summary" {
        let state_guard = APP_STATE.lock().unwrap();
        if let Some(ref state) = *state_guard {
            let graph = state.graph();
            let mut lang_counts: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for n in &graph.nodes {
                *lang_counts.entry(n.language.clone()).or_insert(0) += 1;
            }
            let mut md = String::new();
            md.push_str("# Code Analysis Summary\n\n");
            md.push_str(&format!("**Total Functions:** {}\n", graph.node_count));
            md.push_str(&format!("**Total Edges:** {}\n\n", graph.edge_count));
            md.push_str("## Languages\n\n");
            let mut langs: Vec<_> = lang_counts.iter().collect();
            langs.sort_by(|a, b| b.1.cmp(a.1));
            for (lang, count) in langs {
                md.push_str(&format!("- **{}**: {} functions\n", lang, count));
            }
            let bytes = md.as_bytes();
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: text/markdown\r\nContent-Length: \
                 {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
                bytes.len()
            )?;
            stream.write_all(bytes)?;
            return stream.flush();
        }
    }

    if path == "/api/xref" {
        let state_guard = APP_STATE.lock().unwrap();
        if let Some(ref state) = *state_guard {
            let func_name = query.split('&').find_map(|p| p.strip_prefix("func=")).unwrap_or("");
            let decoded = url_decode(func_name);
            let graph = state.graph();
            let mut callers: Vec<String> = Vec::new();
            let mut callees: Vec<String> = Vec::new();
            for e in &graph.edges {
                if e.target.ends_with(&format!("::{}", decoded)) || e.target == decoded {
                    callers.push(e.source.clone());
                }
                if e.source.ends_with(&format!("::{}", decoded)) || e.source == decoded {
                    callees.push(e.target.clone());
                }
            }
            let resp =
                generated::codeanalyzer::XrefResponse { function: decoded, callers, callees };
            let mut buf = Vec::new();
            prost::Message::encode(&resp, &mut buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
        }
    }

    if path == "/api/fs" {
        let state_guard = APP_STATE.lock().unwrap();
        if let Some(ref state) = *state_guard {
            let dir_path = query.split('&').find_map(|p| p.strip_prefix("path=")).unwrap_or("/");
            let decoded = url_decode(dir_path);
            let full_path = if decoded == "/" {
                state.root_dir.clone()
            } else {
                format!(
                    "{}/{}",
                    state.root_dir.trim_end_matches('/'),
                    decoded.trim_start_matches('/')
                )
            };
            let mut entries: Vec<FileEntry> = Vec::new();
            if let Ok(dir) = std::fs::read_dir(&full_path) {
                for entry in dir.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let path = entry.path();
                    let is_dir = path.is_dir();
                    let entry_path = if decoded == "/" {
                        format!("/{}", name)
                    } else {
                        format!("{}/{}", decoded, name)
                    };
                    entries.push(FileEntry { name, path: entry_path, is_dir });
                }
            }
            entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            });
            let resp = DirListing { entries };
            let mut buf = Vec::new();
            prost::Message::encode(&resp, &mut buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
        }
    }

    if path == "/api/diagnostics" {
        let state_guard = APP_STATE.lock().unwrap();
        if let Some(ref state) = *state_guard {
            let file_path = query.split('&').find_map(|p| p.strip_prefix("path=")).map(url_decode);
            let diags = diagnostics::run_linters(&state.root_dir, file_path.as_deref());
            let resp = GetDiagnosticsResponse {
                diagnostics: diags
                    .into_iter()
                    .map(|d| generated::codeanalyzer::Diagnostic {
                        file: d.file,
                        line: d.line,
                        column: d.column,
                        severity: d.severity,
                        message: d.message,
                        source: d.source,
                    })
                    .collect(),
            };
            let mut buf = Vec::new();
            prost::Message::encode(&resp, &mut buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
        }
    }

    if path == "/api/files" {
        let state_guard = APP_STATE.lock().unwrap();
        if let Some(ref state) = *state_guard {
            let graph = state.graph();

            let mut file_map: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();
            for n in &graph.nodes {
                file_map.entry(n.file.clone()).or_default().push(n.name.clone());
            }
            let mut deps: std::collections::HashMap<String, Vec<String>> =
                std::collections::HashMap::new();
            for e in &graph.edges {
                if e.source.contains("::") && e.target.contains("::") {
                    let src_file = e.source.split("::").next().unwrap_or(&e.source);
                    let dst_file = e.target.split("::").next().unwrap_or(&e.target);
                    if src_file != dst_file {
                        deps.entry(src_file.to_string()).or_default().push(dst_file.to_string());
                    }
                }
            }
            for (_, v) in deps.iter_mut() {
                v.sort();
                v.dedup();
                if v.len() > 20 {
                    v.truncate(20);
                }
            }
            let files: Vec<FileData> = file_map
                .into_iter()
                .map(|(file, funcs)| {
                    let deps_list = deps.get(&file).cloned().unwrap_or_default();
                    FileData { file, functions: funcs, dependencies: deps_list }
                })
                .collect();
            let resp = FilesResponse { files };
            let mut buf = Vec::new();
            prost::Message::encode(&resp, &mut buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
        }
    }

    if path == "/api/repos" && method == "GET" {
        let reg_guard = REGISTRY.lock().unwrap();
        if let Some(ref registry) = *reg_guard {
            let repos: Vec<RepoInfo> = registry
                .list_repos()
                .into_iter()
                .map(|r| RepoInfo {
                    path: r.path.clone(),
                    name: r.name.clone(),
                    is_git_repo: r.is_git_repo,
                    git_remote: r.git_remote.clone(),
                    file_count: r.file_count as u32,
                    total_size_bytes: r.total_size_bytes,
                    last_modified: r.last_modified,
                    index_state: Some(repo_index_state_to_proto(&r.index_state)),
                    languages: r.languages.clone(),
                    added_at: r.added_at,
                })
                .collect();
            let resp = RepoList { repos, active: registry.active_repo().map(String::from) };
            let mut buf = Vec::new();
            prost::Message::encode(&resp, &mut buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
        }
    }

    if path == "/api/repos/discover" && method == "POST" {
        let mut reg_guard = REGISTRY.lock().unwrap();
        if let Some(ref mut registry) = *reg_guard {
            println!("[registry] Discovering repositories...");
            let found = registry.discover_repos();
            println!("[registry] Found {} repositories", found.len());
            registry.save();
            let repos: Vec<RepoInfo> = found
                .into_iter()
                .map(|r| RepoInfo {
                    path: r.path.clone(),
                    name: r.name.clone(),
                    is_git_repo: r.is_git_repo,
                    git_remote: r.git_remote,
                    file_count: r.file_count as u32,
                    total_size_bytes: r.total_size_bytes,
                    last_modified: r.last_modified,
                    index_state: Some(repo_index_state_to_proto(&r.index_state)),
                    languages: r.languages,
                    added_at: r.added_at,
                })
                .collect();
            let resp = DiscoverReposResponse {
                found: repos.len() as u32,
                repos: Some(RepoList { repos, active: registry.active_repo().map(String::from) }),
            };
            let mut buf = Vec::new();
            prost::Message::encode(&resp, &mut buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
        }
    }

    if path == "/api/repos/add" && method == "POST" {
        let repo_path = query
            .split('&')
            .find_map(|p| p.strip_prefix("path="))
            .map(url_decode)
            .unwrap_or_default();

        if repo_path.is_empty() {
            let resp = ErrorResponse { error: "missing 'path' parameter".into() };
            let mut buf = Vec::new();
            let _ = prost::Message::encode(&resp, &mut buf);
            return write_proto_binary(&mut stream, 400, "application/octet-stream", &buf);
        }

        let mut reg_guard = REGISTRY.lock().unwrap();
        if let Some(ref mut registry) = *reg_guard {
            match registry.add_repo(&repo_path) {
                Ok(()) => {
                    let resp = AddRepoResponse {
                        status: Some(StatusResponse {
                            status: "ok".into(),
                            error: None,
                            functions: None,
                            edges: None,
                            found: None,
                        }),
                        path: repo_path,
                    };
                    let mut buf = Vec::new();
                    prost::Message::encode(&resp, &mut buf)
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
                }
                Err(e) => {
                    let resp = ErrorResponse { error: e };
                    let mut buf = Vec::new();
                    let _ = prost::Message::encode(&resp, &mut buf);
                    return write_proto_binary(&mut stream, 400, "application/octet-stream", &buf);
                }
            }
        }
    }

    if path == "/api/repos/remove" && method == "POST" {
        let repo_path = query
            .split('&')
            .find_map(|p| p.strip_prefix("path="))
            .map(url_decode)
            .unwrap_or_default();

        let mut reg_guard = REGISTRY.lock().unwrap();
        if let Some(ref mut registry) = *reg_guard {
            let removed = registry.remove_repo(&repo_path);
            let resp = RemoveRepoResponse {
                status: Some(StatusResponse {
                    status: if removed { "ok" } else { "not_found" }.into(),
                    error: None,
                    functions: None,
                    edges: None,
                    found: None,
                }),
            };
            let mut buf = Vec::new();
            prost::Message::encode(&resp, &mut buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
        }
    }

    if path == "/api/switch" && method == "POST" {
        let repo_path = query
            .split('&')
            .find_map(|p| p.strip_prefix("path="))
            .map(url_decode)
            .unwrap_or_default();

        if repo_path.is_empty() {
            let resp = ErrorResponse { error: "missing 'path' parameter".into() };
            let mut buf = Vec::new();
            let _ = prost::Message::encode(&resp, &mut buf);
            return write_proto_binary(&mut stream, 400, "application/octet-stream", &buf);
        }

        {
            let mut reg_guard = REGISTRY.lock().unwrap();
            if let Some(ref mut registry) = *reg_guard {
                if let Err(e) = registry.switch_repo(&repo_path) {
                    let resp = ErrorResponse { error: e };
                    let mut buf = Vec::new();
                    let _ = prost::Message::encode(&resp, &mut buf);
                    return write_proto_binary(&mut stream, 400, "application/octet-stream", &buf);
                }
            }
        }

        println!("[server] Switching to {}...", repo_path);
        let state = AppState::new(&repo_path);
        {
            let mut state_guard = APP_STATE.lock().unwrap();
            *state_guard = Some(state);
        }

        let graph_push = {
            let guard = APP_STATE.lock().unwrap();
            guard.as_ref().map(|s| {
                let msg = WsMessage { msg: Some(ws_message::Msg::GraphData(s.graph().clone())) };
                let mut buf = Vec::new();
                let _ = prost::Message::encode(&msg, &mut buf);
                buf
            })
        };
        if let Some(push) = graph_push {
            broadcast_proto(&push);
        }

        let resp = {
            let guard = APP_STATE.lock().unwrap();
            guard
                .as_ref()
                .map(|s| {
                    let graph = s.graph();
                    SwitchRepoResponse {
                        status: Some(StatusResponse {
                            status: "ok".into(),
                            error: None,
                            functions: Some(graph.node_count),
                            edges: Some(graph.edge_count),
                            found: None,
                        }),
                    }
                })
                .unwrap_or_else(|| SwitchRepoResponse {
                    status: Some(StatusResponse {
                        status: "error".into(),
                        error: None,
                        functions: None,
                        edges: None,
                        found: None,
                    }),
                })
        };
        let mut buf = Vec::new();
        prost::Message::encode(&resp, &mut buf)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
    }

    if path == "/api/index" && method == "POST" {
        let repo_path = query
            .split('&')
            .find_map(|p| p.strip_prefix("path="))
            .map(url_decode)
            .unwrap_or_default();

        let reg_guard = REGISTRY.lock().unwrap();
        if let Some(ref registry) = *reg_guard {
            match registry.start_indexing(&repo_path) {
                Ok(()) => {
                    let resp = StatusResponse {
                        status: "ok".into(),
                        error: Some("Indexing started".into()),
                        functions: None,
                        edges: None,
                        found: None,
                    };
                    let mut buf = Vec::new();
                    prost::Message::encode(&resp, &mut buf)
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
                    return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
                }
                Err(e) => {
                    let resp = ErrorResponse { error: e };
                    let mut buf = Vec::new();
                    let _ = prost::Message::encode(&resp, &mut buf);
                    return write_proto_binary(&mut stream, 400, "application/octet-stream", &buf);
                }
            }
        }
    }

    if path == "/api/config" && method == "GET" {
        let reg_guard = REGISTRY.lock().unwrap();
        if let Some(ref registry) = *reg_guard {
            let resp = GetConfigResponse {
                config: Some(RegistryConfig {
                    scan_roots: registry.config.scan_roots.clone(),
                    excluded_paths: registry.config.excluded_paths.clone(),
                    max_concurrent_index: registry.config.max_concurrent_index as u32,
                    auto_discover: registry.config.auto_discover,
                }),
            };
            let mut buf = Vec::new();
            prost::Message::encode(&resp, &mut buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
        }
    }

    if path == "/api/config" && method == "POST" {
        let _body_bytes = vec![0u8; content_length];
        if content_length > 0 {}
        let reg_guard = REGISTRY.lock().unwrap();
        if let Some(ref registry) = *reg_guard {
            let resp = GetConfigResponse {
                config: Some(RegistryConfig {
                    scan_roots: registry.config.scan_roots.clone(),
                    excluded_paths: registry.config.excluded_paths.clone(),
                    max_concurrent_index: registry.config.max_concurrent_index as u32,
                    auto_discover: registry.config.auto_discover,
                }),
            };
            let mut buf = Vec::new();
            prost::Message::encode(&resp, &mut buf)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            return write_proto_binary(&mut stream, 200, "application/octet-stream", &buf);
        }
    }

    let body = br#"{"error": "not implemented"}"#;
    write!(
        stream,
        "HTTP/1.1 404 Not Found\r\nContent-Type: application/json\r\nContent-Length: \
         {}\r\nAccess-Control-Allow-Origin: *\r\n\r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}

fn repo_index_state_to_proto(state: &crate::repo_registry::IndexState) -> IndexState {
    match state {
        crate::repo_registry::IndexState::NotIndexed => {
            IndexState { state: Some(index_state::State::NotIndexed(index_state::NotIndexed {})) }
        }
        crate::repo_registry::IndexState::Indexing { progress, files_scanned, total_files } => {
            IndexState {
                state: Some(index_state::State::Indexing(index_state::Indexing {
                    progress: *progress,
                    files_scanned: *files_scanned as u32,
                    total_files: *total_files as u32,
                })),
            }
        }
        crate::repo_registry::IndexState::Indexed { functions, edges, index_time_ms } => {
            IndexState {
                state: Some(index_state::State::Indexed(index_state::Indexed {
                    functions: *functions as u32,
                    edges: *edges as u32,
                    index_time_ms: *index_time_ms,
                })),
            }
        }
        crate::repo_registry::IndexState::Stale { functions, edges, last_indexed } => IndexState {
            state: Some(index_state::State::Stale(index_state::Stale {
                functions: *functions as u32,
                edges: *edges as u32,
                last_indexed: *last_indexed,
            })),
        },
        crate::repo_registry::IndexState::Error { message } => IndexState {
            state: Some(index_state::State::Error(index_state::Error { message: message.clone() })),
        },
    }
}

fn url_decode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.bytes();
    while let Some(b) = chars.next() {
        if b == b'%' {
            let h = chars.next().unwrap_or(b'0');
            let l = chars.next().unwrap_or(b'0');
            let h = (h as char).to_digit(16).unwrap_or(0) as u8;
            let l = (l as char).to_digit(16).unwrap_or(0) as u8;
            out.push((h << 4 | l) as char);
        } else if b == b'+' {
            out.push(' ');
        } else {
            out.push(b as char);
        }
    }
    out
}

fn handle_ws_request(
    request_id: u64,
    action: &str,
    payload: &[u8],
    _viewer_dir: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let state_guard = APP_STATE.lock().unwrap();
    let state = state_guard.as_ref();

    let result: Result<Vec<u8>, String> = match action {
        "get_graph" => {
            if let Some(s) = state {
                let graph = s.graph();
                let resp = WsMessage { msg: Some(ws_message::Msg::GraphData(graph.clone())) };
                let mut buf = Vec::new();
                prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            } else {
                Err("No state available".into())
            }
        }

        "get_file" => {
            let json_payload: serde_json::Value =
                serde_json::from_slice(payload).unwrap_or_default();
            let path = json_payload["path"].as_str().unwrap_or("");
            if let Some(s) = state {
                let decoded = url_decode(path);
                let full_path = format!("{}/{}", s.root_dir.trim_end_matches('/'), decoded);
                match std::fs::read_to_string(&full_path) {
                    Ok(content) => {
                        let resp = GetFileResponse { content };
                        let mut buf = Vec::new();
                        prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
                        Ok(buf)
                    }
                    Err(e) => Err(format!("Cannot read {}: {}", decoded, e)),
                }
            } else {
                Err("No state available".into())
            }
        }

        "get_fs" => {
            let json_payload: serde_json::Value =
                serde_json::from_slice(payload).unwrap_or_default();
            let path = json_payload["path"].as_str().unwrap_or("/");
            if let Some(s) = state {
                let decoded = url_decode(path);
                let full_path = if decoded == "/" {
                    s.root_dir.clone()
                } else {
                    format!(
                        "{}/{}",
                        s.root_dir.trim_end_matches('/'),
                        decoded.trim_start_matches('/')
                    )
                };
                let mut entries: Vec<FileEntry> = Vec::new();
                if let Ok(dir) = std::fs::read_dir(&full_path) {
                    for entry in dir.flatten() {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let is_dir = entry.path().is_dir();
                        let entry_path = if decoded == "/" {
                            format!("/{}", name)
                        } else {
                            format!("{}/{}", decoded, name)
                        };
                        entries.push(FileEntry { name, path: entry_path, is_dir });
                    }
                }
                entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.cmp(&b.name),
                });
                let resp = DirListing { entries };
                let mut buf = Vec::new();
                prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            } else {
                Err("No state available".into())
            }
        }

        "get_view" => {
            let json_payload: serde_json::Value =
                serde_json::from_slice(payload).unwrap_or_default();
            let view_type = json_payload["view"].as_str().unwrap_or("functions");
            if let Some(s) = state {
                let graph = s.graph();
                let (view_nodes, view_edges) = match view_type {
                    "files" => {
                        let mut file_set = std::collections::HashSet::new();
                        let mut file_edges = std::collections::HashSet::new();
                        for n in &graph.nodes {
                            file_set.insert(n.file.clone());
                        }
                        for e in &graph.edges {
                            let sf = e.source.split("::").next().unwrap_or(&e.source);
                            let tf = e.target.split("::").next().unwrap_or(&e.target);
                            if sf != tf {
                                file_edges.insert((sf.to_string(), tf.to_string()));
                            }
                        }
                        (
                            file_set
                                .into_iter()
                                .map(|f| ViewNode { id: f.clone(), name: f, r#type: "file".into() })
                                .collect::<Vec<_>>(),
                            file_edges
                                .into_iter()
                                .map(|(s, t)| ViewEdge {
                                    source: s,
                                    target: t,
                                    kind: "calls".into(),
                                })
                                .collect::<Vec<_>>(),
                        )
                    }
                    "directories" => {
                        let mut dir_set = std::collections::HashSet::new();
                        let mut dir_edges = std::collections::HashSet::new();
                        for n in &graph.nodes {
                            if let Some(pos) = n.file.rfind('/') {
                                dir_set.insert(n.file[..pos].to_string());
                            }
                        }
                        for e in &graph.edges {
                            let sd = e
                                .source
                                .split("::")
                                .next()
                                .and_then(|f| f.rsplit('/').next())
                                .unwrap_or(".");
                            let td = e
                                .target
                                .split("::")
                                .next()
                                .and_then(|f| f.rsplit('/').next())
                                .unwrap_or(".");
                            if sd != td {
                                dir_edges.insert((sd.to_string(), td.to_string()));
                            }
                        }
                        (
                            dir_set
                                .into_iter()
                                .map(|d| ViewNode {
                                    id: d.clone(),
                                    name: d,
                                    r#type: "directory".into(),
                                })
                                .collect::<Vec<_>>(),
                            dir_edges
                                .into_iter()
                                .map(|(s, t)| ViewEdge {
                                    source: s,
                                    target: t,
                                    kind: "calls".into(),
                                })
                                .collect::<Vec<_>>(),
                        )
                    }
                    _ => {
                        let nodes: Vec<ViewNode> = graph
                            .nodes
                            .iter()
                            .map(|n| ViewNode {
                                id: n.id.clone(),
                                name: n.name.clone(),
                                r#type: "function".into(),
                            })
                            .collect();
                        let edges: Vec<ViewEdge> = graph
                            .edges
                            .iter()
                            .map(|e| ViewEdge {
                                source: e.source.clone(),
                                target: e.target.clone(),
                                kind: e.kind.clone(),
                            })
                            .collect();
                        (nodes, edges)
                    }
                };
                let resp = GetViewResponse {
                    data: Some(ViewData { nodes: view_nodes, edges: view_edges }),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            } else {
                Err("No state available".into())
            }
        }

        "get_diagnostics" => {
            let json_payload: serde_json::Value =
                serde_json::from_slice(payload).unwrap_or_default();
            let file_path = json_payload["path"].as_str();
            if let Some(s) = state {
                let diags = diagnostics::run_linters(&s.root_dir, file_path);
                let resp = GetDiagnosticsResponse {
                    diagnostics: diags
                        .into_iter()
                        .map(|d| generated::codeanalyzer::Diagnostic {
                            file: d.file,
                            line: d.line,
                            column: d.column,
                            severity: d.severity,
                            message: d.message,
                            source: d.source,
                        })
                        .collect(),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            } else {
                Err("No state available".into())
            }
        }

        "get_files" => {
            if let Some(s) = state {
                let graph = s.graph();
                let mut file_map: std::collections::HashMap<String, Vec<String>> =
                    std::collections::HashMap::new();
                for n in &graph.nodes {
                    file_map.entry(n.file.clone()).or_default().push(n.name.clone());
                }
                let mut deps: std::collections::HashMap<String, Vec<String>> =
                    std::collections::HashMap::new();
                for e in &graph.edges {
                    if e.source.contains("::") && e.target.contains("::") {
                        let src_file = e.source.split("::").next().unwrap_or(&e.source);
                        let dst_file = e.target.split("::").next().unwrap_or(&e.target);
                        if src_file != dst_file {
                            deps.entry(src_file.to_string())
                                .or_default()
                                .push(dst_file.to_string());
                        }
                    }
                }
                for (_, v) in deps.iter_mut() {
                    v.sort();
                    v.dedup();
                    if v.len() > 20 {
                        v.truncate(20);
                    }
                }
                let files: Vec<FileData> = file_map
                    .into_iter()
                    .map(|(file, funcs)| {
                        let deps_list = deps.get(&file).cloned().unwrap_or_default();
                        FileData { file, functions: funcs, dependencies: deps_list }
                    })
                    .collect();
                let resp = GetFilesResponse { files };
                let mut buf = Vec::new();
                prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            } else {
                Err("No state available".into())
            }
        }

        "analyze" => {
            let json_payload: serde_json::Value =
                serde_json::from_slice(payload).unwrap_or_default();
            let path = json_payload["path"].as_str().unwrap_or("");
            if !path.is_empty() && std::path::Path::new(path).is_dir() {
                let new_state = AppState::new(path);
                drop(state_guard);
                let mut guard = APP_STATE.lock().unwrap();
                *guard = Some(new_state);
                let guard2 = APP_STATE.lock().unwrap();
                if let Some(ref s) = *guard2 {
                    let graph = s.graph();
                    let functions = graph.node_count;
                    let edge_count = graph.edge_count;
                    let msg = WsMessage { msg: Some(ws_message::Msg::GraphData(graph.clone())) };
                    let mut push = Vec::new();
                    let _ = prost::Message::encode(&msg, &mut push);
                    broadcast_proto(&push);
                    let resp = AnalyzeResponse {
                        status: "ok".into(),
                        path: path.into(),
                        functions,
                        edges: edge_count,
                    };
                    let mut buf = Vec::new();
                    prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
                    Ok(buf)
                } else {
                    Err("Analysis failed".into())
                }
            } else {
                Err("Invalid path".into())
            }
        }

        "apply_edit" => {
            let json_payload: serde_json::Value =
                serde_json::from_slice(payload).unwrap_or_default();
            let file_path = json_payload["path"].as_str().unwrap_or("");
            let content = json_payload["content"].as_str().unwrap_or("");
            if file_path.is_empty() {
                Err("Missing 'path' parameter".into())
            } else if let Some(s) = state {
                let clean_path = file_path.trim_start_matches('/');
                let full_path = std::path::Path::new(&s.root_dir).join(clean_path);
                if !full_path.starts_with(&s.root_dir) {
                    Err("Path is outside the project root.".into())
                } else {
                    match std::fs::write(&full_path, content) {
                        Ok(()) => {
                            let resp = ApplyEditResponse {
                                status: "ok".into(),
                                path: file_path.into(),
                                bytes: content.len() as u32,
                            };
                            let mut buf = Vec::new();
                            prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
                            Ok(buf)
                        }
                        Err(e) => Err(format!("Failed to write file: {}", e)),
                    }
                }
            } else {
                Err("No state available".into())
            }
        }

        "chat" => {
            let json_payload: serde_json::Value =
                serde_json::from_slice(payload).unwrap_or_default();
            let message = json_payload["message"].as_str().unwrap_or("");
            let (summary, diagnostics_ctx) = if let Some(s) = state {
                let summary = Some(crate::qwen::build_codebase_summary(s.graph()));
                (summary, Some(s.diagnostics_report.clone()))
            } else {
                (None, None)
            };

            let mut full_context = summary.unwrap_or_default();
            if let Some(ref diag) = diagnostics_ctx {
                if !diag.is_empty() {
                    full_context.push_str("\n\n## Code Quality & Diagnostics\n");
                    full_context.push_str(diag);
                }
            }

            let response = crate::qwen::chat_with_tools(
                message,
                if full_context.is_empty() { None } else { Some(&full_context) },
            );
            let resp = ToolUseResponse {
                reply: response.reply,
                tools: response
                    .tools
                    .into_iter()
                    .map(|t| ToolCallInfo {
                        name: t.name,
                        args: t.args,
                        output: t.output,
                        content: t.content,
                    })
                    .collect(),
                error: response.error,
            };
            let mut buf = Vec::new();
            prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
            Ok(buf)
        }

        "get_repos" => {
            let reg_guard = REGISTRY.lock().unwrap();
            if let Some(ref registry) = *reg_guard {
                let repos: Vec<RepoInfo> = registry
                    .list_repos()
                    .into_iter()
                    .map(|r| RepoInfo {
                        path: r.path.clone(),
                        name: r.name.clone(),
                        is_git_repo: r.is_git_repo,
                        git_remote: r.git_remote.clone(),
                        file_count: r.file_count as u32,
                        total_size_bytes: r.total_size_bytes,
                        last_modified: r.last_modified,
                        index_state: Some(repo_index_state_to_proto(&r.index_state)),
                        languages: r.languages.clone(),
                        added_at: r.added_at,
                    })
                    .collect();
                let resp = GetReposResponse {
                    repos: Some(RepoList {
                        repos,
                        active: registry.active_repo().map(String::from),
                    }),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            } else {
                Err("Registry not initialized".into())
            }
        }

        "discover_repos" => {
            let mut reg_guard = REGISTRY.lock().unwrap();
            if let Some(ref mut registry) = *reg_guard {
                println!("[registry] Discovering repositories...");
                let found = registry.discover_repos();
                println!("[registry] Found {} repositories", found.len());
                registry.save();
                let repos: Vec<RepoInfo> = found
                    .into_iter()
                    .map(|r| RepoInfo {
                        path: r.path.clone(),
                        name: r.name.clone(),
                        is_git_repo: r.is_git_repo,
                        git_remote: r.git_remote,
                        file_count: r.file_count as u32,
                        total_size_bytes: r.total_size_bytes,
                        last_modified: r.last_modified,
                        index_state: Some(repo_index_state_to_proto(&r.index_state)),
                        languages: r.languages,
                        added_at: r.added_at,
                    })
                    .collect();
                let resp = DiscoverReposResponse {
                    found: repos.len() as u32,
                    repos: Some(RepoList {
                        repos,
                        active: registry.active_repo().map(String::from),
                    }),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            } else {
                Err("Registry not initialized".into())
            }
        }

        "add_repo" => {
            let json_payload: serde_json::Value =
                serde_json::from_slice(payload).unwrap_or_default();
            let path = json_payload["path"].as_str().unwrap_or("");
            if path.is_empty() {
                Err("Missing 'path' parameter".into())
            } else {
                let mut reg_guard = REGISTRY.lock().unwrap();
                if let Some(ref mut registry) = *reg_guard {
                    match registry.add_repo(path) {
                        Ok(()) => {
                            let resp = AddRepoResponse {
                                status: Some(StatusResponse {
                                    status: "ok".into(),
                                    error: None,
                                    functions: None,
                                    edges: None,
                                    found: None,
                                }),
                                path: path.into(),
                            };
                            let mut buf = Vec::new();
                            prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
                            Ok(buf)
                        }
                        Err(e) => Err(e),
                    }
                } else {
                    Err("Registry not initialized".into())
                }
            }
        }

        "remove_repo" => {
            let json_payload: serde_json::Value =
                serde_json::from_slice(payload).unwrap_or_default();
            let path = json_payload["path"].as_str().unwrap_or("");
            let mut reg_guard = REGISTRY.lock().unwrap();
            if let Some(ref mut registry) = *reg_guard {
                let _removed = registry.remove_repo(path);
                let resp = RemoveRepoResponse {
                    status: Some(StatusResponse {
                        status: "ok".into(),
                        error: None,
                        functions: None,
                        edges: None,
                        found: None,
                    }),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            } else {
                Err("Registry not initialized".into())
            }
        }

        "switch_repo" => {
            let json_payload: serde_json::Value =
                serde_json::from_slice(payload).unwrap_or_default();
            let path = json_payload["path"].as_str().unwrap_or("");
            if path.is_empty() {
                Err("Missing 'path' parameter".into())
            } else {
                {
                    let mut reg_guard = REGISTRY.lock().unwrap();
                    if let Some(ref mut registry) = *reg_guard {
                        if let Err(e) = registry.switch_repo(path) {
                            return Err(e.into());
                        }
                    }
                }
                println!("[server] Switching to {}...", path);
                let new_state = AppState::new(path);
                drop(state_guard);
                {
                    let mut guard = APP_STATE.lock().unwrap();
                    *guard = Some(new_state);
                }
                let (push, functions, edge_count) = {
                    let guard = APP_STATE.lock().unwrap();
                    guard
                        .as_ref()
                        .map(|s| {
                            let graph = s.graph();
                            let f = graph.node_count;
                            let e = graph.edge_count;
                            let msg =
                                WsMessage { msg: Some(ws_message::Msg::GraphData(graph.clone())) };
                            let mut buf = Vec::new();
                            let _ = prost::Message::encode(&msg, &mut buf);
                            (Some(buf), f, e)
                        })
                        .unwrap_or((None, 0, 0))
                };
                if let Some(p) = push {
                    broadcast_proto(&p);
                }
                let resp = SwitchRepoResponse {
                    status: Some(StatusResponse {
                        status: "ok".into(),
                        error: None,
                        functions: Some(functions),
                        edges: Some(edge_count),
                        found: None,
                    }),
                };
                let mut buf = Vec::new();
                prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
                Ok(buf)
            }
        }

        _ => Err(format!("Unknown action: {}", action)),
    };

    let resp = match result {
        Ok(data) => WsResponse { id: request_id, result: Some(ws_response::Result::Data(data)) },
        Err(e) => WsResponse { id: request_id, result: Some(ws_response::Result::Error(e)) },
    };

    let mut buf = Vec::new();
    prost::Message::encode(&resp, &mut buf).map_err(|e| e.to_string())?;
    Ok(String::from_utf8(buf).map_err(|e| e.to_string())?)
}
