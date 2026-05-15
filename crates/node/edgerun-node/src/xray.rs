//! Node-owned Xray bridge for local repository graph data.
//!
//! This is intentionally feature-gated. It serves rkyv archive payloads over
//! HTTP for local dashboard tooling; it is not part of the default node surface.

use alloc::boxed::Box;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::future::Future;
use core::pin::Pin;
use std::eprintln;
use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use edgerun_codelyzer::analyzer::analyze_full;
use edgerun_codelyzer::filesystem;
use edgerun_codelyzer::generated::codeanalyzer::{
    GraphData, GraphEdge, GraphNode, WsMessage, WsMessageEnum,
};
use edgerun_codelyzer::xray_wire::{ConnectionsEnvelope, LocalConnection};
use edgerun_encoding::percent;
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

use crate::http::{Handler, Request, Response, StatusCode};

const DEFAULT_LISTEN: &str = "127.0.0.1:13337";
const WIRE_PROTOCOL: &str = edgerun_codelyzer::WIRE_PROTOCOL;

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotFingerprint {
    file_count: usize,
    total_bytes: u64,
    combined_hash: u64,
    max_modified_ts: u64,
}

#[derive(Default)]
struct GraphCache {
    entries: BTreeMap<String, CachedGraph>,
}

struct CachedGraph {
    fingerprint: SnapshotFingerprint,
    body: Vec<u8>,
}

#[derive(Debug, Archive, RkyvSerialize, RkyvDeserialize)]
struct XrayHealth {
    ok: bool,
}

#[derive(Debug, Archive, RkyvSerialize, RkyvDeserialize)]
struct XrayError {
    error: String,
    path: Option<String>,
}

#[derive(Clone)]
pub struct XrayBridgeConfig {
    pub root: String,
    pub listen: String,
}

impl XrayBridgeConfig {
    pub fn new(root: impl Into<String>, listen: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            listen: listen.into(),
        }
    }
}

pub fn cmd_xray_server(root: String, listen: Option<String>) {
    if !Path::new(&root).is_dir() {
        eprintln!("error: '{root}' is not a directory");
        std::process::exit(1);
    }

    let config = XrayBridgeConfig::new(root, listen.unwrap_or_else(|| DEFAULT_LISTEN.to_string()));
    let rt = crate::rt::Runtime::new_multi_thread()
        .enable_all()
        .build()
        .unwrap_or_else(|error| {
            eprintln!("error: runtime build failed: {error}");
            std::process::exit(1);
        });

    let result = rt.block_on(run_xray_server(config));
    if let Err(error) = result {
        eprintln!("error: xray server failed: {error}");
        std::process::exit(1);
    }
}

pub async fn run_xray_server(config: XrayBridgeConfig) -> Result<(), String> {
    let handler = XrayBridgeHandler::new(config.root.clone());
    let server = crate::http::HttpServer::new(handler)
        .bind(config.listen.as_str())
        .await
        .map_err(|error| format!("bind {}: {error}", config.listen))?;

    eprintln!("[edged:xray] listening on http://{}", server.local_addr());
    eprintln!("[edged:xray] default repo: {}", config.root);
    server
        .serve()
        .await
        .map_err(|error| format!("serve: {error}"))
}

#[derive(Clone)]
struct XrayBridgeHandler {
    default_root: String,
    cache: Arc<Mutex<GraphCache>>,
}

impl XrayBridgeHandler {
    fn new(default_root: String) -> Self {
        Self {
            default_root,
            cache: Arc::new(Mutex::new(GraphCache::default())),
        }
    }
}

impl Handler for XrayBridgeHandler {
    fn handle(&self, request: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
        Box::pin(async move { self.handle_sync(request) })
    }
}

impl XrayBridgeHandler {
    fn handle_sync(&self, request: Request) -> Response {
        if request.method().as_str() == "OPTIONS" {
            return rkyv_response(200, Vec::new());
        }

        let path = request.uri().path();
        match path {
            "/health" => rkyv_response_result(200, encode_health()),
            "/connections" => {
                let connections = read_local_connections();
                let data = ConnectionsEnvelope {
                    connection_count: connections.len() as u32,
                    connections,
                };
                rkyv_response_result(200, encode_connections(&data))
            }
            "/graph" => {
                let root = request
                    .uri()
                    .query()
                    .and_then(|query| query_param(query, "path"))
                    .unwrap_or_else(|| self.default_root.clone());
                let force_rescan = request
                    .uri()
                    .query()
                    .and_then(|query| query_param(query, "rescan"))
                    .is_some();
                if !Path::new(&root).is_dir() {
                    return rkyv_response_result(
                        400,
                        encode_error("path is not a directory", Some(root)),
                    );
                }

                let mut cache = match self.cache.lock() {
                    Ok(cache) => cache,
                    Err(_) => {
                        return rkyv_response_result(
                            500,
                            encode_error("cache lock poisoned", None),
                        );
                    }
                };
                match graph_response_for_root(&root, &mut cache, force_rescan) {
                    Ok(body) => rkyv_response(200, body),
                    Err(error) => rkyv_response_result(500, encode_error(&error, None)),
                }
            }
            _ => rkyv_response_result(404, encode_error("not found", None)),
        }
    }
}

