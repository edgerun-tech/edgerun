use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::network::{HostSocketTransport, TransportAddress};
use crate::rt::{
    self, AsyncReadExt, AsyncTcpListener, AsyncTcpStream, AsyncWriteExt, CancellationToken,
};
use edgerun_crypto::p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};

#[cfg(target_os = "none")]
use crate::rt::io;
#[cfg(not(target_os = "none"))]
use std::io;
#[cfg(not(target_os = "none"))]
use std::{
    collections::BTreeMap,
    fs::{self, OpenOptions},
    io::Write,
    sync::{Mutex, OnceLock},
};

#[cfg(not(target_os = "none"))]
const NODE_RELAY_DIRECTORY_PATH: &str = "/var/lib/edgerun/.edgerun/node-relay-directory.jsonl";

#[cfg(not(target_os = "none"))]
static NODE_RELAY_DIRECTORY: OnceLock<Mutex<BTreeMap<String, NodeRelayRecord>>> = OnceLock::new();

#[cfg(not(target_os = "none"))]
#[derive(Clone, Debug)]
struct NodeRelayRecord {
    node_id: String,
    public_key_raw_base64: String,
    reachable_target: String,
    updated_at_iso: String,
    expires_at_iso: String,
    protocols_json: String,
    nonce: String,
    signature_base64: String,
}

pub struct HttpNodeBinding {
    listener: Arc<AsyncTcpListener>,
    target_app_id: [u8; 32],
}

impl HttpNodeBinding {
    pub fn bind(addr: &str, target_app_id: [u8; 32]) -> io::Result<Self> {
        let listener = HostSocketTransport
            .bind_stream_now(&TransportAddress::host_stream(addr.as_bytes().to_vec()))
            .map_err(|error| io::Error::new(io::ErrorKind::Other, error.to_string()))?;
        Ok(Self {
            listener: Arc::new(listener),
            target_app_id,
        })
    }

    pub async fn run(self, shutdown: CancellationToken) -> io::Result<()> {
        while !shutdown.is_cancelled() {
            match rt::timeout(
                core::time::Duration::from_millis(100),
                self.listener.accept(),
            )
            .await
            {
                Ok(Ok((stream, _peer))) => {
                    let target_app_id = self.target_app_id;
                    rt::spawn(async move {
                        if let Err(error) = dispatch_to_app(stream, target_app_id).await {
                            crate::node_warn!("http app dispatch failed: {}", error);
                        }
                    });
                }
                Ok(Err(error)) => return Err(io_error(error)),
                Err(_) => {}
            }
        }
        Ok(())
    }
}

async fn dispatch_to_app(
    mut stream: Arc<AsyncTcpStream>,
    target_app_id: [u8; 32],
) -> io::Result<()> {
    let mut buffer = [0u8; 8192];
    let n = stream.read(&mut buffer).await.map_err(io_error)?;
    let request = core::str::from_utf8(&buffer[..n]).unwrap_or("");
    let path = request_path(request);
    #[cfg(not(target_os = "none"))]
    if request_method(request) == Some("GET")
        && path == Some("/work")
        && crate::services::work_websocket::is_websocket_upgrade(request)
    {
        return crate::services::work_websocket::serve_work_websocket(stream, buffer[..n].to_vec())
            .await;
    }
    crate::node_info!(
        "http request accepted for app {:02x}{:02x}{:02x}{:02x}",
        target_app_id[0],
        target_app_id[1],
        target_app_id[2],
        target_app_id[3]
    );
    let response = match (request_method(request), path) {
        (Some("OPTIONS"), Some("/nodes/update")) => cors_preflight_response(),
        (Some("POST"), Some("/nodes/update")) => node_relay_update_response(request),
        (Some("GET"), Some(path)) if path.starts_with("/nodes/") => {
            node_relay_lookup_response(path)
        }
        (_, Some("/health")) => response(
            "200 OK",
            "application/json",
            "{\"status\":\"ok\",\"service\":\"edgerun-server\"}\n",
        ),
        (_, Some("/")) => response(
            "200 OK",
            "text/plain; charset=utf-8",
            "EdgeRun node online\n",
        ),
        _ => response(
            "503 Service Unavailable",
            "text/plain; charset=utf-8",
            "app ipc http dispatch is not wired yet\n",
        ),
    };
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(io_error)
}

fn request_path(request: &str) -> Option<&str> {
    let mut parts = request.lines().next()?.split_whitespace();
    let _method = parts.next()?;
    parts.next()
}

