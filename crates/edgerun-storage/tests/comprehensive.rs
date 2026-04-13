//! Comprehensive edge-case and gap tests for edgerun-storage.
//!
//! Covers: FileIndex standalone, BlobStore standalone, CredentialStore standalone,
//! NodeStore gaps (work accounting, peers, snapshots), corruption resilience,
//! large-scale data, binary content, and error paths.

use edgerun_core::protocol::EventEnvelope;
use edgerun_proto::edgerun::v0::stream::EventType;
use edgerun_storage::{
    NodeStore, NodeStoreConfig, BlobKeySource, BlobStore, BlobStoreConfig,
    FileIndex, CredentialStore, StorageError, CommandReplayResult,
};
use std::fs;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

// ============================================================================
// Helpers
// ============================================================================

fn tmp_data_root() -> std::path::PathBuf {
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("stor_test_{}_{}", std::process::id(), n));
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
            Some(edgerun_core::protocol::Digest { algorithm: 1, value: vec![0u8; 32] })
        } else { None },
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
    let private_key = [0xAAu8; 32];
    NodeStoreConfig {
        data_root,
        blob_key_source: Arc::new(BlobKeySource::Software { private_key_bytes: private_key.to_vec() }),
    }
}

fn make_blob_store(data_root: &std::path::Path) -> BlobStore {
    let blob_dir = data_root.join("blobs");
    let config = BlobStoreConfig { blob_dir };
    BlobStore::open(&config, BlobKeySource::Software { private_key_bytes: vec![0xBBu8; 32] }).unwrap()
}

fn make_file_index(data_root: &std::path::Path) -> FileIndex {
    FileIndex::open(&data_root.to_path_buf()).unwrap()
}

fn make_credential_store(data_root: &std::path::Path) -> CredentialStore {
    let blobs = Arc::new(make_blob_store(data_root));
    let index = Arc::new(make_file_index(data_root));
    CredentialStore::new(blobs, index)
}

// ============================================================================
// FileIndex Standalone Tests
// ============================================================================

#[test]
fn file_index_open_creates_events_dir() {
    let data_root = tmp_data_root();
    let _idx = make_file_index(&data_root);
    assert!(data_root.join("events").is_dir());
}

#[test]
fn file_index_save_load_roundtrip() {
    // FileIndex is now purely in-memory — rebuilt from event log on open.
    // Non-event-derived state (peers, credentials, replay cache, fetch queue)
    // is not persisted; it is repopulated through the event log on future runs.
    // This test verifies in-memory operations work correctly.
    let data_root = tmp_data_root();
    let idx = make_file_index(&data_root);

    // Write event-derived state
    idx.put_event("stream-a", 0, &[0x11; 32], 100, 1).unwrap();
    idx.set_head("stream-a", 0, &[0x11; 32]).unwrap();

    // Write non-event-derived state (in-memory only, lost on restart)
    idx.put_replay_entry("node-1", "hash-1", "cmd-1", 42).unwrap();
    idx.enqueue_fetch("object", "abc123", 10).unwrap();
    idx.mark_object_present("obj-hex", "blob-id", "blob-id").unwrap();
    idx.upsert_peer("peer-1", Some("1.2.3.4"), "active", false).unwrap();
    idx.put_credential("ns", "cred", "blob-xyz", Some("test cred")).unwrap();

    // Verify in-memory state
    assert!(idx.get_event("stream-a", 0).unwrap().is_some());
    let (seq, hash) = idx.get_head("stream-a").unwrap().unwrap();
    assert_eq!(seq, 0);
    assert_eq!(hash, vec![0x11u8; 32]);
    assert!(idx.get_replay_entry("node-1", "hash-1").unwrap().is_some());
    let peers = idx.list_peers().unwrap();
    assert_eq!(peers.len(), 1);
    assert!(idx.get_credential("ns", "cred").unwrap().is_some());

    // After reopening, event-derived state is rebuilt from event log files.
    // Since we didn't write any .log files (NodeStore does that), the index
    // is empty on reopen. This is correct — persistence flows through the event log.
    let idx2 = make_file_index(&data_root);
    assert!(idx2.get_event("stream-a", 0).unwrap().is_none());
    assert!(idx2.list_peers().unwrap().is_empty());
    assert!(idx2.get_credential("ns", "cred").unwrap().is_none());
}

#[test]
fn file_index_peers_crud() {
    let data_root = tmp_data_root();
    let idx = make_file_index(&data_root);

    // Upsert peers
    idx.upsert_peer("peer-1", Some("10.0.0.1:8080"), "active", true).unwrap();
    idx.upsert_peer("peer-2", Some("10.0.0.2:8080"), "unreachable", false).unwrap();
    idx.upsert_peer("peer-3", None, "unknown", false).unwrap();

    let peers = idx.list_peers().unwrap();
    assert_eq!(peers.len(), 3);

    // Update peer status
    idx.update_peer_status("peer-2", "active").unwrap();
    let peers = idx.list_peers().unwrap();
    let peer2 = peers.iter().find(|(id, _, _, _, _)| id == "peer-2").unwrap();
    assert_eq!(peer2.2, "active"); // status is 3rd element

    // List unreachable peers with addr
    let unreachable = idx.list_unreachable_peers_with_addr().unwrap();
    // peer-2 was updated to active, peer-3 has no addr, so empty
    assert!(unreachable.is_empty());
}

