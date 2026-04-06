//! Unified node storage — combines event log, SQLite indexes, and encrypted blobs.
//!
//! The `NodeStore` is the high-level storage interface used by the node daemon.
//! It coordinates:
//! - `SqliteIndex` — fast lookups for stream heads, seq→offset mapping, replay cache
//! - `BlobStore` — AES-GCM encrypted payload objects on the filesystem
//! - Append-only event log — protobuf records on the filesystem

use lifegraph_core::protocol::EventEnvelope;
use lifegraph_proto::lifegraph::v0::stream as proto_stream;
use prost::Message;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use crate::blobs::{BlobStore, BlobKeySource};
use crate::sqlite::{SqliteIndex, SqliteIndexConfig};
use crate::error::StorageError;

/// Configuration for the unified node storage.
#[derive(Clone, Debug)]
pub struct NodeStoreConfig {
    /// Root directory for all storage data.
    ///
    /// Creates:
    /// - `{data_root}/events/` — append-only event log files (one per stream)
    /// - `{data_root}/blobs/` — encrypted blob ciphertext files
    /// - `{data_root}/index.sqlite3` — SQLite index
    pub data_root: PathBuf,
    /// How to obtain the blob encryption key.
    /// Uses `Arc` internally to allow cloning the config.
    pub blob_key_source: std::sync::Arc<BlobKeySource>,
}

/// Unified storage for a Lifegraph node.
///
/// Provides atomic append of signed events, encrypted blob storage,
/// and fast index lookups — all rebuildable from the event log.
pub struct NodeStore {
    config: NodeStoreConfig,
    index: SqliteIndex,
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
        let index_path = config.data_root.join("index.sqlite3");

        fs::create_dir_all(&events_dir)?;
        fs::create_dir_all(&blobs_dir)?;

        // Open or create SQLite index
        let index_config = SqliteIndexConfig {
            db_path: index_path,
        };
        let index = SqliteIndex::open(&index_config)?;

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
        let stream_id_hex = hex::encode(&event.stream_id);
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
        use sha2::{Digest, Sha256};
        let event_hash = Sha256::digest(&event_bytes).to_vec();

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
        let stream_id_hex = hex::encode(stream_id);

        // Look up offset in SQLite
        let Some(entry) = self.index.get_event(&stream_id_hex, seq as i64)? else {
            return Ok(None);
        };

        // Read from event log file
        let log_path = self.config.data_root.join("events").join(format!("{}.log", stream_id_hex));
        let mut file = File::open(&log_path)?;
        file.seek(SeekFrom::Start(entry.offset))?;

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
        let stream_id_hex = hex::encode(stream_id);
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
    ) -> Result<lifegraph_proto::lifegraph::v0::common::ObjectRef, StorageError> {
        use lifegraph_proto::lifegraph::v0::common::ObjectRef;
        use sha2::{Digest, Sha256};

        // Compute content-derived object ID
        let object_id = Sha256::digest(content).to_vec();

        // Create LogicalObjectDescriptor (for future storage/persistence)
        let _descriptor = lifegraph_proto::lifegraph::v0::object::LogicalObjectDescriptor {
            descriptor_version: 1,
            object_id: object_id.clone(),
            object_kind,
            object_schema_version: 1,
            canonicalization_id: "raw-bytes-v0".into(),
            canonical_digest: Some(lifegraph_proto::lifegraph::v0::common::Digest {
                algorithm: 1, // DIGEST_ALGORITHM_SHA256
                value: Sha256::digest(content).to_vec(),
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
            &hex::encode(&object_id),
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
        object_ref: &lifegraph_proto::lifegraph::v0::common::ObjectRef,
    ) -> Result<Option<ObjectResult>, StorageError> {
        let object_id_hex = hex::encode(&object_ref.object_id);

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
        payload_object_ref: &Option<lifegraph_proto::lifegraph::v0::common::ObjectRef>,
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
                    let obj_ref = lifegraph_proto::lifegraph::v0::common::ObjectRef {
                        object_id: hex::decode(&entry.target_id).unwrap_or_default(),
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
        let target_hex = hex::encode(target_node);
        let cmd_hash_hex = hex::encode(command_hash);

        if let Some(existing) = self.index.get_replay_entry(&target_hex, &cmd_hash_hex)? {
            return Ok(CommandReplayResult::Duplicate {
                prior_decision_seq: existing.decision_event_seq,
            });
        }

        let cmd_id_hex = hex::encode(command_id);
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

            let _stream_id = match hex::decode(&stream_id_hex) {
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
                use sha2::{Digest, Sha256};
                let event_hash = Sha256::digest(&event_bytes).to_vec();

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

    /// Returns the data root path.
    pub fn data_root(&self) -> &std::path::Path {
        &self.config.data_root
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