fn request_method(request: &str) -> Option<&str> {
    request.lines().next()?.split_whitespace().next()
}

fn response(status: &str, content_type: &str, body: &str) -> String {
    let mut out = String::new();
    out.push_str("HTTP/1.1 ");
    out.push_str(status);
    out.push_str("\r\nContent-Type: ");
    out.push_str(content_type);
    out.push_str("\r\nContent-Length: ");
    out.push_str(&body.as_bytes().len().to_string());
    out.push_str("\r\nConnection: close\r\n");
    out.push_str("Access-Control-Allow-Origin: https://dash.edgerun.tech\r\n");
    out.push_str("Access-Control-Allow-Methods: GET, POST, OPTIONS\r\n");
    out.push_str("Access-Control-Allow-Headers: content-type\r\n");
    out.push_str("Vary: Origin\r\n\r\n");
    out.push_str(body);
    out
}

fn cors_preflight_response() -> String {
    response("204 No Content", "application/json", "")
}

#[cfg(target_os = "none")]
fn node_relay_update_response(_request: &str) -> String {
    response(
        "501 Not Implemented",
        "application/json",
        "{\"error\":\"node relay directory requires std storage\"}\n",
    )
}

#[cfg(target_os = "none")]
fn node_relay_lookup_response(_path: &str) -> String {
    response(
        "501 Not Implemented",
        "application/json",
        "{\"error\":\"node relay directory requires std storage\"}\n",
    )
}

#[cfg(not(target_os = "none"))]
fn node_relay_update_response(request: &str) -> String {
    let Some(body) = request.split("\r\n\r\n").nth(1) else {
        return json_error("400 Bad Request", "missing request body");
    };
    let envelope_tape = match edgerun_json::parse_json_tape(body) {
        Ok(value) => value,
        Err(_) => return json_error("400 Bad Request", "invalid JSON envelope"),
    };
    let Some(envelope) = envelope_tape.root(body) else {
        return json_error("400 Bad Request", "invalid JSON envelope");
    };
    let signed_payload = match envelope.required_str("signedPayload") {
        Ok(value) => value,
        Err(_) => return json_error("400 Bad Request", "missing signedPayload"),
    };
    let signature_base64 = match envelope.required_str("signatureBase64") {
        Ok(value) => value,
        Err(_) => return json_error("400 Bad Request", "missing signatureBase64"),
    };
    let payload_tape = match edgerun_json::parse_json_tape(signed_payload) {
        Ok(value) => value,
        Err(_) => return json_error("400 Bad Request", "invalid signedPayload JSON"),
    };
    let Some(payload) = payload_tape.root(signed_payload) else {
        return json_error("400 Bad Request", "invalid signedPayload JSON");
    };
    let record = match relay_record_from_payload(payload, signature_base64) {
        Ok(record) => record,
        Err(error) => return json_error("400 Bad Request", error),
    };
    if let Err(error) = verify_relay_record_signature(&record, signed_payload.as_bytes()) {
        return json_error("403 Forbidden", error);
    }

    let directory = relay_directory();
    if let Ok(mut records) = directory.lock() {
        records.insert(record.node_id.clone(), record.clone());
    } else {
        return json_error("500 Internal Server Error", "relay directory lock failed");
    }
    persist_relay_record(&record);
    let relay_host = node_relay_host(&record.node_id);
    response(
        "200 OK",
        "application/json",
        &format!(
            "{{\"ok\":true,\"nodeId\":\"{}\",\"relayHost\":\"{}\",\"updatedAtIso\":\"{}\",\"expiresAtIso\":\"{}\"}}\n",
            json_escape(&record.node_id),
            json_escape(&relay_host),
            json_escape(&record.updated_at_iso),
            json_escape(&record.expires_at_iso)
        ),
    )
}

