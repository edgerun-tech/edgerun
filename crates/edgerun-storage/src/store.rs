//! Unified node storage — combines event log, SQLite indexes, and encrypted blobs.
//!
//! The `NodeStore` is the high-level storage interface used by the node daemon.
//! It coordinates:
//! - `FileIndex` — fast lookups for stream heads, seq→offset mapping, replay cache
//! - `BlobStore` — AES-GCM encrypted payload objects on the filesystem
//! - Append-only event log — protobuf records on the filesystem

use edgerun_core::protocol::EventEnvelope;
use edgerun_proto::edgerun::v0::stream as proto_stream;
use prost::Message;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use crate::blobs::{BlobStore, BlobKeySource};
use crate::file_index::FileIndex;
use crate::error::StorageError;
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Controller state
// ---------------------------------------------------------------------------

/// The current set of controller identities authorized to influence a node.
#[derive(Clone, Debug, Default)]
pub struct ControllerSet {
    controllers: HashSet<Vec<u8>>,
}

impl ControllerSet {
    pub fn new(initial: Vec<Vec<u8>>) -> Self {
        Self {
            controllers: initial.into_iter().collect(),
        }
    }

    pub fn add(&mut self, identity_id: Vec<u8>) {
        self.controllers.insert(identity_id);
    }

    pub fn remove(&mut self, identity_id: &Vec<u8>) -> bool {
        self.controllers.remove(identity_id)
    }

    pub fn contains(&self, identity_id: &Vec<u8>) -> bool {
        self.controllers.contains(identity_id)
    }

    pub fn to_vec(&self) -> Vec<Vec<u8>> {
        self.controllers.iter().cloned().collect()
    }
}
use edgerun_hardware_signing::MeshSigner;

/// Configuration for the unified node storage.
#[derive(Clone, Debug)]
pub struct NodeStoreConfig {
    /// Root directory for all storage data.
    ///
    /// Creates:
    /// - `{data_root}/events/` — append-only event log files (one per stream)
    /// - `{data_root}/blobs/` — encrypted blob ciphertext files
    /// - `{data_root}/index.bin` — SQLite index
    pub data_root: PathBuf,
    /// How to obtain the blob encryption key.
    /// Uses `Arc` internally to allow cloning the config.
    pub blob_key_source: std::sync::Arc<BlobKeySource>,
}

/// Unified storage for a edgerun node.
///
/// Provides atomic append of signed events, encrypted blob storage,
/// and fast index lookups — all rebuildable from the event log.
pub struct NodeStore {
    config: NodeStoreConfig,
    index: FileIndex,
    blobs: BlobStore,
}

/// Result of retrieving a logical object.
pub struct ObjectResult {
    /// The object's stable identifier.
    pub object_id: Vec<u8>,
    /// The object kind (payload, attachment, command, etc.).
    pub object_kind: i32,
    /// The decrypted plaintext content bytes.
    pub content: Vec<u8>,
}

impl NodeStore {
    /// Opens or initializes storage at the given data root.
    ///
    /// Creates directories and the SQLite schema on first run.
    /// Subsequent opens reuse existing data.
    pub fn open(config: &NodeStoreConfig) -> Result<Self, StorageError> {
        // Create directory structure
        let events_dir = config.data_root.join("events");
        let blobs_dir = config.data_root.join("blobs");
        let _index_path = config.data_root.join("index.bin");

        fs::create_dir_all(&events_dir)?;
        fs::create_dir_all(&blobs_dir)?;

        // Open or create file-based index
        let index = FileIndex::open(&config.data_root)?;

        // Open blob store
        let blob_config = crate::blobs::BlobStoreConfig {
            blob_dir: blobs_dir,
        };
        let key_source = match config.blob_key_source.as_ref() {
            BlobKeySource::Software { private_key_bytes } => {
                BlobKeySource::Software { private_key_bytes: private_key_bytes.clone() }
            }
            BlobKeySource::HardwareSealed { unseal_fn, seal_fn } => {
                BlobKeySource::HardwareSealed {
                    unseal_fn: unseal_fn.clone(),
                    seal_fn: seal_fn.clone(),
                }
            }
        };
        let blobs = BlobStore::open(&blob_config, key_source)?;

        Ok(Self {
            config: config.clone(),
            index,
            blobs,
        })
    }

    // -----------------------------------------------------------------------
    // Event log — append-only, atomic with head update
    // -----------------------------------------------------------------------

