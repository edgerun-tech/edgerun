//! DNS response cache with TTL-based expiry.
//!
//! Stores parsed DNS responses keyed by `(name, record_type)`.
//! Each entry has an expiry time derived from the record's TTL.
//! Expired entries are transparently removed on access.
//!
//! # Example
//! ```no_run
//! use edgerun_dns::cache::DnsCache;
//! use edgerun_dns::message::DnsMessage;
//!
//! let mut cache = DnsCache::with_capacity(1024);
//! // Cache a response with 300s TTL
//! // cache.insert("example.com".to_string(), response, 300);
//! // let cached = cache.get("example.com");
//! ```

use crate::std::time::{Duration, Instant};
use alloc::collections::BTreeMap as HashMap;
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use super::message::DnsRecord;
use super::record::DnsRecordType;

/// A single cache entry with expiry.
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Cached answer records.
    records: Vec<DnsRecord>,
    /// Instant when these records should expire.
    expires_at: Instant,
}

impl CacheEntry {
    fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// Remaining time-to-live for this entry.
    fn ttl_remaining(&self) -> Duration {
        self.expires_at.saturating_duration_since(Instant::now())
    }
}

/// DNS cache with TTL-based expiry.
///
/// Thread-safe via `Arc<Mutex<...>>`. Clone to share across tasks.
#[derive(Clone)]
pub struct DnsCache {
    inner: alloc::sync::Arc<crate::std::sync::Mutex<HashMap<String, CacheEntry>>>,
    max_entries: usize,
}

impl DnsCache {
    /// Create a cache with default capacity of 1024 entries.
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Create a cache with the given maximum entry count.
    pub fn with_capacity(max_entries: usize) -> Self {
        Self {
            inner: alloc::sync::Arc::new(crate::std::sync::Mutex::new(HashMap::new())),
            max_entries,
        }
    }

    /// Build a cache key from a name and record type.
    fn cache_key(name: &str, qtype: DnsRecordType) -> String {
        format!("{}\0{}", name.to_lowercase(), qtype.as_u16())
    }

    /// Insert records into the cache with the given TTL.
    ///
    /// The expiry is computed from `now + ttl` seconds.
    pub fn insert(&self, name: &str, qtype: DnsRecordType, records: Vec<DnsRecord>, ttl: u32) {
        let key = Self::cache_key(name, qtype);
        let entry = CacheEntry {
            records,
            expires_at: Instant::now() + Duration::from_secs(ttl as u64),
        };

        let mut guard = self.inner.lock().unwrap();

        // Evict expired entries if we're at capacity.
        if guard.len() >= self.max_entries && !guard.contains_key(&key) {
            self.evict_expired(&mut guard);
        }

        // If still at capacity, remove oldest entry.
        if guard.len() >= self.max_entries && !guard.contains_key(&key) {
            self.evict_oldest(&mut guard);
        }

        guard.insert(key, entry);
    }

    /// Get cached records for a name+type, returning `None` if not found or expired.
    pub fn get(&self, name: &str, qtype: DnsRecordType) -> Option<Vec<DnsRecord>> {
        let key = Self::cache_key(name, qtype);
        let mut guard = self.inner.lock().unwrap();

        if let Some(entry) = guard.get(&key) {
            if entry.is_expired() {
                guard.remove(&key);
                return None;
            }
            return Some(entry.records.clone());
        }
        None
    }

    /// Check if a key exists and is not expired (without removing it).
    pub fn contains(&self, name: &str, qtype: DnsRecordType) -> bool {
        let key = Self::cache_key(name, qtype);
        let guard = self.inner.lock().unwrap();
        guard.get(&key).map(|e| !e.is_expired()).unwrap_or(false)
    }

    /// Remove a specific entry.
    pub fn remove(&self, name: &str, qtype: DnsRecordType) {
        let key = Self::cache_key(name, qtype);
        let mut guard = self.inner.lock().unwrap();
        guard.remove(&key);
    }

