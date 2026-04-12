//! Unified HTTP server supporting HTTP/1.1 and HTTP/2, with optional TLS 1.3.
//!
//! Auto-detects protocol on each connection:
//! - HTTP/2 prior knowledge: starts with `PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n`
//! - HTTP/1.x: starts with a request line like `GET / HTTP/1.1`
//!
//! When TLS is enabled, performs a TLS 1.3 handshake first, then runs the
//! same protocol detection on the decrypted stream.

use crate::handler::Handler;
use crate::header::HeaderMap;
use crate::http1::BufReader;
use crate::http2::frame::{Frame, FrameType, flags};
use crate::http2::hpack::{Decoder, Encoder};
use crate::http2::headers::{validate_header_name_case, validate_request_headers};
use crate::http2::server::{FrameAction, Http2Server};
use crate::http2::ErrorCode;
use crate::method::Method;
use crate::uri::Uri;
use crate::{Request, Response, StatusCode};
use edgerun_rt::{AsyncRead, AsyncReadExt, AsyncTcpListener, AsyncWrite, AsyncWriteExt, sleep, spawn, timeout};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

/// TLS certificate for the server.
pub type TlsCertificate = edgerun_tls::certificate_gen::CertificateAndKey;

/// HTTP/2 connection preface (RFC 9113 §3.4).
const H2_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

/// Unified HTTP server.
pub struct HttpServer {
    handler: Arc<dyn Handler>,
    keep_alive: Option<Duration>,
    max_request_size: usize,
    tls_cert: Option<Arc<TlsCertificate>>,
}

impl HttpServer {
    pub fn new<H: Handler>(handler: H) -> Self {
        Self { handler: Arc::new(handler), keep_alive: Some(Duration::from_secs(5)), max_request_size: 10 * 1024 * 1024, tls_cert: None }
    }
    pub fn keep_alive(mut self, t: Option<Duration>) -> Self { self.keep_alive = t; self }
    pub fn max_request_size(mut self, s: usize) -> Self { self.max_request_size = s; self }

    /// Enable TLS 1.3 with the given certificate.
    pub fn with_tls(mut self, cert: TlsCertificate) -> Self {
        self.tls_cert = Some(Arc::new(cert));
        self
    }

    pub async fn bind(self, addr: impl std::net::ToSocketAddrs) -> std::io::Result<BoundHttpServer> {
        let listener = AsyncTcpListener::bind(addr)?;
        let local_addr = listener.local_addr()?;
        Ok(BoundHttpServer { listener, handler: self.handler, keep_alive: self.keep_alive, max_request_size: self.max_request_size, local_addr, tls_cert: self.tls_cert })
    }
}

pub struct BoundHttpServer {
    listener: AsyncTcpListener,
    handler: Arc<dyn Handler>,
    keep_alive: Option<Duration>,
    max_request_size: usize,
    local_addr: SocketAddr,
    tls_cert: Option<Arc<TlsCertificate>>,
}

impl BoundHttpServer {
    pub fn local_addr(&self) -> SocketAddr { self.local_addr }

    pub async fn serve(&self) -> std::io::Result<()> {
        let tls = self.tls_cert.is_some();
        edgerun_log::info!("HTTP server listening on {} (HTTP/1.1 + HTTP/2{})", self.local_addr, if tls { " + TLS" } else { "" });
        loop {
            match self.listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let handler = Arc::clone(&self.handler);
                    let keep_alive = self.keep_alive;
                    let max_size = self.max_request_size;
                    let tls_cert = self.tls_cert.clone();
                    spawn(async move {
                        if let Err(e) = handle_connection(stream, handler, keep_alive, max_size, tls_cert).await {
                            edgerun_log::warn!("Connection error from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => { edgerun_log::error!("Accept error: {}", e); sleep(Duration::from_millis(100)).await; }
            }
        }
    }

    pub async fn accept_one(&self) -> std::io::Result<()> {
        let (stream, _) = self.listener.accept().await?;
        let handler = Arc::clone(&self.handler);
        handle_connection(stream, handler, self.keep_alive, self.max_request_size, self.tls_cert.clone()).await
    }
}

/// Handle a single connection — optionally does TLS handshake, then auto-detects HTTP/1.1 vs HTTP/2.
async fn handle_connection<S>(stream: S, handler: Arc<dyn Handler>, keep_alive: Option<Duration>, max_request_size: usize, tls_cert: Option<Arc<TlsCertificate>>) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    if let Some(ref cert) = tls_cert {
        use edgerun_tls::async_tls::AsyncTlsServerStream;
        let tls_stream = AsyncTlsServerStream::accept(stream, cert.as_ref())
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("TLS handshake failed: {e}")))?;
        handle_connection_inner(tls_stream, handler, keep_alive, max_request_size).await
    } else {
        handle_connection_inner(stream, handler, keep_alive, max_request_size).await
    }
}

