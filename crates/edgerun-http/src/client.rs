//! Unified HTTP client supporting HTTP/1.1, HTTP/2, and HTTP/3.
//!
//! For HTTP/1.1 over HTTPS, performs a TLS 1.3 handshake using
//! `edgerun_tls::async_tls::AsyncTlsStream`.

use crate::header::HeaderMap;
use crate::http1::pool::ConnectionPool;
use crate::http2::pool::Http2Pool;
use crate::method::Method;
use crate::uri::Uri;
use crate::{Error, Request, Response, Result, StatusCode};
use edgerun_rt::sync::Mutex;
use std::sync::Arc;
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

/// Shared state for the HTTP client — the connection pool.
/// Wrapped in Arc so HttpClient is cheaply cloneable.
struct ClientInner {
    version: HttpVersion,
    connect_timeout: Duration,
    read_timeout: Duration,
    max_redirects: u8,
    follow_redirects: bool,
    auto_decompress: bool,
    pool: Arc<Mutex<ConnectionPool>>,
    h2_pool: Arc<Mutex<Http2Pool>>,
}

/// Unified HTTP client.
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
                pool: Arc::new(Mutex::new(ConnectionPool::new())),
                h2_pool: Arc::new(Mutex::new(Http2Pool::new())),
            }),
        }
    }

    fn inner(&self) -> &ClientInner { &self.inner }

    pub fn version(self, v: HttpVersion) -> Self {
        Self {
            inner: Arc::new(ClientInner {
                version: v,
                connect_timeout: self.inner.connect_timeout,
                read_timeout: self.inner.read_timeout,
                max_redirects: self.inner.max_redirects,
                follow_redirects: self.inner.follow_redirects,
                auto_decompress: self.inner.auto_decompress,
                pool: Arc::clone(&self.inner.pool),
                h2_pool: Arc::clone(&self.inner.h2_pool),
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
                pool: Arc::clone(&self.inner.pool),
                h2_pool: Arc::clone(&self.inner.h2_pool),
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
                pool: Arc::clone(&self.inner.pool),
                h2_pool: Arc::clone(&self.inner.h2_pool),
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
                pool: Arc::clone(&self.inner.pool),
                h2_pool: Arc::clone(&self.inner.h2_pool),
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
                pool: Arc::clone(&self.inner.pool),
                h2_pool: Arc::clone(&self.inner.h2_pool),
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
                pool: Arc::clone(&self.inner.pool),
                h2_pool: Arc::clone(&self.inner.h2_pool),
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
    pub async fn request(&self, method: Method, uri: &str, body: Option<Vec<u8>>) -> Result<Response> {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .body(body.unwrap_or_default())
            .build()?;
        self.execute(&request).await
    }

    /// Execute a custom request.
    pub async fn execute(&self, request: &Request) -> Result<Response> {
        match self.inner().version {
            HttpVersion::Http1 => self.execute_http1(request).await,
            HttpVersion::Http2 | HttpVersion::Http2OrHttp1 => {
                match self.execute_http2(request).await {
                    Ok(r) => Ok(r),
                    Err(_) => self.execute_http1(request).await,
                }
            }
            HttpVersion::Http3 => self.execute_http3(request).await,
            HttpVersion::Best => {
                match self.execute_http3(request).await {
                    Ok(r) => Ok(r),
                    Err(_) => {
                        match self.execute_http2(request).await {
                            Ok(r) => Ok(r),
                            Err(_) => self.execute_http1(request).await,
                        }
                    }
                }
            }
        }
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

        let h1_resp = ConnectionPool::execute_async(&self.inner.pool, &h1_req).await?;

        let mut headers = HeaderMap::new();
        for (k, v) in h1_resp.headers().iter() {
            let _ = headers.insert(k.as_str(), v.as_str());
        }

        Ok(Response::from_parts(h1_resp.status().clone(), headers, h1_resp.body().to_vec()))
    }

    /// Execute an HTTP/3 request via QUIC.
    async fn execute_http3(&self, request: &Request) -> Result<Response> {
        use crate::http3::connection::Http3Connection;
        use crate::http1::compression;

        let uri = request.uri();
        if !uri.is_https() {
            return Err(Error::ProtocolError(
                "HTTP/3 only supports HTTPS scheme".to_string()
            ));
        }

        let host = uri.host().ok_or_else(|| {
            Error::ProtocolError("HTTP/3 requires host in URI".to_string())
        })?.to_string();
        let port = uri.port().unwrap_or(443);

        let mut resp_redirect_count = 0;
        let mut current_request = request.clone();

        loop {
            let cur_uri = current_request.uri();
            let cur_host = cur_uri.host()
                .ok_or_else(|| Error::ProtocolError("No host".to_string()))?;
            let cur_port = cur_uri.port().unwrap_or(443);
            let cur_path = cur_uri.path().to_string();
            let cur_query = cur_uri.query().map(|q| format!("?{}", q)).unwrap_or_default();
            let path_and_query = format!("{}{}", cur_path, cur_query);

            let server_addr = if cur_port == 443 {
                cur_host.to_string()
            } else {
                format!("{}:{}", cur_host, cur_port)
            };

            let mut conn = Http3Connection::connect(&server_addr).await
                .map_err(|e| Error::Network(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused, e
                )))?;

            let mut headers = HeaderMap::new();
            for (k, v) in current_request.headers().iter() {
                let _ = headers.insert(k.as_str(), v.as_str());
            }

            let method = current_request.method().clone();
            let req_uri = Uri::parse(&format!("https://{}{}", server_addr, path_and_query))
                .map_err(|e| Error::ProtocolError(e.to_string()))?;

            let body = current_request.body().map(|b| b.to_vec());

            let stream_id = conn.send_request(&method, &req_uri, &headers, body).await
                .map_err(|e| Error::Network(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe, format!("{:?}", e)
                )))?;

            let response = conn.recv_response(stream_id).await
                .map_err(|e| Error::Network(std::io::Error::new(
                    std::io::ErrorKind::ConnectionReset, format!("{:?}", e)
                )))?;

            let (status, resp_headers, body) = response.ok_or_else(|| {
                Error::ProtocolError("No response received".to_string())
            })?;

            if self.inner.follow_redirects && status.is_redirection() && resp_redirect_count < self.inner.max_redirects as usize {
                if let Some(location) = resp_headers.get("location") {
                    let loc = location.as_str();
                    let _new_uri = Uri::parse(loc)
                        .map_err(|e| Error::ProtocolError(format!("Invalid redirect URI: {}", e)))?;

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
                compression::decompress_body(&body, &resp_headers)
                    .unwrap_or_else(|| body)
            } else {
                body
            };

            return Ok(Response::from_parts(status, resp_headers, body));
        }
    }

    async fn execute_http2(&self, request: &Request) -> Result<Response> {
        let h2_resp = Http2Pool::execute_async(&self.inner.h2_pool, request).await?;
        Ok(Response::from_parts(h2_resp.status, h2_resp.headers, h2_resp.body))
    }
}

impl Default for HttpClient {
    fn default() -> Self { Self::new() }
}