// SPDX-License-Identifier: GPL-2.0-only
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use edgerun_storage::virtual_fs::{
    LogIngestEntry, SourceImportRequestV1, StorageBackedVirtualFs, VfsModeV1, VfsSourceKindV1,
};
use edgerun_storage::StorageError;
use prost::Message;
use serde_json::{json, Value};

#[derive(Clone)]
struct DriverConfig {
    data_dir: PathBuf,
    repo_id: String,
    branch_id: String,
    declared_by: String,
    partition_prefix: String,
    batch_lines: usize,
    ensure_log_source: bool,
    request_max_bytes: usize,
    max_stream_buffer_bytes: usize,
}

struct SessionHandle {
    stop_tx: mpsc::Sender<()>,
    join: JoinHandle<()>,
}

#[derive(Debug, Clone)]
struct StartInfo {
    file: String,
    container_id: String,
    container_name: String,
}

#[derive(Debug, Clone, Copy)]
enum StreamMode {
    Auto,
    ProtobufFramed,
    LineDelimited,
}

#[derive(Debug, Clone)]
struct ParsedLogRecord {
    stream: String,
    ts_unix_ms: u64,
    message: String,
}

#[derive(Clone, PartialEq, Message)]
struct DockerLogEntryWire {
    #[prost(bytes = "vec", tag = "1")]
    line: Vec<u8>,
    #[prost(int64, tag = "2")]
    time_nano: i64,
    #[prost(string, tag = "3")]
    source: String,
    #[prost(bool, tag = "4")]
    partial: bool,
    #[prost(bytes = "vec", tag = "5")]
    partial_log_metadata: Vec<u8>,
}

fn usage() {
    eprintln!(
        "Usage: docker_log_driver --data-dir PATH --repo-id ID --branch ID --socket-path PATH [--declared-by ID] [--partition-prefix PREFIX] [--batch-lines N] [--ensure-log-source] [--request-max-bytes N] [--max-stream-buffer-bytes N]"
    );
}

fn main() {
    let env_data_dir = std::env::var("DATA_DIR").unwrap_or_default();
    let env_socket_path = std::env::var("SOCKET_PATH").unwrap_or_default();
    let mut cfg = DriverConfig {
        data_dir: if env_data_dir.trim().is_empty() {
            PathBuf::new()
        } else {
            PathBuf::from(env_data_dir)
        },
        repo_id: std::env::var("REPO_ID").unwrap_or_default(),
        branch_id: std::env::var("BRANCH").unwrap_or_default(),
        declared_by: std::env::var("DECLARED_BY")
            .unwrap_or_else(|_| "docker_log_driver".to_string()),
        partition_prefix: std::env::var("PARTITION_PREFIX")
            .unwrap_or_else(|_| "docker".to_string()),
        batch_lines: env_usize("BATCH_LINES", 1000).max(1),
        ensure_log_source: env_truthy("ENSURE_LOG_SOURCE"),
        request_max_bytes: env_usize("REQUEST_MAX_BYTES", 1024 * 1024).max(1024),
        max_stream_buffer_bytes: env_usize("MAX_STREAM_BUFFER_BYTES", 8 * 1024 * 1024).max(4096),
    };
    let mut socket_path: Option<PathBuf> = if env_socket_path.trim().is_empty() {
        None
    } else {
        Some(PathBuf::from(env_socket_path))
    };

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--data-dir" => cfg.data_dir = args.next().map(PathBuf::from).unwrap_or_default(),
            "--repo-id" => cfg.repo_id = args.next().unwrap_or_default(),
            "--branch" => cfg.branch_id = args.next().unwrap_or_default(),
            "--socket-path" => socket_path = args.next().map(PathBuf::from),
            "--declared-by" => {
                cfg.declared_by = args.next().unwrap_or_else(|| cfg.declared_by.clone())
            }
            "--partition-prefix" => {
                cfg.partition_prefix = args.next().unwrap_or_else(|| cfg.partition_prefix.clone())
            }
            "--batch-lines" => {
                let raw = args.next().unwrap_or_else(|| "1000".to_string());
                cfg.batch_lines = raw.parse::<usize>().unwrap_or(1000).max(1);
            }
            "--ensure-log-source" => cfg.ensure_log_source = true,
            "--request-max-bytes" => {
                let raw = args.next().unwrap_or_else(|| "1048576".to_string());
                cfg.request_max_bytes = raw.parse::<usize>().unwrap_or(1024 * 1024).max(1024);
            }
            "--max-stream-buffer-bytes" => {
                let raw = args.next().unwrap_or_else(|| "8388608".to_string());
                cfg.max_stream_buffer_bytes = raw.parse::<usize>().unwrap_or(8 * 1024 * 1024);
                cfg.max_stream_buffer_bytes = cfg.max_stream_buffer_bytes.max(4096);
            }
            "--help" | "-h" => {
                usage();
                return;
            }
            _ => {
                eprintln!("unknown arg: {arg}");
                usage();
                std::process::exit(2);
            }
        }
    }

    if cfg.data_dir.as_os_str().is_empty()
        || cfg.repo_id.trim().is_empty()
        || cfg.branch_id.trim().is_empty()
        || socket_path.is_none()
    {
        usage();
        std::process::exit(2);
    }

    if let Err(e) = std::fs::create_dir_all(&cfg.data_dir) {
        eprintln!("failed to create data dir: {e}");
        std::process::exit(1);
    }

    if cfg.ensure_log_source {
        match StorageBackedVirtualFs::open_writer(cfg.data_dir.clone(), &cfg.repo_id) {
            Ok(mut vfs) => {
                let req = SourceImportRequestV1 {
                    schema_version: 1,
                    repo_id: cfg.repo_id.clone(),
                    source_kind: VfsSourceKindV1::VfsSourceKindLogStream as i32,
                    mode: VfsModeV1::VfsModeLog as i32,
                    source_locator: "docker://log-driver".to_string(),
                    source_ref: "stream".to_string(),
                    initiated_by: cfg.declared_by.clone(),
                };
                let _ = vfs.import_source(&req, 0, 0, vec![0u8; 32], "{}".to_string());
            }
            Err(e) => {
                eprintln!("failed to open vfs for ensure-log-source: {e}");
                std::process::exit(1);
            }
        }
    }

    let socket_path = socket_path.expect("validated socket path");
    if socket_path.exists() {
        let _ = std::fs::remove_file(&socket_path);
    }

    let listener = match UnixListener::bind(&socket_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("failed to bind socket {}: {e}", socket_path.display());
            std::process::exit(1);
        }
    };

    let sessions: Arc<Mutex<HashMap<String, SessionHandle>>> = Arc::new(Mutex::new(HashMap::new()));

    println!("docker_log_driver_socket={}", socket_path.display());
    loop {
        let (stream, _) = match listener.accept() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("accept error: {e}");
                continue;
            }
        };

        if let Err(e) = handle_connection(stream, &cfg, &sessions) {
            eprintln!("request handling error: {e}");
        }
    }
}

