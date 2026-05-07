//! Unified HTTP server supporting HTTP/1.1, HTTP/2, and HTTP/3, with optional TLS 1.3.
//!
//! Auto-detects protocol on each TCP connection:
//! - HTTP/2 prior knowledge: starts with `PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n`
//! - HTTP/1.x: starts with a request line like `GET / HTTP/1.1`
//!
//! When TLS is enabled, performs a TLS 1.3 handshake first, then runs the
//! same protocol detection on the decrypted stream.
//!
//! HTTP/3 (QUIC) runs on UDP with built-in TLS 1.3 — enabled via `.with_http3()`.

use crate::handler::Handler;
use crate::header::HeaderMap;
use crate::http2::frame::{flags, Frame, FrameType};
use crate::http2::headers::{validate_header_name_case, validate_request_headers};
use crate::http2::hpack::{Decoder, Encoder};
use crate::http2::{FrameAction, Http2Server};
use crate::http2::ErrorCode;
use crate::method::Method;
use crate::runtime::net::SocketAddr;
use crate::runtime::sync::Arc;
use crate::runtime::time::Duration;
use crate::runtime::CancellationToken;
use crate::runtime::{
    bind_tcp_listener, sleep, spawn, timeout, AsyncRead, AsyncReadExt, AsyncTcpListener,
    AsyncTcpStream, AsyncWrite, AsyncWriteExt, BufReader,
};
use crate::uri::Uri;
use crate::{Request, Response, StatusCode};
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;

/// TLS certificate for the server.
#[cfg(feature = "tls")]
pub type TlsCertificate = edgerun_tls::certificate_gen::CertificateAndKey;

/// HTTP/2 connection preface (RFC 9113 §3.4).
const H2_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

/// Unified HTTP server.
pub struct HttpServer {
    handler: Arc<dyn Handler>,
    keep_alive: Option<Duration>,
    max_request_size: usize,
    #[cfg(feature = "tls")]
    tls_cert: Option<Arc<TlsCertificate>>,
    http2_idle_timeout: Duration,
    #[cfg(feature = "http3")]
    http3_enabled: bool,
}

impl HttpServer {
    pub fn new<H: Handler>(handler: H) -> Self {
        Self {
            handler: Arc::new(handler),
            keep_alive: Some(Duration::from_secs(5)),
            max_request_size: 10 * 1024 * 1024,
            #[cfg(feature = "tls")]
            tls_cert: None,
            http2_idle_timeout: Duration::from_secs(30),
            #[cfg(feature = "http3")]
            http3_enabled: false,
        }
    }
    pub fn keep_alive(mut self, t: Option<Duration>) -> Self {
        self.keep_alive = t;
        self
    }
    pub fn max_request_size(mut self, s: usize) -> Self {
        self.max_request_size = s;
        self
    }
    pub fn http2_idle_timeout(mut self, t: Duration) -> Self {
        self.http2_idle_timeout = t;
        self
    }

    /// Enable TLS 1.3 with the given certificate.
    #[cfg(feature = "tls")]
    pub fn with_tls(mut self, cert: TlsCertificate) -> Self {
        self.tls_cert = Some(Arc::new(cert));
        self
    }

    /// Enable HTTP/3 (QUIC) on the same port as the TCP listener.
    /// Requires TLS to be configured (HTTP/3 embeds TLS 1.3 in QUIC).
    #[cfg(feature = "http3")]
    pub fn with_http3(mut self) -> Self {
        self.http3_enabled = true;
        self
    }

    pub async fn bind(
        self,
        addr: impl crate::runtime::net::ToSocketAddrs,
    ) -> crate::runtime::io::Result<BoundHttpServer> {
        let listener = bind_tcp_listener(addr)?;
        let local_addr = listener
            .local_addr()
            .map_err(crate::runtime::io::Error::other)?;

        #[cfg(feature = "http3")]
        let http3_server = if self.http3_enabled && self.tls_cert.is_some() {
            let cert = self.tls_cert.as_ref().unwrap().as_ref().clone();
            let h3 = crate::http3::Http3Server::bind(local_addr, cert)
                .await
                .map_err(crate::runtime::io::Error::other)?;
            Some(Arc::new(h3))
        } else {
            None
        };

        Ok(BoundHttpServer {
            listener,
            handler: self.handler,
            keep_alive: self.keep_alive,
            max_request_size: self.max_request_size,
            local_addr,
            #[cfg(feature = "tls")]
            tls_cert: self.tls_cert,
            http2_idle_timeout: self.http2_idle_timeout,
            #[cfg(feature = "http3")]
            http3_server,
        })
    }
}

pub struct BoundHttpServer {
    listener: AsyncTcpListener,
    handler: Arc<dyn Handler>,
    keep_alive: Option<Duration>,
    max_request_size: usize,
    local_addr: SocketAddr,
    #[cfg(feature = "tls")]
    tls_cert: Option<Arc<TlsCertificate>>,
    http2_idle_timeout: Duration,
    #[cfg(feature = "http3")]
    http3_server: Option<Arc<crate::http3::Http3Server>>,
}

impl BoundHttpServer {
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Serve HTTP requests indefinitely on both TCP (HTTP/1.1 + HTTP/2) and UDP (HTTP/3).
    pub async fn serve(&self) -> crate::runtime::io::Result<()> {
        let shutdown = CancellationToken::new();
        self.serve_with_shutdown(shutdown).await
    }

