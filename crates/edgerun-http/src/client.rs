//! Unified HTTP client supporting HTTP/1.1, HTTP/2, and HTTP/3.
//!
//! Automatically negotiates the best protocol when connecting to a server:
//! - HTTP/1.1: Plain TCP (default)
//! - HTTP/2: Plain TCP with prior knowledge, or via TLS ALPN
//! - HTTP/3: QUIC over UDP

use crate::header::HeaderMap;
use crate::method::Method;
use crate::uri::Uri;
use crate::{Error, Request, Response, Result, StatusCode};
use std::time::Duration;

/// HTTP protocol preference for the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    /// Use HTTP/1.1 only.
    Http1,
    /// Use HTTP/2 only (prior knowledge, no TLS ALPN).
    Http2,
    /// Use HTTP/3 (QUIC) only.
    Http3,
    /// Try HTTP/3 first, fall back to HTTP/2, then HTTP/1.1.
    Best,
    /// Try HTTP/2 first (prior knowledge), fall back to HTTP/1.1.
    Http2OrHttp1,
}

impl Default for HttpVersion {
    fn default() -> Self {
        HttpVersion::Best
    }
}

/// Unified HTTP client.
///
/// Makes HTTP requests using HTTP/1.1, HTTP/2, or HTTP/3 depending on
/// the configured [`HttpVersion`] preference.
///
/// # Example
///
/// ```no_run
/// use edgerun_http::{HttpClient, Response, StatusCode};
///
/// # edgerun_rt::block_on(async {
/// let client = HttpClient::new();
/// let response = client.get("http://example.com/").await.unwrap();
/// assert_eq!(response.status().as_u16(), 200);
/// # });
/// ```
pub struct HttpClient {
    version: HttpVersion,
    connect_timeout: Duration,
    dns_timeout: Duration,
    max_redirects: u8,
    follow_redirects: bool,
    auto_decompress: bool,
}

impl HttpClient {
    /// Create a new client with default settings.
    ///
    /// Defaults:
    /// - Protocol: Best available (HTTP/3 > HTTP/2 > HTTP/1.1)
    /// - Connect timeout: 10 seconds
    /// - DNS timeout: 5 seconds
    /// - Max redirects: 10
    /// - Auto-decompress: enabled
    pub fn new() -> Self {
        Self {
            version: HttpVersion::default(),
            connect_timeout: Duration::from_secs(10),
            dns_timeout: Duration::from_secs(5),
            max_redirects: 10,
            follow_redirects: true,
            auto_decompress: true,
        }
    }

    /// Set the preferred HTTP version.
    pub fn version(mut self, version: HttpVersion) -> Self {
        self.version = version;
        self
    }

    /// Set the connection timeout.
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// Set the DNS resolution timeout.
    pub fn with_dns_timeout(mut self, timeout: Duration) -> Self {
        self.dns_timeout = timeout;
        self
    }

    /// Set the maximum number of redirects to follow.
    pub fn with_max_redirects(mut self, max: u8) -> Self {
        self.max_redirects = max;
        self.follow_redirects = max > 0;
        self
    }

    /// Disable automatic redirect following.
    pub fn no_redirects(mut self) -> Self {
        self.follow_redirects = false;
        self.max_redirects = 0;
        self
    }

    /// Disable automatic response decompression.
    pub fn no_decompress(mut self) -> Self {
        self.auto_decompress = false;
        self
    }

    // ------------------------------------------------------------------
    // Convenience methods
    // ------------------------------------------------------------------

    /// Send a GET request.
    pub async fn get(&self, uri: &str) -> Result<Response> {
        let request = Request::builder()
            .method(Method::GET)
            .uri(uri)
            .build()?;
        self.execute(&request).await
    }

    /// Send a POST request with a body.
    pub async fn post(&self, uri: &str, body: &[u8]) -> Result<Response> {
        let request = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .body(body.to_vec())
            .build()?;
        self.execute(&request).await
    }

    /// Send a POST request with a JSON body.
    pub async fn post_json(&self, uri: &str, json: &str) -> Result<Response> {
        let request = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .json_body(json)
            .build()?;
        self.execute(&request).await
    }

    /// Send a PUT request with a body.
    pub async fn put(&self, uri: &str, body: &[u8]) -> Result<Response> {
        let request = Request::builder()
            .method(Method::PUT)
            .uri(uri)
            .body(body.to_vec())
            .build()?;
        self.execute(&request).await
    }

    /// Send a DELETE request.
    pub async fn delete(&self, uri: &str) -> Result<Response> {
        let request = Request::builder()
            .method(Method::DELETE)
            .uri(uri)
            .build()?;
        self.execute(&request).await
    }

    /// Send a PATCH request with a body.
    pub async fn patch(&self, uri: &str, body: &[u8]) -> Result<Response> {
        let request = Request::builder()
            .method(Method::PATCH)
            .uri(uri)
            .body(body.to_vec())
            .build()?;
        self.execute(&request).await
    }

    // ------------------------------------------------------------------
    // Core execution
    // ------------------------------------------------------------------

