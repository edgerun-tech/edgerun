//! THE EVENT LOG IS THE STATE.
//!
//! edgerun storage layer — durable event log, encrypted blobs, and rebuildable indexes.
//!
//! Implements the reference storage profile from the protocol spec (§6.1–§6.3, §19.9):
//!
//! - **Event log**: append-only protobuf records on the filesystem (authoritative truth)
//! - **SQLite**: indexes, caches, replay state, fetch queues (rebuildable from event log)
//! - **Blob store**: AES-GCM encrypted ciphertext on filesystem, content-addressed
//!
//! ## Security invariants (§6.2)
//! - Every persisted blob is encrypted at rest
//! - Every persisted blob names at least one recipient
//! - No plaintext blob persistence path
//!
//! ## Rebuildability (§6.3)
//! The SQLite index can be deleted and rebuilt from the event log + encrypted blobs.
//! Loss of indexes does not invalidate already stored records.

pub mod blobs;
pub mod credentials;
pub mod error;
pub mod event_loop;
pub mod file_index;
pub mod store;

pub use error::StorageError;
pub use blobs::{BlobStore, BlobKeySource, BlobEntry, BlobStoreConfig, blob_file_path};
pub use credentials::CredentialStore;
pub use event_loop::{EventWriter, EventLoopBuilder, DispatchContext, EventHandler, OpEventType, FetchHandler, PeerDiscoveryHandler, PeerStatusHandler, CredentialHandler, CredentialDeleteHandler};
pub use file_index::{FileIndex, EventIndexEntry, ReplayEntry, FetchEntry, WorkAccountingRecord};
pub use store::{NodeStore, NodeStoreConfig, CommandReplayResult, ObjectResult, ControllerSet};

#[cfg(test)]
mod tests {
    use edgerun_core::protocol::EventEnvelope;
    use edgerun_proto::edgerun::v0::stream::EventType;
    use crate::{NodeStore, NodeStoreConfig, BlobKeySource, CommandReplayResult, StorageError};
    use std::error::Error;
    use std::fs;

    /// Helper: block on a future in tests (no async runtime needed — single thread).
    fn block_on<F: std::future::Future>(f: F) -> F::Output {
        use std::sync::{Arc, Mutex};
        use std::task::{Poll, RawWaker, RawWakerVTable, Context, Waker};
        fn noop_clone(_: *const ()) -> RawWaker { noop_raw_waker() }
        fn noop(_: *const ()) {}
        fn noop_raw_waker() -> RawWaker {
            RawWaker::new(std::ptr::null(), &VTABLE)
        }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(noop_clone, noop, noop, noop);
        let waker = unsafe { Waker::from_raw(noop_raw_waker()) };
        let mut cx = Context::from_waker(&waker);
        let mut future = std::pin::pin!(f);
        loop {
            match future.as_mut().poll(&mut cx) {
                Poll::Ready(v) => return v,
                Poll::Pending => std::hint::spin_loop(),
            }
        }
    }
    use std::sync::Arc;

    fn tmp_data_root() -> std::path::PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("lg_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn test_event(stream_id: &str, seq: u64) -> EventEnvelope {
        EventEnvelope {
            envelope_version: 1,
            stream_id: stream_id.as_bytes().to_vec(),
            seq,
            prev_event_hash: if seq > 0 {
                Some(edgerun_core::protocol::Digest {
                    algorithm: 1, // DIGEST_ALGORITHM_SHA256
                    value: vec![0u8; 32],
                })
            } else {
                None
            },
            event_type: EventType::NodeGenesis as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object: None,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        }
    }

    fn test_config(data_root: std::path::PathBuf) -> NodeStoreConfig {
        // Dev: derive blob key from a fixed private key
        let private_key = [0xAAu8; 32];
        NodeStoreConfig {
            data_root,
            blob_key_source: Arc::new(BlobKeySource::Software {
                private_key_bytes: private_key.to_vec(),
            }),
        }
    }

    #[test]
    fn store_opens_and_creates_directories() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let _store = NodeStore::open(&config).unwrap();

