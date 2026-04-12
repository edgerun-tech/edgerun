//! Async HTTP/1.1 client.
//!
//! Provides [`Client`] for making HTTP/1.1 requests with:
//! - Async DNS resolution via [`edgerun_dns::DnsClient`]
//! - Async connect via [`edgerun_rt::ConnectFuture`] with configurable timeout
//! - TLS 1.3 support (HTTPS) via [`edgerun_tls::AsyncTlsStream`] (requires `tls` feature)
//! - Chunked transfer encoding support
//! - Full body reading (buffered; streaming bodies require holding the reader)

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use edgerun_dns::DnsClient;
use edgerun_rt::{
    AsyncRead, AsyncWriteExt,
    AsyncTcpStream, AsyncReadHalf,
    ConnectFuture,
};

#[cfg(feature = "tls")]
use edgerun_tls::async_tls::AsyncTlsStream;

use crate::http1::body::AsyncBodyReader;
use crate::http1::buf_reader::BufReader;
use crate::http1::request::Request;
use crate::http1::response::Response;
use crate::{Error, HeaderMap, Method, Result, StatusCode};

/// DNS servers to try in order for hostname resolution.
const DEFAULT_DNS_SERVERS: &[&str] = &["8.8.8.8:53", "1.1.1.1:53"];

/// Async HTTP/1.1 client.
///
/// # Example
/// ```no_run
/// use edgerun_http::http1::Client;
/// use edgerun_rt::Runtime;
///
/// let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
/// rt.block_on(async {
///     let client = Client::new();
///     let response = client.get("http://example.com/").await.unwrap();
///     println!("Status: {}", response.status());
///     println!("Body: {}", response.body_as_string().unwrap());
/// });
/// ```
pub struct Client {
    connect_timeout: Duration,
    dns_timeout: Duration,
    /// Maximum redirects to follow (0 = no redirects)
    max_redirects: u8,
    /// Whether to follow redirects automatically
    follow_redirects: bool,
    /// Whether to send Accept-Encoding and decompress responses
    auto_decompress: bool,
}

impl Client {
    /// Create a new client with default settings.
    ///
    /// DNS resolution uses the first available server from
    /// `8.8.8.8:53` or `1.1.1.1:53`. Literal IP addresses
    /// skip DNS entirely.
    pub fn new() -> Self {
        Client {
            connect_timeout: Duration::from_secs(10),
            dns_timeout: Duration::from_secs(5),
            max_redirects: 10,
            follow_redirects: true,
            auto_decompress: true,
        }
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
    /// Setting to 0 disables redirect following.
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

    /// Send a POST request with JSON body.
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
    // Core execution — plain TCP
    // ------------------------------------------------------------------

    /// Execute an HTTP request over plain TCP.
    ///
    /// Resolves the hostname via [`edgerun_dns::DnsClient`], connects,
    /// sends the request, reads the full response (headers + body),
    /// and returns a [`Response`] with the body buffered in memory.
    ///
    /// If `follow_redirects` is true (default), 3xx responses with a
    /// `Location` header are followed automatically (up to `max_redirects`).
    ///
    /// If `auto_decompress` is true (default), responses with
    /// `Content-Encoding: gzip/deflate/br` are automatically decompressed.
    pub async fn execute(&self, request: &Request) -> Result<Response> {
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
            let is_head = req.method() == &Method::HEAD;

            let host = uri.host()
                .ok_or_else(|| Error::InvalidUri("No host in URI".to_string()))?;
            let port = uri.port().unwrap_or_else(|| if uri.is_https() { 443 } else { 80 });

            let stream = self.resolve_and_connect(host, port).await?;
            let (read_half, mut write_half) = stream.split();

            // Add Accept-Encoding header if auto_decompress is enabled
            let request_bytes = if self.auto_decompress {
                let mut req_bytes = req.to_http_bytes();
                let ae = format!("\r\nAccept-Encoding: {}", compression::accept_encoding_value());
                // Insert before the final \r\n\r\n
                if let Some(pos) = req_bytes.windows(4).rposition(|w| w == b"\r\n\r\n") {
                    let mut new_bytes = Vec::with_capacity(req_bytes.len() + ae.len());
                    new_bytes.extend_from_slice(&req_bytes[..pos]);
                    new_bytes.extend_from_slice(ae.as_bytes());
                    new_bytes.extend_from_slice(&req_bytes[pos..]);
                    new_bytes
                } else {
                    req_bytes
                }
            } else {
                req.to_http_bytes()
            };

            write_half.write_all(&request_bytes).await
                .map_err(Error::Network)?;

            let response = Self::read_response_plain(read_half, is_head).await?;

            // Check for redirect (3xx with Location header)
            if remaining > 0 && self.follow_redirects {
                let status = response.status().as_u16();
                if (300..400).contains(&status) {
                    if let Some(location) = response.headers().get("location") {
                        let loc = location.as_str();
                        remaining -= 1;
                        // Resolve relative URL
                        current_uri = Self::resolve_redirect_url(&current_uri, loc);
                        // For 303, use GET; for 307/308, preserve method; for 301/302, use GET for non-GET
                        if status == 303 && current_method != Method::GET && current_method != Method::HEAD {
                            current_method = Method::GET;
                        }
                        continue;
                    }
                }
            }

            // Decompress response body if auto_decompress is enabled
            if self.auto_decompress {
                let body = response.body();
                if let Some(decompressed) = compression::decompress_body(body, response.headers()) {
                    return Ok(Response::from_parts(
                        response.status().clone(),
                        response.headers().clone(),
                        decompressed,
                    ));
                }
            }

            return Ok(response);
        }
    }

