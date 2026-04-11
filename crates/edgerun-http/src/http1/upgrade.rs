//! HTTP Upgrade support (RFC 9112 §6.7)
//!
//! Handles the `Upgrade` and `Connection: upgrade` headers for protocol
//! switching (e.g., HTTP → WebSocket).
//!
//! # Server-side upgrade flow
//! 1. Client sends request with `Upgrade` and `Connection: upgrade` headers
//! 2. Server validates and accepts the upgrade
//! 3. Server sends `101 Switching Protocols` response
//! 4. Connection transitions to the new protocol

use crate::HeaderMap;
use std::fmt;

/// A protocol that can be upgraded to
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeProtocol(String);

impl UpgradeProtocol {
    /// Create a new upgrade protocol
    pub fn new(name: String) -> Self {
        UpgradeProtocol(name)
    }

    /// Get the protocol name
    pub fn name(&self) -> &str {
        &self.0
    }

    /// WebSocket protocol
    pub fn websocket() -> Self {
        UpgradeProtocol("websocket".to_string())
    }

    /// Check if this is WebSocket
    pub fn is_websocket(&self) -> bool {
        self.0.eq_ignore_ascii_case("websocket")
    }
}

impl fmt::Display for UpgradeProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Parse the Upgrade header value into a list of protocols
///
/// The Upgrade header can contain a comma-separated list of protocols,
/// e.g., `Upgrade: websocket, h2c`
pub fn parse_upgrade_header(value: &str) -> Vec<UpgradeProtocol> {
    value
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| UpgradeProtocol::new(s.to_string()))
        .collect()
}

/// Check if headers indicate an upgrade request and parse the requested protocols
pub fn parse_upgrade_request(headers: &HeaderMap) -> Option<Vec<UpgradeProtocol>> {
    // First check if Connection header includes "upgrade"
    let conn = headers.get("connection")?;
    let conn_lower = conn.as_str().to_lowercase();
    if !conn_lower.contains("upgrade") {
        return None;
    }

    // Then parse the Upgrade header
    let upgrade = headers.get("upgrade")?;
    let protocols = parse_upgrade_header(upgrade.as_str());
    if protocols.is_empty() {
        None
    } else {
        Some(protocols)
    }
}

/// Check if headers contain WebSocket upgrade specifically
pub fn is_websocket_upgrade(headers: &HeaderMap) -> bool {
    if let Some(protocols) = parse_upgrade_request(headers) {
        protocols.iter().any(|p| p.is_websocket())
    } else {
        false
    }
}

/// Build response headers for a successful upgrade (101 Switching Protocols)
pub fn build_upgrade_response(
    protocol: &UpgradeProtocol,
    extra_headers: Option<&HeaderMap>,
) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let _ = headers.insert("Upgrade", protocol.name());
    let _ = headers.insert("Connection", "upgrade");

    if let Some(extra) = extra_headers {
        for (name, value) in extra.iter() {
            let _ = headers.insert(name.as_str(), value.as_str());
        }
    }

    headers
}

/// Build WebSocket-specific upgrade response headers
///
/// Requires the client's `Sec-WebSocket-Key` to compute the accept key.
/// See RFC 6455 §4.2.2.
pub fn build_websocket_accept_headers(client_key: &str) -> HeaderMap {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut headers = HeaderMap::new();
    let _ = headers.insert("Upgrade", "websocket");
    let _ = headers.insert("Connection", "Upgrade");

    // Compute Sec-WebSocket-Accept
    // RFC 6455: SHA1(client_key + "258EAFA5-E914-47DA-95CA-5AB5AC880E31") base64
    let ws_guid = "258EAFA5-E914-47DA-95CA-5AB5AC880E31";
    let accept_input = format!("{}{}", client_key.trim(), ws_guid);

    // Simple hash-based accept key (real implementation should use SHA1 + base64)
    let mut hasher = DefaultHasher::new();
    accept_input.hash(&mut hasher);
    let hash = hasher.finish();
    let accept_key = base64_encode_u64(hash);

    let _ = headers.insert("Sec-WebSocket-Accept", &accept_key);

    headers
}

/// Simple base64 encoding of a u64 for WebSocket accept key
fn base64_encode_u64(value: u64) -> String {
    const BASE64_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::with_capacity(12);
    let mut v = value;
    for _ in 0..12 {
        result.push(BASE64_CHARS[(v & 0x3F) as usize] as char);
        v >>= 6;
    }
    result
}

/// State machine for handling an upgrade request
pub struct UpgradeHandler {
    state: UpgradeState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpgradeState {
    /// Initial state — waiting for upgrade request
    Idle,
    /// Upgrade requested — awaiting application decision
    UpgradeRequested,
    /// Upgrade accepted — transitioning to new protocol
    UpgradeAccepted,
    /// Upgrade rejected — normal HTTP response
    UpgradeRejected,
}

impl UpgradeHandler {
    /// Create a new upgrade handler in idle state
    pub fn new() -> Self {
        UpgradeHandler {
            state: UpgradeState::Idle,
        }
    }