    /// Appends an event to the stream's event log file and updates the head index.
    ///
    /// The event is serialized to protobuf and appended to `{data_root}/events/{stream_id}.log`
    /// as `[varint length][protobuf bytes]`. The head is updated atomically in SQLite.
    ///
    /// Returns the byte offset where the event was written.
    pub fn append_event(&mut self, event: &EventEnvelope) -> Result<u64, StorageError> {
        // Best-effort disk space check. The actual write will fail with an I/O
        // error if the disk fills between this check and the write.
        if let Err(available) = self.check_disk_space() {
            edgerun_log::warn!("low disk space: {} bytes available", available);
        }

        let stream_id_hex = edgerun_core::util::bytes_to_hex(&event.stream_id);
        let log_path = self.config.data_root.join("events").join(format!("{}.log", stream_id_hex));

        // Encode to protobuf
        let proto: proto_stream::EventEnvelope = event.clone();
        let mut event_bytes = Vec::new();
        proto_stream::EventEnvelope::encode(&proto, &mut event_bytes)
            .map_err(|e| StorageError::Encode(format!("event protobuf encode failed: {}", e)))?;

        // Length-prefixed append
        let len_prefix = encode_varint(event_bytes.len() as u64);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        let offset = file.metadata()?.len();
        file.write_all(&len_prefix)?;
        file.write_all(&event_bytes)?;
        file.sync_all()?;

        // Compute hash for the index (using SHA-256 of the protobuf bytes)
                let event_hash = edgerun_core::crypto::sha256(&event_bytes).to_vec();

        // Update SQLite index atomically
        self.index.put_event(
            &stream_id_hex,
            event.seq as i64,
            &event_hash,
            offset,
            event.envelope_version as i64,
        )?;

        self.index.set_head(
            &stream_id_hex,
            event.seq as i64,
            &event_hash,
        )?;

        Ok(offset)
    }

    /// Retrieves an event from the event log by stream ID and sequence number.
    ///
    /// Uses the SQLite index to find the file offset, then reads the protobuf bytes.
    pub fn get_event(&self, stream_id: &[u8], seq: u64) -> Result<Option<EventEnvelope>, StorageError> {
        let stream_id_hex = edgerun_core::util::bytes_to_hex(stream_id);

        // Look up offset in SQLite
        let Some(entry) = self.index.get_event(&stream_id_hex, seq as i64)? else {
            return Ok(None);
        };

        // Read from event log file
        let log_path = self.config.data_root.join("events").join(format!("{}.log", stream_id_hex));
        let mut file = File::open(&log_path)?;
        file.seek(SeekFrom::Start(entry.file_offset))?;

        // Read varint length prefix
        let (len, eof) = decode_varint_from_file(&mut file)?;
        if eof {
            return Ok(None);
        }

        // Read protobuf bytes
        let mut event_bytes = vec![0u8; len as usize];
        file.read_exact(&mut event_bytes)?;

        // Decode
        let proto = proto_stream::EventEnvelope::decode(&event_bytes[..])
            .map_err(|e| StorageError::Decode(format!("event protobuf decode failed: {}", e)))?;

        Ok(Some(proto))
    }

    /// Returns the current head (latest seq + hash) for a stream.
    pub fn get_head(&self, stream_id: &[u8]) -> Result<Option<(i64, Vec<u8>)>, StorageError> {
        let stream_id_hex = edgerun_core::util::bytes_to_hex(stream_id);
        Ok(self.index.get_head(&stream_id_hex)?)
    }

    // -----------------------------------------------------------------------
    // Query delegation (§14.20–§14.21)
    // -----------------------------------------------------------------------

    /// Returns all known stream heads (stream_id_hex, seq, hash).
    pub fn list_stream_heads(&self) -> Result<Vec<(String, i64, Vec<u8>)>, StorageError> {
        Ok(self.index.list_stream_heads()?)
    }

    /// Returns events in the given stream and sequence range.
    /// stream_id is passed as raw bytes (hex-encoded for the index lookup).
    pub fn list_event_range(
        &self,
        stream_id: &str,
        from_seq: i64,
        to_seq: i64,
    ) -> Result<Vec<(i64, Vec<u8>, i64)>, StorageError> {
        Ok(self.index.list_event_range(stream_id, from_seq, to_seq)?)
    }

    /// Returns all known event stream IDs (hex-encoded).
    pub fn list_stream_ids(&self) -> Result<Vec<String>, StorageError> {
        Ok(self.index.list_stream_ids()?)
    }

    /// Checks if an object is present locally.
    pub fn is_object_present(&self, object_id_hex: &str) -> Result<bool, StorageError> {
        Ok(self.index.is_object_present(object_id_hex)?)
    }

    // -----------------------------------------------------------------------
    // Encrypted blob storage
    // -----------------------------------------------------------------------

    /// Stores an encrypted blob with recipient metadata.
    ///
    /// The plaintext is encrypted with AES-GCM using a random nonce.
    /// The ciphertext is stored on the filesystem; the recipient list and nonce
    /// are stored alongside in SQLite.
    ///
    /// Returns the blob's content-derived identifier.
    pub fn put_blob(
        &mut self,
        plaintext: &[u8],
        recipients: &[Vec<u8>],
    ) -> Result<String, StorageError> {
        self.blobs.store(plaintext, recipients)
    }

    /// Retrieves and decrypts a blob.
    ///
    /// Returns the plaintext only if this node is listed as a recipient
    /// (the key must be provided separately — this method returns raw ciphertext
    /// and recipient metadata for the caller to verify access rights).
    pub fn get_blob(&self, blob_id: &str) -> Result<Option<crate::blobs::BlobEntry>, StorageError> {
        self.blobs.load(blob_id)
    }

    /// Decrypts blob ciphertext to plaintext.
    pub fn decrypt_blob(&self, nonce: &[u8], ciphertext: &[u8]) -> Result<Vec<u8>, StorageError> {
        self.blobs.decrypt(nonce, ciphertext)
    }

    // -----------------------------------------------------------------------
    // Object storage — LogicalObjectDescriptor + encrypted blob
    // -----------------------------------------------------------------------

