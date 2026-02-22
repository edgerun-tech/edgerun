// SPDX-License-Identifier: Apache-2.0
pub mod arena;
pub mod async_segment_writer;
pub mod block;
pub mod crash_test;
pub mod crdt;
pub mod durability;
pub mod encryption;
pub mod event;
pub mod index;
pub mod io_reactor;
pub mod key_management;
pub mod lsm_index;
pub mod manifest;
pub mod materialized_state;
pub mod optimized_writer;
pub mod replication;
pub mod segment;
pub mod sharding;

use std::path::PathBuf;
use std::time::Duration;
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
    #[error("Manifest error: {0}")]
    Manifest(#[from] manifest::ManifestError),
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
        })
    }
}

pub struct EngineAppendSession {
    writer: async_segment_writer::AsyncSegmentWriter,
    manifest_manager: manifest::ManifestManager,
    replica_nodes: Vec<replication::NodeInfo>,
    replication_timeout: Duration,
    replication_batch_size: usize,
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
        server.join().unwrap();
        Ok(())
    }

    #[test]
    fn test_engine_append_batch_with_replication_group_commit() -> Result<(), StorageError> {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;
        use std::time::Duration;

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

        server.join().unwrap();
        Ok(())
    }

    #[test]
    fn test_engine_append_stream_with_durability_batches() -> Result<(), StorageError> {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::thread;
        use std::time::Duration;

        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("replicated6.seg", 1024 * 1024)?;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();

            // First batch should contain 2 ops.
            let mut header1 = [0u8; 6];
            stream.read_exact(&mut header1).unwrap();
            assert_eq!(&header1[0..4], b"AKB?");
            let count1 = u16::from_be_bytes([header1[4], header1[5]]) as usize;
            assert_eq!(count1, 2);
            let mut body1 = vec![0u8; count1 * 32];
            stream.read_exact(&mut body1).unwrap();
            let mut resp1 = Vec::with_capacity(6 + body1.len());
            resp1.extend_from_slice(b"AKB!");
            resp1.extend_from_slice(&(count1 as u16).to_be_bytes());
            resp1.extend_from_slice(&body1);
            stream.write_all(&resp1).unwrap();

            // Tail chunk can be sent as single-op ACK? frame.
            let mut magic2 = [0u8; 4];
            stream.read_exact(&mut magic2).unwrap();
            if &magic2 == b"AKB?" {
                let mut count = [0u8; 2];
                stream.read_exact(&mut count).unwrap();
                let count2 = u16::from_be_bytes(count) as usize;
                assert_eq!(count2, 1);
                let mut body2 = vec![0u8; count2 * 32];
                stream.read_exact(&mut body2).unwrap();
                let mut resp2 = Vec::with_capacity(6 + body2.len());
                resp2.extend_from_slice(b"AKB!");
                resp2.extend_from_slice(&(count2 as u16).to_be_bytes());
                resp2.extend_from_slice(&body2);
                stream.write_all(&resp2).unwrap();
            } else {
                assert_eq!(&magic2, b"ACK?");
                let mut op = [0u8; 32];
                stream.read_exact(&mut op).unwrap();
                stream.write_all(b"ACK\n").unwrap();
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

        let temp_dir = TempDir::new().unwrap();
        let engine = StorageEngine::new(temp_dir.path().to_path_buf())?;
        let mut session = engine.create_append_session("replicated7.seg", 1024 * 1024)?;
        session.set_replication_timeout(Duration::from_millis(500));
        session.set_replication_batch_size(2);
        assert_eq!(session.replication_batch_size(), 2);

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();

            // First request should be batch of 2.
            let mut header1 = [0u8; 6];
            stream.read_exact(&mut header1).unwrap();
            assert_eq!(&header1[0..4], b"AKB?");
            let count1 = u16::from_be_bytes([header1[4], header1[5]]) as usize;
            assert_eq!(count1, 2);
            let mut body1 = vec![0u8; count1 * 32];
            stream.read_exact(&mut body1).unwrap();
            let mut resp1 = Vec::with_capacity(6 + body1.len());
            resp1.extend_from_slice(b"AKB!");
            resp1.extend_from_slice(&(count1 as u16).to_be_bytes());
            resp1.extend_from_slice(&body1);
            stream.write_all(&resp1).unwrap();

            // Tail can be single ACK?.
            let mut magic2 = [0u8; 4];
            stream.read_exact(&mut magic2).unwrap();
            assert_eq!(&magic2, b"ACK?");
            let mut op = [0u8; 32];
            stream.read_exact(&mut op).unwrap();
            stream.write_all(b"ACK\n").unwrap();
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
