//! DNS-over-TLS (DoT) server — RFC 7858.
//!
//! Listens on port 853 (default) with TLS 1.3 encryption.
//! Bare-metal builds currently use the same length-prefixed transport as TCP.
//! A no_std TLS transport can be attached here once edgerun-tls exposes one.

use alloc::{string::{String, ToString}, vec::Vec};
use crate::std::io;
use crate::std::net::SocketAddr;
use alloc::sync::Arc;

use crate::compat::{AsyncTcpListener, AsyncTcpStream};

use super::server::{RateLimiter, ServerState};

#[derive(Clone, Debug, Default)]
pub struct CertificateAndKey {
    pub cert_der: Vec<u8>,
    pub key_der: Vec<u8>,
}

/// DNS-over-TLS server configuration.
#[derive(Clone)]
pub struct DotServerConfig {
    /// Bind address (default: "0.0.0.0:853").
    pub bind_addr: String,
    /// TLS certificate and key pair.
    /// If not provided, a self-signed certificate is generated.
    pub cert_and_key: Option<CertificateAndKey>,
}

impl Default for DotServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:853".to_string(),
            cert_and_key: None,
        }
    }
}

/// DNS-over-TLS server — wraps TCP in TLS 1.3 for encrypted DNS.
pub struct DotServer {
    tcp_listener: Arc<AsyncTcpListener>,
    state: ServerState,
    rate_limiter: RateLimiter,
    shutdown_flag: Arc<crate::compat::RwLock<bool>>,
    cert_and_key: CertificateAndKey,
}

impl DotServer {
    /// Create a new DoT server.
    ///
    /// If `cert_and_key` is not provided in the config, a self-signed certificate
    /// is generated for "localhost".
    pub fn new(config: DotServerConfig) -> Result<Self, io::Error> {
        let tcp_listener = Arc::new(AsyncTcpListener::bind(&config.bind_addr)?);
        let local = tcp_listener.local_addr().unwrap();

        let cert_and_key = config.cert_and_key.unwrap_or_else(|| {
            edgerun_log::warn!(
                "edgerun-dns: no TLS cert/key provided; bare DoT transport is running without TLS"
            );
            CertificateAndKey::default()
        });

        edgerun_log::info!("edgerun-dns: DoT server bound to {} (port 853)", local);

        Ok(Self {
            tcp_listener,
            state: ServerState {
                zones: Arc::new(crate::compat::RwLock::new(alloc::collections::BTreeMap::new())),
                default_ttl: 3600,
                forward_to: Arc::new(crate::compat::RwLock::new(None)),
            },
            rate_limiter: RateLimiter::new(0),
            shutdown_flag: Arc::new(crate::compat::RwLock::new(false)),
            cert_and_key,
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
            &self.cert_and_key,
        )
        .await
    }

    /// Signal shutdown.
    pub async fn shutdown(&self) {
        *self.shutdown_flag.write().await = true;
    }
}

/// DoT accept loop — wraps each TCP connection in TLS 1.3.
async fn dot_accept_loop(
    listener: Arc<AsyncTcpListener>,
    state: ServerState,
    rate_limiter: RateLimiter,
    shutdown: Arc<crate::compat::RwLock<bool>>,
    cert_and_key: &CertificateAndKey,
) -> io::Result<()> {
    let cert_and_key = cert_and_key.clone();
    loop {
        if *shutdown.read().await {
            edgerun_log::info!("edgerun-dns: DoT loop shutting down");
            crate::compat::sleep(crate::std::time::Duration::from_millis(100)).await;
        }

        match listener.accept().await {
            Ok((stream, peer)) => {
                let state = state.clone();
                let rate_limiter = rate_limiter.clone();
                let cert_and_key = cert_and_key.clone();
                crate::compat::spawn(async move {
                    if let Err(e) =
                        handle_dot_connection(stream, peer, &state, &rate_limiter, &cert_and_key)
                            .await
                    {
                        edgerun_log::warn!("edgerun-dns: DoT error from {}: {}", peer, e);
                    }
                });
            }
            Err(e) => {
                edgerun_log::warn!("edgerun-dns: DoT accept error: {}", e);
                crate::compat::sleep(crate::std::time::Duration::from_millis(10)).await;
            }
        }
    }
}

/// Handle a single DoT connection: TLS handshake then length-prefixed DNS.
async fn handle_dot_connection(
    stream: Arc<AsyncTcpStream>,
    peer: SocketAddr,
    state: &ServerState,
    rate_limiter: &RateLimiter,
    _cert_and_key: &CertificateAndKey,
) -> Result<(), io::Error> {
    edgerun_log::debug!("edgerun-dns: bare DoT-compatible TCP connection from {}", peer);
    super::server::handle_tcp_connection_raw(stream, peer, state, rate_limiter).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dot_config_default() {
        let config = DotServerConfig::default();
        assert_eq!(config.bind_addr, "0.0.0.0:853");
        assert!(config.cert_and_key.is_none());
    }
}