    /// Serve HTTP requests until the shutdown token is cancelled.
    ///
    /// Runs both the TCP accept loop (HTTP/1.1 + HTTP/2) and the HTTP/3
    /// server (if enabled) concurrently. Cancels when `shutdown` fires.
    pub async fn serve_with_shutdown(
        &self,
        shutdown: CancellationToken,
    ) -> crate::runtime::io::Result<()> {
        #[cfg(feature = "tls")]
        let tls = self.tls_cert.is_some();
        #[cfg(not(feature = "tls"))]
        let tls = false;
        #[cfg(feature = "http3")]
        let h3 = self.http3_server.is_some();
        #[cfg(not(feature = "http3"))]
        let h3 = false;
        edgerun_log::info!(
            "HTTP server listening on {} (HTTP/1.1 + HTTP/2{}{})",
            self.local_addr,
            if tls { " + TLS" } else { "" },
            if h3 { " + HTTP/3" } else { "" }
        );
        let handler = Arc::clone(&self.handler);
        let keep_alive = self.keep_alive;
        let max_size = self.max_request_size;
        #[cfg(feature = "tls")]
        let tls_cert = self.tls_cert.clone();
        let http2_idle_timeout = self.http2_idle_timeout;

        #[cfg(feature = "http3")]
        let h3_handle = if let Some(ref h3_server) = self.http3_server {
            let h3_handler = Arc::clone(&self.handler);
            let h3_shutdown = shutdown.clone();
            let h3_server = Arc::clone(h3_server);
            Some(spawn(async move {
                h3_server
                    .serve(h3_handler, h3_shutdown)
                    .await
                    .map_err(crate::runtime::io::Error::other)
            }))
        } else {
            None
        };

        while !shutdown.is_cancelled() {
            match timeout(Duration::from_millis(100), self.listener.accept()).await {
                Ok(Ok((stream, peer_addr))) => {
                    let h = Arc::clone(&handler);
                    let ka = keep_alive;
                    let ms = max_size;
                    #[cfg(feature = "tls")]
                    let tc = tls_cert.clone();
                    let h2 = http2_idle_timeout;
                    spawn(async move {
                        if let Err(e) = handle_connection(
                            stream,
                            h,
                            ka,
                            ms,
                            #[cfg(feature = "tls")]
                            tc,
                            h2,
                        )
                        .await
                        {
                            edgerun_log::warn!("Connection error from {}: {}", peer_addr, e);
                        }
                    });
                }
                Ok(Err(e)) => {
                    edgerun_log::error!("Accept error: {}", e);
                    sleep(Duration::from_millis(100)).await;
                }
                Err(_) => continue,
            }
        }
        shutdown.cancel();
        #[cfg(feature = "http3")]
        if let Some(h) = h3_handle {
            let _ = h.await;
        }
        Ok(())
    }

    pub async fn accept_one(&self) -> crate::runtime::io::Result<()> {
        let (stream, _) = self
            .listener
            .accept()
            .await
            .map_err(crate::runtime::io::Error::other)?;
        let handler = Arc::clone(&self.handler);
        handle_connection(
            stream,
            handler,
            self.keep_alive,
            self.max_request_size,
            #[cfg(feature = "tls")]
            self.tls_cert.clone(),
            self.http2_idle_timeout,
        )
        .await
    }
}

/// Handle a single connection — optionally does TLS handshake, then auto-detects HTTP/1.1 vs HTTP/2.
async fn handle_connection<S>(
    stream: S,
    handler: Arc<dyn Handler>,
    keep_alive: Option<Duration>,
    max_request_size: usize,
    #[cfg(feature = "tls")] tls_cert: Option<Arc<TlsCertificate>>,
    http2_idle_timeout: Duration,
) -> crate::runtime::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    #[cfg(feature = "tls")]
    if let Some(ref cert) = tls_cert {
        use edgerun_tls::async_tls::AsyncTlsServerStream;
        let tls_stream = AsyncTlsServerStream::accept(stream, cert.as_ref())
            .await
            .map_err(|e| crate::runtime::io::Error::other(format!("TLS handshake failed: {e}")))?;

        // When TLS is established, use the negotiated ALPN protocol to
        // determine the HTTP version.
        // Per RFC 9113 §3.4, the client connection preface
        // "PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n" (24 bytes) is always sent —
        // even over TLS. We must consume the entire preface before reading frames.
        let negotiated_h2 = tls_stream.alpn_protocol() == Some(b"h2");
        if negotiated_h2 {
            edgerun_log::debug!("[h2] TLS+ALPN=h2 detected, entering HTTP/2 path");
            let mut reader = BufReader::new(tls_stream);
            // The BufReader is fresh — the full 24-byte preface may be in the stream.
            // RFC 9113 §3.5: If preface is invalid, MUST respond with GOAWAY and close.
            let mut preface_bytes = vec![0u8; 24];
            match reader.get_mut().read_exact(&mut preface_bytes).await {
                Ok(()) => {
                    // Check if connection preface is valid
                    if preface_bytes != H2_PREFACE {
                        // Send GOAWAY with PROTOCOL_ERROR before closing
                        let goaway = crate::http2::frame::GoawayFrame::new(
                            0,
                            0x1,
                            b"Invalid connection preface".to_vec(),
                        )
                        .to_frame();
                        let goaway_bytes = goaway.to_bytes();
                        let _ = reader.get_mut().write_all(&goaway_bytes).await;
                        let _ = reader.get_mut().flush().await;
                        // Give time for GOAWAY to be sent before closing
                        sleep(Duration::from_millis(50)).await;
                        // Close the write side of the connection
                        let _ = reader.get_mut().shutdown().await;
                        return Err(crate::runtime::io::Error::new(
                            crate::runtime::io::ErrorKind::InvalidInput,
                            "Invalid HTTP/2 connection preface",
                        ));
                    }
                }
                Err(e) => {
                    // RFC 9113 §3.5: If we can't read preface, send GOAWAY and close
                    let goaway =
                        crate::http2::frame::GoawayFrame::new(0, 0x1, b"Connection error".to_vec())
                            .to_frame();
                    let goaway_bytes = goaway.to_bytes();
                    let _ = reader.get_mut().write_all(&goaway_bytes).await;
                    let _ = reader.get_mut().flush().await;
                    sleep(Duration::from_millis(50)).await;
                    let _ = reader.get_mut().shutdown().await;
                    return Err(crate::runtime::bare_io(e));
                }
            }
            // Pass skip_preface=true since we already consumed and validated it above.
            return handle_http2(
                reader,
                handler,
                http2_idle_timeout,
                max_request_size,
                true,
                None,
            )
            .await;
        } else {
            return handle_connection_inner(
                tls_stream,
                handler,
                keep_alive,
                max_request_size,
                http2_idle_timeout,
            )
            .await;
        }
    }

    handle_connection_inner(
        stream,
        handler,
        keep_alive,
        max_request_size,
        http2_idle_timeout,
    )
    .await
}