#[test]
fn file_index_snapshots() {
    let data_root = tmp_data_root();
    let idx = make_file_index(&data_root);

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    idx.put_snapshot("snap-1", "obj-abc", "full", "producer-1", now, 1, "heads-1").unwrap();
    idx.put_snapshot("snap-2", "obj-def", "delta", "producer-2", now, 0, "heads-2").unwrap();

    let snaps = idx.list_snapshots().unwrap();
    assert_eq!(snaps.len(), 2);

    let snap1 = idx.get_snapshot("snap-1").unwrap().unwrap();
    // Tuple: (id, object_id_hex, view_type, producer_hex, produced_at, completeness, base_heads)
    assert_eq!(snap1.2, "full"); // view_type
    assert_eq!(snap1.3, "producer-1"); // producer_hex
    assert_eq!(snap1.5, 1); // completeness

    assert!(idx.get_snapshot("nonexistent").unwrap().is_none());
}

#[test]
fn file_index_delegations() {
    let data_root = tmp_data_root();
    let idx = make_file_index(&data_root);

    idx.store_delegation("del-1", "issuer-a", "recipient-b", "cap-c", None).unwrap();
    idx.store_delegation("del-2", "issuer-a", "recipient-c", "cap-d", Some(9999999)).unwrap();

    let active = idx.list_active_delegations("issuer-a").unwrap();
    assert_eq!(active.len(), 2);

    // Revoke one
    idx.revoke_delegation("del-1").unwrap();
    let active = idx.list_active_delegations("issuer-a").unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(active[0], "del-2");
}

#[test]
fn file_index_revocations() {
    let data_root = tmp_data_root();
    let idx = make_file_index(&data_root);

    idx.store_revocation("rev-1", "issuer-a", "delegation", "del-1", None).unwrap();
    idx.store_revocation("rev-2", "issuer-b", "capability", "cap-1", None).unwrap();

    let revs = idx.list_active_revocations().unwrap();
    assert_eq!(revs.len(), 2);
    // HashMap order is non-deterministic, check both are present
    assert!(revs.contains(&("delegation".to_string(), "del-1".to_string())));
    assert!(revs.contains(&("capability".to_string(), "cap-1".to_string())));
}

#[test]
fn file_index_controller_changes() {
    let data_root = tmp_data_root();
    let idx = make_file_index(&data_root);

    idx.record_controller_change("controller-aa", "added", 1).unwrap();
    idx.record_controller_change("controller-bb", "added", 2).unwrap();
    idx.record_controller_change("controller-aa", "removed", 3).unwrap();

    let changes = idx.list_controller_changes(3).unwrap();
    assert_eq!(changes.len(), 3);
    assert_eq!(changes[0].1, "added");
    assert_eq!(changes[2].1, "removed");

    // Filter by seq
    let changes_upto_1 = idx.list_controller_changes(1).unwrap();
    assert_eq!(changes_upto_1.len(), 1);
}

#[test]
fn file_index_fetch_queue_operations() {
    let data_root = tmp_data_root();
    let idx = make_file_index(&data_root);

    // Enqueue items
    idx.enqueue_fetch("object", "obj-1", 10).unwrap();
    idx.enqueue_fetch("object", "obj-2", 5).unwrap();
    idx.enqueue_fetch("event", "evt-1", 1).unwrap();

    // Dequeue one (first pending)
    let dequeued = idx.dequeue_fetch().unwrap();
    assert!(dequeued.is_some());
    let entry = dequeued.unwrap();
    assert_eq!(entry.id, 1);

    // Mark it done
    idx.mark_fetch_done(1).unwrap();

    // Dequeue next
    let dequeued2 = idx.dequeue_fetch().unwrap();
    assert!(dequeued2.is_some());
    assert_eq!(dequeued2.unwrap().id, 2);

    // Mark as failed
    idx.mark_fetch_failed(2).unwrap();

    // Dequeue last
    let dequeued3 = idx.dequeue_fetch().unwrap();
    assert_eq!(dequeued3.unwrap().id, 3);
}

