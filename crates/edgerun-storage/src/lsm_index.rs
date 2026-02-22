// SPDX-License-Identifier: Apache-2.0
//! LSM-backed index for scalable, persistent indexing.
//!
//! This module implements a Log-Structured Merge Tree index that provides:
//! - Bounded memory usage regardless of dataset size
//! - Persistent index (survives restarts)
//! - Predictable read performance via bloom filters
//! - Efficient range queries via sorted levels

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use std::thread;

use crate::io_reactor::IoReactor;

const COMPACTION_MIN_FILES: usize = 2;
const COMPACTION_READAHEAD: usize = 8;
const COMPACTION_READ_CHUNK_BYTES: usize = (1024 * 1024 / SST_ENTRY_BYTES) * SST_ENTRY_BYTES;
const EXTENDED_SST_MARKER: u64 = u64::MAX;
const EXTENDED_SST_MARKER_V2: u64 = u64::MAX - 1;
const SST_BASE_HEADER_BYTES: usize = 24;
const SST_EXTENDED_HEADER_BYTES: usize = 16;
const SST_EXTENDED_V2_HEADER_BYTES: usize = 24;
const SST_ENTRY_BYTES: usize = 80;
const SST_BLOCK_INDEX_ENTRY_BYTES: usize = 44;
type SStableEntries = (Arc<SSTable>, Vec<(Vec<u8>, IndexEntry)>);

/// Configuration for the LSM index.
#[derive(Debug, Clone)]
pub struct LsmConfig {
    /// Size threshold for memtable flush (bytes)
    pub memtable_size_threshold: usize,

    /// Maximum number of immutable memtables
    pub max_imm_memtables: usize,

    /// Bloom filter bits per key
    pub bloom_bits_per_key: usize,

    /// Block size for SSTables (bytes)
    pub block_size: usize,

    /// Base level size target (bytes)
    pub base_level_size: u64,

    /// Level size multiplier
    pub level_size_multiplier: usize,

    /// Maximum number of levels
    pub max_levels: usize,
}

impl Default for LsmConfig {
    fn default() -> Self {
        Self {
            memtable_size_threshold: 4 * 1024 * 1024, // 4MB
            max_imm_memtables: 2,
            bloom_bits_per_key: 10,
            block_size: 4 * 1024,              // 4KB
            base_level_size: 64 * 1024 * 1024, // 64MB
            level_size_multiplier: 10,
            max_levels: 6,
        }
    }
}

/// LSM Tree index implementation.
pub struct LsmIndex {
    config: LsmConfig,
    data_dir: PathBuf,

    /// Current mutable memtable
    memtable: Arc<RwLock<MemTable>>,

    /// Immutable memtables waiting to flush
    imm_memtables: Arc<RwLock<Vec<Arc<MemTable>>>>,

    /// SSTable levels
    levels: Arc<RwLock<Vec<Level>>>,

    /// Next SSTable ID
    next_sst_id: Arc<RwLock<u64>>,

    /// Centralized async I/O path for compaction.
    io_reactor: Option<Arc<IoReactor>>,

    /// Prevent overlapping compactions.
    compaction_running: Arc<AtomicBool>,

    /// Compaction telemetry counters and timings.
    compaction_metrics: Arc<CompactionMetrics>,
}

#[derive(Debug, Clone, Default)]
pub struct CompactionStats {
    pub scheduled: u64,
    pub running: bool,
    pub completed: u64,
    pub failed: u64,
    pub skipped: u64,
    pub total_duration_ms: u64,
    pub last_duration_ms: u64,
}

#[derive(Default)]
struct CompactionMetrics {
    scheduled: AtomicU64,
    completed: AtomicU64,
    failed: AtomicU64,
    skipped: AtomicU64,
    total_duration_ms: AtomicU64,
    last_duration_ms: AtomicU64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CompactionOutcome {
    Completed,
    Skipped,
    Failed,
}

/// In-memory sorted map (memtable).
pub struct MemTable {
    id: u64,
    data: BTreeMap<Vec<u8>, IndexEntry>,
    approximate_size: usize,
}

/// Entry in the index.
#[derive(Debug, Clone, Copy)]
pub struct IndexEntry {
    pub segment_id: [u8; 32],
    pub offset: u64,
    pub timestamp: u64,
}

/// A level of SSTables.
#[allow(dead_code)]
pub struct Level {
    level_num: usize,
    sstables: Vec<Arc<SSTable>>,
    sstables_by_first: Vec<Arc<SSTable>>,
    max_last_prefix: Vec<Vec<u8>>,
    total_size: u64,
    non_overlapping: bool,
}

/// Sorted String Table (SSTable).
#[allow(dead_code)]
pub struct SSTable {
    id: u64,
    path: PathBuf,

    /// Bloom filter for fast negative lookups
    bloom_filter: BloomFilter,

    /// Block index for efficient seeking
    block_index: Vec<BlockHandle>,

    /// First and last keys in this SST
    first_key: Vec<u8>,
    last_key: Vec<u8>,

    /// Total size
    size: u64,

    /// Number of entries
    entry_count: u64,

    /// Byte offset where entry records begin.
    entries_offset: u64,

    /// Sorted entries for block-local binary search lookups.
    entries: Vec<([u8; 32], IndexEntry)>,
}

/// Handle to a block within an SST.
#[derive(Debug, Clone)]
pub struct BlockHandle {
    pub offset: u64,
    pub size: u32,
    pub first_key: Vec<u8>,
}

/// Bloom filter for efficient negative lookups.
pub struct BloomFilter {
    bits: Vec<u64>,
    num_hashes: usize,
}

impl LsmIndex {
    pub fn new(data_dir: PathBuf, config: LsmConfig) -> std::io::Result<Self> {
        std::fs::create_dir_all(&data_dir)?;

        // Load existing levels or create empty
        let levels = Self::load_levels(&data_dir, &config)?;

        Ok(Self {
            config,
            data_dir,
            memtable: Arc::new(RwLock::new(MemTable::new(0))),
            imm_memtables: Arc::new(RwLock::new(Vec::new())),
            levels: Arc::new(RwLock::new(levels)),
            next_sst_id: Arc::new(RwLock::new(1)),
            io_reactor: IoReactor::global().ok(),
            compaction_running: Arc::new(AtomicBool::new(false)),
            compaction_metrics: Arc::new(CompactionMetrics::default()),
        })
    }

