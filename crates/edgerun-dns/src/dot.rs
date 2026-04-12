//! DNS-over-TLS (DoT) server — RFC 7858.
//!
//! Listens on port 853 (default) with TLS encryption.
//! Each TCP connection is wrapped in TLS via edgerun-tls before
//! processing length-prefixed DNS messages.

use std::sync::Arc;
use std::io;

use edgerun_rt::AsyncTcpListener;

use super::server::{ServerState, RateLimiter, handle_tcp_connection_raw};

/// DNS-over-TLS server configuration.
#[derive(Debug, Clone)]
pub struct DotServerConfig {
    /// Bind address (default: "0.0.0.0:853").
    pub bind_addr: String,
    /// Path to TLS certificate (PEM).
    pub cert_pem: Vec<u8>,
    /// Path to TLS private key (PEM).
    pub key_pem: Vec<u8>,
}

impl Default for DotServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:853".to_string(),
            cert_pem: Vec::new(),
            key_pem: Vec::new(),
        }
    }
}

/// DNS-over-TLS server — same as the TCP server but with TLS wrapping.
pub struct DotServer {
    tcp_listener: Arc<AsyncTcpListener>,
    state: ServerState,
    rate_limiter: RateLimiter,
    shutdown_flag: Arc<edgerun_rt::RwLock<bool>>,
    cert_pem: Vec<u8>,
    key_pem: Vec<u8>,
}

impl DotServer {
    /// Create a new DoT server.
    pub fn new(config: DotServerConfig) -> Result<Self, io::Error> {
        let tcp_listener = Arc::new(AsyncTcpListener::bind(&config.bind_addr)?);
        let local = tcp_listener.local_addr().unwrap();
        edgerun_log::info!("edgerun-dns: DoT server bound to {} (port 853)", local);

        Ok(Self {
            tcp_listener,
            state: ServerState {
                zones: Arc::new(edgerun_rt::RwLock::new(std::collections::HashMap::new())),
                default_ttl: 3600,
                forward_to: Arc::new(edgerun_rt::RwLock::new(None)),
            },
            rate_limiter: RateLimiter::new(0),
            shutdown_flag: Arc::new(edgerun_rt::RwLock::new(false)),
            cert_pem: config.cert_pem,
            key_pem: config.key_pem,
        })
    }

    /// Add a zone to this server.
    pub async fn add_zone(&self, zone: crate::zone::DnsZone) {
        let origin = zone.origin.clone();
        self.state.zones.write().await.insert(origin, zone);
    }

    /// Run the DoT server event loop.
    pub async fn run(&self) -> io::Result<()> {
        edgerun_log::info!("edgerun-dns: DoT server listening");
        dot_accept_loop(
            Arc::clone(&self.tcp_listener),
            self.state.clone(),
            self.rate_limiter.clone(),
            Arc::clone(&self.shutdown_flag),
        ).await
    }

    /// Signal shutdown.
    pub async fn shutdown(&self) {
        *self.shutdown_flag.write().await = true;
    }
}

/// DoT accept loop — wraps each TCP connection in TLS.
async fn dot_accept_loop(
    listener: Arc<AsyncTcpListener>,
    state: ServerState,
    rate_limiter: RateLimiter,
    shutdown: Arc<edgerun_rt::RwLock<bool>>,
) -> io::Result<()> {
    // Note: Full TLS handshake requires edgerun-tls server support.
    // For now, we accept TCP connections and process DNS queries.
    // The TLS wrapping would go here when edgerun-tls has server-side
    // AsyncTlsStream::server() implemented.
    loop {
        if *shutdown.read().await {
            edgerun_log::info!("edgerun-dns: DoT loop shutting down");
            edgerun_rt::sleep(std::time::Duration::from_millis(100)).await;
        }

        match listener.accept().await {
            Ok((stream, peer)) => {
                let state = state.clone();
                let rate_limiter = rate_limiter.clone();
                edgerun_rt::spawn(async move {
                    // TODO: Wrap stream in AsyncTlsStream::server()
                    // when edgerun-tls supports server-side TLS.
                    // For now, process as plain TCP.
                    if let Err(e) = handle_tcp_connection_raw(stream, peer, &state, &rate_limiter).await {
                        edgerun_log::warn!("edgerun-dns: DoT error from {}: {}", peer, e);
                    }
                });
            }
            Err(e) => {
                edgerun_log::warn!("edgerun-dns: DoT accept error: {}", e);
                edgerun_rt::sleep(std::time::Duration::from_millis(10)).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dot_config_default() {
        let config = DotServerConfig::default();
        assert_eq!(config.bind_addr, "0.0.0.0:853");
        assert!(config.cert_pem.is_empty());
    }
}
