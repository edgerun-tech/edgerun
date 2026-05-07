use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use edgerun_crypto::sha1::{Digest, Sha1};
use edgerun_encoding::base64::standard_encode;
use edgerun_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};

const WEBSOCKET_GUID: &[u8] = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
const MAX_HTTP_HEADER_BYTES: usize = 16 * 1024;
const MAX_WS_PAYLOAD_BYTES: u64 = 16 * 1024 * 1024;

type ServerResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    headers: HashMap<String, String>,
}

#[derive(Debug)]
struct WsFrame {
    opcode: u8,
    payload: Vec<u8>,
}

fn main() -> ServerResult<()> {
    let addr: SocketAddr = "0.0.0.0:8080".parse()?;
    let listener = TcpListener::bind(addr)?;
    println!("edgerun-term-server listening on http://{addr}");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    if let Err(err) = handle_connection(stream) {
                        eprintln!("connection error: {err}");
                    }
                });
            }
            Err(err) => eprintln!("accept error: {err}"),
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> ServerResult<()> {
    let request = read_http_request(&mut stream)?;

    if request.path == "/ws" {
        handle_websocket(stream, &request.headers)?;
    } else if request.method == "GET" || request.method == "HEAD" {
        serve_static(&mut stream, &request)?;
    } else {
        write_response(
            &mut stream,
            "405 Method Not Allowed",
            "text/plain; charset=utf-8",
            b"method not allowed",
        )?;
    }

    Ok(())
}

fn read_http_request(stream: &mut TcpStream) -> io::Result<HttpRequest> {
    let mut bytes = Vec::new();
    let mut buf = [0u8; 512];

    while !bytes.windows(4).any(|window| window == b"\r\n\r\n") {
        let n = stream.read(&mut buf)?;
        if n == 0 {
            return Err(io::Error::new(
                ErrorKind::UnexpectedEof,
                "connection closed before request headers",
            ));
        }
        bytes.extend_from_slice(&buf[..n]);
        if bytes.len() > MAX_HTTP_HEADER_BYTES {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "request headers exceed limit",
            ));
        }
    }

    let header_text = String::from_utf8_lossy(&bytes);
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "missing request line"))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts
        .next()
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "missing method"))?
        .to_string();
    let path = request_parts
        .next()
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "missing path"))?
        .split('?')
        .next()
        .unwrap_or("/")
        .to_string();

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }

    Ok(HttpRequest {
        method,
        path,
        headers,
    })
}

fn handle_websocket(mut stream: TcpStream, headers: &HashMap<String, String>) -> ServerResult<()> {
    if !is_websocket_upgrade(headers) {
        write_response(
            &mut stream,
            "400 Bad Request",
            "text/plain; charset=utf-8",
            b"expected websocket upgrade",
        )?;
        return Ok(());
    }

    let Some(key) = headers.get("sec-websocket-key") else {
        write_response(
            &mut stream,
            "400 Bad Request",
            "text/plain; charset=utf-8",
            b"missing Sec-WebSocket-Key",
        )?;
        return Ok(());
    };

    let accept = websocket_accept_key(key);
    write!(
        stream,
        "HTTP/1.1 101 Switching Protocols\r\n\
         Upgrade: websocket\r\n\
         Connection: Upgrade\r\n\
         Sec-WebSocket-Accept: {accept}\r\n\
         \r\n"
    )?;
    stream.flush()?;

    run_pty_session(stream)
}

fn run_pty_session(mut stream: TcpStream) -> ServerResult<()> {
    let pty_system = NativePtySystem::default();
    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/bash".to_string());
    let mut cmd = CommandBuilder::new(shell);
    cmd.env("TERM", "xterm-256color");
    let _child = pair.slave.spawn_command(cmd)?;

    let master = pair.master;
    let mut reader = master.try_clone_reader()?;
    let mut writer = master.take_writer()?;
    let write_stream = Arc::new(Mutex::new(stream.try_clone()?));

    let output_stream = Arc::clone(&write_stream);
    thread::spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if write_ws_frame_locked(&output_stream, 0x2, &buf[..n]).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    while let Some(frame) = read_ws_frame(&mut stream)? {
        match frame.opcode {
            0x1 => {
                let text = String::from_utf8_lossy(&frame.payload);
                if let Some((cols, rows)) = parse_resize(&text) {
                    master.resize(PtySize {
                        cols,
                        rows,
                        pixel_width: 0,
                        pixel_height: 0,
                    })?;
                } else {
                    writer.write_all(text.as_bytes())?;
                    writer.flush()?;
                }
            }
            0x2 => {
                writer.write_all(&frame.payload)?;
                writer.flush()?;
            }
            0x8 => {
                let _ = write_ws_frame_locked(&write_stream, 0x8, &[]);
                break;
            }
            0x9 => {
                write_ws_frame_locked(&write_stream, 0xA, &frame.payload)?;
            }
            0xA => {}
            _ => {}
        }
    }

    Ok(())
}

fn is_websocket_upgrade(headers: &HashMap<String, String>) -> bool {
    let upgrade = headers
        .get("upgrade")
        .is_some_and(|value| value.eq_ignore_ascii_case("websocket"));
    let connection = headers.get("connection").is_some_and(|value| {
        value
            .split(',')
            .any(|token| token.trim().eq_ignore_ascii_case("upgrade"))
    });
    upgrade && connection
}