fn handle_connection(
    mut stream: UnixStream,
    cfg: &DriverConfig,
    sessions: &Arc<Mutex<HashMap<String, SessionHandle>>>,
) -> Result<(), StorageError> {
    reap_finished_sessions(sessions)?;

    let (method, path, body) = read_http_request(&mut stream, cfg.request_max_bytes)
        .map_err(|e| StorageError::InvalidData(format!("invalid http request: {e}")))?;

    if method != "POST" {
        write_json_response(&mut stream, 405, &json!({"Err":"method not allowed"}))?;
        return Ok(());
    }

    match path.as_str() {
        "/Plugin.Activate" => {
            write_json_response(&mut stream, 200, &json!({"Implements":["LogDriver"]}))?;
        }
        "/LogDriver.Capabilities" => {
            write_json_response(&mut stream, 200, &json!({"Cap":{"ReadLogs":false}}))?;
        }
        "/LogDriver.StartLogging" => {
            let start = parse_start_logging(&body)?;
            let mut guard = sessions
                .lock()
                .map_err(|_| StorageError::InvalidData("session lock poisoned".to_string()))?;
            if guard.contains_key(&start.file) {
                write_json_response(&mut stream, 200, &json!({}))?;
                return Ok(());
            }

            let (stop_tx, stop_rx) = mpsc::channel::<()>();
            let worker_cfg = cfg.clone();
            let file_key = start.file.clone();
            let join = thread::spawn(move || run_logging_worker(worker_cfg, start, stop_rx));
            guard.insert(file_key, SessionHandle { stop_tx, join });
            write_json_response(&mut stream, 200, &json!({}))?;
        }
        "/LogDriver.StopLogging" => {
            let file = parse_stop_logging(&body)?;
            let maybe = {
                let mut guard = sessions
                    .lock()
                    .map_err(|_| StorageError::InvalidData("session lock poisoned".to_string()))?;
                guard.remove(&file)
            };
            if let Some(session) = maybe {
                let _ = session.stop_tx.send(());
                let _ = session.join.join();
            }
            write_json_response(&mut stream, 200, &json!({}))?;
        }
        _ => {
            write_json_response(&mut stream, 404, &json!({"Err":"not found"}))?;
        }
    }

    Ok(())
}

