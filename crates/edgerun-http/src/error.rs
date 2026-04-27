//! Shared HTTP error types

#[cfg(target_os = "none")]
use crate::prelude::v1::*;
use edgerun_error::Error;
use std::fmt;

/// HTTP error type — covers HTTP/1.1, HTTP/2, and HTTP/3 errors
#[derive(Debug, Error)]
pub enum Error {
    /// Invalid URI
    #[error("Invalid URI: {0}")]
    InvalidUri(String),
    /// Invalid header
    #[error("Invalid header: {0}")]
    InvalidHeader(String),
    /// Network error
    #[error("Network error: {0}")]
    Network(std::io::Error),
    /// Invalid HTTP method
    #[error("Invalid method: {0}")]
    InvalidMethod(String),
    /// Invalid HTTP request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    /// Invalid status code
    #[error("Invalid status code: {0}")]
    InvalidStatusCode(u16),
    /// Request timeout
    #[error("Request timeout")]
    Timeout,
    /// HTTP protocol error
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    /// Invalid response
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Network(err)
    }
}

impl From<crate::http2::Http2Error> for Error {
    fn from(err: crate::http2::Http2Error) -> Self {
        Error::ProtocolError(format!("HTTP/2 error: {err}"))
    }
}

/// Alias for `Result<T, Error>`
pub type Result<T> = std::result::Result<T, Error>;

/// Unified HTTP error type that wraps HTTP/1.1, HTTP/2, and HTTP/3 errors
#[derive(Debug, Error)]
pub enum HttpError {
    /// HTTP/1.1 error
    #[error("HTTP/1.1 error: {0}")]
    Http1(Error),
    /// HTTP/2 error
    #[error("HTTP/2 error: {0}")]
    Http2(Http2ErrorInner),
    /// HTTP/3 error
    #[error("HTTP/3 error: {0}")]
    Http3(Http3ErrorInner),
    /// I/O error
    #[error("I/O error: {0}")]
    Io(std::io::Error),
}

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

impl From<std::io::Error> for HttpError {
    fn from(err: std::io::Error) -> Self {
        HttpError::Io(err)
    }
}

impl From<Error> for HttpError {
    fn from(err: Error) -> Self {
        HttpError::Http1(err)
    }
}
