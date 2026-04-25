//! DNS-over-TLS (DoT) server — RFC 7858.
//!
//! Listens on port 853 (default) with TLS 1.3 encryption.
//! Each TCP connection is wrapped in TLS via `edgerun-tls`'s
//! `AsyncTlsServerStream` before processing length-prefixed DNS messages.

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use edgerun_rt::{AsyncRead, AsyncTcpListener, AsyncTcpStream, AsyncWrite};
use edgerun_tls::record::TlsRecord;
use edgerun_tls::{AsyncTlsServerStream, CertificateAndKey};

use super::message::{DnsMessage, DnsResponseCode};
use super::server::{RateLimiter, ServerState};

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
    shutdown_flag: Arc<edgerun_rt::RwLock<bool>>,
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
                "edgerun-dns: no TLS cert/key provided, using self-signed for localhost"
            );
            edgerun_tls::generate_self_signed(&["localhost"])
                .expect("self-signed cert generation should not fail")
        });

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
    shutdown: Arc<edgerun_rt::RwLock<bool>>,
    cert_and_key: &CertificateAndKey,
) -> io::Result<()> {
    let cert_and_key = cert_and_key.clone();
    loop {
        if *shutdown.read().await {
            edgerun_log::info!("edgerun-dns: DoT loop shutting down");
            edgerun_rt::sleep(std::time::Duration::from_millis(100)).await;
        }

        match listener.accept().await {
            Ok((stream, peer)) => {
                let state = state.clone();
                let rate_limiter = rate_limiter.clone();
                let cert_and_key = cert_and_key.clone();
                edgerun_rt::spawn(async move {
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
                edgerun_rt::sleep(std::time::Duration::from_millis(10)).await;
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
    cert_and_key: &CertificateAndKey,
) -> Result<(), io::Error> {
    edgerun_log::debug!("edgerun-dns: DoT connection from {}", peer);

    // Perform TLS handshake
    let mut tls_stream = match AsyncTlsServerStream::accept(stream, cert_and_key).await {
        Ok(s) => s,
        Err(e) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("TLS handshake failed: {e}"),
            ));
        }
    };

    edgerun_log::debug!("edgerun-dns: DoT handshake complete from {}", peer);

    let stream_mutex = Arc::new(std::sync::Mutex::new(tls_stream));

    loop {
        // Read 2-byte length prefix (DNS-over-TLS uses length-prefixed messages).
        let mut len_buf = [0u8; 2];
        match tls_read_exact(&stream_mutex, &mut len_buf).await {
            Ok(0) => return Ok(()),
            Ok(2) => {}
            Ok(_) => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "incomplete TCP length",
                ))
            }
            Err(e) => return Err(e),
        }
        let msg_len = u16::from_be_bytes(len_buf) as usize;
        if msg_len == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "zero TCP message length",
            ));
        }

        let mut query_buf = vec![0u8; msg_len];
        tls_read_exact(&stream_mutex, &mut query_buf).await?;

        // Rate limit per source IP
        if !rate_limiter.allow(peer.ip()) {
            edgerun_log::debug!("edgerun-dns: rate limited DoT query from {}", peer.ip());
            let response = DnsMessage::response(0, DnsResponseCode::Refused, Vec::new());
            tls_write_length_prefixed(&stream_mutex, &response.to_wire()).await?;
            continue;
        }

        // Parse and handle the DNS query
        match DnsMessage::from_wire(&query_buf) {
            Ok(query) => {
                let (response_wire, _needs_tcp) =
                    super::server::query::handle_query(&query_buf, state)
                        .await
                        .unwrap_or_else(|_| {
                            let err_resp =
                                DnsMessage::response(0, DnsResponseCode::FormErr, Vec::new());
                            (err_resp.to_wire(), false)
                        });
                tls_write_length_prefixed(&stream_mutex, &response_wire).await?;
            }
            Err(e) => {
                edgerun_log::warn!("edgerun-dns: DoT parse error from {}: {}", peer, e);
                let response = DnsMessage::response(0, DnsResponseCode::FormErr, Vec::new());
                tls_write_length_prefixed(&stream_mutex, &response.to_wire()).await?;
            }
        }
    }
}

/// Read exactly `n` bytes from a TLS stream behind a Mutex.
async fn tls_read_exact<S: AsyncRead + Unpin>(
    stream_mutex: &Arc<std::sync::Mutex<S>>,
    buf: &mut [u8],
) -> io::Result<usize> {
    use std::future::poll_fn;
    use std::task::Poll;

    let mut total = 0;
    let n = buf.len();
    while total < n {
        let read = poll_fn(|cx| {
            let mut guard = stream_mutex.lock().unwrap();
            Pin::new(&mut *guard).poll_read(cx, &mut buf[total..n])
        })
        .await?;

        if read == 0 {
            return if total == 0 {
                Ok(0)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "incomplete TCP read",
                ))
            };
        }
        total += read;
    }
    Ok(total)
}

/// Write a length-prefixed DNS response over TLS.
async fn tls_write_length_prefixed<S: AsyncWrite + Unpin>(
    stream_mutex: &Arc<std::sync::Mutex<S>>,
    data: &[u8],
) -> io::Result<()> {
    use std::future::poll_fn;
    use std::task::Poll;

    let len_bytes = (data.len() as u16).to_be_bytes();

    let mut written = 0;
    let total = len_bytes.len() + data.len();
    let mut buf = Vec::with_capacity(total);
    buf.extend_from_slice(&len_bytes);
    buf.extend_from_slice(data);

    while written < total {
        let n = poll_fn(|cx| {
            let mut guard = stream_mutex.lock().unwrap();
            Pin::new(&mut *guard).poll_write(cx, &buf[written..])
        })
        .await?;
        if n == 0 {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "TCP write zero"));
        }
        written += n;
    }

    // Flush to ensure data is sent
    poll_fn(|cx| {
        let mut guard = stream_mutex.lock().unwrap();
        Pin::new(&mut *guard).poll_flush(cx)
    })
    .await
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