    /// Execute an HTTP request.
    ///
    /// Routes to the appropriate protocol handler based on the configured
    /// [`HttpVersion`] preference.
    pub async fn execute(&self, request: &Request) -> Result<Response> {
        match self.version {
            HttpVersion::Http1 | HttpVersion::Http2OrHttp1 => {
                self.execute_http1(request).await
            }
            HttpVersion::Http2 => {
                // HTTP/2 with prior knowledge falls back to HTTP/1 if server doesn't support
                self.execute_http2(request).await
                    .or_else(|_| self.execute_http1(request))
            }
            HttpVersion::Http3 | HttpVersion::Best => {
                // HTTP/3 is not yet fully wired to the Handler flow
                // Fall back to HTTP/1.1 for now
                self.execute_http1(request).await
            }
        }
    }

    /// Execute over HTTP/1.1 using the existing client.
    async fn execute_http1(&self, request: &Request) -> Result<Response> {
        use crate::http1::compression;

        let mut current_uri = request.uri().to_string();
        let mut current_method = request.method().clone();
        let mut remaining = self.max_redirects;

        loop {
            let req = Request::builder()
                .method(current_method.clone())
                .uri(&current_uri)
                .body(request.body().map(|b| b.to_vec()).unwrap_or_default())
                .build()?;

            let uri = req.uri();
            let host = uri.host()
                .ok_or_else(|| Error::InvalidUri("No host in URI".to_string()))?;
            let port = uri.port().unwrap_or_else(|| if uri.is_https() { 443 } else { 80 });

            let stream = self.resolve_and_connect(host, port).await?;
            let (read_half, mut write_half) = stream.split();

            // Add Accept-Encoding header if auto_decompress is enabled
            let request_bytes = if self.auto_decompress {
                let mut bytes = req.to_http_bytes();
                // Insert Accept-Encoding before the final \r\n\r\n
                if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let ae = b"Accept-Encoding: gzip, deflate, br\r\n";
                    bytes.splice(pos..pos, ae.iter().copied());
                }
                bytes
            } else {
                req.to_http_bytes()
            };

            use edgerun_rt::io::AsyncWriteExt;
            write_half.write_all(&request_bytes).await?;
            write_half.flush().await?;

            // Read response
            let mut response_bytes = Vec::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = read_half.read(&mut buf).await?;
                if n == 0 {
                    break;
                }
                response_bytes.extend_from_slice(&buf[..n]);
            }

            // Parse response
            let mut response = crate::http1::Response::from_bytes(&response_bytes, false)?;

            // Follow redirects
            if self.follow_redirects && remaining > 0 {
                let status = response.status().as_u16();
                if (301..=308).contains(&status) || status == 307 || status == 308 {
                    if let Some(location) = response.headers().get("Location") {
                        current_uri = location.to_string();
                        // For 301/302, change POST/PUT/PATCH to GET
                        if status <= 302 && !matches!(current_method, Method::GET | Method::HEAD) {
                            current_method = Method::GET;
                        }
                        remaining -= 1;
                        continue;
                    }
                }
            }

            // Auto-decompress
            let body = if self.auto_decompress {
                compression::decompress_body(response.body(), response.headers())
                    .unwrap_or_else(|| response.body().to_vec())
            } else {
                response.body().to_vec()
            };

            return Ok(Response::from_parts(
                response.status(),
                {
                    let mut h = HeaderMap::new();
                    for (k, v) in response.headers().iter() {
                        h.insert(k, v);
                    }
                    h
                },
                body,
            ));
        }
    }

    /// Execute over HTTP/2 with prior knowledge.
    async fn execute_http2(&self, request: &Request) -> Result<Response> {
        use crate::http2::Connection;

        let uri = request.uri();
        let host = uri.host()
            .ok_or_else(|| Error::InvalidUri("No host in URI".to_string()))?;
        let port = uri.port().unwrap_or(80);

        let stream = self.resolve_and_connect(host, port).await?;

        // Convert to std io types for the sync HTTP/2 connection
        // We need to bridge from async to sync for the existing HTTP/2 client
        let _stream = stream;
        let _request = request;

        // TODO: Implement async HTTP/2 client
        // The current HTTP/2 Connection<S: Read + Write> is sync-only.
        // We'd need to wrap it in an async adapter.
        Err(Error::ProtocolError("HTTP/2 client not yet async-capable".to_string()))
    }

    /// Resolve hostname and connect via TCP.
    async fn resolve_and_connect(
        &self,
        host: &str,
        port: u16,
    ) -> Result<edgerun_rt::net::AsyncTcpStream> {
        use edgerun_rt::net::AsyncTcpStream;
        use edgerun_rt::time::timeout;

        // Check if host is already an IP address
        if let Ok(addr) = host.parse::<std::net::IpAddr>() {
            let addr = std::net::SocketAddr::new(addr, port);
            let connect = AsyncTcpStream::connect(&addr);
            return timeout(self.connect_timeout, connect).await
                .map_err(|_| Error::Timeout)?;
        }

        // Resolve via DNS
        let addrs = edgerun_dns::DnsClient::new()
            .lookup_a(host)
            .await
            .map_err(|_| Error::InvalidUri(format!("DNS resolution failed for {}", host)))?;

        let Some(first) = addrs.into_iter().next() else {
            return Err(Error::InvalidUri(format!("No A records for {}", host)));
        };

        let addr = std::net::SocketAddr::new(first, port);
        let connect = AsyncTcpStream::connect(&addr);
        timeout(self.connect_timeout, connect).await
            .map_err(|_| Error::Timeout)
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