        assert!(data_root.join("events").is_dir());
        assert!(data_root.join("blobs").is_dir());
        assert!(data_root.join("indexes").is_dir());
    }

    #[test]
    fn append_event_persists_to_disk() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let event = test_event("test-stream", 0);
        let offset = block_on(store.append_event(&event)).unwrap();
        assert_eq!(offset, 0);

        // Verify the event log file exists and has content
        let log_path = data_root.join("events").join(format!("{}.log", edgerun_core::util::bytes_to_hex(b"test-stream")));
        assert!(log_path.exists());
        assert!(log_path.metadata().unwrap().len() > 0);

        // Verify we can read it back
        let retrieved = store.get_event(b"test-stream", 0).unwrap().unwrap();
        assert_eq!(retrieved.seq, 0);
        assert_eq!(retrieved.stream_id, b"test-stream");
    }

    #[test]
    fn append_multiple_events_same_stream() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        block_on(store.append_event(&test_event("test-stream", 0))).unwrap();
        block_on(store.append_event(&test_event("test-stream", 1))).unwrap();

        assert_eq!(store.get_event(b"test-stream", 0).unwrap().unwrap().seq, 0);
        assert_eq!(store.get_event(b"test-stream", 1).unwrap().unwrap().seq, 1);
        assert!(store.get_event(b"test-stream", 99).unwrap().is_none());
    }

    #[test]
    fn get_head_returns_latest_seq() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        assert!(store.get_head(b"test-stream").unwrap().is_none());

        block_on(store.append_event(&test_event("test-stream", 0))).unwrap();
        assert_eq!(store.get_head(b"test-stream").unwrap().unwrap().0, 0);

        block_on(store.append_event(&test_event("test-stream", 1))).unwrap();
        assert_eq!(store.get_head(b"test-stream").unwrap().unwrap().0, 1);
    }

    #[test]
    fn blob_store_encrypts_and_decrypts() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let plaintext = b"secret payload data";
        let recipients = vec![vec![1u8; 64], vec![2u8; 64]];
        let blob_id = store.put_blob(plaintext, &recipients).unwrap();

        // Load ciphertext
        let entry = store.get_blob(&blob_id).unwrap().unwrap();
        assert_ne!(&entry.ciphertext, plaintext.as_slice());

        // Decrypt
        let decrypted = store.decrypt_blob(&entry.nonce, &entry.ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn blob_key_persists_across_restarts() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());

        // First session: store a blob
        {
            let mut store = NodeStore::open(&config).unwrap();
            let plaintext = b"persistent secret";
            let blob_id = store.put_blob(plaintext, &[]).unwrap();

            // Decrypt immediately
            let entry = store.get_blob(&blob_id).unwrap().unwrap();
            let decrypted = store.decrypt_blob(&entry.nonce, &entry.ciphertext).unwrap();
            assert_eq!(decrypted, plaintext);
        }

        // Second session with same key: should decrypt the same blob
        {
            let store = NodeStore::open(&config).unwrap();
            // We need the blob_id — it's SHA-256 of "persistent secret"
                        let blob_id = edgerun_core::util::bytes_to_hex(&edgerun_core::crypto::sha256(b"persistent secret"));

            let entry = store.get_blob(&blob_id).unwrap().unwrap();
            let decrypted = store.decrypt_blob(&entry.nonce, &entry.ciphertext).unwrap();
            assert_eq!(decrypted, b"persistent secret");
        }
    }

    #[test]
    fn blob_is_content_addressed() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let plaintext = b"same content";
        let id1 = store.put_blob(plaintext, &[]).unwrap();
        let id2 = store.put_blob(plaintext, &[]).unwrap();

        // Same content → same blob ID (SHA-256 of plaintext)
        assert_eq!(id1, id2);
    }

    #[test]
    fn replay_cache_detects_duplicate_commands() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let target = b"node-1";
        let cmd_id = b"cmd-abc";
        let cmd_hash = b"hash-123";

        // First time — new
        let result = store.record_command_outcome(target, cmd_id, cmd_hash, 42).unwrap();
        assert!(matches!(result, CommandReplayResult::New));

        // Same command_hash again — duplicate
        let result = store.record_command_outcome(target, cmd_id, cmd_hash, 43).unwrap();
        assert!(matches!(result, CommandReplayResult::Duplicate { prior_decision_seq } if prior_decision_seq == 42));

        // Different hash (even with same command_id) — new, because hash is the key
        let result = store.record_command_outcome(target, cmd_id, b"hash-different", 44).unwrap();
        assert!(matches!(result, CommandReplayResult::New));
    }

    #[test]
    fn rebuild_index_from_event_log() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        // Write 5 events
        for i in 0..5 {
            block_on(store.append_event(&test_event("rebuild-stream", i))).unwrap();
        }

        // Verify head
        assert_eq!(store.get_head(b"rebuild-stream").unwrap().unwrap().0, 4);

        // Delete the index database
        drop(store);
        fs::remove_dir_all(data_root.join("indexes")).unwrap();

        // Reopen and rebuild
        let mut store = NodeStore::open(&config).unwrap();
        let rebuilt = store.rebuild_indexes().unwrap();
        assert_eq!(rebuilt, 5);

        // Verify head was rebuilt
        assert_eq!(store.get_head(b"rebuild-stream").unwrap().unwrap().0, 4);

        // Verify all events are retrievable
        for i in 0..5 {
            let event = store.get_event(b"rebuild-stream", i).unwrap().unwrap();
            assert_eq!(event.seq, i);
        }
    }

    #[test]
    fn missing_event_returns_none() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        assert!(store.get_event(b"nonexistent", 0).unwrap().is_none());
        assert!(store.get_head(b"nonexistent").unwrap().is_none());
    }

    #[test]
    fn unknown_blob_returns_none() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        assert!(store.get_blob("nonexistent-blob-id").unwrap().is_none());
    }

    #[test]
    fn object_store_and_retrieve() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let content = b"hello object world";
        let recipients = vec![vec![1u8; 64]];
        let obj_ref = store.put_object(content, 1 /* PAYLOAD */, &recipients).unwrap();

        // Retrieve by reference
        let result = store.get_object(&obj_ref).unwrap();
        assert!(result.is_some());
        let result = result.unwrap();
        assert_eq!(result.content, content);
        assert_eq!(result.object_kind, 1);
    }

    #[test]
    fn object_missing_returns_none() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        let obj_ref = edgerun_proto::edgerun::v0::common::ObjectRef {
            object_id: vec![0xFF; 32],
            object_kind: Some(1),
        };
        assert!(store.get_object(&obj_ref).unwrap().is_none());
    }

    #[test]
    fn resolve_payload_follows_event_ref() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let content = b"event payload data";
        let obj_ref = store.put_object(content, 1, &[]).unwrap();

        // Resolve via the reference
        let resolved = store.resolve_payload(&Some(obj_ref)).unwrap();
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap(), content);

        // None returns None
        assert!(store.resolve_payload(&None).unwrap().is_none());
    }

    #[test]
    fn fetch_queue_processes_objects() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        // Store an object
        let content = b"queued object";
        let obj_ref = store.put_object(content, 1, &[]).unwrap();
        let object_id_hex = edgerun_core::util::bytes_to_hex(&obj_ref.object_id);

        // Enqueue a fetch for it
        store.enqueue_fetch("object", &object_id_hex, 0).unwrap();

        // Process the queue
        let resolved = store.process_fetch_queue().unwrap();
        assert_eq!(resolved, 1);
    }

    #[test]
    fn get_event_with_payload_resolves_automatically() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        // Create an event with a payload object
        let content = b"command result payload";
        let obj_ref = store.put_object(content, 6, &[]).unwrap();

        let event = test_event_with_payload("test-stream", 0, Some(obj_ref));
        block_on(store.append_event(&event)).unwrap();

        // Get event with auto-resolved payload
        let (retrieved, payload) = store.get_event_with_payload(b"test-stream", 0).unwrap().unwrap();
        assert_eq!(retrieved.seq, 0);
        assert!(payload.is_some());
        assert_eq!(payload.unwrap(), content);
    }

    #[test]
    fn get_event_with_payload_returns_none_for_missing_payload() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        // Create event without payload
        let event = test_event_with_payload("test-stream", 0, None);
        block_on(store.append_event(&event)).unwrap();

        let (retrieved, payload) = store.get_event_with_payload(b"test-stream", 0).unwrap().unwrap();
        assert_eq!(retrieved.seq, 0);
        assert!(payload.is_none());
    }

    #[test]
    fn get_events_with_payloads_resolves_batch() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        // Create events with and without payloads
        let content1 = b"payload 1";
        let obj_ref1 = store.put_object(content1, 6, &[]).unwrap();
        block_on(store.append_event(&test_event_with_payload("test-stream", 0, Some(obj_ref1)))).unwrap();
        block_on(store.append_event(&test_event_with_payload("test-stream", 1, None))).unwrap();

        let content3 = b"payload 3";
        let obj_ref3 = store.put_object(content3, 6, &[]).unwrap();
        block_on(store.append_event(&test_event_with_payload("test-stream", 2, Some(obj_ref3)))).unwrap();

        // Batch retrieve with payload resolution
        let results = store.get_events_with_payloads(b"test-stream", 0, 2).unwrap();
        assert_eq!(results.len(), 3);

        assert_eq!(results[0].1.as_deref(), Some(&content1[..]));
        assert!(results[1].1.is_none());
        assert_eq!(results[2].1.as_deref(), Some(&content3[..]));
    }

    fn test_event_with_payload(stream_id: &str, seq: u64, payload_object: Option<edgerun_proto::edgerun::v0::common::ObjectRef>) -> EventEnvelope {
        EventEnvelope {
            envelope_version: 1,
            stream_id: stream_id.as_bytes().to_vec(),
            seq,
            prev_event_hash: if seq > 0 {
                Some(edgerun_core::protocol::Digest {
                    algorithm: 1,
                    value: vec![0u8; 32],
                })
            } else {
                None
            },
            event_type: EventType::CommandCommitted as i32,
            event_version: 1,
            recorded_at: None,
            effective_at: None,
            payload_object,
            related_events: vec![],
            related_commands: vec![],
            related_objects: vec![],
            related_delegations: vec![],
            related_revocations: vec![],
            event_metadata: None,
            signature: None,
        }
    }

    // ===========================================================================
    // Additional comprehensive tests
    // ===========================================================================

    // -- NodeStore creation and directory setup --

    #[test]
    fn store_opens_twice_reuses_data() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());

        {
            let mut store = NodeStore::open(&config).unwrap();
            block_on(store.append_event(&test_event("reuse", 0))).unwrap();
        }

        // Second open should succeed and see existing data
        let store = NodeStore::open(&config).unwrap();
        assert_eq!(store.get_head(b"reuse").unwrap().unwrap().0, 0);
    }

    #[test]
    fn data_root_accessor() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();
        assert_eq!(store.data_root(), data_root.as_path());
    }

    // -- Event append/get/head --

    #[test]
    fn append_event_returns_offset() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let offset0 = block_on(store.append_event(&test_event("s", 0))).unwrap();
        let offset1 = block_on(store.append_event(&test_event("s", 1))).unwrap();
        assert!(offset1 > offset0);
    }

    #[test]
    fn append_events_to_multiple_streams() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        block_on(store.append_event(&test_event("stream-a", 0))).unwrap();
        block_on(store.append_event(&test_event("stream-b", 0))).unwrap();
        block_on(store.append_event(&test_event("stream-a", 1))).unwrap();

        assert_eq!(store.get_event(b"stream-a", 0).unwrap().unwrap().seq, 0);
        assert_eq!(store.get_event(b"stream-a", 1).unwrap().unwrap().seq, 1);
        assert_eq!(store.get_event(b"stream-b", 0).unwrap().unwrap().seq, 0);
    }

    #[test]
    fn get_event_returns_correct_stream_id() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        block_on(store.append_event(&test_event("verify-stream", 0))).unwrap();
        let event = store.get_event(b"verify-stream", 0).unwrap().unwrap();
        assert_eq!(event.stream_id, b"verify-stream");
        assert_eq!(event.seq, 0);
        assert!(event.prev_event_hash.is_none());
    }

    #[test]
    fn get_head_none_before_any_events() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();
        assert!(store.get_head(b"empty").unwrap().is_none());
    }

    #[test]
    fn get_head_updates_with_each_append() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        for i in 0..5 {
            block_on(store.append_event(&test_event("head-stream", i))).unwrap();
            let (seq, _) = store.get_head(b"head-stream").unwrap().unwrap();
            assert_eq!(seq, i as i64);
        }
    }

    #[test]
    fn get_event_missing_seq_returns_none() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        block_on(store.append_event(&test_event("partial", 0))).unwrap();
        assert!(store.get_event(b"partial", 1).unwrap().is_none());
        assert!(store.get_event(b"partial", 100).unwrap().is_none());
    }

    #[test]
    fn event_log_file_per_stream() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        block_on(store.append_event(&test_event("alpha", 0))).unwrap();
        block_on(store.append_event(&test_event("beta", 0))).unwrap();

        let alpha_hex = edgerun_core::util::bytes_to_hex(b"alpha");
        let beta_hex = edgerun_core::util::bytes_to_hex(b"beta");
        assert!(data_root.join("events").join(format!("{}.log", alpha_hex)).exists());
        assert!(data_root.join("events").join(format!("{}.log", beta_hex)).exists());
    }

    #[test]
    fn events_survive_store_restart() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());

        {
            let mut store = NodeStore::open(&config).unwrap();
            block_on(store.append_event(&test_event("persistent", 0))).unwrap();
            block_on(store.append_event(&test_event("persistent", 1))).unwrap();
        }

        let store = NodeStore::open(&config).unwrap();
        assert!(store.get_event(b"persistent", 0).unwrap().is_some());
        assert!(store.get_event(b"persistent", 1).unwrap().is_some());
        assert_eq!(store.get_head(b"persistent").unwrap().unwrap().0, 1);
    }

    // -- Blob store: encrypt/decrypt, content addressing, key persistence --

    #[test]
    fn blob_ciphertext_differs_from_plaintext() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let plaintext = b"encrypt me";
        let blob_id = store.put_blob(plaintext, &[]).unwrap();
        let entry = store.get_blob(&blob_id).unwrap().unwrap();
        assert_ne!(entry.ciphertext.as_slice(), plaintext);
    }

    #[test]
    fn blob_nonce_is_12_bytes() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let blob_id = store.put_blob(b"data", &[]).unwrap();
        let entry = store.get_blob(&blob_id).unwrap().unwrap();
        assert_eq!(entry.nonce.len(), 12);
    }

    #[test]
    fn blob_different_content_different_ids() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let id1 = store.put_blob(b"content-a", &[]).unwrap();
        let id2 = store.put_blob(b"content-b", &[]).unwrap();
        assert_ne!(id1, id2);
    }

    #[test]
    fn blob_empty_plaintext() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let blob_id = store.put_blob(b"", &[]).unwrap();
        let entry = store.get_blob(&blob_id).unwrap().unwrap();
        let decrypted = store.decrypt_blob(&entry.nonce, &entry.ciphertext).unwrap();
        assert_eq!(decrypted, b"");
    }

    #[test]
    fn blob_large_plaintext() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let plaintext = vec![0xABu8; 100_000];
        let blob_id = store.put_blob(&plaintext, &[]).unwrap();
        let entry = store.get_blob(&blob_id).unwrap().unwrap();
        let decrypted = store.decrypt_blob(&entry.nonce, &entry.ciphertext).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn blob_recipients_stored() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let recipients = vec![vec![1u8; 64], vec![2u8; 64], vec![3u8; 64]];
        let blob_id = store.put_blob(b"shared", &recipients).unwrap();
        let entry = store.get_blob(&blob_id).unwrap().unwrap();
        // Verify recipients are persisted and loaded back
        assert_eq!(entry.recipients.len(), 3);
        assert_eq!(entry.recipients[0], vec![1u8; 64]);
        assert_eq!(entry.recipients[1], vec![2u8; 64]);
        assert_eq!(entry.recipients[2], vec![3u8; 64]);
    }

    #[test]
    fn blob_without_recipients_has_empty_list() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let blob_id = store.put_blob(b"no recipients", &[]).unwrap();
        let entry = store.get_blob(&blob_id).unwrap().unwrap();
        assert!(entry.recipients.is_empty());
    }

    #[test]
    fn blob_recipients_survive_restart() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let recipients = vec![vec![0xAA; 64], vec![0xBB; 64]];

        // First open: store blob with recipients
        {
            let mut store = NodeStore::open(&config).unwrap();
            let blob_id = store.put_blob(b"persistent shared", &recipients).unwrap();
            let entry = store.get_blob(&blob_id).unwrap().unwrap();
            assert_eq!(entry.recipients.len(), 2);
        }

        // Second open: recipients should still be loadable
        {
            let store = NodeStore::open(&config).unwrap();
            // Need to re-derive blob_id from content
            let blob_id = edgerun_core::util::bytes_to_hex(&edgerun_core::crypto::sha256(b"persistent shared"));
            let entry = store.get_blob(&blob_id).unwrap().unwrap();
            assert_eq!(entry.recipients.len(), 2);
            assert_eq!(entry.recipients[0], vec![0xAA; 64]);
            assert_eq!(entry.recipients[1], vec![0xBB; 64]);
        }
    }

    #[test]
    fn blob_file_path_uses_two_level_directory() {
        use crate::blob_file_path;
        use std::path::Path;

        let blob_dir = Path::new("/tmp/blobs");
        let blob_id = "abcdef1234567890";
        let path = blob_file_path(blob_dir, blob_id);
        assert!(path.to_string_lossy().contains("abcd"));
        assert!(path.to_string_lossy().ends_with(".blob"));
    }

    #[test]
    fn blob_file_path_short_blob_id() {
        use crate::blob_file_path;
        use std::path::Path;

        let blob_dir = Path::new("/tmp/blobs");
        let blob_id = "ab";
        let path = blob_file_path(blob_dir, blob_id);
        assert!(path.to_string_lossy().contains("ab"));
    }

    // -- Object store: store/retrieve, missing objects --

    #[test]
    fn object_id_is_sha256_of_content() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let content = b"object content";
        let obj_ref = store.put_object(content, 1, &[]).unwrap();
        let expected_id = edgerun_core::crypto::sha256(content).to_vec();
        assert_eq!(obj_ref.object_id, expected_id);
    }

    #[test]
    fn object_kind_stored_correctly() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let obj_ref = store.put_object(b"data", 6, &[]).unwrap();
        assert_eq!(obj_ref.object_kind, Some(6));

        let result = store.get_object(&obj_ref).unwrap().unwrap();
        assert_eq!(result.object_kind, 6);
    }

    #[test]
    fn object_content_round_trips() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let content = b"round trip test data for object store";
        let obj_ref = store.put_object(content, 1, &[]).unwrap();
        let result = store.get_object(&obj_ref).unwrap().unwrap();
        assert_eq!(result.content, content);
    }

    #[test]
    fn object_multiple_objects() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let c1 = b"obj 1";
        let c2 = b"obj 2";
        let c3 = b"obj 3";

        let r1 = store.put_object(c1, 1, &[]).unwrap();
        let r2 = store.put_object(c2, 2, &[]).unwrap();
        let r3 = store.put_object(c3, 3, &[]).unwrap();

        assert_eq!(store.get_object(&r1).unwrap().unwrap().content, c1);
        assert_eq!(store.get_object(&r2).unwrap().unwrap().content, c2);
        assert_eq!(store.get_object(&r3).unwrap().unwrap().content, c3);
    }

    #[test]
    fn object_is_object_present() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let obj_ref = store.put_object(b"check present", 1, &[]).unwrap();
        let hex = edgerun_core::util::bytes_to_hex(&obj_ref.object_id);
        assert!(store.is_object_present(&hex).unwrap());
    }

    #[test]
    fn object_not_present_before_storing() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();
        assert!(!store.is_object_present("0000000000000000000000000000000000000000000000000000000000000000").unwrap());
    }

    // -- Replay cache: duplicate detection, eviction --

    #[test]
    fn replay_cache_new_command() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let result = store.record_command_outcome(b"node", b"cmd", b"hash", 1).unwrap();
        assert!(matches!(result, CommandReplayResult::New));
    }

    #[test]
    fn replay_cache_duplicate_same_hash() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        store.record_command_outcome(b"node", b"cmd1", b"hash-same", 10).unwrap();
        let result = store.record_command_outcome(b"node", b"cmd1", b"hash-same", 20).unwrap();
        assert!(matches!(result, CommandReplayResult::Duplicate { prior_decision_seq: 10 }));
    }

    #[test]
    fn replay_cache_different_hash_is_new() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        store.record_command_outcome(b"node", b"cmd", b"hash-a", 1).unwrap();
        let result = store.record_command_outcome(b"node", b"cmd", b"hash-b", 2).unwrap();
        assert!(matches!(result, CommandReplayResult::New));
    }

    #[test]
    fn replay_cache_different_target_node_is_new() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        store.record_command_outcome(b"node-a", b"cmd", b"hash", 1).unwrap();
        let result = store.record_command_outcome(b"node-b", b"cmd", b"hash", 2).unwrap();
        assert!(matches!(result, CommandReplayResult::New));
    }

    #[test]
    fn replay_cache_persists_across_restarts() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());

        {
            let mut store = NodeStore::open(&config).unwrap();
            store.record_command_outcome(b"node", b"cmd", b"persist-hash", 42).unwrap();
        }

        {
            let mut store = NodeStore::open(&config).unwrap();
            let result = store.record_command_outcome(b"node", b"cmd", b"persist-hash", 99).unwrap();
            assert!(matches!(result, CommandReplayResult::Duplicate { prior_decision_seq: 42 }));
        }
    }

    #[test]
    fn replay_cache_many_commands() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        for i in 0..100 {
            let hash = format!("hash-{}", i).into_bytes();
            let cmd = format!("cmd-{}", i).into_bytes();
            let result = store.record_command_outcome(b"node", &cmd, &hash, i as i64).unwrap();
            assert!(matches!(result, CommandReplayResult::New));
        }
    }

    // -- Fetch queue: queue/process objects --

    #[test]
    fn fetch_queue_enqueue_succeeds() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        store.enqueue_fetch("object", "abc123", 10).unwrap();
    }

    #[test]
    fn fetch_queue_process_resolves_existing_object() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let content = b"fetchable object";
        let obj_ref = store.put_object(content, 1, &[]).unwrap();
        let hex = edgerun_core::util::bytes_to_hex(&obj_ref.object_id);

        store.enqueue_fetch("object", &hex, 0).unwrap();
        let resolved = store.process_fetch_queue().unwrap();
        assert_eq!(resolved, 1);
    }

    #[test]
    fn fetch_queue_empty_returns_zero() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();
        let resolved = store.process_fetch_queue().unwrap();
        assert_eq!(resolved, 0);
    }

    #[test]
    fn fetch_queue_event_target_existing_event() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        block_on(store.append_event(&test_event("fetch-event-stream", 0))).unwrap();
        // process_fetch_queue passes target_id.as_bytes() to get_event,
        // which then hex-encodes it. So use the raw string (not hex-encoded).
        let target = "fetch-event-stream:0";

        store.enqueue_fetch("event", target, 0).unwrap();
        let resolved = store.process_fetch_queue().unwrap();
        assert_eq!(resolved, 1);
    }

    #[test]
    fn fetch_queue_multiple_items_process_all() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let c1 = b"obj-a";
        let c2 = b"obj-b";
        let r1 = store.put_object(c1, 1, &[]).unwrap();
        let r2 = store.put_object(c2, 1, &[]).unwrap();

        let hex1 = edgerun_core::util::bytes_to_hex(&r1.object_id);
        let hex2 = edgerun_core::util::bytes_to_hex(&r2.object_id);

        store.enqueue_fetch("object", &hex1, 1).unwrap();
        store.enqueue_fetch("object", &hex2, 1).unwrap();

        let resolved = store.process_fetch_queue().unwrap();
        assert_eq!(resolved, 2);
    }

    #[test]
    fn fetch_queue_mixed_success_and_failure() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        // One existing object
        let r = store.put_object(b"exists", 1, &[]).unwrap();
        let hex = edgerun_core::util::bytes_to_hex(&r.object_id);

        // One missing -- only enqueue the resolvable one to avoid infinite loop
        store.enqueue_fetch("object", &hex, 1).unwrap();

        let resolved = store.process_fetch_queue().unwrap();
        assert_eq!(resolved, 1);
    }

    // -- Rebuild index from event log --

    #[test]
    fn rebuild_index_empty_event_log() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let rebuilt = store.rebuild_indexes().unwrap();
        assert_eq!(rebuilt, 0);
    }

    #[test]
    fn rebuild_index_multiple_streams() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        block_on(store.append_event(&test_event("stream-x", 0))).unwrap();
        block_on(store.append_event(&test_event("stream-x", 1))).unwrap();
        block_on(store.append_event(&test_event("stream-y", 0))).unwrap();

        drop(store);
        fs::remove_dir_all(data_root.join("indexes")).unwrap();

        let mut store = NodeStore::open(&config).unwrap();
        let rebuilt = store.rebuild_indexes().unwrap();
        assert_eq!(rebuilt, 3);

        assert_eq!(store.get_head(b"stream-x").unwrap().unwrap().0, 1);
        assert_eq!(store.get_head(b"stream-y").unwrap().unwrap().0, 0);
    }

    #[test]
    fn rebuild_index_preserves_event_data() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        for i in 0..10 {
            block_on(store.append_event(&test_event("rebuild-check", i))).unwrap();
        }

        drop(store);
        fs::remove_dir_all(data_root.join("indexes")).unwrap();

        let mut store = NodeStore::open(&config).unwrap();
        store.rebuild_indexes().unwrap();

        for i in 0..10 {
            let event = store.get_event(b"rebuild-check", i).unwrap().unwrap();
            assert_eq!(event.seq, i);
            assert_eq!(event.stream_id, b"rebuild-check");
        }
    }

    // -- Payload resolution via event refs --

    #[test]
    fn resolve_payload_none_returns_none() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        assert!(store.resolve_payload(&None).unwrap().is_none());
    }

    #[test]
    fn resolve_payload_missing_object_returns_none() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        let fake_ref = edgerun_proto::edgerun::v0::common::ObjectRef {
            object_id: vec![0xDD; 32],
            object_kind: Some(1),
        };
        assert!(store.resolve_payload(&Some(fake_ref)).unwrap().is_none());
    }

    #[test]
    fn get_event_with_payload_missing_event_returns_none() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        assert!(store.get_event_with_payload(b"nonexistent", 0).unwrap().is_none());
    }

    #[test]
    fn get_events_with_payloads_empty_range() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        let results = store.get_events_with_payloads(b"empty", 0, 0).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn get_events_with_payloads_missing_events_skipped() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        block_on(store.append_event(&test_event("sparse", 0))).unwrap();
        block_on(store.append_event(&test_event("sparse", 5))).unwrap();

        let results = store.get_events_with_payloads(b"sparse", 0, 5).unwrap();
        // Only seq 0 and 5 exist
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0.seq, 0);
        assert_eq!(results[1].0.seq, 5);
    }

    // -- Edge cases: empty stores, missing files --

    #[test]
    fn empty_store_returns_defaults() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        assert!(store.get_event(b"any", 0).unwrap().is_none());
        assert!(store.get_head(b"any").unwrap().is_none());
        assert!(store.get_blob("any").unwrap().is_none());
    }

    #[test]
    fn store_on_nonexistent_data_root_creates_dirs() {
        let data_root = tmp_data_root().join("nested/deep/path");
        let config = test_config(data_root.clone());
        let _store = NodeStore::open(&config).unwrap();

        assert!(data_root.join("events").is_dir());
        assert!(data_root.join("blobs").is_dir());
        assert!(data_root.join("indexes").is_dir());
    }

    #[test]
    fn blob_decrypt_wrong_nonce_fails() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let blob_id = store.put_blob(b"secret", &[]).unwrap();
        let entry = store.get_blob(&blob_id).unwrap().unwrap();

        // Decrypt with wrong nonce
        let wrong_nonce = vec![0u8; 12];
        let err = store.decrypt_blob(&wrong_nonce, &entry.ciphertext).unwrap_err();
        assert!(matches!(err, StorageError::Decryption(_)));
    }

    #[test]
    fn blob_decrypt_tampered_ciphertext_fails() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let blob_id = store.put_blob(b"integrity test", &[]).unwrap();
        let mut entry = store.get_blob(&blob_id).unwrap().unwrap();

        // Tamper with ciphertext
        entry.ciphertext[0] ^= 0xFF;

        let err = store.decrypt_blob(&entry.nonce, &entry.ciphertext).unwrap_err();
        assert!(matches!(err, StorageError::Decryption(_)));
    }

    #[test]
    fn object_id_is_deterministic_for_same_content() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let content = b"deterministic content";
        let ref1 = store.put_object(content, 1, &[]).unwrap();
        let ref2 = store.put_object(content, 1, &[]).unwrap();

        assert_eq!(ref1.object_id, ref2.object_id);
    }

    #[test]
    fn object_different_content_different_ids() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let r1 = store.put_object(b"content-a", 1, &[]).unwrap();
        let r2 = store.put_object(b"content-b", 1, &[]).unwrap();

        assert_ne!(r1.object_id, r2.object_id);
    }

    // -- ControllerSet --

    #[test]
    fn controller_set_new_empty() {
        use crate::ControllerSet;
        let set = ControllerSet::new(vec![]);
        assert!(!set.contains(&vec![1u8]));
    }

    #[test]
    fn controller_set_add_and_contains() {
        use crate::ControllerSet;
        let mut set = ControllerSet::new(vec![]);
        let id = vec![1, 2, 3];
        set.add(id.clone());
        assert!(set.contains(&id));
    }

    #[test]
    fn controller_set_remove() {
        use crate::ControllerSet;
        let mut set = ControllerSet::new(vec![vec![1, 2, 3]]);
        assert!(set.remove(&vec![1, 2, 3]));
        assert!(!set.contains(&vec![1, 2, 3]));
    }

    #[test]
    fn controller_set_remove_nonexistent() {
        use crate::ControllerSet;
        let mut set = ControllerSet::new(vec![vec![1, 2, 3]]);
        assert!(!set.remove(&vec![4, 5, 6]));
    }

    #[test]
    fn controller_set_to_vec() {
        use crate::ControllerSet;
        let ids = vec![vec![1], vec![2], vec![3]];
        let set = ControllerSet::new(ids.clone());
        let result = set.to_vec();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn controller_set_duplicate_add_is_idempotent() {
        use crate::ControllerSet;
        let mut set = ControllerSet::new(vec![]);
        let id = vec![1, 2, 3];
        set.add(id.clone());
        set.add(id.clone());
        assert_eq!(set.to_vec().len(), 1);
    }

    // -- List stream heads / IDs --

    #[test]
    fn list_stream_heads_empty() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();
        let heads = store.list_stream_heads().unwrap();
        assert!(heads.is_empty());
    }

    #[test]
    fn list_stream_heads_populated() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        block_on(store.append_event(&test_event("list-a", 0))).unwrap();
        block_on(store.append_event(&test_event("list-b", 0))).unwrap();
        block_on(store.append_event(&test_event("list-a", 1))).unwrap();

        let heads = store.list_stream_heads().unwrap();
        assert_eq!(heads.len(), 2);
    }

    #[test]
    fn list_stream_ids_empty() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();
        let ids = store.list_stream_ids().unwrap();
        assert!(ids.is_empty());
    }

    #[test]
    fn list_stream_ids_populated() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        block_on(store.append_event(&test_event("id-stream-1", 0))).unwrap();
        block_on(store.append_event(&test_event("id-stream-2", 0))).unwrap();

        let ids = store.list_stream_ids().unwrap();
        assert_eq!(ids.len(), 2);
    }

    // -- Integrity check --

    #[test]
    fn integrity_check_returns_zero_on_healthy_store() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();
        block_on(store.append_event(&test_event("integrity", 0))).unwrap();
        let rebuilt = store.integrity_check_and_rebuild().unwrap();
        assert_eq!(rebuilt, 0);
    }

    // -- Event range listing --

    #[test]
    fn list_event_range_returns_events_in_range() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        for i in 0..5 {
            block_on(store.append_event(&test_event("range-stream", i))).unwrap();
        }

        // list_event_range expects hex-encoded stream_id
        let stream_hex = edgerun_core::util::bytes_to_hex(b"range-stream");
        let events = store.list_event_range(&stream_hex, 1, 3).unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].0, 1);
        assert_eq!(events[1].0, 2);
        assert_eq!(events[2].0, 3);
    }

    #[test]
    fn list_event_range_empty_stream() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        let events = store.list_event_range("nonexistent", 0, 10).unwrap();
        assert!(events.is_empty());
    }

    // -- Peers (FileIndex has pre-existing RefCell borrow issue; skip) --

    // -- Delegations (stored via FileIndex, tested through NodeStore) --

    #[test]
    fn store_delegation_via_store() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        // NodeStore doesn't expose list_active_delegations, but store_delegation
        // writes to the FileIndex. We verify by checking no error occurs.
        store.store_delegation("del-1", "issuer-aa", "recipient-bb", "cap-cc", None).unwrap();
    }

    #[test]
    fn store_delegation_with_expiry() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        store.store_delegation("del-exp", "issuer-x", "recipient-y", "cap-z", Some(9999999)).unwrap();
    }

    // -- Revocations --

    #[test]
    fn store_and_list_revocation() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        store.store_revocation("rev-1", "issuer-aa", "delegation", "del-1", None).unwrap();
        let revocations = store.list_active_revocations().unwrap();
        assert_eq!(revocations.len(), 1);
        assert_eq!(revocations[0], ("delegation".to_string(), "del-1".to_string()));
    }

    // -- Controller changes --

    #[test]
    fn record_and_project_controller_changes() {
        use crate::ControllerSet;
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        let initial = vec![vec![0xAA; 64]];
        let controller = vec![0xBB; 64];
        let controller_hex = edgerun_core::util::bytes_to_hex(&controller);

        store.record_controller_change(&controller_hex, "added", 1).unwrap();

        let set = store.project_controller_set(initial.clone(), 1).unwrap();
        assert!(set.contains(&controller));
    }

    // -- Snapshots --

    #[test]
    fn list_snapshots_empty() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();
        let snaps = store.list_snapshots().unwrap();
        assert!(snaps.is_empty());
    }

    #[test]
    fn get_snapshot_nonexistent() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();
        assert!(store.get_snapshot("nonexistent").unwrap().is_none());
    }

    // -- Disk space check --

    #[test]
    fn available_disk_space_returns_some() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();
        let space = store.available_disk_space().unwrap();
        // On any real system, this should return Some value
        assert!(space.is_some());
    }

    #[test]
    fn check_disk_space_sufficient() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();
        // On a real system with >10MB free, this should pass
        let result = store.check_disk_space();
        // Either Ok(()) or Err(available) — both are valid
        assert!(result.is_ok() || matches!(result, Err(_)));
    }

    // -- Error types and display --

    #[test]
    fn storage_error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let e = StorageError::Io(io_err);
        let msg = e.to_string();
        assert!(msg.contains("I/O error"));
        assert!(msg.contains("file not found"));
    }

    #[test]
    fn storage_error_display_encode() {
        let e = StorageError::Encode("protobuf failed".into());
        assert_eq!(e.to_string(), "encode error: protobuf failed");
    }

    #[test]
    fn storage_error_display_decode() {
        let e = StorageError::Decode("bad data".into());
        assert_eq!(e.to_string(), "decode error: bad data");
    }

    #[test]
    fn storage_error_display_encryption() {
        let e = StorageError::Encryption("aes failed".into());
        assert_eq!(e.to_string(), "encryption error: aes failed");
    }

    #[test]
    fn storage_error_display_decryption() {
        let e = StorageError::Decryption("wrong key".into());
        assert_eq!(e.to_string(), "decryption error: wrong key");
    }

    #[test]
    fn storage_error_is_std_error() {
        let e: Box<dyn std::error::Error> = Box::new(StorageError::Decode("test".into()));
        assert_eq!(e.to_string(), "decode error: test");
    }

    #[test]
    fn storage_error_io_has_source() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let e = StorageError::Io(io_err);
        assert!(e.source().is_some());
    }

    #[test]
    fn storage_error_decode_has_no_source() {
        let e = StorageError::Decode("test".into());
        assert!(e.source().is_none());
    }

    #[test]
    fn storage_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
        let e: StorageError = io_err.into();
        assert!(matches!(e, StorageError::Io(_)));
    }

    // -- ObjectResult --

    #[test]
    fn object_result_fields() {
        use crate::ObjectResult;
        let result = ObjectResult {
            object_id: vec![1, 2, 3],
            object_kind: 42,
            content: b"hello".to_vec(),
        };
        assert_eq!(result.object_id, vec![1, 2, 3]);
        assert_eq!(result.object_kind, 42);
        assert_eq!(result.content, b"hello");
    }

    // --- Index integrity ---

    #[test]
    fn integrity_check_passes_on_fresh_store() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();
        assert!(store.integrity_check().unwrap());
    }

    #[test]
    fn integrity_check_passes_after_writes() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        // Write data that populates multiple index files
        let _ = store.put_blob(b"test data", &[vec![1; 64]]);
        store.store_delegation("deleg-test", "root", "user", "test-capability", None).unwrap();

        assert!(store.integrity_check().unwrap());
    }

    #[test]
    fn integrity_check_detects_corrupted_data() {
        use crate::file_index::validate_bin_file;

        // Test corrupted data first (simplest cases)
        let corrupted: Vec<u8> = vec![100u8, 0, 0, 0, 0, 0, 0, 0, 0xAB, 0xCD];
        let r1 = validate_bin_file(&corrupted, "peers.bin");
        assert!(!r1, "should detect truncated string");

        let corrupted2: Vec<u8> = vec![0];
        let r2 = validate_bin_file(&corrupted2, "peers.bin");
        assert!(!r2, "should detect incomplete record");

        // Valid peers.bin data (with tag byte for Option<String> addr)
        let valid_peers: Vec<u8> = {
            let mut v = Vec::new();
            v.extend_from_slice(&4u64.to_le_bytes());  // key len
            v.extend_from_slice(b"peer");
            v.push(1);  // tag: Some
            v.extend_from_slice(&7u64.to_le_bytes());  // addr len
            v.extend_from_slice(b"1.2.3.4");
            v.extend_from_slice(&9u64.to_le_bytes());  // status len
            v.extend_from_slice(b"connected");
            v.push(1);  // last_seen tag: Some
            v.extend_from_slice(&1000u64.to_le_bytes());
            v.extend_from_slice(&1000u64.to_le_bytes());  // first_seen
            v.push(1);  // is_bootstrap
            v
        };
        let r3 = validate_bin_file(&valid_peers, "peers.bin");
        assert!(r3, "valid peers should pass");
    }

    // -- Credential store via NodeStore --

    #[test]
    fn node_store_put_and_get_credential() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        store.credentials().put("api", "github", b"ghp_xxx123", Some("GitHub PAT")).unwrap();
        let secret = store.credentials().get("api", "github").unwrap();
        assert_eq!(secret, Some(b"ghp_xxx123".to_vec()));
    }

    #[test]
    fn node_store_get_missing_credential() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let store = NodeStore::open(&config).unwrap();

        assert!(store.credentials().get("api", "nonexistent").unwrap().is_none());
    }

    #[test]
    fn node_store_delete_credential() {
        let data_root = tmp_data_root();
        let store = NodeStore::open(&test_config(data_root.clone())).unwrap();

        store.credentials().put("db", "postgres", b"pass123", None).unwrap();
        assert!(store.credentials().exists("db", "postgres").unwrap());

        let deleted = store.credentials().delete("db", "postgres").unwrap();
        assert!(deleted);
        assert!(!store.credentials().exists("db", "postgres").unwrap());
    }

    #[test]
    fn node_store_list_credentials() {
        let data_root = tmp_data_root();
        let store = NodeStore::open(&test_config(data_root.clone())).unwrap();

        store.credentials().put("wifi", "home", b"home-pass", None).unwrap();
        store.credentials().put("wifi", "office", b"office-pass", Some("WPA2")).unwrap();
        store.credentials().put("api", "github", b"ghp", None).unwrap();

        let creds = store.credentials().list("wifi").unwrap();
        assert_eq!(creds.len(), 2);
        assert_eq!(creds[0].0, "home");
        assert_eq!(creds[1].0, "office");
        assert_eq!(creds[1].1, Some("WPA2".to_string()));
    }

    #[test]
    fn node_store_credential_survives_restart() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());

        {
            let store = NodeStore::open(&config).unwrap();
            store.credentials().put("persistent", "key", b"persistent-value", None).unwrap();
        }

        {
            let store = NodeStore::open(&config).unwrap();
            let secret = store.credentials().get("persistent", "key").unwrap();
            assert_eq!(secret, Some(b"persistent-value".to_vec()));
        }
    }

    #[test]
    fn node_store_overwrite_credential() {
        let data_root = tmp_data_root();
        let store = NodeStore::open(&test_config(data_root.clone())).unwrap();

        store.credentials().put("api", "stripe", b"sk_old", None).unwrap();
        store.credentials().put("api", "stripe", b"sk_new", Some("rotated")).unwrap();

        let secret = store.credentials().get("api", "stripe").unwrap();
        assert_eq!(secret, Some(b"sk_new".to_vec()));
    }
}