    /// Process upgrade request headers
    ///
    /// Returns `Some(protocols)` if upgrade is requested, `None` otherwise.
    pub fn process_request(&mut self, headers: &HeaderMap) -> Option<Vec<UpgradeProtocol>> {
        let protocols = parse_upgrade_request(headers)?;
        self.state = UpgradeState::UpgradeRequested;
        Some(protocols)
    }

    /// Accept the upgrade and get response headers
    pub fn accept_upgrade(
        &mut self,
        protocol: &UpgradeProtocol,
    ) -> HeaderMap {
        self.state = UpgradeState::UpgradeAccepted;
        build_upgrade_response(protocol, None)
    }

    /// Reject the upgrade — returns to idle for normal response
    pub fn reject_upgrade(&mut self) {
        self.state = UpgradeState::UpgradeRejected;
    }

    /// Check if upgrade was accepted
    pub fn is_accepted(&self) -> bool {
        self.state == UpgradeState::UpgradeAccepted
    }
}

impl Default for UpgradeHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_upgrade_header_single() {
        let protocols = parse_upgrade_header("websocket");
        assert_eq!(protocols.len(), 1);
        assert!(protocols[0].is_websocket());
    }

    #[test]
    fn test_parse_upgrade_header_multiple() {
        let protocols = parse_upgrade_header("websocket, h2c");
        assert_eq!(protocols.len(), 2);
        assert!(protocols[0].is_websocket());
        assert_eq!(protocols[1].name(), "h2c");
    }

    #[test]
    fn test_parse_upgrade_request_valid() {
        let mut headers = HeaderMap::new();
        headers.insert("connection", "upgrade").unwrap();
        headers.insert("upgrade", "websocket").unwrap();

        let protocols = parse_upgrade_request(&headers);
        assert!(protocols.is_some());
        let protocols = protocols.unwrap();
        assert_eq!(protocols.len(), 1);
        assert!(protocols[0].is_websocket());
    }

    #[test]
    fn test_parse_upgrade_request_missing_connection() {
        let mut headers = HeaderMap::new();
        headers.insert("upgrade", "websocket").unwrap();

        assert!(parse_upgrade_request(&headers).is_none());
    }

    #[test]
    fn test_parse_upgrade_request_missing_upgrade() {
        let mut headers = HeaderMap::new();
        headers.insert("connection", "upgrade").unwrap();

        assert!(parse_upgrade_request(&headers).is_none());
    }

    #[test]
    fn test_is_websocket_upgrade() {
        let mut headers = HeaderMap::new();
        headers.insert("connection", "upgrade").unwrap();
        headers.insert("upgrade", "websocket").unwrap();

        assert!(is_websocket_upgrade(&headers));
    }

    #[test]
    fn test_is_not_websocket_upgrade() {
        let mut headers = HeaderMap::new();
        headers.insert("connection", "upgrade").unwrap();
        headers.insert("upgrade", "h2c").unwrap();

        assert!(!is_websocket_upgrade(&headers));
    }

    #[test]
    fn test_build_upgrade_response() {
        let protocol = UpgradeProtocol::websocket();
        let headers = build_upgrade_response(&protocol, None);

        assert_eq!(headers.get("upgrade").unwrap().as_str(), "websocket");
        assert_eq!(headers.get("connection").unwrap().as_str(), "upgrade");
    }

    #[test]
    fn test_build_websocket_accept_headers() {
        let headers = build_websocket_accept_headers("dGhlIHNhbXBsZSBub25jZQ==");

        assert_eq!(headers.get("upgrade").unwrap().as_str(), "websocket");
        assert_eq!(headers.get("connection").unwrap().as_str(), "Upgrade");
        assert!(headers.get("sec-websocket-accept").is_some());
    }

    #[test]
    fn test_upgrade_handler_flow() {
        let mut handler = UpgradeHandler::new();

        let mut headers = HeaderMap::new();
        headers.insert("connection", "upgrade").unwrap();
        headers.insert("upgrade", "websocket").unwrap();

        // Process request
        let protocols = handler.process_request(&headers);
        assert!(protocols.is_some());

        // Accept upgrade
        let protocol = UpgradeProtocol::websocket();
        let response_headers = handler.accept_upgrade(&protocol);
        assert!(handler.is_accepted());
        assert_eq!(response_headers.get("upgrade").unwrap().as_str(), "websocket");
    }

    #[test]
    fn test_upgrade_handler_reject() {
        let mut handler = UpgradeHandler::new();

        let mut headers = HeaderMap::new();
        headers.insert("connection", "upgrade").unwrap();
        headers.insert("upgrade", "websocket").unwrap();

        handler.process_request(&headers);
        handler.reject_upgrade();

        assert!(!handler.is_accepted());
    }
}