fn reap_finished_sessions(
    sessions: &Arc<Mutex<HashMap<String, SessionHandle>>>,
) -> Result<(), StorageError> {
    let finished_keys = {
        let guard = sessions
            .lock()
            .map_err(|_| StorageError::InvalidData("session lock poisoned".to_string()))?;
        guard
            .iter()
            .filter_map(|(k, v)| {
                if v.join.is_finished() {
                    Some(k.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };

    if finished_keys.is_empty() {
        return Ok(());
    }

    let drained = {
        let mut guard = sessions
            .lock()
            .map_err(|_| StorageError::InvalidData("session lock poisoned".to_string()))?;
        let mut out = Vec::with_capacity(finished_keys.len());
        for k in finished_keys {
            if let Some(s) = guard.remove(&k) {
                out.push(s);
            }
        }
        out
    };

    for session in drained {
        let _ = session.join.join();
    }
    Ok(())
}

fn run_logging_worker(cfg: DriverConfig, start: StartInfo, stop_rx: Receiver<()>) {
    let mut vfs = match StorageBackedVirtualFs::open_writer(cfg.data_dir.clone(), &cfg.repo_id) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("worker open vfs failed (file={}): {e}", start.file);
            return;
        }
    };

    let partition = format!(
        "{}.{}",
        sanitize_component(&cfg.partition_prefix),
        sanitize_component(preferred_container_name(
            &start.container_name,
            &start.container_id
        ))
    );

    let mut f = match OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK)
        .open(&start.file)
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("worker open log stream failed ({}): {e}", start.file);
            return;
        }
    };

    let mut mode = StreamMode::Auto;
    let mut buffer = Vec::<u8>::with_capacity(64 * 1024);
    let mut pending = Vec::<LogIngestEntry>::with_capacity(cfg.batch_lines);

    loop {
        match stop_rx.try_recv() {
            Ok(()) => break,
            Err(TryRecvError::Disconnected) => break,
            Err(TryRecvError::Empty) => {}
        }

        let mut tmp = [0u8; 8192];
        match f.read(&mut tmp) {
            Ok(0) => {
                flush_pending(&mut vfs, &cfg, &partition, &mut pending);
                thread::sleep(Duration::from_millis(15));
            }
            Ok(n) => {
                buffer.extend_from_slice(&tmp[..n]);
                if buffer.len() > cfg.max_stream_buffer_bytes {
                    eprintln!(
                        "worker stream buffer exceeded {} bytes for {}, dropping buffered data",
                        cfg.max_stream_buffer_bytes, start.file
                    );
                    buffer.clear();
                }
                drain_buffer(
                    &mut mode,
                    &mut buffer,
                    &start,
                    &mut pending,
                    cfg.batch_lines,
                    &mut vfs,
                    &cfg,
                    &partition,
                );
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                flush_pending(&mut vfs, &cfg, &partition, &mut pending);
                thread::sleep(Duration::from_millis(10));
            }
            Err(e) => {
                eprintln!("worker read error ({}): {e}", start.file);
                break;
            }
        }
    }

    flush_pending(&mut vfs, &cfg, &partition, &mut pending);
}

#[allow(clippy::too_many_arguments)]
fn drain_buffer(
    mode: &mut StreamMode,
    buffer: &mut Vec<u8>,
    start: &StartInfo,
    pending: &mut Vec<LogIngestEntry>,
    batch_lines: usize,
    vfs: &mut StorageBackedVirtualFs,
    cfg: &DriverConfig,
    partition: &str,
) {
    loop {
        let next = match mode {
            StreamMode::Auto => {
                if let Some(rec) = try_decode_framed_entry(buffer) {
                    *mode = StreamMode::ProtobufFramed;
                    Some(rec)
                } else if find_newline(buffer).is_some() {
                    *mode = StreamMode::LineDelimited;
                    try_decode_line_entry(buffer)
                } else {
                    None
                }
            }
            StreamMode::ProtobufFramed => try_decode_framed_entry(buffer),
            StreamMode::LineDelimited => try_decode_line_entry(buffer),
        };

        let Some(record) = next else {
            break;
        };

        pending.push(to_ingest_entry(start, record));
        if pending.len() >= batch_lines {
            flush_pending(vfs, cfg, partition, pending);
        }
    }
}

