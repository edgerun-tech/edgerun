//! DNS-over-HTTPS (DoH) server — RFC 8484.
//!
//! Serves DNS queries over HTTPS at `/dns-query`.
//! Supports both GET (base64url-encoded `dns` parameter) and
//! POST (wire-format DNS message in body).

use alloc::{boxed::Box, format, string::{String, ToString}, vec, vec::Vec};
use alloc::collections::BTreeMap as HashMap;
use alloc::sync::Arc;

use super::cache::DnsCache;
use super::server::{handle_query, ServerState};
use crate::message::{DnsMessage, DnsResponseCode};

/// DNS-over-HTTPS server configuration.
#[derive(Debug, Clone)]
pub struct DohServerConfig {
    /// Bind address (default: "127.0.0.1:443").
    pub bind_addr: String,
    /// Enable GET method (base64url `dns` param). Default: true.
    pub enable_get: bool,
    /// Enable POST method (wire-format body). Default: true.
    pub enable_post: bool,
}

impl Default for DohServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "127.0.0.1:443".to_string(),
            enable_get: true,
            enable_post: true,
        }
    }
}

/// DNS-over-HTTPS server state.
pub struct DohServer {
    state: ServerState,
    cache: Option<DnsCache>,
    config: DohServerConfig,
}

impl DohServer {
    /// Create a new DoH server.
    pub fn new(config: DohServerConfig) -> Self {
        Self {
            state: ServerState {
                zones: Arc::new(crate::compat::RwLock::new(HashMap::new())),
                default_ttl: 3600,
                forward_to: Arc::new(crate::compat::RwLock::new(None)),
            },
            cache: None,
            config,
        }
    }

    /// Add a zone.
    pub async fn add_zone(&self, zone: crate::zone::DnsZone) {
        let origin = zone.origin.clone();
        self.state.zones.write().await.insert(origin, zone);
    }

    /// Get server state for external HTTP server integration.
    pub fn state(&self) -> &ServerState {
        &self.state
    }

    /// Handle a DoH GET request (`/dns-query?dns=<base64url>`).
    ///
    /// Returns `(status_code, content_type, body)`.
    pub async fn handle_get(&self, dns_param: &str) -> (u16, &'static str, Vec<u8>) {
        if !self.config.enable_get {
            return (405, "text/plain", b"GET method not enabled".to_vec());
        }

        // Decode base64url DNS query
        let wire = match decode_base64url(dns_param) {
            Some(w) => w,
            None => return (400, "text/plain", b"Invalid dns parameter".to_vec()),
        };

        self.process_query(&wire).await
    }

    /// Handle a DoH POST request (wire-format DNS message in body).
    ///
    /// Returns `(status_code, content_type, body)`.
    pub async fn handle_post(&self, body: &[u8]) -> (u16, &'static str, Vec<u8>) {
        if !self.config.enable_post {
            return (405, "text/plain", b"POST method not enabled".to_vec());
        }
        self.process_query(body).await
    }

    /// Process a raw DNS wire-format query.
    async fn process_query(&self, wire: &[u8]) -> (u16, &'static str, Vec<u8>) {
        match handle_query(wire, &self.state).await {
            Ok((response_wire, _needs_tcp)) => (200, "application/dns-message", response_wire),
            Err(_) => {
                let resp = DnsMessage::response(0, DnsResponseCode::FormErr, Vec::new());
                (400, "application/dns-message", resp.to_wire())
            }
        }
    }
}

/// Decode base64url (RFC 4648 §5, no padding).
fn decode_base64url(input: &str) -> Option<Vec<u8>> {
    edgerun_encoding::base64::base64url_decode(input).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doh_config_default() {
        let config = DohServerConfig::default();
        assert!(config.enable_get);
        assert!(config.enable_post);
    }

    #[test]
    fn test_decode_base64url() {
        // "test" → dGVzdA==
        let result = decode_base64url("dGVzdA").unwrap();
        assert_eq!(result, b"test");
    }

    #[test]
    fn test_doh_invalid_base64() {
        // Not implemented as async, but test the sync part
        let result = decode_base64url("!!!invalid!!!");
        assert!(result.is_none());
    }
}
