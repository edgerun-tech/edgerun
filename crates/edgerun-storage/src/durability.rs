// SPDX-License-Identifier: GPL-2.0-only
/// Durability levels define how data is persisted before acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityLevel {
    /// Appended to memory buffer, not persisted.
    ///
    /// Guarantees:
    /// - Fastest write path (no fsync)
    /// - Visible to readers immediately
    /// - Lost on process crash
    ///
    /// Use cases: Telemetry, high-volume logging, ephemeral data
    AckLocal,

    /// Written to OS page cache, fdatasync scheduled.
    ///
    /// Guarantees:
    /// - Data in kernel buffers
    /// - May be lost on power failure (kernel may not flush)
    /// - Survives process crash
    ///
    /// Use cases: Standard throughput-sensitive writes
    AckBuffered,

    /// Fsync'd to storage device.
    ///
    /// Guarantees:
    /// - Data on physical media
    /// - Survives power loss
    /// - May lose last fsync_interval on crash
    ///
    /// Use cases: Standard durability (default)
    AckDurable,

    /// Fsync'd and manifest epoch flipped.
    ///
    /// Guarantees:
    /// - Full recovery possible
    /// - State machine checkpointed
    /// - Highest durability
    ///
    /// Use cases: Critical transactions, consensus
    AckCheckpointed,

    /// Durable on N peers (distributed).
    ///
    /// Guarantees:
    /// - Geographic redundancy
    /// - Byzantine fault tolerance (with proper N)
    /// - Latency depends on network RTT
    ///
    /// Use cases: Distributed consensus, multi-region
    AckReplicatedN(u8),
}

impl Default for DurabilityLevel {
    fn default() -> Self {
        DurabilityLevel::AckDurable
    }
}

/// Read consistency levels define visibility guarantees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadConsistency {
    /// Read the absolute latest data.
    ///
    /// Guarantees:
    /// - May see uncommitted writes
    /// - May see writes that will be rolled back
    /// - Best read performance
    ///
    /// Use cases: Monitoring, best-effort queries
    Latest,

    /// Read from a specific checkpoint epoch.
    ///
    /// Guarantees:
    /// - Consistent snapshot at epoch
    /// - No phantom reads
    /// - Bounded staleness (last checkpoint)
    ///
    /// Use cases: Analytics, backups, time-travel
    Stable(CheckpointEpoch),

    /// Read at the session's causal frontier.
    ///
    /// Guarantees:
    /// - Monotonic reads (never go backwards)
    /// - Read-your-writes
    /// - Bounded by session writes
    ///
    /// Use cases: User sessions, transactional reads
    Causal(SessionId),

    /// Strong consistency (linearizable).
    ///
    /// Guarantees:
    /// - All prior writes visible
    /// - No stale reads
    /// - Highest latency
    ///
    /// Use cases: Consensus, critical reads
    Strong,
}

impl Default for ReadConsistency {
    fn default() -> Self {
        ReadConsistency::Latest
    }
}

/// Checkpoint epoch identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CheckpointEpoch(pub u64);

impl CheckpointEpoch {
    pub fn new(epoch: u64) -> Self {
        Self(epoch)
    }

    pub fn increment(&mut self) {
        self.0 += 1;
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// Session identifier for causal consistency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(pub [u8; 16]);

impl SessionId {
    pub fn new() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let mut id = [0u8; 16];
        rng.fill(&mut id);
        Self(id)
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Synchronization policy for the storage engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyncPolicy {
    /// Fsync interval in bytes (0 = per event)
    pub fsync_interval_bytes: u64,

    /// Maximum events between fsyncs
    pub fsync_interval_events: usize,

    /// Timeout for batching fsyncs
    pub fsync_timeout_ms: u64,

    /// Use fdatasync instead of fsync (faster, skips metadata)
    pub use_fdatasync: bool,

    /// Async centralized io_uring reactor
    pub use_io_uring: bool,
}

impl Default for SyncPolicy {
    fn default() -> Self {
        Self {
            fsync_interval_bytes: 1024 * 1024, // 1MB
            fsync_interval_events: 1000,
            fsync_timeout_ms: 10,
            use_fdatasync: true,
            use_io_uring: false,
        }
    }
}

impl SyncPolicy {
    /// Conservative policy: fsync every event
    pub fn conservative() -> Self {
        Self {
            fsync_interval_bytes: 0,
            fsync_interval_events: 1,
            fsync_timeout_ms: 0,
            use_fdatasync: true,
            use_io_uring: false,
        }
    }

    /// Aggressive throughput policy: fsync per segment
    pub fn throughput() -> Self {
        Self {
            fsync_interval_bytes: 128 * 1024 * 1024, // 128MB
            fsync_interval_events: usize::MAX,
            fsync_timeout_ms: 1000,
            use_fdatasync: true,
            use_io_uring: true,
        }
    }
}

/// Durability configuration for an append operation.
#[derive(Debug, Clone)]
pub struct DurabilityConfig {
    pub level: DurabilityLevel,
    pub sync_policy: SyncPolicy,
    pub timeout: Option<Duration>,
}

impl Default for DurabilityConfig {
    fn default() -> Self {
        Self {
            level: DurabilityLevel::default(),
            sync_policy: SyncPolicy::default(),
            timeout: None,
        }
    }
}

use std::time::Duration;
