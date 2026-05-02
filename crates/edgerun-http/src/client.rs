//! Unified HTTP client supporting HTTP/1.1, HTTP/2, and HTTP/3.
//!
//! For HTTP/1.1 over HTTPS, performs a TLS 1.3 handshake using
//! `edgerun_tls::async_tls::AsyncTlsStream`.

use crate::header::HeaderMap;
use crate::http1::pool::ConnectionPool;
#[cfg(feature = "tls")]
use crate::http2::pool::Http2Pool;
use crate::method::Method;
use crate::runtime::time::Duration;
use crate::runtime::timeout as rt_timeout;
use crate::runtime::Mutex;
use crate::uri::Uri;
use crate::{Error, Request, Response, Result, StatusCode};
#[cfg(feature = "http3")]
use alloc::boxed::Box;
#[cfg(any(feature = "tls", feature = "http3"))]
use alloc::collections::BTreeSet;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{format, string::ToString};
#[cfg(all(feature = "http3", feature = "std"))]
use core::future::Future;

/// HTTP protocol preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpVersion {
    Http1,
    Http2,
    Http3,
    #[default]
    Best,
    Http2OrHttp1,
}

/// Shared state for the HTTP client — the connection pool.
/// Wrapped in Arc so HttpClient is cheaply cloneable.
struct ClientInner {
    version: HttpVersion,
    connect_timeout: Duration,
    read_timeout: Duration,
    max_redirects: u8,
    follow_redirects: bool,
    auto_decompress: bool,
    tls12_first: bool,
    http3_accept_invalid_certs: bool,
    pool: Arc<Mutex<ConnectionPool>>,
    #[cfg(feature = "tls")]
    h2_pool: Arc<Mutex<Http2Pool>>,
    #[cfg(feature = "tls")]
    h2_fallback_disabled_hosts: Arc<Mutex<BTreeSet<String>>>,
    #[cfg(feature = "http3")]
    h3_fallback_disabled_hosts: Arc<Mutex<BTreeSet<String>>>,
}

