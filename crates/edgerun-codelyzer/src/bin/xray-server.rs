//! Minimal codelyzer -> Xray bridge.
//!
//! Serves repository analysis and local network observations as rkyv bytes.
//!
//! Usage:
//!   cargo run -p edgerun-codelyzer --bin xray-server -- /path/to/repo
//!   cargo run -p edgerun-codelyzer --bin xray-server -- /path/to/repo --listen 127.0.0.1:13337
//!
//! Endpoints:
//!   GET /graph
//!   GET /graph?path=/path/to/repo
//!   GET /graph?rescan=1
//!   GET /connections
//!   GET /health

use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{Read, Write},
    net::{Ipv4Addr, Ipv6Addr, TcpListener, TcpStream},
    path::Path,
    time::Instant,
};

use edgerun_codelyzer::{analyzer::analyze_full, filesystem, WIRE_PROTOCOL};
use rkyv::{Archive, Deserialize as RkyvDeserialize, Serialize as RkyvSerialize};

#[derive(Debug, Clone, PartialEq, Eq)]
struct SnapshotFingerprint {
    file_count: usize,
    total_bytes: u64,
    combined_hash: u64,
    max_modified_ts: u64,
}

#[derive(Default)]
struct GraphCache {
    entries: HashMap<String, CachedGraph>,
}

struct CachedGraph {
    fingerprint: SnapshotFingerprint,
    body: Vec<u8>,
}

#[derive(Debug, Archive, RkyvSerialize, RkyvDeserialize)]
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

#[derive(Debug, Archive, RkyvSerialize, RkyvDeserialize)]
struct XrayGraphEdge {
    source: String,
    target: String,
    kind: String,
    tags: Vec<String>,
}

#[derive(Debug, Archive, RkyvSerialize, RkyvDeserialize)]
struct RepoFileEntry {
    path: String,
    language: String,
    size: u64,
    modified_ts: u64,
}

#[derive(Debug, Archive, RkyvSerialize, RkyvDeserialize)]
struct XrayGraphData {
    nodes: Vec<XrayGraphNode>,
    edges: Vec<XrayGraphEdge>,
    files: Vec<RepoFileEntry>,
    total_bytes: u64,
    node_count: u32,
    edge_count: u32,
    file_count: u32,
    elapsed_ms: u64,
    source: String,
    cache_status: String,
}

#[derive(Debug, Archive, RkyvSerialize, RkyvDeserialize)]
enum XrayEnvelopeKind {
    GraphUpdate,
}

#[derive(Debug, Archive, RkyvSerialize, RkyvDeserialize)]
struct GraphUpdateEnvelope {
    kind: XrayEnvelopeKind,
    data: XrayGraphData,
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

#[derive(Debug, Archive, RkyvSerialize, RkyvDeserialize)]
struct LocalConnection {
    network_transport: String,
    local_address: String,
    local_port: u16,
    remote_address: String,
    remote_port: u16,
    state: String,
}

#[derive(Debug, Archive, RkyvSerialize, RkyvDeserialize)]
struct ConnectionsEnvelope {
    connections: Vec<LocalConnection>,
    connection_count: u32,
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
    let mut graph_cache = GraphCache::default();
    println!("[xray-server] listening on http://{}", listen);
    println!("[xray-server] default repo: {}", root);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_client(stream, &root, &mut graph_cache),
            Err(err) => eprintln!("[xray-server] connection error: {err}"),
        }
    }
}

fn print_usage(program: &str) {
    eprintln!("Usage: {program} <repo-path> [--listen 127.0.0.1:13337]");
}

fn handle_client(mut stream: TcpStream, default_root: &str, graph_cache: &mut GraphCache) {
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
        respond_rkyv_bytes(&mut stream, 200, encode_health());
        return;
    }

    if path.starts_with("/connections") {
        let connections = read_local_connections();
        let data = ConnectionsEnvelope {
            connection_count: connections.len() as u32,
            connections,
        };
        respond_rkyv_bytes(&mut stream, 200, encode_connections(&data));
        return;
    }

    if !path.starts_with("/graph") {
        respond_rkyv_bytes(&mut stream, 404, encode_error("not found", None));
        return;
    }

    let root = query_param(path, "path").unwrap_or_else(|| default_root.to_string());
    let force_rescan = query_param(path, "rescan").is_some();
    if !Path::new(&root).is_dir() {
        respond_rkyv_bytes(&mut stream, 400, encode_error("path is not a directory", Some(root)));
        return;
    }

    match graph_response_for_root(&root, graph_cache, force_rescan) {
        Ok(body) => respond_rkyv_bytes(&mut stream, 200, Ok(body)),
        Err(err) => respond_rkyv_bytes(&mut stream, 500, encode_error(&err, None)),
    }
}

