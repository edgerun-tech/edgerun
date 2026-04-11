//! Async HTTP/1.1 server.
//!
//! Provides [`Server`] for serving HTTP/1.1 requests with:
//! - Async accept via [`edgerun_rt::AsyncTcpListener`]
//! - Per-connection task spawning via [`edgerun_rt::spawn`]
//! - Keep-alive timeout
//! - Graceful connection handling
//!
//! # TLS (HTTPS)
//!
//! Use [`TlsServer`] for TLS 1.3 connections. Requires the `tls` feature.
//!
//! ```no_run
//! use edgerun_http::http1::{TlsServer, into_handler, Request, Response};
//! use edgerun_tls::generate_self_signed;
//! use edgerun_rt::Runtime;
//!
//! let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
//! rt.block_on(async {
//!     let cert = generate_self_signed(&["127.0.0.1", "localhost"]);
//!     let handler = into_handler(|_req: Request| {
//!         Response::new(edgerun_http::StatusCode::new(200).unwrap())
//!     });
//!
//!     TlsServer::new(handler, cert)
//!         .bind("127.0.0.1:8443")
//!         .await
//!         .unwrap()
//!         .serve()
//!         .await
//!         .unwrap();
//! });
//! ```

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use edgerun_rt::{
    AsyncRead, AsyncWrite, AsyncWriteExt,
    AsyncTcpListener,
    spawn,
};

#[cfg(feature = "tls")]
use edgerun_rt::AsyncTcpStream;
#[cfg(feature = "tls")]
use edgerun_tls::async_tls::AsyncTlsServerStream;
#[cfg(feature = "tls")]
use edgerun_tls::CertificateAndKey;

/// Cloneable certificate reference for async TLS server tasks.
#[cfg(feature = "tls")]
struct CertRef {
    cert_der: Vec<u8>,
    signing_key: std::sync::Arc<edgerun_crypto::p256::ecdsa::SigningKey>,
}

#[cfg(feature = "tls")]
impl CertRef {
    fn as_cert_and_key(&self) -> CertificateAndKey {
        CertificateAndKey {
            cert_der: self.cert_der.clone(),
            signing_key: (*self.signing_key).clone(),
        }
    }
}

use crate::http1::buf_reader::BufReader;
use crate::http1::connection::{determine_connection, ConnectionState};
use crate::http1::handler::Handler;
use crate::http1::request::Request;
use crate::http1::response::Response;
use crate::http1::upgrade::is_websocket_upgrade;
use crate::http1::version::HttpVersion;
use crate::{HeaderMap, Method, StatusCode};

/// An HTTP/1.1 server.
///
/// # Example
/// ```no_run
/// use edgerun_http::http1::{Server, into_handler, Request, Response};
/// use edgerun_rt::Runtime;
///
/// let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
/// rt.block_on(async {
///     let handler = into_handler(|_req: Request| {
///         Response::new(edgerun_http::StatusCode::new(200).unwrap())
///     });
///
///     Server::new(handler)
///         .bind("127.0.0.1:8080")
///         .await
///         .unwrap()
///         .serve()
///         .await
///         .unwrap();
/// });
/// ```
pub struct Server {
    handler: Arc<dyn Handler>,
    keep_alive_timeout: Option<Duration>,
    max_request_size: usize,
}

impl Server {
    /// Create a new server with the given handler.
    pub fn new<H: Handler>(handler: H) -> Self {
        Server {
            handler: Arc::new(handler),
            keep_alive_timeout: Some(Duration::from_secs(5)),
            max_request_size: 10 * 1024 * 1024, // 10 MB
        }
    }

    /// Set the keep-alive timeout. `None` disables keep-alive.
    pub fn keep_alive_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.keep_alive_timeout = timeout;
        self
    }

    /// Set the maximum request size (headers + body) in bytes.
    pub fn max_request_size(mut self, size: usize) -> Self {
        self.max_request_size = size;
        self
    }

    /// Bind to the given address and return a running server instance.
    pub async fn bind(self, addr: impl std::net::ToSocketAddrs) -> std::io::Result<BoundServer> {
        let listener = AsyncTcpListener::bind(addr)?;
        let local_addr = listener.local_addr()?;
        Ok(BoundServer {
            listener,
            handler: self.handler,
            keep_alive_timeout: self.keep_alive_timeout,
            max_request_size: self.max_request_size,
            local_addr,
        })
    }
}

/// A server bound to a listening socket.
pub struct BoundServer {
    listener: AsyncTcpListener,
    handler: Arc<dyn Handler>,
    keep_alive_timeout: Option<Duration>,
    max_request_size: usize,
    local_addr: SocketAddr,
}

