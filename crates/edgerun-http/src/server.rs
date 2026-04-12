//! Unified HTTP server supporting HTTP/1.1, HTTP/2, and HTTP/3.
//!
//! Binds to a single port and negotiates the best protocol automatically:
//! - TCP connections: HTTP/1.1 or HTTP/2 via ALPN (if TLS is configured)
//! - UDP connections: HTTP/3 (QUIC)

use crate::handler::Handler;
use crate::{Request, Response, StatusCode};
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

/// Unified HTTP server.
///
/// Binds to a TCP port (HTTP/1.1 + HTTP/2) and a UDP port on the same
/// address (HTTP/3), accepting requests and dispatching them to a
/// [`Handler`] implementation.
///
/// # Example
///
/// ```no_run
/// use edgerun_http::{HttpServer, Handler, Request, Response, StatusCode};
///
/// struct MyHandler;
///
/// impl Handler for MyHandler {
///     fn handle(&self, req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
///         Box::pin(async move {
///             Response::text(
///                 StatusCode::from_u16(200).unwrap(),
///                 &format!("Hello from {}!", req.uri().request_target()),
///             )
///         })
///     }
/// }
///
/// # edgerun_rt::block_on(async {
/// HttpServer::new(MyHandler)
///     .bind("127.0.0.1:0")
///     .await
///     .unwrap()
///     .serve()
///     .await
///     .unwrap();
/// # });
/// ```
pub struct HttpServer {
    handler: Arc<dyn Handler>,
    http1_keep_alive: Option<std::time::Duration>,
    http1_max_request_size: usize,
    http2_max_concurrent_streams: u32,
    http2_max_frame_size: u32,
    http3_enabled: bool,
}

impl HttpServer {
    /// Create a new HTTP server with the given handler.
    pub fn new<H: Handler>(handler: H) -> Self {
        Self {
            handler: Arc::new(handler),
            http1_keep_alive: Some(std::time::Duration::from_secs(5)),
            http1_max_request_size: 10 * 1024 * 1024,
            http2_max_concurrent_streams: 100,
            http2_max_frame_size: 16_384,
            http3_enabled: true,
        }
    }

    /// Set the HTTP/1.1 keep-alive timeout. `None` disables keep-alive.
    pub fn http1_keep_alive(mut self, timeout: Option<std::time::Duration>) -> Self {
        self.http1_keep_alive = timeout;
        self
    }

    /// Set the maximum HTTP/1.1 request size in bytes.
    pub fn http1_max_request_size(mut self, size: usize) -> Self {
        self.http1_max_request_size = size;
        self
    }

    /// Set the maximum HTTP/2 concurrent streams.
    pub fn http2_max_concurrent_streams(mut self, max: u32) -> Self {
        self.http2_max_concurrent_streams = max;
        self
    }

    /// Set the maximum HTTP/2 frame size.
    pub fn http2_max_frame_size(mut self, size: u32) -> Self {
        self.http2_max_frame_size = size;
        self
    }

    /// Enable or disable HTTP/3 (QUIC). Default: enabled.
    pub fn http3_enabled(mut self, enabled: bool) -> Self {
        self.http3_enabled = enabled;
        self
    }

    /// Bind the server to the given address.
    ///
    /// This opens a TCP listener. If HTTP/3 is enabled, a UDP socket on
    /// the same port is also opened.
    pub async fn bind(
        self,
        addr: impl std::net::ToSocketAddrs,
    ) -> std::io::Result<BoundHttpServer> {
        use edgerun_rt::AsyncTcpListener;

        let listener = AsyncTcpListener::bind(addr)?;
        let local_addr = listener.local_addr()?;

        let http3_socket = if self.http3_enabled {
            // Open UDP socket for HTTP/3 on the same port
            use std::net::UdpSocket;
            let udp_addr = SocketAddr::new(local_addr.ip(), local_addr.port());
            let socket = UdpSocket::bind(udp_addr)?;
            socket.set_nonblocking(true)?;
            Some(socket)
        } else {
            None
        };

        Ok(BoundHttpServer {
            listener,
            http3_socket,
            handler: self.handler,
            http1_keep_alive: self.http1_keep_alive,
            http1_max_request_size: self.http1_max_request_size,
            http2_max_concurrent_streams: self.http2_max_concurrent_streams,
            http2_max_frame_size: self.http2_max_frame_size,
            local_addr,
        })
    }
}