#[cfg(not(target_os = "none"))]
fn node_relay_lookup_response(path: &str) -> String {
    let node_id = path.trim_start_matches("/nodes/").trim_matches('/');
    if node_id.is_empty() || !is_node_id(node_id) {
        return json_error("400 Bad Request", "invalid node id");
    }
    let directory = relay_directory();
    let records = match directory.lock() {
        Ok(records) => records,
        Err(_) => return json_error("500 Internal Server Error", "relay directory lock failed"),
    };
    let Some(record) = records.get(node_id) else {
        return json_error("404 Not Found", "node route not found");
    };
    let relay_host = node_relay_host(&record.node_id);
    response(
        "200 OK",
        "application/json",
        &format!(
            "{{\"version\":1,\"nodeId\":\"{}\",\"relayHost\":\"{}\",\"reachable\":true,\"updatedAtIso\":\"{}\",\"expiresAtIso\":\"{}\",\"protocols\":{},\"signedUpdate\":{{\"publicKeyRawBase64\":\"{}\",\"nonce\":\"{}\",\"signatureBase64\":\"{}\"}}}}\n",
            json_escape(&record.node_id),
            json_escape(&relay_host),
            json_escape(&record.updated_at_iso),
            json_escape(&record.expires_at_iso),
            record.protocols_json,
            json_escape(&record.public_key_raw_base64),
            json_escape(&record.nonce),
            json_escape(&record.signature_base64)
        ),
    )
}

#[cfg(not(target_os = "none"))]
fn relay_record_from_payload(
    payload: edgerun_json::TapeValue<'_>,
    signature_base64: &str,
) -> Result<NodeRelayRecord, &'static str> {
    let version = payload
        .required_u64("version")
        .map_err(|_| "missing version")?;
    if version != 1 {
        return Err("unsupported relay update version");
    }
    let node_id = payload
        .required_str("nodeId")
        .map_err(|_| "missing nodeId")?
        .to_ascii_lowercase();
    if !is_node_id(&node_id) {
        return Err("invalid nodeId");
    }
    let public_key_raw_base64 = payload
        .required_str("publicKeyRawBase64")
        .map_err(|_| "missing publicKeyRawBase64")?
        .to_string();
    let reachable_target = payload
        .required_str("reachableTarget")
        .map_err(|_| "missing reachableTarget")?
        .to_string();
    if reachable_target.len() > 256 || reachable_target.contains('\n') {
        return Err("invalid reachableTarget");
    }
    let updated_at_iso = payload
        .required_str("updatedAtIso")
        .map_err(|_| "missing updatedAtIso")?
        .to_string();
    let expires_at_iso = payload
        .required_str("expiresAtIso")
        .map_err(|_| "missing expiresAtIso")?
        .to_string();
    let nonce = payload
        .required_str("nonce")
        .map_err(|_| "missing nonce")?
        .to_string();
    let protocols_json = payload
        .get("protocols")
        .and_then(|value| value.to_json_value())
        .and_then(|value| value.to_json_string().ok())
        .unwrap_or_else(|| "[]".to_string());
    Ok(NodeRelayRecord {
        node_id,
        public_key_raw_base64,
        reachable_target,
        updated_at_iso,
        expires_at_iso,
        protocols_json,
        nonce,
        signature_base64: signature_base64.to_string(),
    })
}

#[cfg(not(target_os = "none"))]
fn verify_relay_record_signature(
    record: &NodeRelayRecord,
    signed_payload: &[u8],
) -> Result<(), &'static str> {
    let public_key = base64_decode(&record.public_key_raw_base64).ok_or("invalid public key")?;
    if public_key.len() != 65 || public_key.first().copied() != Some(0x04) {
        return Err("public key must be uncompressed P-256 SEC1");
    }
    if hex_lower(&public_key[1..]) != record.node_id {
        return Err("nodeId does not match public key identity");
    }
    let signature = base64_decode(&record.signature_base64).ok_or("invalid signature")?;
    let verifying_key =
        VerifyingKey::from_sec1_bytes(&public_key).map_err(|_| "invalid P-256 public key")?;
    let signature =
        Signature::try_from(signature.as_slice()).map_err(|_| "invalid ECDSA signature")?;
    verifying_key
        .verify(signed_payload, &signature)
        .map_err(|_| "relay update signature verification failed")
}

#[cfg(not(target_os = "none"))]
fn relay_directory() -> &'static Mutex<BTreeMap<String, NodeRelayRecord>> {
    NODE_RELAY_DIRECTORY.get_or_init(|| Mutex::new(load_relay_directory()))
}

#[cfg(not(target_os = "none"))]
fn load_relay_directory() -> BTreeMap<String, NodeRelayRecord> {
    let mut records = BTreeMap::new();
    let Ok(contents) = fs::read_to_string(NODE_RELAY_DIRECTORY_PATH) else {
        return records;
    };
    for line in contents.lines() {
        let Ok(tape) = edgerun_json::parse_json_tape(line) else {
            continue;
        };
        let Some(value) = tape.root(line) else {
            continue;
        };
        let Ok(record) = persisted_relay_record(value) else {
            continue;
        };
        records.insert(record.node_id.clone(), record);
    }
    records
}