#[test]
fn file_index_work_accounting() {
    let data_root = tmp_data_root();
    let idx = make_file_index(&data_root);

    use edgerun_storage::WorkAccountingRecord;

    let record1 = WorkAccountingRecord {
        data: vec![1, 2, 3],
        record_hash: "hash1".to_string(),
        requester_hex: "aa".to_string(),
        provider_hex: "bb".to_string(),
        workload_class: "inference".to_string(),
        status: "completed".to_string(),
        started_at_us: 1_000_000,
        billable_rc_us: 500_000,
    };
    let record2 = WorkAccountingRecord {
        data: vec![4, 5, 6],
        record_hash: "hash2".to_string(),
        requester_hex: "aa".to_string(),
        provider_hex: "cc".to_string(),
        workload_class: "embedding".to_string(),
        status: "completed".to_string(),
        started_at_us: 2_000_000,
        billable_rc_us: 300_000,
    };
    let record3 = WorkAccountingRecord {
        data: vec![7, 8, 9],
        record_hash: "hash3".to_string(),
        requester_hex: "dd".to_string(),
        provider_hex: "bb".to_string(),
        workload_class: "inference".to_string(),
        status: "failed".to_string(),
        started_at_us: 3_000_000,
        billable_rc_us: 100_000,
    };

    idx.record_work_accounting(record1).unwrap();
    idx.record_work_accounting(record2).unwrap();
    idx.record_work_accounting(record3).unwrap();

    // Total billable for requester AA (only completed)
    let total = idx.total_billable_for_requester("aa").unwrap();
    assert_eq!(total, 800_000);

    // Total billable for provider BB (only completed)
    let total = idx.total_billable_for_provider("bb").unwrap();
    assert_eq!(total, 500_000);

    // Work by class
    let inference = idx.work_by_class("inference").unwrap();
    assert_eq!(inference.len(), 2);

    // Work by status
    let completed = idx.work_by_status("completed").unwrap();
    assert_eq!(completed.len(), 2);

    // Work in time range
    let in_range = idx.work_in_time_range(1_500_000, 2_500_000).unwrap();
    assert_eq!(in_range.len(), 1);
    assert_eq!(in_range[0].billable_rc_us, 300_000);

    // List all
    let all = idx.list_all_work().unwrap();
    assert_eq!(all.len(), 3);
}

#[test]
fn file_index_credentials_standalone() {
    let data_root = tmp_data_root();
    let idx = make_file_index(&data_root);

    idx.put_credential("api", "github", "blob-1", Some("GitHub PAT")).unwrap();
    idx.put_credential("api", "stripe", "blob-2", None).unwrap();
    idx.put_credential("db", "postgres", "blob-3", Some("DB password")).unwrap();

    // Get
    let cred = idx.get_credential("api", "github").unwrap().unwrap();
    assert_eq!(cred.blob_id, "blob-1");
    assert_eq!(cred.description, Some("GitHub PAT".to_string()));

    // List namespace
    let api_creds = idx.list_credentials("api").unwrap();
    assert_eq!(api_creds.len(), 2);

    // List namespaces
    let namespaces = idx.list_credential_namespaces().unwrap();
    assert_eq!(namespaces.len(), 2);
    assert!(namespaces.contains(&"api".to_string()));
    assert!(namespaces.contains(&"db".to_string()));

    // Delete
    let deleted = idx.delete_credential("api", "stripe").unwrap();
    assert!(deleted);
    assert!(idx.get_credential("api", "stripe").unwrap().is_none());

    // Delete nonexistent
    let deleted = idx.delete_credential("api", "stripe").unwrap();
    assert!(!deleted);
}

#[test]
fn file_index_clear() {
    let data_root = tmp_data_root();
    let idx = make_file_index(&data_root);

    idx.put_event("stream", 0, &[1; 32], 0, 1).unwrap();
    idx.set_head("stream", 0, &[1; 32]).unwrap();
    idx.upsert_peer("p1", Some("1.2.3.4"), "up", false).unwrap();
    idx.put_credential("ns", "cred", "blob", None).unwrap();

    idx.clear().unwrap();

    // In-memory state should be empty after clear
    assert!(idx.get_head("stream").unwrap().is_none());
    assert!(idx.list_peers().unwrap().is_empty());
    assert!(idx.list_credential_namespaces().unwrap().is_empty());
}

#[test]
fn file_index_integrity_check() {
    let data_root = tmp_data_root();
    let idx = make_file_index(&data_root);

    // Fresh index should be valid
    assert!(idx.integrity_check().unwrap());

    // Add data and check again
    idx.put_event("stream", 0, &[1; 32], 0, 1).unwrap();
    idx.set_head("stream", 0, &[1; 32]).unwrap();
    assert!(idx.integrity_check().unwrap());
}

#[test]
fn file_index_wal_noops() {
    let data_root = tmp_data_root();
    let idx = make_file_index(&data_root);

    // WAL operations are no-ops for file-based index
    assert_eq!(idx.wal_checkpoint().unwrap(), 0);
    assert_eq!(idx.wal_size_bytes().unwrap(), 0);
}

// ============================================================================
// BlobStore Standalone Tests
// ============================================================================

#[test]
fn blobstore_open_creates_dir() {
    let data_root = tmp_data_root();
    let _store = make_blob_store(&data_root);
    assert!(data_root.join("blobs").is_dir());
}

