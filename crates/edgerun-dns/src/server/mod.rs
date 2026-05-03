//! Async DNS server — handles queries concurrently using edgerun-rt.
//!
//! Listens on both UDP and TCP. Each query is spawned as a separate
//! async task via `crate::compat::spawn`.

use crate::std::io;
use alloc::collections::BTreeMap as HashMap;
use alloc::sync::Arc;
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::compat::AsyncTcpListener;
use crate::compat::AsyncUdpSocket;

pub mod query;
pub(crate) mod tcp;
pub(crate) mod udp;

pub use query::{handle_query, ServerState, MAX_UDP_RESPONSE};
pub use tcp::handle_tcp_connection_raw;

/// DNS server configuration.
#[derive(Debug, Clone)]
pub struct DnsServerConfig {
    /// Bind address (e.g. "0.0.0.0:53").
    pub bind_addr: String,
    /// Default TTL for records.
    pub default_ttl: u32,
    /// Maximum queries per second per source IP (0 = unlimited).
    pub rate_limit_qps: u32,
    /// Optional IPv6 bind address (e.g. "[::]:53").
    /// If set, a second UDP+TCP listener is created for IPv6.
    pub bind_addr_ipv6: Option<String>,
}

impl Default for DnsServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:53".to_string(),
            default_ttl: 3600,
            rate_limit_qps: 0,
            bind_addr_ipv6: None,
        }
    }
}

/// Rate limiter for per-IP query throttling.
#[derive(Clone)]
pub struct RateLimiter {
    /// Max queries per second per IP. 0 = unlimited.
    max_qps: u32,
    /// Per-IP state: (token_count, last_refill_time).
    state: Arc<
        crate::std::sync::Mutex<HashMap<crate::std::net::IpAddr, (u32, crate::std::time::Instant)>>,
    >,
}

