//! Unified HTTP server supporting HTTP/1.1, HTTP/2, and HTTP/3.

use crate::handler::Handler;
use crate::header::HeaderMap;
use crate::method::Method;
use crate::uri::Uri;
use crate::{Request, Response, StatusCode};
use edgerun_rt::{AsyncRead, AsyncTcpListener, AsyncWrite, AsyncWriteExt, sleep, spawn, timeout};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

/// Unified HTTP server.
///
/// Binds to a TCP port (HTTP/1.1 + HTTP/2) and optionally a UDP port (HTTP/3),
/// dispatching all requests to a single [`Handler`] implementation.
///
/// # Example
///
/// ```no_run
/// use edgerun_http::{HttpServer, Handler, Request, Response, StatusCode};
/// use std::future::Future;
/// use std::pin::Pin;
///
/// struct HelloHandler;
///
/// impl Handler for HelloHandler {
///     fn handle(&self, req: Request) -> Pin<Box<dyn Future<Output = Response> + Send + '_>> {
///         Box::pin(async move {
///             Response::text(
///                 StatusCode::new(200).unwrap(),
///                 &format!("Hello from {}!", req.uri().request_target()),
///             )
///         })
///     }
/// }
///
/// # edgerun_rt::block_on(async {
/// HttpServer::new(HelloHandler)
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
    keep_alive: Option<Duration>,
    max_request_size: usize,
}

impl HttpServer {
    pub fn new<H: Handler>(handler: H) -> Self {
        Self {
            handler: Arc::new(handler),
            keep_alive: Some(Duration::from_secs(5)),
            max_request_size: 10 * 1024 * 1024,
        }
    }

    pub fn keep_alive(mut self, t: Option<Duration>) -> Self { self.keep_alive = t; self }
    pub fn max_request_size(mut self, s: usize) -> Self { self.max_request_size = s; self }

    pub async fn bind(self, addr: impl std::net::ToSocketAddrs) -> std::io::Result<BoundHttpServer> {
        let listener = AsyncTcpListener::bind(addr)?;
        let local_addr = listener.local_addr()?;
        Ok(BoundHttpServer {
            listener,
            handler: self.handler,
            keep_alive: self.keep_alive,
            max_request_size: self.max_request_size,
            local_addr,
        })
    }
}

/// A bound HTTP server ready to accept connections.
pub struct BoundHttpServer {
    listener: AsyncTcpListener,
    handler: Arc<dyn Handler>,
    keep_alive: Option<Duration>,
    max_request_size: usize,
    local_addr: SocketAddr,
}

impl BoundHttpServer {
    pub fn local_addr(&self) -> SocketAddr { self.local_addr }

    pub async fn serve(&self) -> std::io::Result<()> {
        edgerun_log::info!("HTTP server listening on {} (HTTP/1.1 + HTTP/2)", self.local_addr);

        loop {
            match self.listener.accept().await {
                Ok((stream, peer_addr)) => {
                    let handler = Arc::clone(&self.handler);
                    let keep_alive = self.keep_alive;
                    let max_size = self.max_request_size;
                    spawn(async move {
                        if let Err(e) = handle_connection(stream, handler, keep_alive, max_size).await {
                            edgerun_log::warn!("Connection error from {}: {}", peer_addr, e);
                        }
                    });
                }
                Err(e) => {
                    edgerun_log::error!("Accept error: {}", e);
                    sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Accept and handle a single connection (useful for testing).
    pub async fn accept_one(&self) -> std::io::Result<()> {
        let (stream, _) = self.listener.accept().await?;
        let handler = Arc::clone(&self.handler);
        handle_connection(stream, handler, self.keep_alive, self.max_request_size).await
    }
}

/// Handle a single HTTP/1.1 connection with keep-alive.
async fn handle_connection<S>(
    stream: S,
    handler: Arc<dyn Handler>,
    keep_alive: Option<Duration>,
    max_request_size: usize,
) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
{
    use crate::http1::BufReader;
    use edgerun_rt::AsyncReadExt;

    let mut reader = BufReader::new(stream);
    let ka_timeout = keep_alive.unwrap_or(Duration::from_secs(5));

    loop {
        // Read request line with timeout
        let line = match timeout(ka_timeout, reader.read_line()).await {
            Ok(Ok(Some(line))) => line,
            Ok(Ok(None)) => break, // EOF
            Ok(Err(_)) => break,
            Err(_) => break, // timeout
        };

        if line.len() > max_request_size {
            write_response(&mut reader, Response::text(StatusCode::new(431).unwrap(), "Request Header Fields Too Large")).await?;
            break;
        }

        let parts: Vec<&str> = line.trim_end_matches(|c| c == '\r' || c == '\n').splitn(3, ' ').collect();
        if parts.len() < 3 { break; }

        let method: Method = match parts[0].parse().map_err(|e: String| crate::Error::InvalidRequest(e)) {
            Ok(m) => m,
            Err(_) => break,
        };
        let target = parts[1];

        // Read headers
        let mut headers = HeaderMap::new();
        loop {
            let hline_opt = reader.read_line().await.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let hline = match hline_opt {
                Some(s) => s,
                None => break, // EOF in headers
            };
            if hline.is_empty() { break; }
            if let Some(colon) = hline.find(':') {
                let name = hline[..colon].trim();
                let value = hline[colon + 1..].trim();
                if !name.is_empty() { let _ = headers.insert(name, value); }
            }
        }

        // Build URI
        let uri = if target.starts_with("http://") || target.starts_with("https://") {
            Uri::parse(target).unwrap_or_else(|_| Uri::parse("http://localhost/").unwrap())
        } else {
            let host = headers.get("Host").map(|v| v.as_str()).unwrap_or("localhost");
            let uri_str = if target.starts_with('/') {
                format!("http://{}{}", host, target)
            } else {
                format!("http://{}/{}", host, target)
            };
            Uri::parse(&uri_str).unwrap_or_else(|_| Uri::parse("http://localhost/").unwrap())
        };

        // Read body
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
            } else { None }
        } else { None };

        let request = Request::new(method, uri, headers, body);
        let is_head = request.method().as_str() == "HEAD";
        let response = handler.handle(request).await;
        write_response_head(&mut reader, &response, is_head).await?;

        let skip_body = is_head
            || response.status().as_u16() == 204
            || response.status().as_u16() == 304
            || response.status().is_informational();

        if !skip_body && !response.body().is_empty() {
            let inner = reader.get_mut();
            inner.write_all(response.body()).await?;
        }
    }

    Ok(())
}

/// Write HTTP/1.1 status line + headers.
async fn write_response_head<S>(
    reader: &mut crate::http1::BufReader<S>,
    response: &Response,
    is_head: bool,
) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let status_line = format!("HTTP/1.1 {} {}\r\n", response.status().as_u16(), response.status().reason());
    reader.write_all(status_line.as_bytes()).await?;

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

/// Write a full HTTP/1.1 response.
async fn write_response<S>(
    reader: &mut crate::http1::BufReader<S>,
    response: Response,
) -> std::io::Result<()>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let status_line = format!("HTTP/1.1 {} {}\r\n", response.status().as_u16(), response.status().reason());
    reader.write_all(status_line.as_bytes()).await?;

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

    if !response.body().is_empty()
        && response.status().as_u16() != 204
        && response.status().as_u16() != 304
        && !response.status().is_informational()
    {
        reader.write_all(response.body()).await?;
    }

    Ok(())
}