/// Inner connection handler — reads first line to detect HTTP/1.1 vs HTTP/2.
async fn handle_connection_inner<S>(stream: S, handler: Arc<dyn Handler>, keep_alive: Option<Duration>, max_request_size: usize) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut reader = BufReader::new(stream);
    let first_line = match timeout(Duration::from_secs(5), reader.read_line()).await {
        Ok(Ok(Some(line))) => line,
        _ => return Ok(()),
    };

    if first_line.as_bytes().starts_with(H2_PREFACE) {
        handle_http2(reader.into_inner(), handler).await
    } else {
        handle_http1_line(reader, first_line, handler, keep_alive, max_request_size).await
    }
}

/// Handle an HTTP/1.x connection, given the first request line was already read.
async fn handle_http1_line<S>(mut reader: BufReader<S>, first_line: String, handler: Arc<dyn Handler>, keep_alive: Option<Duration>, max_request_size: usize) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let ka_timeout = keep_alive.unwrap_or(Duration::from_secs(5));
    let mut current_line = first_line;
    loop {
        if current_line.len() > max_request_size {
            write_response(&mut reader, Response::text(StatusCode::new(431).unwrap(), "Request Header Fields Too Large")).await?;
            break;
        }
        let parts: Vec<&str> = current_line.trim_end_matches(|c| c == '\r' || c == '\n').splitn(3, ' ').collect();
        if parts.len() < 3 { break; }
        let method: Method = match parts[0].parse().map_err(|e: String| crate::Error::InvalidRequest(e)) {
            Ok(m) => m, Err(_) => break,
        };
        let target = parts[1];

        let mut headers = HeaderMap::new();
        loop {
            let hline_opt = reader.read_line().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let hline = match hline_opt { Some(s) => s, None => break };
            if hline.is_empty() { break; }
            if let Some(colon) = hline.find(':') {
                let name = hline[..colon].trim();
                let value = hline[colon + 1..].trim();
                if !name.is_empty() { let _ = headers.insert(name, value); }
            }
        }

        let uri = if target.starts_with("http://") || target.starts_with("https://") {
            Uri::parse(target).unwrap_or_else(|_| Uri::parse("http://localhost/").unwrap())
        } else {
            let host = headers.get("Host").map(|v| v.as_str()).unwrap_or("localhost");
            let uri_str = if target.starts_with('/') { format!("http://{}{}", host, target) } else { format!("http://{}/{}", host, target) };
            Uri::parse(&uri_str).unwrap_or_else(|_| Uri::parse("http://localhost/").unwrap())
        };

        let body = if let Some(cl) = headers.get("Content-Length") {
            if let Ok(len) = cl.as_str().parse::<usize>() {
                let mut buf = vec![0u8; len];
                let mut total = 0;
                while total < len { match reader.read(&mut buf[total..]).await { Ok(0) => break, Ok(n) => total += n, Err(_) => break } }
                Some(buf)
            } else { None }
        } else { None };

        let request = Request::new(method, uri, headers, body);
        let is_head = request.method().as_str() == "HEAD";
        let response = handler.handle(request).await;
        write_response_head(&mut reader, &response, is_head).await?;

        let skip_body = is_head || response.status().as_u16() == 204 || response.status().as_u16() == 304 || response.status().is_informational();
        if !skip_body && !response.body().is_empty() { reader.get_mut().write_all(response.body()).await?; }

        current_line = match timeout(ka_timeout, reader.read_line()).await { Ok(Ok(Some(line))) => line, _ => break };
    }
    Ok(())
}