#[test]
fn blobstore_store_and_load() {
    let data_root = tmp_data_root();
    let store = make_blob_store(&data_root);

    let plaintext = b"standalone blob test";
    let recipients = vec![vec![1; 64], vec![2; 64]];
    let blob_id = store.store(plaintext, &recipients).unwrap();

    // Load
    let entry = store.load(&blob_id).unwrap().unwrap();
    assert!(!entry.ciphertext.is_empty());
    assert_eq!(entry.nonce.len(), 12);
    assert_eq!(entry.recipients.len(), 2);

    // Decrypt
    let decrypted = store.decrypt(&entry.nonce, &entry.ciphertext).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn blobstore_load_nonexistent_returns_none() {
    let data_root = tmp_data_root();
    let store = make_blob_store(&data_root);
    assert!(store.load("nonexistent-blob").unwrap().is_none());
}

#[test]
fn blobstore_decrypt_wrong_nonce() {
    let data_root = tmp_data_root();
    let store = make_blob_store(&data_root);

    let blob_id = store.store(b"secret", &[]).unwrap();
    let entry = store.load(&blob_id).unwrap().unwrap();

    let wrong_nonce = vec![0u8; 12];
    let err = store.decrypt(&wrong_nonce, &entry.ciphertext).unwrap_err();
    assert!(matches!(err, StorageError::Decryption(_)));
}

#[test]
fn blobstore_decrypt_tampered_ciphertext() {
    let data_root = tmp_data_root();
    let store = make_blob_store(&data_root);

    let blob_id = store.store(b"integrity", &[]).unwrap();
    let mut entry = store.load(&blob_id).unwrap().unwrap();

    entry.ciphertext[0] ^= 0xFF;
    let err = store.decrypt(&entry.nonce, &entry.ciphertext).unwrap_err();
    assert!(matches!(err, StorageError::Decryption(_)));
}

#[test]
fn blobstore_empty_plaintext() {
    let data_root = tmp_data_root();
    let store = make_blob_store(&data_root);

    let blob_id = store.store(b"", &[]).unwrap();
    let entry = store.load(&blob_id).unwrap().unwrap();
    let decrypted = store.decrypt(&entry.nonce, &entry.ciphertext).unwrap();
    assert_eq!(decrypted, b"");
}

#[test]
fn blobstore_large_payload() {
    let data_root = tmp_data_root();
    let store = make_blob_store(&data_root);

    let plaintext = vec![0xABu8; 1_000_000]; // 1 MB
    let blob_id = store.store(&plaintext, &[]).unwrap();
    let entry = store.load(&blob_id).unwrap().unwrap();
    let decrypted = store.decrypt(&entry.nonce, &entry.ciphertext).unwrap();
    assert_eq!(decrypted, plaintext);
}

#[test]
fn blobstore_content_addressing() {
    let data_root = tmp_data_root();
    let store = make_blob_store(&data_root);

    let id1 = store.store(b"same content", &[]).unwrap();
    let id2 = store.store(b"same content", &[]).unwrap();
    assert_eq!(id1, id2);

    let id3 = store.store(b"different content", &[]).unwrap();
    assert_ne!(id1, id3);
}

#[test]
fn blobstore_recipient_metadata_file() {
    let data_root = tmp_data_root();
    let store = make_blob_store(&data_root);

    let recipients = vec![vec![0xAA; 64], vec![0xBB; 64]];
    let blob_id = store.store(b"shared secret", &recipients).unwrap();

    // Verify .meta file exists
    let prefix = &blob_id[..4];
    let meta_path = data_root.join("blobs").join(prefix).join(format!("{}.blob.meta", blob_id));
    assert!(meta_path.exists());

    // Verify content
    let meta_content = fs::read_to_string(&meta_path).unwrap();
    let lines: Vec<&str> = meta_content.lines().collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn blobstore_corrupted_file_too_short() {
    let data_root = tmp_data_root();
    let store = make_blob_store(&data_root);

    let blob_id = store.store(b"data", &[]).unwrap();
    let blob_path = edgerun_storage::blob_file_path(&data_root.join("blobs"), &blob_id);

    // Truncate to less than 12 bytes (nonce size)
    fs::write(&blob_path, vec![0u8; 5]).unwrap();

    let result = store.load(&blob_id);
    assert!(result.is_err());
}

#[test]
fn blobstore_key_derivation_deterministic() {
    let data_root = tmp_data_root();
    let blob_dir = data_root.join("blobs");

    // Two stores with same key source should decrypt each other's blobs
    let config1 = BlobStoreConfig { blob_dir: blob_dir.clone() };
    let store1 = BlobStore::open(&config1, BlobKeySource::Software { private_key_bytes: vec![0x66; 32] }).unwrap();

    let blob_id = store1.store(b"cross-decrypt", &[]).unwrap();

    let config2 = BlobStoreConfig { blob_dir };
    let store2 = BlobStore::open(&config2, BlobKeySource::Software { private_key_bytes: vec![0x66; 32] }).unwrap();

    let entry = store2.load(&blob_id).unwrap().unwrap();
    let decrypted = store2.decrypt(&entry.nonce, &entry.ciphertext).unwrap();
    assert_eq!(decrypted, b"cross-decrypt");
}

#[test]
fn blobstore_different_key_cannot_decrypt() {
    let data_root = tmp_data_root();
    let blob_dir1 = data_root.join("blobs1");
    let blob_dir2 = data_root.join("blobs2");

    let config1 = BlobStoreConfig { blob_dir: blob_dir1.clone() };
    let store1 = BlobStore::open(&config1, BlobKeySource::Software { private_key_bytes: vec![0x77; 32] }).unwrap();

    let blob_id = store1.store(b"secret", &[]).unwrap();

    let config2 = BlobStoreConfig { blob_dir: blob_dir2.clone() };
    let store2 = BlobStore::open(&config2, BlobKeySource::Software { private_key_bytes: vec![0x88; 32] }).unwrap();

    // Copy blob file to second store's directory
    let src_path = edgerun_storage::blob_file_path(&blob_dir1, &blob_id);
    let dst_dir = blob_dir2.join(&blob_id[..4]);
    fs::create_dir_all(&dst_dir).unwrap();
    let dst_path = dst_dir.join(format!("{}.blob", blob_id));
    fs::copy(&src_path, &dst_path).unwrap();

    let entry = store2.load(&blob_id).unwrap().unwrap();
    let err = store2.decrypt(&entry.nonce, &entry.ciphertext).unwrap_err();
    assert!(matches!(err, StorageError::Decryption(_)));
}

#[test]
fn blobstore_hardware_sealed_key_first_open() {
    let data_root = tmp_data_root();
    let blob_dir = data_root.join("blobs");

    let seal_fn = Arc::new(|key: &[u8; 32]| -> Result<Vec<u8>, StorageError> { Ok(key.to_vec()) });
    let unseal_fn = Arc::new(|data: &[u8]| -> Result<[u8; 32], StorageError> {
        let mut arr = [0u8; 32];
        arr.copy_from_slice(data);
        Ok(arr)
    });

    let config = BlobStoreConfig { blob_dir: blob_dir.clone() };
    let key_source = BlobKeySource::HardwareSealed { unseal_fn, seal_fn };

    let store = BlobStore::open(&config, key_source).unwrap();

    // Should have created the sealed key file
    assert!(blob_dir.join(".blob_key.sealed").exists());

    // Should be able to store and decrypt
    let blob_id = store.store(b"hardware key test", &[]).unwrap();
    let entry = store.load(&blob_id).unwrap().unwrap();
    let decrypted = store.decrypt(&entry.nonce, &entry.ciphertext).unwrap();
    assert_eq!(decrypted, b"hardware key test");
}

#[test]
fn blobstore_hardware_sealed_key_persists() {
    let data_root = tmp_data_root();
    let blob_dir = data_root.join("blobs");

    // First open: create key and store blob
    {
        let config = BlobStoreConfig { blob_dir: blob_dir.clone() };
        let seal_fn = Arc::new(|key: &[u8; 32]| Ok(key.to_vec()));
        let unseal_fn = Arc::new(|data: &[u8]| -> Result<[u8; 32], StorageError> {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(data);
            Ok(arr)
        });
        let key_source = BlobKeySource::HardwareSealed { unseal_fn, seal_fn };
        let store = BlobStore::open(&config, key_source).unwrap();
        store.store(b"persistent hw blob", &[]).unwrap();
    }

    // Second open: should unseal existing key and decrypt
    {
        let config = BlobStoreConfig { blob_dir };
        let seal_fn = Arc::new(|key: &[u8; 32]| Ok(key.to_vec()));
        let unseal_fn = Arc::new(|data: &[u8]| -> Result<[u8; 32], StorageError> {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(data);
            Ok(arr)
        });
        let key_source = BlobKeySource::HardwareSealed { unseal_fn, seal_fn };
        let store = BlobStore::open(&config, key_source).unwrap();

        let blob_id = edgerun_core::util::bytes_to_hex(&edgerun_core::crypto::sha256(b"persistent hw blob"));
        let entry = store.load(&blob_id).unwrap().unwrap();
        let decrypted = store.decrypt(&entry.nonce, &entry.ciphertext).unwrap();
        assert_eq!(decrypted, b"persistent hw blob");
    }
}

// ============================================================================
// CredentialStore Standalone Tests
// ============================================================================

#[test]
fn credential_store_put_get_delete() {
    let data_root = tmp_data_root();
    let cs = make_credential_store(&data_root);

    cs.put("ns", "cred1", b"secret-value", Some("A secret")).unwrap();
    let val = cs.get("ns", "cred1").unwrap().unwrap();
    assert_eq!(val, b"secret-value");

    let deleted = cs.delete("ns", "cred1").unwrap();
    assert!(deleted);
    assert!(cs.get("ns", "cred1").unwrap().is_none());
}

#[test]
fn credential_store_list() {
    let data_root = tmp_data_root();
    let cs = make_credential_store(&data_root);

    cs.put("api", "github", b"ghp_xxx", None).unwrap();
    cs.put("api", "stripe", b"sk_yyy", Some("Stripe key")).unwrap();
    cs.put("db", "postgres", b"pass", None).unwrap();

    let api_creds = cs.list("api").unwrap();
    assert_eq!(api_creds.len(), 2);

    let namespaces = cs.list_namespaces().unwrap();
    assert_eq!(namespaces.len(), 2);
}

#[test]
fn credential_store_exists() {
    let data_root = tmp_data_root();
    let cs = make_credential_store(&data_root);

    cs.put("ns", "cred", b"val", None).unwrap();
    assert!(cs.exists("ns", "cred").unwrap());
    assert!(!cs.exists("ns", "missing").unwrap());
    assert!(!cs.exists("missing", "cred").unwrap());
}

#[test]
fn credential_store_rotate() {
    let data_root = tmp_data_root();
    let cs = make_credential_store(&data_root);

    cs.put("ns", "api-key", b"old-key", None).unwrap();
    cs.rotate("ns", "api-key", b"new-key", Some("rotated")).unwrap();

    let val = cs.get("ns", "api-key").unwrap().unwrap();
    assert_eq!(val, b"new-key");
}

#[test]
fn credential_store_encrypted_at_rest() {
    let data_root = tmp_data_root();
    let cs = make_credential_store(&data_root);

    cs.put("ns", "secret", b"plaintext-secret", None).unwrap();

    // Verify by checking the blob files don't contain plaintext
    let blob_dir = data_root.join("blobs");
    for entry in fs::read_dir(blob_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().map_or(false, |e| e == "blob") {
            let content = fs::read(&path).unwrap();
            // The plaintext should not appear verbatim in the blob file (first 12 bytes are nonce)
            if content.len() > 12 {
                assert!(!content[12..].windows(16).any(|w| w == b"plaintext-secret"),
                    "Found plaintext in blob file: {:?}", path);
            }
        }
    }
}

#[test]
fn credential_store_edge_cases() {
    let data_root = tmp_data_root();
    let cs = make_credential_store(&data_root);

    // Empty secret
    cs.put("ns", "empty", b"", None).unwrap();
    assert_eq!(cs.get("ns", "empty").unwrap().unwrap(), b"");

    // Large secret (50KB)
    let large = vec![0xCCu8; 50_000];
    cs.put("ns", "large", &large, None).unwrap();
    assert_eq!(cs.get("ns", "large").unwrap().unwrap(), large);

    // Special characters in name
    cs.put("ns", "key-with-dashes", b"v1", None).unwrap();
    cs.put("ns", "key.with.dots", b"v2", None).unwrap();
    assert_eq!(cs.get("ns", "key-with-dashes").unwrap().unwrap(), b"v1");
    assert_eq!(cs.get("ns", "key.with.dots").unwrap().unwrap(), b"v2");

    // UTF-8 secret
    let utf8_secret = "🔐 Unicode secret 🔑".as_bytes();
    cs.put("ns", "utf8", utf8_secret, Some("Unicode test")).unwrap();
    assert_eq!(cs.get("ns", "utf8").unwrap().unwrap(), utf8_secret);

    // Overwrite
    cs.put("ns", "overwrite", b"old", None).unwrap();
    cs.put("ns", "overwrite", b"new", Some("updated")).unwrap();
    assert_eq!(cs.get("ns", "overwrite").unwrap().unwrap(), b"new");
}

#[test]
fn credential_store_restart() {
    let data_root = tmp_data_root();

    {
        let cs = make_credential_store(&data_root);
        cs.put("persistent", "key", b"persistent-value", None).unwrap();
        assert!(cs.get("persistent", "key").unwrap().is_some());
    }

    // Credential index is in-memory only — not persisted across restart.
    // The encrypted blob exists on disk but the name→blob_id mapping is lost.
    {
        let cs = make_credential_store(&data_root);
        assert!(cs.get("persistent", "key").unwrap().is_none());
    }
}

// ============================================================================
// NodeStore Gaps: Work Accounting, Peers, Fetch Queue
// ============================================================================

#[test]
fn nodestore_work_accounting() {
    let data_root = tmp_data_root();
    let config = test_config(data_root.clone());
    let store = NodeStore::open(&config).unwrap();

    let record = edgerun_core::accounting::WorkAccounting {
        work_id: [0x01; 32],
        requester_id: [0x11; 64],
        provider_id: [0x22; 64],
        delegation_hash: [0x33; 32],
        started_at_us: 1_000_000,
        completed_at_us: 2_000_000,
        physical_core_us: 400_000,
        physical_memory_gb: 4,
        memory_duration_seconds: 60,
        gpu_core_us: 0,
        npu_core_us: 0,
        storage_read_bytes: 0,
        storage_written_bytes: 0,
        storage_read_ops: 0,
        storage_write_ops: 0,
        network_sent_bytes: 0,
        network_received_bytes: 0,
        workload_class: edgerun_core::accounting::WorkloadClass::Inference,
        priority: edgerun_core::accounting::WorkPriority::Standard,
        status: edgerun_core::accounting::WorkStatus::Completed,
        exit_code: Some(0),
        provider_cert_digest: [0x44; 32],
        cpu_multiplier: edgerun_core::fixed_point::FixedPoint16::from_int(1),
        memory_multiplier: edgerun_core::fixed_point::FixedPoint16::from_int(1),
        storage_multiplier: edgerun_core::fixed_point::FixedPoint16::from_int(1),
        billable_compute_rc_us: 500_000,
    };
    store.record_work_accounting(&record).unwrap();

    let total = store.total_billable_for_requester(&[0x11; 64]).unwrap();
    assert_eq!(total, 500_000);

    let total = store.total_billable_for_provider(&[0x22; 64]).unwrap();
    assert_eq!(total, 500_000);

    let by_class = store.work_by_class("inference").unwrap();
    assert_eq!(by_class.len(), 1);

    let by_status = store.work_by_status("completed").unwrap();
    assert_eq!(by_status.len(), 1);
}

#[test]
fn nodestore_peer_operations() {
    let data_root = tmp_data_root();
    let config = test_config(data_root.clone());
    let store = NodeStore::open(&config).unwrap();

    store.upsert_peer("node-1", Some("10.0.0.1:9000"), "active", true).unwrap();
    store.upsert_peer("node-2", Some("10.0.0.2:9000"), "unreachable", false).unwrap();

    let peers = store.list_peers().unwrap();
    assert_eq!(peers.len(), 2);

    store.update_peer_status("node-2", "active").unwrap();
    let unreachable = store.list_unreachable_peers_with_addr().unwrap();
    assert!(unreachable.is_empty()); // both are active now
}

#[test]
fn nodestore_delegation_and_revocation() {
    let data_root = tmp_data_root();
    let config = test_config(data_root.clone());
    let store = NodeStore::open(&config).unwrap();

    store.store_delegation("del-1", "issuer-a", "recipient-b", "cap-c", None).unwrap();
    store.store_revocation("rev-1", "issuer-a", "delegation", "del-1", None).unwrap();

    let revocations = store.list_active_revocations().unwrap();
    assert_eq!(revocations.len(), 1);
    assert_eq!(revocations[0], ("delegation".to_string(), "del-1".to_string()));
}

// ============================================================================
// Edge Cases: Corruption, Large Data, Binary IDs
// ============================================================================

#[test]
fn nodestore_very_long_stream_id() {
    let data_root = tmp_data_root();
    let config = test_config(data_root.clone());
    let mut store = NodeStore::open(&config).unwrap();

    // Use a long but OS-acceptable stream ID (max filename ~255 bytes on most systems)
    // The hex encoding doubles the length, so keep it under ~100 bytes raw
    let long_id = "a".repeat(100);
    store.append_event(&EventEnvelope {
        envelope_version: 1,
        stream_id: long_id.as_bytes().to_vec(),
        seq: 0,
        prev_event_hash: None,
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
    }).unwrap();

    assert!(store.get_head(long_id.as_bytes()).unwrap().is_some());
}

#[test]
fn nodestore_many_streams() {
    let data_root = tmp_data_root();
    let config = test_config(data_root.clone());
    let mut store = NodeStore::open(&config).unwrap();

    // Create 100 streams with 10 events each
    for s in 0..100 {
        let stream = format!("stream-{}", s);
        for i in 0..10 {
            store.append_event(&test_event(&stream, i)).unwrap();
        }
    }

    let heads = store.list_stream_heads().unwrap();
    assert_eq!(heads.len(), 100);

    let ids = store.list_stream_ids().unwrap();
    assert_eq!(ids.len(), 100);

    // Verify a random stream
    let (seq, _) = store.get_head(b"stream-42").unwrap().unwrap();
    assert_eq!(seq, 9);
}

#[test]
fn nodestore_many_blobs() {
    let data_root = tmp_data_root();
    let config = test_config(data_root.clone());
    let mut store = NodeStore::open(&config).unwrap();

    for i in 0..100 {
        let content = format!("blob-content-{}", i);
        store.put_blob(content.as_bytes(), &[]).unwrap();
    }

    // Verify all blobs are retrievable
    for i in 0..100 {
        let content = format!("blob-content-{}", i);
        let blob_id = store.put_blob(content.as_bytes(), &[]).unwrap();
        let entry = store.get_blob(&blob_id).unwrap().unwrap();
        let decrypted = store.decrypt_blob(&entry.nonce, &entry.ciphertext).unwrap();
        assert_eq!(decrypted, content.as_bytes());
    }
}

#[test]
fn nodestore_blob_with_many_recipients() {
    let data_root = tmp_data_root();
    let config = test_config(data_root.clone());
    let mut store = NodeStore::open(&config).unwrap();

    let recipients: Vec<Vec<u8>> = (0..50).map(|i| vec![i as u8; 64]).collect();
    let blob_id = store.put_blob(b"multi-recipient", &recipients).unwrap();
    let entry = store.get_blob(&blob_id).unwrap().unwrap();
    assert_eq!(entry.recipients.len(), 50);
    assert_eq!(entry.recipients[0], vec![0u8; 64]);
    assert_eq!(entry.recipients[49], vec![49u8; 64]);
}

#[test]
fn nodestore_credential_special_characters() {
    let data_root = tmp_data_root();
    let config = test_config(data_root.clone());
    let store = NodeStore::open(&config).unwrap();

    store.credentials().put("ns", "my-credential_2024", b"val", None).unwrap();
    let val = store.credentials().get("ns", "my-credential_2024").unwrap().unwrap();
    assert_eq!(val, b"val");
}

#[test]
fn nodestore_credential_empty_secret() {
    let data_root = tmp_data_root();
    let config = test_config(data_root.clone());
    let store = NodeStore::open(&config).unwrap();

    store.credentials().put("ns", "empty", b"", None).unwrap();
    let val = store.credentials().get("ns", "empty").unwrap().unwrap();
    assert_eq!(val, b"");
}

#[test]
fn nodestore_credential_large_secret() {
    let data_root = tmp_data_root();
    let config = test_config(data_root.clone());
    let store = NodeStore::open(&config).unwrap();

    let large_secret = vec![0xABu8; 100_000];
    store.credentials().put("ns", "large", &large_secret, None).unwrap();
    let val = store.credentials().get("ns", "large").unwrap().unwrap();
    assert_eq!(val, large_secret);
}

// ============================================================================
// StorageError Exhaustive Tests
// ============================================================================

#[test]
fn storage_error_all_variants_display() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
    let e_io = StorageError::Io(io_err);
    assert!(e_io.to_string().contains("I/O error"));
    assert!(e_io.to_string().contains("not found"));

    let e_enc = StorageError::Encode("encode failed".into());
    assert_eq!(e_enc.to_string(), "encode error: encode failed");

    let e_dec = StorageError::Decode("decode failed".into());
    assert_eq!(e_dec.to_string(), "decode error: decode failed");

    let e_encrypt = StorageError::Encryption("encrypt failed".into());
    assert_eq!(e_encrypt.to_string(), "encryption error: encrypt failed");

    let e_decrypt = StorageError::Decryption("decrypt failed".into());
    assert_eq!(e_decrypt.to_string(), "decryption error: decrypt failed");
}

#[test]
fn storage_error_std_error_trait() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied");
    let e = StorageError::Io(io_err);
    let boxed: Box<dyn std::error::Error> = Box::new(e);
    assert!(boxed.source().is_some());

    let e2 = StorageError::Encode("test".into());
    let boxed2: Box<dyn std::error::Error> = Box::new(e2);
    assert!(boxed2.source().is_none());
}