    /// Remove all cached entries.
    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }

    /// Number of cached entries (including expired ones that haven't been collected yet).
    pub fn len(&self) -> usize {
        self.inner.lock().unwrap().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Evict all expired entries (internal).
    fn evict_expired(&self, guard: &mut HashMap<String, CacheEntry>) {
        guard.retain(|_, entry| !entry.is_expired());
    }

    /// Evict the oldest entry (earliest expiry).
    fn evict_oldest(&self, guard: &mut HashMap<String, CacheEntry>) {
        if let Some(oldest_key) = guard
            .iter()
            .min_by_key(|(_, entry)| entry.expires_at)
            .map(|(key, _)| key.clone())
        {
            guard.remove(&oldest_key);
        }
    }

    /// Evict expired entries and return the count removed.
    pub fn prune(&self) -> usize {
        let mut guard = self.inner.lock().unwrap();
        let before = guard.len();
        self.evict_expired(&mut guard);
        before - guard.len()
    }

    /// Cache statistics.
    pub fn stats(&self) -> CacheStats {
        let guard = self.inner.lock().unwrap();
        let now = Instant::now();
        let mut expired = 0;
        let mut total_ttl: u64 = 0;
        for entry in guard.values() {
            if entry.expires_at <= now {
                expired += 1;
            }
            total_ttl += entry.ttl_remaining().as_secs();
        }
        CacheStats {
            total: guard.len(),
            expired,
            avg_ttl_secs: if guard.is_empty() {
                0
            } else {
                total_ttl / guard.len() as u64
            },
        }
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total number of entries.
    pub total: usize,
    /// Number of expired (but not yet collected) entries.
    pub expired: usize,
    /// Average remaining TTL across all entries.
    pub avg_ttl_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::std::net::Ipv4Addr;
    use core::str::FromStr;

    fn make_a(ip: &str, ttl: u32) -> Vec<DnsRecord> {
        vec![DnsRecord::a(
            "example.com".to_string(),
            Ipv4Addr::from_str(ip).unwrap(),
            ttl,
        )]
    }

    #[test]
    fn test_cache_insert_and_get() {
        let cache = DnsCache::new();
        let records = make_a("1.2.3.4", 300);
        cache.insert("example.com", DnsRecordType::A, records.clone(), 300);

        let cached = cache.get("example.com", DnsRecordType::A).unwrap();
        assert_eq!(cached.len(), 1);
    }

    #[test]
    fn test_cache_miss() {
        let cache = DnsCache::new();
        assert!(cache.get("nonexistent", DnsRecordType::A).is_none());
    }

    #[test]
    fn test_cache_expiry() {
        let cache = DnsCache::new();
        let records = make_a("1.2.3.4", 300);
        // Insert with 0 TTL — immediately expired
        cache.insert("example.com", DnsRecordType::A, records, 0);

        assert!(cache.get("example.com", DnsRecordType::A).is_none());
    }

    #[test]
    fn test_cache_different_types() {
        let cache = DnsCache::new();
        cache.insert("example.com", DnsRecordType::A, make_a("1.2.3.4", 300), 300);
        cache.insert(
            "example.com",
            DnsRecordType::MX,
            make_a("5.6.7.8", 600),
            600,
        );

        assert!(cache.get("example.com", DnsRecordType::A).is_some());
        assert!(cache.get("example.com", DnsRecordType::MX).is_some());
    }

    #[test]
    fn test_cache_contains() {
        let cache = DnsCache::new();
        cache.insert("example.com", DnsRecordType::A, make_a("1.2.3.4", 300), 300);

        assert!(cache.contains("example.com", DnsRecordType::A));
        assert!(!cache.contains("example.com", DnsRecordType::MX));
    }

    #[test]
    fn test_cache_remove() {
        let cache = DnsCache::new();
        cache.insert("example.com", DnsRecordType::A, make_a("1.2.3.4", 300), 300);
        cache.remove("example.com", DnsRecordType::A);
        assert!(cache.get("example.com", DnsRecordType::A).is_none());
    }

    #[test]
    fn test_cache_clear() {
        let cache = DnsCache::new();
        cache.insert("a.com", DnsRecordType::A, make_a("1.1.1.1", 300), 300);
        cache.insert("b.com", DnsRecordType::A, make_a("2.2.2.2", 300), 300);
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_prune() {
        let cache = DnsCache::new();
        cache.insert("expired.com", DnsRecordType::A, make_a("1.1.1.1", 0), 0);
        cache.insert("valid.com", DnsRecordType::A, make_a("2.2.2.2", 3600), 3600);

        let removed = cache.prune();
        assert_eq!(removed, 1);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_cache_max_capacity() {
        let cache = DnsCache::with_capacity(2);
        cache.insert("a.com", DnsRecordType::A, make_a("1.1.1.1", 300), 300);
        cache.insert("b.com", DnsRecordType::A, make_a("2.2.2.2", 300), 300);
        // Third insert should evict the oldest
        cache.insert("c.com", DnsRecordType::A, make_a("3.3.3.3", 300), 300);
        assert!(cache.len() <= 2);
    }

    #[test]
    fn test_cache_stats() {
        let cache = DnsCache::new();
        cache.insert("a.com", DnsRecordType::A, make_a("1.1.1.1", 0), 0);
        cache.insert("b.com", DnsRecordType::A, make_a("2.2.2.2", 300), 300);

        let stats = cache.stats();
        assert_eq!(stats.total, 2);
        assert_eq!(stats.expired, 1);
    }
}