/// Handle an HTTP/2 connection with the unified Handler trait.
async fn handle_http2<S>(stream: S, handler: Arc<dyn Handler>) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut rdwr = stream;
    let mut encoder = Encoder::new();
    let mut decoder = Decoder::new();
    let mut server = Http2Server::new();
    let mut expecting_continuation: Option<(u32, Vec<u8>)> = None;

    loop {
        let frame = match read_frame(&mut rdwr).await { Ok(f) => f, Err(_) => break };

        if frame.frame_type == FrameType::Settings {
            let settings_frame = match crate::http2::frame::SettingsFrame::from_frame(&frame) {
                Ok(sf) => sf,
                Err(_) => { write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::PROTOCOL_ERROR.to_u32(), b"Bad SETTINGS").await; break; }
            };
            match server.apply_client_settings(&settings_frame) {
                FrameAction::WriteFrames(frames) => {
                    for f in &frames { write_frame(&mut rdwr, f).await?; }
                    decoder.set_max_table_size(server.client_settings.header_table_size as usize);
                }
                FrameAction::Goaway { error_code, debug_data, .. } => { write_goaway(&mut rdwr, server.last_processed_stream_id, error_code, &debug_data).await; break; }
                _ => {}
            }
            continue;
        }

        if frame.payload.len() as u32 > server.max_frame_size { write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::FRAME_SIZE_ERROR.to_u32(), b"Frame too large").await; break; }
        if let Err(ec) = frame.validate_semantics() { write_goaway(&mut rdwr, server.last_processed_stream_id, ec, b"Semantic violation").await; break; }

        let action = match frame.frame_type {
            FrameType::Ping => server.handle_ping(&frame),
            FrameType::WindowUpdate => server.handle_window_update(&frame),
            FrameType::RstStream => server.handle_rst_stream(&frame),
            FrameType::Priority => {
                let pf = if frame.payload.len() >= 5 {
                    let dep_raw = u32::from_be_bytes([frame.payload[0], frame.payload[1], frame.payload[2], frame.payload[3]]);
                    crate::http2::frame::PriorityFrame::new(frame.stream_id, (dep_raw >> 31) != 0, dep_raw & 0x7FFFFFFF, frame.payload[4].wrapping_add(1))
                } else {
                    crate::http2::frame::PriorityFrame::new(frame.stream_id, false, 0, 16)
                };
                server.handle_priority(&pf)
            }
            FrameType::Goaway => { server.handle_goaway(); break; }
            FrameType::PushPromise => { write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::PROTOCOL_ERROR.to_u32(), b"No push").await; break; }
            FrameType::Data => FrameAction::None,
            FrameType::Headers => {
                let hf = match crate::http2::frame::HeadersFrame::from_frame(&frame) {
                    Ok(hf) => hf,
                    Err(_) => { write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::PROTOCOL_ERROR.to_u32(), b"Bad HEADERS").await; break; }
                };
                if hf.stream_id == 0 || hf.stream_id % 2 == 0 { write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::PROTOCOL_ERROR.to_u32(), b"Invalid stream").await; break; }

                server.update_last_stream(hf.stream_id);
                let _ = server.stream_manager.get_or_create_stream(hf.stream_id);
                if let Some(s) = server.stream_manager.get_stream_mut(hf.stream_id) { let _ = s.open(); }

                let end_headers = frame.flags & flags::HEADERS_END_HEADERS != 0;
                if hf.end_stream && end_headers {
                    process_request(hf.stream_id, &hf.header_block, &mut decoder, &mut encoder, &mut server, &handler).await
                } else if end_headers && !hf.end_stream {
                    let headers = match decoder.decode(&hf.header_block) {
                        Ok(h) => h,
                        Err(_) => { write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::COMPRESSION_ERROR.to_u32(), b"HPACK error").await; break; }
                    };
                    if let Err((ec, _)) = validate_request_headers(&headers) { write_rst_stream(&mut rdwr, hf.stream_id, ec).await; FrameAction::None }
                    else if let Err((ec, _)) = validate_header_name_case(&headers) { write_rst_stream(&mut rdwr, hf.stream_id, ec).await; FrameAction::None }
                    else { server.pending_headers.insert(hf.stream_id, headers); FrameAction::None }
                } else if !end_headers {
                    expecting_continuation = Some((hf.stream_id, hf.header_block.clone()));
                    FrameAction::None
                } else { FrameAction::None }
            }
            FrameType::Continuation => {
                if let Some((stream_id, ref mut block)) = expecting_continuation {
                    block.extend_from_slice(&frame.payload);
                    let end_headers = frame.flags & flags::HEADERS_END_HEADERS != 0;
                    if end_headers {
                        let block = block.clone();
                        expecting_continuation = None;
                        process_request(stream_id, &block, &mut decoder, &mut encoder, &mut server, &handler).await
                    } else { FrameAction::None }
                } else { write_goaway(&mut rdwr, server.last_processed_stream_id, ErrorCode::PROTOCOL_ERROR.to_u32(), b"Unexpected CONTINUATION").await; break; }
            }
            _ => FrameAction::None,
        };

        match action {
            FrameAction::WriteFrames(frames) => { for f in &frames { if write_frame(&mut rdwr, f).await.is_err() { break; } } }
            FrameAction::Goaway { last_stream_id, error_code, debug_data } => { write_goaway(&mut rdwr, last_stream_id, error_code, &debug_data).await; break; }
            FrameAction::CloseConnection => break,
            FrameAction::None => {}
        }
    }
    Ok(())
}