/// Unified HTTP client.
#[derive(Clone)]
pub struct HttpClient {
    inner: Arc<ClientInner>,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(ClientInner {
                version: HttpVersion::default(),
                connect_timeout: Duration::from_secs(10),
                read_timeout: Duration::from_secs(30),
                max_redirects: 10,
                follow_redirects: true,
                auto_decompress: true,
                tls12_first: false,
                http3_accept_invalid_certs: false,
                pool: Arc::new(Mutex::new(ConnectionPool::new())),
                #[cfg(feature = "tls")]
                h2_pool: Arc::new(Mutex::new(Http2Pool::new())),
                #[cfg(feature = "tls")]
                h2_fallback_disabled_hosts: Arc::new(Mutex::new(BTreeSet::new())),
                #[cfg(feature = "http3")]
                h3_fallback_disabled_hosts: Arc::new(Mutex::new(BTreeSet::new())),
            }),
        }
    }

    fn inner(&self) -> &ClientInner {
        &self.inner
    }

    pub fn version(self, v: HttpVersion) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                version: v,
                connect_timeout: self.inner.connect_timeout,
                read_timeout: self.inner.read_timeout,
                max_redirects: self.inner.max_redirects,
                follow_redirects: self.inner.follow_redirects,
                auto_decompress: self.inner.auto_decompress,
                tls12_first: self.inner.tls12_first,
                http3_accept_invalid_certs: self.inner.http3_accept_invalid_certs,
                pool: Arc::clone(&self.inner.pool),
                #[cfg(feature = "tls")]
                h2_pool: Arc::clone(&self.inner.h2_pool),
                #[cfg(feature = "tls")]
                h2_fallback_disabled_hosts: Arc::clone(&self.inner.h2_fallback_disabled_hosts),
                #[cfg(feature = "http3")]
                h3_fallback_disabled_hosts: Arc::clone(&self.inner.h3_fallback_disabled_hosts),
            }),
        }
    }
    pub fn with_connect_timeout(self, t: Duration) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                version: self.inner.version,
                connect_timeout: t,
                read_timeout: self.inner.read_timeout,
                max_redirects: self.inner.max_redirects,
                follow_redirects: self.inner.follow_redirects,
                auto_decompress: self.inner.auto_decompress,
                tls12_first: self.inner.tls12_first,
                http3_accept_invalid_certs: self.inner.http3_accept_invalid_certs,
                pool: Arc::clone(&self.inner.pool),
                #[cfg(feature = "tls")]
                h2_pool: Arc::clone(&self.inner.h2_pool),
                #[cfg(feature = "tls")]
                h2_fallback_disabled_hosts: Arc::clone(&self.inner.h2_fallback_disabled_hosts),
                #[cfg(feature = "http3")]
                h3_fallback_disabled_hosts: Arc::clone(&self.inner.h3_fallback_disabled_hosts),
            }),
        }
    }
    pub fn with_read_timeout(self, t: Duration) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                version: self.inner.version,
                connect_timeout: self.inner.connect_timeout,
                read_timeout: t,
                max_redirects: self.inner.max_redirects,
                follow_redirects: self.inner.follow_redirects,
                auto_decompress: self.inner.auto_decompress,
                tls12_first: self.inner.tls12_first,
                http3_accept_invalid_certs: self.inner.http3_accept_invalid_certs,
                pool: Arc::clone(&self.inner.pool),
                #[cfg(feature = "tls")]
                h2_pool: Arc::clone(&self.inner.h2_pool),
                #[cfg(feature = "tls")]
                h2_fallback_disabled_hosts: Arc::clone(&self.inner.h2_fallback_disabled_hosts),
                #[cfg(feature = "http3")]
                h3_fallback_disabled_hosts: Arc::clone(&self.inner.h3_fallback_disabled_hosts),
            }),
        }
    }
    pub fn with_max_redirects(self, max: u8) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                version: self.inner.version,
                connect_timeout: self.inner.connect_timeout,
                read_timeout: self.inner.read_timeout,
                max_redirects: max,
                follow_redirects: max > 0,
                auto_decompress: self.inner.auto_decompress,
                tls12_first: self.inner.tls12_first,
                http3_accept_invalid_certs: self.inner.http3_accept_invalid_certs,
                pool: Arc::clone(&self.inner.pool),
                #[cfg(feature = "tls")]
                h2_pool: Arc::clone(&self.inner.h2_pool),
                #[cfg(feature = "tls")]
                h2_fallback_disabled_hosts: Arc::clone(&self.inner.h2_fallback_disabled_hosts),
                #[cfg(feature = "http3")]
                h3_fallback_disabled_hosts: Arc::clone(&self.inner.h3_fallback_disabled_hosts),
            }),
        }
    }
    pub fn no_redirects(self) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                version: self.inner.version,
                connect_timeout: self.inner.connect_timeout,
                read_timeout: self.inner.read_timeout,
                max_redirects: 0,
                follow_redirects: false,
                auto_decompress: self.inner.auto_decompress,
                tls12_first: self.inner.tls12_first,
                http3_accept_invalid_certs: self.inner.http3_accept_invalid_certs,
                pool: Arc::clone(&self.inner.pool),
                #[cfg(feature = "tls")]
                h2_pool: Arc::clone(&self.inner.h2_pool),
                #[cfg(feature = "tls")]
                h2_fallback_disabled_hosts: Arc::clone(&self.inner.h2_fallback_disabled_hosts),
                #[cfg(feature = "http3")]
                h3_fallback_disabled_hosts: Arc::clone(&self.inner.h3_fallback_disabled_hosts),
            }),
        }
    }
    pub fn no_decompress(self) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                version: self.inner.version,
                connect_timeout: self.inner.connect_timeout,
                read_timeout: self.inner.read_timeout,
                max_redirects: self.inner.max_redirects,
                follow_redirects: self.inner.follow_redirects,
                auto_decompress: false,
                tls12_first: self.inner.tls12_first,
                http3_accept_invalid_certs: self.inner.http3_accept_invalid_certs,
                pool: Arc::clone(&self.inner.pool),
                #[cfg(feature = "tls")]
                h2_pool: Arc::clone(&self.inner.h2_pool),
                #[cfg(feature = "tls")]
                h2_fallback_disabled_hosts: Arc::clone(&self.inner.h2_fallback_disabled_hosts),
                #[cfg(feature = "http3")]
                h3_fallback_disabled_hosts: Arc::clone(&self.inner.h3_fallback_disabled_hosts),
            }),
        }
    }

    /// Prefer TLS 1.2 for HTTP/1.1 HTTPS connections.
    ///
    /// This is useful for compatibility-heavy navigation paths where a failed
    /// TLS 1.3 attempt would otherwise require a second TCP connection before
    /// falling back to TLS 1.2. HTTP/2 still uses the HTTP/2 TLS path.
    pub fn with_tls12_first(self, enabled: bool) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                version: self.inner.version,
                connect_timeout: self.inner.connect_timeout,
                read_timeout: self.inner.read_timeout,
                max_redirects: self.inner.max_redirects,
                follow_redirects: self.inner.follow_redirects,
                auto_decompress: self.inner.auto_decompress,
                tls12_first: enabled,
                http3_accept_invalid_certs: self.inner.http3_accept_invalid_certs,
                pool: Arc::clone(&self.inner.pool),
                #[cfg(feature = "tls")]
                h2_pool: Arc::clone(&self.inner.h2_pool),
                #[cfg(feature = "tls")]
                h2_fallback_disabled_hosts: Arc::clone(&self.inner.h2_fallback_disabled_hosts),
                #[cfg(feature = "http3")]
                h3_fallback_disabled_hosts: Arc::clone(&self.inner.h3_fallback_disabled_hosts),
            }),
        }
    }

    /// Accept invalid HTTP/3 server certificates.
    ///
    /// The current QUIC/TLS verifier does not implement full X.509 chain,
    /// hostname, and CertificateVerify validation. Keep this disabled for
    /// production use; enable it only for local same-stack tests or explicitly
    /// trusted development endpoints.
    pub fn danger_accept_invalid_http3_certs(self, enabled: bool) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                version: self.inner.version,
                connect_timeout: self.inner.connect_timeout,
                read_timeout: self.inner.read_timeout,
                max_redirects: self.inner.max_redirects,
                follow_redirects: self.inner.follow_redirects,
                auto_decompress: self.inner.auto_decompress,
                tls12_first: self.inner.tls12_first,
                http3_accept_invalid_certs: enabled,
                pool: Arc::clone(&self.inner.pool),
                #[cfg(feature = "tls")]
                h2_pool: Arc::clone(&self.inner.h2_pool),
                #[cfg(feature = "tls")]
                h2_fallback_disabled_hosts: Arc::clone(&self.inner.h2_fallback_disabled_hosts),
                #[cfg(feature = "http3")]
                h3_fallback_disabled_hosts: Arc::clone(&self.inner.h3_fallback_disabled_hosts),
            }),
        }
    }

    pub async fn get(&self, uri: &str) -> Result<Response> {
        self.request(Method::GET, uri, None).await
    }

    pub async fn post(&self, uri: &str, body: impl Into<Vec<u8>>) -> Result<Response> {
        self.request(Method::POST, uri, Some(body.into())).await
    }

    pub async fn post_json(&self, uri: &str, json: &str) -> Result<Response> {
        let request = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .json_body(json)
            .build()?;
        self.execute(&request).await
    }

    pub async fn put(&self, uri: &str, body: impl Into<Vec<u8>>) -> Result<Response> {
        self.request(Method::PUT, uri, Some(body.into())).await
    }

    pub async fn delete(&self, uri: &str) -> Result<Response> {
        self.request(Method::DELETE, uri, None).await
    }

    pub async fn patch(&self, uri: &str, body: impl Into<Vec<u8>>) -> Result<Response> {
        self.request(Method::PATCH, uri, Some(body.into())).await
    }

    pub async fn head(&self, uri: &str) -> Result<Response> {
        self.request(Method::HEAD, uri, None).await
    }

    pub async fn options(&self, uri: &str) -> Result<Response> {
        self.request(Method::OPTIONS, uri, None).await
    }

    /// Generic request with any method.
    pub async fn request(
        &self,
        method: Method,
        uri: &str,
        body: Option<Vec<u8>>,
    ) -> Result<Response> {
        let mut request = Request::builder().method(method).uri(uri);
        if let Some(body) = body {
            request = request.body(body);
        }
        let request = request.build()?;
        self.execute(&request).await
    }

    /// Execute a custom request.
    pub async fn execute(&self, request: &Request) -> Result<Response> {
        edgerun_log::debug!("CLIENT: execute version={:?}", self.inner().version);
        match self.inner().version {
            HttpVersion::Http1 => {
                edgerun_log::debug!("CLIENT: using HTTP/1.1");
                self.execute_http1(request).await
            }
            HttpVersion::Http2 => {
                edgerun_log::debug!("CLIENT: using HTTP/2");
                self.execute_http2_with_timeout(request).await
            }
            HttpVersion::Http2OrHttp1 => {
                edgerun_log::debug!("CLIENT: using HTTP/2");
                self.execute_http2_or_http1(request).await
            }
            HttpVersion::Http3 => {
                edgerun_log::debug!("CLIENT: using HTTP/3");
                self.execute_http3(request).await
            }
            HttpVersion::Best => {
                edgerun_log::debug!("CLIENT: using Best");
                self.execute_best(request).await
            }
        }
    }

    async fn execute_http2_or_http1(&self, request: &Request) -> Result<Response> {
        if self.is_h2_fallback_disabled(request) {
            return self.execute_http1(request).await;
        }
        match self.execute_http2_with_timeout(request).await {
            Ok(r) => Ok(r),
            Err(err) => {
                timing_log_http2_fallback(request, &err);
                self.disable_h2_fallback_for(request);
                self.execute_http1(request).await
            }
        }
    }

    #[cfg(all(feature = "http3", feature = "std"))]
    async fn execute_best(&self, request: &Request) -> Result<Response> {
        enum BestRace {
            Http3(Response),
            Tcp(Response),
            TcpFailed(Error),
        }

        if !request.uri().is_https() {
            return self.execute_http2_or_http1(request).await;
        }
        if self.is_h3_fallback_disabled(request) {
            return self.execute_http2_or_http1(request).await;
        }

        let h3_client = self.clone().version(HttpVersion::Http3);
        let h3_request = request.clone();
        let h3_timeout = h3_client.inner.read_timeout;
        let (h3_tx, h3_rx) = std::sync::mpsc::channel();
        let h3_rx = std::sync::Arc::new(std::sync::Mutex::new(h3_rx));
        std::thread::spawn(move || {
            let result = edgerun_rt::block_on(async move {
                match rt_timeout(h3_timeout, h3_client.execute_http3(&h3_request)).await {
                    Ok(result) => result,
                    Err(_) => Err(Error::Timeout),
                }
            });
            let _ = h3_tx.send(result);
        });

        let tcp_client = self.clone().version(HttpVersion::Http2OrHttp1);
        let mut tcp = Box::pin(async move { tcp_client.execute_http2_or_http1(request).await });
        let mut h3_done = false;
        let mut tcp_error = None;

        match edgerun_rt::poll_fn(|cx| {
            if tcp_error.is_none() {
                match tcp.as_mut().poll(cx) {
                    core::task::Poll::Ready(Ok(response)) => {
                        return core::task::Poll::Ready(BestRace::Tcp(response));
                    }
                    core::task::Poll::Ready(Err(err)) => tcp_error = Some(err),
                    core::task::Poll::Pending => {}
                }
            }

            if !h3_done {
                let h3_result = h3_rx
                    .lock()
                    .map(|receiver| receiver.try_recv())
                    .unwrap_or(Err(std::sync::mpsc::TryRecvError::Disconnected));
                match h3_result {
                    Ok(Ok(response)) => {
                        return core::task::Poll::Ready(BestRace::Http3(response));
                    }
                    Ok(Err(err)) => {
                        timing_log_http3_fallback(request, &err);
                        self.disable_h3_fallback_for(request);
                        h3_done = true;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        self.disable_h3_fallback_for(request);
                        h3_done = true;
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                }
            }

            if let Some(err) = tcp_error.take() {
                return core::task::Poll::Ready(BestRace::TcpFailed(err));
            }

            core::task::Poll::Pending
        })
        .await
        {
            BestRace::Http3(response) => {
                timing_log_best_winner(request, "http3");
                Ok(response)
            }
            BestRace::Tcp(response) => {
                timing_log_best_winner(request, "tcp");
                self.disable_h3_fallback_for(request);
                Ok(response)
            }
            BestRace::TcpFailed(err) => Err(err),
        }
    }

    #[cfg(any(not(feature = "http3"), not(feature = "std")))]
    async fn execute_best(&self, request: &Request) -> Result<Response> {
        self.execute_http2_or_http1(request).await
    }

    fn h2_fallback_key(request: &Request) -> Option<String> {
        let uri = request.uri();
        if !uri.is_https() {
            return None;
        }
        let host = uri.host()?;
        let port = uri.port().unwrap_or(443);
        Some(format!("{host}:{port}"))
    }

    #[cfg(feature = "tls")]
    fn is_h2_fallback_disabled(&self, request: &Request) -> bool {
        let Some(key) = Self::h2_fallback_key(request) else {
            return false;
        };
        self.inner.h2_fallback_disabled_hosts.lock().contains(&key)
    }

    #[cfg(not(feature = "tls"))]
    fn is_h2_fallback_disabled(&self, _request: &Request) -> bool {
        false
    }

    #[cfg(feature = "tls")]
    fn disable_h2_fallback_for(&self, request: &Request) {
        if let Some(key) = Self::h2_fallback_key(request) {
            self.inner.h2_fallback_disabled_hosts.lock().insert(key);
        }
    }

    #[cfg(not(feature = "tls"))]
    fn disable_h2_fallback_for(&self, _request: &Request) {}

    #[cfg(feature = "http3")]
    fn is_h3_fallback_disabled(&self, request: &Request) -> bool {
        let Some(key) = Self::h2_fallback_key(request) else {
            return false;
        };
        self.inner.h3_fallback_disabled_hosts.lock().contains(&key)
    }

    #[cfg(not(feature = "http3"))]
    fn is_h3_fallback_disabled(&self, _request: &Request) -> bool {
        true
    }

    #[cfg(feature = "http3")]
    fn disable_h3_fallback_for(&self, request: &Request) {
        if let Some(key) = Self::h2_fallback_key(request) {
            self.inner.h3_fallback_disabled_hosts.lock().insert(key);
        }
    }

    #[cfg(not(feature = "http3"))]
    fn disable_h3_fallback_for(&self, _request: &Request) {}

    /// Execute an HTTP/1.1 request and pass response body chunks to `on_chunk`.
    ///
    /// The returned response contains status and headers, with an empty body.
    /// This intentionally only targets HTTP/1.1 so callers that need streaming
    /// can opt into a concrete transport while the unified `execute` API remains
    /// fully materialized.
    pub async fn execute_http1_body_chunks<F>(
        &self,
        request: &Request,
        mut on_chunk: F,
    ) -> Result<Response>
    where
        F: FnMut(&[u8]) -> Result<()>,
    {
        let uri_str = request.uri().to_string();
        let mut h1_req = crate::http1::Request::builder()
            .method(request.method().clone())
            .uri(&uri_str);

        for (k, v) in request.headers().iter() {
            h1_req = h1_req.header(k.as_str(), v.as_str());
        }

        if let Some(body) = request.body() {
            h1_req = h1_req.body(body.to_vec());
        }

        let h1_req = h1_req.build()?;
        {
            let mut pool = self.inner.pool.lock();
            pool.set_connect_timeout(self.inner.connect_timeout);
            pool.set_read_timeout(self.inner.read_timeout);
            pool.set_max_redirects(0);
            pool.set_follow_redirects(false);
            pool.set_auto_decompress(false);
            pool.set_tls12_first(self.inner.tls12_first);
        }

        let h1_resp =
            ConnectionPool::execute_async_body_chunks(&self.inner.pool, &h1_req, &mut on_chunk)
                .await?;

        let mut headers = HeaderMap::new();
        for (k, v) in h1_resp.headers().iter() {
            let _ = headers.insert(k.as_str(), v.as_str());
        }

        Ok(Response::from_parts(h1_resp.status(), headers, Vec::new()))
    }

    /// Execute an HTTP/1.1 request using the connection pool.
    async fn execute_http1(&self, request: &Request) -> Result<Response> {
        let uri_str = request.uri().to_string();
        let mut h1_req = crate::http1::Request::builder()
            .method(request.method().clone())
            .uri(&uri_str);

        for (k, v) in request.headers().iter() {
            h1_req = h1_req.header(k.as_str(), v.as_str());
        }

        if let Some(body) = request.body() {
            h1_req = h1_req.body(body.to_vec());
        }

        let h1_req = h1_req.build()?;

        {
            let mut pool = self.inner.pool.lock();
            pool.set_connect_timeout(self.inner.connect_timeout);
            pool.set_read_timeout(self.inner.read_timeout);
            pool.set_max_redirects(self.inner.max_redirects);
            pool.set_follow_redirects(self.inner.follow_redirects);
            pool.set_auto_decompress(self.inner.auto_decompress);
            pool.set_tls12_first(self.inner.tls12_first);
        }

        let h1_resp = ConnectionPool::execute_async(&self.inner.pool, &h1_req).await?;

        let mut headers = HeaderMap::new();
        for (k, v) in h1_resp.headers().iter() {
            let _ = headers.insert(k.as_str(), v.as_str());
        }

        Ok(Response::from_parts(
            h1_resp.status(),
            headers,
            h1_resp.body().to_vec(),
        ))
    }

    /// Execute an HTTP/3 request via QUIC.
    #[cfg(feature = "http3")]
    async fn execute_http3(&self, request: &Request) -> Result<Response> {
        use crate::http1::compression;
        use crate::http3::connection::Http3Connection;
        use crate::http3::quic::QuicConnectOptions;

        edgerun_log::debug!("CLIENT HTTP3: execute_http3 start");
        let uri = request.uri();
        edgerun_log::debug!("CLIENT HTTP3: got uri");
        if !uri.is_https() {
            edgerun_log::debug!("HTTP/3: only supports HTTPS, falling back");
            return Err(Error::ProtocolError(
                "HTTP/3 only supports HTTPS scheme".to_string(),
            ));
        }

        let host = uri
            .host()
            .ok_or_else(|| Error::ProtocolError("HTTP/3 requires host in URI".to_string()))?
            .to_string();
        let port = uri.port().unwrap_or(443);

        edgerun_log::debug!("CLIENT HTTP3: host={}, port={}", host, port);
        edgerun_log::debug!("HTTP/3: connecting to {}:{}", host, port);

        let mut resp_redirect_count = 0;
        let mut current_request = request.clone();

        loop {
            let cur_uri = current_request.uri();
            let cur_host = cur_uri
                .host()
                .ok_or_else(|| Error::ProtocolError("No host".to_string()))?;
            let cur_port = cur_uri.port().unwrap_or(443);
            let cur_path = cur_uri.path().to_string();
            let cur_query = cur_uri
                .query()
                .map(|q| format!("?{}", q))
                .unwrap_or_default();
            let path_and_query = format!("{}{}", cur_path, cur_query);

            let server_addr = if cur_port == 443 {
                cur_host.to_string()
            } else {
                format!("{}:{}", cur_host, cur_port)
            };

            let mut conn = Http3Connection::connect_with_options(
                &server_addr,
                QuicConnectOptions {
                    accept_invalid_certs: self.inner.http3_accept_invalid_certs,
                },
            )
            .await
            .map_err(|e| {
                Error::Network(crate::runtime::io::Error::new(
                    crate::runtime::io::ErrorKind::ConnectionRefused,
                    e,
                ))
            })?;

            let mut headers = HeaderMap::new();
            for (k, v) in current_request.headers().iter() {
                let _ = headers.insert(k.as_str(), v.as_str());
            }

            let method = current_request.method().clone();
            let req_uri = Uri::parse(&format!("https://{}{}", server_addr, path_and_query))
                .map_err(|e| Error::ProtocolError(e.to_string()))?;

            let body = current_request.body().map(|b| b.to_vec());

            let stream_id = conn
                .send_request(&method, &req_uri, &headers, body)
                .await
                .map_err(|e| {
                    Error::Network(crate::runtime::io::Error::new(
                        crate::runtime::io::ErrorKind::BrokenPipe,
                        format!("{:?}", e),
                    ))
                })?;

            let response = conn.recv_response(stream_id).await.map_err(|e| {
                Error::Network(crate::runtime::io::Error::new(
                    crate::runtime::io::ErrorKind::ConnectionReset,
                    format!("{:?}", e),
                ))
            })?;

            let (status, resp_headers, body) =
                response.ok_or_else(|| Error::ProtocolError("No response received".to_string()))?;

            if self.inner.follow_redirects
                && status.is_redirection()
                && resp_redirect_count < self.inner.max_redirects as usize
            {
                if let Some(location) = resp_headers.get("location") {
                    let loc = location.as_str();
                    let _new_uri = Uri::parse(loc).map_err(|e| {
                        Error::ProtocolError(format!("Invalid redirect URI: {}", e))
                    })?;

                    current_request = Request::builder()
                        .method(if status.as_u16() == 303 {
                            Method::GET
                        } else {
                            current_request.method().clone()
                        })
                        .uri(loc)
                        .build()?;
                    resp_redirect_count += 1;
                    continue;
                }
            }

            let body = if self.inner.auto_decompress {
                compression::decompress_body(&body, &resp_headers).unwrap_or(body)
            } else {
                body
            };

            return Ok(Response::from_parts(status, resp_headers, body));
        }
    }

    #[cfg(not(feature = "http3"))]
    async fn execute_http3(&self, _request: &Request) -> Result<Response> {
        Err(Error::ProtocolError(
            "HTTP/3 requires edgerun-http http3 feature".into(),
        ))
    }

    #[cfg(feature = "tls")]
    async fn execute_http2_with_timeout(&self, request: &Request) -> Result<Response> {
        match rt_timeout(self.inner.read_timeout, self.execute_http2(request)).await {
            Ok(result) => result,
            Err(_) => Err(Error::Timeout),
        }
    }

    #[cfg(not(feature = "tls"))]
    async fn execute_http2_with_timeout(&self, request: &Request) -> Result<Response> {
        self.execute_http2(request).await
    }

    #[cfg(feature = "tls")]
    async fn execute_http2(&self, request: &Request) -> Result<Response> {
        let uri = request.uri();
        edgerun_log::debug!("HTTP/2: connecting to {}", uri);
        let h2_resp = Http2Pool::execute_async(&self.inner.h2_pool, request)
            .await
            .map_err(|e| {
                edgerun_log::debug!("HTTP/2 failed: {}", e);
                e
            })?;
        Ok(Response::from_parts(
            h2_resp.status,
            h2_resp.headers,
            h2_resp.body,
        ))
    }

    #[cfg(not(feature = "tls"))]
    async fn execute_http2(&self, _request: &Request) -> Result<Response> {
        Err(Error::ProtocolError(
            "HTTP/2 client requires edgerun-http tls feature".into(),
        ))
    }
}

fn timing_log_http2_fallback(request: &Request, err: &Error) {
    #[cfg(feature = "std")]
    if std::env::var_os("EDGERUN_TIMING").is_some() {
        eprintln!("http2.fallback url={} error={}", request.uri(), err);
    }
}

fn timing_log_http3_fallback(request: &Request, err: &Error) {
    #[cfg(feature = "std")]
    if std::env::var_os("EDGERUN_TIMING").is_some() {
        eprintln!("http3.fallback url={} error={}", request.uri(), err);
    }
}

fn timing_log_best_winner(request: &Request, protocol: &str) {
    #[cfg(feature = "std")]
    if std::env::var_os("EDGERUN_TIMING").is_some() {
        eprintln!("best.winner protocol={} url={}", protocol, request.uri());
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
