//! Async HTTP/1.1 server.
//!
//! Provides [`Server`] for serving HTTP/1.1 requests with:
//! - Async accept via [`edgerun_rt::AsyncTcpListener`]
//! - Per-connection task spawning via [`edgerun_rt::spawn`]
//! - Keep-alive timeout
//! - Graceful connection handling

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use edgerun_rt::{
    AsyncRead, AsyncWrite, AsyncWriteExt,
    AsyncTcpListener, AsyncTcpStream,
    spawn,
};

use crate::http1::body::AsyncBodyReader;
use crate::http1::buf_reader::BufReader;
use crate::http1::handler::Handler;
use crate::http1::request::Request;
use crate::http1::response::Response;
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

/// Handle a single connection. Processes multiple requests over the same
/// connection (HTTP/1.1 keep-alive) until the client closes, timeout fires,
/// or a parse error occurs.
async fn handle_connection(
    stream: Arc<AsyncTcpStream>,
    peer_addr: SocketAddr,
    handler: Arc<dyn Handler>,
    keep_alive_timeout: Option<Duration>,
    max_request_size: usize,
) -> std::io::Result<()> {
    let (read_half, mut write_half) = stream.split();
    let mut buf_reader = BufReader::new(read_half);

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
                write_response(&mut write_half, &resp, false).await?;
                return Ok(());
            }
        };

        let method = request.method().clone();
        let is_head = method == Method::HEAD;

        // Drain request body if present
        let content_length = request.headers().get("content-length")
            .and_then(|v| v.as_str().parse::<u64>().ok());
        let is_chunked = request.headers().get("transfer-encoding")
            .map(|v| v.as_str().to_lowercase())
            .map_or(false, |v| v.contains("chunked"));

        let inner = buf_reader.into_inner();
        let body_reader = if is_chunked {
            AsyncBodyReader::chunked(inner)
        } else if let Some(len) = content_length {
            AsyncBodyReader::with_length(inner, len)
        } else {
            AsyncBodyReader::until_eof(inner)
        };

        // Drain the body (discard it)
        let _ = body_reader.collect().await;

        // Call handler
        let response = handler.handle(request).await;

        // Write response
        write_response(&mut write_half, &response, is_head).await?;
    }
}

/// Read an HTTP/1.1 request (status line + headers) from the buffered reader.
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

    // Validate HTTP version
    let version = parts.get(2).copied().unwrap_or("");
    if version != "HTTP/1.1" && version != "HTTP/1.0" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unsupported HTTP version: {}", version),
        ));
    }

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

    let mut builder = Request::builder().method(method).uri(&uri_str);

    for (name, value) in headers.iter() {
        builder = builder.header(name.as_str(), value.as_str());
    }

    let request = builder.build()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    Ok(Some(request))
}

/// Write an HTTP/1.1 response.
///
/// If `is_head` is true, the status line and headers are written but the body
/// is suppressed (per RFC 9112 §6.3 — HEAD responses MUST NOT contain a body).
async fn write_response<W: AsyncWrite + Unpin>(
    writer: &mut W,
    response: &Response,
    is_head: bool,
) -> std::io::Result<()> {
    let status = response.status();
    let mut buf = format!(
        "HTTP/1.1 {} {}\r\n",
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
        buf.extend_from_slice(b"Connection: keep-alive\r\n");
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
            write_response(&mut writer, &resp, false).await.unwrap();
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
        resp.set_body(b"hello");

        let output = rt.block_on(async move {
            let mut buf = Vec::new();
            let mut writer = VecWriter(&mut buf);
            write_response(&mut writer, &resp, true).await.unwrap();
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