/// Decode HPACK headers, call Handler, encode response as HTTP/2 HEADERS + DATA.
async fn process_request(
    stream_id: u32,
    header_block: &[u8],
    decoder: &mut Decoder<'_>,
    encoder: &mut Encoder<'_>,
    server: &mut Http2Server,
    handler: &dyn Handler,
) -> FrameAction {
    let headers = match decoder.decode(header_block) {
        Ok(h) => h,
        Err(_) => return FrameAction::None,
    };
    if let Err((ec, _)) = validate_request_headers(&headers) {
        return FrameAction::WriteFrames(vec![crate::http2::frame::RstStreamFrame::new(stream_id, ec).to_frame()]);
    }
    if let Err((ec, _)) = validate_header_name_case(&headers) {
        return FrameAction::WriteFrames(vec![crate::http2::frame::RstStreamFrame::new(stream_id, ec).to_frame()]);
    }

    let method = headers.iter().find(|(k, _)| k == b":method").and_then(|(_, v)| std::str::from_utf8(v).ok()?.parse().ok()).unwrap_or(Method::GET);
    let scheme = headers.iter().find(|(k, _)| k == b":scheme").and_then(|(_, v)| std::str::from_utf8(v).ok()).unwrap_or("http");
    let path = headers.iter().find(|(k, _)| k == b":path").and_then(|(_, v)| std::str::from_utf8(v).ok()).unwrap_or("/");
    let authority = headers.iter().find(|(k, _)| k == b":authority").and_then(|(_, v)| std::str::from_utf8(v).ok()).unwrap_or("localhost");
    let uri = Uri::parse(&format!("{}://{}{}", scheme, authority, path)).unwrap_or_else(|_| Uri::parse("http://localhost/").unwrap());

    let mut req_headers = HeaderMap::new();
    for (k, v) in &headers {
        if !k.starts_with(b":") {
            if let (Ok(kk), Ok(vv)) = (std::str::from_utf8(k), std::str::from_utf8(v)) { let _ = req_headers.insert(kk, vv); }
        }
    }

    let content_length = headers.iter().find(|(k, _)| k == b"content-length").and_then(|(_, v)| std::str::from_utf8(v).ok()?.parse::<usize>().ok());
    if let Some(s) = server.stream_manager.get_stream_mut(stream_id) { s.content_length = content_length.map(|x| x as u64); }

    let request = Request::new(method, uri, req_headers, None);
    let response = handler.handle(request).await;

    let mut resp_frames = Vec::new();
    let mut resp_headers = vec![(b":status".to_vec(), response.status().as_u16().to_string().into_bytes())];
    for (k, v) in response.headers().iter() { resp_headers.push((k.as_str().as_bytes().to_vec(), v.as_str().as_bytes().to_vec())); }
    let resp_header_block = encoder.encode(resp_headers.iter().map(|(k, v)| (k.as_slice(), v.as_slice())));
    resp_frames.push(crate::http2::frame::HeadersFrame::new(stream_id, resp_header_block, response.body().is_empty()).to_frame());
    if !response.body().is_empty() { resp_frames.push(crate::http2::frame::DataFrame::new(stream_id, response.body().to_vec(), true).to_frame()); }
    if let Some(s) = server.stream_manager.get_stream_mut(stream_id) { let _ = s.half_close_local(); }
    server.half_close_remote(stream_id);
    FrameAction::WriteFrames(resp_frames)
}