impl BoundServer {
    /// Get the local address the server is listening on.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Start accepting connections. Runs until the listener errors.
    pub async fn serve(&self) -> std::io::Result<()> {
        edgerun_log::info!("HTTP/1.1 server listening on {}", self.local_addr);

        loop {
            match self.listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let handler = Arc::clone(&self.handler);
                    let keep_alive = self.keep_alive_timeout;
                    let max_size = self.max_request_size;

                    spawn(async move {
                        if let Err(e) = handle_connection(
                            stream,
                            peer_addr,
                            handler,
                            keep_alive,
                            max_size,
                        ).await {
                            edgerun_log::warn!("Connection error from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    edgerun_log::error!("Accept error: {}", e);
                    edgerun_rt::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Accept and handle a single connection (useful for testing).
    pub async fn accept_one(&self) -> std::io::Result<()> {
        let (stream, peer_addr) = self.listener.accept().await?;
        let handler = Arc::clone(&self.handler);
        let keep_alive = self.keep_alive_timeout;
        let max_size = self.max_request_size;

        handle_connection(stream, peer_addr, handler, keep_alive, max_size).await
    }
}

// ---------------------------------------------------------------------------
// TLS server
// ---------------------------------------------------------------------------

/// An HTTP/1.1 server with TLS 1.3.
///
/// Requires the `tls` feature. Uses `edgerun-tls` for the TLS handshake
/// and then delegates to the same HTTP/1.1 request handling as [`Server`].
#[cfg(feature = "tls")]
pub struct TlsServer {
    handler: Arc<dyn Handler>,
    cert: CertificateAndKey,
    keep_alive_timeout: Option<Duration>,
    max_request_size: usize,
}

#[cfg(feature = "tls")]
impl TlsServer {
    /// Create a new TLS server with the given handler and certificate.
    pub fn new<H: Handler>(handler: H, cert: CertificateAndKey) -> Self {
        TlsServer {
            handler: Arc::new(handler),
            cert,
            keep_alive_timeout: Some(Duration::from_secs(5)),
            max_request_size: 10 * 1024 * 1024,
        }
    }

    /// Set the keep-alive timeout.
    pub fn keep_alive_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.keep_alive_timeout = timeout;
        self
    }

    /// Set the maximum request size.
    pub fn max_request_size(mut self, size: usize) -> Self {
        self.max_request_size = size;
        self
    }

    /// Bind and return a running TLS server.
    pub async fn bind(self, addr: impl std::net::ToSocketAddrs) -> std::io::Result<TlsBoundServer> {
        let listener = AsyncTcpListener::bind(addr)?;
        let local_addr = listener.local_addr()?;
        Ok(TlsBoundServer {
            listener,
            handler: self.handler,
            cert: self.cert,
            keep_alive_timeout: self.keep_alive_timeout,
            max_request_size: self.max_request_size,
            local_addr,
        })
    }
}

/// A TLS server bound to a listening socket.
#[cfg(feature = "tls")]
pub struct TlsBoundServer {
    listener: AsyncTcpListener,
    handler: Arc<dyn Handler>,
    cert: CertificateAndKey,
    keep_alive_timeout: Option<Duration>,
    max_request_size: usize,
    local_addr: SocketAddr,
}

#[cfg(feature = "tls")]
impl TlsBoundServer {
    /// Get the local address.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Start accepting TLS connections.
    pub async fn serve(&self) -> std::io::Result<()> {
        edgerun_log::info!("HTTPS server listening on {}", self.local_addr);

        // Wrap cert in Arc for sharing across tasks.
        // CertificateAndKey is not Clone (SigningKey isn't), so we Arc it.
        let cert = Arc::new(CertRef {
            cert_der: self.cert.cert_der.clone(),
            signing_key: Arc::new(self.cert.signing_key.clone()),
        });

        loop {
            match self.listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let handler = Arc::clone(&self.handler);
                    let cert = Arc::clone(&cert);
                    let keep_alive = self.keep_alive_timeout;
                    let max_size = self.max_request_size;

                    spawn(async move {
                        if let Err(e) = handle_tls_connection(
                            stream,
                            peer_addr,
                            handler,
                            &cert,
                            keep_alive,
                            max_size,
                        ).await {
                            edgerun_log::warn!("TLS connection error from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    edgerun_log::error!("Accept error: {}", e);
                    edgerun_rt::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Accept and handle a single TLS connection.
    pub async fn accept_one(&self) -> std::io::Result<()> {
        let (stream, peer_addr) = self.listener.accept().await?;
        let handler = Arc::clone(&self.handler);
        let keep_alive = self.keep_alive_timeout;
        let max_size = self.max_request_size;
        let cert = CertRef {
            cert_der: self.cert.cert_der.clone(),
            signing_key: std::sync::Arc::new(self.cert.signing_key.clone()),
        };

        handle_tls_connection(
            stream,
            peer_addr,
            handler,
            &cert,
            keep_alive,
            max_size,
        ).await
    }
}

/// Handle a single TLS connection.
#[cfg(feature = "tls")]
async fn handle_tls_connection(
    stream: Arc<AsyncTcpStream>,
    peer_addr: SocketAddr,
    handler: Arc<dyn Handler>,
    cert: &CertRef,
    keep_alive_timeout: Option<Duration>,
    max_request_size: usize,
) -> std::io::Result<()> {
    let cert_and_key = cert.as_cert_and_key();
    let tls_stream = match AsyncTlsServerStream::accept(stream, &cert_and_key) {
        Ok(s) => s,
        Err(e) => {
            edgerun_log::debug!("TLS handshake failed from {}: {}", peer_addr, e);
            return Ok(());
        }
    };

    handle_connection(tls_stream, peer_addr, handler, keep_alive_timeout, max_request_size).await
}

/// Handle a single connection. Processes multiple requests over the same
/// connection (HTTP/1.1 keep-alive) until the client closes, timeout fires,
/// or a parse error occurs.
///
/// If an upgrade request is detected and accepted by the handler,
/// the connection transitions to the upgraded protocol and this function
/// returns `Ok(())` without closing the connection.
async fn handle_connection<S>(
    stream: S,
    peer_addr: SocketAddr,
    handler: Arc<dyn Handler>,
    keep_alive_timeout: Option<Duration>,
    max_request_size: usize,
) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let mut buf_reader = BufReader::new(stream);
    let mut conn_state = ConnectionState::KeepAlive;

    loop {
        // Read request headers with optional timeout
        let request_result = if let Some(timeout) = keep_alive_timeout {
            match edgerun_rt::timeout(timeout, read_request(&mut buf_reader, max_request_size)).await {
                Ok(result) => result,
                Err(_) => {
                    edgerun_log::debug!("Keep-alive timeout for {}", peer_addr);
                    return Ok(());
                }
            }
        } else {
            read_request(&mut buf_reader, max_request_size).await
        };

        let request = match request_result {
            Ok(Some(req)) => req,
            Ok(None) => return Ok(()), // Client closed connection
            Err(e) => {
                edgerun_log::debug!("Request parse error from {}: {}", peer_addr, e);
                let resp = Response::new(StatusCode::BAD_REQUEST);
                write_response(&mut buf_reader, &resp, false, HttpVersion::Http11).await?;
                return Ok(());
            }
        };

        let method = request.method().clone();
        let is_head = method == Method::HEAD;
        let version = *request.version();
        let is_upgrade = is_websocket_upgrade(request.headers());

        // Determine connection state from headers + version
        conn_state = determine_connection(request.headers(), version);

        // For upgrade requests, don't drain the body — let handler deal with it
        if !is_upgrade {
            drain_body(&mut buf_reader, &request).await?;
        }

        // Call handler
        let response = handler.handle(request).await;

        // Check if handler returned 101 Switching Protocols
        let is_101 = response.status().as_u16() == 101;

        // Write response with correct version
        write_response(&mut buf_reader, &response, is_head, version).await?;

        // If upgrade was accepted, transition to upgraded protocol
        // (handler takes over the raw stream — we return here)
        if is_101 {
            edgerun_log::debug!("Upgrade accepted, transitioning to upgraded protocol");
            return Ok(());
        }

        // Close connection if requested
        if !conn_state.is_persistent() {
            return Ok(());
        }
    }
}

/// Drain and discard the request body through the buffered reader.
async fn drain_body<R: AsyncRead + AsyncWrite + Unpin>(
    buf_reader: &mut BufReader<R>,
    request: &Request,
) -> std::io::Result<()> {
    let content_length = request.headers().get("content-length")
        .and_then(|v| v.as_str().parse::<u64>().ok());
    let is_chunked = request.headers().get("transfer-encoding")
        .map(|v| v.as_str().to_lowercase())
        .map_or(false, |v| v.contains("chunked"));

    if is_chunked {
        // Drain chunked body: read chunk-size lines and data until 0-length chunk
        drain_chunked_body(buf_reader).await
    } else if let Some(len) = content_length {
        // Drain exactly `len` bytes
        drain_exact(buf_reader, len).await
    } else {
        // No body indicator — nothing to drain
        Ok(())
    }
}

/// Drain exactly `n` bytes from the buffered reader.
async fn drain_exact<R: AsyncRead + Unpin>(
    buf_reader: &mut BufReader<R>,
    mut remaining: u64,
) -> std::io::Result<()> {
    let mut buf = [0u8; 8192];
    while remaining > 0 {
        let to_read = (remaining as usize).min(buf.len());
        let n = buf_reader.read(&mut buf[..to_read]).await?;
        if n == 0 { break; }
        remaining -= n as u64;
    }
    Ok(())
}

/// Drain a chunked transfer-encoded body.
async fn drain_chunked_body<R: AsyncRead + Unpin>(
    buf_reader: &mut BufReader<R>,
) -> std::io::Result<()> {
    let mut line_buf = Vec::with_capacity(32);
    loop {
        // Read chunk-size line
        line_buf.clear();
        loop {
            let mut byte = [0u8; 1];
            let n = buf_reader.read(&mut byte).await?;
            if n == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "unexpected EOF reading chunk size",
                ));
            }
            if byte[0] == b'\n' { break; }
            line_buf.push(byte[0]);
        }
        // Remove trailing \r
        if line_buf.last() == Some(&b'\r') { line_buf.pop(); }

        let size_hex = std::str::from_utf8(&line_buf)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid chunk size"))?;
        let size_str = size_hex.split(';').next().unwrap_or(size_hex).trim();
        let chunk_size = usize::from_str_radix(size_str, 16)
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid chunk size"))?;

        if chunk_size == 0 {
            // Last chunk — drain trailer headers until blank line
            return drain_trailers(buf_reader).await;
        }

        // Drain chunk data
        drain_exact(buf_reader, chunk_size as u64).await?;

        // Skip trailing \r\n
        let mut crlf = [0u8; 2];
        let n = buf_reader.read(&mut crlf).await?;
        if n == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "unexpected EOF after chunk data",
            ));
        }
    }
}

/// Drain trailer headers (until blank line).
async fn drain_trailers<R: AsyncRead + Unpin>(
    buf_reader: &mut BufReader<R>,
) -> std::io::Result<()> {
    let mut buf = [0u8; 8192];
    // Read until we find \r\n\r\n (or connection closes)
    let mut saw_cr = false;
    loop {
        let n = buf_reader.read(&mut buf[..1]).await?;
        if n == 0 { return Ok(()); }
        match buf[0] {
            b'\r' => saw_cr = true,
            b'\n' if saw_cr => return Ok(()),
            b'\n' => saw_cr = true,
            _ => saw_cr = false,
        }
    }
}

/// Read an HTTP/1.x request (status line + headers) from the buffered reader.
///
/// Returns `Ok(Some(request))` on success, `Ok(None)` on clean EOF,
/// or `Err` on parse error.
async fn read_request<R: AsyncRead + Unpin>(
    buf_reader: &mut BufReader<R>,
    max_request_size: usize,
) -> std::io::Result<Option<Request>> {
    // Read status line
    let status_line = match buf_reader.read_line().await? {
        Some(line) => line,
        None => return Ok(None), // Clean EOF
    };

    if status_line.len() > max_request_size {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "request line too long",
        ));
    }

    // Parse: METHOD SP REQUEST-TARGET SP HTTP-VERSION
    let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid request line",
        ));
    }

    let method: Method = parts[0].parse()
        .map_err(|e: String| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    let request_target = parts[1];

    // Parse HTTP version
    let version_str = parts.get(2).copied().unwrap_or("HTTP/1.0");
    let version = HttpVersion::from_str(version_str)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    // Read headers
    let mut headers = HeaderMap::new();
    loop {
        let line = buf_reader.read_line().await?
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "unexpected EOF reading headers",
            ))?;

        if line.is_empty() {
            break;
        }

        if line.len() > max_request_size {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "header too long",
            ));
        }

        if let Some(colon) = line.find(':') {
            let name = line[..colon].trim();
            let value = line[colon + 1..].trim();
            if !name.is_empty() {
                let _ = headers.insert(name, value);
            }
        }
    }

    // Build the request. The server sees an origin-form request target
    // (e.g., "/path?query"), so we need to construct a proper URI.
    // Use the Host header if available.
    let host = headers.get("host").map(|v| v.as_str().to_string());
    let uri_str = if let Some(ref h) = host {
        format!("http://{}{}", h, request_target)
    } else {
        format!("http://localhost{}", request_target)
    };

    let mut builder = Request::builder()
        .method(method)
        .version(version)
        .uri(&uri_str);

    for (name, value) in headers.iter() {
        builder = builder.header(name.as_str(), value.as_str());
    }

    let request = builder.build()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    Ok(Some(request))
}

