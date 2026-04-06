//! Lifegraph storage layer — durable event log, encrypted blobs, and rebuildable indexes.
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
pub mod error;
pub mod sqlite;
pub mod store;

pub use error::StorageError;
pub use blobs::{BlobStore, BlobKeySource, BlobEntry, blob_file_path};
pub use sqlite::{SqliteIndex, SqliteIndexConfig, EventIndexEntry, ReplayEntry, FetchEntry};
pub use store::{NodeStore, NodeStoreConfig, CommandReplayResult, ObjectResult};

#[cfg(test)]
mod tests {
    use lifegraph_core::protocol::EventEnvelope;
    use lifegraph_proto::lifegraph::v0::stream::EventType;
    use crate::{NodeStore, NodeStoreConfig, BlobKeySource, CommandReplayResult};
    use std::fs;
    use std::sync::Arc;

    fn tmp_data_root() -> std::path::PathBuf {
        let dir = tempfile::tempdir().unwrap();
        dir.keep().to_path_buf()
    }

    fn test_event(stream_id: &str, seq: u64) -> EventEnvelope {
        EventEnvelope {
            envelope_version: 1,
            stream_id: stream_id.as_bytes().to_vec(),
            seq,
            prev_event_hash: if seq > 0 {
                Some(lifegraph_core::protocol::Digest {
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
        assert!(data_root.join("index.sqlite3").exists());
    }

    #[test]
    fn append_event_persists_to_disk() {
        let data_root = tmp_data_root();
        let config = test_config(data_root.clone());
        let mut store = NodeStore::open(&config).unwrap();

        let event = test_event("test-stream", 0);
        let offset = store.append_event(&event).unwrap();
        assert_eq!(offset, 0);

        // Verify the event log file exists and has content
        let log_path = data_root.join("events").join(format!("{}.log", hex::encode("test-stream")));
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

        store.append_event(&test_event("test-stream", 0)).unwrap();
        store.append_event(&test_event("test-stream", 1)).unwrap();

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

        store.append_event(&test_event("test-stream", 0)).unwrap();
        assert_eq!(store.get_head(b"test-stream").unwrap().unwrap().0, 0);

        store.append_event(&test_event("test-stream", 1)).unwrap();
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
            use sha2::{Digest, Sha256};
            let blob_id = hex::encode(Sha256::digest(b"persistent secret"));

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
            store.append_event(&test_event("rebuild-stream", i)).unwrap();
        }

        // Verify head
        assert_eq!(store.get_head(b"rebuild-stream").unwrap().unwrap().0, 4);

        // Delete the SQLite database
        drop(store);
        fs::remove_file(data_root.join("index.sqlite3")).unwrap();

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

        let obj_ref = lifegraph_proto::lifegraph::v0::common::ObjectRef {
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
        let object_id_hex = hex::encode(&obj_ref.object_id);

        // Enqueue a fetch for it
        store.enqueue_fetch("object", &object_id_hex, 0).unwrap();

        // Process the queue
        let resolved = store.process_fetch_queue().unwrap();
        assert_eq!(resolved, 1);
    }
}
