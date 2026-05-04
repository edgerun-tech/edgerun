use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};

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
    match clean_path {
        "/health" | "/" => ("200 OK", health_json(state)),
        "/protocol/node/status" => ("200 OK", node_status_json(state)),
        "/protocol/apps" => ("200 OK", apps_json()),
        "/protocol/capabilities" => ("200 OK", capabilities_json()),
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
        escape_json_num(uptime),
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

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn escape_json_num(value: u64) -> u64 { value }

fn now_iso_stub() -> &'static str { "local" }
