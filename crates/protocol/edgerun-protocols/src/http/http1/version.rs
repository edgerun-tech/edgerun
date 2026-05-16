//! HTTP version handling (RFC 9112 §2.6)
//!
//! Supports both HTTP/1.0 and HTTP/1.1 with proper default behaviors:
//! - HTTP/1.0: defaults to `Connection: close`
//! - HTTP/1.1: defaults to `Connection: keep-alive`

use alloc::format;
use core::fmt;

pub type Result<T> = core::result::Result<T, HttpVersionError>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpVersionError(pub alloc::string::String);

impl fmt::Display for HttpVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// HTTP protocol version
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Http1Version {
    /// HTTP/1.0 (RFC 1945)
    Http10,
    /// HTTP/1.1 (RFC 9112)
    #[default]
    Http11,
}

impl Http1Version {
    /// Parse from version string (e.g., "HTTP/1.0", "HTTP/1.1")
    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "HTTP/1.0" => Ok(Http1Version::Http10),
            "HTTP/1.1" => Ok(Http1Version::Http11),
            other => Err(HttpVersionError(format!(
                "unsupported HTTP version: {}",
                other
            ))),
        }
    }

    /// Format as HTTP version string
    pub fn as_str(&self) -> &'static str {
        match self {
            Http1Version::Http10 => "HTTP/1.0",
            Http1Version::Http11 => "HTTP/1.1",
        }
    }

    /// Returns true if this is HTTP/1.0
    pub fn is_http10(&self) -> bool {
        *self == Http1Version::Http10
    }

    /// Returns true if this is HTTP/1.1
    pub fn is_http11(&self) -> bool {
        *self == Http1Version::Http11
    }

    /// HTTP/1.0 defaults to `Connection: close` unless explicitly overridden.
    /// HTTP/1.1 defaults to `Connection: keep-alive` unless explicitly overridden.
    pub fn default_connection_behavior(&self) -> ConnectionDefault {
        match self {
            Http1Version::Http10 => ConnectionDefault::Close,
            Http1Version::Http11 => ConnectionDefault::KeepAlive,
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

impl fmt::Display for Http1Version {
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
            Http1Version::from_str("HTTP/1.0").unwrap(),
            Http1Version::Http10
        );
        assert_eq!(
            Http1Version::from_str("HTTP/1.1").unwrap(),
            Http1Version::Http11
        );
        assert!(Http1Version::from_str("HTTP/2.0").is_err());
        assert!(Http1Version::from_str("HTTP/3.0").is_err());
    }

    #[test]
    fn test_version_predicates() {
        assert!(Http1Version::Http10.is_http10());
        assert!(!Http1Version::Http10.is_http11());
        assert!(Http1Version::Http11.is_http11());
        assert!(!Http1Version::Http11.is_http10());
    }

    #[test]
    fn test_default_connection_behavior() {
        assert_eq!(
            Http1Version::Http10.default_connection_behavior(),
            ConnectionDefault::Close
        );
        assert_eq!(
            Http1Version::Http11.default_connection_behavior(),
            ConnectionDefault::KeepAlive
        );
    }
}