    /// Insert a key-value pair into the index.
    pub fn insert(&self, key: [u8; 32], segment_id: [u8; 32], offset: u64) {
        let entry = IndexEntry {
            segment_id,
            offset,
            timestamp: Self::current_timestamp(),
        };

        let key_vec = key.to_vec();

        {
            let mut memtable = self.memtable.write().unwrap();
            memtable.insert(key_vec.clone(), entry);

            // Check if memtable needs to be frozen
            if memtable.approximate_size >= self.config.memtable_size_threshold {
                drop(memtable);
                self.freeze_memtable();
            }
        }
    }

    /// Lookup a key in the index.
    pub fn get(&self, key: &[u8; 32]) -> Option<IndexEntry> {
        // 1. Check mutable memtable
        {
            let memtable = self.memtable.read().unwrap();
            if let Some(entry) = memtable.get(key) {
                return Some(*entry);
            }
        }

        // 2. Check immutable memtables (newest first)
        {
            let imm_memtables = self.imm_memtables.read().unwrap();
            for memtable in imm_memtables.iter().rev() {
                if let Some(entry) = memtable.get(key) {
                    return Some(*entry);
                }
            }
        }

        // 3. Check SSTable levels (L0 to Ln)
        {
            let levels = self.levels.read().unwrap();
            for level in levels.iter() {
                if let Some(entry) = level.get(key) {
                    return Some(entry);
                }
            }
        }

        None
    }

    /// Freeze the current memtable and create a new one.
    fn freeze_memtable(&self) {
        let mut memtable = self.memtable.write().unwrap();
        let mut imm_memtables = self.imm_memtables.write().unwrap();

        // If we have too many immutable memtables, force a flush
        if imm_memtables.len() >= self.config.max_imm_memtables {
            drop(imm_memtables);
            drop(memtable);
            self.flush_oldest_imm_memtable();
            return;
        }

        // Create new memtable with incremented ID
        let old_id = memtable.id;
        let new_memtable = MemTable::new(old_id + 1);
        let frozen_memtable = std::mem::replace(&mut *memtable, new_memtable);

        imm_memtables.push(Arc::new(frozen_memtable));
    }

    /// Flush the oldest immutable memtable to disk.
    fn flush_oldest_imm_memtable(&self) {
        let mut imm_memtables = self.imm_memtables.write().unwrap();

        if let Some(memtable) = imm_memtables.pop() {
            drop(imm_memtables);

            // Flush to L0
            if let Err(e) = self.flush_memtable_to_level(&memtable, 0) {
                eprintln!("Failed to flush memtable: {e}");
            }

            // Check if compaction is needed
            self.maybe_schedule_compaction();
        }
    }

    /// Flush a memtable to a specific level.
    fn flush_memtable_to_level(&self, memtable: &MemTable, level: usize) -> std::io::Result<()> {
        let sst_id = {
            let mut next_id = self.next_sst_id.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };

        let sst_path = self.data_dir.join(format!("{sst_id:06}.sst"));

        // Build SSTable from memtable
        let sstable = SSTable::build_from_memtable(
            sst_id,
            sst_path,
            memtable,
            self.config.bloom_bits_per_key,
            self.config.block_size,
            self.io_reactor.as_ref(),
        )?;

        // Add to level
        let mut levels = self.levels.write().unwrap();
        if level >= levels.len() {
            levels.resize_with(level + 1, || Level::new(level));
        }
        levels[level].add_sstable(Arc::new(sstable));

        Ok(())
    }

    /// Check if compaction is needed and schedule it.
    fn maybe_schedule_compaction(&self) {
        // This would typically spawn a background thread
        let config = self.config.clone();

        let mut candidate_level = None;
        let levels = self.levels.read().unwrap();
        for (level_num, level) in levels.iter().enumerate() {
            let target_size = if level_num == 0 {
                // L0 can have multiple files, but limit the total
                config.base_level_size / 4
            } else {
                config.base_level_size * (config.level_size_multiplier as u64).pow(level_num as u32)
            };

            if level.total_size > target_size {
                candidate_level = Some(level_num);
                break;
            }
        }

        let Some(level_num) = candidate_level else {
            return;
        };

        if self
            .compaction_running
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        self.compaction_metrics
            .scheduled
            .fetch_add(1, Ordering::Relaxed);

        let data_dir = self.data_dir.clone();
        let levels = Arc::clone(&self.levels);
        let next_sst_id = Arc::clone(&self.next_sst_id);
        let io_reactor = self.io_reactor.clone();
        let cfg = self.config.clone();
        let running = Arc::clone(&self.compaction_running);
        let metrics = Arc::clone(&self.compaction_metrics);

        thread::spawn(move || {
            let started = std::time::Instant::now();
            let outcome = Self::compact_level_inner(
                level_num,
                &data_dir,
                &levels,
                &next_sst_id,
                &cfg,
                io_reactor.as_ref(),
            );
            let elapsed_ms = started.elapsed().as_millis() as u64;
            metrics
                .last_duration_ms
                .store(elapsed_ms, Ordering::Relaxed);
            metrics
                .total_duration_ms
                .fetch_add(elapsed_ms, Ordering::Relaxed);
            match outcome {
                CompactionOutcome::Completed => {
                    metrics.completed.fetch_add(1, Ordering::Relaxed);
                }
                CompactionOutcome::Skipped => {
                    metrics.skipped.fetch_add(1, Ordering::Relaxed);
                }
                CompactionOutcome::Failed => {
                    metrics.failed.fetch_add(1, Ordering::Relaxed);
                }
            }
            running.store(false, Ordering::Release);
        });
    }

    /// Compact a level into the next level.
    #[cfg(test)]
    fn compact_level(&self, level_num: usize) {
        let _ = Self::compact_level_inner(
            level_num,
            &self.data_dir,
            &self.levels,
            &self.next_sst_id,
            &self.config,
            self.io_reactor.as_ref(),
        );
    }

    fn compact_level_inner(
        level_num: usize,
        data_dir: &Path,
        levels_ref: &Arc<RwLock<Vec<Level>>>,
        next_sst_id_ref: &Arc<RwLock<u64>>,
        config: &LsmConfig,
        io_reactor: Option<&Arc<IoReactor>>,
    ) -> CompactionOutcome {
        let to_compact: Vec<Arc<SSTable>> = {
            let levels = levels_ref.read().unwrap();
            if level_num >= levels.len() || levels[level_num].sstables.len() < COMPACTION_MIN_FILES
            {
                return CompactionOutcome::Skipped;
            }
            levels[level_num].sstables.clone()
        };

        let read_inputs = match Self::read_sstable_entries_pipelined(io_reactor, &to_compact) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Compaction read failed at level {level_num}: {e}");
                return CompactionOutcome::Failed;
            }
        };

