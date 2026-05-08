//! Ingress screening for the edgerun node daemon.
//!
//! Implements §18.3: not every received byte sequence deserves a durable
//! rejection event. A node MAY silently drop, rate-limit, or locally
//! quarantine traffic that fails cheap pre-filtering.
//!
//! Screening order (cheap → expensive):
//! 1. Size and framing sanity (done by TCP framing)
//! 2. Rate limit check (token bucket, per-connection and global)
//! 3. Recent duplicate detection (hash cache, prevents redundant work)
//! 4. Peer allowlist check (if configured)
//! 5. Only then: deeper signature, decryption, and authority work

use std::collections::HashSet;
use std::collections::VecDeque;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Token bucket rate limiter
// ---------------------------------------------------------------------------

/// A simple token bucket rate limiter.
///
/// Tokens are added at a fixed rate up to a maximum capacity.
/// Each operation consumes one token. If no tokens are available,
/// the operation is rejected.
#[derive(Clone, Debug)]
pub struct TokenBucket {
    tokens: f64,
    max_tokens: f64,
    refill_rate: f64, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    /// Creates a new token bucket.
    ///
    /// `max_tokens` is the burst capacity.
    /// `refill_rate` is how many tokens are added per second.
    pub fn new(max_tokens: u64, refill_rate: u64) -> Self {
        Self {
            tokens: max_tokens as f64,
            max_tokens: max_tokens as f64,
            refill_rate: refill_rate as f64,
            last_refill: Instant::now(),
        }
    }

    /// Refill tokens based on elapsed time.
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.max_tokens);
        self.last_refill = now;
    }

    /// Attempts to consume one token.
    ///
    /// Returns `true` if the operation is allowed, `false` if rate-limited.
    pub fn try_consume(&mut self) -> bool {
        self.refill();
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

// ---------------------------------------------------------------------------
// Recent message hash cache (bounded dedup)
// ---------------------------------------------------------------------------

/// A bounded cache of recent message hashes for duplicate detection.
///
/// Stores SHA-256 hashes of recently seen messages. When the cache is full,
/// the oldest entry is evicted. This prevents redundant expensive work
/// (signature verification, etc.) on duplicate messages.
pub struct RecentHashCache {
    hashes: VecDeque<[u8; 32]>,
    set: HashSet<[u8; 32]>,
    capacity: usize,
}

impl RecentHashCache {
    /// Creates a new cache with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            hashes: VecDeque::with_capacity(capacity),
            set: HashSet::with_capacity(capacity),
            capacity,
        }
    }

    /// Checks if a hash is already in the cache.
    pub fn contains(&self, hash: &[u8; 32]) -> bool {
        self.set.contains(hash)
    }

    /// Inserts a hash, evicting the oldest if at capacity.
    pub fn insert(&mut self, hash: [u8; 32]) {
        if self.capacity == 0 || self.set.contains(&hash) {
            return;
        }
        if self.hashes.len() >= self.capacity {
            if let Some(oldest) = self.hashes.pop_front() {
                self.set.remove(&oldest);
            }
        }
        self.hashes.push_back(hash);
        self.set.insert(hash);
    }
}

/// Computes the SHA-256 hash used for duplicate detection before authority work.
#[inline]
pub fn quick_message_hash(bytes: &[u8]) -> [u8; 32] {
    let digest = edgerun_protocols::core_protocol::crypto::sha256(bytes);
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

// ---------------------------------------------------------------------------
// Ingress filter result
// ---------------------------------------------------------------------------

/// Result of ingress screening.
#[derive(Debug, PartialEq)]
pub enum IngressResult {
    /// Message passed all cheap checks — proceed to crypto/authority work.
    Allow,
    /// Rate-limited — drop silently.
    RateLimited,
    /// Recently seen duplicate — drop silently.
    Duplicate,
    /// Peer not in allowlist — drop silently.
    PeerNotAllowed,
}

// ---------------------------------------------------------------------------
// Peer allowlist
// ---------------------------------------------------------------------------

/// Checks if a peer identity is allowed.
///
/// If `allowed_peers` is empty, all peers are allowed (open mode).
/// If non-empty, only listed peers are permitted.
pub fn is_peer_allowed(peer_id: &[u8], allowed_peers: &[Vec<u8>]) -> bool {
    if allowed_peers.is_empty() {
        return true;
    }
    allowed_peers.iter().any(|p| p.as_slice() == peer_id)
}

// ---------------------------------------------------------------------------
// Composite screening (spec §18.3 correct order)
// ---------------------------------------------------------------------------

/// Screens an incoming message through the full cheap pre-filter pipeline.
///
/// Spec §18.3 recommended order:
/// 1. Size and framing sanity (done by TCP framing layer — assumed OK here)
/// 2. Target relevance (does this message target MY node?)
/// 3. Duplicate or already-known hash/head check
/// 4. Local interest check (peer allowlist)
/// 5. Rate limit
/// 6. Only then: deeper signature, decryption, and authority work
///
/// All parameters are mutable references so the caller's rate limiter and
/// duplicate cache are updated in place.
pub fn screen_message(
    message_bytes: &[u8],
    target_node_id: Option<&[u8]>,
    local_node_id: &[u8],
    rate_limiter: &mut TokenBucket,
    recent_hashes: &mut RecentHashCache,
    allowed_peers: &[Vec<u8>],
    peer_id: Option<&[u8]>,
) -> IngressResult {
    // Step 2: Target relevance — drop messages not targeting this node
    if let Some(target) = target_node_id {
        if target != local_node_id {
            return IngressResult::PeerNotAllowed; // mis-targeted
        }
    }

    // Step 3: Duplicate detection (before rate limit to avoid wasting tokens on dups)
    let msg_hash = quick_message_hash(message_bytes);
    if recent_hashes.contains(&msg_hash) {
        return IngressResult::Duplicate;
    }

    // Step 4: Local interest / peer allowlist
    if let Some(pid) = peer_id {
        if !is_peer_allowed(pid, allowed_peers) {
            return IngressResult::PeerNotAllowed;
        }
    }

    // Step 5: Rate limit
    if !rate_limiter.try_consume() {
        return IngressResult::RateLimited;
    }

    // Record the hash after passing all checks (so we don't cache irrelevant msgs)
    recent_hashes.insert(msg_hash);

    IngressResult::Allow
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;