fn flush_pending(
    vfs: &mut StorageBackedVirtualFs,
    cfg: &DriverConfig,
    partition: &str,
    pending: &mut Vec<LogIngestEntry>,
) {
    if pending.is_empty() {
        return;
    }
    let result = vfs.ingest_log_entries(&cfg.branch_id, partition, pending, &cfg.declared_by);
    if let Err(e) = result {
        eprintln!("worker ingest failed (partition={}): {e}", partition);
    }
    pending.clear();
}

fn to_ingest_entry(start: &StartInfo, record: ParsedLogRecord) -> LogIngestEntry {
    let canonical = format!(
        "{}|{}|{}|{}|{}",
        start.container_id, start.container_name, record.stream, record.ts_unix_ms, record.message
    );
    let idempotency_key = hex::encode(blake3::hash(canonical.as_bytes()).as_bytes());
    let entry_key = format!(
        "{}:{}:{}:{}",
        start.container_id, record.stream, record.ts_unix_ms, idempotency_key
    );
    let payload = format!(
        "container_id={}\ncontainer_name={}\nstream={}\nts_unix_ms={}\nmessage={}\n",
        start.container_id, start.container_name, record.stream, record.ts_unix_ms, record.message
    )
    .into_bytes();

    LogIngestEntry {
        entry_key,
        entry_payload: payload,
        idempotency_key,
        offset: None,
    }
}

fn try_decode_framed_entry(buffer: &mut Vec<u8>) -> Option<ParsedLogRecord> {
    let (frame_len, prefix_len) = match decode_uvarint_prefix(buffer) {
        VarIntPrefix::NeedMore => return None,
        VarIntPrefix::Invalid => return None,
        VarIntPrefix::Ok(v) => v,
    };
    let total = prefix_len.saturating_add(frame_len as usize);
    if buffer.len() < total {
        return None;
    }

    let frame = buffer[prefix_len..total].to_vec();
    buffer.drain(..total);

    let wire = match DockerLogEntryWire::decode(frame.as_slice()) {
        Ok(v) => v,
        Err(_) => return None,
    };

    let message = String::from_utf8_lossy(&wire.line)
        .trim_end_matches('\n')
        .to_string();
    let ts_unix_ms = if wire.time_nano > 0 {
        (wire.time_nano as u64) / 1_000_000
    } else {
        now_unix_ms()
    };
    let stream = if wire.source.trim().is_empty() {
        "stdout".to_string()
    } else {
        wire.source
    };

    Some(ParsedLogRecord {
        stream,
        ts_unix_ms,
        message,
    })
}

fn try_decode_line_entry(buffer: &mut Vec<u8>) -> Option<ParsedLogRecord> {
    let line_end = find_newline(buffer)?;
    let line = buffer[..line_end].to_vec();
    buffer.drain(..=line_end);
    let message = String::from_utf8_lossy(&line)
        .trim_end_matches('\r')
        .to_string();
    Some(ParsedLogRecord {
        stream: "stdout".to_string(),
        ts_unix_ms: now_unix_ms(),
        message,
    })
}

fn find_newline(buf: &[u8]) -> Option<usize> {
    buf.iter().position(|b| *b == b'\n')
}

enum VarIntPrefix {
    Ok((u64, usize)),
    NeedMore,
    Invalid,
}

fn decode_uvarint_prefix(data: &[u8]) -> VarIntPrefix {
    let mut x = 0u64;
    let mut s = 0u32;
    for (i, b) in data.iter().copied().enumerate() {
        if b < 0x80 {
            if i > 9 || (i == 9 && b > 1) {
                return VarIntPrefix::Invalid;
            }
            x |= (b as u64) << s;
            return VarIntPrefix::Ok((x, i + 1));
        }
        x |= ((b & 0x7F) as u64) << s;
        s += 7;
    }
    VarIntPrefix::NeedMore
}

fn parse_start_logging(body: &[u8]) -> Result<StartInfo, StorageError> {
    let v: Value = serde_json::from_slice(body)
        .map_err(|e| StorageError::InvalidData(format!("invalid StartLogging JSON: {e}")))?;
    let file = v
        .get("File")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let info = v.get("Info").unwrap_or(&Value::Null);
    let container_id = info
        .get("ContainerID")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let container_name = info
        .get("ContainerName")
        .and_then(Value::as_str)
        .map(strip_container_name)
        .unwrap_or_else(|| container_id.clone());

    if file.trim().is_empty() {
        return Err(StorageError::InvalidData(
            "StartLogging missing File".to_string(),
        ));
    }
    if container_id.trim().is_empty() {
        return Err(StorageError::InvalidData(
            "StartLogging missing Info.ContainerID".to_string(),
        ));
    }

    Ok(StartInfo {
        file,
        container_id,
        container_name,
    })
}

