//! HTTP/1.1 connection pool with keep-alive reuse.
//!
//! Pools established connections per host, reusing TCP+TLS connections
//! across multiple requests to eliminate connect + TLS handshake overhead.
//!
//! # Architecture
//! ```text
//! ConnectionPool
//!   └─ HashMap<PoolKey, Vec<PooledConn>>
//!         ├─ PoolKey = (host, port, is_tls)
//!         └─ PooledConn = enum { Plain(BufReader<AsyncTcpStream>), Tls(BufReader<AsyncTlsStream<...>>) }
//!
//! Request flow:
//!   1. pool.execute(&request) → try reuse pooled conn or create new
//!   2. Send request, read response
//!   3. If connection is still alive (keep-alive), return to pool
//!   4. Next request to same host reuses the pooled connection
//! ```

#[cfg(target_os = "none")]
use crate::prelude::v1::*;

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use edgerun_bare_rt::{
    timeout as rt_timeout, AsyncRead, AsyncReadExt, AsyncTcpStream, AsyncWrite, AsyncWriteExt,
    BufReader, ConnectFuture,
};

use edgerun_tls::async_tls::AsyncTlsStream;
use edgerun_tls::SessionCache;

use edgerun_bare_rt::sync::Mutex;

use crate::http1::compression;
use crate::{Error, HeaderMap, Method, Request, Response, Result, StatusCode};

fn bare_io(error: edgerun_bare_rt::IoError) -> std::io::Error {
    match error {
        edgerun_bare_rt::IoError::UnexpectedEof => {
            std::io::Error::new(std::io::ErrorKind::UnexpectedEof, error)
        }
        edgerun_bare_rt::IoError::WriteZero => {
            std::io::Error::new(std::io::ErrorKind::WriteZero, error)
        }
        edgerun_bare_rt::IoError::Other(_) => std::io::Error::other(error),
    }
}

// ===========================================================================
// Pool key
// ===========================================================================

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct PoolKey {
    host: String,
    port: u16,
    is_tls: bool,
}

// ===========================================================================
// Pooled connection (enum to handle both plain and TLS)
// ===========================================================================

/// A pooled HTTP/1.1 connection.
enum PooledConn {
    /// Plain TCP connection.
    Plain(BufReader<Arc<AsyncTcpStream>>),
    /// TLS-wrapped connection.
    Tls(BufReader<AsyncTlsStream<Arc<AsyncTcpStream>>>),
}

impl PooledConn {
    /// Read a line from the buffered connection.
    async fn read_line(&mut self) -> std::io::Result<Option<String>> {
        match self {
            PooledConn::Plain(r) => r.read_line().await.map_err(bare_io),
            PooledConn::Tls(r) => r.read_line().await.map_err(bare_io),
        }
    }

    /// Read bytes from the connection.
    async fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            PooledConn::Plain(r) => r.read(buf).await.map_err(bare_io),
            PooledConn::Tls(r) => r.read(buf).await.map_err(bare_io),
        }
    }

    /// Write the request and flush.
    async fn write_request(&mut self, data: &[u8]) -> std::io::Result<()> {
        match self {
            PooledConn::Plain(r) => {
                r.get_mut().write_all(data).await.map_err(bare_io)?;
                r.get_mut().flush().await.map_err(bare_io)
            }
            PooledConn::Tls(r) => {
                r.get_mut().write_all(data).await.map_err(bare_io)?;
                r.get_mut().flush().await.map_err(bare_io)
            }
        }
    }
}

// ===========================================================================
// ConnectionPool
// ===========================================================================