        let mut merged: BTreeMap<Vec<u8>, IndexEntry> = BTreeMap::new();
        for (_, entries) in read_inputs {
            for (key, entry) in entries {
                match merged.get(&key) {
                    Some(existing) if existing.timestamp > entry.timestamp => {}
                    _ => {
                        merged.insert(key, entry);
                    }
                }
            }
        }

        let merged_entries: Vec<(Vec<u8>, IndexEntry)> = merged.into_iter().collect();
        if merged_entries.is_empty() {
            return CompactionOutcome::Skipped;
        }

        let new_sst_id = {
            let mut next_id = next_sst_id_ref.write().unwrap();
            let id = *next_id;
            *next_id += 1;
            id
        };
        let target_level = level_num + 1;
        let new_path = data_dir.join(format!("{new_sst_id:06}.sst"));
        let new_sst = match SSTable::build_from_entries(
            new_sst_id,
            new_path,
            merged_entries,
            config.bloom_bits_per_key,
            config.block_size,
            io_reactor,
        ) {
            Ok(sst) => Arc::new(sst),
            Err(e) => {
                eprintln!("Compaction write failed at level {level_num}: {e}");
                return CompactionOutcome::Failed;
            }
        };

        let compact_ids: std::collections::HashSet<u64> = to_compact.iter().map(|s| s.id).collect();
        let compact_paths: Vec<PathBuf> = to_compact.iter().map(|s| s.path.clone()).collect();

        {
            let mut levels = levels_ref.write().unwrap();
            if target_level >= levels.len() {
                levels.resize_with(target_level + 1, || Level::new(target_level));
            }

            let lvl = &mut levels[level_num];
            lvl.sstables.retain(|s| !compact_ids.contains(&s.id));
            lvl.refresh_layout();
            levels[target_level].add_sstable(new_sst);
        }

