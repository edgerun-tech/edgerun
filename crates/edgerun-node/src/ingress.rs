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
    if recent_hashes.contains(msg_hash) {
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

    // -----------------------------------------------------------------------
    // Token bucket edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn token_bucket_zero_capacity_rejects_all() {
        let mut bucket = TokenBucket::new(0, 0);
        assert!(!bucket.try_consume());
        assert!(!bucket.try_consume());
    }

    #[test]
    fn token_bucket_zero_refill_no_recovery() {
        // Once exhausted, a zero-refill bucket never recovers
        let mut bucket = TokenBucket::new(1, 0);
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume());
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(!bucket.try_consume());
    }

    #[test]
    fn token_bucket_single_token() {
        let mut bucket = TokenBucket::new(1, 1);
        assert!(bucket.try_consume());
        assert!(!bucket.try_consume());
    }

    #[test]
    fn token_bucket_does_not_exceed_max() {
        // Even with long sleep, tokens should cap at max_tokens
        let mut bucket = TokenBucket::new(3, 100);
        // Consume all
        bucket.try_consume();
        bucket.try_consume();
        bucket.try_consume();
        assert!(!bucket.try_consume());
        // Sleep enough to refill past max
        std::thread::sleep(std::time::Duration::from_millis(200));
        // Should have some tokens back but never exceed 3
        bucket.try_consume();
        bucket.try_consume();
        bucket.try_consume();
        assert!(!bucket.try_consume()); // should still be capped
    }

    #[test]
    fn token_bucket_high_refill_rate() {
        // With max_tokens=5, high refill rate refills multiple tokens between consumes
        let mut bucket = TokenBucket::new(5, 1000); // burst 5, 1000/sec
                                                    // Consume all 5 initial tokens
        for _ in 0..5 {
            assert!(bucket.try_consume());
        }
        // Now tokens are at 0
        assert!(!bucket.try_consume());
        // Sleep to refill (20ms at 1000/sec = 20 tokens, but capped at 5)
        std::thread::sleep(std::time::Duration::from_millis(20));
        // Should have refilled up to 5 tokens
        for _ in 0..5 {
            assert!(bucket.try_consume());
        }
    }

    #[test]
    fn token_bucket_clone() {
        let bucket = TokenBucket::new(5, 1);
        let mut cloned = bucket.clone();
        // Both start with same state
        assert!(cloned.try_consume());
    }

    #[test]
    fn token_bucket_debug_repr() {
        let bucket = TokenBucket::new(10, 5);
        let debug_str = format!("{:?}", bucket);
        assert!(debug_str.contains("TokenBucket"));
    }

    // -----------------------------------------------------------------------
    // Recent hash cache edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn recent_hash_cache_capacity_zero() {
        let mut cache = RecentHashCache::new(0);
        // With capacity 0, len >= capacity is true (0 >= 0),
        // but pop_front from empty VecDeque returns None (no-op),
        // then the item gets pushed. So it stores 1 item.
        cache.insert(42);
        assert!(cache.contains(42));
    }

    #[test]
    fn recent_hash_cache_capacity_one() {
        let mut cache = RecentHashCache::new(1);
        cache.insert(100);
        assert!(cache.contains(100));
        cache.insert(200); // evicts 100
        assert!(!cache.contains(100));
        assert!(cache.contains(200));
    }

    #[test]
    fn recent_hash_cache_multiple_evictions() {
        let mut cache = RecentHashCache::new(3);
        cache.insert(1);
        cache.insert(2);
        cache.insert(3);
        cache.insert(4); // evicts 1
        cache.insert(5); // evicts 2
        assert!(!cache.contains(1));
        assert!(!cache.contains(2));
        assert!(cache.contains(3));
        assert!(cache.contains(4));
        assert!(cache.contains(5));
    }

    #[test]
    fn recent_hash_cache_reinsert_same_hash_no_eviction() {
        let mut cache = RecentHashCache::new(2);
        cache.insert(1);
        cache.insert(1); // re-insert same, set dedup but VecDeque still grows
                         // The hash is still in the cache
        assert!(cache.contains(1));
        // VecDeque has 2 entries but set has 1
        assert_eq!(cache.set.len(), 1);
        assert_eq!(cache.hashes.len(), 2);
    }

    #[test]
    fn quick_hash_empty_input() {
        let hash = quick_message_hash(&[]);
        // FNV-1a of empty input is the offset basis
        assert_eq!(hash, 0xcbf29ce484222325u64);
    }

    #[test]
    fn quick_hash_single_byte() {
        let h1 = quick_message_hash(&[0]);
        let h2 = quick_message_hash(&[1]);
        assert_ne!(h1, h2);
    }

    #[test]
    fn quick_hash_different_lengths() {
        let h1 = quick_message_hash(b"a");
        let h2 = quick_message_hash(b"aa");
        assert_ne!(h1, h2);
    }

    #[test]
    fn quick_hash_collision_resistance_basic() {
        // Two very similar strings should produce different hashes
        let h1 = quick_message_hash(b"message-v1");
        let h2 = quick_message_hash(b"message-v2");
        assert_ne!(h1, h2);
    }

    // -----------------------------------------------------------------------
    // Peer allowlist edge cases
    // -----------------------------------------------------------------------

    #[test]
    fn peer_allowlist_empty_peer_id_allowed_in_open_mode() {
        assert!(is_peer_allowed(&[], &[]));
    }

    #[test]
    fn peer_allowlist_empty_peer_id_in_strict_mode() {
        let allowed = vec![vec![1, 2, 3]];
        assert!(!is_peer_allowed(&[], &allowed));
    }

    #[test]
    fn peer_allowlist_exact_match() {
        let allowed = vec![vec![0xDE, 0xAD, 0xBE, 0xEF]];
        assert!(is_peer_allowed(&[0xDE, 0xAD, 0xBE, 0xEF], &allowed));
    }

    #[test]
    fn peer_allowlist_multiple_peers() {
        let allowed = vec![vec![1], vec![2], vec![3], vec![4]];
        assert!(is_peer_allowed(&[3], &allowed));
        assert!(!is_peer_allowed(&[5], &allowed));
    }

    #[test]
    fn peer_allowlist_single_entry() {
        let allowed = vec![vec![42]];
        assert!(is_peer_allowed(&[42], &allowed));
        assert!(!is_peer_allowed(&[43], &allowed));
    }

    #[test]
    fn peer_allowlist_large_identity() {
        let big_id: Vec<u8> = (0..256).map(|i| (i % 256) as u8).collect();
        let allowed = vec![big_id.clone()];
        assert!(is_peer_allowed(&big_id, &allowed));
        let mut other = big_id.clone();
        other[0] ^= 0xFF;
        assert!(!is_peer_allowed(&other, &allowed));
    }

    // -----------------------------------------------------------------------
    // IngressResult
    // -----------------------------------------------------------------------

    #[test]
    fn ingress_result_debug_format() {
        let results = [
            IngressResult::Allow,
            IngressResult::RateLimited,
            IngressResult::Duplicate,
            IngressResult::PeerNotAllowed,
        ];
        for r in &results {
            let s = format!("{:?}", r);
            assert!(!s.is_empty());
        }
    }

    #[test]
    fn ingress_result_equality() {
        assert_eq!(IngressResult::Allow, IngressResult::Allow);
        assert_ne!(IngressResult::Allow, IngressResult::RateLimited);
        assert_ne!(IngressResult::Duplicate, IngressResult::PeerNotAllowed);
    }

    // -----------------------------------------------------------------------
    // screen_message composite screening
    // -----------------------------------------------------------------------

    #[test]
    fn screen_message_allows_valid_message() {
        let mut bucket = TokenBucket::new(100, 100);
        let mut cache = RecentHashCache::new(100);
        let msg = b"test message";
        let result = screen_message(
            msg,
            Some(&[1u8; 64]),
            &[1u8; 64],
            &mut bucket,
            &mut cache,
            &[], // open peer mode
            None,
        );
        assert_eq!(result, IngressResult::Allow);
    }

    #[test]
    fn screen_message_rejects_wrong_target() {
        let mut bucket = TokenBucket::new(100, 100);
        let mut cache = RecentHashCache::new(100);
        let result = screen_message(
            b"msg",
            Some(&[1u8; 64]),
            &[2u8; 64], // different node
            &mut bucket,
            &mut cache,
            &[],
            None,
        );
        assert_eq!(result, IngressResult::PeerNotAllowed);
    }

    #[test]
    fn screen_message_detects_duplicate() {
        let mut bucket = TokenBucket::new(100, 100);
        let mut cache = RecentHashCache::new(100);
        let msg = b"same message";
        let r1 = screen_message(msg, None, &[0u8; 64], &mut bucket, &mut cache, &[], None);
        assert_eq!(r1, IngressResult::Allow);
        let r2 = screen_message(msg, None, &[0u8; 64], &mut bucket, &mut cache, &[], None);
        assert_eq!(r2, IngressResult::Duplicate);
    }

    #[test]
    fn screen_message_respects_peer_allowlist() {
        let mut bucket = TokenBucket::new(100, 100);
        let mut cache = RecentHashCache::new(100);
        let allowed = vec![vec![1u8, 2, 3]];
        let result = screen_message(
            b"msg",
            None,
            &[0u8; 64],
            &mut bucket,
            &mut cache,
            &allowed,
            Some(&[9u8, 9, 9]), // not in allowlist
        );
        assert_eq!(result, IngressResult::PeerNotAllowed);
    }

    #[test]
    fn screen_message_rate_limits() {
        let mut bucket = TokenBucket::new(1, 0); // 1 burst, no refill
        let mut cache = RecentHashCache::new(100);
        let msg1 = b"message one";
        let msg2 = b"message two";
        assert_eq!(
            screen_message(msg1, None, &[0u8; 64], &mut bucket, &mut cache, &[], None),
            IngressResult::Allow
        );
        assert_eq!(
            screen_message(msg2, None, &[0u8; 64], &mut bucket, &mut cache, &[], None),
            IngressResult::RateLimited
        );
    }
}
