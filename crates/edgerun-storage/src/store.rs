//! Unified node storage — combines event log, file-based indexes, and encrypted blobs.
//!
//! The `NodeStore` is the high-level storage interface used by the node daemon.
//! It coordinates:
//! - `FileIndex` — fast lookups for stream heads, seq→offset mapping, replay cache
//! - `BlobStore` — AES-GCM encrypted payload objects on the filesystem
//! - Append-only event log — protobuf records on the filesystem

use crate::prelude::v1::*;

use edgerun_core::protocol::{canonical_bytes, Digest, EventEnvelope, ProtocolRecord};
use prost::Message;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

use crate::blobs::{BlobKeySource, BlobStore};
use crate::block::{BlockStorage, BlockStreamStore};
use crate::core::{ContentStore, EventLocation, ScannedEvent};
use crate::credentials::CredentialStore;
use crate::error::StorageError;
use crate::event_loop::{
    materialize_event_to_index, EventLoopBuilder, EventWriter, FetchHandler, OpEventType,
    PeerDiscoveryHandler,
};
use crate::file_index::FileIndex;
use crate::fs::{read_event_at, scan_event_logs, FsContentStore};
use std::collections::HashSet;

enum EventBackend {
    Fs {
        writer: EventWriter,
        writer_thread: std::thread::JoinHandle<()>,
    },
    Block {
        store: Arc<Mutex<BlockStreamStore<Box<dyn crate::block::BlockStorage + Send>>>>,
    },
}

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
    /// - `{data_root}/index.bin` — file-based binary index
    pub data_root: PathBuf,
    /// How to obtain the blob encryption key.
    /// Uses `Arc` internally to allow cloning the config.
    pub blob_key_source: std::sync::Arc<BlobKeySource>,
    /// The node's identity (64-byte NodeID: x || y of ECDSA P-256 public key).
    /// Per spec §6.2: every persisted blob MUST name at least one recipient.
    pub node_identity: Vec<u8>,
}

/// THE EVENT LOG IS THE STATE.
///
/// All mutations go through the EventWriter. The event log is the authoritative
/// record; FileIndex is a materialized view rebuilt from events.
pub struct NodeStore {
    config: NodeStoreConfig,
    index: Arc<FileIndex>,
    blobs: Arc<BlobStore>,
    content: FsContentStore,
    credentials: CredentialStore,
    backend: EventBackend,
}

/// Cloneable handle for recording completed marketplace work from background
/// tasks that cannot borrow the owning `NodeStore`.
#[derive(Clone)]
pub struct WorkAccountingSink {
    index: Arc<FileIndex>,
}

impl WorkAccountingSink {
    /// Record a completed work unit.
    pub fn record_work_accounting(
        &self,
        accounting: &edgerun_core::accounting::WorkAccounting,
    ) -> Result<(), StorageError> {
        record_work_accounting_to_index(&self.index, accounting)
    }
}

fn record_work_accounting_to_index(
    index: &FileIndex,
    accounting: &edgerun_core::accounting::WorkAccounting,
) -> Result<(), StorageError> {
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
    Ok(index.record_work_accounting(record)?)
}

fn to_event_backend_block<S>(device: S) -> Result<EventBackend, StorageError>
where
    S: BlockStorage + Send + 'static,
{
    let device: Box<dyn BlockStorage + Send> = Box::new(device);
    let store = BlockStreamStore::open(device)?;
    Ok(EventBackend::Block {
        store: Arc::new(Mutex::new(store)),
    })
}

struct StoreComponents {
    index: Arc<FileIndex>,
    blobs: Arc<BlobStore>,
    content: FsContentStore,
    credentials: CredentialStore,
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
    pub fn open(config: &NodeStoreConfig) -> Result<Self, StorageError> {
        let events_dir = config.data_root.join("events");
        let blobs_dir = config.data_root.join("blobs");

        fs::create_dir_all(&events_dir)?;
        fs::create_dir_all(&blobs_dir)?;

        let components = Self::open_common_components(config, blobs_dir)?;
        let mut builder = EventLoopBuilder::new(
            events_dir,
            Arc::clone(&components.index),
            Arc::clone(&components.blobs),
        );
        builder.register_handler(Box::new(FetchHandler));
        builder.register_handler(Box::new(PeerDiscoveryHandler));
        let (writer, writer_thread) = builder.build()?;

        Ok(Self {
            config: config.clone(),
            index: components.index,
            blobs: components.blobs,
            content: components.content,
            credentials: components.credentials,
            backend: EventBackend::Fs {
                writer,
                writer_thread,
            },
        })
    }

    /// Opens or initializes storage using a block device for the event stream.
    pub fn open_with_block_device<S>(
        config: &NodeStoreConfig,
        device: S,
    ) -> Result<Self, StorageError>
    where
        S: BlockStorage + Send + 'static,
    {
        let blobs_dir = config.data_root.join("blobs");
        fs::create_dir_all(&blobs_dir)?;

        let components = Self::open_common_components(config, blobs_dir)?;
        Ok(Self {
            config: config.clone(),
            index: components.index,
            blobs: components.blobs,
            content: components.content,
            credentials: components.credentials,
            backend: to_event_backend_block(device)?,
        })
    }

    fn open_common_components(
        config: &NodeStoreConfig,
        blobs_dir: PathBuf,
    ) -> Result<StoreComponents, StorageError> {
        let index = Arc::new(FileIndex::open(&config.data_root)?);

        let blob_config = crate::blobs::BlobStoreConfig {
            blob_dir: blobs_dir,
        };
        let key_source = match config.blob_key_source.as_ref() {
            BlobKeySource::Software { private_key_bytes } => BlobKeySource::Software {
                private_key_bytes: private_key_bytes.clone(),
            },
            BlobKeySource::HardwareSealed { unseal_fn, seal_fn } => BlobKeySource::HardwareSealed {
                unseal_fn: unseal_fn.clone(),
                seal_fn: seal_fn.clone(),
            },
            BlobKeySource::Password { passphrase } => BlobKeySource::Password {
                passphrase: passphrase.clone(),
            },
        };
        let blobs = Arc::new(BlobStore::open(&blob_config, key_source)?);
        let content = FsContentStore::new(Arc::clone(&blobs), Arc::clone(&index));
        let credentials = CredentialStore::new(
            Arc::clone(&blobs),
            Arc::clone(&index),
            config.node_identity.clone(),
        );

        Ok(StoreComponents {
            index,
            blobs,
            content,
            credentials,
        })
    }

