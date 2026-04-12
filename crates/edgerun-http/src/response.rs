//! Protocol-agnostic HTTP response.

use crate::header::HeaderMap;
use crate::status::StatusCode;
use std::fmt;

/// An HTTP response, protocol-agnostic.
///
/// Returned by [`crate::Handler`] implementations across HTTP/1.1, HTTP/2,
/// and HTTP/3 servers. Also used by all three client implementations.
#[derive(Debug, Clone)]
pub struct Response {
    status: StatusCode,
    headers: HeaderMap,
    body: Vec<u8>,
    trailers: HeaderMap,
}

impl Response {
    pub fn new(status: StatusCode) -> Self {
        Self { status, headers: HeaderMap::new(), body: Vec::new(), trailers: HeaderMap::new() }
    }

    pub fn from_parts(status: StatusCode, headers: HeaderMap, body: Vec<u8>) -> Self {
        Self { status, headers, body, trailers: HeaderMap::new() }
    }

    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        let body = body.into();
        if !self.headers.contains_key("Content-Length") {
            let _ = self.headers.insert("Content-Length", &body.len().to_string());
        }
        self.body = body;
        self
    }

    pub fn text(status: StatusCode, body: &str) -> Self {
        Self::new(status)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_body(body)
    }

    pub fn json(status: StatusCode, body: &str) -> Self {
        Self::new(status)
            .with_header("Content-Type", "application/json")
            .with_body(body)
    }

    pub fn html(status: StatusCode, body: &str) -> Self {
        Self::new(status)
            .with_header("Content-Type", "text/html; charset=utf-8")
            .with_body(body)
    }

    pub fn not_found() -> Self {
        Self::text(StatusCode::new(404).unwrap(), "404 Not Found")
    }

    pub fn internal_error() -> Self {
        Self::text(StatusCode::new(500).unwrap(), "500 Internal Server Error")
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        let _ = self.headers.insert(name, value);
        self
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap { &mut self.headers }
    pub fn status(&self) -> StatusCode { self.status }
    pub fn headers(&self) -> &HeaderMap { &self.headers }
    pub fn body(&self) -> &[u8] { &self.body }
    pub fn body_as_string(&self) -> Result<&str, std::str::Utf8Error> { std::str::from_utf8(&self.body) }
    pub fn trailers(&self) -> &HeaderMap { &self.trailers }
    pub fn set_body(&mut self, body: Vec<u8>) { self.body = body; }
    pub fn set_headers(&mut self, headers: HeaderMap) { self.headers = headers; }
    pub fn is_success(&self) -> bool { self.status.is_success() }

    /// Serialize the response as raw HTTP/1.1 bytes.
    pub fn to_http_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(b"HTTP/1.1 ");
        buf.extend_from_slice(self.status.as_u16().to_string().as_bytes());
        buf.push(b' ');
        buf.extend_from_slice(self.status.reason().as_bytes());
        buf.extend_from_slice(b"\r\n");

        for (name, value) in self.headers.iter() {
            buf.extend_from_slice(name.as_str().as_bytes());
            buf.extend_from_slice(b": ");
            buf.extend_from_slice(value.as_str().as_bytes());
            buf.extend_from_slice(b"\r\n");
        }

        if !self.headers.contains_key("Content-Length") {
            let cl = self.body.len().to_string();
            buf.extend_from_slice(b"Content-Length: ");
            buf.extend_from_slice(cl.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }

        buf.extend_from_slice(b"\r\n");

        let skip_body = self.status.as_u16() == 204
            || self.status.as_u16() == 304
            || self.status.is_informational();
        if !skip_body {
            buf.extend_from_slice(&self.body);
        }
        buf
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.status.as_u16(), self.status.reason())
    }
}