/// Inner connection handler — reads first line to detect HTTP/1.1 vs HTTP/2.
async fn handle_connection_inner<S>(
    stream: S,
    handler: Arc<dyn Handler>,
    keep_alive: Option<Duration>,
    max_request_size: usize,
    http2_idle_timeout: Duration,
) -> crate::runtime::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let mut reader = BufReader::new(stream);
    let first_line = match timeout(Duration::from_secs(5), reader.read_line()).await {
        Ok(Ok(Some(line))) => line,
        _ => return Ok(()),
    };

    if first_line.starts_with("PRI * HTTP/2.0") {
        handle_http2(
            reader,
            handler,
            http2_idle_timeout,
            max_request_size,
            false,
            Some(first_line),
        )
        .await
    } else {
        handle_http1_line(reader, first_line, handler, keep_alive, max_request_size).await
    }
}

/// Handle an HTTP/1.x connection, given the first request line was already read.
async fn handle_http1_line<S>(
    mut reader: BufReader<S>,
    first_line: String,
    handler: Arc<dyn Handler>,
    keep_alive: Option<Duration>,
    max_request_size: usize,
) -> crate::runtime::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let ka_timeout = keep_alive.unwrap_or(Duration::from_secs(5));
    let mut current_line = first_line;
    loop {
        if current_line.len() > max_request_size {
            write_response(
                &mut reader,
                Response::text(
                    StatusCode::new(431).unwrap(),
                    "Request Header Fields Too Large",
                ),
            )
            .await?;
            break;
        }
        let parts: Vec<&str> = current_line
            .trim_end_matches(['\r', '\n'])
            .splitn(3, ' ')
            .collect();
        if parts.len() < 3 {
            break;
        }
        let method: Method = match parts[0]
            .parse()
            .map_err(|e: String| crate::Error::InvalidRequest(e))
        {
            Ok(m) => m,
            Err(_) => break,
        };
        let target = parts[1];

        let mut headers = HeaderMap::new();
        loop {
            let hline_opt = reader
                .read_line_max(max_request_size)
                .await
                .map_err(crate::runtime::bare_io)?;
            let hline = match hline_opt {
                Some(s) => s,
                None => break,
            };
            if hline.trim_end_matches(['\r', '\n']).is_empty() {
                break;
            }
            if let Some(colon) = hline.find(':') {
                let name = hline[..colon].trim();
                let value = hline[colon + 1..].trim();
                if !name.is_empty() {
                    let _ = headers.insert(name, value);
                }
            }
        }

        let uri = if target.starts_with("http://") || target.starts_with("https://") {
            Uri::parse(target).unwrap_or_else(|_| Uri::parse("http://localhost/").unwrap())
        } else {
            let host = headers
                .get("Host")
                .map(|v| v.as_str())
                .unwrap_or("localhost");
            let uri_str = if target.starts_with('/') {
                format!("http://{}{}", host, target)
            } else {
                format!("http://{}/{}", host, target)
            };
            Uri::parse(&uri_str).unwrap_or_else(|_| Uri::parse("http://localhost/").unwrap())
        };

        let body = if let Some(cl) = headers.get("Content-Length") {
            if let Ok(len) = cl.as_str().parse::<usize>() {
                let mut buf = vec![0u8; len];
                let mut total = 0;
                while total < len {
                    match reader.read(&mut buf[total..]).await {
                        Ok(0) => break,
                        Ok(n) => total += n,
                        Err(_) => break,
                    }
                }
                Some(buf)
            } else {
                None
            }
        } else {
            None
        };

        let request = Request::new(method, uri, headers, body);
        let is_head = request.method().as_str() == "HEAD";
        let response = handler.handle(request.clone()).await;
        let response = maybe_compress_response(&request, response);
        write_response_head(&mut reader, &response, is_head).await?;

        let skip_body = is_head
            || response.status().as_u16() == 204
            || response.status().as_u16() == 304
            || response.status().is_informational();
        if !skip_body && !response.body().is_empty() {
            reader
                .get_mut()
                .write_all(response.body())
                .await
                .map_err(crate::runtime::bare_io)?;
        }

        if keep_alive.is_none() {
            break;
        }

        current_line = match timeout(ka_timeout, reader.read_line_max(max_request_size)).await {
            Ok(Ok(Some(line))) => line,
            _ => break,
        };
    }
    Ok(())
}