    /// Resolve a redirect URL relative to the current URL.
    fn resolve_redirect_url(current: &str, location: &str) -> String {
        if location.starts_with("http://") || location.starts_with("https://") {
            return location.to_string();
        }

        // Parse current URL
        if let Some((scheme, rest)) = current.split_once("://") {
            if let Some((host_port, path)) = rest.split_once('/') {
                if location.starts_with('/') {
                    return format!("{}://{}/{}", scheme, host_port, &location[1..]);
                }
                // Relative path
                let base = path.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
                return format!("{}://{}{}/{}", scheme, host_port, base, location);
            } else {
                // No path — location is relative to host
                if location.is_empty() {
                    return format!("{}://{}/", scheme, rest);
                }
                return format!("{}://{}/{}", scheme, rest, location);
            }
        }

        location.to_string()
    }

    /// Execute a request and return a streaming body reader (plain TCP).
    pub async fn execute_streaming(
        &self,
        request: &Request,
    ) -> Result<(Response, AsyncBodyReader<AsyncReadHalf>)> {
        let uri = request.uri();
        let is_head = request.method() == &Method::HEAD;

        let host = uri.host()
            .ok_or_else(|| Error::InvalidUri("No host in URI".to_string()))?;
        let port = uri.port().unwrap_or_else(|| if uri.is_https() { 443 } else { 80 });

        let stream = self.resolve_and_connect(host, port).await?;
        let (read_half, mut write_half) = stream.split();

        let request_bytes = request.to_http_bytes();
        write_half.write_all(&request_bytes).await
            .map_err(Error::Network)?;

        Self::read_response_headers_impl(read_half, is_head).await
    }

    // ------------------------------------------------------------------
    // Core execution — TLS (HTTPS)
    // ------------------------------------------------------------------

    /// Execute an HTTPS request.
    ///
    /// Resolves the hostname via DNS, connects TCP, performs a TLS 1.3
    /// handshake, sends the request, and reads the full response.
    ///
    /// Requires the `tls` feature (enabled by `features = ["tls"]`).
    #[cfg(feature = "tls")]
    pub async fn execute_tls(&self, request: &Request) -> Result<Response> {
        let uri = request.uri();
        let is_head = request.method() == &Method::HEAD;

        let host = uri.host()
            .ok_or_else(|| Error::InvalidUri("No host in URI".to_string()))?;
        let port = uri.port().unwrap_or(443);

        let mut tls = self.resolve_and_connect_tls(host, port).await?;

        let request_bytes = request.to_http_bytes();
        tls.write_all(&request_bytes).await
            .map_err(Error::Network)?;

        Self::read_response_tls(tls, is_head).await
    }