        for path in compact_paths {
            let _ = std::fs::remove_file(path);
        }
        CompactionOutcome::Completed
    }

    /// Snapshot compaction telemetry for observability/tuning.
    pub fn compaction_stats(&self) -> CompactionStats {
        CompactionStats {
            scheduled: self.compaction_metrics.scheduled.load(Ordering::Relaxed),
            running: self.compaction_running.load(Ordering::Relaxed),
            completed: self.compaction_metrics.completed.load(Ordering::Relaxed),
            failed: self.compaction_metrics.failed.load(Ordering::Relaxed),
            skipped: self.compaction_metrics.skipped.load(Ordering::Relaxed),
            total_duration_ms: self
                .compaction_metrics
                .total_duration_ms
                .load(Ordering::Relaxed),
            last_duration_ms: self
                .compaction_metrics
                .last_duration_ms
                .load(Ordering::Relaxed),
        }
    }

    fn read_sstable_entries_pipelined(
        io_reactor: Option<&Arc<IoReactor>>,
        sstables: &[Arc<SSTable>],
    ) -> std::io::Result<Vec<SStableEntries>> {
        if let Some(reactor) = io_reactor {
            let mut out = Vec::with_capacity(sstables.len());
            for sst in sstables {
                if sst.entry_count == 0 {
                    out.push((Arc::clone(sst), Vec::new()));
                    continue;
                }

                let handle = reactor.open_file(&sst.path, false, true, false, false)?;
                let total_bytes = (sst.entry_count as usize).saturating_mul(SST_ENTRY_BYTES);
                let mut next_read = 0usize;
                let mut inflight = std::collections::VecDeque::new();
                let mut entries = Vec::with_capacity(sst.entry_count as usize);

                while next_read < total_bytes || !inflight.is_empty() {
                    while next_read < total_bytes && inflight.len() < COMPACTION_READAHEAD {
                        let len = COMPACTION_READ_CHUNK_BYTES.min(total_bytes - next_read);
                        let ticket =
                            reactor.read(handle, sst.entries_offset + next_read as u64, len);
                        inflight.push_back((ticket, len));
                        next_read += len;
                    }

                    let Some((ticket, expected_len)) = inflight.pop_front() else {
                        break;
                    };
                    let chunk = ticket.wait()?;
                    if chunk.len() != expected_len {
                        reactor.close(handle);
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "short read during compaction",
                        ));
                    }
                    if chunk.len() % SST_ENTRY_BYTES != 0 {
                        reactor.close(handle);
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::InvalidData,
                            "misaligned compaction chunk",
                        ));
                    }

                    for entry in chunk.chunks_exact(SST_ENTRY_BYTES) {
                        let key = entry[0..32].to_vec();
                        let segment_id: [u8; 32] = entry[32..64].try_into().unwrap();
                        let offset = u64::from_le_bytes([
                            entry[64], entry[65], entry[66], entry[67], entry[68], entry[69],
                            entry[70], entry[71],
                        ]);
                        let timestamp = u64::from_le_bytes([
                            entry[72], entry[73], entry[74], entry[75], entry[76], entry[77],
                            entry[78], entry[79],
                        ]);
                        entries.push((
                            key,
                            IndexEntry {
                                segment_id,
                                offset,
                                timestamp,
                            },
                        ));
                    }
                }

                reactor.close(handle);
                out.push((Arc::clone(sst), entries));
            }
            return Ok(out);
        }

        let mut out = Vec::with_capacity(sstables.len());
        for sst in sstables {
            if sst.entry_count == 0 {
                out.push((Arc::clone(sst), Vec::new()));
                continue;
            }

            use std::io::{Read, Seek, SeekFrom};
            let mut file = std::fs::File::open(&sst.path)?;
            file.seek(SeekFrom::Start(sst.entries_offset))?;
            let mut remaining = (sst.entry_count as usize).saturating_mul(SST_ENTRY_BYTES);
            let mut entries = Vec::with_capacity(sst.entry_count as usize);
            let mut chunk = vec![0u8; COMPACTION_READ_CHUNK_BYTES.max(SST_ENTRY_BYTES)];

            while remaining > 0 {
                let read_len = chunk.len().min(remaining);
                file.read_exact(&mut chunk[..read_len])?;
                for entry in chunk[..read_len].chunks_exact(SST_ENTRY_BYTES) {
                    let key = entry[0..32].to_vec();
                    let segment_id: [u8; 32] = entry[32..64].try_into().unwrap();
                    let offset = u64::from_le_bytes([
                        entry[64], entry[65], entry[66], entry[67], entry[68], entry[69],
                        entry[70], entry[71],
                    ]);
                    let timestamp = u64::from_le_bytes([
                        entry[72], entry[73], entry[74], entry[75], entry[76], entry[77],
                        entry[78], entry[79],
                    ]);
                    entries.push((
                        key,
                        IndexEntry {
                            segment_id,
                            offset,
                            timestamp,
                        },
                    ));
                }
                remaining -= read_len;
            }
            out.push((Arc::clone(sst), entries));
        }
        Ok(out)
    }

    #[cfg(test)]
    fn parse_sstable_entries(
        sst: &SSTable,
        bytes: &[u8],
    ) -> std::io::Result<Vec<(Vec<u8>, IndexEntry)>> {
        if bytes.len() < 24 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "sstable too small",
            ));
        }

        let first_key_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
        let last_key_len =
            u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
        let header_end = 24 + first_key_len + last_key_len;
        if bytes.len() < header_end {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid sstable header",
            ));
        }

        if sst.entry_count == 0 {
            return Ok(Vec::new());
        }

        const ENTRY_SIZE: usize = 80;
        let entries_bytes = sst.entry_count as usize * ENTRY_SIZE;
        if bytes.len() < entries_bytes || bytes.len() - entries_bytes < header_end {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid sstable entry layout",
            ));
        }
        let entries_start = bytes.len() - entries_bytes;
        let mut entries = Vec::with_capacity(sst.entry_count as usize);

        for chunk in bytes[entries_start..].chunks_exact(ENTRY_SIZE) {
            let key = chunk[0..32].to_vec();
            let segment_id: [u8; 32] = chunk[32..64].try_into().unwrap();
            let offset = u64::from_le_bytes([
                chunk[64], chunk[65], chunk[66], chunk[67], chunk[68], chunk[69], chunk[70],
                chunk[71],
            ]);
            let timestamp = u64::from_le_bytes([
                chunk[72], chunk[73], chunk[74], chunk[75], chunk[76], chunk[77], chunk[78],
                chunk[79],
            ]);

            entries.push((
                key,
                IndexEntry {
                    segment_id,
                    offset,
                    timestamp,
                },
            ));
        }

        Ok(entries)
    }

    /// Load existing levels from disk.
    fn load_levels(data_dir: &Path, config: &LsmConfig) -> std::io::Result<Vec<Level>> {
        let mut levels = Vec::with_capacity(config.max_levels);
        for i in 0..config.max_levels {
            levels.push(Level::new(i));
        }

        // Scan for existing SSTables in data_dir
        let sstable_dir = data_dir.join("sstables");
        if !sstable_dir.exists() {
            std::fs::create_dir_all(&sstable_dir)?;
            return Ok(levels);
        }

        if let Ok(entries) = std::fs::read_dir(&sstable_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "sst") {
                    // Try to load the SSTable
                    if let Ok(sst) = SSTable::open(path.clone()) {
                        let level = (sst.id as usize) % config.max_levels;
                        if let Some(lvl) = levels.get_mut(level) {
                            lvl.add_sstable(Arc::new(sst));
                        }
                    }
                }
            }
        }

        Ok(levels)
    }

    /// Get memory usage statistics.
    pub fn memory_usage(&self) -> MemoryStats {
        let memtable_size = {
            let memtable = self.memtable.read().unwrap();
            memtable.approximate_size
        };

        let imm_memtables_size = {
            let imm_memtables = self.imm_memtables.read().unwrap();
            imm_memtables.iter().map(|m| m.approximate_size).sum()
        };

        MemoryStats {
            memtable_bytes: memtable_size,
            imm_memtables_bytes: imm_memtables_size,
            total_bytes: memtable_size + imm_memtables_size,
        }
    }

    /// Get disk usage statistics.
    pub fn disk_usage(&self) -> DiskStats {
        let levels = self.levels.read().unwrap();
        let total_size: u64 = levels.iter().map(|l| l.total_size).sum();
        let sst_count: usize = levels.iter().map(|l| l.sstables.len()).sum();

        DiskStats {
            total_bytes: total_size,
            sstable_count: sst_count,
            level_count: levels.len(),
        }
    }

    fn current_timestamp() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64
    }
}

impl MemTable {
    fn new(id: u64) -> Self {
        Self {
            id,
            data: BTreeMap::new(),
            approximate_size: 0,
        }
    }

    fn insert(&mut self, key: Vec<u8>, value: IndexEntry) {
        let key_size = key.len();
        let value_size = std::mem::size_of::<IndexEntry>();
        self.approximate_size += key_size + value_size + 32; // overhead
        self.data.insert(key, value);
    }

    fn get(&self, key: &[u8]) -> Option<&IndexEntry> {
        self.data.get(key)
    }

    fn iter(&self) -> impl Iterator<Item = (&Vec<u8>, &IndexEntry)> {
        self.data.iter()
    }
}

impl Level {
    fn new(level_num: usize) -> Self {
        Self {
            level_num,
            sstables: Vec::new(),
            sstables_by_first: Vec::new(),
            max_last_prefix: Vec::new(),
            total_size: 0,
            non_overlapping: level_num > 0,
        }
    }

    fn add_sstable(&mut self, sst: Arc<SSTable>) {
        self.sstables.push(sst);
        self.refresh_layout();
    }

    fn refresh_layout(&mut self) {
        self.total_size = self.sstables.iter().map(|s| s.size).sum();

        if self.level_num == 0 {
            // L0 may contain overlapping ranges; probe newest-first on reads.
            self.sstables.sort_by_key(|sst| sst.id);
            self.non_overlapping = false;
        } else {
            // L1+ are expected to be non-overlapping and sorted by key range.
            self.sstables.sort_by(|a, b| a.first_key.cmp(&b.first_key));
            self.non_overlapping = self
                .sstables
                .windows(2)
                .all(|pair| pair[0].last_key.as_slice() < pair[1].first_key.as_slice());
        }

        // Secondary interval index for overlap pruning.
        self.sstables_by_first = self.sstables.clone();
        self.sstables_by_first
            .sort_by(|a, b| a.first_key.cmp(&b.first_key));

        self.max_last_prefix.clear();
        self.max_last_prefix.reserve(self.sstables_by_first.len());
        let mut running_max = Vec::new();
        for sst in &self.sstables_by_first {
            if running_max.as_slice() < sst.last_key.as_slice() {
                running_max = sst.last_key.clone();
            }
            self.max_last_prefix.push(running_max.clone());
        }
    }