/// Handle an HTTP/2 connection with the unified Handler trait.
///
/// When `skip_preface` is true (TLS+ALPN=h2 path), the preface was already
/// consumed by the caller — go straight to the frame loop.
/// When false (h2c upgrade path), consume the remaining preface bytes first.
async fn handle_http2<S>(
    mut reader: BufReader<S>,
    handler: Arc<dyn Handler>,
    idle_timeout: Duration,
    max_header_size: usize,
    skip_preface: bool,
    first_line: Option<String>,
) -> crate::runtime::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    edgerun_log::debug!("[h2] handle_http2 entered, skip_preface={}", skip_preface);
    // RFC 9113 §3.4: Server MUST validate the connection preface.
    // The preface is "PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n" (24 bytes).
    // When skip_preface=false (plaintext HTTP/2), we already read "PRI * HTTP/2.0\r\n" via read_line().
    // We need to validate it and consume the remaining 7 bytes.
    if !skip_preface {
        // RFC 9113 §3.4: Server MUST validate the connection preface.
        // first_line contains "PRI * HTTP/2.0\r\n" (or partial/invalid data)
        let first_line_preface = first_line
            .as_ref()
            .expect("first_line required when skip_preface=false");
        let preface_bytes = first_line_preface.as_bytes();

        // Check if it matches "PRI * HTTP/2.0\r\n"
        if !preface_bytes.starts_with(b"PRI * HTTP/2.0\r\n") || preface_bytes.len() < 17 {
            // RFC 9113 §3.5: Must respond with GOAWAY and close.
            let goaway = crate::http2::frame::GoawayFrame::new(
                0,
                0x1,
                b"Invalid connection preface".to_vec(),
            )
            .to_frame();
            let mut rdwr = reader.into_inner();
            let _ = rdwr.write_all(&goaway.to_bytes()).await;
            let _ = rdwr.flush().await;
            return Err(crate::runtime::io::Error::new(
                crate::runtime::io::ErrorKind::InvalidInput,
                "Invalid HTTP/2 connection preface",
            ));
        }

        // Now consume remaining 7 bytes (\r\nSM\r\n\r\n) from buffer
        let remaining_preface = H2_PREFACE.len() - 17;

        // First consume from buffer, then read remaining from stream if needed
        let buffered = reader.buffered();
        if buffered >= remaining_preface {
            reader.consume(remaining_preface);
        } else {
            // Consume what's buffered, read the rest from stream
            reader.consume(buffered);
            let mut discard = [0u8; 7];
            let to_read = remaining_preface - buffered;
            reader
                .get_mut()
                .read_exact(&mut discard[..to_read])
                .await
                .map_err(crate::runtime::bare_io)?;
        }
    }

    let mut rdwr = reader.into_inner();
    let mut encoder = Encoder::new();
    let mut decoder = Decoder::new();
    let mut server = Http2Server::new();
    let mut expecting_continuation: Option<(u32, Vec<u8>, bool)> = None;
    let mut pending_body_data: alloc::collections::BTreeMap<u32, Vec<u8>> =
        alloc::collections::BTreeMap::new();
    let mut frame_count: u64 = 0;

    // RFC 9113 §3.4: Server MUST send initial SETTINGS frame immediately
    let entries = server.server_settings.to_entries();
    let settings_frame = crate::http2::frame::SettingsFrame::new(entries.clone());
    write_frame(&mut rdwr, &settings_frame.to_frame()).await?;

    loop {
        frame_count += 1;
        // Periodically clean up closed streams (every 100 frames)
        if frame_count.is_multiple_of(100) {
            server.cleanup_closed_streams();
        }

        let frame = match timeout(idle_timeout, read_frame(&mut rdwr, server.max_frame_size)).await
        {
            Ok(Ok(Ok(f))) => {
                edgerun_log::debug!(
                    "[h2] Server got frame: type={}, stream={}",
                    f.frame_type as u8,
                    f.stream_id
                );
                f
            }
            Ok(Ok(Err((stream_id, error_code)))) => {
                write_goaway(
                    &mut rdwr,
                    server.last_processed_stream_id,
                    error_code,
                    b"Frame too large",
                )
                .await;
                break;
            }
            Ok(Err(_)) => break,
            Err(_) => {
                edgerun_log::debug!("HTTP/2 connection idle timeout reached");
                break;
            }
        };

        if frame.frame_type == FrameType::Settings {
            let settings_frame = match crate::http2::frame::SettingsFrame::from_frame(&frame) {
                Ok(sf) => sf,
                Err(_) => {
                    write_goaway(
                        &mut rdwr,
                        server.last_processed_stream_id,
                        ErrorCode::PROTOCOL_ERROR.to_u32(),
                        b"Bad SETTINGS",
                    )
                    .await;
                    break;
                }
            };
            match server.apply_client_settings(&settings_frame) {
                FrameAction::WriteFrames(frames) => {
                    for f in &frames {
                        write_frame(&mut rdwr, f).await?;
                    }
                    decoder.set_max_table_size(server.client_settings.header_table_size as usize);
                }
                FrameAction::Goaway {
                    error_code,
                    debug_data,
                    ..
                } => {
                    write_goaway(
                        &mut rdwr,
                        server.last_processed_stream_id,
                        error_code,
                        &debug_data,
                    )
                    .await;
                    break;
                }
                _ => {}
            }
            continue;
        }

        // RFC 9113 §4.1: skip semantic validation for unknown frame types
        if !matches!(frame.frame_type, FrameType::Extension) {
            if let Err(ec) = frame.validate_semantics() {
                write_goaway(
                    &mut rdwr,
                    server.last_processed_stream_id,
                    ec,
                    b"Semantic violation",
                )
                .await;
                break;
            }
        }

        let action = match frame.frame_type {
            FrameType::Ping => server.handle_ping(&frame),
            FrameType::WindowUpdate => server.handle_window_update(&frame),
            FrameType::RstStream => {
                server.handle_rst_stream(&frame);
                // Clean up any pending headers/body for this stream
                pending_body_data.remove(&frame.stream_id);
                server.pending_headers.remove(&frame.stream_id);
                FrameAction::None
            }
            FrameType::Priority => {
                let pf = if frame.payload.len() >= 5 {
                    let dep_raw = u32::from_be_bytes([
                        frame.payload[0],
                        frame.payload[1],
                        frame.payload[2],
                        frame.payload[3],
                    ]);
                    crate::http2::frame::PriorityFrame::new(
                        frame.stream_id,
                        (dep_raw >> 31) != 0,
                        dep_raw & 0x7FFFFFFF,
                        frame.payload[4].wrapping_add(1),
                    )
                } else {
                    crate::http2::frame::PriorityFrame::new(frame.stream_id, false, 0, 16)
                };
                server.handle_priority(&pf)
            }
            FrameType::Goaway => {
                server.handle_goaway();
                break;
            }
            FrameType::PushPromise => {
                write_goaway(
                    &mut rdwr,
                    server.last_processed_stream_id,
                    ErrorCode::PROTOCOL_ERROR.to_u32(),
                    b"No push",
                )
                .await;
                break;
            }
            FrameType::Data => {
                let end_stream = frame.flags & flags::DATA_END_STREAM != 0;
                let sid = frame.stream_id;
                let body_entry = pending_body_data.entry(sid).or_default();
                if body_entry.len() + frame.payload.len() > 100 * 1024 * 1024 {
                    pending_body_data.remove(&sid);
                    write_goaway(
                        &mut rdwr,
                        server.last_processed_stream_id,
                        ErrorCode::FRAME_SIZE_ERROR.to_u32(),
                        b"Body too large",
                    )
                    .await;
                    break;
                }
                body_entry.extend_from_slice(&frame.payload);

                if end_stream {
                    // We have the complete body — check if we also have pending headers
                    if let Some(headers) = server.pending_headers.remove(&sid) {
                        let body = pending_body_data.remove(&sid).unwrap_or_default();
                        server.update_last_stream(sid);
                        let action = process_request_with_body(
                            sid,
                            &headers,
                            body,
                            &mut decoder,
                            &mut encoder,
                            &mut server,
                            &handler,
                        )
                        .await;
                        action
                    } else {
                        // DATA arrived with END_STREAM but no headers — discard body
                        pending_body_data.remove(&sid);
                        FrameAction::None
                    }
                } else {
                    FrameAction::None
                }
            }
            FrameType::Headers => {
                let hf = match crate::http2::frame::HeadersFrame::from_frame(&frame) {
                    Ok(hf) => hf,
                    Err(_) => {
                        write_goaway(
                            &mut rdwr,
                            server.last_processed_stream_id,
                            ErrorCode::PROTOCOL_ERROR.to_u32(),
                            b"Bad HEADERS",
                        )
                        .await;
                        break;
                    }
                };
                if hf.stream_id == 0 || hf.stream_id % 2 == 0 {
                    write_goaway(
                        &mut rdwr,
                        server.last_processed_stream_id,
                        ErrorCode::PROTOCOL_ERROR.to_u32(),
                        b"Invalid stream",
                    )
                    .await;
                    break;
                }

                server.update_last_stream(hf.stream_id);
                let _ = server.stream_manager.get_or_create_stream(hf.stream_id);
                if let Some(s) = server.stream_manager.get_stream_mut(hf.stream_id) {
                    let _ = s.open();
                }

                let end_headers = frame.flags & flags::HEADERS_END_HEADERS != 0;
                if end_headers {
                    // END_HEADERS is set — decode and process headers
                    let headers = match decoder.decode(&hf.header_block) {
                        Ok(h) => h,
                        Err(e) => {
                            write_goaway(
                                &mut rdwr,
                                server.last_processed_stream_id,
                                ErrorCode::COMPRESSION_ERROR.to_u32(),
                                b"HPACK error",
                            )
                            .await;
                            break;
                        }
                    };
                    if let Err((ec, _)) = validate_request_headers(&headers) {
                        write_rst_stream(&mut rdwr, hf.stream_id, ec).await;
                        FrameAction::None
                    } else if let Err((ec, _)) = validate_header_name_case(&headers) {
                        write_rst_stream(&mut rdwr, hf.stream_id, ec).await;
                        FrameAction::None
                    } else if hf.end_stream {
                        // No body — process now
                        let action = process_request(
                            hf.stream_id,
                            &hf.header_block,
                            &mut decoder,
                            &mut encoder,
                            &mut server,
                            &handler,
                        )
                        .await;
                        action
                    } else {
                        // Body may come later — store headers
                        server.pending_headers.insert(hf.stream_id, headers);
                        FrameAction::None
                    }
                } else if !end_headers {
                    if hf.header_block.len() > max_header_size {
                        write_goaway(
                            &mut rdwr,
                            server.last_processed_stream_id,
                            ErrorCode::FRAME_SIZE_ERROR.to_u32(),
                            b"Header block too large",
                        )
                        .await;
                        break;
                    }
                    expecting_continuation =
                        Some((hf.stream_id, hf.header_block.clone(), hf.end_stream));
                    FrameAction::None
                } else {
                    FrameAction::None
                }
            }
            FrameType::Continuation => {
                if let Some((stream_id, ref mut block, headers_end_stream)) = expecting_continuation
                {
                    if block.len() + frame.payload.len() > max_header_size {
                        write_goaway(
                            &mut rdwr,
                            server.last_processed_stream_id,
                            ErrorCode::FRAME_SIZE_ERROR.to_u32(),
                            b"Header block too large",
                        )
                        .await;
                        break;
                    }
                    block.extend_from_slice(&frame.payload);
                    let end_headers = frame.flags & flags::HEADERS_END_HEADERS != 0;
                    if end_headers {
                        let block = block.clone();
                        let end_stream = headers_end_stream;
                        expecting_continuation = None;
                        if end_stream {
                            // HEADERS + CONTINUATION with END_STREAM — process immediately
                            let action = process_request(
                                stream_id,
                                &block,
                                &mut decoder,
                                &mut encoder,
                                &mut server,
                                &handler,
                            )
                            .await;
                            action
                        } else {
                            // Headers complete but need body — store as pending
                            let headers = match decoder.decode(&block) {
                                Ok(h) => h,
                                Err(_) => {
                                    write_goaway(
                                        &mut rdwr,
                                        server.last_processed_stream_id,
                                        ErrorCode::COMPRESSION_ERROR.to_u32(),
                                        b"HPACK error",
                                    )
                                    .await;
                                    break;
                                }
                            };
                            if let Err((ec, _)) = validate_request_headers(&headers) {
                                write_rst_stream(&mut rdwr, stream_id, ec).await;
                                FrameAction::None
                            } else if let Err((ec, _)) = validate_header_name_case(&headers) {
                                write_rst_stream(&mut rdwr, stream_id, ec).await;
                                FrameAction::None
                            } else {
                                server.pending_headers.insert(stream_id, headers);
                                FrameAction::None
                            }
                        }
                    } else {
                        FrameAction::None
                    }
                } else {
                    write_goaway(
                        &mut rdwr,
                        server.last_processed_stream_id,
                        ErrorCode::PROTOCOL_ERROR.to_u32(),
                        b"Unexpected CONTINUATION",
                    )
                    .await;
                    break;
                }
            }
            _ => {
                // RFC 9113 §4.1: frame types that are not understood MUST be ignored.
                // Don't send RST_STREAM, GOAWAY, or any response - just ignore the frame.
                if matches!(frame.frame_type, FrameType::Extension) {
                    edgerun_log::debug!("[h2] Ignoring unknown frame type");
                }

                // RFC 7540 §6.10: During a CONTINUATION sequence, only HEADERS,
                // PRIORITY, RST_STREAM, and CONTINUATION may be received.
                // WINDOW_UPDATE, DATA, SETTINGS, PING, GOAWAY, PUSH_PROMISE are forbidden.
                // RFC 9113 §4.1: unknown frame types are ignored entirely.
                if expecting_continuation.is_some() {
                    match frame.frame_type {
                        FrameType::Data
                        | FrameType::PushPromise
                        | FrameType::Goaway
                        | FrameType::Ping
                        | FrameType::Settings
                        | FrameType::WindowUpdate => {
                            write_goaway(
                                &mut rdwr,
                                server.last_processed_stream_id,
                                ErrorCode::PROTOCOL_ERROR.to_u32(),
                                b"Frame during CONTINUATION",
                            )
                            .await;
                            break;
                        }
                        FrameType::Extension => {
                            // RFC 9113 §4.1: ignore unknown frame types even during CONTINUATION
                            edgerun_log::debug!("[h2] Ignoring unknown frame during CONTINUATION");
                            FrameAction::None
                        }
                        _ => FrameAction::None,
                    }
                } else {
                    FrameAction::None
                }
            }
        };

        match action {
            FrameAction::WriteFrames(frames) => {
                for f in &frames {
                    if write_frame(&mut rdwr, f).await.is_err() {
                        break;
                    }
                }
            }
            FrameAction::Goaway {
                last_stream_id,
                error_code,
                debug_data,
            } => {
                write_goaway(&mut rdwr, last_stream_id, error_code, &debug_data).await;
                break;
            }
            FrameAction::CloseConnection => break,
            FrameAction::None => {}
        }
    }
    Ok(())
}