    /// Execute an HTTPS request with streaming body reader.
    #[cfg(feature = "tls")]
    pub async fn execute_tls_streaming(
        &self,
        request: &Request,
    ) -> Result<(Response, AsyncBodyReader<AsyncTlsStream<AsyncTcpStream>>)> {
        let uri = request.uri();
        let is_head = request.method() == &Method::HEAD;

        let host = uri.host()
            .ok_or_else(|| Error::InvalidUri("No host in URI".to_string()))?;
        let port = uri.port().unwrap_or(443);

        let mut tls = self.resolve_and_connect_tls(host, port).await?;

        let request_bytes = request.to_http_bytes();
        tls.write_all(&request_bytes).await
            .map_err(Error::Network)?;

        Self::read_response_tls(tls, is_head).await
    }

    // ------------------------------------------------------------------
    // Internal helpers — DNS and TCP
    // ------------------------------------------------------------------

    /// Resolve a hostname via DNS, then connect TCP.
    async fn resolve_and_connect(&self, host: &str, port: u16) -> Result<Arc<AsyncTcpStream>> {
        if let Ok(ip) = host.parse::<IpAddr>() {
            return self.connect_sock(&SocketAddr::new(ip, port)).await;
        }

        match self.dns_resolve(host).await {
            Ok(resolved) => return self.connect_sock(&SocketAddr::new(resolved, port)).await,
            Err(_) => {}
        }

        // Fallback to sync resolution.
        self.connect(&format!("{}:{}", host, port)).await
    }

    /// Resolve hostname via DNS, trying A then AAAA records.
    async fn dns_resolve(&self, host: &str) -> Result<IpAddr> {
        let mut local_dns = DnsClient::new(DEFAULT_DNS_SERVERS[0])
            .map_err(|e| Error::ProtocolError(format!("DNS client creation failed: {}", e)))?;
        local_dns.set_timeout(self.dns_timeout);

        let ips = edgerun_rt::timeout(self.dns_timeout, local_dns.query_a(host)).await
            .map_err(|_| Error::Timeout)?
            .map_err(|e| Error::ProtocolError(format!("DNS A query failed: {}", e)))?;

        if let Some(ip) = ips.first() {
            return Ok(IpAddr::V4(*ip));
        }

        let ips6 = edgerun_rt::timeout(self.dns_timeout, local_dns.query_aaaa(host)).await
            .map_err(|_| Error::Timeout)?
            .map_err(|e| Error::ProtocolError(format!("DNS AAAA query failed: {}", e)))?;

        if let Some(ip) = ips6.first() {
            return Ok(IpAddr::V6(*ip));
        }

        Err(Error::InvalidUri(format!("DNS resolution failed for {}", host)))
    }

    async fn connect_sock(&self, addr: &SocketAddr) -> Result<Arc<AsyncTcpStream>> {
        self.connect(&addr.to_string()).await
    }

    async fn connect(&self, addr: &str) -> Result<Arc<AsyncTcpStream>> {
        let fut = ConnectFuture::new(addr);
        match edgerun_rt::timeout(self.connect_timeout, fut).await {
            Ok(Ok(stream)) => Ok(stream),
            Ok(Err(e)) => Err(Error::Network(e)),
            Err(_) => Err(Error::Timeout),
        }
    }

    // ------------------------------------------------------------------
    // Internal helpers — TLS
    // ------------------------------------------------------------------

