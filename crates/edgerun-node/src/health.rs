use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;

/// Shared health state updated by the daemon.
#[derive(Clone, Debug)]
pub struct HealthState {
    pub node_id: String,
    /// Transitional compatibility field while daemon setup is moved from
    /// configured stream ids to node-id-derived event identity.
    pub stream_id: String,
    pub started_at: std::time::Instant,
}

/// Runs a small local HTTP control surface on the given port.
///
/// Routes:
/// - GET /health
/// - GET /protocol/node/status
/// - GET /protocol/apps
/// - GET /protocol/capabilities
/// - GET /protocol/approvals
/// - GET /protocol/approvals/<id>/approve
/// - GET /protocol/approvals/<id>/reject
/// - GET /codelyzer/graph       -> proxies 127.0.0.1:13337/graph
/// - GET /codelyzer/connections -> proxies 127.0.0.1:13337/connections
/// - GET /codelyzer/viewport-command -> proxies codelyzer viewport command endpoint when present
pub async fn run_health_server(port: u16, state: HealthState) {
    use edgerun_rt::{AsyncReadExt, AsyncWriteExt};

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match edgerun_rt::AsyncTcpListener::bind(addr) {
        Ok(l) => l,
        Err(e) => {
            edgerun_log::error!("failed to bind local HTTP endpoint on {}: {}", addr, e);
            return;
        }
    };
    edgerun_log::info!("local HTTP endpoint listening");

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                let state = state.clone();
                edgerun_rt::spawn(async move {
                    let mut buf = [0u8; 4096];
                    let n = stream.read(&mut buf).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&buf[..n]);
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/");

                    let (status, body) = route_local_http(path, &state);
                    let response = format!(
                        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: content-type\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        status,
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.flush().await;
                });
            }
            Err(e) => {
                edgerun_log::warn!("local HTTP endpoint accept error: {}", e);
            }
        }
    }
}

