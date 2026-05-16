//! Shared HTTP error types

use alloc::format;
use alloc::string::{String, ToString};
use core::fmt;

/// HTTP error type — covers HTTP/1.1, HTTP/2, and HTTP/3 errors
#[derive(Debug)]
pub enum Error {
    /// Invalid URI
    InvalidUri(String),
    /// Invalid header
    InvalidHeader(String),
    /// Network error
    Network(crate::http::io::Error),
    /// Invalid HTTP method
    InvalidMethod(String),
    /// Invalid HTTP request
    InvalidRequest(String),
    /// Invalid status code
    InvalidStatusCode(u16),
    /// Request timeout
    Timeout,
    /// HTTP protocol error
    ProtocolError(String),
    /// Invalid response
    InvalidResponse(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidUri(value) => write!(f, "Invalid URI: {value}"),
            Error::InvalidHeader(value) => write!(f, "Invalid header: {value}"),
            Error::Network(value) => write!(f, "Network error: {value}"),
            Error::InvalidMethod(value) => write!(f, "Invalid method: {value}"),
            Error::InvalidRequest(value) => write!(f, "Invalid request: {value}"),
            Error::InvalidStatusCode(value) => write!(f, "Invalid status code: {value}"),
            Error::Timeout => f.write_str("Request timeout"),
            Error::ProtocolError(value) => write!(f, "Protocol error: {value}"),
            Error::InvalidResponse(value) => write!(f, "Invalid response: {value}"),
        }
    }
}

impl core::error::Error for Error {}

impl From<crate::http::io::Error> for Error {
    fn from(err: crate::http::io::Error) -> Self {
        Error::Network(err)
    }
}

#[cfg(feature = "http2")]
impl From<crate::http::http2::Http2Error> for Error {
    fn from(err: crate::http::http2::Http2Error) -> Self {
        Error::ProtocolError(format!("HTTP/2 error: {err}"))
    }
}

/// Alias for `Result<T, Error>`
pub type Result<T> = core::result::Result<T, Error>;

/// Unified HTTP error type that wraps HTTP/1.1, HTTP/2, and HTTP/3 errors
#[derive(Debug)]
pub enum HttpError {
    /// HTTP/1.1 error
    Http1(Error),
    /// HTTP/2 error
    Http2(Http2ErrorInner),
    /// HTTP/3 error
    Http3(Http3ErrorInner),
    /// I/O error
    Io(crate::http::io::Error),
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpError::Http1(value) => write!(f, "HTTP/1.1 error: {value}"),
            HttpError::Http2(value) => write!(f, "HTTP/2 error: {value}"),
            HttpError::Http3(value) => write!(f, "HTTP/3 error: {value}"),
            HttpError::Io(value) => write!(f, "I/O error: {value}"),
        }
    }
}

impl core::error::Error for HttpError {}

/// HTTP/2 error details
#[derive(Debug)]
pub struct Http2ErrorInner {
    /// Error code
    pub error_code: u32,
    /// Reason
    pub reason: String,
}

/// HTTP/3 error details
#[derive(Debug)]
pub struct Http3ErrorInner {
    /// Error code
    pub error_code: u64,
    /// Reason
    pub reason: String,
}

impl fmt::Display for Http2ErrorInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02x}: {}", self.error_code, self.reason)
    }
}

impl fmt::Display for Http3ErrorInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02x}: {}", self.error_code, self.reason)
    }
}

impl From<crate::http::io::Error> for HttpError {
    fn from(err: crate::http::io::Error) -> Self {
        HttpError::Io(err)
    }
}

impl From<Error> for HttpError {
    fn from(err: Error) -> Self {
        HttpError::Http1(err)
    }
}
