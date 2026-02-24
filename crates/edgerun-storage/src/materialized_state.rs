// SPDX-License-Identifier: GPL-2.0-only
//! Materialized State Index (MSI) for hot-key reads.
//!
//! MSI provides:
//! - LRU cache for hot keys
//! - Per-stream snapshots for fast reads
//! - Bounded tail replay for consistency
//! - Adaptive snapshot cadence

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use crate::durability::CheckpointEpoch;
use crate::event::StreamId;

/// Configuration for the Materialized State Index.
#[derive(Debug, Clone)]
pub struct MsiConfig {
    /// Maximum number of cached keys (LRU)
    pub cache_capacity: usize,

    /// Snapshot every N events
    pub snapshot_event_interval: usize,

    /// Snapshot every T seconds
    pub snapshot_time_interval: Duration,

    /// Maximum tail length to replay
    pub max_tail_length: usize,

    /// Enable adaptive snapshotting
    pub adaptive_snapshots: bool,

    /// Target tail length for adaptive mode
    pub target_tail_length: usize,
}

impl Default for MsiConfig {
    fn default() -> Self {
        Self {
            cache_capacity: 100_000,
            snapshot_event_interval: 1000,
            snapshot_time_interval: Duration::from_secs(30),
            max_tail_length: 256,
            adaptive_snapshots: true,
            target_tail_length: 100,
        }
    }
}

/// Materialized State Index.
pub struct MaterializedStateIndex {
    config: MsiConfig,

    /// LRU cache of hot keys
    cache: Arc<RwLock<LruCache<Vec<u8>, CachedEntry>>>,

    /// Per-stream snapshots
    stream_snapshots: Arc<RwLock<HashMap<StreamId, StreamSnapshot>>>,

    /// Global snapshot epoch
    last_snapshot_epoch: Arc<RwLock<CheckpointEpoch>>,

    /// Metrics for adaptive tuning
    metrics: Arc<RwLock<MsiMetrics>>,
}

/// Cached entry in the MSI.
#[derive(Debug, Clone)]
pub struct CachedEntry {
    /// The value
    pub value: Vec<u8>,

    /// Epoch when this was cached
    pub cached_at: CheckpointEpoch,

    /// Stream ID for this entry
    pub stream_id: StreamId,

    /// Start of tail (events after this need replay)
    pub tail_start: u64,

    /// Number of events in tail
    pub tail_length: usize,

    /// Last access time for LRU
    pub last_access: Instant,
}

/// Snapshot of a stream's state.
#[derive(Debug, Clone)]
pub struct StreamSnapshot {
    pub stream_id: StreamId,
    pub epoch: CheckpointEpoch,
    pub event_count: u64,
    pub created_at: Instant,

    /// Key-value pairs at snapshot time
    pub state: HashMap<Vec<u8>, Vec<u8>>,
}

/// Simple LRU cache implementation.
pub struct LruCache<K, V> {
    capacity: usize,
    map: HashMap<K, V>,
    access_order: Vec<K>,
}

/// MSI performance metrics.
#[derive(Debug, Default, Clone)]
pub struct MsiMetrics {
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub snapshot_count: u64,
    pub tail_replay_count: u64,
    pub tail_replay_events: u64,
    pub avg_tail_length: f64,
}

impl MaterializedStateIndex {
    pub fn new(config: MsiConfig) -> Self {
        Self {
            config: config.clone(),
            cache: Arc::new(RwLock::new(LruCache::new(config.cache_capacity))),
            stream_snapshots: Arc::new(RwLock::new(HashMap::new())),
            last_snapshot_epoch: Arc::new(RwLock::new(CheckpointEpoch::new(0))),
            metrics: Arc::new(RwLock::new(MsiMetrics::default())),
        }
    }