    fn get(&self, key: &[u8]) -> Option<IndexEntry> {
        if self.level_num == 0 {
            // L0 can overlap; interval-prune candidates, then newest table wins.
            let upper = self
                .sstables_by_first
                .partition_point(|sst| sst.first_key.as_slice() <= key);
            if upper == 0 {
                return None;
            }

            let mut best: Option<(u64, IndexEntry)> = None;
            for idx in (0..upper).rev() {
                if self.max_last_prefix[idx].as_slice() < key {
                    break;
                }
                let sst = &self.sstables_by_first[idx];
                if key > sst.last_key.as_slice() {
                    continue;
                }
                if let Some(entry) = sst.get(key) {
                    if best.map(|(id, _)| sst.id > id).unwrap_or(true) {
                        best = Some((sst.id, entry));
                    }
                }
            }
            return best.map(|(_, entry)| entry);
        }

        if self.non_overlapping {
            // In non-overlapping levels, at most one SST can contain key.
            let upper = self
                .sstables_by_first
                .partition_point(|sst| sst.first_key.as_slice() <= key);
            if upper == 0 {
                return None;
            }
            let candidate = &self.sstables_by_first[upper - 1];
            if key >= candidate.first_key.as_slice() && key <= candidate.last_key.as_slice() {
                return candidate.get(key);
            }
            return None;
        }

        // Fallback for potentially overlapping L1+ files (legacy/recovery state):
        // prune with first_key and prefix max(last_key), then probe narrowed candidates.
        let upper = self
            .sstables_by_first
            .partition_point(|sst| sst.first_key.as_slice() <= key);
        if upper == 0 {
            return None;
        }
        for idx in (0..upper).rev() {
            if self.max_last_prefix[idx].as_slice() < key {
                break;
            }
            let sst = &self.sstables_by_first[idx];
            if key > sst.last_key.as_slice() {
                continue;
            }
            if let Some(entry) = sst.get(key) {
                return Some(entry);
            }
        }

        None
    }
}

