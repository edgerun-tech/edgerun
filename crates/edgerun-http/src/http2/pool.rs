//! HTTP/2 connection pool with stream multiplexing.
//!
//! Maintains one persistent HTTP/2 connection per host,
//! multiplexing all requests as concurrent streams over the
//! same TCP+TLS connection.

#[cfg(target_os = "none")]
use crate::prelude::v1::*;

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use edgerun_bare_rt::sync::Mutex;
use edgerun_bare_rt::{timeout as rt_timeout, AsyncTcpStream, ConnectFuture};

use edgerun_tls::async_tls::AsyncTlsStream;
use edgerun_tls::SessionCache;

use crate::http2::client::{AsyncClient, HttpResponse};
use crate::{Error, Method, Result};

/// Key for HTTP/2 connections: host + port.
#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
struct PoolKey {
    host: String,
    port: u16,
}

/// An HTTP/2 connection, wrapped for shared access.
struct Http2Conn {
    client: Option<AsyncClient>,
    last_used: Instant,
}

/// HTTP/2 connection pool.
///
/// Maintains one persistent HTTP/2 connection per host,
/// multiplexing all requests as concurrent streams.
///
/// # Thread safety
/// Use [`Http2Pool::execute_async`] with an `Arc<Mutex<Http2Pool>>`
/// to avoid holding a lock across async boundaries.
pub struct Http2Pool {
    connections: HashMap<PoolKey, Http2Conn>,
    idle_timeout: Duration,
    connect_timeout: Duration,
    dns_timeout: Duration,
    session_cache: SessionCache,
}