#[test]
fn storage_error_from_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broken");
    let e: StorageError = io_err.into();
    assert!(matches!(e, StorageError::Io(_)));
}

// ============================================================================
// Event Log: Multiple Streams, Gaps, Ordering
// ============================================================================

#[test]
fn event_log_interleaved_streams() {
    let data_root = tmp_data_root();
    let config = test_config(data_root.clone());
    let mut store = NodeStore::open(&config).unwrap();

    // Interleave events from 3 streams
    store.append_event(&test_event("alpha", 0)).unwrap();
    store.append_event(&test_event("beta", 0)).unwrap();
    store.append_event(&test_event("gamma", 0)).unwrap();
    store.append_event(&test_event("alpha", 1)).unwrap();
    store.append_event(&test_event("beta", 1)).unwrap();
    store.append_event(&test_event("gamma", 1)).unwrap();

    // Each stream should have independent seq tracking
    assert_eq!(store.get_head(b"alpha").unwrap().unwrap().0, 1);
    assert_eq!(store.get_head(b"beta").unwrap().unwrap().0, 1);
    assert_eq!(store.get_head(b"gamma").unwrap().unwrap().0, 1);

    // All events retrievable
    for stream in &["alpha", "beta", "gamma"] {
        for i in 0..2 {
            let event = store.get_event(stream.as_bytes(), i).unwrap().unwrap();
            assert_eq!(event.seq, i);
        }
    }
}

