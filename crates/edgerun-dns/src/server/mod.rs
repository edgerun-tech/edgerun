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
    /// Maximum queries per second per source IP (0 = unlimited).
    pub rate_limit_qps: u32,
}

impl Default for DnsServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:53".to_string(),
            default_ttl: 3600,
            rate_limit_qps: 0,
        }
    }
}

/// Rate limiter for per-IP query throttling.
#[derive(Clone)]
pub struct RateLimiter {
    /// Max queries per second per IP. 0 = unlimited.
    max_qps: u32,
    /// Per-IP state: (token_count, last_refill_time).
    state: Arc<std::sync::Mutex<HashMap<std::net::IpAddr, (u32, std::time::Instant)>>>,
}

impl RateLimiter {
    fn new(max_qps: u32) -> Self {
        Self {
            max_qps,
            state: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Check if a query from `addr` is allowed.
    fn allow(&self, addr: std::net::IpAddr) -> bool {
        if self.max_qps == 0 { return true; }
        let mut guard = self.state.lock().unwrap();
        let now = std::time::Instant::now();
        let (tokens, last) = guard.entry(addr).or_insert((self.max_qps, now));
        let elapsed = now.duration_since(*last).as_secs_f64();
        *tokens = (*tokens as f64 + elapsed * self.max_qps as f64)
            .min(self.max_qps as f64) as u32;
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
/// via `edgerun_rt::spawn`.
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
    shutdown_flag: Arc<edgerun_rt::RwLock<bool>>,
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
            rate_limiter: RateLimiter::new(config.rate_limit_qps),
            shutdown_flag: Arc::new(edgerun_rt::RwLock::new(false)),
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
            edgerun_rt::spawn(tcp::tcp_accept_loop_with_shutdown(listener, state, rate_limiter, shutdown));
        }

        // Run UDP receive loop.
        udp::udp_recv_loop_with_rate_limiting(
            Arc::clone(&self.udp_socket),
            self.state.clone(),
            self.rate_limiter.clone(),
            Arc::clone(&self.shutdown_flag),
        ).await
    }

    /// Signal the server to shut down.
    ///
    /// Sets the shutdown flag, causing both UDP and TCP loops to exit.
    /// This method returns immediately; [`DnsServer::run()`] will return shortly after.
    pub async fn shutdown(&self) {
        *self.shutdown_flag.write().await = true;
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