/// Decode HPACK headers, call Handler, encode response as HTTP/2 HEADERS + DATA.
/// Used when HEADERS and END_STREAM arrive together (no body).
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
        Err(_) => {
            // RFC 9113 §4.3: HPACK decode error is a connection error (GOAWAY).
            return FrameAction::Goaway {
                last_stream_id: server.last_processed_stream_id,
                error_code: ErrorCode::COMPRESSION_ERROR.to_u32(),
                debug_data: b"HPACK decode error in process_request".to_vec(),
            };
        }
    };
    process_request_with_body(
        stream_id,
        &headers,
        Vec::new(),
        decoder,
        encoder,
        server,
        handler,
    )
    .await
}

/// Process a complete HTTP/2 request with pre-decoded headers and accumulated body.
async fn process_request_with_body(
    stream_id: u32,
    headers: &[(Vec<u8>, Vec<u8>)],
    body: Vec<u8>,
    decoder: &mut Decoder<'_>,
    encoder: &mut Encoder<'_>,
    server: &mut Http2Server,
    handler: &dyn Handler,
) -> FrameAction {
    if let Err((ec, _)) = validate_request_headers(headers) {
        return FrameAction::WriteFrames(vec![crate::http2::frame::RstStreamFrame::new(
            stream_id, ec,
        )
        .to_frame()]);
    }
    if let Err((ec, _)) = validate_header_name_case(headers) {
        return FrameAction::WriteFrames(vec![crate::http2::frame::RstStreamFrame::new(
            stream_id, ec,
        )
        .to_frame()]);
    }

    let method = headers
        .iter()
        .find(|(k, _)| k == b":method")
        .and_then(|(_, v)| core::str::from_utf8(v).ok()?.parse().ok())
        .unwrap_or(Method::GET);
    let scheme = headers
        .iter()
        .find(|(k, _)| k == b":scheme")
        .and_then(|(_, v)| core::str::from_utf8(v).ok())
        .unwrap_or("http");
    let path = headers
        .iter()
        .find(|(k, _)| k == b":path")
        .and_then(|(_, v)| core::str::from_utf8(v).ok())
        .unwrap_or("/");
    let authority = headers
        .iter()
        .find(|(k, _)| k == b":authority")
        .and_then(|(_, v)| core::str::from_utf8(v).ok())
        .unwrap_or("localhost");
    let uri = Uri::parse(&format!("{}://{}{}", scheme, authority, path))
        .unwrap_or_else(|_| Uri::parse("http://localhost/").unwrap());

    let mut req_headers = HeaderMap::new();
    for (k, v) in headers {
        if !k.starts_with(b":") {
            if let (Ok(kk), Ok(vv)) = (core::str::from_utf8(k), core::str::from_utf8(v)) {
                let _ = req_headers.insert(kk, vv);
            }
        }
    }

    let content_length = headers
        .iter()
        .find(|(k, _)| k == b"content-length")
        .and_then(|(_, v)| core::str::from_utf8(v).ok()?.parse::<usize>().ok());
    if let Some(s) = server.stream_manager.get_stream_mut(stream_id) {
        s.content_length = content_length.map(|x| x as u64);
    }

    let request = Request::new(method, uri, req_headers, Some(body));
    let response = handler.handle(request.clone()).await;
    let response = maybe_compress_response(&request, response);

    let mut resp_frames = Vec::new();
    let mut resp_headers = vec![(
        b":status".to_vec(),
        response.status().as_u16().to_string().into_bytes(),
    )];
    for (k, v) in response.headers().iter() {
        resp_headers.push((
            k.as_str().as_bytes().to_vec(),
            v.as_str().as_bytes().to_vec(),
        ));
    }
    let resp_header_block = encoder.encode(
        resp_headers
            .iter()
            .map(|(k, v)| (k.as_slice(), v.as_slice())),
    );
    resp_frames.push(
        crate::http2::frame::HeadersFrame::new(
            stream_id,
            resp_header_block,
            response.body().is_empty(),
        )
        .to_frame(),
    );
    if !response.body().is_empty() {
        resp_frames.push(
            crate::http2::frame::DataFrame::new(stream_id, response.body().to_vec(), true)
                .to_frame(),
        );
    }
    if let Some(s) = server.stream_manager.get_stream_mut(stream_id) {
        let _ = s.half_close_local();
    }
    server.half_close_remote(stream_id);
    FrameAction::WriteFrames(resp_frames)
}