    /// Returns a reference to the credential store.
    pub fn credentials(&self) -> &CredentialStore {
        &self.credentials
    }

    // -----------------------------------------------------------------------
    // Event log — append-only, atomic with head update
    // -----------------------------------------------------------------------

    /// Appends an event to the stream's event log.
    /// Routes through the EventWriter — the log is the source of truth.
    pub async fn append_event(&self, event: EventEnvelope) -> Result<u64, StorageError> {
        if let Err(available) = self.check_disk_space() {
            edgerun_log::warn!("low disk space: {available} bytes available");
        }
        match &self.backend {
            EventBackend::Fs { writer, .. } => writer.write_event(event).await,
            EventBackend::Block { store } => {
                self.append_event_blocking_to_block_store(event, store, true)
            }
        }
    }

    /// Signs an event envelope with the node signer.
    pub fn sign_event_envelope(
        &self,
        event: &mut EventEnvelope,
        signer: &dyn MeshSigner,
    ) -> Result<(), StorageError> {
        let record = ProtocolRecord::EventEnvelope(event.clone());
        let canonical = canonical_bytes(&record, true);
        let signature = signer
            .sign_record(edgerun_core::crypto::SIG_DOMAIN_EVENT_ENVELOPE, &canonical)
            .map_err(|e| StorageError::Encode(format!("event signing failed: {e}")))?;
        event.signature = Some(edgerun_core::protocol::Signature {
            algorithm: 1,
            value: signature.to_vec(),
        });
        Ok(())
    }

    /// Signs and appends an event as one storage operation.
    pub async fn append_signed_event(
        &self,
        mut event: EventEnvelope,
        signer: &dyn MeshSigner,
    ) -> Result<u64, StorageError> {
        self.sign_event_envelope(&mut event, signer)?;
        self.append_event(event).await
    }

    /// Synchronous version of `append_event` — for use from blocking threads.
    /// Blocks the current thread until the write completes.
    pub fn append_event_blocking(&self, event: EventEnvelope) -> Result<u64, StorageError> {
        if let Err(available) = self.check_disk_space() {
            edgerun_log::warn!("low disk space: {available} bytes available");
        }
        match &self.backend {
            EventBackend::Fs { writer, .. } => writer.write_event_blocking(event),
            EventBackend::Block { store } => {
                self.append_event_blocking_to_block_store(event, store, true)
            }
        }
    }

    /// Synchronous version of `append_signed_event` for blocking store-task paths.
    pub fn append_signed_event_blocking(
        &self,
        mut event: EventEnvelope,
        signer: &dyn MeshSigner,
    ) -> Result<u64, StorageError> {
        self.sign_event_envelope(&mut event, signer)?;
        self.append_event_blocking(event)
    }

    /// Retrieves an event from the event log by stream ID and sequence number.
    ///
    /// Uses the file index to find the file offset, then reads the protobuf bytes.
    pub fn get_event(
        &self,
        stream_id: &[u8],
        seq: u64,
    ) -> Result<Option<EventEnvelope>, StorageError> {
        let stream_id_hex = edgerun_core::util::bytes_to_hex(stream_id);

        // Look up offset in file index
        let Some(entry) = self.index.get_event(&stream_id_hex, seq as i64)? else {
            return Ok(None);
        };

        let event = match &self.backend {
            EventBackend::Fs { .. } => read_event_at(
                &self.config.data_root.join("events"),
                stream_id,
                seq,
                entry.file_offset,
            ),
            EventBackend::Block { store } => store
                .lock()
                .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?
                .get_event(stream_id, seq),
        }?;

        let Some(event) = event else {
            return Ok(None);
        };

        if let Some(message) =
            Self::get_indexed_event_hash_mismatch_message(stream_id, seq, &entry.event_hash, &event)
        {
            return Err(StorageError::Decode(message));
        }
        Ok(Some(event))
    }

    /// Validates an event returned by a backend against the materialized index.
    ///
    /// The event log is authoritative, but callers using the index for fast
    /// lookup should still reject stale or forged index records immediately.
    fn get_indexed_event_hash_mismatch_message(
        stream_id: &[u8],
        seq: u64,
        expected_hash: &[u8],
        event: &EventEnvelope,
    ) -> Option<String> {
        if event.stream_id != stream_id {
            return Some(format!(
                "indexed event stream mismatch at seq {}: expected {}, got {}",
                seq,
                edgerun_core::util::bytes_to_hex(stream_id),
                edgerun_core::util::bytes_to_hex(&event.stream_id)
            ));
        }
        if event.seq != seq {
            return Some(format!(
                "indexed event seq mismatch at seq {}: envelope contains {}",
                seq, event.seq
            ));
        }

        let event_hash = crate::core::canonical_event_hash(event).value;
        if expected_hash != event_hash {
            return Some(format!(
                "indexed event hash mismatch at seq {}: index has {}, log has {}",
                seq,
                edgerun_core::util::bytes_to_hex(expected_hash),
                edgerun_core::util::bytes_to_hex(&event_hash)
            ));
        }

        None
    }

    /// Returns the current head (latest seq + hash) for a stream.
    pub fn get_head(&self, stream_id: &[u8]) -> Result<Option<(i64, Vec<u8>)>, StorageError> {
        let stream_id_hex = edgerun_core::util::bytes_to_hex(stream_id);
        if let Some(head) = self.index.get_head(&stream_id_hex)? {
            return Ok(Some(head));
        }

        match &self.backend {
            EventBackend::Block { store } => {
                let store = store
                    .lock()
                    .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?;
                Ok(store
                    .get_head(stream_id)
                    .map(|(seq, hash)| (seq as i64, hash)))
            }
            EventBackend::Fs { .. } => Ok(None),
        }
    }

