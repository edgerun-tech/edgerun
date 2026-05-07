//! Protocol-agnostic HTTP response.

use crate::http::header::HeaderMap;
use crate::http::status::StatusCode;
use alloc::string::String;
use alloc::vec::Vec;
use edgerun_protocols::http::{HttpMessageError, HttpResponse};

/// An HTTP response, protocol-agnostic.
///
/// Returned by [`crate::http::Handler`] implementations across HTTP/1.1, HTTP/2,
/// and HTTP/3 servers. Also used by all three client implementations.
#[derive(Debug, Clone)]
pub struct Response(HttpResponse);

impl Response {
    pub fn new(status: StatusCode) -> Self {
        Self(HttpResponse::new(status))
    }

    pub fn from_parts(status: StatusCode, headers: HeaderMap, body: Vec<u8>) -> Self {
        Self(HttpResponse::from_parts(status, headers, body))
    }

    pub fn with_body(mut self, body: impl Into<Vec<u8>>) -> Self {
        self.0 = self.0.with_body(body);
        self
    }

    pub fn status(&self) -> StatusCode {
        self.0.status()
    }

    pub fn headers(&self) -> &HeaderMap {
        self.0.headers()
    }

    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        self.0.headers_mut()
    }

    pub fn body(&self) -> &[u8] {
        self.0.body()
    }

    pub fn body_as_string(&self) -> Option<String> {
        self.0.body_as_string()
    }

    pub fn is_success(&self) -> bool {
        self.0.is_success()
    }

    pub fn trailers(&self) -> &HeaderMap {
        self.0.trailers()
    }

    pub fn is_streaming(&self) -> bool {
        self.0.is_streaming()
    }

    /// Create a streaming response with chunked transfer encoding.
    pub fn streaming(status: StatusCode) -> Self {
        Self(HttpResponse::streaming(status))
    }

    /// Add a chunk to a streaming response body.
    pub fn add_chunk(&mut self, chunk: impl Into<Vec<u8>>) {
        self.0.add_chunk(chunk);
    }

    pub fn text(status: StatusCode, body: &str) -> Self {
        Self(HttpResponse::text(status, body))
    }

    pub fn json(status: StatusCode, body: &str) -> Self {
        Self(HttpResponse::json(status, body))
    }

    pub fn html(status: StatusCode, body: &str) -> Self {
        Self(HttpResponse::html(status, body))
    }

    pub fn not_found() -> Self {
        Self(HttpResponse::not_found())
    }

    pub fn internal_error() -> Self {
        Self(HttpResponse::internal_error())
    }

    pub fn internal_error_msg(msg: &str) -> Self {
        Self(HttpResponse::internal_error_msg(msg))
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.0 = self.0.with_header(name, value);
        self
    }

    pub fn from_http(raw: &str) -> crate::http::Result<Self> {
        HttpResponse::from_http(raw)
            .map(Self)
            .map_err(map_http_message_error)
    }
}

fn map_http_message_error(err: HttpMessageError) -> crate::http::Error {
    match err {
        HttpMessageError::InvalidRequest(value) => crate::http::Error::InvalidRequest(value),
        HttpMessageError::InvalidResponse(value) => crate::http::Error::InvalidResponse(value),
        HttpMessageError::InvalidUri(value) => crate::http::Error::InvalidUri(value),
        HttpMessageError::InvalidStatusCode(value) => crate::http::Error::InvalidStatusCode(value),
    }
}