async fn read_frame<S>(
    stream: &mut S,
    max_frame_size: u32,
) -> crate::runtime::io::Result<Result<Frame, (u32, u32)>>
where
    S: AsyncRead + Unpin,
{
    let mut hdr = [0u8; 9];
    stream
        .read_exact(&mut hdr)
        .await
        .map_err(crate::runtime::bare_io)?;
    let length = ((hdr[0] as u32) << 16) | ((hdr[1] as u32) << 8) | (hdr[2] as u32);
    let raw_type = hdr[3];
    let flags_byte = hdr[4];
    let stream_id = u32::from_be_bytes([hdr[5], hdr[6], hdr[7], hdr[8]]) & 0x7FFFFFFF;

    // Validate frame size BEFORE allocating payload buffer (prevent DoS)
    if length > max_frame_size {
        // Discard the oversized payload in chunks
        let mut discard = [0u8; 4096];
        let mut remaining = length as usize;
        while remaining > 0 {
            let to_read = remaining.min(discard.len());
            stream
                .read_exact(&mut discard[..to_read])
                .await
                .map_err(crate::runtime::bare_io)?;
            remaining -= to_read;
        }
        return Ok(Err((stream_id, ErrorCode::FRAME_SIZE_ERROR.to_u32())));
    }

    let mut payload = vec![0u8; length as usize];
    if length > 0 {
        stream
            .read_exact(&mut payload)
            .await
            .map_err(crate::runtime::bare_io)?;
    }
    Ok(Ok(Frame {
        frame_type: FrameType::from_u8(raw_type).unwrap_or(FrameType::Data),
        flags: flags_byte,
        stream_id,
        payload,
    }))
}

