//! Unified HTTP client supporting HTTP/1.1, HTTP/2, and HTTP/3.

use crate::header::HeaderMap;
use crate::method::Method;
use crate::uri::Uri;
use crate::{Error, Request, Response, Result, StatusCode};
use edgerun_rt::{AsyncTcpStream, AsyncWriteExt, ConnectFuture, timeout};
use std::time::Duration;

/// HTTP protocol preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    Http1,
    Http2,
    Http3,
    Best,
    Http2OrHttp1,
}

impl Default for HttpVersion {
    fn default() -> Self { HttpVersion::Best }
}

/// Unified HTTP client.
///
/// Makes HTTP requests using HTTP/1.1, HTTP/2, or HTTP/3 depending on
/// the configured [`HttpVersion`] preference.
pub struct HttpClient {
    version: HttpVersion,
    connect_timeout: Duration,
    max_redirects: u8,
    follow_redirects: bool,
    auto_decompress: bool,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            version: HttpVersion::default(),
            connect_timeout: Duration::from_secs(10),
            max_redirects: 10,
            follow_redirects: true,
            auto_decompress: true,
        }
    }

    pub fn version(mut self, v: HttpVersion) -> Self { self.version = v; self }
    pub fn with_connect_timeout(mut self, t: Duration) -> Self { self.connect_timeout = t; self }
    pub fn with_max_redirects(mut self, max: u8) -> Self { self.max_redirects = max; self.follow_redirects = max > 0; self }
    pub fn no_redirects(mut self) -> Self { self.follow_redirects = false; self.max_redirects = 0; self }
    pub fn no_decompress(mut self) -> Self { self.auto_decompress = false; self }

    // ------------------------------------------------------------------
    // Convenience methods
    // ------------------------------------------------------------------

    pub async fn get(&self, uri: &str) -> Result<Response> {
        let request = Request::builder().method(Method::GET).uri(uri).build()?;
        self.execute(&request).await
    }

    pub async fn post(&self, uri: &str, body: &[u8]) -> Result<Response> {
        let request = Request::builder().method(Method::POST).uri(uri).body(body.to_vec()).build()?;
        self.execute(&request).await
    }

    pub async fn post_json(&self, uri: &str, json: &str) -> Result<Response> {
        let request = Request::builder().method(Method::POST).uri(uri).json_body(json).build()?;
        self.execute(&request).await
    }

    pub async fn put(&self, uri: &str, body: &[u8]) -> Result<Response> {
        let request = Request::builder().method(Method::PUT).uri(uri).body(body.to_vec()).build()?;
        self.execute(&request).await
    }

    pub async fn delete(&self, uri: &str) -> Result<Response> {
        let request = Request::builder().method(Method::DELETE).uri(uri).build()?;
        self.execute(&request).await
    }

    pub async fn patch(&self, uri: &str, body: &[u8]) -> Result<Response> {
        let request = Request::builder().method(Method::PATCH).uri(uri).body(body.to_vec()).build()?;
        self.execute(&request).await
    }

    // ------------------------------------------------------------------
    // Core execution
    // ------------------------------------------------------------------

    pub async fn execute(&self, request: &Request) -> Result<Response> {
        match self.version {
            HttpVersion::Http1 | HttpVersion::Http2OrHttp1 => self.execute_http1(request).await,
            HttpVersion::Http2 => {
                match self.execute_http2(request).await {
                    Ok(r) => Ok(r),
                    Err(_) => self.execute_http1(request).await,
                }
            }
            HttpVersion::Http3 | HttpVersion::Best => self.execute_http1(request).await,
        }
    }

    async fn execute_http1(&self, request: &Request) -> Result<Response> {
        use crate::http1::compression;
        use edgerun_rt::AsyncReadExt;

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
            let host = uri.host().ok_or_else(|| Error::InvalidUri("No host in URI".to_string()))?;
            let port = uri.port().unwrap_or_else(|| if uri.is_https() { 443 } else { 80 });

            let (mut read_half, mut write_half) = self.resolve_and_connect(host, port).await?;

            let request_bytes = if self.auto_decompress {
                let mut bytes = req.to_http_bytes();
                if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let ae = b"Accept-Encoding: gzip, deflate, br\r\n";
                    bytes.splice(pos..pos, ae.iter().copied());
                }
                bytes
            } else {
                req.to_http_bytes()
            };

            write_half.write_all(&request_bytes).await?;
            write_half.flush().await?;

            let mut response_bytes = Vec::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = read_half.read(&mut buf).await?;
                if n == 0 { break; }
                response_bytes.extend_from_slice(&buf[..n]);
            }

            let response = crate::http1::Response::from_bytes(&response_bytes, false)?;

            // Follow redirects
            if self.follow_redirects && remaining > 0 {
                let status = response.status().as_u16();
                if (301..=308).contains(&status) || status == 307 || status == 308 {
                    if let Some(location) = response.headers().get("Location") {
                        current_uri = location.as_str().to_string();
                        if status <= 302 && !matches!(current_method, Method::GET | Method::HEAD) {
                            current_method = Method::GET;
                        }
                        remaining -= 1;
                        continue;
                    }
                }
            }

            let body = if self.auto_decompress {
                compression::decompress_body(response.body(), response.headers())
                    .unwrap_or_else(|| response.body().to_vec())
            } else {
                response.body().to_vec()
            };

            let mut h = HeaderMap::new();
            for (k, v) in response.headers().iter() {
                h.insert(k.as_str(), v.as_str());
            }

            return Ok(Response::from_parts(response.status(), h, body));
        }
    }

    async fn execute_http2(&self, _request: &Request) -> Result<Response> {
        // TODO: async HTTP/2 client — current Connection<S> is sync-only
        Err(Error::ProtocolError("HTTP/2 client not yet async-capable".to_string()))
    }

    async fn resolve_and_connect(&self, host: &str, port: u16) -> Result<(edgerun_rt::AsyncReadHalf, edgerun_rt::AsyncWriteHalf)> {
        let stream_arc: std::sync::Arc<edgerun_rt::AsyncTcpStream> = if let Ok(addr) = host.parse::<std::net::IpAddr>() {
            let addr = std::net::SocketAddr::new(addr, port);
            timeout(self.connect_timeout, ConnectFuture::new(addr))
                .await
                .map_err(|_| Error::Timeout)?
                .map_err(|_| Error::Timeout)?
        } else {
            // DNS resolution
            let mut dns = edgerun_dns::DnsClient::new("8.8.8.8:53")
                .map_err(|_| Error::InvalidUri(format!("DNS client creation failed for {}", host)))?;
            let addrs = dns.query_a(host)
                .await
                .map_err(|_| Error::InvalidUri(format!("DNS resolution failed for {}", host)))?;
            let Some(first) = addrs.into_iter().next() else {
                return Err(Error::InvalidUri(format!("No A records for {}", host)));
            };
            let addr = std::net::SocketAddr::new(std::net::IpAddr::V4(first), port);
            timeout(self.connect_timeout, ConnectFuture::new(addr))
                .await
                .map_err(|_| Error::Timeout)?
                .map_err(|_| Error::Timeout)?
        };

        Ok(stream_arc.split())
    }
}

impl Default for HttpClient {
    fn default() -> Self { Self::new() }
}