impl SSTable {
    /// Open an existing SSTable from disk
    fn open(path: PathBuf) -> std::io::Result<Self> {
        use std::io::Read;

        let mut file = std::fs::File::open(&path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        // Read base header: id (8) + first_key_len (4) + last_key_len (4) + marker/num_blocks (8)
        let mut header_buf = [0u8; SST_BASE_HEADER_BYTES];
        file.read_exact(&mut header_buf)?;

        let id = u64::from_le_bytes([
            header_buf[0],
            header_buf[1],
            header_buf[2],
            header_buf[3],
            header_buf[4],
            header_buf[5],
            header_buf[6],
            header_buf[7],
        ]);
        let first_key_len =
            u32::from_le_bytes([header_buf[8], header_buf[9], header_buf[10], header_buf[11]]);
        let last_key_len = u32::from_le_bytes([
            header_buf[12],
            header_buf[13],
            header_buf[14],
            header_buf[15],
        ]);
        let num_blocks_or_marker = u64::from_le_bytes([
            header_buf[16],
            header_buf[17],
            header_buf[18],
            header_buf[19],
            header_buf[20],
            header_buf[21],
            header_buf[22],
            header_buf[23],
        ]);

        // Optional extended metadata for robust recovery across restarts.
        let (entry_count, bloom_words, block_count, extra_header) =
            if num_blocks_or_marker == EXTENDED_SST_MARKER {
                let mut ext = [0u8; SST_EXTENDED_HEADER_BYTES];
                file.read_exact(&mut ext)?;
                let entry_count = u64::from_le_bytes([
                    ext[0], ext[1], ext[2], ext[3], ext[4], ext[5], ext[6], ext[7],
                ]);
                let bloom_words = u64::from_le_bytes([
                    ext[8], ext[9], ext[10], ext[11], ext[12], ext[13], ext[14], ext[15],
                ]);
                (
                    entry_count,
                    bloom_words as usize,
                    0usize,
                    SST_EXTENDED_HEADER_BYTES,
                )
            } else if num_blocks_or_marker == EXTENDED_SST_MARKER_V2 {
                let mut ext = [0u8; SST_EXTENDED_V2_HEADER_BYTES];
                file.read_exact(&mut ext)?;
                let entry_count = u64::from_le_bytes([
                    ext[0], ext[1], ext[2], ext[3], ext[4], ext[5], ext[6], ext[7],
                ]);
                let bloom_words = u64::from_le_bytes([
                    ext[8], ext[9], ext[10], ext[11], ext[12], ext[13], ext[14], ext[15],
                ]);
                let block_count = u64::from_le_bytes([
                    ext[16], ext[17], ext[18], ext[19], ext[20], ext[21], ext[22], ext[23],
                ]);
                (
                    entry_count,
                    bloom_words as usize,
                    block_count as usize,
                    SST_EXTENDED_V2_HEADER_BYTES,
                )
            } else {
                (0, 0, 0, 0)
            };

        // Read keys
        let mut first_key = vec![0u8; first_key_len as usize];
        let mut last_key = vec![0u8; last_key_len as usize];
        file.read_exact(&mut first_key)?;
        file.read_exact(&mut last_key)?;

        let base_prefix = SST_BASE_HEADER_BYTES + extra_header + first_key.len() + last_key.len();
        let block_index_bytes = block_count.saturating_mul(SST_BLOCK_INDEX_ENTRY_BYTES);
        let (entry_count, bloom_words, block_count) = if entry_count > 0 || bloom_words > 0 {
            (entry_count, bloom_words, block_count)
        } else {
            let (legacy_entry_count, legacy_bloom_words) =
                Self::infer_legacy_layout(size as usize, base_prefix)?;
            (legacy_entry_count, legacy_bloom_words, 0)
        };

        let mut block_index = Vec::with_capacity(block_count.max(1));
        if block_count > 0 {
            for _ in 0..block_count {
                let mut block = [0u8; SST_BLOCK_INDEX_ENTRY_BYTES];
                file.read_exact(&mut block)?;
                let offset = u64::from_le_bytes([
                    block[0], block[1], block[2], block[3], block[4], block[5], block[6], block[7],
                ]);
                let size = u32::from_le_bytes([block[8], block[9], block[10], block[11]]);
                let first_key = block[12..44].to_vec();
                block_index.push(BlockHandle {
                    offset,
                    size,
                    first_key,
                });
            }
        }

        let bloom_bytes_len = bloom_words.saturating_mul(8);
        let mut bloom_bytes = vec![0u8; bloom_bytes_len];
        if bloom_bytes_len > 0 {
            file.read_exact(&mut bloom_bytes)?;
        }
        let bloom_filter = BloomFilter::from_bytes(&bloom_bytes);

        let entries_offset = (base_prefix + block_index_bytes + bloom_bytes_len) as u64;
        let entries = Self::load_entries(&path, entries_offset, entry_count)?;
        if block_index.is_empty() && !entries.is_empty() {
            block_index.push(BlockHandle {
                offset: entries_offset,
                size: (entries.len() * SST_ENTRY_BYTES) as u32,
                first_key: entries[0].0.to_vec(),
            });
        }

        Ok(Self {
            id,
            path,
            bloom_filter,
            block_index,
            first_key,
            last_key,
            size,
            entry_count,
            entries_offset,
            entries,
        })
    }

    fn build_from_memtable(
        id: u64,
        path: PathBuf,
        memtable: &MemTable,
        bloom_bits_per_key: usize,
        _block_size: usize,
        io_reactor: Option<&Arc<IoReactor>>,
    ) -> std::io::Result<Self> {
        let mut entries: Vec<(Vec<u8>, IndexEntry)> = Vec::with_capacity(memtable.data.len());
        for (key, value) in memtable.iter() {
            entries.push((key.clone(), *value));
        }

        Self::build_from_entries(
            id,
            path,
            entries,
            bloom_bits_per_key,
            _block_size,
            io_reactor,
        )
    }

    fn build_from_entries(
        id: u64,
        path: PathBuf,
        mut entries: Vec<(Vec<u8>, IndexEntry)>,
        bloom_bits_per_key: usize,
        block_size: usize,
        io_reactor: Option<&Arc<IoReactor>>,
    ) -> std::io::Result<Self> {
        // Build bloom filter
        let num_keys = entries.len();
        let mut bloom_filter = BloomFilter::new(num_keys, bloom_bits_per_key);
        for (key, _) in &entries {
            bloom_filter.add(key);
        }

        // Sort entries by key
        entries.sort_by(|a, b| a.0.cmp(&b.0));

        let first_key = entries.first().map(|(k, _)| k.clone()).unwrap_or_default();
        let last_key = entries.last().map(|(k, _)| k.clone()).unwrap_or_default();
        let first_key_len = first_key.len() as u32;
        let last_key_len = last_key.len() as u32;
        let entries_per_block = (block_size.max(SST_ENTRY_BYTES) / SST_ENTRY_BYTES).max(1);
        let mut block_index = Vec::new();
        let entries_offset = (SST_BASE_HEADER_BYTES
            + SST_EXTENDED_V2_HEADER_BYTES
            + first_key.len()
            + last_key.len()
            + (entries.len().div_ceil(entries_per_block) * SST_BLOCK_INDEX_ENTRY_BYTES)
            + (bloom_filter.bits.len() * 8)) as u64;
        for (block_idx, chunk) in entries.chunks(entries_per_block).enumerate() {
            block_index.push(BlockHandle {
                offset: entries_offset + (block_idx * entries_per_block * SST_ENTRY_BYTES) as u64,
                size: (chunk.len() * SST_ENTRY_BYTES) as u32,
                first_key: chunk[0].0.clone(),
            });
        }
        let num_blocks_or_marker = EXTENDED_SST_MARKER_V2;

        let mut file_data = Vec::with_capacity(
            24 + first_key.len()
                + last_key.len()
                + (block_index.len() * SST_BLOCK_INDEX_ENTRY_BYTES)
                + (bloom_filter.bits.len() * 8)
                + (entries.len() * 80),
        );
        file_data.extend_from_slice(&id.to_le_bytes());
        file_data.extend_from_slice(&first_key_len.to_le_bytes());
        file_data.extend_from_slice(&last_key_len.to_le_bytes());
        file_data.extend_from_slice(&num_blocks_or_marker.to_le_bytes());
        file_data.extend_from_slice(&(num_keys as u64).to_le_bytes());
        file_data.extend_from_slice(&(bloom_filter.bits.len() as u64).to_le_bytes());
        file_data.extend_from_slice(&(block_index.len() as u64).to_le_bytes());
        file_data.extend_from_slice(&first_key);
        file_data.extend_from_slice(&last_key);
        for block in &block_index {
            file_data.extend_from_slice(&block.offset.to_le_bytes());
            file_data.extend_from_slice(&block.size.to_le_bytes());
            let mut first_key_buf = block.first_key.clone();
            first_key_buf.resize(32, 0);
            file_data.extend_from_slice(&first_key_buf);
        }
        for word in &bloom_filter.bits {
            file_data.extend_from_slice(&word.to_le_bytes());
        }
        for (key, entry) in &entries {
            let mut key_buf = key.clone();
            key_buf.resize(32, 0);
            file_data.extend_from_slice(&key_buf);
            file_data.extend_from_slice(&entry.segment_id);
            file_data.extend_from_slice(&entry.offset.to_le_bytes());
            file_data.extend_from_slice(&entry.timestamp.to_le_bytes());
        }

        let size = file_data.len() as u64;
        if let Some(reactor) = io_reactor {
            let file = reactor.open_file(&path, true, true, true, true)?;
            reactor.truncate(file, 0).wait()?;
            reactor.write(file, 0, file_data).wait()?;
            reactor.fsync(file, false).wait()?;
            reactor.close(file);
        } else {
            let mut file = std::fs::File::create(&path)?;
            file.write_all(&file_data)?;
        }

        Ok(Self {
            id,
            path,
            bloom_filter,
            block_index,
            first_key,
            last_key,
            size,
            entry_count: num_keys as u64,
            entries_offset,
            entries: entries
                .iter()
                .filter_map(|(k, v)| k.as_slice().try_into().ok().map(|key| (key, *v)))
                .collect(),
        })
    }

    fn get(&self, key: &[u8]) -> Option<IndexEntry> {
        // Check if key is in range
        if key < self.first_key.as_slice() || key > self.last_key.as_slice() {
            return None;
        }

        if self.entry_count == 0 || key.len() != 32 {
            return None;
        }

        if !self.bloom_filter.might_contain(key) {
            return None;
        }

        let key_arr: [u8; 32] = key.try_into().ok()?;
        let (start, end) = self.search_window(key)?;
        self.entries[start..end]
            .binary_search_by_key(&key_arr, |(k, _)| *k)
            .ok()
            .map(|idx| self.entries[start + idx].1)
    }

    fn load_entries(
        path: &PathBuf,
        entries_offset: u64,
        entry_count: u64,
    ) -> std::io::Result<Vec<([u8; 32], IndexEntry)>> {
        use std::io::{Read, Seek, SeekFrom};

        if entry_count == 0 {
            return Ok(Vec::new());
        }

        let mut file = std::fs::File::open(path)?;
        file.seek(SeekFrom::Start(entries_offset))?;

        let mut entries = Vec::with_capacity(entry_count as usize);
        let mut buf = [0u8; SST_ENTRY_BYTES];
        for _ in 0..entry_count {
            file.read_exact(&mut buf)?;
            let key: [u8; 32] = buf[0..32]
                .try_into()
                .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid key"))?;
            let segment_id: [u8; 32] = buf[32..64].try_into().map_err(|_| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid segment id")
            })?;
            let offset = u64::from_le_bytes([
                buf[64], buf[65], buf[66], buf[67], buf[68], buf[69], buf[70], buf[71],
            ]);
            let timestamp = u64::from_le_bytes([
                buf[72], buf[73], buf[74], buf[75], buf[76], buf[77], buf[78], buf[79],
            ]);

            entries.push((
                key,
                IndexEntry {
                    segment_id,
                    offset,
                    timestamp,
                },
            ));
        }

        Ok(entries)
    }