    #[cfg(feature = "tls")]
    async fn resolve_and_connect_tls(
        &self,
        host: &str,
        port: u16,
    ) -> Result<AsyncTlsStream<AsyncTcpStream>> {
        let stream = self.resolve_and_connect(host, port).await?;
        AsyncTlsStream::client(stream, host)
            .map_err(|e| Error::ProtocolError(format!("TLS handshake failed: {}", e)))
    }

    // ------------------------------------------------------------------
    // Internal helpers — response reading (generic over transport)
    // ------------------------------------------------------------------

    async fn read_response_plain(
        read_half: AsyncReadHalf,
        is_head: bool,
    ) -> Result<Response> {
        let (resp, body_reader) = Self::read_response_headers_impl(read_half, is_head).await?;
        let body = body_reader.collect().await.map_err(|e| Error::Network(e))?;
        Ok(resp.with_body(body))
    }

    #[cfg(feature = "tls")]
    async fn read_response_tls(
        tls: AsyncTlsStream<AsyncTcpStream>,
        is_head: bool,
    ) -> Result<Response> {
        let (resp, body_reader) = Self::read_response_headers_impl(tls, is_head).await?;
        let body = body_reader.collect().await.map_err(|e| Error::Network(e))?;
        Ok(resp.with_body(body))
    }

    async fn read_response_headers_impl<R: AsyncRead + Unpin>(
        transport: R,
        is_head: bool,
    ) -> Result<(Response, AsyncBodyReader<R>)> {
        let mut buf_reader = BufReader::new(transport);

        let status_line = buf_reader.read_line().await
            .map_err(|e| Error::Network(e))?
            .ok_or_else(|| Error::InvalidResponse("Unexpected EOF reading status line".to_string()))?;

        let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
        if parts.len() < 2 {
            return Err(Error::InvalidResponse("Invalid status line".to_string()));
        }
        let status_code = parts[1].parse::<u16>()
            .map_err(|_| Error::InvalidResponse("Invalid status code".to_string()))?;
        let status = StatusCode::new(status_code)
            .map_err(|e| Error::InvalidResponse(e))?;

        let mut headers = HeaderMap::new();
        loop {
            let line = buf_reader.read_line().await
                .map_err(|e| Error::Network(e))?
                .ok_or_else(|| Error::InvalidResponse("Unexpected EOF reading headers".to_string()))?;

            if line.is_empty() { break; }

            if let Some(colon) = line.find(':') {
                let name = line[..colon].trim();
                let value = line[colon + 1..].trim();
                if !name.is_empty() {
                    let _ = headers.insert(name, value);
                }
            }
        }

        let status_code_val = status.as_u16();
        if is_head || status_code_val < 200 || status_code_val == 204 || status_code_val == 304 {
            let resp = Response::from_parts(status, headers, Vec::new());
            let body_reader = AsyncBodyReader::with_length(buf_reader.into_inner(), 0);
            return Ok((resp, body_reader));
        }

        let is_chunked = headers.get("transfer-encoding")
            .map(|v| v.as_str().to_lowercase())
            .map_or(false, |v| v.contains("chunked"));

        let content_length = headers.get("content-length")
            .and_then(|v| v.as_str().parse::<u64>().ok());

        let inner = buf_reader.into_inner();
        let body_reader = if is_chunked {
            AsyncBodyReader::chunked(inner)
        } else if let Some(len) = content_length {
            AsyncBodyReader::with_length(inner, len)
        } else {
            AsyncBodyReader::until_eof(inner)
        };

        let resp = Response::from_parts(status, headers, Vec::new());
        Ok((resp, body_reader))
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_new() {
        let client = Client::new();
        assert_eq!(client.connect_timeout, Duration::from_secs(10));
        assert_eq!(client.dns_timeout, Duration::from_secs(5));
    }

    #[test]
    fn test_client_with_timeout() {
        let client = Client::new()
            .with_connect_timeout(Duration::from_secs(30))
            .with_dns_timeout(Duration::from_secs(10));
        assert_eq!(client.connect_timeout, Duration::from_secs(30));
        assert_eq!(client.dns_timeout, Duration::from_secs(10));
    }
}
