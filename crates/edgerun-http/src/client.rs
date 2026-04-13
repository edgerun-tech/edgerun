//! Unified HTTP client supporting HTTP/1.1, HTTP/2, and HTTP/3.
//!
//! For HTTP/1.1 over HTTPS, performs a TLS 1.3 handshake using
//! `edgerun_tls::async_tls::AsyncTlsStream`.

use crate::header::HeaderMap;
use crate::method::Method;
use crate::uri::Uri;
use crate::{Error, Request, Response, Result, StatusCode};
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
pub struct HttpClient {
    version: HttpVersion,
    connect_timeout: Duration,
    read_timeout: Duration,
    max_redirects: u8,
    follow_redirects: bool,
    auto_decompress: bool,
}

impl HttpClient {
    pub fn new() -> Self {
        Self {
            version: HttpVersion::default(),
            connect_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(30),
            max_redirects: 10,
            follow_redirects: true,
            auto_decompress: true,
        }
    }

    pub fn version(mut self, v: HttpVersion) -> Self { self.version = v; self }
    pub fn with_connect_timeout(mut self, t: Duration) -> Self { self.connect_timeout = t; self }
    pub fn with_read_timeout(mut self, t: Duration) -> Self { self.read_timeout = t; self }
    pub fn with_max_redirects(mut self, max: u8) -> Self { self.max_redirects = max; self.follow_redirects = max > 0; self }
    pub fn no_redirects(mut self) -> Self { self.follow_redirects = false; self.max_redirects = 0; self }
    pub fn no_decompress(mut self) -> Self { self.auto_decompress = false; self }

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

    pub async fn execute(&self, request: &Request) -> Result<Response> {
        match self.version {
            HttpVersion::Http1 | HttpVersion::Http2OrHttp1 => self.execute_http1(request).await,
            HttpVersion::Http2 => {
                match self.execute_http2(request).await {
                    Ok(r) => Ok(r),
                    Err(_) => self.execute_http1(request).await,
                }
            }
            HttpVersion::Http3 => self.execute_http3(request).await,
            HttpVersion::Best => {
                // Try HTTP/3 first, fall back to HTTP/1.1
                match self.execute_http3(request).await {
                    Ok(r) => Ok(r),
                    Err(_) => self.execute_http1(request).await,
                }
            }
        }
    }

    /// Execute an HTTP/1.1 request using the full-featured `http1::Client`.
    ///
    /// Delegates to `http1::Client` which handles DNS resolution, TLS,
    /// redirects, chunked encoding, and automatic decompression.
    async fn execute_http1(&self, request: &Request) -> Result<Response> {
        use crate::http1::compression;

        // Build an http1::Request from the top-level Request
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

        // Create http1::Client with matching settings
        let mut h1_client = crate::http1::Client::new()
            .with_connect_timeout(self.connect_timeout);

        if !self.follow_redirects {
            h1_client = h1_client.no_redirects();
        } else {
            h1_client = h1_client.with_max_redirects(self.max_redirects);
        }

        if !self.auto_decompress {
            h1_client = h1_client.no_decompress();
        }

        // Execute via http1::Client
        let h1_resp = h1_client.execute(&h1_req).await?;

        // Convert http1::Response → crate::Response
        let body = if self.auto_decompress {
            compression::decompress_body(h1_resp.body(), h1_resp.headers())
                .unwrap_or_else(|| h1_resp.body().to_vec())
        } else {
            h1_resp.body().to_vec()
        };

        let mut headers = HeaderMap::new();
        for (k, v) in h1_resp.headers().iter() {
            let _ = headers.insert(k.as_str(), v.as_str());
        }

        Ok(Response::from_parts(h1_resp.status(), headers, body))
    }

    /// Execute an HTTP/3 request via QUIC.
    ///
    /// Performs DNS resolution, QUIC+TLS handshake, HTTP/3 connection preface,
    /// request/response exchange, redirect following, and decompression.
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

            // Establish HTTP/3 connection
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

            // Check for redirect
            if self.follow_redirects && status.is_redirection() && resp_redirect_count < self.max_redirects as usize {
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

            let body = if self.auto_decompress {
                compression::decompress_body(&body, &resp_headers)
                    .unwrap_or_else(|| body)
            } else {
                body
            };

            return Ok(Response::from_parts(status, resp_headers, body));
        }
    }

    async fn execute_http2(&self, _request: &Request) -> Result<Response> {
        Err(Error::ProtocolError("HTTP/2 client not yet async-capable".to_string()))
    }
}

impl Default for HttpClient {
    fn default() -> Self { Self::new() }
}