async fn read_frame<S>(stream: &mut S) -> std::io::Result<Frame>
where S: AsyncRead + Unpin,
{
    let mut hdr = [0u8; 9];
    stream.read_exact(&mut hdr).await?;
    let length = ((hdr[0] as u32) << 16) | ((hdr[1] as u32) << 8) | (hdr[2] as u32);
    let raw_type = hdr[3];
    let flags_byte = hdr[4];
    let stream_id = u32::from_be_bytes([hdr[5], hdr[6], hdr[7], hdr[8]]) & 0x7FFFFFFF;
    let mut payload = vec![0u8; length as usize];
    if length > 0 { stream.read_exact(&mut payload).await?; }
    Ok(Frame { frame_type: FrameType::from_u8(raw_type).unwrap_or(FrameType::Data), flags: flags_byte, stream_id, payload })
}

async fn write_frame<S>(stream: &mut S, frame: &Frame) -> std::io::Result<()>
where S: AsyncWrite + Unpin,
{
    stream.write_all(&frame.to_bytes()).await?;
    stream.flush().await
}

async fn write_goaway<S>(stream: &mut S, last_stream_id: u32, error_code: u32, debug: &[u8])
where S: AsyncWrite + Unpin,
{
    let _ = write_frame(stream, &crate::http2::frame::GoawayFrame::new(last_stream_id, error_code, debug.to_vec()).to_frame()).await;
}

async fn write_rst_stream<S>(stream: &mut S, stream_id: u32, error_code: u32)
where S: AsyncWrite + Unpin,
{
    let _ = write_frame(stream, &crate::http2::frame::RstStreamFrame::new(stream_id, error_code).to_frame()).await;
}

async fn write_response_head<S>(reader: &mut BufReader<S>, response: &Response, is_head: bool) -> std::io::Result<()>
where S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    reader.write_all(format!("HTTP/1.1 {} {}\r\n", response.status().as_u16(), response.status().reason()).as_bytes()).await?;
    for (name, value) in response.headers().iter() {
        reader.write_all(name.as_str().as_bytes()).await?;
        reader.write_all(b": ").await?;
        reader.write_all(value.as_str().as_bytes()).await?;
        reader.write_all(b"\r\n").await?;
    }
    if !response.headers().contains_key("Content-Length") {
        let cl = if is_head { 0 } else { response.body().len() };
        reader.write_all(format!("Content-Length: {}\r\n", cl).as_bytes()).await?;
    }
    reader.write_all(b"\r\n").await?;
    Ok(())
}

async fn write_response<S>(reader: &mut BufReader<S>, response: Response) -> std::io::Result<()>
where S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    reader.write_all(format!("HTTP/1.1 {} {}\r\n", response.status().as_u16(), response.status().reason()).as_bytes()).await?;
    for (name, value) in response.headers().iter() {
        reader.write_all(name.as_str().as_bytes()).await?;
        reader.write_all(b": ").await?;
        reader.write_all(value.as_str().as_bytes()).await?;
        reader.write_all(b"\r\n").await?;
    }
    if !response.headers().contains_key("Content-Length") {
        reader.write_all(format!("Content-Length: {}\r\n", response.body().len()).as_bytes()).await?;
    }
    reader.write_all(b"\r\n").await?;
    if !response.body().is_empty() && response.status().as_u16() != 204 && response.status().as_u16() != 304 && !response.status().is_informational() {
        reader.write_all(response.body()).await?;
    }
    Ok(())
}
