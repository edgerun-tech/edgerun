//! TLS 1.3 session resumption via NewSessionTicket caching.
//!
//! After a successful handshake, the server sends a `NewSessionTicket`
//! message (post-handshake) containing a session ticket. We cache it
//! and reuse on the next connection, reducing handshake from 1-RTT
//! (with certificate exchange) to a PSK-resumed handshake.
//!
//! # Architecture
//! ```text
//! SessionCache (shared, per-client)
//!   └─ HashMap<server_name, Vec<SessionTicket>>
//!         └─ SessionTicket { ticket, cipher_suite, age, received_at }
//!
//! Client flow:
//!   1. Before handshake: look up cached ticket for server_name
//!   2. If found: include `pre_shared_key` extension in ClientHello
//!   3. Server responds with selected PSK (or falls back to full handshake)
//!   4. After handshake: parse `NewSessionTicket` and cache new ticket
//! ```

use crate::std;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A cached session ticket from a NewSessionTicket message.
#[derive(Clone)]
pub struct SessionTicket {
    /// The opaque ticket bytes from the server.
    pub ticket: Vec<u8>,
    /// The cipher suite negotiated in the original handshake.
    pub cipher_suite: u16,
    /// Ticket lifetime in seconds (from the server's NewSessionTicket).
    pub lifetime: u32,
    /// Age of the ticket at receipt (for obfuscated_ticket_age calculation).
    pub age_add: u32,
    /// When we received this ticket.
    pub received_at: Instant,
}

impl SessionTicket {
    /// Check if this ticket is still valid (within its lifetime).
    pub fn is_valid(&self) -> bool {
        self.received_at.elapsed() < Duration::from_secs(self.lifetime as u64)
    }

    /// Calculate the obfuscated ticket age for the PSK binder.
    pub fn obfuscated_age(&self) -> u32 {
        let elapsed_ms = self.received_at.elapsed().as_millis() as u32;
        elapsed_ms.wrapping_add(self.age_add)
    }
}

/// Shared session ticket cache for TLS 1.3 session resumption.
///
/// Stores tickets keyed by server name. Tickets are automatically
/// evicted when they expire.
///
/// Clone the Arc to share across multiple clients.
#[derive(Clone)]
pub struct SessionCache {
    inner: Arc<Mutex<HashMap<String, Vec<SessionTicket>>>>,
    /// Maximum tickets to keep per server.
    max_per_server: usize,
}

impl SessionCache {
    /// Create a new session cache.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            max_per_server: 4,
        }
    }

    /// Set maximum tickets to cache per server.
    pub fn with_max_per_server(mut self, n: usize) -> Self {
        self.max_per_server = n;
        self
    }

    /// Store a session ticket for a server.
    pub fn store(&self, server_name: &str, ticket: SessionTicket) {
        if ticket.ticket.is_empty() {
            return;
        }
        let mut map = self.inner.lock();
        let tickets = map.entry(server_name.to_string()).or_default();
        // Evict expired tickets
        tickets.retain(|t| t.is_valid());
        // Don't store duplicates
        if tickets.iter().any(|t| t.ticket == ticket.ticket) {
            return;
        }
        // Enforce max
        if tickets.len() >= self.max_per_server {
            // Remove oldest first
            if let Some(pos) = tickets.iter().position(|t| {
                t.received_at
                    == tickets
                        .iter()
                        .map(|x| x.received_at)
                        .min()
                        .unwrap_or(t.received_at)
            }) {
                tickets.remove(pos);
            }
        }
        tickets.push(ticket);
    }

    /// Get a valid session ticket for a server, if available.
    /// Returns the most recently received valid ticket.
    pub fn get(&self, server_name: &str) -> Option<SessionTicket> {
        let map = self.inner.lock();
        if let Some(tickets) = map.get(server_name) {
            // Return the most recent valid ticket
            tickets.iter().rev().find(|t| t.is_valid()).cloned()
        } else {
            None
        }
    }

    /// Clear all cached tickets.
    pub fn clear(&self) {
        self.inner.lock().clear();
    }

    /// Remove expired tickets and return count of remaining.
    pub fn prune(&self) -> usize {
        let mut map = self.inner.lock();
        for tickets in map.values_mut() {
            tickets.retain(|t| t.is_valid());
        }
        map.values().map(|v| v.len()).sum()
    }
}

impl Default for SessionCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a NewSessionTicket message from encrypted handshake data.
///
/// NewSessionTicket format (RFC 8446 §4.6.1):
///   ticket_lifetime (4 bytes)
///   ticket_age_add (4 bytes)
///   ticket_nonce (1 byte len + data)
///   ticket (2 bytes len + data)
///   extensions (2 bytes len + data)
pub fn parse_new_session_ticket(data: &[u8]) -> Option<SessionTicket> {
    if data.len() < 10 {
        return None;
    }

    let lifetime = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
    let age_add = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);

    // Skip ticket_nonce
    let nonce_len = data[8] as usize;
    let pos = 9 + nonce_len;
    if pos + 2 > data.len() {
        return None;
    }

    let ticket_len = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;
    let ticket_start = pos + 2;
    if ticket_start + ticket_len > data.len() {
        return None;
    }

    let ticket = data[ticket_start..ticket_start + ticket_len].to_vec();

    Some(SessionTicket {
        ticket,
        cipher_suite: 0x1301, // TLS_AES_128_GCM_SHA256 (default for our client)
        lifetime,
        age_add,
        received_at: Instant::now(),
    })
}