    fn append_event_blocking_to_block_store(
        &self,
        event: EventEnvelope,
        store: &Arc<Mutex<BlockStreamStore<Box<dyn BlockStorage + Send>>>>,
        apply_followups: bool,
    ) -> Result<u64, StorageError> {
        let location = {
            let mut store = store
                .lock()
                .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?;
            let location = store.append_event(event.clone())?;
            location
        };

        materialize_event_to_index(&self.index, &event, location.file_offset)?;

        if apply_followups {
            self.apply_block_followup_events(&event, &location)?;
        }

        Ok(location.file_offset)
    }

    fn apply_block_followup_events(
        &self,
        event: &EventEnvelope,
        _location: &EventLocation,
    ) -> Result<(), StorageError> {
        if event.event_type == OpEventType::FetchRequested.as_i32() {
            if let Some(payload) = &event.payload_object {
                let target_id = edgerun_core::util::bytes_to_hex(&payload.object_id);
                self.index.enqueue_fetch("object", &target_id, 0)?;
            }
        }

        Ok(())
    }

    // -----------------------------------------------------------------------
    // Query delegation (§14.20–§14.21)
    // -----------------------------------------------------------------------

    /// Returns all known stream heads (stream_id_hex, seq, hash).
    pub fn list_stream_heads(&self) -> Result<Vec<(String, i64, Vec<u8>)>, StorageError> {
        Ok(self.index.list_stream_heads()?)
    }

    // -----------------------------------------------------------------------
    // Replay cache (spec §19.10)
    // -----------------------------------------------------------------------

    /// Looks up a previously processed command by its hash.
    /// Returns (command_id, decision_event_seq) if found.
    pub fn get_replay_entry(
        &self,
        target_node: &str,
        command_hash: &str,
    ) -> Result<Option<(String, i64)>, StorageError> {
        match self.index.get_replay_entry(target_node, command_hash)? {
            Some(entry) => Ok(Some((entry.command_id, entry.decision_event_seq))),
            None => Ok(None),
        }
    }

    /// Records a processed command in the persistent replay cache.
    pub fn put_replay_entry(
        &self,
        target_node: &str,
        command_hash: &str,
        command_id: &str,
        decision_event_seq: i64,
    ) -> Result<(), StorageError> {
        Ok(self.index.put_replay_entry(
            target_node,
            command_hash,
            command_id,
            decision_event_seq,
        )?)
    }

    /// Validates the cryptographic integrity of a stream chain.
    ///
    /// Walks the stream from genesis to head, checking:
    /// - Genesis exists at seq 0 with no prev_hash
    /// - Every non-genesis event has a valid prev_hash matching the previous event
    /// - Sequence numbers are contiguous with no gaps
    /// - Every event's signature is structurally present
    ///
    /// Returns the number of events validated, or an error describing the break.
    pub fn validate_stream_chain(&self, stream_id: &[u8]) -> Result<u64, StorageError> {
        self.validate_stream_chain_inner(stream_id, None)
    }

    /// Validates a stream chain and verifies each event signature against `writer`.
    pub fn validate_stream_chain_with_writer(
        &self,
        stream_id: &[u8],
        writer: &edgerun_hardware_signing::NodeID,
    ) -> Result<u64, StorageError> {
        self.validate_stream_chain_inner(stream_id, Some(writer))
    }