    fn search_window(&self, key: &[u8]) -> Option<(usize, usize)> {
        if self.block_index.is_empty() {
            return Some((0, self.entries.len()));
        }

        let upper = self
            .block_index
            .partition_point(|b| b.first_key.as_slice() <= key);
        let block_idx = upper.saturating_sub(1);
        let block = self.block_index.get(block_idx)?;
        let start = ((block.offset.saturating_sub(self.entries_offset)) as usize) / SST_ENTRY_BYTES;
        let len = (block.size as usize) / SST_ENTRY_BYTES;
        let end = (start + len).min(self.entries.len());
        Some((start, end))
    }

    fn infer_legacy_layout(
        total_size: usize,
        fixed_prefix: usize,
    ) -> std::io::Result<(u64, usize)> {
        if total_size < fixed_prefix {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "invalid sstable size",
            ));
        }
        let remaining = total_size - fixed_prefix;
        if remaining == 0 {
            return Ok((0, 0));
        }

        let mut best: Option<(f64, u64, usize)> = None; // (score, entry_count, bloom_words)
        let max_entries = remaining / SST_ENTRY_BYTES;

        for entries in 0..=max_entries {
            let entries_bytes = entries * SST_ENTRY_BYTES;
            let bloom_bytes = remaining.saturating_sub(entries_bytes);
            if !bloom_bytes.is_multiple_of(8) {
                continue;
            }

            let bloom_words = bloom_bytes / 8;
            if entries == 0 {
                best = best.or(Some((1000.0, 0, bloom_words)));
                continue;
            }

            if bloom_words == 0 {
                continue;
            }

            let bits_per_item = (bloom_words * 64) as f64 / entries as f64;
            if !(2.0..=32.0).contains(&bits_per_item) {
                continue;
            }
            let score = (bits_per_item - 10.0).abs();

            match best {
                Some((best_score, best_entries, _)) => {
                    if score < best_score || (score == best_score && entries as u64 > best_entries)
                    {
                        best = Some((score, entries as u64, bloom_words));
                    }
                }
                None => best = Some((score, entries as u64, bloom_words)),
            }
        }

        if let Some((_, entry_count, bloom_words)) = best {
            return Ok((entry_count, bloom_words));
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "could not infer legacy sstable layout",
        ))
    }
}

impl BloomFilter {
    fn new(num_items: usize, bits_per_item: usize) -> Self {
        let num_bits = num_items * bits_per_item;
        let num_words = num_bits.div_ceil(64);
        let num_hashes = Self::optimal_num_hashes(bits_per_item);

        Self {
            bits: vec![0u64; num_words],
            num_hashes,
        }
    }

    fn from_bytes(data: &[u8]) -> Self {
        // Reconstruct bloom filter from bytes
        let words: Vec<u64> = data
            .chunks_exact(8)
            .map(|chunk| {
                u64::from_le_bytes([
                    chunk[0], chunk[1], chunk[2], chunk[3], chunk[4], chunk[5], chunk[6], chunk[7],
                ])
            })
            .collect();

        Self {
            bits: words,
            num_hashes: 7, // Matches ~10 bits/key default configuration.
        }
    }

    fn optimal_num_hashes(bits_per_item: usize) -> usize {
        // k = ln(2) * (m/n)
        // where m/n is bits_per_item
        ((0.693f64) * (bits_per_item as f64)).ceil() as usize
    }

    fn add(&mut self, key: &[u8]) {
        if self.bits.is_empty() {
            return;
        }
        let hash = Self::hash_key(key);

        for i in 0..self.num_hashes {
            let bit = self.nth_hash(hash, i);
            let word = bit / 64;
            let bit_in_word = bit % 64;

            if word < self.bits.len() {
                self.bits[word] |= 1u64 << bit_in_word;
            }
        }
    }

    fn might_contain(&self, key: &[u8]) -> bool {
        if self.bits.is_empty() {
            return false;
        }
        let hash = Self::hash_key(key);

        for i in 0..self.num_hashes {
            let bit = self.nth_hash(hash, i);
            let word = bit / 64;
            let bit_in_word = bit % 64;

            if word >= self.bits.len() {
                return false;
            }

            if (self.bits[word] & (1u64 << bit_in_word)) == 0 {
                return false;
            }
        }

        true
    }

    fn hash_key(key: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    fn nth_hash(&self, base_hash: u64, n: usize) -> usize {
        // Double hashing technique
        let hash1 = base_hash;
        let hash2 = base_hash.wrapping_mul(0x9e3779b97f4a7c15);
        let combined = hash1.wrapping_add((n as u64).wrapping_mul(hash2));
        (combined as usize) % (self.bits.len() * 64)
    }
}

/// Memory usage statistics.
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub memtable_bytes: usize,
    pub imm_memtables_bytes: usize,
    pub total_bytes: usize,
}