/// HTTP/1.1 connection pool.
///
/// Reuses TCP+TLS connections across requests to the same host,
/// eliminating connect + TLS handshake overhead for keep-alive servers.
///
/// # Example
/// ```ignore
/// let pool = ConnectionPool::new()
///     .with_max_connections_per_host(6)
///     .with_idle_timeout(Duration::from_secs(30));
///
/// let request = Request::builder()
///     .method(Method::GET)
///     .uri("https://api.example.com/data")
///     .build()?;
/// let response = pool.execute(&request).await?;
/// ```
pub struct ConnectionPool {
    /// Active connections per host.
    connections: HashMap<PoolKey, Vec<(PooledConn, Instant)>>,
    /// Max connections per host.
    max_per_host: usize,
    /// Idle timeout for pooled connections.
    idle_timeout: Duration,
    /// Connect timeout.
    connect_timeout: Duration,
    /// Read timeout.
    read_timeout: Duration,
    /// DNS timeout.
    dns_timeout: Duration,
    /// Whether to follow redirects.
    follow_redirects: bool,
    /// Max redirects.
    max_redirects: u8,
    /// Whether to auto-decompress.
    auto_decompress: bool,
    /// TLS session ticket cache for session resumption.
    session_cache: SessionCache,
}

impl ConnectionPool {
    /// Create a new connection pool with default settings.
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            max_per_host: 6,
            idle_timeout: Duration::from_secs(30),
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            dns_timeout: Duration::from_secs(5),
            follow_redirects: true,
            max_redirects: 10,
            auto_decompress: true,
            session_cache: SessionCache::new(),
        }
    }

    /// Set the TLS session cache for session resumption.
    /// Share the same cache across multiple pools to reuse sessions across hosts.
    pub fn with_session_cache(mut self, cache: SessionCache) -> Self {
        self.session_cache = cache;
        self
    }

    /// Set max connections per host.
    pub fn with_max_connections_per_host(mut self, n: usize) -> Self {
        self.max_per_host = n;
        self
    }

    /// Set idle timeout for pooled connections.
    pub fn with_idle_timeout(mut self, t: Duration) -> Self {
        self.idle_timeout = t;
        self
    }

    /// Set connect timeout.
    pub fn with_connect_timeout(mut self, t: Duration) -> Self {
        self.connect_timeout = t;
        self
    }

    /// Set read timeout.
    pub fn with_read_timeout(mut self, t: Duration) -> Self {
        self.read_timeout = t;
        self
    }

    /// Set DNS timeout.
    pub fn with_dns_timeout(mut self, t: Duration) -> Self {
        self.dns_timeout = t;
        self
    }

    /// Disable redirect following.
    pub fn no_redirects(mut self) -> Self {
        self.follow_redirects = false;
        self.max_redirects = 0;
        self
    }

    /// Set max redirects.
    pub fn with_max_redirects(mut self, max: u8) -> Self {
        self.max_redirects = max;
        self.follow_redirects = max > 0;
        self
    }

    /// Disable auto-decompression.
    pub fn no_decompress(mut self) -> Self {
        self.auto_decompress = false;
        self
    }

    /// Set follow_redirects (for runtime configuration).
    pub fn set_follow_redirects(&mut self, v: bool) {
        self.follow_redirects = v;
    }

    /// Set connect_timeout (for runtime configuration).
    pub fn set_connect_timeout(&mut self, t: Duration) {
        self.connect_timeout = t;
    }

    /// Set read_timeout (for runtime configuration).
    pub fn set_read_timeout(&mut self, t: Duration) {
        self.read_timeout = t;
    }

    /// Set max_redirects (for runtime configuration).
    pub fn set_max_redirects(&mut self, max: u8) {
        self.max_redirects = max;
        self.follow_redirects = max > 0;
    }

    /// Set auto_decompress (for runtime configuration).
    pub fn set_auto_decompress(&mut self, v: bool) {
        self.auto_decompress = v;
    }

    /// Execute an HTTP/1.1 request, reusing pooled connections when available.
    ///
    /// This is the simple sync-friendly API: lock, execute, release — all in one call.
    /// For use from async contexts, use [`Self::execute_async`] instead.
    ///
    /// Auto-detects HTTPS from the URI scheme.
    pub async fn execute(&mut self, request: &Request) -> Result<Response> {
        let mut current_uri = request.uri().to_string();
        let mut current_method = request.method().clone();
        let mut remaining = self.max_redirects;

        loop {
            let mut headers = request.headers().clone();
            headers.remove("Host");
            let mut req = Request::builder()
                .method(current_method.clone())
                .uri(&current_uri)
                .with_headers(headers);
            if let Some(body) = request.body() {
                req = req.body(body.to_vec());
            }
            let req = req.build()?;

            let uri = req.uri();
            let is_head = req.method() == &Method::HEAD;
            let is_https = uri.is_https();

            let host = uri
                .host()
                .ok_or_else(|| Error::InvalidUri("No host in URI".to_string()))?;
            let port = uri.port().unwrap_or(if is_https { 443 } else { 80 });

            let response = self
                .execute_single(&req, host, port, is_https, is_head)
                .await?;

            // Check for redirect
            if remaining > 0 && self.follow_redirects {
                let status = response.status().as_u16();
                if (300..400).contains(&status) {
                    if let Some(location) = response.headers().get("location") {
                        let loc = location.as_str();
                        remaining -= 1;
                        current_uri = Self::resolve_redirect_url(&current_uri, loc);
                        if status == 303
                            && current_method != Method::GET
                            && current_method != Method::HEAD
                        {
                            current_method = Method::GET;
                        }
                        continue;
                    }
                }
            }

            // Decompress if needed
            if self.auto_decompress {
                let body = response.body();
                if let Some(decompressed) = compression::decompress_body(body, response.headers()) {
                    return Ok(Response::from_parts(
                        response.status(),
                        response.headers().clone(),
                        decompressed,
                    ));
                }
            }

            return Ok(response);
        }
    }

    /// Execute a single HTTP/1.1 request with connection pooling, designed
    /// for use from async contexts where the pool is behind `Arc<Mutex<>>`.
    ///
    /// This method acquires a connection, executes the request, and releases
    /// the connection back to the pool — all without holding a lock across await.
    pub async fn execute_async(pool: &Arc<Mutex<Self>>, request: &Request) -> Result<Response> {
        let mut current_uri = request.uri().to_string();
        let mut current_method = request.method().clone();
        let mut remaining: u8;
        let follow_redirects: bool;
        let auto_decompress: bool;
        {
            let p = pool.lock();
            remaining = p.max_redirects;
            follow_redirects = p.follow_redirects;
            auto_decompress = p.auto_decompress;
        }

        loop {
            let mut headers = request.headers().clone();
            headers.remove("Host");
            let mut req = Request::builder()
                .method(current_method.clone())
                .uri(&current_uri)
                .with_headers(headers);
            if let Some(body) = request.body() {
                req = req.body(body.to_vec());
            }
            let req = req.build()?;

            let uri = req.uri();
            let is_head = req.method() == &Method::HEAD;
            let is_https = uri.is_https();

            let host = uri
                .host()
                .ok_or_else(|| Error::InvalidUri("No host in URI".to_string()))?;
            let port = uri.port().unwrap_or(if is_https { 443 } else { 80 });

            let response =
                Self::execute_single_async(pool, &req, host, port, is_https, is_head).await?;

            // Check for redirect
            if remaining > 0 && follow_redirects {
                let status = response.status().as_u16();
                if (300..400).contains(&status) {
                    if let Some(location) = response.headers().get("location") {
                        let loc = location.as_str();
                        remaining -= 1;
                        current_uri = Self::resolve_redirect_url(&current_uri, loc);
                        if status == 303
                            && current_method != Method::GET
                            && current_method != Method::HEAD
                        {
                            current_method = Method::GET;
                        }
                        continue;
                    }
                }
            }

            // Decompress if needed
            if auto_decompress {
                let body = response.body();
                if let Some(decompressed) = compression::decompress_body(body, response.headers()) {
                    return Ok(Response::from_parts(
                        response.status(),
                        response.headers().clone(),
                        decompressed,
                    ));
                }
            }

            return Ok(response);
        }
    }

    /// Execute a single request, trying the pool first.
    async fn execute_single(
        &mut self,
        request: &Request,
        host: &str,
        port: u16,
        is_https: bool,
        is_head: bool,
    ) -> Result<Response> {
        let key = PoolKey {
            host: host.to_string(),
            port,
            is_tls: is_https,
        };

        // Prune idle connections for this host
        self.prune_idle(&key);

        // Try to reuse an existing connection — we need to extract from pool,
        // use it, then put it back. Can't hold mutable borrow across await.
        let conns = self.connections.remove(&key).unwrap_or_default();
        let (result, leftover_conns) = Self::try_pooled_requests(
            conns,
            request,
            is_head,
            &self.auto_decompress,
            self.read_timeout,
        )
        .await;

        // Put leftover connections back
        if !leftover_conns.is_empty() {
            self.connections.insert(key.clone(), leftover_conns);
        }

        if let Ok((response, conn)) = result {
            // Return working connection to pool
            self.connections
                .entry(key)
                .or_default()
                .push((conn, Instant::now()));
            return Ok(response);
        }

        // All pooled connections failed — create a new one
        let pooled = self.create_connection(host, port, is_https).await?;
        let (response, conn) = self.try_request_on_conn(pooled, request, is_head).await?;
        self.connections
            .entry(key)
            .or_default()
            .push((conn, Instant::now()));
        Ok(response)
    }

    /// Execute a single request via `Arc<Mutex<ConnectionPool>>` — no lock held across await.
    async fn execute_single_async(
        pool: &Arc<Mutex<Self>>,
        request: &Request,
        host: &str,
        port: u16,
        is_https: bool,
        is_head: bool,
    ) -> Result<Response> {
        let key = PoolKey {
            host: host.to_string(),
            port,
            is_tls: is_https,
        };
        let (auto_decompress, read_timeout) = {
            let p = pool.lock();
            (p.auto_decompress, p.read_timeout)
        };

        // Prune idle (under brief lock)
        {
            let mut p = pool.lock();
            p.prune_idle(&key);
        }

        // Extract connections (brief lock)
        let conns = {
            let mut p = pool.lock();
            p.connections.remove(&key).unwrap_or_default()
        };

        // Try pooled (no lock)
        let (result, leftover_conns) =
            Self::try_pooled_requests(conns, request, is_head, &auto_decompress, read_timeout)
                .await;

        // Return leftovers (brief lock)
        if !leftover_conns.is_empty() {
            let mut p = pool.lock();
            p.connections.insert(key.clone(), leftover_conns);
        }

        if let Ok((response, conn)) = result {
            let mut p = pool.lock();
            p.connections
                .entry(key)
                .or_default()
                .push((conn, Instant::now()));
            return Ok(response);
        }

        // Create new connection (no lock)
        let (ct, dt, rt, sc) = {
            let p = pool.lock();
            (
                p.connect_timeout,
                p.dns_timeout,
                p.read_timeout,
                p.session_cache.clone(),
            )
        };
        let pooled = Self::create_connection_static(ct, dt, host, port, is_https, &sc).await?;
        let (response, conn) =
            Self::try_request_on_conn_static(pooled, request, is_head, auto_decompress, rt).await?;

        // Return to pool (brief lock)
        {
            let mut p = pool.lock();
            p.connections
                .entry(key)
                .or_default()
                .push((conn, Instant::now()));
        }
        Ok(response)
    }

    /// Try each pooled connection until one works. Returns (Ok(response, conn), remaining_conns)
    /// or (Err, remaining_conns) if all failed.
    async fn try_pooled_requests(
        mut conns: Vec<(PooledConn, Instant)>,
        request: &Request,
        is_head: bool,
        auto_decompress: &bool,
        read_timeout: Duration,
    ) -> (Result<(Response, PooledConn)>, Vec<(PooledConn, Instant)>) {
        while let Some((pooled, _last_used)) = conns.pop() {
            match Self::try_request_on_conn_static(
                pooled,
                request,
                is_head,
                *auto_decompress,
                read_timeout,
            )
            .await
            {
                Ok(result) => return (Ok(result), conns),
                Err(_) => {
                    // Connection is stale — continue
                }
            }
        }
        (
            Err(Error::Network(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "all pooled connections failed",
            ))),
            conns,
        )
    }

    /// Try a request on a connection (static method, no self borrow).
    /// Returns (response, conn) on success.
    async fn try_request_on_conn_static(
        mut conn: PooledConn,
        request: &Request,
        is_head: bool,
        auto_decompress: bool,
        read_timeout: Duration,
    ) -> Result<(Response, PooledConn)> {
        // Build request bytes
        let mut request_bytes = request.to_http_bytes();

        // Add Accept-Encoding if auto_decompress
        if auto_decompress {
            let ae = format!(
                "\r\nAccept-Encoding: {}",
                compression::accept_encoding_value()
            );
            if let Some(pos) = request_bytes.windows(4).rposition(|w| w == b"\r\n\r\n") {
                let mut new_bytes = Vec::with_capacity(request_bytes.len() + ae.len());
                new_bytes.extend_from_slice(&request_bytes[..pos]);
                new_bytes.extend_from_slice(ae.as_bytes());
                new_bytes.extend_from_slice(&request_bytes[pos..]);
                request_bytes = new_bytes;
            }
        }
        // Write request
        conn.write_request(&request_bytes)
            .await
            .map_err(Error::Network)?;

        // Read response
        let response = Self::read_response(&mut conn, is_head, read_timeout).await?;

        // Connection is still alive
        Ok((response, conn))
    }

    /// Try a request on a connection. Returns (response, conn) on success.
    /// On error, the connection is consumed (caller must drop it).
    async fn try_request_on_conn(
        &self,
        conn: PooledConn,
        request: &Request,
        is_head: bool,
    ) -> Result<(Response, PooledConn)> {
        Self::try_request_on_conn_static(
            conn,
            request,
            is_head,
            self.auto_decompress,
            self.read_timeout,
        )
        .await
    }

    /// Create a new connection (TCP + optional TLS) — static version for async use.
    async fn create_connection_static(
        connect_timeout: Duration,
        dns_timeout: Duration,
        host: &str,
        port: u16,
        is_https: bool,
        session_cache: &SessionCache,
    ) -> Result<PooledConn> {
        if is_https {
            let stream =
                Self::resolve_and_connect_static(connect_timeout, dns_timeout, host, port).await?;
            let tls = AsyncTlsStream::client(stream, host, &[], Some(session_cache))
                .await
                .map_err(|e| Error::ProtocolError(format!("TLS handshake failed: {e}")))?;
            let reader = BufReader::new(tls);
            Ok(PooledConn::Tls(reader))
        } else {
            let stream =
                Self::resolve_and_connect_static(connect_timeout, dns_timeout, host, port).await?;
            let reader = BufReader::new(stream);
            Ok(PooledConn::Plain(reader))
        }
    }

    /// Create a new connection (instance method for non-async use).
    async fn create_connection(&self, host: &str, port: u16, is_https: bool) -> Result<PooledConn> {
        Self::create_connection_static(
            self.connect_timeout,
            self.dns_timeout,
            host,
            port,
            is_https,
            &self.session_cache,
        )
        .await
    }

    /// Resolve hostname and connect TCP — static version.
    async fn resolve_and_connect_static(
        connect_timeout: Duration,
        dns_timeout: Duration,
        host: &str,
        port: u16,
    ) -> Result<Arc<AsyncTcpStream>> {
        if let Ok(ip) = host.parse::<IpAddr>() {
            return Self::connect_sock_static(connect_timeout, &SocketAddr::new(ip, port)).await;
        }

        let host_owned = host.to_string();
        let dns_result = rt_timeout(
            dns_timeout,
            edgerun_bare_rt::spawn_blocking(move || {
                use std::net::ToSocketAddrs;
                format!("{}:{}", host_owned, port).to_socket_addrs()
            }),
        )
        .await;

        if let Ok(Ok(Ok(addrs))) = dns_result {
            let addr_list: Vec<SocketAddr> = addrs.into_iter().collect();
            for addr in addr_list.iter() {
                match addr.ip() {
                    IpAddr::V4(_) => return Self::connect_sock_static(connect_timeout, addr).await,
                    IpAddr::V6(_) => continue,
                }
            }
            for addr in addr_list.into_iter() {
                if let IpAddr::V6(_) = addr.ip() {
                    return Self::connect_sock_static(connect_timeout, &addr).await;
                }
            }
        }

        if let Some(mut client) = edgerun_dns::DnsClient::system() {
            client.set_timeout(dns_timeout);
            if let Ok(ips) = rt_timeout(dns_timeout, client.query_a(host)).await {
                if let Ok(ips) = ips {
                    if let Some(ip) = ips.first() {
                        return Self::connect_sock_static(
                            connect_timeout,
                            &SocketAddr::new(IpAddr::V4(*ip), port),
                        )
                        .await;
                    }
                }
            }
            if let Ok(ips) = rt_timeout(dns_timeout, client.query_aaaa(host)).await {
                if let Ok(ips) = ips {
                    if let Some(ip) = ips.first() {
                        return Self::connect_sock_static(
                            connect_timeout,
                            &SocketAddr::new(IpAddr::V6(*ip), port),
                        )
                        .await;
                    }
                }
            }
        }

        Err(Error::InvalidUri(format!(
            "DNS resolution failed for {host}"
        )))
    }

    async fn connect_sock_static(
        connect_timeout: Duration,
        addr: &SocketAddr,
    ) -> Result<Arc<AsyncTcpStream>> {
        let fut = ConnectFuture::new(addr.to_string());
        match rt_timeout(connect_timeout, fut).await {
            Ok(Ok(stream)) => Ok(stream),
            Ok(Err(e)) => Err(Error::Network(e.into())),
            Err(_) => Err(Error::Timeout),
        }
    }

    /// Resolve hostname and connect TCP.
    async fn resolve_and_connect(&self, host: &str, port: u16) -> Result<Arc<AsyncTcpStream>> {
        Self::resolve_and_connect_static(self.connect_timeout, self.dns_timeout, host, port).await
    }

    async fn connect_sock(&self, addr: &SocketAddr) -> Result<Arc<AsyncTcpStream>> {
        Self::connect_sock_static(self.connect_timeout, addr).await
    }

    /// Prune idle connections that have exceeded the idle timeout.
    fn prune_idle(&mut self, key: &PoolKey) {
        if let Some(conns) = self.connections.get_mut(key) {
            let now = Instant::now();
            conns.retain(|(_, last_used)| now.duration_since(*last_used) < self.idle_timeout);
            if conns.is_empty() {
                self.connections.remove(key);
            }
        }
    }

    async fn read_line_with_timeout(
        conn: &mut PooledConn,
        read_timeout: Duration,
    ) -> Result<Option<String>> {
        rt_timeout(read_timeout, conn.read_line())
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(Error::Network)
    }

    async fn read_with_timeout(
        conn: &mut PooledConn,
        buf: &mut [u8],
        read_timeout: Duration,
    ) -> Result<usize> {
        rt_timeout(read_timeout, conn.read(buf))
            .await
            .map_err(|_| Error::Timeout)?
            .map_err(Error::Network)
    }

    /// Read an HTTP/1.1 response from a pooled connection.
    async fn read_response(
        conn: &mut PooledConn,
        is_head: bool,
        read_timeout: Duration,
    ) -> Result<Response> {
        let status_line = Self::read_line_with_timeout(conn, read_timeout)
            .await?
            .ok_or_else(|| {
                Error::InvalidResponse("Unexpected EOF reading status line".to_string())
            })?;

        let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
        if parts.len() < 2 {
            return Err(Error::InvalidResponse("Invalid status line".to_string()));
        }
        let status_code = parts[1]
            .parse::<u16>()
            .map_err(|_| Error::InvalidResponse("Invalid status code".to_string()))?;
        let status = StatusCode::new(status_code).map_err(Error::InvalidResponse)?;

        let mut headers = HeaderMap::new();
        loop {
            let line = Self::read_line_with_timeout(conn, read_timeout)
                .await?
                .ok_or_else(|| {
                    Error::InvalidResponse("Unexpected EOF reading headers".to_string())
                })?;
            if line.is_empty() {
                break;
            }
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
            return Ok(Response::from_parts(status, headers, Vec::new()));
        }

        // Read body
        let is_chunked = headers
            .get("transfer-encoding")
            .map(|v| v.as_str().to_lowercase())
            .is_some_and(|v| v.contains("chunked"));

        let content_length = headers
            .get("content-length")
            .and_then(|v| v.as_str().parse::<usize>().ok());

        if (300..400).contains(&status_code_val) && !is_chunked && content_length.is_none() {
            return Ok(Response::from_parts(status, headers, Vec::new()));
        }

        let body = if is_chunked {
            Self::read_chunked_body(conn, read_timeout).await?
        } else if let Some(len) = content_length {
            let mut buf = vec![0u8; len];
            let mut total = 0;
            while total < len {
                let n = Self::read_with_timeout(conn, &mut buf[total..], read_timeout).await?;
                if n == 0 {
                    break;
                }
                total += n;
            }
            buf.truncate(total);
            buf
        } else {
            let mut body = Vec::new();
            let mut buf = [0u8; 8192];
            loop {
                let n = match Self::read_with_timeout(conn, &mut buf, read_timeout).await {
                    Ok(n) => n,
                    Err(Error::Timeout) if !body.is_empty() => break,
                    Err(e) => return Err(e),
                };
                if n == 0 {
                    break;
                }
                body.extend_from_slice(&buf[..n]);
            }
            body
        };

        Ok(Response::from_parts(status, headers, body))
    }

    /// Read a chunked transfer-encoded body.
    async fn read_chunked_body(conn: &mut PooledConn, read_timeout: Duration) -> Result<Vec<u8>> {
        let mut body = Vec::new();
        let mut line_buf = Vec::with_capacity(32);

        loop {
            // Read chunk-size line byte-by-byte
            line_buf.clear();
            loop {
                let mut byte = [0u8; 1];
                let n = Self::read_with_timeout(conn, &mut byte, read_timeout).await?;
                if n == 0 {
                    return Err(Error::InvalidResponse(
                        "unexpected EOF reading chunk size".into(),
                    ));
                }
                if byte[0] == b'\n' {
                    break;
                }
                line_buf.push(byte[0]);
            }
            if line_buf.last() == Some(&b'\r') {
                line_buf.pop();
            }

            let size_hex = std::str::from_utf8(&line_buf)
                .map_err(|_| Error::InvalidResponse("invalid chunk size".into()))?;
            let size_str = size_hex.split(';').next().unwrap_or(size_hex).trim();
            let chunk_size = usize::from_str_radix(size_str, 16)
                .map_err(|_| Error::InvalidResponse("invalid chunk size".into()))?;

            if chunk_size == 0 {
                // Drain trailer headers until blank line
                loop {
                    let line = Self::read_line_with_timeout(conn, read_timeout).await?;
                    if line.is_none_or(|l| l.is_empty()) {
                        break;
                    }
                }
                break;
            }

            // Read chunk data
            let mut remaining = chunk_size;
            let mut buf = [0u8; 8192];
            while remaining > 0 {
                let to_read = remaining.min(buf.len());
                let n = Self::read_with_timeout(conn, &mut buf[..to_read], read_timeout).await?;
                if n == 0 {
                    break;
                }
                body.extend_from_slice(&buf[..n]);
                remaining -= n;
            }

            // Skip trailing \r\n
            let mut crlf = [0u8; 2];
            let _ = Self::read_with_timeout(conn, &mut crlf, read_timeout).await;
        }

        Ok(body)
    }

    /// Resolve a redirect URL relative to the current URL.
    fn resolve_redirect_url(current: &str, location: &str) -> String {
        if location.starts_with("http://") || location.starts_with("https://") {
            return location.to_string();
        }
        if let Some((scheme, rest)) = current.split_once("://") {
            if let Some((host_port, path)) = rest.split_once('/') {
                if let Some(stripped) = location.strip_prefix('/') {
                    return format!("{}://{}/{}", scheme, host_port, stripped);
                }
                let base = path.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
                return format!("{}://{}{}/{}", scheme, host_port, base, location);
            } else {
                if location.is_empty() {
                    return format!("{}://{}/", scheme, rest);
                }
                return format!("{}://{}/{}", scheme, rest, location);
            }
        }
        location.to_string()
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}