#[test]
fn event_log_gap_in_sequence() {
    let data_root = tmp_data_root();
    let config = test_config(data_root.clone());
    let mut store = NodeStore::open(&config).unwrap();

    store.append_event(&test_event("gap-stream", 0)).unwrap();
    // Skip seq 1-4
    store.append_event(&test_event("gap-stream", 5)).unwrap();

    // Gap should be readable as missing
    assert!(store.get_event(b"gap-stream", 1).unwrap().is_none());
    assert!(store.get_event(b"gap-stream", 2).unwrap().is_none());
    assert!(store.get_event(b"gap-stream", 3).unwrap().is_none());
    assert!(store.get_event(b"gap-stream", 4).unwrap().is_none());
    assert!(store.get_event(b"gap-stream", 5).unwrap().is_some());
}

// ============================================================================
// ControllerSet Exhaustive
// ============================================================================

#[test]
fn controller_set_exhaustive() {
    use edgerun_storage::ControllerSet;

    // Empty set
    let mut set = ControllerSet::new(vec![]);
    assert!(set.to_vec().is_empty());

    // Add
    set.add(vec![1]);
    assert!(set.contains(&vec![1]));
    assert!(!set.contains(&vec![2]));

    // Duplicate add
    set.add(vec![1]);
    assert_eq!(set.to_vec().len(), 1);

    // Remove existing
    assert!(set.remove(&vec![1]));
    assert!(!set.contains(&vec![1]));

    // Remove nonexistent
    assert!(!set.remove(&vec![1]));

    // Add multiple
    set.add(vec![1]);
    set.add(vec![2]);
    set.add(vec![3]);
    let controllers = set.to_vec();
    assert_eq!(controllers.len(), 3);
}

// ============================================================================
// ObjectResult
// ============================================================================

#[test]
fn object_result_all_fields() {
    use edgerun_storage::ObjectResult;
    let result = ObjectResult {
        object_id: vec![0x01, 0x02, 0x03],
        object_kind: 42,
        content: b"test content".to_vec(),
    };
    assert_eq!(result.object_id, vec![0x01, 0x02, 0x03]);
    assert_eq!(result.object_kind, 42);
    assert_eq!(result.content, b"test content");
}