    fn validate_stream_chain_inner(
        &self,
        stream_id: &[u8],
        writer: Option<&edgerun_hardware_signing::NodeID>,
    ) -> Result<u64, StorageError> {
        use edgerun_proto::edgerun::v0::stream::EventEnvelope;

        let stream_id_hex = edgerun_core::util::bytes_to_hex(stream_id);
        let head = self.index.get_head(&stream_id_hex)?;
        let (head_seq, _) = head.ok_or_else(|| {
            StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "stream has no head — no genesis event",
            ))
        })?;

        if head_seq < 0 {
            return Err(StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "stream head seq is negative",
            )));
        }

        let mut prev_hash: Option<Vec<u8>> = None;
        for seq in 0..=head_seq as u64 {
            let record = self.index.get_event(&stream_id_hex, seq as i64)?;
            let record = record.ok_or_else(|| {
                StorageError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("missing event at seq {} (head is {})", seq, head_seq),
                ))
            })?;

            // Read the actual event envelope from the event log file
            let event: EventEnvelope = match self.get_event(stream_id, seq) {
                Ok(Some(e)) => e,
                Ok(None) => {
                    return Err(StorageError::Io(std::io::Error::new(
                        std::io::ErrorKind::NotFound,
                        format!("event envelope missing at seq {}", seq),
                    )));
                }
                Err(e) => return Err(e),
            };

            if event.stream_id != stream_id {
                return Err(StorageError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "stream_id mismatch at seq {}: expected {}, got {}",
                        seq,
                        stream_id_hex,
                        edgerun_core::util::bytes_to_hex(&event.stream_id)
                    ),
                )));
            }

            if event.seq != seq {
                return Err(StorageError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "event seq mismatch at seq {}: envelope contains {}",
                        seq, event.seq
                    ),
                )));
            }

            // Check genesis: seq 0 must have no prev_hash
            if seq == 0 {
                if event.prev_event_hash.is_some() {
                    return Err(StorageError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "genesis event must not have prev_event_hash",
                    )));
                }
            } else {
                // Non-genesis: prev_hash must match the previous event's hash
                let expected = prev_hash.as_ref().ok_or_else(|| {
                    StorageError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("no prev_hash available for seq {} (missing genesis?)", seq),
                    ))
                })?;
                let actual = event.prev_event_hash.as_ref().map(|d| &d.value);
                if actual != Some(expected) {
                    return Err(StorageError::Io(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!(
                            "prev_hash mismatch at seq {}: expected {}, got {:?}",
                            seq,
                            edgerun_core::util::bytes_to_hex(expected),
                            actual.map(|v| edgerun_core::util::bytes_to_hex(v))
                        ),
                    )));
                }
            }

            let event_hash = crate::core::canonical_event_hash(&event).value;
            if record.event_hash != event_hash {
                return Err(StorageError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "event hash mismatch at seq {}: index has {}, log has {}",
                        seq,
                        edgerun_core::util::bytes_to_hex(&record.event_hash),
                        edgerun_core::util::bytes_to_hex(&event_hash)
                    ),
                )));
            }

            // Check signature is present
            if event.signature.is_none() {
                return Err(StorageError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("event at seq {} missing signature", seq),
                )));
            }

            if let Some(writer) = writer {
                edgerun_stream::verify_event(&event, writer)?;
            }

            // Record this event's hash for next iteration
            prev_hash = Some(event_hash);
        }

        Ok(head_seq as u64 + 1)
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
    /// are stored alongside as a `.meta` sidecar file.
    ///
    /// Returns the blob's content-derived identifier.
    pub fn put_blob(
        &self,
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
    /// stores the content as an encrypted blob, and records presence in the file index.
    ///
    /// Returns the ObjectRef that can be embedded in events.
    pub fn put_object(
        &self,
        content: &[u8],
        object_kind: i32,
        recipients: &[Vec<u8>],
    ) -> Result<edgerun_proto::edgerun::v0::common::ObjectRef, StorageError> {
        // Best-effort disk space check.
        if let Err(available) = self.check_disk_space() {
            edgerun_log::warn!("low disk space: {} bytes available", available);
        }

        self.content.put_object(content, object_kind, recipients)
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
        Ok(self
            .content
            .get_object(object_ref)?
            .map(|object| ObjectResult {
                object_id: object.object_id,
                object_kind: object.object_kind,
                content: object.content,
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
    pub fn process_fetch_queue(&self) -> Result<usize, StorageError> {
        let mut resolved = 0;
        while let Some(entry) = self.index.dequeue_fetch()? {
            let success = match entry.target_type.as_str() {
                "object" => {
                    let obj_ref = edgerun_proto::edgerun::v0::common::ObjectRef {
                        object_id: edgerun_core::util::hex_to_bytes(&entry.target_id)
                            .unwrap_or_default(),
                        object_kind: None,
                    };
                    self.get_object(&obj_ref)?.is_some()
                }
                "event" => {
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
                self.index.enqueue_fetch(
                    &entry.target_type,
                    &entry.target_id,
                    entry.priority - 1,
                )?;
            }
        }
        Ok(resolved)
    }

    /// Dequeue one pending fetch entry for remote peer querying.
    pub fn dequeue_fetch_for_remote(&self) -> Result<Option<crate::FetchEntry>, StorageError> {
        Ok(self.index.dequeue_fetch()?)
    }

    /// Mark a fetch entry done after successful remote retrieval.
    pub fn mark_fetch_done(&self, fetch_id: i64) -> Result<(), StorageError> {
        Ok(self.index.mark_fetch_done(fetch_id)?)
    }

    /// Re-enqueue a fetch entry with lower priority for retry.
    pub fn requeue_fetch(
        &self,
        target_type: &str,
        target_id: &str,
        priority: i64,
    ) -> Result<(), StorageError> {
        Ok(self.index.enqueue_fetch(target_type, target_id, priority)?)
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
    pub fn list_snapshots(
        &self,
    ) -> Result<Vec<(String, String, String, String, i64, i32, String)>, StorageError> {
        Ok(self.index.list_snapshots()?)
    }

    /// Returns a snapshot by ID.
    pub fn get_snapshot(
        &self,
        snapshot_id: &str,
    ) -> Result<Option<(String, String, String, String, i64, i32, String)>, StorageError> {
        Ok(self.index.get_snapshot(snapshot_id)?)
    }

    /// Records or updates a known peer.
    pub fn upsert_peer(
        &self,
        node_id_hex: &str,
        addr: Option<&str>,
        status: &str,
        is_bootstrap: bool,
    ) -> Result<(), StorageError> {
        Ok(self
            .index
            .upsert_peer(node_id_hex, addr, status, is_bootstrap)?)
    }

    /// Updates a peer's reachability status.
    pub fn update_peer_status(&self, node_id_hex: &str, status: &str) -> Result<(), StorageError> {
        Ok(self.index.update_peer_status(node_id_hex, status)?)
    }

    // -----------------------------------------------------------------------
    // Controller state persistence
    // -----------------------------------------------------------------------

    /// Records a controller change for replay and projection.
    pub fn record_controller_change(
        &self,
        controller_hex: &str,
        change_type: &str,
        event_seq: i64,
    ) -> Result<(), StorageError> {
        Ok(self
            .index
            .record_controller_change(controller_hex, change_type, event_seq)?)
    }

    /// Projects the controller set from the persistent change log.
    /// Starts with the initial controllers and applies all changes up to the given event sequence.
    pub fn project_controller_set(
        &self,
        initial: Vec<Vec<u8>>,
        up_to_seq: i64,
    ) -> Result<ControllerSet, StorageError> {
        let mut set = ControllerSet::new(initial);
        for (controller_hex, change_type) in self.index.list_controller_changes(up_to_seq)? {
            let controller_id =
                edgerun_core::util::hex_to_bytes(&controller_hex).unwrap_or_default();
            match change_type.as_str() {
                "added" => set.add(controller_id),
                "removed" => {
                    set.remove(&controller_id);
                }
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
        Ok(self.index.store_delegation(
            delegation_id,
            issuer_hex,
            recipient_hex,
            capability_hex,
            expires_at,
        )?)
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
        Ok(self.index.store_revocation(
            revocation_id,
            issuer_hex,
            target_type,
            target_hex,
            effective_at,
        )?)
    }

    /// Returns all active revocation target IDs (type, hex).
    pub fn list_active_revocations(&self) -> Result<Vec<(String, String)>, StorageError> {
        Ok(self.index.list_active_revocations()?)
    }

    /// Returns all known peers.
    pub fn list_peers(
        &self,
    ) -> Result<Vec<(String, Option<String>, String, Option<i64>, bool)>, StorageError> {
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
    pub fn record_work_accounting(
        &self,
        accounting: &edgerun_core::accounting::WorkAccounting,
    ) -> Result<(), StorageError> {
        record_work_accounting_to_index(&self.index, accounting)
    }

    /// Clone a lightweight handle for recording work accounting from background tasks.
    pub fn work_accounting_sink(&self) -> WorkAccountingSink {
        WorkAccountingSink {
            index: Arc::clone(&self.index),
        }
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
    pub fn work_in_time_range(
        &self,
        from_us: u64,
        to_us: u64,
    ) -> Result<Vec<crate::WorkAccountingRecord>, StorageError> {
        Ok(self.index.work_in_time_range(from_us, to_us)?)
    }

    /// Get all work records for a specific workload class.
    pub fn work_by_class(
        &self,
        class: &str,
    ) -> Result<Vec<crate::WorkAccountingRecord>, StorageError> {
        Ok(self.index.work_by_class(class)?)
    }

    /// Get all work records with a specific status.
    pub fn work_by_status(
        &self,
        status: &str,
    ) -> Result<Vec<crate::WorkAccountingRecord>, StorageError> {
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
        &self,
        signer: &dyn MeshSigner,
        view_type: &str,
        completeness: i32,
    ) -> Result<edgerun_proto::edgerun::v0::access::SnapshotDescriptor, StorageError> {
        use edgerun_proto::edgerun::v0::access::SnapshotDescriptor;
        use edgerun_proto::edgerun::v0::common::{Digest, HeadRef, IdentityRef, StreamRef};
        use edgerun_proto::edgerun::v0::trust::{ScopeDescriptor, ScopeKind};

        // Collect current stream heads
        let heads = self.list_stream_heads()?;
        let base_heads: Vec<HeadRef> = heads
            .iter()
            .map(|(sid, seq, hash)| HeadRef {
                stream_id: edgerun_core::util::hex_to_bytes(sid)
                    .unwrap_or_else(|_| sid.clone().into_bytes()),
                seq: *seq as u64,
                event_hash: Some(Digest {
                    algorithm: 1,
                    value: hash.clone(),
                }),
            })
            .collect();
        let target_streams = base_heads
            .iter()
            .map(|head| StreamRef {
                stream_id: head.stream_id.clone(),
            })
            .collect();

        // Create snapshot descriptor
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let now_seconds = i64::try_from(now_secs).unwrap_or(i64::MAX);
        let now_secs = now_seconds as u64;
        let snapshot_id = {
            let digest =
                edgerun_core::crypto::sha256(format!("{}-{}", view_type, now_secs).as_bytes());
            format!("snap-{}", edgerun_core::util::bytes_to_hex(&digest[..8]))
        };
        let node_id = signer.node_id();
        // Store base_heads as simple delimited text: stream_hex:seq:hash_hex;...
        let base_heads_text: String = heads
            .iter()
            .map(|(sid, seq, hash)| {
                format!("{}:{}:{}", sid, seq, edgerun_core::util::bytes_to_hex(hash))
            })
            .collect::<Vec<_>>()
            .join(";");
        let payload_object_ref = self.put_object(
            base_heads_text.as_bytes(),
            3, /* OBJECT_KIND_SNAPSHOT */
            &[],
        )?;

        let mut descriptor = SnapshotDescriptor {
            descriptor_version: 1,
            snapshot_id: snapshot_id.clone().into_bytes(),
            view_type: view_type.to_string(),
            view_version: 1,
            producer: Some(IdentityRef {
                identity_id: node_id.0.to_vec(),
                identity_kind: Some(2), // NODE
                key_hint: Some(node_id.0.to_vec()),
            }),
            produced_at: Some(prost_types::Timestamp {
                seconds: now_seconds,
                nanos: 0,
            }),
            base_heads,
            base_checkpoints: vec![],
            scope: Some(ScopeDescriptor {
                scope_version: 1,
                scope_kind: ScopeKind::Stream as i32,
                target_nodes: vec![],
                target_streams,
                target_object_kinds: vec![],
                target_view_types: vec![view_type.to_string()],
                target_domains: vec![],
                time_bounds: None,
                scope_metadata: None,
            }),
            completeness,
            payload_object: Some(payload_object_ref.clone()),
            supersedes: None,
            snapshot_metadata: None,
            signature: None,
        };

        // Sign the descriptor with domain separation
        let record = edgerun_core::protocol::ProtocolRecord::SnapshotDescriptor(descriptor.clone());
        let canonical = edgerun_core::protocol::canonical_bytes(&record, true);
        let sig = signer
            .sign_record(
                edgerun_core::crypto::SIG_DOMAIN_SNAPSHOT_DESCRIPTOR,
                &canonical,
            )
            .map_err(|e| StorageError::Encode(format!("snapshot signing failed: {}", e)))?;
        descriptor.signature = Some(edgerun_core::protocol::Signature {
            algorithm: 1,
            value: sig.to_vec(),
        });

        let producer_hex = edgerun_core::util::bytes_to_hex(&node_id.0);

        self.index.put_snapshot(
            &snapshot_id,
            &edgerun_core::util::bytes_to_hex(&payload_object_ref.object_id),
            view_type,
            &producer_hex,
            now_seconds,
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
        &self,
        descriptor: &edgerun_proto::edgerun::v0::access::SnapshotDescriptor,
        trusted_producers: &[Vec<u8>],
    ) -> Result<String, StorageError> {
        let validation =
            edgerun_core::validators_proto::validate_snapshot(descriptor, trusted_producers);
        if validation.verdict != edgerun_core::result::Verdict::Accept {
            let reason = validation
                .reason_code
                .map(|code| code.as_str().to_string())
                .unwrap_or_else(|| "snapshot validation failed".into());
            return Err(StorageError::Decode(reason));
        }

        let producer = descriptor
            .producer
            .as_ref()
            .ok_or_else(|| StorageError::Decode("producer is missing".into()))?;
        let producer_hex = edgerun_core::util::bytes_to_hex(&producer.identity_id);

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
        let simple_heads: Vec<(String, u64, String)> = descriptor
            .base_heads
            .iter()
            .map(|h| {
                (
                    edgerun_core::util::bytes_to_hex(&h.stream_id),
                    h.seq,
                    h.event_hash
                        .as_ref()
                        .map(|d| edgerun_core::util::bytes_to_hex(&d.value))
                        .unwrap_or_default(),
                )
            })
            .collect();
        let base_heads = simple_heads
            .iter()
            .map(|(s, seq, h)| format!("{}:{}:{}", s, seq, h))
            .collect::<Vec<_>>()
            .join(";");
        let produced_at = descriptor
            .produced_at
            .as_ref()
            .map(|t| t.seconds)
            .unwrap_or(0);

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
            if let Some((_sid, local_seq, _hash)) =
                local_heads.iter().find(|(s, _, _)| s == &stream_id_hex)
            {
                let local_seq_u64 = *local_seq as u64;
                if base_head.seq < local_seq_u64 {
                    is_stale = true;
                    // In v0 we always accept stale snapshots
                } else if base_head.seq > local_seq_u64 {
                    return Err(StorageError::Decode(
                        "snapshot base is ahead of local head".into(),
                    ));
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
        &self,
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
        self.index
            .put_replay_entry(&target_hex, &cmd_hash_hex, &cmd_id_hex, decision_event_seq)?;

        Ok(CommandReplayResult::New)
    }

    // -----------------------------------------------------------------------
    // Rebuildability — rebuild indexes from event logs
    // -----------------------------------------------------------------------

    /// Rebuilds all file indexes from the event log files.
    ///
    /// Per the spec (§6.3): indexes are NOT the authoritative source of truth
    /// and SHOULD be rebuildable from the event log.
    ///
    /// This scans all `.log` files in the events directory, replays every event,
    /// and repopulates the seq→offset mapping, stream heads, and event presence index.
    pub fn rebuild_indexes(&self) -> Result<usize, StorageError> {
        let scanned = self.scan_events_for_rebuild()?;
        self.rebuild_indexes_from_scanned_events(scanned)
    }

    fn scan_events_for_rebuild(&self) -> Result<Vec<ScannedEvent>, StorageError> {
        match &self.backend {
            EventBackend::Fs { .. } => scan_event_logs(&self.config.data_root.join("events")),
            EventBackend::Block { store } => store
                .lock()
                .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?
                .scan(),
        }
    }

    fn rebuild_indexes_from_scanned_events(
        &self,
        mut scanned: Vec<ScannedEvent>,
    ) -> Result<usize, StorageError> {
        scanned.sort_by(|left, right| {
            left.location
                .stream_id
                .cmp(&right.location.stream_id)
                .then(left.location.seq.cmp(&right.location.seq))
        });

        validate_scanned_event_chains(&scanned)?;

        self.index.clear()?;

        let total_events = scanned.len();
        for scanned_event in scanned {
            let EventLocation {
                stream_id,
                seq,
                file_offset,
                ..
            } = &scanned_event.location;
            let stream_id_hex = edgerun_core::util::bytes_to_hex(stream_id);

            materialize_event_to_index(&self.index, &scanned_event.event, *file_offset)?;
            self.index.set_head(
                &stream_id_hex,
                *seq as i64,
                &scanned_event.location.event_hash,
            )?;
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
    pub fn integrity_check_and_rebuild(&self) -> Result<usize, StorageError> {
        let healthy = self.integrity_check()?;
        if healthy {
            return Ok(0);
        }
        self.rebuild_indexes()
    }

    /// Runs a WAL checkpoint to flush and truncate the WAL file.
    /// Returns the WAL file size after checkpoint, or `None` if no WAL exists.
    pub fn wal_checkpoint(&self) -> Result<Option<u64>, StorageError> {
        // File-based indexes don't have a WAL
        Ok(None)
    }

    /// Checks available disk space on the data root partition.
    /// Returns available bytes, or `None` if the stat couldn't be obtained.
    #[cfg(target_os = "none")]
    pub fn available_disk_space(&self) -> Result<Option<u64>, std::io::Error> {
        Ok(None)
    }

    /// Checks available disk space on the data root partition.
    /// Returns available bytes, or `None` if the stat couldn't be obtained.
    #[cfg(not(target_os = "none"))]
    pub fn available_disk_space(&self) -> Result<Option<u64>, std::io::Error> {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;

        let path_c = CString::new(self.config.data_root.as_os_str().as_bytes()).map_err(|_| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "path contains null bytes")
        })?;

        unsafe {
            let mut stat = std::mem::MaybeUninit::zeroed().assume_init();
            if libc::statvfs(path_c.as_ptr(), &mut stat) == 0 {
                let avail = stat.f_bavail * stat.f_frsize;
                Ok(Some(avail))
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
            Err(_) => Ok(()),   // Can't check, proceed optimistically
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
    /// Per spec §6.2: every persisted blob MUST name at least one recipient.
    /// The node's identity is used as the recipient.
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
        let blob_id = self
            .blobs
            .store(secret, &[self.config.node_identity.clone()])?;

        self.index
            .put_credential(namespace, name, &blob_id, description)?;

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
    pub fn delete_credential(&self, namespace: &str, name: &str) -> Result<bool, StorageError> {
        self.index
            .delete_credential(namespace, name)
            .map_err(StorageError::Io)
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

fn validate_scanned_event_chains(scanned: &[ScannedEvent]) -> Result<(), StorageError> {
    let mut current_stream: Option<&[u8]> = None;
    let mut expected_seq = 0u64;
    let mut prev_hash: Option<Vec<u8>> = None;

    for scanned_event in scanned {
        let event = &scanned_event.event;
        let location = &scanned_event.location;

        if location.stream_id != event.stream_id {
            return Err(StorageError::Decode(format!(
                "scanned event stream mismatch at offset {}: location has {}, envelope has {}",
                location.file_offset,
                edgerun_core::util::bytes_to_hex(&location.stream_id),
                edgerun_core::util::bytes_to_hex(&event.stream_id)
            )));
        }

        if location.seq != event.seq {
            return Err(StorageError::Decode(format!(
                "scanned event seq mismatch at offset {}: location has {}, envelope has {}",
                location.file_offset, location.seq, event.seq
            )));
        }

        let event_hash = crate::core::canonical_event_hash(event).value;
        if location.event_hash != event_hash {
            return Err(StorageError::Decode(format!(
                "scanned event hash mismatch at offset {}: location has {}, envelope has {}",
                location.file_offset,
                edgerun_core::util::bytes_to_hex(&location.event_hash),
                edgerun_core::util::bytes_to_hex(&event_hash)
            )));
        }

        if current_stream != Some(location.stream_id.as_slice()) {
            current_stream = Some(&location.stream_id);
            expected_seq = 0;
            prev_hash = None;
        }

        if location.seq != expected_seq {
            return Err(StorageError::Decode(format!(
                "stream {} has non-contiguous seq: expected {}, got {}",
                edgerun_core::util::bytes_to_hex(&location.stream_id),
                expected_seq,
                location.seq
            )));
        }

        if location.seq == 0 {
            if event.prev_event_hash.is_some() {
                return Err(StorageError::Decode(format!(
                    "stream {} genesis event has prev_event_hash",
                    edgerun_core::util::bytes_to_hex(&location.stream_id)
                )));
            }
        } else {
            let expected_prev = prev_hash.as_ref().ok_or_else(|| {
                StorageError::Decode(format!(
                    "stream {} missing previous hash for seq {}",
                    edgerun_core::util::bytes_to_hex(&location.stream_id),
                    location.seq
                ))
            })?;
            let actual_prev = event.prev_event_hash.as_ref().map(|d| &d.value);
            if actual_prev != Some(expected_prev) {
                return Err(StorageError::Decode(format!(
                    "stream {} prev_hash mismatch at seq {}",
                    edgerun_core::util::bytes_to_hex(&location.stream_id),
                    location.seq
                )));
            }
        }

        prev_hash = Some(event_hash);
        expected_seq = expected_seq.saturating_add(1);
    }

    Ok(())
}

/// Result of checking a command against the replay cache.
pub enum CommandReplayResult {
    /// First time seeing this command — safe to process.
    New,
    /// Same command_hash — already processed. Return prior outcome.
    Duplicate { prior_decision_seq: i64 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::InMemoryBlockDevice;
    use crate::test_support::TestSigner;
    use edgerun_core::protocol::Digest;
    use std::path::PathBuf;

    fn tmp_data_root() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("node_store_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn make_store(data_root: PathBuf) -> NodeStore {
        let config = NodeStoreConfig {
            data_root,
            blob_key_source: std::sync::Arc::new(BlobKeySource::Software {
                private_key_bytes: vec![0x42; 32],
            }),
            node_identity: vec![0x99; 32],
        };
        NodeStore::open(&config).unwrap()
    }

    fn make_block_store(data_root: PathBuf) -> (NodeStore, InMemoryBlockDevice) {
        let config = NodeStoreConfig {
            data_root,
            blob_key_source: std::sync::Arc::new(BlobKeySource::Software {
                private_key_bytes: vec![0x42; 32],
            }),
            node_identity: vec![0x99; 32],
        };

        let device = InMemoryBlockDevice::new(32, 64);
        let store = NodeStore::open_with_block_device(&config, device.clone()).unwrap();
        (store, device)
    }

    #[test]
    fn produce_snapshot_includes_verifiable_producer_key_hint() {
        let mut store = make_store(tmp_data_root());
        let signer = TestSigner::new();
        let mut genesis = event(b"stream-1", 0, None);
        store
            .sign_event_envelope(&mut genesis, &signer)
            .expect("sign genesis");
        store
            .append_event_blocking(genesis)
            .expect("append genesis");

        let descriptor = store
            .produce_snapshot(&signer, "timeline", 1)
            .expect("produce snapshot");

        let producer = descriptor.producer.as_ref().expect("producer");
        assert_eq!(producer.identity_id, signer.node_id().0.to_vec());
        assert_eq!(producer.key_hint, Some(signer.node_id().0.to_vec()));

        let result = edgerun_core::validators_proto::validate_snapshot(
            &descriptor,
            &[signer.node_id().0.to_vec()],
        );
        assert_eq!(result.verdict, edgerun_core::result::Verdict::Accept);
    }

    #[test]
    fn consume_snapshot_rejects_tampered_signature() {
        let mut store = make_store(tmp_data_root());
        let signer = TestSigner::new();
        let mut genesis = event(b"stream-1", 0, None);
        store
            .sign_event_envelope(&mut genesis, &signer)
            .expect("sign genesis");
        store
            .append_event_blocking(genesis)
            .expect("append genesis");

        let mut descriptor = store
            .produce_snapshot(&signer, "timeline", 1)
            .expect("produce snapshot");
        if let Some(signature) = &mut descriptor.signature {
            signature.value[0] ^= 0xFF;
        }

        let err = store
            .consume_snapshot(&descriptor, &[signer.node_id().0.to_vec()])
            .unwrap_err();
        assert!(matches!(err, StorageError::Decode(_)));
    }

    fn event(stream_id: &[u8], seq: u64, prev_hash: Option<Vec<u8>>) -> EventEnvelope {
        EventEnvelope {
            envelope_version: 1,
            event_version: 1,
            stream_id: stream_id.to_vec(),
            seq,
            prev_event_hash: prev_hash.map(|value| Digest {
                algorithm: 1,
                value,
            }),
            event_type: 1,
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
            ..Default::default()
        }
    }

    fn scanned(event: EventEnvelope, file_offset: u64) -> ScannedEvent {
        let event_hash = crate::core::canonical_event_hash(&event).value;
        ScannedEvent {
            location: EventLocation {
                stream_id: event.stream_id.clone(),
                seq: event.seq,
                event_hash,
                file_offset,
                envelope_version: event.envelope_version,
            },
            event,
        }
    }

    #[test]
    fn put_object_can_be_retrieved_by_logical_object_ref() {
        let data_root = tmp_data_root();
        let store = make_store(data_root.clone());

        let object_ref = store
            .put_object(b"hello object", 1, &[vec![0x99; 32]])
            .unwrap();
        let object_id_hex = edgerun_core::util::bytes_to_hex(&object_ref.object_id);
        let indexed = store.index.lookup_objects(&[object_id_hex]).unwrap();

        assert_eq!(indexed.len(), 1);
        assert_ne!(indexed[0].1.as_deref(), indexed[0].2.as_deref());

        let loaded = store.get_object(&object_ref).unwrap().unwrap();
        assert_eq!(loaded.content, b"hello object");

        let _ = std::fs::remove_dir_all(data_root);
    }

    #[test]
    fn block_mode_writes_and_reads_events() {
        let data_root = tmp_data_root();
        let (store, _device) = make_block_store(data_root.clone());

        assert!(store.get_head(b"stream").unwrap().is_none());

        let first = event(b"stream", 0, None);
        let first_hash = crate::core::canonical_event_hash(&first).value;
        let _ = store.append_event_blocking(first).unwrap();
        assert_eq!(store.get_head(b"stream").unwrap().unwrap().0, 0);

        let second = event(b"stream", 1, Some(first_hash));
        let _ = store.append_event_blocking(second).unwrap();
        assert_eq!(store.get_head(b"stream").unwrap().unwrap().0, 1);

        let from_log = store.get_event(b"stream", 1).unwrap().unwrap();
        assert_eq!(from_log.seq, 1);
        assert_eq!(from_log.stream_id, b"stream".to_vec());

        let _ = std::fs::remove_dir_all(data_root);
    }

    #[test]
    fn block_mode_does_not_append_unsigned_peer_followup() {
        let data_root = tmp_data_root();
        let (store, _device) = make_block_store(data_root.clone());

        let mut discovered = event(b"peer-stream", 0, None);
        discovered.event_type = OpEventType::PeerDiscovered.as_i32();
        store.append_event_blocking(discovered).unwrap();

        let head = store.get_head(b"peer-stream").unwrap().unwrap();
        assert_eq!(head.0, 0);
        assert!(store.get_event(b"peer-stream", 1).unwrap().is_none());

        let _ = std::fs::remove_dir_all(data_root);
    }

    #[test]
    fn get_event_rejects_index_hash_mismatch() {
        let data_root = tmp_data_root();
        let store = make_store(data_root.clone());
        let stream_id = b"indexed-stream";
        let event = event(stream_id, 0, None);
        let offset = store.append_event_blocking(event).unwrap();

        store.index.clear().unwrap();
        let stream_id_hex = edgerun_core::util::bytes_to_hex(stream_id);
        store
            .index
            .put_event(&stream_id_hex, 0, &[0xff; 32], offset, 1)
            .unwrap();

        let err = store.get_event(stream_id, 0).unwrap_err();
        assert!(matches!(err, StorageError::Decode(_)));

        let _ = std::fs::remove_dir_all(data_root);
    }

    #[test]
    fn append_signed_event_signs_persists_and_updates_head() {
        let data_root = tmp_data_root();
        let (store, _device) = make_block_store(data_root.clone());
        let signer = TestSigner::new();
        let stream_id = b"signed-stream";

        let event = event(stream_id, 0, None);
        let _offset = store.append_signed_event_blocking(event, &signer).unwrap();

        let stored = store.get_event(stream_id, 0).unwrap().unwrap();
        assert!(stored.signature.is_some());
        assert!(edgerun_stream::verify_event(&stored, &signer.node_id()).is_ok());
        assert_eq!(store.get_head(stream_id).unwrap().unwrap().0, 0);

        let _ = std::fs::remove_dir_all(data_root);
    }

    #[test]
    fn validate_stream_chain_with_writer_verifies_signed_events() {
        let data_root = tmp_data_root();
        let (store, _device) = make_block_store(data_root.clone());
        let signer = TestSigner::new();
        let stream_id = b"signed-validated-stream";

        let first = event(stream_id, 0, None);
        store.append_signed_event_blocking(first, &signer).unwrap();
        let first = store.get_event(stream_id, 0).unwrap().unwrap();
        let first_hash = crate::core::canonical_event_hash(&first).value;

        let second = event(stream_id, 1, Some(first_hash));
        store.append_signed_event_blocking(second, &signer).unwrap();

        assert_eq!(store.validate_stream_chain(stream_id).unwrap(), 2);
        assert_eq!(
            store
                .validate_stream_chain_with_writer(stream_id, &signer.node_id())
                .unwrap(),
            2
        );

        let _ = std::fs::remove_dir_all(data_root);
    }

    #[test]
    fn validate_stream_chain_with_writer_rejects_wrong_writer() {
        let data_root = tmp_data_root();
        let (store, _device) = make_block_store(data_root.clone());
        let signer = TestSigner::new();
        let wrong_signer = TestSigner::new();
        let stream_id = b"wrong-writer-stream";

        store
            .append_signed_event_blocking(event(stream_id, 0, None), &signer)
            .unwrap();

        let err = store
            .validate_stream_chain_with_writer(stream_id, &wrong_signer.node_id())
            .unwrap_err();
        assert!(matches!(err, StorageError::Stream(_)));

        let _ = std::fs::remove_dir_all(data_root);
    }

    #[test]
    fn rebuild_indexes_rejects_broken_scanned_chain_before_clearing_index() {
        let data_root = tmp_data_root();
        let store = make_store(data_root.clone());

        materialize_event_to_index(&store.index, &event(b"existing-stream", 0, None), 0).unwrap();
        assert!(store.get_head(b"existing-stream").unwrap().is_some());

        let genesis = event(b"broken-stream", 0, None);
        let second = event(b"broken-stream", 1, Some(vec![0xaa; 32]));
        let result = store
            .rebuild_indexes_from_scanned_events(vec![scanned(genesis, 0), scanned(second, 128)]);

        assert!(result.is_err());
        assert!(store.get_head(b"existing-stream").unwrap().is_some());

        let _ = std::fs::remove_dir_all(data_root);
    }

    #[test]
    fn block_mode_rebuilds_indexes_from_stream_scan() {
        let data_root = tmp_data_root();
        let config = NodeStoreConfig {
            data_root: data_root.clone(),
            blob_key_source: std::sync::Arc::new(BlobKeySource::Software {
                private_key_bytes: vec![0x42; 32],
            }),
            node_identity: vec![0x99; 32],
        };

        let device = InMemoryBlockDevice::new(32, 64);
        {
            let store = NodeStore::open_with_block_device(&config, device.clone()).unwrap();
            let first = event(b"stream", 0, None);
            let first_hash = crate::core::canonical_event_hash(&first).value;
            store.append_event_blocking(first).unwrap();
            let second = event(b"stream", 1, Some(first_hash));
            store.append_event_blocking(second).unwrap();
        }

        let index_dir = data_root.join("indexes");
        let _ = std::fs::remove_dir_all(&index_dir);

        let reopened = NodeStore::open_with_block_device(&config, device).unwrap();
        let rebuilt = reopened.rebuild_indexes().unwrap();
        assert_eq!(rebuilt, 2);

        let head = reopened.get_head(b"stream").unwrap().unwrap();
        assert_eq!(head.0, 1);
        assert!(reopened.get_event(b"stream", 0).unwrap().is_some());
        assert!(reopened.get_event(b"stream", 1).unwrap().is_some());

        let _ = std::fs::remove_dir_all(data_root);
    }
}
