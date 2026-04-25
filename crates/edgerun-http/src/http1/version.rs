//! HTTP version handling (RFC 9112 §2.6)
//!
//! Supports both HTTP/1.0 and HTTP/1.1 with proper default behaviors:
//! - HTTP/1.0: defaults to `Connection: close`
//! - HTTP/1.1: defaults to `Connection: keep-alive`

use crate::Result;
use std::fmt;

/// HTTP protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HttpVersion {
    /// HTTP/1.0 (RFC 1945)
    Http10,
    /// HTTP/1.1 (RFC 9112)
    #[default]
    Http11,
}

impl HttpVersion {
    /// Parse from version string (e.g., "HTTP/1.0", "HTTP/1.1")
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "HTTP/1.0" => Ok(HttpVersion::Http10),
            "HTTP/1.1" => Ok(HttpVersion::Http11),
            other => Err(crate::Error::InvalidRequest(format!(
                "unsupported HTTP version: {}",
                other
            ))),
        }
    }

    /// Format as HTTP version string
    pub fn as_str(&self) -> &'static str {
        match self {
            HttpVersion::Http10 => "HTTP/1.0",
            HttpVersion::Http11 => "HTTP/1.1",
        }
    }

    /// Returns true if this is HTTP/1.0
    pub fn is_http10(&self) -> bool {
        *self == HttpVersion::Http10
    }

    /// Returns true if this is HTTP/1.1
    pub fn is_http11(&self) -> bool {
        *self == HttpVersion::Http11
    }

    /// HTTP/1.0 defaults to `Connection: close` unless explicitly overridden.
    /// HTTP/1.1 defaults to `Connection: keep-alive` unless explicitly overridden.
    pub fn default_connection_behavior(&self) -> ConnectionDefault {
        match self {
            HttpVersion::Http10 => ConnectionDefault::Close,
            HttpVersion::Http11 => ConnectionDefault::KeepAlive,
        }
    }
}

/// Default connection behavior for a given HTTP version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionDefault {
    /// Connection should close after this response
    Close,
    /// Connection should stay open for more requests
    KeepAlive,
}

impl fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_from_str() {
        assert_eq!(
            HttpVersion::from_str("HTTP/1.0").unwrap(),
            HttpVersion::Http10
        );
        assert_eq!(
            HttpVersion::from_str("HTTP/1.1").unwrap(),
            HttpVersion::Http11
        );
        assert!(HttpVersion::from_str("HTTP/2.0").is_err());
        assert!(HttpVersion::from_str("HTTP/3.0").is_err());
    }

    #[test]
    fn test_version_predicates() {
        assert!(HttpVersion::Http10.is_http10());
        assert!(!HttpVersion::Http10.is_http11());
        assert!(HttpVersion::Http11.is_http11());
        assert!(!HttpVersion::Http11.is_http10());
    }

    #[test]
    fn test_default_connection_behavior() {
        assert_eq!(
            HttpVersion::Http10.default_connection_behavior(),
            ConnectionDefault::Close
        );
        assert_eq!(
            HttpVersion::Http11.default_connection_behavior(),
            ConnectionDefault::KeepAlive
        );
    }
}
