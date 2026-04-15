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
    streaming: bool,
}

impl Response {
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body: Vec::new(),
            trailers: HeaderMap::new(),
            streaming: false,
        }
    }

    pub fn from_parts(status: StatusCode, headers: HeaderMap, body: Vec<u8>) -> Self {
        Self {
            status,
            headers,
            body,
            trailers: HeaderMap::new(),
            streaming: false,
        }
    }

    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        let body = body.into();
        if !self.streaming && !self.headers.contains_key("Content-Length") {
            let _ = self
                .headers
                .insert("Content-Length", &body.len().to_string());
        }
        self.body = body;
        self
    }

    pub fn status(&self) -> StatusCode {
        self.status.clone()
    }
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }
    pub fn body(&self) -> &[u8] {
        &self.body
    }
    pub fn trailers(&self) -> &HeaderMap {
        &self.trailers
    }
    pub fn is_streaming(&self) -> bool {
        self.streaming
    }

    /// Create a streaming response with chunked transfer encoding.
    pub fn streaming(status: StatusCode) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            body: Vec::new(),
            trailers: HeaderMap::new(),
            streaming: true,
        }
    }

    /// Add a chunk to a streaming response body.
    pub fn add_chunk(&mut self, chunk: impl Into<Vec<u8>>) {
        let chunk = chunk.into();
        if self.streaming && !chunk.is_empty() {
            self.body.extend_from_slice(&chunk);
        }
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

    pub fn internal_error_msg(msg: &str) -> Self {
        Self::text(StatusCode::new(500).unwrap(), msg)
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        let _ = self.headers.insert(name, value);
        self
    }
}