fn parse_stop_logging(body: &[u8]) -> Result<String, StorageError> {
    let v: Value = serde_json::from_slice(body)
        .map_err(|e| StorageError::InvalidData(format!("invalid StopLogging JSON: {e}")))?;
    let file = v
        .get("File")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if file.trim().is_empty() {
        return Err(StorageError::InvalidData(
            "StopLogging missing File".to_string(),
        ));
    }
    Ok(file)
}

fn strip_container_name(raw: &str) -> String {
    raw.trim_start_matches('/').trim().to_string()
}

fn preferred_container_name<'a>(name: &'a str, id: &'a str) -> &'a str {
    if name.trim().is_empty() {
        id
    } else {
        name
    }
}

fn sanitize_component(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return "unknown".to_string();
    }
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unknown".to_string()
    } else {
        out
    }
}

fn read_http_request(
    stream: &mut UnixStream,
    max_body: usize,
) -> Result<(String, String, Vec<u8>), std::io::Error> {
    let mut head = Vec::<u8>::with_capacity(4096);
    let mut one = [0u8; 1];
    let mut header_done = false;
    while head.len() < 64 * 1024 {
        let n = stream.read(&mut one)?;
        if n == 0 {
            break;
        }
        head.push(one[0]);
        if head.ends_with(b"\r\n\r\n") {
            header_done = true;
            break;
        }
    }
    if !header_done {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "incomplete request headers",
        ));
    }

    let head_str = String::from_utf8_lossy(&head);
    let mut lines = head_str.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or_default().to_string();

    let mut content_length: usize = 0;
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            if k.eq_ignore_ascii_case("content-length") {
                content_length = v.trim().parse::<usize>().unwrap_or(0);
            }
        }
    }

    if content_length > max_body {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "request body too large",
        ));
    }

    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        stream.read_exact(&mut body)?;
    }

    Ok((method, path, body))
}

fn write_json_response(
    stream: &mut UnixStream,
    status: u16,
    payload: &Value,
) -> Result<(), StorageError> {
    let body = serde_json::to_vec(payload)
        .map_err(|e| StorageError::InvalidData(format!("json encode error: {e}")))?;
    let status_line = match status {
        200 => "HTTP/1.1 200 OK",
        404 => "HTTP/1.1 404 Not Found",
        405 => "HTTP/1.1 405 Method Not Allowed",
        _ => "HTTP/1.1 500 Internal Server Error",
    };
    let header = format!(
        "{status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(&body)?;
    stream.flush()?;
    Ok(())
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn env_usize(key: &str, default: usize) -> usize {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(default)
}

fn env_truthy(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .map(|v| {
            let t = v.trim().to_ascii_lowercase();
            t == "1" || t == "true" || t == "yes" || t == "on"
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_varint(mut value: u64) -> Vec<u8> {
        let mut out = Vec::new();
        while value >= 0x80 {
            out.push((value as u8) | 0x80);
            value >>= 7;
        }
        out.push(value as u8);
        out
    }

    #[test]
    fn decode_varint_prefix_roundtrip() {
        let n = 300u64;
        let enc = encode_varint(n);
        match decode_uvarint_prefix(&enc) {
            VarIntPrefix::Ok((val, used)) => {
                assert_eq!(val, n);
                assert_eq!(used, enc.len());
            }
            _ => panic!("expected ok"),
        }
    }

    #[test]
    fn decode_framed_entry_parses_wire_payload() {
        let msg = DockerLogEntryWire {
            line: b"hello\n".to_vec(),
            time_nano: 2_000_000,
            source: "stderr".to_string(),
            partial: false,
            partial_log_metadata: Vec::new(),
        };
        let mut body = msg.encode_to_vec();
        let mut buf = encode_varint(body.len() as u64);
        buf.append(&mut body);

        let rec = try_decode_framed_entry(&mut buf).expect("record");
        assert_eq!(rec.message, "hello");
        assert_eq!(rec.ts_unix_ms, 2);
        assert_eq!(rec.stream, "stderr");
        assert!(buf.is_empty());
    }

    #[test]
    fn start_logging_json_parses_container_fields() {
        let body = br#"{"File":"/tmp/fifo","Info":{"ContainerID":"abc","ContainerName":"/web"}}"#;
        let start = parse_start_logging(body).expect("parse start");
        assert_eq!(start.file, "/tmp/fifo");
        assert_eq!(start.container_id, "abc");
        assert_eq!(start.container_name, "web");
    }
}
