//! Per-IP rate limiting for SMTP connections.
//!
//! Uses a sliding window counter: tracks connection count per IP
//! within a configurable time window. When the limit is exceeded,
//! new connections are temporarily rejected.

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::{Duration, Instant};

use edgerun_rt::Mutex;

/// Configuration for per-IP rate limiting.
#[derive(Clone, Debug)]
pub struct RateLimitConfig {
    /// Maximum number of connections per IP within the window.
    pub max_connections_per_ip: usize,
    /// Duration of the sliding window.
    pub window: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_connections_per_ip: 100,
            window: Duration::from_secs(60),
        }
    }
}

/// Per-IP rate limiter.
pub struct RateLimiter {
    config: RateLimitConfig,
    entries: Mutex<HashMap<IpAddr, Vec<Instant>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            entries: Mutex::new(HashMap::new()),
        }
    }

    /// Check if a connection from the given IP is allowed.
    /// Returns `true` if allowed, `false` if rate limited.
    pub async fn is_allowed(&self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let cutoff = now - self.config.window;

        let mut entries = self.entries.lock().await;
        let timestamps = entries.entry(ip).or_insert_with(Vec::new);

        // Prune old entries
        timestamps.retain(|&t| t > cutoff);

        if timestamps.len() >= self.config.max_connections_per_ip {
            false
        } else {
            timestamps.push(now);
            true
        }
    }

    /// Remove an IP's entry (e.g., after disconnect).
    pub async fn release(&self, ip: IpAddr) {
        let mut entries = self.entries.lock().await;
        if let Some(timestamps) = entries.get_mut(&ip) {
            timestamps.pop();
            if timestamps.is_empty() {
                entries.remove(&ip);
            }
        }
    }
}