/// Disk usage statistics.
#[derive(Debug, Clone)]
pub struct DiskStats {
    pub total_bytes: u64,
    pub sstable_count: usize,
    pub level_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_lsm_index_creation() {
        let temp_dir = TempDir::new().unwrap();
        let index = LsmIndex::new(temp_dir.path().to_path_buf(), LsmConfig::default()).unwrap();

        let mem_stats = index.memory_usage();
        assert_eq!(mem_stats.memtable_bytes, 0);

        let disk_stats = index.disk_usage();
        assert_eq!(disk_stats.total_bytes, 0);
    }

    #[test]
    fn test_memtable_insert_and_get() {
        let mut memtable = MemTable::new(0);

        let key = vec![1u8, 2, 3];
        let entry = IndexEntry {
            segment_id: [4u8; 32],
            offset: 100,
            timestamp: 12345,
        };

        memtable.insert(key.clone(), entry);

        let retrieved = memtable.get(&key).unwrap();
        assert_eq!(retrieved.offset, 100);
    }

    #[test]
    fn test_bloom_filter() {
        let mut bloom = BloomFilter::new(1000, 10);

        bloom.add(b"key1");
        bloom.add(b"key2");

        assert!(bloom.might_contain(b"key1"));
        assert!(bloom.might_contain(b"key2"));
        assert!(!bloom.might_contain(b"key3")); // Should usually be false
    }

    #[test]
    fn test_lsm_insert_and_lookup() {
        let temp_dir = TempDir::new().unwrap();
        let index = LsmIndex::new(temp_dir.path().to_path_buf(), LsmConfig::default()).unwrap();

        // Insert some keys
        for i in 0..100 {
            let key = [i as u8; 32];
            index.insert(key, [i as u8; 32], i as u64);
        }

        // Lookup
        let key = [50u8; 32];
        let entry = index.get(&key).unwrap();
        assert_eq!(entry.offset, 50);

        // Lookup missing key
        let missing_key = [255u8; 32];
        assert!(index.get(&missing_key).is_none());
    }

    #[test]
    fn test_compaction_merges_and_promotes_level() {
        let temp_dir = TempDir::new().unwrap();
        let index = LsmIndex::new(temp_dir.path().to_path_buf(), LsmConfig::default()).unwrap();

        let key_a = vec![1u8; 32];
        let key_b = vec![2u8; 32];

        let sst1_entries = vec![
            (
                key_a.clone(),
                IndexEntry {
                    segment_id: [10u8; 32],
                    offset: 10,
                    timestamp: 100,
                },
            ),
            (
                key_b.clone(),
                IndexEntry {
                    segment_id: [11u8; 32],
                    offset: 11,
                    timestamp: 101,
                },
            ),
        ];
        let sst2_entries = vec![(
            key_a.clone(),
            IndexEntry {
                segment_id: [20u8; 32],
                offset: 20,
                timestamp: 200,
            },
        )];

        let sst1_path = temp_dir.path().join("000001.sst");
        let sst2_path = temp_dir.path().join("000002.sst");
        let sst1 = Arc::new(
            SSTable::build_from_entries(1, sst1_path.clone(), sst1_entries, 10, 4096, None)
                .unwrap(),
        );
        let sst2 = Arc::new(
            SSTable::build_from_entries(2, sst2_path.clone(), sst2_entries, 10, 4096, None)
                .unwrap(),
        );

        {
            let mut levels = index.levels.write().unwrap();
            levels[0].add_sstable(sst1);
            levels[0].add_sstable(sst2);
        }
        *index.next_sst_id.write().unwrap() = 3;

        index.compact_level(0);

        let compacted_sst = {
            let levels = index.levels.read().unwrap();
            assert!(levels[0].sstables.is_empty());
            assert_eq!(levels[1].sstables.len(), 1);
            levels[1].sstables[0].clone()
        };

        let bytes = std::fs::read(&compacted_sst.path).unwrap();
        let entries = LsmIndex::parse_sstable_entries(&compacted_sst, &bytes).unwrap();
        let latest = entries
            .iter()
            .find(|(k, _)| k.as_slice() == [1u8; 32])
            .map(|(_, v)| v)
            .unwrap();
        assert_eq!(latest.offset, 20);
        assert_eq!(latest.timestamp, 200);
        assert!(!sst1_path.exists());
        assert!(!sst2_path.exists());
    }

    #[test]
    fn test_sstable_open_reconstructs_entry_count() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("open_reconstruct.sst");

        let entries = vec![
            (
                vec![7u8; 32],
                IndexEntry {
                    segment_id: [1u8; 32],
                    offset: 42,
                    timestamp: 1000,
                },
            ),
            (
                vec![8u8; 32],
                IndexEntry {
                    segment_id: [2u8; 32],
                    offset: 43,
                    timestamp: 1001,
                },
            ),
        ];

        let _built = SSTable::build_from_entries(7, path.clone(), entries, 10, 4096, None).unwrap();
        let opened = SSTable::open(path).unwrap();
        assert_eq!(opened.entry_count, 2);
        let got = opened.get(&[7u8; 32]).unwrap();
        assert_eq!(got.offset, 42);
        assert_eq!(got.timestamp, 1000);
    }

    #[test]
    fn test_level0_overlap_lookup_prefers_newest() {
        let temp_dir = TempDir::new().unwrap();
        let mut level = Level::new(0);

        let old = Arc::new(
            SSTable::build_from_entries(
                1,
                temp_dir.path().join("000001.sst"),
                vec![(
                    vec![9u8; 32],
                    IndexEntry {
                        segment_id: [1u8; 32],
                        offset: 100,
                        timestamp: 1000,
                    },
                )],
                10,
                4096,
                None,
            )
            .unwrap(),
        );
        let new = Arc::new(
            SSTable::build_from_entries(
                2,
                temp_dir.path().join("000002.sst"),
                vec![(
                    vec![9u8; 32],
                    IndexEntry {
                        segment_id: [2u8; 32],
                        offset: 200,
                        timestamp: 2000,
                    },
                )],
                10,
                4096,
                None,
            )
            .unwrap(),
        );

        level.add_sstable(old);
        level.add_sstable(new);

        let got = level.get(&[9u8; 32]).unwrap();
        assert_eq!(got.offset, 200);
        assert_eq!(got.timestamp, 2000);
    }
}