    /// Read a key with potential tail replay.
    ///
    /// Returns the value if found, applying any tail events since the snapshot.
    pub fn read(&self, key: &[u8], stream_id: StreamId) -> Option<Vec<u8>> {
        let key_vec = key.to_vec();

        // 1. Check MSI cache
        {
            let mut cache = self.cache.write().unwrap();
            if let Some(entry) = cache.get_mut(&key_vec) {
                entry.last_access = Instant::now();

                // Update metrics
                let mut metrics = self.metrics.write().unwrap();
                metrics.cache_hits += 1;

                // If tail is small enough, return immediately
                if entry.tail_length <= self.config.max_tail_length {
                    return Some(entry.value.clone());
                }

                // Otherwise, we need to check if there's a newer snapshot
                let _ = entry;
                drop(cache);
                drop(metrics);
            } else {
                let mut metrics = self.metrics.write().unwrap();
                metrics.cache_misses += 1;
            }
        }

        // 2. Check stream snapshot
        let snapshot_result = {
            let snapshots = self.stream_snapshots.read().unwrap();
            snapshots.get(&stream_id).cloned()
        };

        if let Some(snapshot) = snapshot_result {
            if let Some(value) = snapshot.state.get(&key_vec) {
                // Calculate tail bounds
                let tail_start = snapshot.event_count;
                let tail_end = self.get_current_event_count(&stream_id);
                let tail_length = (tail_end - tail_start) as usize;

                // Replay tail if needed
                if tail_length > 0 && tail_length <= self.config.max_tail_length {
                    // Tail replay is intentionally not applied until event-level merge logic exists.
                    // Returning the snapshot value here would serve stale data.
                    let mut metrics = self.metrics.write().unwrap();
                    metrics.cache_misses += 1;
                    return None;
                } else if tail_length == 0 {
                    // No tail to replay, cache and return
                    self.cache_result(key_vec, stream_id, value.clone(), tail_start, 0);
                    return Some(value.clone());
                }
            }
        }

        // 3. Not in MSI, need to read from source
        None
    }

    /// Write a value to MSI (typically called by storage engine).
    pub fn write(&self, key: Vec<u8>, value: Vec<u8>, stream_id: StreamId, event_num: u64) {
        // Update cache if present
        {
            let mut cache = self.cache.write().unwrap();
            if let Some(entry) = cache.get_mut(&key) {
                entry.value = value.clone();
                entry.tail_start = event_num;
                entry.tail_length = 0;
                entry.last_access = Instant::now();
            }
        }

        // Check if we need to create a new snapshot
        self.maybe_create_snapshot(stream_id, event_num);
    }

    /// Create a snapshot for a stream.
    pub fn create_snapshot(
        &self,
        stream_id: StreamId,
        state: HashMap<Vec<u8>, Vec<u8>>,
        event_count: u64,
    ) {
        let epoch = {
            let mut last_epoch = self.last_snapshot_epoch.write().unwrap();
            last_epoch.increment();
            *last_epoch
        };

        let snapshot = StreamSnapshot {
            stream_id: stream_id.clone(),
            epoch,
            event_count,
            created_at: Instant::now(),
            state,
        };

        {
            let mut snapshots = self.stream_snapshots.write().unwrap();
            snapshots.insert(stream_id.clone(), snapshot);
        }

        {
            let mut metrics = self.metrics.write().unwrap();
            metrics.snapshot_count += 1;
        }

        // Evict old snapshots if needed
        self.evict_old_snapshots();
    }

    /// Get metrics.
    pub fn metrics(&self) -> MsiMetrics {
        self.metrics.read().unwrap().clone()
    }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> CacheStats {
        let cache = self.cache.read().unwrap();
        let metrics = self.metrics.read().unwrap();

        let total_requests = metrics.cache_hits + metrics.cache_misses;
        let hit_rate = if total_requests > 0 {
            metrics.cache_hits as f64 / total_requests as f64
        } else {
            0.0
        };

        CacheStats {
            size: cache.len(),
            capacity: self.config.cache_capacity,
            hit_rate,
            hits: metrics.cache_hits,
            misses: metrics.cache_misses,
        }
    }

    /// Cache a read result.
    fn cache_result(
        &self,
        key: Vec<u8>,
        stream_id: StreamId,
        value: Vec<u8>,
        tail_start: u64,
        tail_length: usize,
    ) {
        let entry = CachedEntry {
            value,
            cached_at: self.get_current_epoch(),
            stream_id,
            tail_start,
            tail_length,
            last_access: Instant::now(),
        };

        let mut cache = self.cache.write().unwrap();
        cache.put(key, entry);
    }