async fn write_frame<S>(stream: &mut S, frame: &Frame) -> crate::runtime::io::Result<()>
where
    S: AsyncWrite + Unpin,
{
    stream
        .write_all(&frame.to_bytes())
        .await
        .map_err(crate::runtime::bare_io)?;
    stream.flush().await.map_err(crate::runtime::bare_io)
}

async fn write_goaway<S>(stream: &mut S, last_stream_id: u32, error_code: u32, debug: &[u8])
where
    S: AsyncWrite + Unpin,
{
    let _ = write_frame(
        stream,
        &crate::http2::frame::GoawayFrame::new(last_stream_id, error_code, debug.to_vec())
            .to_frame(),
    )
    .await;
}

async fn write_rst_stream<S>(stream: &mut S, stream_id: u32, error_code: u32)
where
    S: AsyncWrite + Unpin,
{
    let _ = write_frame(
        stream,
        &crate::http2::frame::RstStreamFrame::new(stream_id, error_code).to_frame(),
    )
    .await;
}

async fn write_response_head<S>(
    reader: &mut BufReader<S>,
    response: &Response,
    is_head: bool,
) -> crate::runtime::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    let content_length = if is_head { 0 } else { response.body().len() };
    write_response_headers(reader, response, content_length).await
}

async fn write_response<S>(
    reader: &mut BufReader<S>,
    response: Response,
) -> crate::runtime::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    write_response_headers(reader, &response, response.body().len()).await?;
    if should_write_response_body(&response) {
        reader
            .write_all(response.body())
            .await
            .map_err(crate::runtime::bare_io)?;
    }
    Ok(())
}

