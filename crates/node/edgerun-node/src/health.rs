use std::fs;
use std::net::SocketAddr;
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
/// - GET  /health
/// - GET  /protocol/node/status
/// - GET  /protocol/apps
/// - GET  /protocol/capabilities
/// - GET  /protocol/approvals
/// - GET  /protocol/approvals/<id>/approve
/// - GET  /protocol/approvals/<id>/reject
/// - POST /protocol/tools/invoke
pub async fn run_health_server(port: u16, state: HealthState) {
    use edgerun_node::network::{HostSocketTransport, TransportAddress};
    use edgerun_node::rt::{AsyncReadExt, AsyncWriteExt};

    const ACCEPT_ERROR_BACKOFF: core::time::Duration = core::time::Duration::from_millis(100);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = match HostSocketTransport.bind_stream_now(&TransportAddress::host_stream(
        addr.to_string().into_bytes(),
    )) {
        Ok(l) => l,
        Err(e) => {
            crate::node_error!("failed to bind local HTTP endpoint on {}: {}", addr, e);
            return;
        }
    };
    crate::node_info!("local HTTP endpoint listening");

    loop {
        match listener.accept().await {
            Ok((mut stream, _)) => {
                let state = state.clone();
                edgerun_node::rt::spawn(async move {
                    let mut buf = [0u8; 16384];
                    let n = stream.read(&mut buf).await.unwrap_or(0);
                    let request = String::from_utf8_lossy(&buf[..n]);
                    let first = request.lines().next().unwrap_or_default();
                    let mut parts = first.split_whitespace();
                    let method = parts.next().unwrap_or("GET");
                    let path = parts.next().unwrap_or("/");
                    let body = request.split("\r\n\r\n").nth(1).unwrap_or_default();

                    let (status, body) = route_local_http(method, path, body, &state);
                    let response = format!(
                        "HTTP/1.1 {}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: content-type\r\nAccess-Control-Allow-Methods: GET,POST,OPTIONS\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        status,
                        body.len(),
                        body
                    );
                    let _ = stream.write_all(response.as_bytes()).await;
                    let _ = stream.flush().await;
                });
            }
            Err(e) => {
                crate::node_warn!("local HTTP endpoint accept error: {}", e);
                edgerun_node::rt::sleep(ACCEPT_ERROR_BACKOFF).await;
            }
        }
    }
}

