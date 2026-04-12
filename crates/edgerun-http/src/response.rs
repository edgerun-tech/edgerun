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
    /// Trailers (HTTP/2 and HTTP/3 only; ignored for HTTP/1.1).
    trailers: HeaderMap,
}

impl Response {
    /// Create a new response with the given status.
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body: Vec::new(),
            trailers: HeaderMap::new(),
        }
    }

    /// Create a response from parts.
    pub fn from_parts(status: StatusCode, headers: HeaderMap, body: Vec<u8>) -> Self {
        Self {
            status,
            headers,
            body,
            trailers: HeaderMap::new(),
        }
    }

    /// Set the response body.
    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        let body = body.into();
        // Auto-set Content-Length if not already set
        if !self.headers.contains_key("Content-Length") {
            self.headers.insert("Content-Length", &body.len().to_string());
        }
        self.body = body;
        self
    }

    /// Create a text/plain response.
    pub fn text(status: StatusCode, body: &str) -> Self {
        Self::new(status)
            .with_header("Content-Type", "text/plain; charset=utf-8")
            .with_body(body)
    }

    /// Create an application/json response.
    pub fn json(status: StatusCode, body: &str) -> Self {
        Self::new(status)
            .with_header("Content-Type", "application/json")
            .with_body(body)
    }

    /// Create an HTML response.
    pub fn html(status: StatusCode, body: &str) -> Self {
        Self::new(status)
            .with_header("Content-Type", "text/html; charset=utf-8")
            .with_body(body)
    }

    /// 404 Not Found convenience constructor.
    pub fn not_found() -> Self {
        Self::text(
            StatusCode::from_u16(404).unwrap(),
            "404 Not Found",
        )
    }

    /// 500 Internal Server Error convenience constructor.
    pub fn internal_error() -> Self {
        Self::text(
            StatusCode::from_u16(500).unwrap(),
            "500 Internal Server Error",
        )
    }

    /// Add a header.
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Mutable access to headers.
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    /// Response status code.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Response headers.
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Response body.
    pub fn body(&self) -> &[u8] {
        &self.body
    }

    /// Response body as string.
    pub fn body_as_string(&self) -> Result<&str, std::str::Utf8Error> {
        std::str::from_utf8(&self.body)
    }

    /// Response trailers (HTTP/2, HTTP/3 only).
    pub fn trailers(&self) -> &HeaderMap {
        &self.trailers
    }

    /// Set the response body.
    pub fn set_body(&mut self, body: Vec<u8>) {
        self.body = body;
    }

    /// Set headers.
    pub fn set_headers(&mut self, headers: HeaderMap) {
        self.headers = headers;
    }

    /// Check if the response is successful (2xx).
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Serialize the response as raw HTTP/1.1 bytes.
    ///
    /// Useful for debugging or proxy forwarding.
    pub fn to_http_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Status line
        buf.extend_from_slice(b"HTTP/1.1 ");
        buf.extend_from_slice(self.status.as_u16().to_string().as_bytes());
        buf.push(b' ');
        if let Some(reason) = self.status.reason() {
            buf.extend_from_slice(reason.as_bytes());
        }
        buf.extend_from_slice(b"\r\n");

        // Headers
        for (name, value) in self.headers.iter() {
            buf.extend_from_slice(name.as_bytes());
            buf.extend_from_slice(b": ");
            buf.extend_from_slice(value.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }

        // Content-Length if not set
        if !self.headers.contains_key("Content-Length") {
            let cl = self.body.len().to_string();
            buf.extend_from_slice(b"Content-Length: ");
            buf.extend_from_slice(cl.as_bytes());
            buf.extend_from_slice(b"\r\n");
        }

        buf.extend_from_slice(b"\r\n");

        // Body (skip for HEAD-equivalent status codes)
        if self.status.as_u16() != 204 && self.status.as_u16() != 304 && self.status.category() != 1 {
            buf.extend_from_slice(&self.body);
        }

        buf
    }
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.status.as_u16(), self.status.reason().unwrap_or(""))
    }
}