impl Http2Pool {
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            idle_timeout: Duration::from_secs(60),
            connect_timeout: Duration::from_secs(10),
            dns_timeout: Duration::from_secs(5),
            session_cache: SessionCache::new(),
        }
    }

    pub fn with_session_cache(mut self, cache: SessionCache) -> Self {
        self.session_cache = cache;
        self
    }

    pub fn with_idle_timeout(mut self, t: Duration) -> Self {
        self.idle_timeout = t;
        self
    }

    pub fn with_connect_timeout(mut self, t: Duration) -> Self {
        self.connect_timeout = t;
        self
    }

    pub fn with_dns_timeout(mut self, t: Duration) -> Self {
        self.dns_timeout = t;
        self
    }

    pub async fn get(&mut self, uri: &str) -> Result<HttpResponse> {
        let request = crate::Request::builder()
            .method(Method::GET)
            .uri(uri)
            .build()?;
        self.execute(&request).await
    }

    pub async fn post(&mut self, uri: &str, body: &[u8]) -> Result<HttpResponse> {
        let request = crate::Request::builder()
            .method(Method::POST)
            .uri(uri)
            .body(body.to_vec())
            .build()?;
        self.execute(&request).await
    }

    /// Execute an HTTP/2 request. Requires `&mut self` — for use from
    /// async contexts, prefer [`Self::execute_async`].
    pub async fn execute(&mut self, request: &crate::Request) -> Result<HttpResponse> {
        let uri = request.uri();
        if !uri.is_https() {
            return Err(Error::ProtocolError(
                "HTTP/2 requires HTTPS (h2c not supported)".into(),
            ));
        }

        let host = uri
            .host()
            .ok_or_else(|| Error::InvalidUri("No host in URI".into()))?
            .to_string();
        let port = uri.port().unwrap_or(443);
        let path_and_query = if let Some(q) = uri.query() {
            format!("{}?{}", uri.path(), q)
        } else {
            uri.path().to_string()
        };

        let key = PoolKey {
            host: host.clone(),
            port,
        };

        self.prune_idle();

        if !self.connections.contains_key(&key) {
            let client = self.create_connection(&host, port).await?;
            self.connections.insert(
                key.clone(),
                Http2Conn {
                    client: Some(client),
                    last_used: Instant::now(),
                },
            );
        }

        let h2_headers = Self::build_h2_headers(request, &path_and_query, &host, true);

        let conn = self
            .connections
            .get_mut(&key)
            .ok_or_else(|| Error::ProtocolError("connection not found".into()))?;
        conn.last_used = Instant::now();

        let body = request.body().map(|b| b.to_vec());
        let client = conn
            .client
            .as_mut()
            .ok_or_else(|| Error::ProtocolError("connection has no client".into()))?;
        let pending = client
            .request(&h2_headers, body)
            .await
            .map_err(|e| Error::ProtocolError(format!("HTTP/2 request failed: {e:?}")))?;
        pending
            .into_full_response()
            .await
            .map_err(|e| Error::ProtocolError(format!("HTTP/2 response failed: {e:?}")))
    }

    /// Execute via `Arc<Mutex<Http2Pool>>` — no lock held across await.
    pub async fn execute_async(
        pool: &Arc<Mutex<Self>>,
        request: &crate::Request,
    ) -> Result<HttpResponse> {
        let uri = request.uri();
        if !uri.is_https() {
            return Err(Error::ProtocolError(
                "HTTP/2 requires HTTPS (h2c not supported)".into(),
            ));
        }

        let host = uri
            .host()
            .ok_or_else(|| Error::InvalidUri("No host in URI".into()))?
            .to_string();
        let port = uri.port().unwrap_or(443);
        let path_and_query = if let Some(q) = uri.query() {
            format!("{}?{}", uri.path(), q)
        } else {
            uri.path().to_string()
        };

        let key = PoolKey {
            host: host.clone(),
            port,
        };

        // Prune + ensure connection exists (brief locks)
        let need_conn = {
            let mut p = pool.lock();
            p.prune_idle();
            !p.connections.contains_key(&key)
        };
        if need_conn {
            let (ct, dt, sc) = {
                let p = pool.lock();
                (p.connect_timeout, p.dns_timeout, p.session_cache.clone())
            };
            let client = Self::create_connection_static(ct, dt, &host, port, &sc).await?;
            let mut p2 = pool.lock();
            p2.connections.insert(
                key.clone(),
                Http2Conn {
                    client: Some(client),
                    last_used: Instant::now(),
                },
            );
        }

        let h2_headers = Self::build_h2_headers(request, &path_and_query, &host, true);

        // Extract client, make request, put it back
        let (body, mut client) = {
            let mut p = pool.lock();
            let conn = p
                .connections
                .get_mut(&key)
                .ok_or_else(|| Error::ProtocolError("connection not found".into()))?;
            conn.last_used = Instant::now();
            (
                request.body().map(|b| b.to_vec()),
                conn.client
                    .take()
                    .ok_or_else(|| Error::ProtocolError("connection has no client".into()))?,
            )
        };

        let result = client
            .request(&h2_headers, body)
            .await
            .map_err(|e| Error::ProtocolError(format!("HTTP/2 request failed: {e:?}")));

        let response = match result {
            Ok(pending) => pending
                .into_full_response()
                .await
                .map_err(|e| Error::ProtocolError(format!("HTTP/2 response failed: {e:?}"))),
            Err(e) => Err(e),
        };

        // Put client back
        {
            let mut p = pool.lock();
            if let Some(conn) = p.connections.get_mut(&key) {
                conn.client = Some(client);
            }
        }

        response
    }

    fn build_h2_headers(
        request: &crate::Request,
        path: &str,
        host: &str,
        _is_https: bool,
    ) -> Vec<(Vec<u8>, Vec<u8>)> {
        let mut headers = Vec::new();
        headers.push((
            b":method".to_vec(),
            request.method().as_str().as_bytes().to_vec(),
        ));
        headers.push((b":scheme".to_vec(), b"https".to_vec()));
        headers.push((b":authority".to_vec(), host.as_bytes().to_vec()));
        headers.push((b":path".to_vec(), path.as_bytes().to_vec()));
        for (name, value) in request.headers().iter() {
            // HTTP/2 requires lowercase header field names (RFC 9113 §8.2.1)
            let name_lower = name.as_str().to_lowercase();
            headers.push((
                name_lower.as_bytes().to_vec(),
                value.as_str().as_bytes().to_vec(),
            ));
        }
        headers
    }

    /// Create a new HTTP/2 connection (instance method).
    async fn create_connection(&self, host: &str, port: u16) -> Result<AsyncClient> {
        Self::create_connection_static(
            self.connect_timeout,
            self.dns_timeout,
            host,
            port,
            &self.session_cache,
        )
        .await
    }

    /// Create a new HTTP/2 connection (static, no self borrow).
    async fn create_connection_static(
        connect_timeout: Duration,
        dns_timeout: Duration,
        host: &str,
        port: u16,
        session_cache: &SessionCache,
    ) -> Result<AsyncClient> {
        let stream =
            Self::resolve_and_connect_static(connect_timeout, dns_timeout, host, port).await?;

        let tls = AsyncTlsStream::client(stream, host, &[b"h2"], Some(session_cache))
            .await
            .map_err(|e| Error::ProtocolError(format!("TLS handshake failed: {e}")))?;

        let client = AsyncClient::new(tls)
            .await
            .map_err(|e| Error::ProtocolError(format!("HTTP/2 connection failed: {e:?}")))?;

        Ok(client)
    }

    async fn resolve_and_connect_static(
        connect_timeout: Duration,
        dns_timeout: Duration,
        host: &str,
        port: u16,
    ) -> Result<Arc<AsyncTcpStream>> {
        if let Ok(ip) = host.parse::<IpAddr>() {
            return Self::connect_sock_static(connect_timeout, &SocketAddr::new(ip, port)).await;
        }

        let host_str = host.to_string();
        if let Ok(Ok(addrs)) = rt_timeout(
            dns_timeout,
            edgerun_bare_rt::spawn_blocking(move || {
                use std::net::ToSocketAddrs;
                format!("{}:443", host_str).to_socket_addrs()
            }),
        )
        .await
        {
            let mut ipv4_fallback = None;
            if let Ok(mut addrs) = addrs {
                for addr in addrs.by_ref() {
                    match addr.ip() {
                        IpAddr::V6(_) => {
                            return Self::connect_sock_static(connect_timeout, &addr).await
                        }
                        IpAddr::V4(_) => {
                            if ipv4_fallback.is_none() {
                                ipv4_fallback = Some(addr);
                            }
                        }
                    }
                }
            }
            if let Some(addr) = ipv4_fallback {
                return Self::connect_sock_static(connect_timeout, &addr).await;
            }
        }

        if let Some(mut client) = edgerun_dns::DnsClient::system() {
            client.set_timeout(dns_timeout);
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

    fn prune_idle(&mut self) {
        let now = Instant::now();
        self.connections
            .retain(|_, conn| now.duration_since(conn.last_used) < self.idle_timeout);
    }

    pub fn close(&mut self) {
        self.connections.clear();
    }

    pub fn connection_count(&self) -> usize {
        self.connections.len()
    }
}

impl Default for Http2Pool {
    fn default() -> Self {
        Self::new()
    }
}