fn graph_response_for_root(
    root: &str,
    cache: &mut GraphCache,
    force_rescan: bool,
) -> Result<Vec<u8>, String> {
    let snapshot = filesystem::scan_dir(root);
    let fingerprint = fingerprint_snapshot(&snapshot);

    if !force_rescan {
        if let Some(entry) = cache.entries.get(root) {
            if entry.fingerprint == fingerprint {
                return Ok(entry.body.clone());
            }
        }
    }

    let data = analyze_repo_for_xray(root, &snapshot)?;
    let message = WsMessage {
        msg: Some(WsMessageEnum::GraphData(data)),
    };
    let body = message.encode_rkyv().map_err(|error| error.to_string())?;

    cache.entries.insert(
        root.to_string(),
        CachedGraph {
            fingerprint,
            body: body.clone(),
        },
    );
    Ok(body)
}

fn fingerprint_snapshot(snapshot: &[filesystem::FileInfo]) -> SnapshotFingerprint {
    let mut combined_hash = 0xcbf29ce484222325u64;
    let mut total_bytes = 0u64;
    let mut max_modified_ts = 0u64;

    for file in snapshot {
        total_bytes = total_bytes.saturating_add(file.size);
        max_modified_ts = max_modified_ts.max(file.modified_ts);
        combined_hash ^= file.hash;
        combined_hash = combined_hash.wrapping_mul(0x100000001b3);
        combined_hash ^= file.size;
        combined_hash = combined_hash.wrapping_mul(0x100000001b3);
        combined_hash ^= file.modified_ts;
        combined_hash = combined_hash.wrapping_mul(0x100000001b3);
        for byte in file.path.as_bytes() {
            combined_hash ^= *byte as u64;
            combined_hash = combined_hash.wrapping_mul(0x100000001b3);
        }
    }

    SnapshotFingerprint {
        file_count: snapshot.len(),
        total_bytes,
        combined_hash,
        max_modified_ts,
    }
}