fn websocket_accept_key(client_key: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(client_key.trim().as_bytes());
    hasher.update(WEBSOCKET_GUID);
    let digest = hasher.finalize();
    standard_encode(&digest)
}

fn read_ws_frame(stream: &mut TcpStream) -> io::Result<Option<WsFrame>> {
    let mut header = [0u8; 2];
    if let Err(err) = stream.read_exact(&mut header) {
        return if err.kind() == ErrorKind::UnexpectedEof {
            Ok(None)
        } else {
            Err(err)
        };
    }

    let opcode = header[0] & 0x0F;
    let masked = (header[1] & 0x80) != 0;
    let len_code = header[1] & 0x7F;
    let payload_len = match len_code {
        126 => {
            let mut bytes = [0u8; 2];
            stream.read_exact(&mut bytes)?;
            u16::from_be_bytes(bytes) as u64
        }
        127 => {
            let mut bytes = [0u8; 8];
            stream.read_exact(&mut bytes)?;
            u64::from_be_bytes(bytes)
        }
        n => n as u64,
    };

    if payload_len > MAX_WS_PAYLOAD_BYTES {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "websocket payload exceeds limit",
        ));
    }

    let mut mask = [0u8; 4];
    if masked {
        stream.read_exact(&mut mask)?;
    }

    let mut payload = vec![0u8; payload_len as usize];
    stream.read_exact(&mut payload)?;

    if masked {
        for (idx, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask[idx % 4];
        }
    }

    Ok(Some(WsFrame { opcode, payload }))
}

fn write_ws_frame_locked(
    stream: &Arc<Mutex<TcpStream>>,
    opcode: u8,
    payload: &[u8],
) -> io::Result<()> {
    let mut stream = stream
        .lock()
        .map_err(|_| io::Error::other("websocket write lock poisoned"))?;
    write_ws_frame(&mut stream, opcode, payload)
}

fn write_ws_frame(stream: &mut TcpStream, opcode: u8, payload: &[u8]) -> io::Result<()> {
    let mut header = Vec::with_capacity(10);
    header.push(0x80 | (opcode & 0x0F));
    match payload.len() {
        len @ 0..=125 => header.push(len as u8),
        len @ 126..=65_535 => {
            header.push(126);
            header.extend_from_slice(&(len as u16).to_be_bytes());
        }
        len => {
            header.push(127);
            header.extend_from_slice(&(len as u64).to_be_bytes());
        }
    }
    stream.write_all(&header)?;
    stream.write_all(payload)?;
    stream.flush()
}

fn serve_static(stream: &mut TcpStream, request: &HttpRequest) -> io::Result<()> {
    let Some(path) = static_path_for(&request.path) else {
        return write_response(
            stream,
            "403 Forbidden",
            "text/plain; charset=utf-8",
            b"forbidden",
        );
    };

    let Ok(body) = fs::read(&path) else {
        return write_response(
            stream,
            "404 Not Found",
            "text/plain; charset=utf-8",
            b"not found",
        );
    };

    let body = if request.method == "HEAD" {
        &[][..]
    } else {
        body.as_slice()
    };
    write_response(stream, "200 OK", content_type(&path), body)
}

fn static_path_for(path: &str) -> Option<PathBuf> {
    let rel = if path == "/" || path.is_empty() {
        "index.html"
    } else {
        path.strip_prefix('/')?
    };

    let mut safe = PathBuf::new();
    for component in Path::new(rel).components() {
        match component {
            Component::Normal(part) => safe.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    Some(static_root().join(safe))
}

fn static_root() -> PathBuf {
    let term_local = PathBuf::from("edgerun-term-web");
    if term_local.is_dir() {
        term_local
    } else {
        PathBuf::from("crates/edgerun-term/edgerun-term-web")
    }
}

fn content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("css") => "text/css; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("wasm") => "application/wasm",
        _ => "application/octet-stream",
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: &str,
    content_type: &str,
    body: &[u8],
) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\n\
         Content-Length: {}\r\n\
         Content-Type: {content_type}\r\n\
         Connection: close\r\n\
         \r\n",
        body.len()
    )?;
    stream.write_all(body)?;
    stream.flush()
}

fn parse_resize(text: &str) -> Option<(u16, u16)> {
    let rest = text.strip_prefix("resize:")?;
    let mut parts = rest.split('x');
    let cols = parts.next()?.parse::<u16>().ok()?;
    let rows = parts.next()?.parse::<u16>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((cols, rows))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_rfc_websocket_accept_key() {
        assert_eq!(
            websocket_accept_key("dGhlIHNhbXBsZSBub25jZQ=="),
            "s3pPLMBiTxaQ9kYGzzhZRbK+xOo="
        );
    }

    #[test]
    fn parses_resize_message() {
        assert_eq!(parse_resize("resize:120x33"), Some((120, 33)));
        assert_eq!(parse_resize("resize:120x33x9"), None);
        assert_eq!(parse_resize("hello"), None);
    }

    #[test]
    fn rejects_parent_static_paths() {
        assert!(static_path_for("/../Cargo.toml").is_none());
        assert!(static_path_for("/assets/../index.html").is_none());
        assert!(static_path_for("/index.html").is_some());
    }
}
