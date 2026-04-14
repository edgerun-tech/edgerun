//! Shared HTTP error types

use std::fmt;

/// HTTP error type — covers HTTP/1.1, HTTP/2, and HTTP/3 errors
#[derive(Debug)]
pub enum Error {
    /// Invalid URI
    InvalidUri(String),
    /// Invalid header
    InvalidHeader(String),
    /// Network error
    Network(std::io::Error),
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
            Error::InvalidUri(msg) => write!(f, "Invalid URI: {msg}"),
            Error::InvalidHeader(msg) => write!(f, "Invalid header: {msg}"),
            Error::Network(err) => write!(f, "Network error: {err}"),
            Error::InvalidMethod(msg) => write!(f, "Invalid method: {msg}"),
            Error::InvalidRequest(msg) => write!(f, "Invalid request: {msg}"),
            Error::InvalidStatusCode(code) => {
                write!(f, "Invalid status code: {code}")
            }
            Error::Timeout => write!(f, "Request timeout"),
            Error::ProtocolError(msg) => write!(f, "Protocol error: {msg}"),
            Error::InvalidResponse(msg) => write!(f, "Invalid response: {msg}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Network(err) => Some(err),
            _ => None,
        }
    }
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
#[derive(Debug)]
pub enum HttpError {
    /// HTTP/1.1 error
    Http1(Error),
    /// HTTP/2 error
    Http2(Http2ErrorInner),
    /// HTTP/3 error
    Http3(Http3ErrorInner),
    /// I/O error
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

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpError::Http1(err) => write!(f, "HTTP/1.1 error: {}", err),
            HttpError::Http2(inner) => {
                write!(f, "HTTP/2 error (0x{:02x}): {}", inner.error_code, inner.reason)
            }
            HttpError::Http3(inner) => {
                write!(f, "HTTP/3 error (0x{:02x}): {}", inner.error_code, inner.reason)
            }
            HttpError::Io(err) => write!(f, "I/O error: {}", err),
        }
    }
}

impl std::error::Error for HttpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            HttpError::Http1(err) => Some(err),
            HttpError::Io(err) => Some(err),
            _ => None,
        }
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