fn analyze_repo_for_xray(
    root: &str,
    snapshot: &[filesystem::FileInfo],
) -> Result<GraphData, String> {
    let started = Instant::now();
    let (result, _changes) = analyze_full(root);
    let program = result.program;

    let mut connectivity: BTreeMap<String, u32> = BTreeMap::new();
    for edge in &program.edges {
        *connectivity.entry(edge.caller.clone()).or_default() += 1;
        *connectivity.entry(edge.callee.clone()).or_default() += 1;
    }

    let mut nodes: Vec<GraphNode> = program
        .functions_ordered()
        .into_iter()
        .map(|func| GraphNode {
            id: func.id.clone(),
            name: func.name.clone(),
            file: func.file.clone(),
            language: func.language.clone(),
            is_static: func.is_static,
            connections: connectivity.get(&func.id).copied().unwrap_or(0),
            tags: tags_for_function(func),
            commit: func.last_modified_commit.clone(),
        })
        .collect();

    let mut edges: Vec<GraphEdge> = program
        .edges
        .iter()
        .map(|edge| GraphEdge {
            source: edge.caller.clone(),
            target: edge.callee.clone(),
            kind: edge.kind.label().to_string(),
        })
        .collect();

    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    edges.sort_by(|a, b| a.source.cmp(&b.source).then(a.target.cmp(&b.target)));

    let _elapsed_ms = started.elapsed().as_millis() as u64;
    let total_bytes = snapshot.iter().map(|file| file.size).sum();

    Ok(GraphData {
        node_count: nodes.len() as u32,
        edge_count: edges.len() as u32,
        nodes,
        edges,
        tag_groups: Vec::new(),
        total_bytes,
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

fn read_local_connections() -> Vec<LocalConnection> {
    let mut connections = Vec::new();
    connections.extend(read_proc_net("/proc/net/tcp", "tcp", false));
    connections.extend(read_proc_net("/proc/net/tcp6", "tcp6", true));
    connections.extend(read_proc_net("/proc/net/udp", "udp", false));
    connections.extend(read_proc_net("/proc/net/udp6", "udp6", true));

    let mut seen = BTreeSet::new();
    connections.retain(|conn| {
        let key = format!(
            "{}:{}:{}:{}:{}",
            conn.network_transport,
            conn.local_address,
            conn.local_port,
            conn.remote_address,
            conn.remote_port
        );
        seen.insert(key)
    });
    connections.sort_by(|a, b| {
        a.remote_address
            .cmp(&b.remote_address)
            .then(a.remote_port.cmp(&b.remote_port))
    });
    connections
}

fn read_proc_net(path: &str, protocol: &str, ipv6: bool) -> Vec<LocalConnection> {
    let Ok(content) = fs::read_to_string(path) else {
        return Vec::new();
    };

    content
        .lines()
        .skip(1)
        .filter_map(|line| parse_proc_net_line(line, protocol, ipv6))
        .filter(|conn| {
            conn.remote_address != "0.0.0.0" && conn.remote_address != "::" && conn.remote_port != 0
        })
        .collect()
}

fn parse_proc_net_line(line: &str, protocol: &str, ipv6: bool) -> Option<LocalConnection> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let local = fields.get(1)?;
    let remote = fields.get(2)?;
    let state = fields.get(3).copied().unwrap_or("00");
    let (local_address, local_port) = parse_socket_addr(local, ipv6)?;
    let (remote_address, remote_port) = parse_socket_addr(remote, ipv6)?;

    Some(LocalConnection {
        network_transport: protocol.to_string(),
        local_address,
        local_port,
        remote_address,
        remote_port,
        state: tcp_state(state).to_string(),
    })
}

fn parse_socket_addr(value: &str, ipv6: bool) -> Option<(String, u16)> {
    let (addr_hex, port_hex) = value.split_once(':')?;
    let port = u16::from_str_radix(port_hex, 16).ok()?;
    let address = if ipv6 {
        parse_ipv6_hex(addr_hex)?
    } else {
        parse_ipv4_hex(addr_hex)?
    };
    Some((address, port))
}

fn parse_ipv4_hex(hex: &str) -> Option<String> {
    let raw = u32::from_str_radix(hex, 16).ok()?;
    Some(Ipv4Addr::from(raw.to_le_bytes()).to_string())
}

fn parse_ipv6_hex(hex: &str) -> Option<String> {
    if hex.len() != 32 {
        return None;
    }
    let mut bytes = [0u8; 16];
    for i in 0..4 {
        let chunk = u32::from_str_radix(&hex[i * 8..i * 8 + 8], 16).ok()?;
        bytes[i * 4..i * 4 + 4].copy_from_slice(&chunk.to_le_bytes());
    }
    Some(Ipv6Addr::from(bytes).to_string())
}

fn tcp_state(hex: &str) -> &'static str {
    match hex {
        "01" => "established",
        "02" => "syn_sent",
        "03" => "syn_recv",
        "04" => "fin_wait1",
        "05" => "fin_wait2",
        "06" => "time_wait",
        "07" => "closed",
        "08" => "close_wait",
        "09" => "last_ack",
        "0A" => "listen",
        "0B" => "closing",
        _ => "unknown",
    }
}

fn rkyv_response_result(status: u16, body: Result<Vec<u8>, rkyv::rancor::Error>) -> Response {
    let body = body.unwrap_or_else(|error| {
        rkyv::to_bytes::<rkyv::rancor::Error>(&XrayError {
            error: format!("rkyv encode error: {error}"),
            path: None,
        })
        .map(|bytes| bytes.to_vec())
        .unwrap_or_default()
    });
    rkyv_response(status, body)
}

fn rkyv_response(status: u16, body: Vec<u8>) -> Response {
    Response::new(StatusCode::new(status).unwrap_or_else(|_| StatusCode::new(500).unwrap()))
        .with_header("content-type", "application/octet-stream")
        .with_header("x-edgerun-wire-protocol", WIRE_PROTOCOL)
        .with_header("access-control-allow-origin", "*")
        .with_header(
            "access-control-allow-headers",
            "content-type,x-edgerun-wire-protocol",
        )
        .with_header("access-control-allow-methods", "GET,OPTIONS")
        .with_body(body)
}

fn encode_health() -> Result<Vec<u8>, rkyv::rancor::Error> {
    rkyv::to_bytes::<rkyv::rancor::Error>(&XrayHealth { ok: true }).map(|bytes| bytes.to_vec())
}

fn encode_connections(data: &ConnectionsEnvelope) -> Result<Vec<u8>, rkyv::rancor::Error> {
    data.encode_rkyv()
}

fn encode_error(message: &str, path: Option<String>) -> Result<Vec<u8>, rkyv::rancor::Error> {
    rkyv::to_bytes::<rkyv::rancor::Error>(&XrayError {
        error: message.to_string(),
        path,
    })
    .map(|bytes| bytes.to_vec())
}

fn query_param(query: &str, key: &str) -> Option<String> {
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k == key {
            return Some(percent::percent_decode(v));
        }
    }
    None
}