    /// Check if we should create a new snapshot.
    fn maybe_create_snapshot(&self, stream_id: StreamId, event_num: u64) {
        let should_snapshot = {
            let snapshots = self.stream_snapshots.read().unwrap();

            if let Some(snapshot) = snapshots.get(&stream_id) {
                let events_since = event_num - snapshot.event_count;
                let time_since = snapshot.created_at.elapsed();

                events_since >= self.config.snapshot_event_interval as u64
                    || time_since >= self.config.snapshot_time_interval
            } else {
                true // No snapshot exists
            }
        };

        if should_snapshot && self.config.adaptive_snapshots {
            // In adaptive mode, check if tail is getting too long
            let metrics = self.metrics.read().unwrap();
            let avg_tail = metrics.avg_tail_length;

            if avg_tail > self.config.target_tail_length as f64 * 1.5 {
                // Tail is too long, create snapshot more aggressively
                // Signal to storage engine to create snapshot
            }
        }
    }

    /// Evict old snapshots to limit memory.
    fn evict_old_snapshots(&self) {
        // Keep only recent snapshots per stream
        // In production, this would have more sophisticated eviction
    }

    /// Get current event count for a stream.
    fn get_current_event_count(&self, _stream_id: &StreamId) -> u64 {
        // This would query the storage engine
        // Placeholder
        0
    }

    /// Get current epoch.
    fn get_current_epoch(&self) -> CheckpointEpoch {
        *self.last_snapshot_epoch.read().unwrap()
    }
}

impl<K: std::hash::Hash + Eq + Clone, V> LruCache<K, V> {
    fn new(capacity: usize) -> Self {
        Self {
            capacity,
            map: HashMap::with_capacity(capacity),
            access_order: Vec::with_capacity(capacity),
        }
    }

    fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if self.map.contains_key(key) {
            // Move to end (most recently used)
            self.access_order.retain(|k| k != key);
            self.access_order.push(key.clone());
            self.map.get_mut(key)
        } else {
            None
        }
    }

    fn put(&mut self, key: K, value: V) {
        if self.map.contains_key(&key) {
            // Update existing
            self.map.insert(key.clone(), value);
            self.access_order.retain(|k| k != &key);
            self.access_order.push(key);
        } else {
            // Insert new
            if self.map.len() >= self.capacity {
                // Evict least recently used
                if let Some(lru_key) = self.access_order.first().cloned() {
                    self.map.remove(&lru_key);
                    self.access_order.remove(0);
                }
            }
            self.map.insert(key.clone(), value);
            self.access_order.push(key);
        }
    }

    fn len(&self) -> usize {
        self.map.len()
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub hit_rate: f64,
    pub hits: u64,
    pub misses: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_msi_creation() {
        let config = MsiConfig::default();
        let msi = MaterializedStateIndex::new(config);

        let metrics = msi.metrics();
        assert_eq!(metrics.cache_hits, 0);
    }

    #[test]
    fn test_lru_cache() {
        let mut cache = LruCache::new(2);

        cache.put("a", 1);
        cache.put("b", 2);
        assert_eq!(cache.len(), 2);

        // Access 'a', making 'b' LRU
        assert_eq!(cache.get_mut(&"a"), Some(&mut 1));

        // Insert 'c', should evict 'b'
        cache.put("c", 3);
        assert_eq!(cache.len(), 2);
        assert!(cache.get_mut(&"b").is_none());
        assert!(cache.get_mut(&"a").is_some());
        assert!(cache.get_mut(&"c").is_some());
    }

    #[test]
    fn test_stream_snapshot() {
        let mut state = HashMap::new();
        state.insert(b"key1".to_vec(), b"value1".to_vec());

        let snapshot = StreamSnapshot {
            stream_id: StreamId::new(),
            epoch: CheckpointEpoch::new(1),
            event_count: 100,
            created_at: Instant::now(),
            state,
        };

        assert_eq!(snapshot.event_count, 100);
        assert!(snapshot.state.contains_key(b"key1".as_slice()));
    }
}