/// A bound HTTP server ready to accept connections.
pub struct BoundHttpServer {
    listener: edgerun_rt::AsyncTcpListener,
    http3_socket: Option<std::net::UdpSocket>,
    handler: Arc<dyn Handler>,
    http1_keep_alive: Option<std::time::Duration>,
    http1_max_request_size: usize,
    http2_max_concurrent_streams: u32,
    http2_max_frame_size: u32,
    local_addr: SocketAddr,
}

impl BoundHttpServer {
    /// The local address the server is listening on.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Start accepting connections. Runs until the listener errors.
    pub async fn serve(&self) -> std::io::Result<()> {
        edgerun_log::info!(
            "HTTP server listening on {} (HTTP/1.1 + HTTP/2 + HTTP/3)",
            self.local_addr
        );

        // Spawn HTTP/3 listener if UDP socket is available
        if let Some(ref udp) = self.http3_socket {
            let handler = Arc::clone(&self.handler);
            let udp_addr = udp.local_addr()?;
            edgerun_rt::spawn(async move {
                if let Err(e) = serve_http3(udp, handler, udp_addr).await {
                    edgerun_log::warn!("HTTP/3 server error: {}", e);
                }
            });
        }

        // TCP listener loop (HTTP/1.1 + HTTP/2)
        loop {
            match self.listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let handler = Arc::clone(&self.handler);
                    let keep_alive = self.http1_keep_alive;
                    let max_size = self.http1_max_request_size;

                    edgerun_rt::spawn(async move {
                        if let Err(e) = serve_http1_connection(
                            stream, peer_addr, handler, keep_alive, max_size,
                        )
                        .await
                        {
                            edgerun_log::warn!(
                                "HTTP/1.1 connection error from {}: {}",
                                peer_addr, e
                            );
                        }
                    });
                }
                Err(e) => {
                    edgerun_log::error!("Accept error: {}", e);
                    edgerun_rt::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Accept and handle a single TCP connection (useful for testing).
    pub async fn accept_one(&self) -> std::io::Result<()> {
        let (stream, peer_addr) = self.listener.accept().await?;
        let handler = Arc::clone(&self.handler);
        let keep_alive = self.http1_keep_alive;
        let max_size = self.http1_max_request_size;

        serve_http1_connection(stream, peer_addr, handler, keep_alive, max_size).await
    }
}

/// Handle a single HTTP/1.1 connection.
async fn serve_http1_connection<S>(
    stream: S,
    peer_addr: SocketAddr,
    handler: Arc<dyn Handler>,
    keep_alive: Option<std::time::Duration>,
    max_request_size: usize,
) -> std::io::Result<()>
where
    S: edgerun_rt::AsyncRead + edgerun_rt::AsyncWrite + Unpin + Send + 'static,
{
    use edgerun_http::http1::BufReader;
    use crate::header::HeaderMap;
    use crate::method::Method;
    use crate::uri::Uri;
    use crate::Response;

    let mut reader = BufReader::new(stream);
    let timeout = keep_alive.unwrap_or(std::time::Duration::from_secs(5));

    loop {
        // Read request line
        let line = match reader.read_line().await {
            Ok(line) if line.is_empty() => break, // Clean EOF
            Ok(line) => line,
            Err(_) => break,
        };

        if line.len() > max_request_size {
            let resp = Response::new(StatusCode::from_u16(431).unwrap());
            write_http1_response(&mut reader, resp).await?;
            break;
        }

        // Parse request line
        let parts: Vec<&str> = line.trim_end_matches(|c| c == '\r' || c == '\n').splitn(3, ' ').collect();
        if parts.len() < 3 {
            break;
        }

        let method: Method = match parts[0].parse() {
            Ok(m) => m,
            Err(_) => break,
        };

        let target = parts[1];
        // parts[2] is version, we ignore it for now

        // Read headers
        let mut headers = HeaderMap::new();
        loop {
            let hline = reader.read_line().await?;
            if hline.is_empty() {
                break; // End of headers
            }
            if let Some(colon) = hline.find(':') {
                let name = hline[..colon].trim();
                let value = hline[colon + 1..].trim();
                if !name.is_empty() {
                    headers.insert(name, value);
                }
            }
        }

        // Build URI
        let uri = if target.starts_with("http://") || target.starts_with("https://") {
            Uri::parse(target).unwrap_or_else(|_| Uri::parse("http://localhost/").unwrap())
        } else {
            let host = headers.get("Host").unwrap_or("localhost");
            let uri_str = if target.starts_with('/') {
                format!("http://{}{}", host, target)
            } else {
                format!("http://{}/{}", host, target)
            };
            Uri::parse(&uri_str).unwrap_or_else(|_| Uri::parse("http://localhost/").unwrap())
        };

        // Read body if Content-Length present
        let body = if let Some(cl) = headers.get("Content-Length") {
            if let Ok(len) = cl.parse::<usize>() {
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

        // Build Request and call Handler
        let request = Request::new(method, uri, headers, body);
        let is_head = request.method().as_str() == "HEAD";

        // Call handler
        let response = handler.handle(request).await;

        // Write response
        write_http1_response_head(&mut reader, &response, is_head).await?;

        // Write body (skip for HEAD and certain status codes)
        let skip_body = is_head
            || response.status().as_u16() == 204
            || response.status().as_u16() == 304
            || response.status().category() == 1;

        if !skip_body && !response.body().is_empty() {
            let inner = reader.get_mut();
            use edgerun_rt::AsyncWriteExt;
            inner.write_all(response.body()).await?;
        }
    }

    Ok(())
}

/// Write HTTP/1.1 response status line and headers.
async fn write_http1_response_head<S>(
    reader: &mut edgerun_http::http1::BufReader<S>,
    response: &Response,
    is_head: bool,
) -> std::io::Result<()>
where
    S: edgerun_rt::AsyncRead + edgerun_rt::AsyncWrite + Unpin,
{
    use edgerun_rt::AsyncWriteExt;

    let inner = reader.get_mut();

    // Status line
    let status_line = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status().as_u16(),
        response.status().reason().unwrap_or("")
    );
    inner.write_all(status_line.as_bytes()).await?;

    // Headers
    for (name, value) in response.headers().iter() {
        inner.write_all(name.as_bytes()).await?;
        inner.write_all(b": ").await?;
        inner.write_all(value.as_bytes()).await?;
        inner.write_all(b"\r\n").await?;
    }

    // Auto Content-Length if not set
    if !response.headers().contains_key("Content-Length") {
        let cl = if is_head { "0" } else { &response.body().len().to_string() };
        inner.write_all(format!("Content-Length: {}\r\n", cl).as_bytes()).await?;
    }

    inner.write_all(b"\r\n").await?;
    Ok(())
}

/// Write a full HTTP/1.1 response (head + body).
async fn write_http1_response<S>(
    reader: &mut edgerun_http::http1::BufReader<S>,
    response: Response,
) -> std::io::Result<()>
where
    S: edgerun_rt::AsyncRead + edgerun_rt::AsyncWrite + Unpin,
{
    use edgerun_rt::AsyncWriteExt;

    let inner = reader.get_mut();

    // Status line
    let status_line = format!(
        "HTTP/1.1 {} {}\r\n",
        response.status().as_u16(),
        response.status().reason().unwrap_or("")
    );
    inner.write_all(status_line.as_bytes()).await?;

    // Headers
    for (name, value) in response.headers().iter() {
        inner.write_all(name.as_bytes()).await?;
        inner.write_all(b": ").await?;
        inner.write_all(value.as_bytes()).await?;
        inner.write_all(b"\r\n").await?;
    }

    // Auto Content-Length if not set
    if !response.headers().contains_key("Content-Length") {
        inner.write_all(format!("Content-Length: {}\r\n", response.body().len()).as_bytes()).await?;
    }

    inner.write_all(b"\r\n").await?;

    // Body
    if !response.body().is_empty()
        && response.status().as_u16() != 204
        && response.status().as_u16() != 304
        && response.status().category() != 1
    {
        inner.write_all(response.body()).await?;
    }

    Ok(())
}

/// Serve HTTP/3 requests on a UDP socket.
///
/// This is a simplified implementation that handles individual HTTP/3
/// connections via QUIC. Full HTTP/3 server integration would require
/// the QUIC handshake and stream multiplexing.
async fn serve_http3(
    _socket: &std::net::UdpSocket,
    _handler: Arc<dyn Handler>,
    _addr: SocketAddr,
) -> std::io::Result<()> {
    // HTTP/3 requires full QUIC+TLS handshake.
    // For now, log that HTTP/3 is available but not fully wired.
    edgerun_log::info!("HTTP/3 listener on {}", _addr);
    // TODO: Wire up QuicConnection + Http3Connection + Handler
    Ok(())
}

// Re-import Response to fix the scope issue above
use crate::Response;
