// SPDX-License-Identifier: GPL-2.0-only
pub mod arena;
pub mod async_segment_writer;
pub mod block;
pub mod context_engine;
pub mod crash_test;
pub mod crdt;
pub mod durability;
pub mod encryption;
pub mod event;
pub mod event_bus;
pub mod index;
pub mod io_reactor;
pub mod key_management;
pub mod lsm_index;
pub mod manifest;
pub mod materialized_state;
pub mod optimized_writer;
pub mod os;
pub mod replication;
pub mod seal_policy;
pub mod segment;
pub mod sharding;
pub mod timeline;
pub mod virtual_fs;

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Block error: {0}")]
    Block(#[from] block::BlockError),
    #[error("Event error: {0}")]
    Event(#[from] event::EventError),
    #[error("Segment error: {0}")]
    Segment(#[from] segment::SegmentError),
    #[error("Index error: {0}")]
    Index(#[from] index::IndexError),
    #[error("CRDT error: {0}")]
    Crdt(#[from] crdt::CrdtError),
    #[error("Replication error: {0}")]
    Replication(#[from] replication::ReplicationError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Key management error: {0}")]
    KeyManagement(#[from] key_management::KeyManagementError),
    #[error("Invalid epoch")]
    InvalidEpoch,
    #[error("Not found")]
    NotFound,
    #[error("Corrupted")]
    Corrupted,
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error("Invalid seal policy: {0}")]
    InvalidSealPolicy(String),
}

#[derive(Debug, Clone, Default)]
pub struct QueryCursor {
    pub offset: u64,
}

#[derive(Debug, Clone, Default)]
pub struct EventQueryFilter {
    pub stream_id: Option<event::StreamId>,
    pub actor_id: Option<event::ActorId>,
    pub min_hlc: Option<event::HlcTimestamp>,
    pub max_hlc: Option<event::HlcTimestamp>,
}

#[derive(Debug, Clone)]
pub struct EventQueryOptions {
    pub limit: usize,
    pub cursor: Option<QueryCursor>,
    pub filter: EventQueryFilter,
}

impl Default for EventQueryOptions {
    fn default() -> Self {
        Self {
            limit: 100,
            cursor: None,
            filter: EventQueryFilter::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct QueriedEvent {
    pub offset: u64,
    pub wire_len: u64,
    pub event_hash: [u8; 32],
    pub event: event::Event,
}

#[derive(Debug, Clone, Default)]
pub struct EventQueryResult {
    pub events: Vec<QueriedEvent>,
    pub next_cursor: Option<QueryCursor>,
}

#[derive(Debug, Clone)]
pub struct SealPendingResult {
    pub decision: seal_policy::SealDecision,
    pub sealed: bool,
    pub segment_id: Option<[u8; 32]>,
    pub progress_event_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SignedChainProgressEvent {
    pub progress_event_id: String,
    pub slot: u64,
    pub epoch: u64,
    pub observed_at_unix_ms: u64,
    pub signer: String,
    pub signature: String,
}

pub struct StorageEngine {
    data_dir: PathBuf,
}

impl StorageEngine {
    pub fn new(data_dir: PathBuf) -> Result<Self, StorageError> {
        std::fs::create_dir_all(&data_dir)?;
        Ok(Self { data_dir })
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    /// Create an engine-managed append session that wires checkpoint durability
    /// through the manifest manager and io_uring writer.
    pub fn create_append_session(
        &self,
        segment_file: &str,
        max_segment_size: u64,
    ) -> Result<EngineAppendSession, StorageError> {
        let factory = async_segment_writer::AsyncSegmentWriterFactory::new()?;
        let segment_path = self.data_dir.join(segment_file);
        let writer = factory.create_writer(segment_path, max_segment_size)?;
        let manifest_manager = manifest::ManifestManager::new(self.data_dir.clone())?;
        Ok(EngineAppendSession {
            writer,
            manifest_manager,
            replica_nodes: Vec::new(),
            replication_timeout: Duration::from_millis(250),
            replication_batch_size: 64,
            opened_at_unix_ms: now_unix_ms(),
            last_append_unix_ms: now_unix_ms(),
            opened_chain: None,
            latest_chain: None,
            latest_progress_event_id: None,
            auto_seal_controller: None,
        })
    }

    /// Query events from a segment through the storage engine abstraction.
    /// Returns at most `limit` events in segment order.
    pub fn query_events(
        &self,
        segment_file: &str,
        limit: usize,
    ) -> Result<Vec<event::Event>, StorageError> {
        let options = EventQueryOptions {
            limit,
            ..EventQueryOptions::default()
        };
        let result = self.query_events_raw(segment_file, options)?;
        Ok(result.events.into_iter().map(|entry| entry.event).collect())
    }

    /// Query raw event entries (offset + wire length + hash + decoded event)
    /// through the engine abstraction. This is the stable primitive to build
    /// higher-level query APIs without coupling callers to segment internals.
    pub fn query_events_raw(
        &self,
        segment_file: &str,
        options: EventQueryOptions,
    ) -> Result<EventQueryResult, StorageError> {
        let segment_path = self.data_dir.join(segment_file);
        if !segment_path.exists() {
            return Ok(EventQueryResult::default());
        }
        let reader = segment::SegmentReader::from_file(segment_path)?;
        let capped_limit = options.limit.min(10_000);
        let start_offset = options.cursor.map(|c| c.offset).unwrap_or(0);
        if capped_limit == 0 {
            return Ok(EventQueryResult {
                events: Vec::new(),
                next_cursor: Some(QueryCursor {
                    offset: start_offset,
                }),
            });
        }

        let mut rows = Vec::with_capacity(capped_limit.min(1024));
        let mut current_offset = 0u64;

        for item in reader.iter_events() {
            let event = item?;
            let wire_len = event.serialize()?.len() as u64;
            let event_offset = current_offset;
            current_offset = current_offset.saturating_add(wire_len);

            if event_offset < start_offset {
                continue;
            }
            if !matches_filter(&event, &options.filter) {
                continue;
            }

            rows.push(QueriedEvent {
                offset: event_offset,
                wire_len,
                event_hash: event.compute_hash(),
                event,
            });
            if rows.len() >= capped_limit {
                break;
            }
        }

        let next_cursor = if rows.len() == capped_limit
            && current_offset < reader.data_len() as u64
            && !rows.is_empty()
        {
            Some(QueryCursor {
                offset: current_offset,
            })
        } else {
            None
        };

        Ok(EventQueryResult {
            events: rows,
            next_cursor,
        })
    }

    /// Append a single event into a storage-managed segmented journal.
    /// Storage owns segment part naming, cache/registry persistence and sealing.
    pub fn append_event_to_segmented_journal(
        &self,
        journal_base: &str,
        event: &event::Event,
        max_segment_size: u64,
        durability: durability::DurabilityLevel,
    ) -> Result<u64, StorageError> {
        let mut segments = self.load_segmented_journal_registry(journal_base)?;
        let next_idx = segments.len().saturating_add(1);
        let segment_file = format!("{}.part-{:020}.seg", journal_base, next_idx);
        let mut session = self.create_append_session(&segment_file, max_segment_size)?;
        let offset = session.append_with_durability(event, durability)?;
        let _ = session.seal_now()?;
        segments.push(segment_file);
        self.save_segmented_journal_registry(journal_base, &segments)?;
        Ok(offset)
    }

    /// Query all raw events from a storage-managed segmented journal.
    /// Includes backward-compatible fallback to single-segment mode.
    pub fn query_segmented_journal_raw(
        &self,
        journal_base: &str,
    ) -> Result<Vec<QueriedEvent>, StorageError> {
        let segments = self.load_segmented_journal_registry(journal_base)?;
        let mut rows = Vec::new();
        if segments.is_empty() {
            let result = self.query_events_raw(
                journal_base,
                EventQueryOptions {
                    limit: 100_000,
                    cursor: Some(QueryCursor { offset: 0 }),
                    filter: EventQueryFilter::default(),
                },
            )?;
            rows.extend(result.events);
            return Ok(rows);
        }

        for segment_file in segments {
            let result = self.query_events_raw(
                &segment_file,
                EventQueryOptions {
                    limit: 100_000,
                    cursor: Some(QueryCursor { offset: 0 }),
                    filter: EventQueryFilter::default(),
                },
            )?;
            rows.extend(result.events);
        }
        Ok(rows)
    }

    #[allow(dead_code)]
    pub(crate) fn query_events_raw_with_seal(
        &self,
        segment_file: &str,
        options: EventQueryOptions,
        sessions: &mut [&mut EngineAppendSession],
        controller: &seal_policy::SealController,
        now_unix_ms: u64,
        chain_now: Option<seal_policy::ChainProgress>,
    ) -> Result<EventQueryResult, StorageError> {
        let _ = self.seal_pending_segments(sessions, controller, now_unix_ms, chain_now)?;
        self.query_events_raw(segment_file, options)
    }

    pub fn seal_pending_segments(
        &self,
        sessions: &mut [&mut EngineAppendSession],
        controller: &seal_policy::SealController,
        now_unix_ms: u64,
        chain_now: Option<seal_policy::ChainProgress>,
    ) -> Result<Vec<SealPendingResult>, StorageError> {
        let mut out = Vec::with_capacity(sessions.len());
        for session in sessions.iter_mut() {
            out.push(session.seal_pending_segment(controller, now_unix_ms, chain_now)?);
        }
        Ok(out)
    }

    fn load_segmented_journal_registry(
        &self,
        journal_base: &str,
    ) -> Result<Vec<String>, StorageError> {
        let path = self.segmented_journal_registry_path(journal_base);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let raw = std::fs::read_to_string(path)?;
        let mut segments = Vec::new();
        for line in raw.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                segments.push(trimmed.to_string());
            }
        }
        Ok(segments)
    }

    fn save_segmented_journal_registry(
        &self,
        journal_base: &str,
        segments: &[String],
    ) -> Result<(), StorageError> {
        let path = self.segmented_journal_registry_path(journal_base);
        let mut raw = String::new();
        for segment in segments {
            raw.push_str(segment);
            raw.push('\n');
        }
        std::fs::write(path, raw)?;
        Ok(())
    }

    fn segmented_journal_registry_path(&self, journal_base: &str) -> PathBuf {
        self.data_dir.join(format!("{}.segments", journal_base))
    }
}

fn matches_filter(event: &event::Event, filter: &EventQueryFilter) -> bool {
    if let Some(stream_id) = &filter.stream_id {
        if &event.stream_id != stream_id {
            return false;
        }
    }
    if let Some(actor_id) = &filter.actor_id {
        if &event.actor_id != actor_id {
            return false;
        }
    }
    if let Some(min_hlc) = filter.min_hlc {
        if event.hlc_timestamp < min_hlc {
            return false;
        }
    }
    if let Some(max_hlc) = filter.max_hlc {
        if event.hlc_timestamp > max_hlc {
            return false;
        }
    }
    true
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub struct EngineAppendSession {
    writer: async_segment_writer::AsyncSegmentWriter,
    manifest_manager: manifest::ManifestManager,
    replica_nodes: Vec<replication::NodeInfo>,
    replication_timeout: Duration,
    replication_batch_size: usize,
    opened_at_unix_ms: u64,
    last_append_unix_ms: u64,
    opened_chain: Option<seal_policy::ChainProgress>,
    latest_chain: Option<seal_policy::ChainProgress>,
    latest_progress_event_id: Option<String>,
    auto_seal_controller: Option<seal_policy::SealController>,
}

impl EngineAppendSession {
    pub fn configure_replica_nodes(&mut self, nodes: Vec<replication::NodeInfo>) {
        self.replica_nodes = nodes;
    }

    pub fn set_replication_timeout(&mut self, timeout: Duration) {
        self.replication_timeout = timeout;
    }

    pub fn set_replication_batch_size(&mut self, batch_size: usize) {
        self.replication_batch_size = batch_size.max(1);
    }

    pub fn replication_batch_size(&self) -> usize {
        self.replication_batch_size
    }

    pub fn mark_chain_progress(&mut self, slot: u64, epoch: u64) {
        self.latest_chain = Some(seal_policy::ChainProgress { slot, epoch });
        if self.opened_chain.is_none() {
            self.opened_chain = Some(seal_policy::ChainProgress { slot, epoch });
        }
    }

    pub fn apply_signed_chain_progress_event(
        &mut self,
        event: &SignedChainProgressEvent,
    ) -> Result<(), StorageError> {
        if event.progress_event_id.trim().is_empty() {
            return Err(StorageError::InvalidSealPolicy(
                "signed chain progress event has empty progress_event_id".to_string(),
            ));
        }
        if event.signature.trim().is_empty() || event.signer.trim().is_empty() {
            return Err(StorageError::InvalidSealPolicy(
                "signed chain progress event requires signer and signature".to_string(),
            ));
        }
        if let Some(prev) = self.latest_chain {
            if event.epoch < prev.epoch || (event.epoch == prev.epoch && event.slot < prev.slot) {
                return Err(StorageError::InvalidSealPolicy(format!(
                    "chain progress regression: prev=({}, {}), new=({}, {})",
                    prev.slot, prev.epoch, event.slot, event.epoch
                )));
            }
        }
        self.latest_progress_event_id = Some(event.progress_event_id.clone());
        self.mark_chain_progress(event.slot, event.epoch);
        Ok(())
    }

    /// Update chain progress from an RPC reader callback without coupling this
    /// crate to any specific chain/RPC client implementation.
    pub fn refresh_chain_progress_from_rpc_read<F>(
        &mut self,
        read_rpc: F,
    ) -> Result<(), StorageError>
    where
        F: FnOnce() -> Result<Option<seal_policy::ChainProgress>, StorageError>,
    {
        if let Some(progress) = read_rpc()? {
            self.mark_chain_progress(progress.slot, progress.epoch);
        }
        Ok(())
    }

    pub fn enable_auto_seal(&mut self, controller: seal_policy::SealController) {
        self.auto_seal_controller = Some(controller);
    }

    pub fn disable_auto_seal(&mut self) {
        self.auto_seal_controller = None;
    }

    pub fn active_segment_state(&self) -> seal_policy::ActiveSegmentState {
        seal_policy::ActiveSegmentState {
            opened_at_unix_ms: self.opened_at_unix_ms,
            last_append_unix_ms: self.last_append_unix_ms,
            opened_chain: self.opened_chain,
        }
    }

    pub fn should_seal(
        &self,
        controller: &seal_policy::SealController,
        now_unix_ms: u64,
        chain_now: Option<seal_policy::ChainProgress>,
    ) -> Result<seal_policy::SealDecision, StorageError> {
        controller.decide(now_unix_ms, chain_now, self.active_segment_state())
    }

    pub fn is_sealed(&self) -> bool {
        self.writer.is_sealed()
    }

    pub fn seal_now(&mut self) -> Result<Option<[u8; 32]>, StorageError> {
        if self.writer.is_sealed() {
            return Ok(None);
        }
        let segment_id = self.writer.seal()?;
        Ok(Some(segment_id))
    }

    pub fn seal_pending_segment(
        &mut self,
        controller: &seal_policy::SealController,
        now_unix_ms: u64,
        chain_now: Option<seal_policy::ChainProgress>,
    ) -> Result<SealPendingResult, StorageError> {
        if self.writer.is_sealed() {
            return Ok(SealPendingResult {
                decision: seal_policy::SealDecision::no(),
                sealed: false,
                segment_id: None,
                progress_event_id: self.latest_progress_event_id.clone(),
            });
        }
        let decision = self.should_seal(controller, now_unix_ms, chain_now)?;
        if !decision.should_seal {
            return Ok(SealPendingResult {
                decision,
                sealed: false,
                segment_id: None,
                progress_event_id: self.latest_progress_event_id.clone(),
            });
        }
        let segment_id = self.writer.seal()?;
        Ok(SealPendingResult {
            decision,
            sealed: true,
            segment_id: Some(segment_id),
            progress_event_id: self.latest_progress_event_id.clone(),
        })
    }

    pub fn enable_encryption(
        &mut self,
        store_key: [u8; 32],
        key_epoch: u32,
        mode: encryption::EncryptionMode,
        chunk_size: usize,
    ) -> Result<(), StorageError> {
        let store_uuid = self.current_store_uuid_bytes()?;
        let cfg = encryption::SegmentEncryptionConfig {
            store_uuid,
            key_epoch,
            chunk_size,
            mode,
            store_key,
        };
        self.writer.enable_encryption(cfg);
        Ok(())
    }

    pub fn enable_encryption_with_provider<P: key_management::KeyProvider>(
        &mut self,
        provider: &P,
        key_epoch: u32,
        mode: encryption::EncryptionMode,
        chunk_size: usize,
    ) -> Result<(), StorageError> {
        let store_uuid = self.current_store_uuid_bytes()?;
        let store_key = provider.load_store_key(store_uuid)?;
        self.enable_encryption(store_key, key_epoch, mode, chunk_size)
    }

    fn current_store_uuid_bytes(&self) -> Result<[u8; 16], StorageError> {
        let uuid: Uuid = self.manifest_manager.store_uuid()?;
        let mut out = [0u8; 16];
        out.copy_from_slice(uuid.as_bytes());
        Ok(out)
    }

    pub fn append_with_replication_acks(
        &mut self,
        event: &event::Event,
        required: u8,
        acked_replicas: &[event::ActorId],
    ) -> Result<u64, StorageError> {
        let offset = self.writer.append(event)?;
        self.last_append_unix_ms = now_unix_ms();
        self.writer
            .flush_with_durability(durability::DurabilityLevel::AckDurable)?;

        let mut quorum = replication::QuorumTracker::new(
            required,
            self.replication_timeout,
            &self.replica_nodes,
        )?;
        quorum.ack_local_durable();
        for peer in acked_replicas {
            quorum.ack_remote(peer)?;
        }
        quorum.finalize()?;
        self.auto_seal_after_append()?;
        Ok(offset)
    }

    pub fn append_with_durability(
        &mut self,
        event: &event::Event,
        durability: durability::DurabilityLevel,
    ) -> Result<u64, StorageError> {
        let mut offsets =
            self.append_batch_with_durability(std::slice::from_ref(event), durability)?;
        Ok(offsets.remove(0))
    }

    /// Append a batch of events and apply durability as a group commit.
    ///
    /// For `AckReplicatedN`, this performs one local durable flush and one
    /// batched remote ACK collection for all operation IDs.
    pub fn append_batch_with_durability(
        &mut self,
        events: &[event::Event],
        durability: durability::DurabilityLevel,
    ) -> Result<Vec<u64>, StorageError> {
        if events.is_empty() {
            return Ok(Vec::new());
        }
        self.last_append_unix_ms = now_unix_ms();
        let mut offsets = Vec::with_capacity(events.len());
        for event in events {
            offsets.push(self.writer.append(event)?);
        }

        match durability {
            durability::DurabilityLevel::AckCheckpointed => {
                let last = events.last().expect("non-empty events");
                let checkpoint = manifest::Checkpoint {
                    segment_id: self.writer.segment_id(),
                    offset: *offsets.last().expect("non-empty offsets"),
                    hlc: last.hlc_timestamp,
                };
                let (manifest_path, prepared) =
                    self.manifest_manager.prepare_checkpoint_write(checkpoint)?;
                self.writer.attach_manifest(manifest_path.clone())?;
                self.writer.flush_checkpointed(prepared.serialize()?)?;
                self.manifest_manager
                    .commit_prepared_checkpoint(&manifest_path, &prepared)?;
            }
            durability::DurabilityLevel::AckDurable => {
                self.writer.flush_with_durability(durability)?;
            }
            durability::DurabilityLevel::AckReplicatedN(required) => {
                let mut trackers = Vec::with_capacity(events.len());
                let mut op_ids = Vec::with_capacity(events.len());
                for event in events {
                    let tracker = replication::QuorumTracker::new(
                        required,
                        self.replication_timeout,
                        &self.replica_nodes,
                    )?;
                    trackers.push(tracker);
                    op_ids.push(event.compute_hash());
                }

                self.writer
                    .flush_with_durability(durability::DurabilityLevel::AckDurable)?;
                for tracker in &mut trackers {
                    tracker.ack_local_durable();
                }

                let acked_per_op = replication::collect_network_acks_for_ops(
                    &self.replica_nodes,
                    &op_ids,
                    self.replication_timeout,
                );

                for (i, tracker) in trackers.iter_mut().enumerate() {
                    if let Some(acked) = acked_per_op.get(&op_ids[i]) {
                        for peer in acked {
                            tracker.ack_remote(peer)?;
                            if tracker.is_satisfied() {
                                break;
                            }
                        }
                    }
                    tracker.finalize()?;
                }
            }
            durability::DurabilityLevel::AckLocal | durability::DurabilityLevel::AckBuffered => {}
        }
        self.auto_seal_after_append()?;
        Ok(offsets)
    }

    /// Append an event stream using internal chunking to drive batched durability.
    ///
    /// This is the preferred higher-level producer-loop API when the caller
    /// naturally emits a long stream of events and wants group-commit behavior
    /// without managing manual batch slices.
    pub fn append_stream_with_durability<I>(
        &mut self,
        events: I,
        durability: durability::DurabilityLevel,
        batch_size: usize,
    ) -> Result<Vec<u64>, StorageError>
    where
        I: IntoIterator<Item = event::Event>,
    {
        let batch_size = batch_size.max(1);
        let mut all_offsets = Vec::new();
        let mut chunk = Vec::with_capacity(batch_size);

        for event in events {
            chunk.push(event);
            if chunk.len() >= batch_size {
                let mut offsets = self.append_batch_with_durability(&chunk, durability)?;
                all_offsets.append(&mut offsets);
                chunk.clear();
            }
        }

        if !chunk.is_empty() {
            let mut offsets = self.append_batch_with_durability(&chunk, durability)?;
            all_offsets.append(&mut offsets);
        }

        Ok(all_offsets)
    }

    /// Append a stream with replicated durability using the session's default
    /// replication batch size.
    pub fn append_replicated_stream<I>(
        &mut self,
        events: I,
        required: u8,
    ) -> Result<Vec<u64>, StorageError>
    where
        I: IntoIterator<Item = event::Event>,
    {
        self.append_stream_with_durability(
            events,
            durability::DurabilityLevel::AckReplicatedN(required),
            self.replication_batch_size,
        )
    }

    pub fn flush(&mut self, durability: durability::DurabilityLevel) -> Result<(), StorageError> {
        self.writer.flush_with_durability(durability)?;
        Ok(())
    }

    fn auto_seal_after_append(&mut self) -> Result<(), StorageError> {
        let Some(controller) = self.auto_seal_controller.clone() else {
            return Ok(());
        };
        let _ = self.seal_pending_segment(&controller, now_unix_ms(), self.latest_chain)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::durability::DurabilityLevel;
    use tempfile::TempDir;

    #[test]
    fn test_storage_engine_new() -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        assert!(engine.data_dir().exists());
        Ok(())
    }

    #[test]
    fn test_storage_engine_creates_directory() -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let new_dir = temp_dir.path().join("data");
        let _engine = StorageEngine::new(new_dir.clone())?;
        assert!(new_dir.exists());
        Ok(())
    }

    #[test]
    fn test_query_events_raw_with_cursor_and_filter() -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let path = engine.data_dir().join("query.seg");
        let mut writer = segment::SegmentWriter::new(path, 1024 * 1024);

        let actor_a = event::ActorId::from_bytes([1u8; 16]);
        let actor_b = event::ActorId::from_bytes([2u8; 16]);
        let stream = event::StreamId::from_bytes([9u8; 16]);

        let e1 = event::Event::new(stream.clone(), actor_a.clone(), b"one".to_vec());
        let e2 = event::Event::new(stream.clone(), actor_b, b"two".to_vec());
        let e3 = event::Event::new(stream.clone(), actor_a.clone(), b"three".to_vec());

        let _ = writer.append(&e1)?;
        let _ = writer.append(&e2)?;
        let _ = writer.append(&e3)?;
        let _ = writer.seal_and_flush()?;

        let first = engine.query_events_raw(
            "query.seg",
            EventQueryOptions {
                limit: 2,
                ..EventQueryOptions::default()
            },
        )?;
        assert_eq!(first.events.len(), 2);
        assert!(first.next_cursor.is_some());
        assert_eq!(first.events[0].offset, 0);

        let second = engine.query_events_raw(
            "query.seg",
            EventQueryOptions {
                limit: 2,
                cursor: first.next_cursor.clone(),
                ..EventQueryOptions::default()
            },
        )?;
        assert_eq!(second.events.len(), 1);

        let filtered = engine.query_events_raw(
            "query.seg",
            EventQueryOptions {
                limit: 10,
                filter: EventQueryFilter {
                    actor_id: Some(actor_a),
                    ..EventQueryFilter::default()
                },
                ..EventQueryOptions::default()
            },
        )?;
        assert_eq!(filtered.events.len(), 2);
        assert_eq!(filtered.events[0].event.payload, b"one");
        assert_eq!(filtered.events[1].event.payload, b"three");
        Ok(())
    }

    #[test]
    fn test_engine_should_seal_uses_chain_policy() -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("seal-check.seg", 1024 * 1024)?;
        let event = event::Event::new(
            event::StreamId::new(),
            event::ActorId::new(),
            b"seal-me".to_vec(),
        );
        let _ = session.append_with_durability(&event, DurabilityLevel::AckDurable)?;
        session.mark_chain_progress(100, 1);

        let controller = seal_policy::SealController::new(seal_policy::StorageSealPolicy {
            mode: seal_policy::SealPolicyMode::ChainPreferred,
            chain: seal_policy::ChainSealPolicy {
                max_slot_span: 10,
                max_epoch_span: 10,
            },
            time: seal_policy::TimeSealPolicy {
                max_age_ms: u64::MAX / 2,
                idle_ms: u64::MAX / 2,
            },
        })?;

        let decision = session.should_seal(
            &controller,
            now_unix_ms(),
            Some(seal_policy::ChainProgress {
                slot: 111,
                epoch: 1,
            }),
        )?;
        assert!(decision.should_seal);
        assert_eq!(
            decision.trigger,
            Some(seal_policy::SealTrigger::ChainSlotSpan)
        );
        let applied = session.seal_pending_segment(
            &controller,
            now_unix_ms(),
            Some(seal_policy::ChainProgress {
                slot: 111,
                epoch: 1,
            }),
        )?;
        assert!(applied.sealed);
        assert!(applied.segment_id.is_some());
        assert!(session.is_sealed());
        Ok(())
    }

    #[test]
    fn test_auto_seal_on_append_chain_trigger() -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("auto-seal.seg", 1024 * 1024)?;

        let controller = seal_policy::SealController::new(seal_policy::StorageSealPolicy {
            mode: seal_policy::SealPolicyMode::ChainPreferred,
            chain: seal_policy::ChainSealPolicy {
                max_slot_span: 1,
                max_epoch_span: 10,
            },
            time: seal_policy::TimeSealPolicy {
                max_age_ms: u64::MAX / 2,
                idle_ms: u64::MAX / 2,
            },
        })?;
        session.enable_auto_seal(controller);
        session.mark_chain_progress(100, 1);
        session.mark_chain_progress(101, 1);

        let event = event::Event::new(
            event::StreamId::new(),
            event::ActorId::new(),
            b"auto-seal-now".to_vec(),
        );
        let _ = session.append_with_durability(&event, DurabilityLevel::AckDurable)?;
        assert!(session.is_sealed());
        Ok(())
    }

    #[test]
    fn test_query_events_raw_with_seal_seals_pending_then_reads() -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("query-with-seal.seg", 1024 * 1024)?;
        let event = event::Event::new(
            event::StreamId::new(),
            event::ActorId::new(),
            b"visible-after-seal".to_vec(),
        );
        let _ = session.append_with_durability(&event, DurabilityLevel::AckDurable)?;
        session.mark_chain_progress(50, 1);

        let controller = seal_policy::SealController::new(seal_policy::StorageSealPolicy {
            mode: seal_policy::SealPolicyMode::ChainPreferred,
            chain: seal_policy::ChainSealPolicy {
                max_slot_span: 1,
                max_epoch_span: 10,
            },
            time: seal_policy::TimeSealPolicy {
                max_age_ms: u64::MAX / 2,
                idle_ms: u64::MAX / 2,
            },
        })?;
        let mut sessions: Vec<&mut EngineAppendSession> = vec![&mut session];
        let result = engine.query_events_raw_with_seal(
            "query-with-seal.seg",
            EventQueryOptions {
                limit: 10,
                ..EventQueryOptions::default()
            },
            &mut sessions,
            &controller,
            now_unix_ms(),
            Some(seal_policy::ChainProgress { slot: 51, epoch: 1 }),
        )?;

        assert!(session.is_sealed());
        assert_eq!(result.events.len(), 1);
        assert_eq!(result.events[0].event.payload, b"visible-after-seal");
        Ok(())
    }

    #[test]
    fn test_auto_seal_on_append_with_rpc_read_progress() -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("auto-seal-rpc.seg", 1024 * 1024)?;

        let controller = seal_policy::SealController::new(seal_policy::StorageSealPolicy {
            mode: seal_policy::SealPolicyMode::ChainPreferred,
            chain: seal_policy::ChainSealPolicy {
                max_slot_span: 1,
                max_epoch_span: 10,
            },
            time: seal_policy::TimeSealPolicy {
                max_age_ms: u64::MAX / 2,
                idle_ms: u64::MAX / 2,
            },
        })?;
        session.enable_auto_seal(controller);

        session.refresh_chain_progress_from_rpc_read(|| {
            Ok(Some(seal_policy::ChainProgress {
                slot: 500,
                epoch: 2,
            }))
        })?;
        session.refresh_chain_progress_from_rpc_read(|| {
            Ok(Some(seal_policy::ChainProgress {
                slot: 501,
                epoch: 2,
            }))
        })?;

        let event = event::Event::new(
            event::StreamId::new(),
            event::ActorId::new(),
            b"rpc-fed-progress".to_vec(),
        );
        let _ = session.append_with_durability(&event, DurabilityLevel::AckDurable)?;
        assert!(session.is_sealed());
        Ok(())
    }

    #[test]
    fn test_signed_chain_progress_event_is_monotonic_and_attached_to_seal_result(
    ) -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("signed-progress.seg", 1024 * 1024)?;

        let event1 = SignedChainProgressEvent {
            progress_event_id: "prog-1".to_string(),
            slot: 700,
            epoch: 3,
            observed_at_unix_ms: now_unix_ms(),
            signer: "scheduler-key".to_string(),
            signature: "sig-1".to_string(),
        };
        session.apply_signed_chain_progress_event(&event1)?;

        let regressive = SignedChainProgressEvent {
            progress_event_id: "prog-0".to_string(),
            slot: 699,
            epoch: 3,
            observed_at_unix_ms: now_unix_ms(),
            signer: "scheduler-key".to_string(),
            signature: "sig-0".to_string(),
        };
        let err = session
            .apply_signed_chain_progress_event(&regressive)
            .unwrap_err();
        assert!(err.to_string().contains("chain progress regression"));

        let controller = seal_policy::SealController::new(seal_policy::StorageSealPolicy {
            mode: seal_policy::SealPolicyMode::ChainPreferred,
            chain: seal_policy::ChainSealPolicy {
                max_slot_span: 1,
                max_epoch_span: 10,
            },
            time: seal_policy::TimeSealPolicy {
                max_age_ms: u64::MAX / 2,
                idle_ms: u64::MAX / 2,
            },
        })?;
        let event = event::Event::new(
            event::StreamId::new(),
            event::ActorId::new(),
            b"signed-path".to_vec(),
        );
        let _ = session.append_with_durability(&event, DurabilityLevel::AckDurable)?;

        let event2 = SignedChainProgressEvent {
            progress_event_id: "prog-2".to_string(),
            slot: 701,
            epoch: 3,
            observed_at_unix_ms: now_unix_ms(),
            signer: "scheduler-key".to_string(),
            signature: "sig-2".to_string(),
        };
        session.apply_signed_chain_progress_event(&event2)?;
        let seal = session.seal_pending_segment(
            &controller,
            now_unix_ms(),
            Some(seal_policy::ChainProgress {
                slot: 701,
                epoch: 3,
            }),
        )?;
        assert_eq!(seal.progress_event_id.as_deref(), Some("prog-2"));
        Ok(())
    }

    #[test]
    fn test_end_to_end_signed_progress_seal_and_query_roundtrip() -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("roundtrip.seg", 1024 * 1024)?;

        let controller = seal_policy::SealController::new(seal_policy::StorageSealPolicy {
            mode: seal_policy::SealPolicyMode::ChainPreferred,
            chain: seal_policy::ChainSealPolicy {
                max_slot_span: 1,
                max_epoch_span: 1,
            },
            time: seal_policy::TimeSealPolicy {
                max_age_ms: u64::MAX / 2,
                idle_ms: u64::MAX / 2,
            },
        })?;

        session.apply_signed_chain_progress_event(&SignedChainProgressEvent {
            progress_event_id: "rt-1".to_string(),
            slot: 1000,
            epoch: 4,
            observed_at_unix_ms: now_unix_ms(),
            signer: "scheduler".to_string(),
            signature: "sig-1".to_string(),
        })?;
        let payload = b"kind=roundtrip;n=1".to_vec();
        let event = event::Event::new(event::StreamId::new(), event::ActorId::new(), payload);
        let _ = session.append_with_durability(&event, DurabilityLevel::AckDurable)?;

        session.apply_signed_chain_progress_event(&SignedChainProgressEvent {
            progress_event_id: "rt-2".to_string(),
            slot: 1001,
            epoch: 4,
            observed_at_unix_ms: now_unix_ms(),
            signer: "scheduler".to_string(),
            signature: "sig-2".to_string(),
        })?;
        let sealed = session.seal_pending_segment(
            &controller,
            now_unix_ms(),
            Some(seal_policy::ChainProgress {
                slot: 1001,
                epoch: 4,
            }),
        )?;
        assert!(sealed.sealed);
        assert_eq!(sealed.progress_event_id.as_deref(), Some("rt-2"));

        let queried = engine.query_events_raw(
            "roundtrip.seg",
            EventQueryOptions {
                limit: 10,
                ..EventQueryOptions::default()
            },
        )?;
        assert_eq!(queried.events.len(), 1);
        assert_eq!(
            queried.events[0].event.payload,
            b"kind=roundtrip;n=1".to_vec()
        );
        Ok(())
    }

    #[test]
    fn test_end_to_end() -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().to_path_buf();

        // 1. Create storage engine
        let engine = StorageEngine::new(data_dir.clone())?;
        assert!(engine.data_dir().exists());

        // 2. Create block device
        let block_path = data_dir.join("data.bin");
        let mut device = block::FileBlockDevice::new(block_path, false)?;
        device.set_size(1024 * 1024)?;

        // 3. Create events
        let actor_id = event::ActorId::new();
        let stream_id = event::StreamId::new();

        let event1 =
            event::Event::new(stream_id.clone(), actor_id.clone(), b"hello world".to_vec());
        let event1_hash = event1.compute_hash();

        // 4. Write to segment
        let segment_path = data_dir.join("segment.bin");
        let mut writer = segment::SegmentWriter::new(segment_path.clone(), 1024 * 1024);

        let offset1 = writer.append(&event1)?;
        assert_eq!(offset1, 0);

        // Write second event
        let event2 = event::Event::new(stream_id, actor_id.clone(), b"second event".to_vec());
        let offset2 = writer.append(&event2)?;
        assert_eq!(offset2, event1.serialize()?.len() as u64);

        // 5. Seal and flush segment to disk
        let segment_id = writer.seal_and_flush()?;
        assert!(!segment_id.iter().all(|&b| b == 0));

        // 6. Read back events from disk
        let reader = segment::SegmentReader::from_file(segment_path)?;
        assert!(reader.is_sealed());
        assert_eq!(reader.record_count(), 2);

        let retrieved_event1 = reader.get_event_at(0)?;
        assert_eq!(retrieved_event1.payload, b"hello world");

        // 7. Test CRDT
        let mut or_set = crdt::OrSet::new();
        or_set.add("item1".to_string(), actor_id.clone());
        or_set.add("item2".to_string(), actor_id.clone());

        assert!(or_set.contains("item1"));
        assert!(or_set.contains("item2"));
        assert!(!or_set.contains("item3"));

        // 8. Test indexes
        let event_hash_index = index::EventHashIndex::new();
        event_hash_index.insert(event1_hash, segment_id, offset1);

        let entry = event_hash_index.get(&event1_hash);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().offset, offset1);

        // 9. Test manifest with persistence
        let manifest_manager = manifest::ManifestManager::new(data_dir.clone())?;
        let sealed_seg = manifest::SealedSegment::new(segment_id, offset1, 1024);
        manifest_manager.add_sealed_segment(sealed_seg)?;

        // 10. Re-read manifest from disk to verify persistence
        let manifest_manager2 = manifest::ManifestManager::new(data_dir.clone())?;
        let segments = manifest_manager2.get_sealed_segments()?;
        assert_eq!(segments.len(), 1);

        // 10. Test replication
        let replicator = replication::Replicator::new(actor_id, [1u8; 16]);
        replicator.add_segment(segment_id);

        assert!(replicator.has_segment(&segment_id));

        // 11. Test Merkle tree
        let tree = replicator.create_merkle_tree();
        assert!(tree.root_hash().is_some());

        // 12. Test version vector
        let mut vv = index::VersionVector::new();
        vv.increment([1u8; 16]);
        vv.increment([1u8; 16]);
        assert_eq!(vv.get(&[1u8; 16]), 2);

        Ok(())
    }

    #[test]
    fn test_engine_append_session_checkpointed() -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("session.seg", 1024 * 1024)?;

        let actor_id = event::ActorId::new();
        let stream_id = event::StreamId::new();
        let event = event::Event::new(stream_id, actor_id, b"checkpoint".to_vec());
        let _ = session.append_with_durability(&event, DurabilityLevel::AckCheckpointed)?;

        let manifest = manifest::ManifestManager::new(engine.data_dir().clone())?.read_current()?;
        assert!(manifest.last_checkpoint.is_some());
        assert!(manifest.epoch >= 1);
        Ok(())
    }

    #[test]
    fn test_engine_append_session_replicated_n_local_only() -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("replicated.seg", 1024 * 1024)?;

        let event = event::Event::new(
            event::StreamId::new(),
            event::ActorId::new(),
            b"replicated-local".to_vec(),
        );
        let _ = session.append_with_durability(&event, DurabilityLevel::AckReplicatedN(1))?;
        Ok(())
    }

    #[test]
    fn test_engine_append_session_replicated_quorum_not_met() -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("replicated2.seg", 1024 * 1024)?;

        let event = event::Event::new(
            event::StreamId::new(),
            event::ActorId::new(),
            b"replicated-quorum".to_vec(),
        );
        let err = session
            .append_with_durability(&event, DurabilityLevel::AckReplicatedN(2))
            .unwrap_err();
        assert!(matches!(
            err,
            StorageError::Replication(
                replication::ReplicationError::InvalidQuorumRequirement { .. }
            )
        ));
        Ok(())
    }

    #[test]
    fn test_engine_append_with_replication_acks() -> Result<(), StorageError> {
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("replicated3.seg", 1024 * 1024)?;

        let peer = replication::NodeInfo::new(
            event::ActorId::new(),
            "127.0.0.1:9000".to_string(),
            [7u8; 16],
        );
        let peer_id = peer.actor_id.clone();
        session.configure_replica_nodes(vec![peer]);

        let event = event::Event::new(
            event::StreamId::new(),
            event::ActorId::new(),
            b"replicated-acks".to_vec(),
        );
        let _ = session.append_with_replication_acks(&event, 2, &[peer_id])?;
        Ok(())
    }

    #[test]
    fn test_engine_append_session_replicated_network_ack() -> Result<(), StorageError> {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;
        use std::time::Duration;

        replication::close_pooled_ack_connections();
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("replicated4.seg", 1024 * 1024)?;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut req = [0u8; 36];
            stream.read_exact(&mut req).unwrap();
            stream.write_all(b"ACK\n").unwrap();
        });

        let peer = replication::NodeInfo::new(event::ActorId::new(), addr.to_string(), [8u8; 16]);
        session.configure_replica_nodes(vec![peer]);
        session.set_replication_timeout(Duration::from_millis(500));

        let event = event::Event::new(
            event::StreamId::new(),
            event::ActorId::new(),
            b"replicated-network".to_vec(),
        );
        let _ = session.append_with_durability(&event, DurabilityLevel::AckReplicatedN(2))?;
        replication::close_pooled_ack_connections();
        server.join().unwrap();
        Ok(())
    }

    #[test]
    fn test_engine_append_batch_with_replication_group_commit() -> Result<(), StorageError> {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;
        use std::time::Duration;

        replication::close_pooled_ack_connections();
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("replicated5.seg", 1024 * 1024)?;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut header = [0u8; 6];
            stream.read_exact(&mut header).unwrap();
            assert_eq!(&header[0..4], b"AKB?");
            let count = u16::from_be_bytes([header[4], header[5]]) as usize;
            assert_eq!(count, 2);

            let mut body = vec![0u8; count * 32];
            stream.read_exact(&mut body).unwrap();

            let mut response = Vec::with_capacity(6 + body.len());
            response.extend_from_slice(b"AKB!");
            response.extend_from_slice(&(count as u16).to_be_bytes());
            response.extend_from_slice(&body);
            stream.write_all(&response).unwrap();
        });

        let peer = replication::NodeInfo::new(event::ActorId::new(), addr.to_string(), [9u8; 16]);
        session.configure_replica_nodes(vec![peer]);
        session.set_replication_timeout(Duration::from_millis(500));

        let event1 = event::Event::new(
            event::StreamId::new(),
            event::ActorId::new(),
            b"replicated-batch-1".to_vec(),
        );
        let event2 = event::Event::new(
            event::StreamId::new(),
            event::ActorId::new(),
            b"replicated-batch-2".to_vec(),
        );
        let offsets = session
            .append_batch_with_durability(&[event1, event2], DurabilityLevel::AckReplicatedN(2))?;
        assert_eq!(offsets.len(), 2);

        replication::close_pooled_ack_connections();
        server.join().unwrap();
        Ok(())
    }

    #[test]
    fn test_engine_append_stream_with_durability_batches() -> Result<(), StorageError> {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;
        use std::time::Duration;

        replication::close_pooled_ack_connections();
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("replicated6.seg", 1024 * 1024)?;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let mut handled = 0usize;
            let mut stream: Option<std::net::TcpStream> = None;
            while handled < 2 {
                if stream.is_none() {
                    let (accepted, _) = listener.accept().unwrap();
                    stream = Some(accepted);
                }
                let mut current = stream.take().unwrap();
                let mut magic = [0u8; 4];
                match current.read_exact(&mut magic) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => continue,
                    Err(e) => panic!("failed to read ack magic: {e}"),
                }
                if &magic == b"AKB?" {
                    let mut count = [0u8; 2];
                    current.read_exact(&mut count).unwrap();
                    let batch_count = u16::from_be_bytes(count) as usize;
                    if handled == 0 {
                        assert_eq!(batch_count, 2);
                    } else {
                        assert_eq!(batch_count, 1);
                    }
                    let mut body = vec![0u8; batch_count * 32];
                    current.read_exact(&mut body).unwrap();
                    let mut resp = Vec::with_capacity(6 + body.len());
                    resp.extend_from_slice(b"AKB!");
                    resp.extend_from_slice(&(batch_count as u16).to_be_bytes());
                    resp.extend_from_slice(&body);
                    current.write_all(&resp).unwrap();
                } else {
                    assert_eq!(&magic, b"ACK?");
                    let mut op = [0u8; 32];
                    current.read_exact(&mut op).unwrap();
                    current.write_all(b"ACK\n").unwrap();
                }
                handled += 1;
                stream = Some(current);
            }
        });

        let peer = replication::NodeInfo::new(event::ActorId::new(), addr.to_string(), [0x66; 16]);
        session.configure_replica_nodes(vec![peer]);
        session.set_replication_timeout(Duration::from_millis(500));

        let events = vec![
            event::Event::new(
                event::StreamId::new(),
                event::ActorId::new(),
                b"stream-batch-1".to_vec(),
            ),
            event::Event::new(
                event::StreamId::new(),
                event::ActorId::new(),
                b"stream-batch-2".to_vec(),
            ),
            event::Event::new(
                event::StreamId::new(),
                event::ActorId::new(),
                b"stream-batch-3".to_vec(),
            ),
        ];
        let offsets =
            session.append_stream_with_durability(events, DurabilityLevel::AckReplicatedN(2), 2)?;
        assert_eq!(offsets.len(), 3);
        replication::close_pooled_ack_connections();

        server.join().unwrap();
        Ok(())
    }

    #[test]
    fn test_engine_append_replicated_stream_uses_default_batch_size() -> Result<(), StorageError> {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;
        use std::time::Duration;

        replication::close_pooled_ack_connections();
        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("replicated7.seg", 1024 * 1024)?;
        session.set_replication_timeout(Duration::from_millis(500));
        session.set_replication_batch_size(2);
        assert_eq!(session.replication_batch_size(), 2);

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let mut handled = 0usize;
            let mut stream: Option<std::net::TcpStream> = None;
            while handled < 2 {
                if stream.is_none() {
                    let (accepted, _) = listener.accept().unwrap();
                    stream = Some(accepted);
                }
                let mut current = stream.take().unwrap();
                let mut magic = [0u8; 4];
                match current.read_exact(&mut magic) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => continue,
                    Err(e) => panic!("failed to read ack magic: {e}"),
                }
                if &magic == b"AKB?" {
                    let mut count = [0u8; 2];
                    current.read_exact(&mut count).unwrap();
                    let batch_count = u16::from_be_bytes(count) as usize;
                    if handled == 0 {
                        assert_eq!(batch_count, 2);
                    } else {
                        assert_eq!(batch_count, 1);
                    }
                    let mut body = vec![0u8; batch_count * 32];
                    current.read_exact(&mut body).unwrap();
                    let mut resp = Vec::with_capacity(6 + body.len());
                    resp.extend_from_slice(b"AKB!");
                    resp.extend_from_slice(&(batch_count as u16).to_be_bytes());
                    resp.extend_from_slice(&body);
                    current.write_all(&resp).unwrap();
                } else {
                    assert_eq!(&magic, b"ACK?");
                    let mut op = [0u8; 32];
                    current.read_exact(&mut op).unwrap();
                    current.write_all(b"ACK\n").unwrap();
                }
                handled += 1;
                stream = Some(current);
            }
        });

        let peer = replication::NodeInfo::new(event::ActorId::new(), addr.to_string(), [0x77; 16]);
        session.configure_replica_nodes(vec![peer]);

        let events = vec![
            event::Event::new(
                event::StreamId::new(),
                event::ActorId::new(),
                b"rep-stream-default-1".to_vec(),
            ),
            event::Event::new(
                event::StreamId::new(),
                event::ActorId::new(),
                b"rep-stream-default-2".to_vec(),
            ),
            event::Event::new(
                event::StreamId::new(),
                event::ActorId::new(),
                b"rep-stream-default-3".to_vec(),
            ),
        ];
        let offsets = session.append_replicated_stream(events, 2)?;
        assert_eq!(offsets.len(), 3);
        replication::close_pooled_ack_connections();
        server.join().unwrap();
        Ok(())
    }

    #[test]
    fn test_engine_session_enable_encryption_with_provider() -> Result<(), StorageError> {
        use crate::key_management::PassphraseKeyProvider;

        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("enc-provider.seg", 1024 * 1024)?;
        let provider = PassphraseKeyProvider::new("unit-test-passphrase");
        session.enable_encryption_with_provider(
            &provider,
            1,
            encryption::EncryptionMode::PayloadOnly,
            4096,
        )?;

        let event = event::Event::new(
            event::StreamId::new(),
            event::ActorId::new(),
            b"encrypted-via-provider".to_vec(),
        );
        let _ = session.append_with_durability(&event, DurabilityLevel::AckDurable)?;
        session.flush(DurabilityLevel::AckDurable)?;
        Ok(())
    }
}
