//! Ingress screening for the Lifegraph node daemon.
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

use std::collections::VecDeque;
use std::collections::HashSet;
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
    refill_rate: f64,   // tokens per second
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
    hashes: VecDeque<u64>,
    set: HashSet<u64>,
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
    pub fn contains(&self, hash: u64) -> bool {
        self.set.contains(&hash)
    }

    /// Inserts a hash, evicting the oldest if at capacity.
    pub fn insert(&mut self, hash: u64) {
        if self.hashes.len() >= self.capacity {
            if let Some(oldest) = self.hashes.pop_front() {
                self.set.remove(&oldest);
            }
        }
        self.hashes.push_back(hash);
        self.set.insert(hash);
    }
}

/// Computes a fast 64-bit hash of the message bytes for dedup purposes.
/// This is NOT cryptographic — it's only for quick duplicate detection
/// before expensive signature verification.
#[inline]
pub fn quick_message_hash(bytes: &[u8]) -> u64 {
    // FNV-1a 64-bit: fast enough for dedup, no crypto dependency
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

// ---------------------------------------------------------------------------
// Ingress filter result
// ---------------------------------------------------------------------------

/// Result of ingress screening.
#[derive(Debug, PartialEq)]
#[allow(dead_code)]
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
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_bucket_allows_burst() {
        let mut bucket = TokenBucket::new(5, 1); // burst of 5, 1/sec refill
        // Should allow 5 in a row
        for _ in 0..5 {
            assert!(bucket.try_consume());
        }
        // 6th should fail
        assert!(!bucket.try_consume());
    }

    #[test]
    fn token_bucket_refills_over_time() {
        let mut bucket = TokenBucket::new(1, 10); // burst of 1, 10/sec
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume());
        std::thread::sleep(std::time::Duration::from_millis(200)); // 2 tokens worth
        assert!(bucket.try_consume());
    }

    #[test]
    fn recent_hash_cache_dedup() {
        let mut cache = RecentHashCache::new(3);
        cache.insert(42);
        assert!(cache.contains(42));
        assert!(!cache.contains(99));
    }

    #[test]
    fn recent_hash_cache_evicts_oldest() {
        let mut cache = RecentHashCache::new(2);
        cache.insert(1);
        cache.insert(2);
        cache.insert(3); // evicts 1
        assert!(!cache.contains(1));
        assert!(cache.contains(2));
        assert!(cache.contains(3));
    }

    #[test]
    fn peer_allowlist_empty_allows_all() {
        assert!(is_peer_allowed(b"any-peer", &[]));
    }

    #[test]
    fn peer_allowlist_blocks_unknown() {
        let allowed = vec![vec![1, 2, 3], vec![4, 5, 6]];
        assert!(is_peer_allowed(&[1, 2, 3], &allowed));
        assert!(!is_peer_allowed(&[7, 8, 9], &allowed));
    }

    #[test]
    fn quick_hash_deterministic() {
        let data = b"hello world";
        assert_eq!(quick_message_hash(data), quick_message_hash(data));
        assert_ne!(quick_message_hash(data), quick_message_hash(b"hello worle"));
    }
}
