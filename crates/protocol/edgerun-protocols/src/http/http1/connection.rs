//! HTTP Connection header management (RFC 9112 §9.6)
//!
//! Handles `Connection` header parsing and default behavior:
//! - HTTP/1.0: defaults to close, `Connection: keep-alive` enables persistence
//! - HTTP/1.1: defaults to keep-alive, `Connection: close` disables persistence
//! - `Connection: upgrade` signals protocol upgrade (e.g., WebSocket)

use crate::http::HeaderMap;
use crate::http::header::header_value_has_token;
use crate::http::http1::version::{ConnectionDefault, Http1Version};

/// Connection header state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection will close after current response
    Close,
    /// Connection stays open for more requests
    KeepAlive,
    /// Client requested protocol upgrade (e.g., WebSocket)
    Upgrade,
}

impl ConnectionState {
    /// Returns true if connection should stay open
    pub fn is_persistent(&self) -> bool {
        matches!(self, ConnectionState::KeepAlive | ConnectionState::Upgrade)
    }

    /// Returns true if this is an upgrade request
    pub fn is_upgrade(&self) -> bool {
        *self == ConnectionState::Upgrade
    }
}

/// Determine the connection state from headers and HTTP version.
///
/// This checks the `Connection` header and falls back to the version's
/// default behavior if not present.
pub fn determine_connection(headers: &HeaderMap, version: Http1Version) -> ConnectionState {
    if let Some(conn_header) = headers.get("connection") {
        parse_connection_value(conn_header.as_str())
    } else {
        // Fall back to version default
        match version.default_connection_behavior() {
            ConnectionDefault::Close => ConnectionState::Close,
            ConnectionDefault::KeepAlive => ConnectionState::KeepAlive,
        }
    }
}

/// Parse a Connection header value into a ConnectionState
fn parse_connection_value(value: &str) -> ConnectionState {
    if header_value_has_token(value, "upgrade") {
        ConnectionState::Upgrade
    } else if header_value_has_token(value, "close") {
        ConnectionState::Close
    } else {
        // "keep-alive" or any other value → treat as keep-alive
        ConnectionState::KeepAlive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_connection_explicit_close() {
        let mut headers = HeaderMap::new();
        headers.insert("connection", "close").unwrap();
        assert_eq!(
            determine_connection(&headers, Http1Version::Http11),
            ConnectionState::Close
        );
    }

    #[test]
    fn test_determine_connection_explicit_keep_alive() {
        let mut headers = HeaderMap::new();
        headers.insert("connection", "keep-alive").unwrap();
        assert_eq!(
            determine_connection(&headers, Http1Version::Http10),
            ConnectionState::KeepAlive
        );
    }

    #[test]
    fn test_determine_connection_upgrade() {
        let mut headers = HeaderMap::new();
        headers.insert("connection", "upgrade").unwrap();
        headers.insert("upgrade", "websocket").unwrap();
        assert_eq!(
            determine_connection(&headers, Http1Version::Http11),
            ConnectionState::Upgrade
        );
    }

    #[test]
    fn test_determine_connection_defaults() {
        let headers = HeaderMap::new();
        assert_eq!(
            determine_connection(&headers, Http1Version::Http10),
            ConnectionState::Close
        );
        assert_eq!(
            determine_connection(&headers, Http1Version::Http11),
            ConnectionState::KeepAlive
        );
    }

    #[test]
    fn test_connection_state_predicates() {
        assert!(ConnectionState::KeepAlive.is_persistent());
        assert!(ConnectionState::Upgrade.is_persistent());
        assert!(!ConnectionState::Close.is_persistent());

        assert!(ConnectionState::Upgrade.is_upgrade());
        assert!(!ConnectionState::KeepAlive.is_upgrade());
        assert!(!ConnectionState::Close.is_upgrade());
    }

    #[test]
    fn test_connection_tokens_are_exact() {
        let mut headers = HeaderMap::new();
        headers.insert("connection", "xclose").unwrap();

        assert_eq!(
            determine_connection(&headers, Http1Version::Http11),
            ConnectionState::KeepAlive
        );
    }
}