/// Write an HTTP/1.x response.
///
/// If `is_head` is true, the status line and headers are written but the body
/// is suppressed (per RFC 9112 §6.3 — HEAD responses MUST NOT contain a body).
/// The `version` parameter determines the HTTP version in the status line.
async fn write_response<W: AsyncWrite + Unpin>(
    writer: &mut W,
    response: &Response,
    is_head: bool,
    version: HttpVersion,
) -> std::io::Result<()> {
    let status = response.status();
    let mut buf = format!(
        "{} {} {}\r\n",
        version.as_str(),
        status.as_u16(),
        status.reason()
    ).into_bytes();

    for (name, value) in response.headers().iter() {
        buf.extend_from_slice(name.as_str().as_bytes());
        buf.push(b':');
        buf.push(b' ');
        buf.extend_from_slice(value.as_str().as_bytes());
        buf.extend_from_slice(b"\r\n");
    }

    if !response.headers().contains_key("Content-Length") {
        let len_line = format!("Content-Length: {}\r\n", response.body().len());
        buf.extend_from_slice(len_line.as_bytes());
    }

    if !response.headers().contains_key("Connection") {
        match version.default_connection_behavior() {
            crate::http1::version::ConnectionDefault::Close => {
                buf.extend_from_slice(b"Connection: close\r\n");
            }
            crate::http1::version::ConnectionDefault::KeepAlive => {
                buf.extend_from_slice(b"Connection: keep-alive\r\n");
            }
        }
    }

    buf.extend_from_slice(b"\r\n");
    writer.write_all(&buf).await?;

    // Suppress body for HEAD requests (RFC 9112 §6.3)
    if !is_head {
        writer.write_all(response.body()).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_rt::Runtime;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    #[test]
    fn test_write_response() {
        let rt = Runtime::new_multi_thread().enable_all().build().unwrap();

        let mut resp = Response::new(StatusCode::new(200).unwrap());
        resp.set_body(b"hello".to_vec());

        let output = rt.block_on(async move {
            let mut buf = Vec::new();
            let mut writer = VecWriter(&mut buf);
            write_response(&mut writer, &resp, false, HttpVersion::Http11).await.unwrap();
            buf
        });

        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(output_str.contains("Content-Length: 5\r\n"));
        assert!(output_str.ends_with("\r\n\r\nhello"));
    }

    #[test]
    fn test_write_response_head() {
        let rt = Runtime::new_multi_thread().enable_all().build().unwrap();

        let mut resp = Response::new(StatusCode::new(200).unwrap());
        resp.set_body(b"hello".to_vec());

        let output = rt.block_on(async move {
            let mut buf = Vec::new();
            let mut writer = VecWriter(&mut buf);
            write_response(&mut writer, &resp, true, HttpVersion::Http11).await.unwrap();
            buf
        });

        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(output_str.contains("Content-Length: 5\r\n"));
        // Body must NOT be present for HEAD
        assert!(!output_str.ends_with("hello"));
        assert!(output_str.ends_with("\r\n\r\n"));
    }

    #[test]
    fn test_write_response_http10() {
        let rt = Runtime::new_multi_thread().enable_all().build().unwrap();

        let mut resp = Response::new(StatusCode::new(200).unwrap());
        resp.set_body(b"hello".to_vec());

        let output = rt.block_on(async move {
            let mut buf = Vec::new();
            let mut writer = VecWriter(&mut buf);
            write_response(&mut writer, &resp, false, HttpVersion::Http10).await.unwrap();
            buf
        });

        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.starts_with("HTTP/1.0 200 OK\r\n"));
        assert!(output_str.contains("Content-Length: 5\r\n"));
        assert!(output_str.contains("Connection: close\r\n"));
        assert!(output_str.ends_with("\r\n\r\nhello"));
    }

    #[test]
    fn test_status_reason_uses_status_code() {
        let ok = StatusCode::new(200).unwrap();
        assert_eq!(ok.reason(), "OK");
        let not_found = StatusCode::new(404).unwrap();
        assert_eq!(not_found.reason(), "Not Found");
        let internal = StatusCode::new(500).unwrap();
        assert_eq!(internal.reason(), "Internal Server Error");
        let unknown = StatusCode::new(999).unwrap();
        assert_eq!(unknown.reason(), "Unknown Status");
    }

    struct VecWriter<'a>(&'a mut Vec<u8>);

    impl AsyncWrite for VecWriter<'_> {
        fn poll_write(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            let this = self.get_mut();
            this.0.extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }

        fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }

        fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
    }

    impl Unpin for VecWriter<'_> {}
}