#[cfg(not(target_os = "none"))]
fn persisted_relay_record(value: edgerun_json::TapeValue<'_>) -> Result<NodeRelayRecord, ()> {
    Ok(NodeRelayRecord {
        node_id: value.required_str("nodeId").map_err(|_| ())?.to_string(),
        public_key_raw_base64: value
            .required_str("publicKeyRawBase64")
            .map_err(|_| ())?
            .to_string(),
        reachable_target: value
            .required_str("reachableTarget")
            .map_err(|_| ())?
            .to_string(),
        updated_at_iso: value
            .required_str("updatedAtIso")
            .map_err(|_| ())?
            .to_string(),
        expires_at_iso: value
            .required_str("expiresAtIso")
            .map_err(|_| ())?
            .to_string(),
        protocols_json: value
            .get("protocols")
            .and_then(|value| value.to_json_value())
            .and_then(|value| value.to_json_string().ok())
            .unwrap_or_else(|| "[]".to_string()),
        nonce: value.required_str("nonce").map_err(|_| ())?.to_string(),
        signature_base64: value
            .required_str("signatureBase64")
            .map_err(|_| ())?
            .to_string(),
    })
}

#[cfg(not(target_os = "none"))]
fn persist_relay_record(record: &NodeRelayRecord) {
    if let Some(parent) = std::path::Path::new(NODE_RELAY_DIRECTORY_PATH).parent() {
        let _ = fs::create_dir_all(parent);
    }
    let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(NODE_RELAY_DIRECTORY_PATH)
    else {
        return;
    };
    let _ = writeln!(
        file,
        "{{\"nodeId\":\"{}\",\"publicKeyRawBase64\":\"{}\",\"reachableTarget\":\"{}\",\"updatedAtIso\":\"{}\",\"expiresAtIso\":\"{}\",\"protocols\":{},\"nonce\":\"{}\",\"signatureBase64\":\"{}\"}}",
        json_escape(&record.node_id),
        json_escape(&record.public_key_raw_base64),
        json_escape(&record.reachable_target),
        json_escape(&record.updated_at_iso),
        json_escape(&record.expires_at_iso),
        record.protocols_json,
        json_escape(&record.nonce),
        json_escape(&record.signature_base64)
    );
}

#[cfg(not(target_os = "none"))]
fn json_error(status: &str, message: &str) -> String {
    response(
        status,
        "application/json",
        &format!("{{\"error\":\"{}\"}}\n", json_escape(message)),
    )
}

#[cfg(not(target_os = "none"))]
fn is_node_id(value: &str) -> bool {
    value.len() == 128 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

#[cfg(not(target_os = "none"))]
fn node_relay_host(node_id: &str) -> String {
    let _ = node_id;
    "nodes.edgerun.tech".to_string()
}

#[cfg(not(target_os = "none"))]
fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(not(target_os = "none"))]
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut block = [0u8; 4];
    let mut len = 0;
    for byte in input.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => 64,
            _ => return None,
        };
        block[len] = value;
        len += 1;
        if len == 4 {
            if block[0] == 64 || block[1] == 64 {
                return None;
            }
            out.push((block[0] << 2) | (block[1] >> 4));
            if block[2] != 64 {
                out.push((block[1] << 4) | (block[2] >> 2));
            }
            if block[3] != 64 {
                out.push((block[2] << 6) | block[3]);
            }
            len = 0;
        }
    }
    if len != 0 {
        while len < 4 {
            block[len] = 64;
            len += 1;
        }
        if block[0] == 64 || block[1] == 64 {
            return None;
        }
        out.push((block[0] << 2) | (block[1] >> 4));
        if block[2] != 64 {
            out.push((block[1] << 4) | (block[2] >> 2));
        }
        if block[3] != 64 {
            out.push((block[2] << 6) | block[3]);
        }
    }
    Some(out)
}

fn io_error(error: rt::IoError) -> io::Error {
    match error {
        rt::IoError::UnexpectedEof => io::Error::new(io::ErrorKind::UnexpectedEof, error),
        rt::IoError::WriteZero => io::Error::new(io::ErrorKind::WriteZero, error),
        rt::IoError::Other(_) => io::Error::new(io::ErrorKind::Other, error),
    }
}