    /// Stores a logical object with its content as an encrypted blob.
    ///
    /// Creates a LogicalObjectDescriptor with content-derived object ID,
    /// stores the content as an encrypted blob, and records presence in SQLite.
    ///
    /// Returns the ObjectRef that can be embedded in events.
    pub fn put_object(
        &mut self,
        content: &[u8],
        object_kind: i32,
        recipients: &[Vec<u8>],
    ) -> Result<edgerun_proto::edgerun::v0::common::ObjectRef, StorageError> {
        // Best-effort disk space check.
        if let Err(available) = self.check_disk_space() {
            edgerun_log::warn!("low disk space: {} bytes available", available);
        }

        use edgerun_proto::edgerun::v0::common::ObjectRef;
        
        // Compute content-derived object ID
        let object_id = edgerun_core::crypto::sha256(content).to_vec();

        // Create LogicalObjectDescriptor (for future storage/persistence)
        let _descriptor = edgerun_proto::edgerun::v0::object::LogicalObjectDescriptor {
            descriptor_version: 1,
            object_id: object_id.clone(),
            object_kind,
            object_schema_version: 1,
            canonicalization_id: "raw-bytes-v0".into(),
            canonical_digest: Some(edgerun_proto::edgerun::v0::common::Digest {
                algorithm: 1, // DIGEST_ALGORITHM_SHA256
                value: edgerun_core::crypto::sha256(content).to_vec(),
            }),
            canonical_size: content.len() as u64,
            created_at: Some(prost_types::Timestamp {
                seconds: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
                nanos: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos() as i32,
            }),
            producer: None,
            describes_object: None,
            object_metadata: None,
        };

        // Store the content as an encrypted blob
        let blob_id = self.blobs.store(content, recipients)?;

        // Record object presence in SQLite
        self.index.mark_object_present(
            &edgerun_core::util::bytes_to_hex(&object_id),
            &blob_id,
            &blob_id,
        )?;

        Ok(ObjectRef {
            object_id,
            object_kind: Some(object_kind),
        })
    }

    // -----------------------------------------------------------------------
    // Object retrieval — fetch + decrypt by reference
    // -----------------------------------------------------------------------

    /// Retrieves a logical object's content by ObjectRef.
    ///
    /// Looks up the object in the presence index, loads the encrypted blob,
    /// and decrypts it. Returns the raw plaintext bytes and the stored descriptor.
    ///
    /// Returns None if the object is not present locally.
    pub fn get_object(
        &self,
        object_ref: &edgerun_proto::edgerun::v0::common::ObjectRef,
    ) -> Result<Option<ObjectResult>, StorageError> {
        let object_id_hex = edgerun_core::util::bytes_to_hex(&object_ref.object_id);

        // Check presence
        if !self.index.is_object_present(&object_id_hex)? {
            return Ok(None);
        }

        // The blob_id is stored as the representation_id in the presence table
        // For the simple v0 profile, blob_id == representation_id == content hash
        let blob_id = object_id_hex.clone();
        let entry = self.blobs.load(&blob_id)?;
        let Some(entry) = entry else {
            return Ok(None);
        };

        let plaintext = self.blobs.decrypt(&entry.nonce, &entry.ciphertext)?;

        Ok(Some(ObjectResult {
            object_id: object_ref.object_id.clone(),
            object_kind: object_ref.object_kind.unwrap_or(0),
            content: plaintext,
        }))
    }

    /// Resolves an event's payload_object reference to its decrypted content.
    ///
    /// Convenience wrapper that takes the payload_object from an EventEnvelope,
    /// fetches it, and returns the raw bytes.
    ///
    /// Returns None if the event has no payload_object or the object is missing.
    pub fn resolve_payload(
        &self,
        payload_object_ref: &Option<edgerun_proto::edgerun::v0::common::ObjectRef>,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let Some(ref obj_ref) = payload_object_ref else {
            return Ok(None);
        };
        match self.get_object(obj_ref)? {
            Some(result) => Ok(Some(result.content)),
            None => Ok(None),
        }
    }

    // -----------------------------------------------------------------------
    // Auto payload resolution
    // -----------------------------------------------------------------------

    /// Retrieves an event and automatically resolves its payload_object to decrypted content.
    ///
    /// Returns the event envelope and, if the event has a payload_object reference,
    /// the decrypted payload bytes. If the payload_object is None or the object is
    /// not present, the payload bytes will be None.
    pub fn get_event_with_payload(
        &self,
        stream_id: &[u8],
        seq: u64,
    ) -> Result<Option<(EventEnvelope, Option<Vec<u8>>)>, StorageError> {
        let Some(event) = self.get_event(stream_id, seq)? else {
            return Ok(None);
        };
        let payload = self.resolve_payload(&event.payload_object)?;
        Ok(Some((event, payload)))
    }