impl RateLimiter {
    pub fn new(max_qps: u32) -> Self {
        Self {
            max_qps,
            state: Arc::new(crate::std::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Check if a query from `addr` is allowed.
    pub fn allow(&self, addr: crate::std::net::IpAddr) -> bool {
        if self.max_qps == 0 {
            return true;
        }
        let mut guard = self.state.lock().unwrap();
        let now = crate::std::time::Instant::now();
        let (tokens, last) = guard.entry(addr).or_insert((self.max_qps, now));
        let elapsed = now.duration_since(*last).as_secs_f64();
        *tokens = (*tokens as f64 + elapsed * self.max_qps as f64).min(self.max_qps as f64) as u32;
        *last = now;
        if *tokens > 0 {
            *tokens -= 1;
            true
        } else {
            false
        }
    }
}

/// DNS server — authoritative server for one or more zones.
///
/// Listens on both UDP and TCP. Each query is handled concurrently
/// via `crate::compat::spawn`.
///
/// # Graceful Shutdown
/// Call [`DnsServer::shutdown()`] to stop the server loops. The [`DnsServer::run()`]
/// method will then return instead of running forever.
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
///     // server.run().await; // runs until shutdown() is called
/// });
/// ```
pub struct DnsServer {
    udp_socket: Arc<AsyncUdpSocket>,
    tcp_listener: Arc<AsyncTcpListener>,
    state: ServerState,
    rate_limiter: RateLimiter,
    /// Shutdown signal — when set to true, server loops exit.
    shutdown_flag: Arc<crate::compat::RwLock<bool>>,
    /// Optional IPv6 UDP socket.
    udp_socket_ipv6: Option<Arc<AsyncUdpSocket>>,
    /// Optional IPv6 TCP listener.
    tcp_listener_ipv6: Option<Arc<AsyncTcpListener>>,
}

impl Clone for DnsServer {
    fn clone(&self) -> Self {
        Self {
            udp_socket: Arc::clone(&self.udp_socket),
            tcp_listener: Arc::clone(&self.tcp_listener),
            state: self.state.clone(),
            rate_limiter: self.rate_limiter.clone(),
            shutdown_flag: Arc::clone(&self.shutdown_flag),
            udp_socket_ipv6: self.udp_socket_ipv6.clone(),
            tcp_listener_ipv6: self.tcp_listener_ipv6.clone(),
        }
    }
}

impl DnsServer {
    /// Create a new DNS server, binding to both UDP and TCP.
    ///
    /// If `config.bind_addr_ipv6` is set, also binds to the IPv6 address.
    pub fn new(config: DnsServerConfig) -> Result<Self, io::Error> {
        let udp_socket = Arc::new(AsyncUdpSocket::bind(&config.bind_addr)?);
        let tcp_listener = Arc::new(AsyncTcpListener::bind(&config.bind_addr)?);

        let local = udp_socket.local_addr().unwrap();
        edgerun_log::info!("edgerun-dns: server bound to {} (UDP + TCP)", local);

        let (udp_ipv6, tcp_ipv6) = if let Some(ref v6_addr) = config.bind_addr_ipv6 {
            let udp6 = Arc::new(AsyncUdpSocket::bind(v6_addr)?);
            let tcp6 = Arc::new(AsyncTcpListener::bind(v6_addr)?);
            let local6 = udp6.local_addr().unwrap();
            edgerun_log::info!("edgerun-dns: server bound to {} (IPv6 UDP + TCP)", local6);
            (Some(udp6), Some(tcp6))
        } else {
            (None, None)
        };

        Ok(Self {
            udp_socket,
            tcp_listener,
            state: ServerState {
                zones: Arc::new(crate::compat::RwLock::new(HashMap::new())),
                default_ttl: config.default_ttl,
                forward_to: Arc::new(crate::compat::RwLock::new(None)),
            },
            rate_limiter: RateLimiter::new(config.rate_limit_qps),
            shutdown_flag: Arc::new(crate::compat::RwLock::new(false)),
            udp_socket_ipv6: udp_ipv6,
            tcp_listener_ipv6: tcp_ipv6,
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

    /// Run the server event loop (async).
    ///
    /// Spawns concurrent loops for UDP and TCP. Returns when [`DnsServer::shutdown()`]
    /// is called or an unrecoverable error occurs.
    pub async fn run(&self) -> io::Result<()> {
        let local = self.udp_socket.local_addr().unwrap();
        let zone_names: Vec<_> = self.state.zones.read().await.keys().cloned().collect();
        edgerun_log::info!("edgerun-dns: server listening on {} (UDP + TCP)", local);
        edgerun_log::info!("  zones: {:?}", zone_names);

        // Spawn TCP accept loop.
        {
            let listener = Arc::clone(&self.tcp_listener);
            let state = self.state.clone();
            let rate_limiter = self.rate_limiter.clone();
            let shutdown = Arc::clone(&self.shutdown_flag);
            crate::compat::spawn(tcp::tcp_accept_loop_with_shutdown(
                listener,
                state,
                rate_limiter,
                shutdown,
            ));
        }

        // Spawn IPv6 loops if configured.
        if let (Some(udp6), Some(tcp6)) = (&self.udp_socket_ipv6, &self.tcp_listener_ipv6) {
            let state = self.state.clone();
            let rate_limiter = self.rate_limiter.clone();
            let shutdown = Arc::clone(&self.shutdown_flag);
            let udp6 = Arc::clone(udp6);
            let tcp6 = Arc::clone(tcp6);
            crate::compat::spawn(udp::udp_recv_loop_with_rate_limiting(
                udp6,
                state.clone(),
                rate_limiter.clone(),
                shutdown.clone(),
            ));
            crate::compat::spawn(tcp::tcp_accept_loop_with_shutdown(
                tcp6,
                state,
                rate_limiter,
                shutdown,
            ));
        }

        // Run UDP receive loop.
        udp::udp_recv_loop_with_rate_limiting(
            Arc::clone(&self.udp_socket),
            self.state.clone(),
            self.rate_limiter.clone(),
            Arc::clone(&self.shutdown_flag),
        )
        .await
    }

    /// Signal the server to shut down.
    ///
    /// Sets the shutdown flag, causing both UDP and TCP loops to exit.
    /// This method returns immediately; [`DnsServer::run()`] will return shortly after.
    pub async fn shutdown(&self) {
        *self.shutdown_flag.write().await = true;
        self.tcp_listener.unblock_accept();
        if let Some(listener) = &self.tcp_listener_ipv6 {
            listener.unblock_accept();
        }
        edgerun_log::info!("edgerun-dns: shutdown requested");
    }

    /// Check if shutdown has been requested.
    pub async fn is_shutting_down(&self) -> bool {
        *self.shutdown_flag.read().await
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