fn graph_response_for_root(root: &str, cache: &mut GraphCache, force_rescan: bool) -> Result<Vec<u8>, String> {
    let snapshot = filesystem::scan_dir(root);
    let fingerprint = fingerprint_snapshot(&snapshot);

    if !force_rescan {
        if let Some(entry) = cache.entries.get(root) {
            if entry.fingerprint == fingerprint {
                return Ok(entry.body.clone());
            }
        }
    }

    let mut data = analyze_repo_for_xray(root, snapshot)?;
    data.cache_status = if force_rescan { "forced_rescan" } else { "miss" }.to_string();
    let envelope = GraphUpdateEnvelope { kind: XrayEnvelopeKind::GraphUpdate, data };
    let body = rkyv::to_bytes::<rkyv::rancor::Error>(&envelope)
        .map(|bytes| bytes.to_vec())
        .map_err(|err| err.to_string())?;

    cache.entries.insert(root.to_string(), CachedGraph { fingerprint, body: body.clone() });
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

fn analyze_repo_for_xray(root: &str, snapshot: Vec<filesystem::FileInfo>) -> Result<XrayGraphData, String> {
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

    let mut files: Vec<RepoFileEntry> = snapshot
        .iter()
        .map(|file| RepoFileEntry {
            path: file.path.clone(),
            language: file.language.clone(),
            size: file.size,
            modified_ts: file.modified_ts,
        })
        .collect();
    files.sort_by(|a, b| a.path.cmp(&b.path));

    let total_bytes = snapshot.iter().map(|file| file.size).sum();

    Ok(XrayGraphData {
        node_count: nodes.len() as u32,
        edge_count: edges.len() as u32,
        file_count: files.len() as u32,
        nodes,
        edges,
        files,
        total_bytes,
        elapsed_ms: started.elapsed().as_millis() as u64,
        source: root.to_string(),
        cache_status: "miss".to_string(),
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

    let mut seen = HashSet::new();
    connections.retain(|conn| {
        let key = format!("{}:{}:{}:{}:{}", conn.network_transport, conn.local_address, conn.local_port, conn.remote_address, conn.remote_port);
        seen.insert(key)
    });
    connections.sort_by(|a, b| a.remote_address.cmp(&b.remote_address).then(a.remote_port.cmp(&b.remote_port)));
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
        .filter(|conn| conn.remote_address != "0.0.0.0" && conn.remote_address != "::" && conn.remote_port != 0)
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

fn respond_rkyv_bytes(stream: &mut TcpStream, status: u16, body: Result<Vec<u8>, rkyv::rancor::Error>) {
    let body = body.unwrap_or_else(|err| {
        rkyv::to_bytes::<rkyv::rancor::Error>(&XrayError {
            error: format!("rkyv encode error: {err}"),
            path: None,
        })
        .map(|bytes| bytes.to_vec())
        .unwrap_or_default()
    });
    let status_text = match status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };

    let response = format!(
        "HTTP/1.1 {status} {status_text}\r\nContent-Type: application/octet-stream\r\nX-Edgerun-Wire-Protocol: {WIRE_PROTOCOL}\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: content-type,x-edgerun-wire-protocol\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(&body);
}

fn encode_health() -> Result<Vec<u8>, rkyv::rancor::Error> {
    rkyv::to_bytes::<rkyv::rancor::Error>(&XrayHealth { ok: true }).map(|bytes| bytes.to_vec())
}

fn encode_connections(data: &ConnectionsEnvelope) -> Result<Vec<u8>, rkyv::rancor::Error> {
    rkyv::to_bytes::<rkyv::rancor::Error>(data).map(|bytes| bytes.to_vec())
}

fn encode_error(message: &str, path: Option<String>) -> Result<Vec<u8>, rkyv::rancor::Error> {
    rkyv::to_bytes::<rkyv::rancor::Error>(&XrayError {
        error: message.to_string(),
        path,
    })
    .map(|bytes| bytes.to_vec())
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