fn route_local_http(path: &str, state: &HealthState) -> (&'static str, String) {
    let clean_path = path.split('?').next().unwrap_or(path);
    if let Some((approval_id, decision)) = parse_approval_decision_path(clean_path) {
        return decide_approval(approval_id, decision);
    }

    match clean_path {
        "/health" | "/" => ("200 OK", health_json(state)),
        "/protocol/node/status" => ("200 OK", node_status_json(state)),
        "/protocol/apps" => ("200 OK", apps_json()),
        "/protocol/capabilities" => ("200 OK", capabilities_json()),
        "/protocol/approvals" => ("200 OK", approvals_json()),
        "/codelyzer/graph" => proxy_codelyzer("/graph"),
        "/codelyzer/connections" => proxy_codelyzer("/connections"),
        "/codelyzer/viewport-command" => proxy_codelyzer("/viewport-command"),
        _ => ("404 Not Found", r#"{"error":"not_found"}"#.to_string()),
    }
}

fn health_json(state: &HealthState) -> String {
    let uptime = state.started_at.elapsed().as_secs();
    format!(
        r#"{{"status":"ok","uptime_secs":{},"node_id":"{}","stream_id":"{}"}}"#,
        uptime,
        escape_json(&state.node_id),
        escape_json(&state.stream_id),
    )
}

fn node_status_json(state: &HealthState) -> String {
    format!(
        r#"{{"nodeId":"{}","identity":"{}","health":"healthy","streamHead":{{"streamId":"{}","lastSeq":0,"lastEventId":""}},"runtimeVersion":"{}","protocolVersions":["0"],"syncStatus":"synced","lastRefresh":"{}"}}"#,
        escape_json(&state.node_id),
        escape_json(&state.node_id),
        escape_json(&state.stream_id),
        env!("CARGO_PKG_VERSION"),
        now_iso_stub(),
    )
}

fn apps_json() -> String {
    r#"{"apps":[{"id":"codelyzer","name":"Codelyzer","status":"available","routes":["/codelyzer/graph","/codelyzer/connections","/codelyzer/viewport-command"]},{"id":"xray","name":"Xray","status":"available"}]}"#.to_string()
}

fn capabilities_json() -> String {
    r#"{"capabilities":[{"id":"codelyzer.graph.read","name":"Read codelyzer graph","status":"granted"},{"id":"codelyzer.viewport.command","name":"Control Xray viewport","status":"granted"},{"id":"repo.text_edit","name":"Text edit repository files","status":"requires_user_approval"},{"id":"repo.rust_ast_edit","name":"AST-safe Rust editing","status":"requires_user_approval"}]}"#.to_string()
}

fn approvals_json() -> String {
    let dir = approval_dir();
    let mut approvals = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Ok(raw) = fs::read_to_string(&path) else { continue };
            let token = extract_json_string(&raw, "token").unwrap_or_else(|| path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown").to_string());
            let approved = dir.join(format!("{token}.approved")).exists();
            let rejected = dir.join(format!("{token}.rejected")).exists();
            if approved || rejected {
                continue;
            }
            let operation = extract_json_string(&raw, "operation").unwrap_or_else(|| "repo_text_edit".to_string());
            let target = extract_json_string(&raw, "target_path").unwrap_or_else(|| "unknown".to_string());
            approvals.push(format!(
                r#"{{"approvalId":"{}","operation":"{}","scope":"repo_text_edit","description":"{} {}","requestedAt":"local","riskLevel":"high","source":"codelyzer-mcp"}}"#,
                escape_json(&token),
                escape_json(&operation),
                escape_json(&operation),
                escape_json(&target),
            ));
        }
    }
    format!(r#"{{"approvals":[{}]}}"#, approvals.join(","))
}

fn parse_approval_decision_path(path: &str) -> Option<(&str, &str)> {
    let rest = path.strip_prefix("/protocol/approvals/")?;
    if let Some(id) = rest.strip_suffix("/approve") {
        return Some((id, "approved"));
    }
    if let Some(id) = rest.strip_suffix("/reject") {
        return Some((id, "rejected"));
    }
    None
}

fn decide_approval(approval_id: &str, decision: &str) -> (&'static str, String) {
    if approval_id.contains('/') || approval_id.contains("..") {
        return ("400 Bad Request", r#"{"error":"invalid_approval_id"}"#.to_string());
    }
    let dir = approval_dir();
    if let Err(err) = fs::create_dir_all(&dir) {
        return ("500 Internal Server Error", format!(r#"{{"error":"{}"}}"#, escape_json(&err.to_string())));
    }
    let marker = dir.join(format!("{approval_id}.{decision}"));
    match fs::write(&marker, b"ok") {
        Ok(_) => ("200 OK", format!(r#"{{"approvalId":"{}","decision":"{}"}}"#, escape_json(approval_id), decision)),
        Err(err) => ("500 Internal Server Error", format!(r#"{{"error":"{}"}}"#, escape_json(&err.to_string()))),
    }
}

fn approval_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(xdg).join("edgerun-codelyzer").join("permissions");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".cache").join("edgerun-codelyzer").join("permissions");
    }
    PathBuf::from("/tmp").join("edgerun-codelyzer").join("permissions")
}

fn proxy_codelyzer(path: &str) -> (&'static str, String) {
    match http_get_local("127.0.0.1:13337", path) {
        Ok(body) => ("200 OK", body),
        Err(err) => (
            "503 Service Unavailable",
            format!(r#"{{"error":"codelyzer_unavailable","detail":"{}"}}"#, escape_json(&err)),
        ),
    }
}

fn http_get_local(addr: &str, path: &str) -> Result<String, String> {
    let mut stream = TcpStream::connect(addr).map_err(|err| err.to_string())?;
    let request = format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", path, addr);
    stream.write_all(request.as_bytes()).map_err(|err| err.to_string())?;
    let mut response = String::new();
    stream.read_to_string(&mut response).map_err(|err| err.to_string())?;
    let body = response
        .split("\r\n\r\n")
        .nth(1)
        .unwrap_or("")
        .to_string();
    Ok(body)
}

fn extract_json_string(raw: &str, key: &str) -> Option<String> {
    let needle = format!("\"{}\"", key);
    let start = raw.find(&needle)?;
    let after_key = &raw[start + needle.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let after_quote = after_colon.strip_prefix('"')?;
    let end = after_quote.find('"')?;
    Some(after_quote[..end].to_string())
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn now_iso_stub() -> &'static str { "local" }
