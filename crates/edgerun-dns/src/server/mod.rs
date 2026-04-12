//! Async DNS server — handles queries concurrently using edgerun-rt.
//!
//! Listens on both UDP and TCP. Each query is spawned as a separate
//! async task via `edgerun_rt::spawn`.

use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use edgerun_rt::AsyncTcpListener;
use edgerun_rt::AsyncUdpSocket;

pub mod query;
mod udp;
mod tcp;

pub use query::{ServerState, handle_query, MAX_UDP_RESPONSE};

/// DNS server configuration.
#[derive(Debug, Clone)]
pub struct DnsServerConfig {
    /// Bind address (e.g. "0.0.0.0:53").
    pub bind_addr: String,
    /// Default TTL for records.
    pub default_ttl: u32,
}

impl Default for DnsServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:53".to_string(),
            default_ttl: 3600,
        }
    }
}

/// DNS server — authoritative server for one or more zones.
///
/// Listens on both UDP and TCP. Each query is handled concurrently
/// via `edgerun_rt::spawn`.
///
/// # Example
/// ```no_run
/// use edgerun_dns::server::{DnsServer, DnsServerConfig};
/// use edgerun_dns::zone::DnsZone;
/// use edgerun_rt::Runtime;
/// use std::net::Ipv4Addr;
///
/// let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
/// rt.block_on(async {
///     let mut zone = DnsZone::new("example.com");
///     zone.add_a("@", Ipv4Addr::new(192, 168, 1, 1), 3600);
///
///     let config = DnsServerConfig::default();
///     let server = DnsServer::new(config).unwrap();
///     server.add_zone(zone).await;
///     // server.run().await; // runs forever
/// });
/// ```
pub struct DnsServer {
    udp_socket: Arc<AsyncUdpSocket>,
    tcp_listener: Arc<AsyncTcpListener>,
    state: ServerState,
}

impl DnsServer {
    /// Create a new DNS server, binding to both UDP and TCP.
    pub fn new(config: DnsServerConfig) -> Result<Self, io::Error> {
        let udp_socket = Arc::new(AsyncUdpSocket::bind(&config.bind_addr)?);
        let tcp_listener = Arc::new(AsyncTcpListener::bind(&config.bind_addr)?);

        let local = udp_socket.local_addr().unwrap();
        edgerun_log::info!(
            "edgerun-dns: server bound to {} (UDP + TCP)",
            local
        );

        Ok(Self {
            udp_socket,
            tcp_listener,
            state: ServerState {
                zones: Arc::new(edgerun_rt::RwLock::new(HashMap::new())),
                default_ttl: config.default_ttl,
                forward_to: Arc::new(edgerun_rt::RwLock::new(None)),
            },
        })
    }

    /// Add a zone to this server.
    pub async fn add_zone(&self, zone: crate::zone::DnsZone) {
        let origin = zone.origin.clone();
        self.state.zones.write().await.insert(origin, zone);
    }

    /// Remove a zone.
    pub async fn remove_zone(&self, origin: &str) {
        self.state.zones.write().await.remove(origin);
    }

    /// Set the upstream resolver address for recursive forwarding.
    pub async fn set_forward_to(&self, addr: Option<String>) {
        *self.state.forward_to.write().await = addr;
    }

    /// Run the server event loop (async, runs forever).
    ///
    /// Spawns concurrent loops for UDP and TCP. Never returns.
    pub async fn run(&self) -> ! {
        let local = self.udp_socket.local_addr().unwrap();
        let zone_names: Vec<_> = self.state.zones.read().await.keys().cloned().collect();
        edgerun_log::info!("edgerun-dns: server listening on {} (UDP + TCP)", local);
        edgerun_log::info!("  zones: {:?}", zone_names);

        // Spawn TCP accept loop.
        {
            let listener = Arc::clone(&self.tcp_listener);
            let state = self.state.clone();
            edgerun_rt::spawn(tcp::tcp_accept_loop(listener, state));
        }

        // Run UDP receive loop in this task.
        udp::udp_recv_loop(Arc::clone(&self.udp_socket), self.state.clone()).await
    }

    /// Get zone count.
    pub async fn zone_count(&self) -> usize {
        self.state.zones.read().await.len()
    }

    /// Get the names of all loaded zones.
    pub async fn zone_names(&self) -> Vec<String> {
        self.state.zones.read().await.keys().cloned().collect()
    }
}