async fn write_response_headers<S>(
    reader: &mut BufReader<S>,
    response: &Response,
    content_length: usize,
) -> crate::runtime::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    reader
        .write_all(
            format!(
                "HTTP/1.1 {} {}\r\n",
                response.status().as_u16(),
                response.status().reason()
            )
            .as_bytes(),
        )
        .await
        .map_err(crate::runtime::bare_io)?;
    for (name, value) in response.headers().iter() {
        reader
            .write_all(name.as_str().as_bytes())
            .await
            .map_err(crate::runtime::bare_io)?;
        reader
            .write_all(b": ")
            .await
            .map_err(crate::runtime::bare_io)?;
        reader
            .write_all(value.as_str().as_bytes())
            .await
            .map_err(crate::runtime::bare_io)?;
        reader
            .write_all(b"\r\n")
            .await
            .map_err(crate::runtime::bare_io)?;
    }
    if !response.headers().contains_key("Content-Length") {
        reader
            .write_all(format!("Content-Length: {}\r\n", content_length).as_bytes())
            .await
            .map_err(crate::runtime::bare_io)?;
    }
    reader
        .write_all(b"\r\n")
        .await
        .map_err(crate::runtime::bare_io)?;
    Ok(())
}

fn should_write_response_body(response: &Response) -> bool {
    !response.body().is_empty()
        && response.status().as_u16() != 204
        && response.status().as_u16() != 304
        && !response.status().is_informational()
}

fn maybe_compress_response(request: &Request, response: Response) -> Response {
    if !should_write_response_body(&response)
        || response.body().len() < 512
        || response.headers().contains_key("Content-Encoding")
        || request.headers().contains_key("Range")
    {
        return response;
    }
    let Some(content_type) = response
        .headers()
        .get("Content-Type")
        .map(|value| value.as_str())
    else {
        return response;
    };
    if !is_compressible_content_type(content_type) {
        return response;
    }
    let Some(accept_encoding) = request
        .headers()
        .get("Accept-Encoding")
        .map(|value| value.as_str())
    else {
        return response;
    };
    let encoding = crate::http1::preferred_response_encoding(accept_encoding);
    if encoding == crate::http1::ContentEncoding::Identity {
        return response;
    }
    let Some(compressed) = crate::http1::compress_body(response.body(), encoding) else {
        return response;
    };
    if compressed.len() >= response.body().len() {
        return response;
    }

    let mut headers = response.headers().clone();
    headers.remove("Content-Length");
    headers.remove("Content-Encoding");
    let _ = headers.insert("Content-Encoding", encoding.as_str());
    headers.remove("Vary");
    let _ = headers.insert("Vary", "Accept-Encoding");
    let _ = headers.insert("Content-Length", &compressed.len().to_string());
    Response::from_parts(response.status(), headers, compressed)
}

fn is_compressible_content_type(content_type: &str) -> bool {
    let content_type = content_type
        .split(';')
        .next()
        .unwrap_or(content_type)
        .trim()
        .to_ascii_lowercase();
    content_type.starts_with("text/")
        || matches!(
            content_type.as_str(),
            "application/javascript"
                | "application/json"
                | "application/manifest+json"
                | "application/opensearchdescription+xml"
                | "application/xml"
                | "application/atom+xml"
                | "image/svg+xml"
        )
}