fn route_local_http(
    method: &str,
    path: &str,
    body: &str,
    state: &HealthState,
) -> (&'static str, String) {
    if method == "OPTIONS" {
        return ("200 OK", "{}".to_string());
    }

    let clean_path = path.split('?').next().unwrap_or(path);
    if let Some((approval_id, decision)) = parse_approval_decision_path(clean_path) {
        return decide_approval(approval_id, decision);
    }

    match (method, clean_path) {
        ("GET", "/health") | ("GET", "/") => ("200 OK", health_json(state)),
        ("GET", "/protocol/node/status") => ("200 OK", node_status_json(state)),
        ("GET", "/protocol/apps") => ("200 OK", apps_json()),
        ("GET", "/protocol/capabilities") => ("200 OK", capabilities_json()),
        ("GET", "/protocol/approvals") => ("200 OK", approvals_json()),
        ("POST", "/protocol/tools/invoke") => invoke_protocol_tool(body),
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
    r#"{"apps":[{"id":"xray","name":"Xray","status":"available"}]}"#.to_string()
}

fn capabilities_json() -> String {
    r#"{"capabilities":[{"id":"xray_viewport_control","name":"Control Xray viewport","status":"granted"},{"id":"node_connection","name":"Talk to local EdgeRun node","status":"granted"}]}"#.to_string()
}

fn invoke_protocol_tool(body: &str) -> (&'static str, String) {
    let tool_id = extract_json_string(body, "toolId").unwrap_or_default();
    if tool_id.is_empty() {
        return (
            "400 Bad Request",
            r#"{"status":"failed","error":"missing toolId"}"#.to_string(),
        );
    }

    match tool_id.as_str() {
        "xray.viewport.focus_node" => write_viewport_command("focus_node", body),
        "xray.viewport.show_related" => write_viewport_command("show_related", body),
        "xray.viewport.set_camera" => write_viewport_command("set_camera", body),
        _ => (
            "200 OK",
            format!(
                r#"{{"status":"blocked","error":"unknown tool {}"}}"#,
                escape_json(&tool_id)
            ),
        ),
    }
}

fn pending_tool_approval(tool_id: &str, body: &str) -> (&'static str, String) {
    let target = extract_nested_input_string(body, "path")
        .or_else(|| extract_nested_input_string(body, "name"))
        .unwrap_or_else(|| tool_id.to_string());
    let approval_id = stable_token(tool_id, &target);
    let dir = approval_dir();
    if let Err(err) = fs::create_dir_all(&dir) {
        return (
            "500 Internal Server Error",
            format!(
                r#"{{"status":"failed","error":"{}"}}"#,
                escape_json(&err.to_string())
            ),
        );
    }

    let approved_path = dir.join(format!("{approval_id}.approved"));
    if approved_path.exists() {
        return (
            "200 OK",
            format!(
                r#"{{"status":"executed","result":{{"approved":true,"toolId":"{}","note":"approval recorded; backend execution handoff comes next"}},"evidenceRefs":["approval:{}"]}}"#,
                escape_json(tool_id),
                escape_json(&approval_id)
            ),
        );
    }

    let request_path = dir.join(format!("{approval_id}.json"));
    let request = format!(
        r#"{{"kind":"edgerun.tool.approval","token":"{}","operation":"{}","target_path":"{}","tool_request":{}}}"#,
        escape_json(&approval_id),
        escape_json(tool_id),
        escape_json(&target),
        if body.trim().is_empty() {
            "{}"
        } else {
            body.trim()
        },
    );
    if let Err(err) = fs::write(&request_path, request) {
        return (
            "500 Internal Server Error",
            format!(
                r#"{{"status":"failed","error":"{}"}}"#,
                escape_json(&err.to_string())
            ),
        );
    }

    (
        "200 OK",
        format!(
            r#"{{"status":"pending_approval","approvalId":"{}"}}"#,
            escape_json(&approval_id)
        ),
    )
}

fn write_viewport_command(command_type: &str, body: &str) -> (&'static str, String) {
    let dir = local_control_dir();
    if let Err(err) = fs::create_dir_all(&dir) {
        return (
            "500 Internal Server Error",
            format!(
                r#"{{"status":"failed","error":"{}"}}"#,
                escape_json(&err.to_string())
            ),
        );
    }
    let path = dir.join("viewport-command.json");
    let payload = format!(
        r#"{{"type":"{}","request":{},"ts":"{}"}}"#,
        escape_json(command_type),
        if body.trim().is_empty() {
            "{}"
        } else {
            body.trim()
        },
        now_iso_stub(),
    );
    match fs::write(&path, payload) {
        Ok(_) => (
            "200 OK",
            format!(
                r#"{{"status":"executed","result":{{"queued":true,"command":"{}"}}}}"#,
                escape_json(command_type)
            ),
        ),
        Err(err) => (
            "500 Internal Server Error",
            format!(
                r#"{{"status":"failed","error":"{}"}}"#,
                escape_json(&err.to_string())
            ),
        ),
    }
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
            let Ok(raw) = fs::read_to_string(&path) else {
                continue;
            };
            let token = extract_json_string(&raw, "token").unwrap_or_else(|| {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            });
            let approved = dir.join(format!("{token}.approved")).exists();
            let rejected = dir.join(format!("{token}.rejected")).exists();
            if approved || rejected {
                continue;
            }
            let operation = extract_json_string(&raw, "operation")
                .unwrap_or_else(|| "repo_text_edit".to_string());
            let target =
                extract_json_string(&raw, "target_path").unwrap_or_else(|| "unknown".to_string());
            let scope = if operation.contains("rust_ast") {
                "repo_rust_ast_edit"
            } else {
                "repo_text_edit"
            };
            let risk = if operation.contains("text") {
                "critical"
            } else {
                "high"
            };
            approvals.push(format!(
                r#"{{"approvalId":"{}","operation":"{}","scope":"{}","description":"{} {}","requestedAt":"local","riskLevel":"{}","source":"node-tool"}}"#,
                escape_json(&token),
                escape_json(&operation),
                scope,
                escape_json(&operation),
                escape_json(&target),
                risk,
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
        return (
            "400 Bad Request",
            r#"{"error":"invalid_approval_id"}"#.to_string(),
        );
    }
    let dir = approval_dir();
    if let Err(err) = fs::create_dir_all(&dir) {
        return (
            "500 Internal Server Error",
            format!(r#"{{"error":"{}"}}"#, escape_json(&err.to_string())),
        );
    }
    let marker = dir.join(format!("{approval_id}.{decision}"));
    match fs::write(&marker, b"ok") {
        Ok(_) => (
            "200 OK",
            format!(
                r#"{{"approvalId":"{}","decision":"{}"}}"#,
                escape_json(approval_id),
                decision
            ),
        ),
        Err(err) => (
            "500 Internal Server Error",
            format!(r#"{{"error":"{}"}}"#, escape_json(&err.to_string())),
        ),
    }
}

fn approval_dir() -> PathBuf {
    local_control_dir().join("permissions")
}

fn local_control_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        return PathBuf::from(xdg).join("edgerun-node-control");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".cache")
            .join("edgerun-node-control");
    }
    PathBuf::from("/tmp").join("edgerun-node-control")
}

fn extract_nested_input_string(raw: &str, key: &str) -> Option<String> {
    let input_start = raw.find("\"input\"")?;
    extract_json_string(&raw[input_start..], key)
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

fn stable_token(operation: &str, target: &str) -> String {
    let input = format!("{}\0{}", operation, target);
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in input.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("approval-{hash:016x}")
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

fn now_iso_stub() -> &'static str {
    "local"
}