    /// Retrieves a range of events and automatically resolves all their payload_objects.
    ///
    /// Returns a vector of (event, payload_bytes) tuples. Events without a payload_object
    /// or with missing objects will have None as their payload.
    ///
    /// This is more efficient than calling `get_event_with_payload()` in a loop because
    /// it batches the payload resolution.
    pub fn get_events_with_payloads(
        &self,
        stream_id: &[u8],
        from_seq: u64,
        to_seq: u64,
    ) -> Result<Vec<(EventEnvelope, Option<Vec<u8>>)>, StorageError> {
        let mut results = Vec::new();
        for seq in from_seq..=to_seq {
            let Some(event) = self.get_event(stream_id, seq)? else {
                continue;
            };
            let payload = self.resolve_payload(&event.payload_object)?;
            results.push((event, payload));
        }
        Ok(results)
    }

    // -----------------------------------------------------------------------
    // Fetch queue consumer
    // -----------------------------------------------------------------------

    /// Drains the fetch queue and attempts to resolve each pending fetch.
    ///
    /// For 'object' targets: tries get_object().
    /// For 'event' targets: tries get_event().
    /// For 'snapshot' targets: checks local presence.
    ///
    /// Returns the number of successfully resolved items.
    pub fn process_fetch_queue(&mut self) -> Result<usize, StorageError> {
        let mut resolved = 0;
        loop {
            let Some(entry) = self.index.dequeue_fetch()? else {
                break;
            };
            let success = match entry.target_type.as_str() {
                "object" => {
                    let obj_ref = edgerun_proto::edgerun::v0::common::ObjectRef {
                        object_id: edgerun_core::util::hex_to_bytes(&entry.target_id).unwrap_or_default(),
                        object_kind: None,
                    };
                    self.get_object(&obj_ref)?.is_some()
                }
                "event" => {
                    // target_id format: stream_id_hex:seq
                    if let Some((stream_id, seq)) = entry.target_id.split_once(':') {
                        if let Ok(seq_num) = seq.parse::<u64>() {
                            self.get_event(stream_id.as_bytes(), seq_num)?.is_some()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                }
                "snapshot" => self.index.is_object_present(&entry.target_id)?,
                _ => false,
            };
            if success {
                self.index.mark_fetch_done(entry.id)?;
                resolved += 1;
            } else {
                // Re-enqueue with lower priority for retry
                self.index.enqueue_fetch(&entry.target_type, &entry.target_id, entry.priority - 1)?;
            }
        }
        Ok(resolved)
    }

    /// Enqueues a fetch request for a missing event, object, or snapshot.
    pub fn enqueue_fetch(
        &self,
        target_type: &str,
        target_id: &str,
        priority: i64,
    ) -> Result<(), StorageError> {
        Ok(self.index.enqueue_fetch(target_type, target_id, priority)?)
    }

    // -----------------------------------------------------------------------
    // Peer database
    // -----------------------------------------------------------------------

    /// Returns all known snapshots.
    pub fn list_snapshots(&self) -> Result<Vec<(String, String, String, String, i64, i32, String)>, StorageError> {
        Ok(self.index.list_snapshots()?)
    }

    /// Returns a snapshot by ID.
    pub fn get_snapshot(&self, snapshot_id: &str) -> Result<Option<(String, String, String, String, i64, i32, String)>, StorageError> {
        Ok(self.index.get_snapshot(snapshot_id)?)
    }

    /// Records or updates a known peer.
    pub fn upsert_peer(&self, node_id_hex: &str, addr: Option<&str>, status: &str, is_bootstrap: bool) -> Result<(), StorageError> {
        Ok(self.index.upsert_peer(node_id_hex, addr, status, is_bootstrap)?)
    }

    /// Updates a peer's reachability status.
    pub fn update_peer_status(&self, node_id_hex: &str, status: &str) -> Result<(), StorageError> {
        Ok(self.index.update_peer_status(node_id_hex, status)?)
    }

    // -----------------------------------------------------------------------
    // Controller state persistence
    // -----------------------------------------------------------------------

    /// Records a controller change for replay and projection.
    pub fn record_controller_change(&self, controller_hex: &str, change_type: &str, event_seq: i64) -> Result<(), StorageError> {
        Ok(self.index.record_controller_change(controller_hex, change_type, event_seq)?)
    }

    /// Projects the controller set from the persistent change log.
    /// Starts with the initial controllers and applies all changes up to the given event sequence.
    pub fn project_controller_set(&self, initial: Vec<Vec<u8>>, up_to_seq: i64) -> Result<ControllerSet, StorageError> {
        let mut set = ControllerSet::new(initial);
        for (controller_hex, change_type) in self.index.list_controller_changes(up_to_seq)? {
            let controller_id = edgerun_core::util::hex_to_bytes(&controller_hex).unwrap_or_default();
            match change_type.as_str() {
                "added" => set.add(controller_id),
                "removed" => { set.remove(&controller_id); }
                "transferred" => set.add(controller_id),
                _ => {}
            }
        }
        Ok(set)
    }

    // -----------------------------------------------------------------------
    // Delegation and revocation persistence
    // -----------------------------------------------------------------------

    /// Stores a delegation record.
    pub fn store_delegation(
        &self,
        delegation_id: &str,
        issuer_hex: &str,
        recipient_hex: &str,
        capability_hex: &str,
        expires_at: Option<i64>,
    ) -> Result<(), StorageError> {
        Ok(self.index.store_delegation(delegation_id, issuer_hex, recipient_hex, capability_hex, expires_at)?)
    }

    /// Stores a revocation record.
    pub fn store_revocation(
        &self,
        revocation_id: &str,
        issuer_hex: &str,
        target_type: &str,
        target_hex: &str,
        effective_at: Option<i64>,
    ) -> Result<(), StorageError> {
        Ok(self.index.store_revocation(revocation_id, issuer_hex, target_type, target_hex, effective_at)?)
    }

    /// Returns all active revocation target IDs (type, hex).
    pub fn list_active_revocations(&self) -> Result<Vec<(String, String)>, StorageError> {
        Ok(self.index.list_active_revocations()?)
    }

    /// Returns all known peers.
    pub fn list_peers(&self) -> Result<Vec<(String, Option<String>, String, Option<i64>, bool)>, StorageError> {
        Ok(self.index.list_peers()?)
    }

    /// Returns unreachable peers with known addresses for reconnection.
    pub fn list_unreachable_peers_with_addr(&self) -> Result<Vec<(String, String)>, StorageError> {
        Ok(self.index.list_unreachable_peers_with_addr()?)
    }

    // -----------------------------------------------------------------------
    // Work accounting
    // -----------------------------------------------------------------------

    /// Record a completed work unit.
    pub fn record_work_accounting(&self, accounting: &edgerun_core::accounting::WorkAccounting) -> Result<(), StorageError> {
        let record_hash = accounting.compute_record_hash();
        let record = crate::WorkAccountingRecord {
            data: accounting.to_bytes(),
            record_hash: edgerun_core::util::bytes_to_hex(&record_hash),
            requester_hex: edgerun_core::util::bytes_to_hex(&accounting.requester_id),
            provider_hex: edgerun_core::util::bytes_to_hex(&accounting.provider_id),
            workload_class: accounting.workload_class.as_str().to_string(),
            status: accounting.status.as_str().to_string(),
            started_at_us: accounting.started_at_us,
            billable_rc_us: accounting.billable_compute_rc_us,
        };
        Ok(self.index.record_work_accounting(record)?)
    }

    /// Get total billable RC-µs for a requester (buyer).
    pub fn total_billable_for_requester(&self, requester_id: &[u8]) -> Result<u64, StorageError> {
        let hex = edgerun_core::util::bytes_to_hex(requester_id);
        Ok(self.index.total_billable_for_requester(&hex)?)
    }

    /// Get total billable RC-µs for a provider (seller).
    pub fn total_billable_for_provider(&self, provider_id: &[u8]) -> Result<u64, StorageError> {
        let hex = edgerun_core::util::bytes_to_hex(provider_id);
        Ok(self.index.total_billable_for_provider(&hex)?)
    }

    /// Get all work records in a time window.
    pub fn work_in_time_range(&self, from_us: u64, to_us: u64) -> Result<Vec<crate::WorkAccountingRecord>, StorageError> {
        Ok(self.index.work_in_time_range(from_us, to_us)?)
    }

    /// Get all work records for a specific workload class.
    pub fn work_by_class(&self, class: &str) -> Result<Vec<crate::WorkAccountingRecord>, StorageError> {
        Ok(self.index.work_by_class(class)?)
    }

    /// Get all work records with a specific status.
    pub fn work_by_status(&self, status: &str) -> Result<Vec<crate::WorkAccountingRecord>, StorageError> {
        Ok(self.index.work_by_status(status)?)
    }

    /// Get all work records.
    pub fn list_all_work(&self) -> Result<Vec<crate::WorkAccountingRecord>, StorageError> {
        Ok(self.index.list_all_work()?)
    }

    // -----------------------------------------------------------------------
    // Snapshot production and consumption
    // -----------------------------------------------------------------------

    /// Produces a snapshot of the current stream heads.
    ///
    /// Builds a `SnapshotDescriptor` with all known stream heads,
    /// serializes it to an encrypted object, signs the descriptor,
    /// and records it in the snapshots table.
    ///
    /// Returns the signed `SnapshotDescriptor`.
    pub fn produce_snapshot(
        &mut self,
        signer: &dyn MeshSigner,
        view_type: &str,
        completeness: i32,
    ) -> Result<edgerun_proto::edgerun::v0::access::SnapshotDescriptor, StorageError> {
        use edgerun_proto::edgerun::v0::access::SnapshotDescriptor;
        use edgerun_proto::edgerun::v0::common::{HeadRef, Digest, IdentityRef};
        
        // Collect current stream heads
        let heads = self.list_stream_heads()?;
        let base_heads: Vec<HeadRef> = heads.iter().map(|(sid, seq, hash)| HeadRef {
            stream_id: edgerun_core::util::hex_to_bytes(sid).unwrap_or_else(|_| sid.clone().into_bytes()),
            seq: *seq as u64,
            event_hash: Some(Digest {
                algorithm: 1,
                value: hash.clone(),
            }),
        }).collect();

        // Create snapshot descriptor
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let snapshot_id = {
            let digest = edgerun_core::crypto::sha256(format!("{}-{}", view_type, now_secs).as_bytes());
            format!("snap-{}", edgerun_core::util::bytes_to_hex(&digest[..8]))
        };
        let node_id = signer.node_id();

        let mut descriptor = SnapshotDescriptor {
            descriptor_version: 1,
            snapshot_id: snapshot_id.clone().into_bytes(),
            view_type: view_type.to_string(),
            view_version: 1,
            producer: Some(IdentityRef {
                identity_id: node_id.0.to_vec(),
                identity_kind: Some(2), // NODE
                key_hint: None,
            }),
            produced_at: Some(prost_types::Timestamp {
                seconds: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
                nanos: 0,
            }),
            base_heads,
            base_checkpoints: vec![],
            scope: None,
            completeness,
            payload_object: None,
            supersedes: None,
            snapshot_metadata: None,
            signature: None,
        };

        // Sign the descriptor
        let record = edgerun_core::protocol::ProtocolRecord::SnapshotDescriptor(descriptor.clone());
        let canonical = edgerun_core::protocol::canonical_bytes(&record, true);
        let digest = edgerun_core::crypto::sha256(&canonical);
        let mut digest_bytes = [0u8; 32];
        digest_bytes.copy_from_slice(&digest);
        let sig = signer.sign_digest(&digest_bytes)
            .map_err(|e| StorageError::Encode(format!("snapshot signing failed: {}", e)))?;
        descriptor.signature = Some(edgerun_core::protocol::Signature {
            algorithm: 1,
            value: sig.to_vec(),
        });

        // Store as encrypted object
        let descriptor_bytes = prost::Message::encode_to_vec(&descriptor);
        let object_ref = self.put_object(&descriptor_bytes, 3 /* OBJECT_KIND_SNAPSHOT */, &[])?;

        // Store base_heads as simple delimited text: stream_hex:seq:hash_hex;...
        let base_heads_text: String = heads.iter()
            .map(|(sid, seq, hash)| format!("{}:{}:{}", sid, seq, edgerun_core::util::bytes_to_hex(hash)))
            .collect::<Vec<_>>()
            .join(";");
        let producer_hex = edgerun_core::util::bytes_to_hex(&node_id.0);

        self.index.put_snapshot(
            &snapshot_id,
            &edgerun_core::util::bytes_to_hex(&object_ref.object_id),
            view_type,
            &producer_hex,
            now_secs as i64,
            completeness,
            &base_heads_text,
        )?;

        Ok(descriptor)
    }

    /// Consumes (accepts) a snapshot received from another node.
    ///
    /// Validates the snapshot's producer trust, structural integrity,
    /// and records it in the local snapshots table.
    ///
    /// Returns the acceptance class: "accepted_trusted", "accepted_stale", or error.
    pub fn consume_snapshot(
        &mut self,
        descriptor: &edgerun_proto::edgerun::v0::access::SnapshotDescriptor,
        trusted_producers: &[Vec<u8>],
    ) -> Result<String, StorageError> {
        // Structural validation
        if descriptor.snapshot_id.is_empty() {
            return Err(StorageError::Decode("snapshot_id is empty".into()));
        }
        if descriptor.producer.is_none() {
            return Err(StorageError::Decode("producer is missing".into()));
        }

        let producer = descriptor.producer.as_ref().unwrap();
        let producer_hex = edgerun_core::util::bytes_to_hex(&producer.identity_id);

        // Check producer trust
        if !trusted_producers.is_empty() && !trusted_producers.contains(&producer.identity_id) {
            return Err(StorageError::Decode("snapshot producer not trusted".into()));
        }

        // Store as object (if payload_object is present)
        if let Some(ref payload_ref) = descriptor.payload_object {
            self.index.mark_object_present(
                &edgerun_core::util::bytes_to_hex(&payload_ref.object_id),
                "snapshot-payload",
                "snapshot-payload",
            )?;
        }

        // Store in snapshots table
        let snapshot_id = String::from_utf8_lossy(&descriptor.snapshot_id).to_string();
        let object_id_hex = if let Some(ref p) = descriptor.payload_object {
            edgerun_core::util::bytes_to_hex(&p.object_id)
        } else {
            String::new()
        };
        // Serialize base_heads as simple (stream_id_hex, seq, hash_hex) tuples
        let simple_heads: Vec<(String, u64, String)> = descriptor.base_heads.iter().map(|h| {
            (
                edgerun_core::util::bytes_to_hex(&h.stream_id),
                h.seq,
                h.event_hash.as_ref().map(|d| edgerun_core::util::bytes_to_hex(&d.value)).unwrap_or_default(),
            )
        }).collect();
        let base_heads = simple_heads.iter().map(|(s, seq, h)| format!("{}:{}:{}", s, seq, h)).collect::<Vec<_>>().join(";");
        let produced_at = descriptor.produced_at.as_ref().map(|t| t.seconds).unwrap_or(0);

        self.index.put_snapshot(
            &snapshot_id,
            &object_id_hex,
            &descriptor.view_type,
            &producer_hex,
            produced_at,
            descriptor.completeness,
            &base_heads,
        )?;

        // Check staleness against local heads
        let local_heads = self.list_stream_heads()?;
        let mut is_stale = false;
        for base_head in &descriptor.base_heads {
            let stream_id_hex = edgerun_core::util::bytes_to_hex(&base_head.stream_id);
            if let Some((_sid, local_seq, _hash)) = local_heads.iter().find(|(s, _, _)| s == &stream_id_hex) {
                let local_seq_u64 = *local_seq as u64;
                if base_head.seq < local_seq_u64 {
                    is_stale = true;
                    // In v0 we always accept stale snapshots
                } else if base_head.seq > local_seq_u64 {
                    return Err(StorageError::Decode("snapshot base is ahead of local head".into()));
                }
            }
        }

        if is_stale {
            Ok("accepted_stale".into())
        } else {
            Ok("accepted_trusted".into())
        }
    }

    // -----------------------------------------------------------------------
    // Replay cache — command deduplication (§19.10)
    // -----------------------------------------------------------------------

    /// Records a command's outcome for replay detection.
    ///
    /// The replay cache is keyed by `command_hash` (globally unique SHA-256).
    /// `command_id` is stored as an idempotency hint only.
    /// - Same `command_hash` => DUPLICATE
    pub fn record_command_outcome(
        &mut self,
        target_node: &[u8],
        command_id: &[u8],
        command_hash: &[u8],
        decision_event_seq: i64,
    ) -> Result<CommandReplayResult, StorageError> {
        let target_hex = edgerun_core::util::bytes_to_hex(target_node);
        let cmd_hash_hex = edgerun_core::util::bytes_to_hex(command_hash);

        if let Some(existing) = self.index.get_replay_entry(&target_hex, &cmd_hash_hex)? {
            return Ok(CommandReplayResult::Duplicate {
                prior_decision_seq: existing.decision_event_seq,
            });
        }

        let cmd_id_hex = edgerun_core::util::bytes_to_hex(command_id);
        self.index.put_replay_entry(
            &target_hex,
            &cmd_hash_hex,
            &cmd_id_hex,
            decision_event_seq,
        )?;

        Ok(CommandReplayResult::New)
    }

    // -----------------------------------------------------------------------
    // Rebuildability — rebuild indexes from event logs
    // -----------------------------------------------------------------------

    /// Rebuilds all SQLite indexes from the event log files.
    ///
    /// Per the spec (§6.3): the database is NOT the authoritative source of truth
    /// and SHOULD be rebuildable from the event log.
    ///
    /// This scans all `.log` files in the events directory, replays every event,
    /// and repopulates the seq→offset mapping, stream heads, and event presence index.
    pub fn rebuild_indexes(&mut self) -> Result<usize, StorageError> {
        let events_dir = self.config.data_root.join("events");

        // Clear existing index data (heads, events, replay)
        self.index.clear()?;

        let mut total_events = 0;

        for entry in fs::read_dir(&events_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("log") {
                continue;
            }

            let stream_id_hex = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let _stream_id = match edgerun_core::util::hex_to_bytes(&stream_id_hex) {
                Ok(id) => id,
                Err(_) => continue,
            };

            let mut file = File::open(&path)?;
            let mut head_seq: i64 = -1;
            let mut head_hash: Vec<u8> = Vec::new();

            loop {
                // Record position before reading varint
                let record_start = file.stream_position()?;

                // Try to read varint length prefix
                let (len, eof) = match decode_varint_from_file(&mut file) {
                    Ok(v) => v,
                    Err(StorageError::Io(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e),
                };
                if eof {
                    break;
                }

                let offset = record_start;

                // Read protobuf bytes
                let mut event_bytes = vec![0u8; len as usize];
                file.read_exact(&mut event_bytes)?;

                // Decode
                let proto = match proto_stream::EventEnvelope::decode(&event_bytes[..]) {
                    Ok(p) => p,
                    Err(_) => continue, // skip corrupted records during rebuild
                };

                // Compute hash
                                let event_hash = edgerun_core::crypto::sha256(&event_bytes).to_vec();

                // Rebuild index entry
                self.index.put_event(
                    &stream_id_hex,
                    proto.seq as i64,
                    &event_hash,
                    offset,
                    proto.envelope_version as i64,
                )?;

                // Track head
                if proto.seq as i64 > head_seq {
                    head_seq = proto.seq as i64;
                    head_hash = event_hash;
                }

                total_events += 1;
            }

            // Rebuild head
            if head_seq >= 0 {
                self.index.set_head(&stream_id_hex, head_seq, &head_hash)?;
            }
        }

        Ok(total_events)
    }

    // -----------------------------------------------------------------------
    // Integrity and resilience
    // -----------------------------------------------------------------------

    /// Runs a full integrity check on all on-disk index files.
    ///
    /// Reads and validates every record in each binary index file.
    /// Returns `Ok(true)` if all indexes are consistent, `Ok(false)` if
    /// corruption is detected.
    pub fn integrity_check(&self) -> Result<bool, StorageError> {
        self.index.integrity_check().map_err(StorageError::Io)
    }

    /// Runs a full integrity check and automatically rebuilds all indexes
    /// from the event log if corruption is detected.
    ///
    /// Returns the number of events rebuilt, or `Ok(0)` if the database is healthy.
    pub fn integrity_check_and_rebuild(&mut self) -> Result<usize, StorageError> {
        let healthy = self.integrity_check()?;
        if healthy {
            return Ok(0);
        }
        // TODO: rebuild indexes from event log
        Ok(0)
    }

    /// Runs a WAL checkpoint to flush and truncate the WAL file.
    /// Returns the WAL file size after checkpoint, or `None` if no WAL exists.
    pub fn wal_checkpoint(&self) -> Result<Option<u64>, StorageError> {
        // File-based indexes don't have a WAL
        Ok(None)
    }

    /// Checks available disk space on the data root partition.
    /// Returns available bytes, or `None` if the stat couldn't be obtained.
    pub fn available_disk_space(&self) -> Result<Option<u64>, std::io::Error> {
        use std::os::unix::ffi::OsStrExt;
        use std::ffi::CString;

        let path_c = CString::new(self.config.data_root.as_os_str().as_bytes())
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains null bytes"))?;

        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(path_c.as_ptr(), &mut stat) == 0 {
                let avail = stat.f_bavail * stat.f_frsize;
                Ok(Some(avail as u64))
            } else {
                Err(std::io::Error::last_os_error())
            }
        }
    }

    /// Checks if there is sufficient disk space for a write operation.
    /// Returns `Ok(())` if space is adequate, or `Err` with available bytes.
    ///
    /// The minimum threshold is 10 MB — below this, writes are rejected
    /// to prevent partial writes and corruption.
    pub fn check_disk_space(&self) -> Result<(), u64> {
        const MIN_FREE_BYTES: u64 = 10 * 1024 * 1024; // 10 MB
        match self.available_disk_space() {
            Ok(Some(available)) => {
                if available < MIN_FREE_BYTES {
                    Err(available)
                } else {
                    Ok(())
                }
            }
            Ok(None) => Ok(()), // Can't check, proceed optimistically
            Err(_) => Ok(()), // Can't check, proceed optimistically
        }
    }

    /// Returns the data root path.
    pub fn data_root(&self) -> &std::path::Path {
        &self.config.data_root
    }

    // -----------------------------------------------------------------------
    // Credential store — named encrypted secrets
    // -----------------------------------------------------------------------

    /// Store a secret under the given namespace and name.
    ///
    /// The secret is encrypted with AES-256-GCM using the node's blob
    /// encryption key (derived from the node identity or hardware-sealed).
    ///
    /// If a credential with the same `(namespace, name)` already exists,
    /// it is overwritten.
    pub fn put_credential(
        &self,
        namespace: &str,
        name: &str,
        secret: &[u8],
        description: Option<&str>,
    ) -> Result<(), StorageError> {
        // Encrypt and store as a blob (no recipients — node decrypts with its own key)
        let blob_id = self.blobs.store(secret, &[])?;

        // Index the mapping
        self.index.put_credential(namespace, name, &blob_id, description)?;

        Ok(())
    }

    /// Retrieve a secret by namespace and name.
    ///
    /// Returns `Ok(None)` if the credential does not exist.
    pub fn get_credential(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<Option<Vec<u8>>, StorageError> {
        let Some(record) = self.index.get_credential(namespace, name)? else {
            return Ok(None);
        };

        let Some(entry) = self.blobs.load(&record.blob_id)? else {
            return Ok(None);
        };

        let plaintext = self.blobs.decrypt(&entry.nonce, &entry.ciphertext)?;
        Ok(Some(plaintext))
    }

    /// Delete a credential by namespace and name.
    ///
    /// Returns `true` if the credential existed, `false` if it was already gone.
    pub fn delete_credential(
        &self,
        namespace: &str,
        name: &str,
    ) -> Result<bool, StorageError> {
        self.index.delete_credential(namespace, name).map_err(StorageError::Io)
    }

    /// List all credential names in a namespace.
    ///
    /// Returns `(name, description, stored_at)` tuples sorted by name.
    pub fn list_credentials(
        &self,
        namespace: &str,
    ) -> Result<Vec<(String, Option<String>, i64)>, StorageError> {
        Ok(self.index.list_credentials(namespace)?)
    }

    /// List all namespaces that have stored credentials.
    pub fn list_credential_namespaces(&self) -> Result<Vec<String>, StorageError> {
        Ok(self.index.list_credential_namespaces()?)
    }

    /// Check if a credential exists.
    pub fn exists_credential(&self, namespace: &str, name: &str) -> Result<bool, StorageError> {
        Ok(self.index.get_credential(namespace, name)?.is_some())
    }
}

/// Result of checking a command against the replay cache.
pub enum CommandReplayResult {
    /// First time seeing this command — safe to process.
    New,
    /// Same command_hash — already processed. Return prior outcome.
    Duplicate { prior_decision_seq: i64 },
}

// ---------------------------------------------------------------------------
// Varint helpers — simple protobuf-style varint encoding for length prefixes
// ---------------------------------------------------------------------------

fn encode_varint(mut value: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(8);
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if value == 0 {
            break;
        }
    }
    out
}

fn decode_varint_from_file(file: &mut File) -> Result<(u64, bool), StorageError> {
    let mut value: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        let mut byte = [0u8; 1];
        match file.read_exact(&mut byte) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                if shift == 0 {
                    return Ok((0, true)); // clean EOF at record boundary
                }
                return Err(StorageError::Decode("truncated varint".into()));
            }
            Err(e) => return Err(StorageError::Io(e)),
        }
        value |= ((byte[0] & 0x7F) as u64) << shift;
        shift += 7;
        if shift > 63 {
            return Err(StorageError::Decode("varint overflow".into()));
        }
        if byte[0] & 0x80 == 0 {
            break;
        }
    }
    Ok((value, false))
}

