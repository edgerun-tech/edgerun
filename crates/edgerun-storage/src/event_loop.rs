//! THE EVENT LOG IS THE STATE.
//!
//! Single-writer event dispatch loop.
//!
//! Architecture:
//! 1. All mutations create EventEnvelope → send through EventWriter::write_event()
//! 2. Single background thread drains channel, writes to disk (fsync)
//! 3. After write confirms, FileIndex is updated (materialized from the event)
//! 4. Additional handlers may produce new events → written INLINE (no re-queue)
//!
//! The event log IS the source of truth. FileIndex is a materialized view.

use edgerun_core::protocol::EventEnvelope;
use edgerun_proto::edgerun::v0::stream as proto_stream;
use prost::Message;
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::sync::Arc;
use std::thread::JoinHandle;

use crate::blobs::BlobStore;
use crate::error::StorageError;
use crate::file_index::FileIndex;

// ---------------------------------------------------------------------------
// Operational event types — written to the event log alongside protocol events
// Protocol uses 1-7. We use 100+ for operational events.
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
pub enum OpEventType {
    PeerDiscovered = 100,
    PeerStatusChanged = 101,
    FetchRequested = 102,
    FetchCompleted = 103,
    FetchFailed = 104,
    ConnectionAttempt = 105,
    ConnectionEstablished = 106,
    ConnectionFailed = 107,
    CredentialStored = 108,
    CredentialDeleted = 109,
    DelegationStored = 110,
    RevocationStored = 111,
    ControllerChange = 112,
    WorkAccountingRecord = 113,
    ObjectStored = 114,
    ObjectFetched = 115,
    SnapshotProduced = 116,
    SnapshotConsumed = 117,
    CommandRecorded = 118,
    ReplayCached = 119,
}

impl OpEventType {
    pub fn from_i32(v: i32) -> Option<Self> {
        match v {
            100 => Some(Self::PeerDiscovered),
            101 => Some(Self::PeerStatusChanged),
            102 => Some(Self::FetchRequested),
            103 => Some(Self::FetchCompleted),
            104 => Some(Self::FetchFailed),
            105 => Some(Self::ConnectionAttempt),
            106 => Some(Self::ConnectionEstablished),
            107 => Some(Self::ConnectionFailed),
            108 => Some(Self::CredentialStored),
            109 => Some(Self::CredentialDeleted),
            110 => Some(Self::DelegationStored),
            111 => Some(Self::RevocationStored),
            112 => Some(Self::ControllerChange),
            113 => Some(Self::WorkAccountingRecord),
            114 => Some(Self::ObjectStored),
            115 => Some(Self::ObjectFetched),
            116 => Some(Self::SnapshotProduced),
            117 => Some(Self::SnapshotConsumed),
            118 => Some(Self::CommandRecorded),
            119 => Some(Self::ReplayCached),
            _ => None,
        }
    }

    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

// ---------------------------------------------------------------------------
// Event submission
// ---------------------------------------------------------------------------

/// Default capacity for the event submission channel.
/// Producers get `TrySendError::Full` when the writer thread falls behind.
const EVENT_CHANNEL_CAPACITY: usize = 4096;

struct WriteRequest {
    event: EventEnvelope,
    result_tx: SyncSender<Result<u64, StorageError>>,
}

/// Single-writer event submission handle. Cloneable for multiple producers.
#[derive(Clone)]
pub struct EventWriter {
    tx: Arc<std::sync::Mutex<SyncSender<WriteRequest>>>,
}

impl EventWriter {
    /// Submit an event to be written. Async — returns when the write completes.
    pub async fn write_event(&self, event: EventEnvelope) -> Result<u64, StorageError> {
        let (result_tx, result_rx) = mpsc::sync_channel(1);
        let request = WriteRequest { event, result_tx };

        {
            let tx = self.tx.lock().unwrap();
            tx.send(request).map_err(|e| {
                StorageError::Io(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("event writer channel closed: {e}"),
                ))
            })?;
        }

        edgerun_rt::spawn_blocking(move || {
            result_rx.recv().map_err(|e| {
                StorageError::Io(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("event writer result channel closed: {e}"),
                ))
            })?
        })
        .await
        .map_err(|e| StorageError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?
    }

    /// Synchronous version — blocks the current thread until the write completes.
    pub fn write_event_blocking(&self, event: EventEnvelope) -> Result<u64, StorageError> {
        let (result_tx, result_rx) = mpsc::sync_channel(1);
        let request = WriteRequest { event, result_tx };

        let tx = self.tx.lock().unwrap();
        tx.send(request).map_err(|e| {
            StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("event writer channel closed: {e}"),
            ))
        })?;
        drop(tx);

        result_rx.recv().map_err(|e| {
            StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("event writer result channel closed: {e}"),
            ))
        })?
    }
}

// ---------------------------------------------------------------------------
// Dispatcher context — for handlers running on the writer thread.
//
// Fix #10: produce_event() writes INLINE — no channel re-queue.
// The event writer thread already owns exclusive access to stream files and
// the next_seq map, so there is no need to re-queue through the channel.
// Re-queuing would deadlock: the writer thread sends to its own channel,
// then blocks on result_rx.recv() waiting for its own response.
// ---------------------------------------------------------------------------

/// Shared access to the stream_files map on the writer thread.
/// Only used by the single writer thread — no actual concurrency.
type StreamFilesRef = Arc<std::sync::Mutex<HashMap<Vec<u8>, File>>>;

pub struct DispatchContext {
    events_dir: PathBuf,
    stream_files: StreamFilesRef,
    index: Arc<FileIndex>,
    next_seq: Arc<std::sync::Mutex<HashMap<Vec<u8>, u64>>>,
    blobs: Arc<BlobStore>,
}

impl DispatchContext {
    /// Produce an event by writing INLINE to the event log.
    ///
    /// This bypasses the submission channel entirely. The event writer thread
    /// already holds exclusive access to the stream files and next_seq map,
    /// so re-queuing through the channel would deadlock.
    ///
    /// Returns the file offset where the event was written.
    pub fn produce_event(&self, mut event: EventEnvelope) -> Result<u64, StorageError> {
        let stream_id = event.stream_id.clone();

        // Assign sequence number
        {
            let mut next_seq = self.next_seq.lock().unwrap();
            let seq = next_seq.entry(stream_id.clone()).or_insert(0);
            event.seq = *seq;
            *seq += 1;
        }

        // Get or open the log file for this stream
        let mut stream_files = self.stream_files.lock().unwrap();
        let file = stream_files
            .entry(stream_id.clone())
            .or_insert_with(|| open_stream_file(&self.events_dir, &stream_id));

        // Encode event
        let proto: proto_stream::EventEnvelope = event.clone();
        let mut event_bytes = Vec::new();
        proto_stream::EventEnvelope::encode(&proto, &mut event_bytes)
            .map_err(|e| StorageError::Encode(format!("encode failed: {e}")))?;

        // Get current file offset before writing
        let offset = file.metadata().map_err(StorageError::Io)?.len();

        // Write: [varint length][protobuf bytes]
        let len_prefix = edgerun_core::varint::encode_varint(event_bytes.len() as u64);
        file.write_all(&len_prefix).map_err(StorageError::Io)?;
        file.write_all(&event_bytes).map_err(StorageError::Io)?;
        file.sync_all().map_err(StorageError::Io)?;

        drop(stream_files);

        // Update next_seq tracking (ensure it's at least seq+1)
        {
            let mut seq_map = self.next_seq.lock().unwrap();
            let entry = seq_map.entry(stream_id).or_insert(0);
            if event.seq + 1 > *entry {
                *entry = event.seq + 1;
            }
        }

        // Materialize: update FileIndex
        let stream_id_hex = edgerun_core::util::bytes_to_hex(&event.stream_id);
        let event_hash = edgerun_core::crypto::sha256(&event_bytes).to_vec();

        self.index.put_event(
            &stream_id_hex,
            event.seq as i64,
            &event_hash,
            offset,
            event.envelope_version as i64,
        )?;
        self.index
            .set_head(&stream_id_hex, event.seq as i64, &event_hash)?;

        // Materialize operational events
        if let Some(op) = OpEventType::from_i32(event.event_type) {
            match op {
                OpEventType::CredentialStored => {
                    if let Some(payload) = &event.payload_object {
                        let namespace = edgerun_core::util::bytes_to_hex(&payload.object_id);
                        let name = edgerun_core::util::bytes_to_hex(&event.stream_id);
                        let blob_id = edgerun_core::util::bytes_to_hex(&payload.object_id);
                        let _ = self
                            .index
                            .put_credential(&namespace, &name, &blob_id, None);
                    }
                }
                OpEventType::PeerDiscovered | OpEventType::PeerStatusChanged => {
                    if let Some(payload) = &event.payload_object {
                        let node_id = edgerun_core::util::bytes_to_hex(&payload.object_id);
                        let status = match op {
                            OpEventType::PeerDiscovered => "discovered",
                            _ => "status_changed",
                        };
                        let _ = self.index.upsert_peer(&node_id, None, status, false);
                    }
                }
                _ => {}
            }
        }

        Ok(offset)
    }
}

// ---------------------------------------------------------------------------
// Event handler trait
// ---------------------------------------------------------------------------

pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &EventEnvelope, ctx: &DispatchContext) -> Result<(), StorageError>;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn open_stream_file(events_dir: &PathBuf, stream_id: &[u8]) -> File {
    let stream_id_hex = edgerun_core::util::bytes_to_hex(stream_id);
    let log_path = events_dir.join(format!("{stream_id_hex}.log"));
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .expect("failed to open event log file")
}

/// Write a single event to disk (used by both the main loop and produce_event).
fn write_event_to_disk(
    file: &mut File,
    event: &EventEnvelope,
) -> Result<u64, StorageError> {
    let proto: proto_stream::EventEnvelope = event.clone();
    let mut event_bytes = Vec::new();
    proto_stream::EventEnvelope::encode(&proto, &mut event_bytes)
        .map_err(|e| StorageError::Encode(format!("encode failed: {e}")))?;

    let offset = file.metadata().map_err(StorageError::Io)?.len();

    let len_prefix = edgerun_core::varint::encode_varint(event_bytes.len() as u64);
    file.write_all(&len_prefix).map_err(StorageError::Io)?;
    file.write_all(&event_bytes).map_err(StorageError::Io)?;
    file.sync_all().map_err(StorageError::Io)?;

    Ok(offset)
}

// ---------------------------------------------------------------------------
// EventLoop builder
// ---------------------------------------------------------------------------

pub struct EventLoopBuilder {
    events_dir: PathBuf,
    index: Arc<FileIndex>,
    blobs: Arc<BlobStore>,
    handlers: Vec<Box<dyn EventHandler>>,
}

impl EventLoopBuilder {
    pub fn new(events_dir: PathBuf, index: Arc<FileIndex>, blobs: Arc<BlobStore>) -> Self {
        Self {
            events_dir,
            index,
            blobs,
            handlers: Vec::new(),
        }
    }

    pub fn register_handler(&mut self, handler: Box<dyn EventHandler>) {
        self.handlers.push(handler);
    }

    pub fn build(self) -> (EventWriter, JoinHandle<()>) {
        // Fix #4: bounded channel instead of unbounded mpsc::channel()
        let (tx, rx) = mpsc::sync_channel(EVENT_CHANNEL_CAPACITY);
        let next_seq: Arc<std::sync::Mutex<HashMap<Vec<u8>, u64>>> =
            Arc::new(std::sync::Mutex::new(HashMap::new()));
        let stream_files: StreamFilesRef =
            Arc::new(std::sync::Mutex::new(HashMap::new()));

        let events_dir = self.events_dir.clone();
        let blobs = Arc::clone(&self.blobs);
        let index = Arc::clone(&self.index);
        let handlers = self.handlers;
        let next_seq_clone = Arc::clone(&next_seq);
        let stream_files_clone = Arc::clone(&stream_files);

        let handle = std::thread::Builder::new()
            .name("event-writer".to_string())
            .spawn(move || {
                run_event_loop(
                    rx,
                    events_dir,
                    index,
                    blobs,
                    handlers,
                    next_seq_clone,
                    stream_files_clone,
                );
            })
            .expect("failed to spawn event writer thread");

        let writer = EventWriter {
            tx: Arc::new(std::sync::Mutex::new(tx)),
        };

        (writer, handle)
    }
}

/// The event loop: writes to disk → materializes FileIndex → dispatches handlers.
fn run_event_loop(
    rx: Receiver<WriteRequest>,
    events_dir: PathBuf,
    index: Arc<FileIndex>,
    blobs: Arc<BlobStore>,
    handlers: Vec<Box<dyn EventHandler>>,
    next_seq: Arc<std::sync::Mutex<HashMap<Vec<u8>, u64>>>,
    stream_files: StreamFilesRef,
) {
    let materializer = Materializer { index: Arc::clone(&index) };

    loop {
        let request = match rx.recv() {
            Ok(req) => req,
            Err(_) => break,
        };

        let event = &request.event;
        let stream_id = event.stream_id.clone();

        // Get or open the log file for this stream
        let mut sf = stream_files.lock().unwrap();
        let file = sf
            .entry(stream_id.clone())
            .or_insert_with(|| open_stream_file(&events_dir, &stream_id));

        // Write event to disk
        let offset = match write_event_to_disk(file, event) {
            Ok(off) => off,
            Err(e) => {
                let _ = request.result_tx.send(Err(e));
                continue;
            }
        };
        drop(sf);

        // Track next seq per stream
        {
            let mut seq_map = next_seq.lock().unwrap();
            let entry = seq_map.entry(stream_id.clone()).or_insert(0);
            if event.seq + 1 > *entry {
                *entry = event.seq + 1;
            }
        }

        // Report success
        let _ = request.result_tx.send(Ok(offset));

        // Materialize: update FileIndex from the event
        if let Err(e) = materializer.materialize_with_offset(event, offset) {
            edgerun_log::warn!("materialize error: {e}");
        }

        // Dispatch to additional handlers
        let ctx = DispatchContext {
            events_dir: events_dir.clone(),
            stream_files: Arc::clone(&stream_files),
            index: Arc::clone(&index),
            next_seq: Arc::clone(&next_seq),
            blobs: Arc::clone(&blobs),
        };

        for handler in &handlers {
            if let Err(e) = handler.handle(event, &ctx) {
                edgerun_log::warn!("event handler error: {e}");
            }
        }
    }
}

struct Materializer {
    index: Arc<FileIndex>,
}

impl Materializer {
    fn materialize_with_offset(
        &self,
        event: &EventEnvelope,
        offset: u64,
    ) -> Result<(), StorageError> {
        let stream_id_hex = edgerun_core::util::bytes_to_hex(&event.stream_id);

        // Encode to get hash
        let proto: proto_stream::EventEnvelope = event.clone();
        let mut event_bytes = Vec::new();
        proto_stream::EventEnvelope::encode(&proto, &mut event_bytes)
            .map_err(|e| StorageError::Encode(format!("encode failed: {e}")))?;
        let event_hash = edgerun_core::crypto::sha256(&event_bytes).to_vec();

        // Index the event with correct offset
        self.index.put_event(
            &stream_id_hex,
            event.seq as i64,
            &event_hash,
            offset,
            event.envelope_version as i64,
        )?;

        // Update stream head
        self.index
            .set_head(&stream_id_hex, event.seq as i64, &event_hash)?;

        // Materialize operational events
        if let Some(op) = OpEventType::from_i32(event.event_type) {
            match op {
                OpEventType::CredentialStored => {
                    if let Some(payload) = &event.payload_object {
                        let namespace =
                            edgerun_core::util::bytes_to_hex(&payload.object_id);
                        let name = edgerun_core::util::bytes_to_hex(&event.stream_id);
                        let blob_id =
                            edgerun_core::util::bytes_to_hex(&payload.object_id);
                        let _ = self
                            .index
                            .put_credential(&namespace, &name, &blob_id, None);
                    }
                }
                OpEventType::PeerDiscovered | OpEventType::PeerStatusChanged => {
                    if let Some(payload) = &event.payload_object {
                        let node_id =
                            edgerun_core::util::bytes_to_hex(&payload.object_id);
                        let status = match op {
                            OpEventType::PeerDiscovered => "discovered",
                            _ => "status_changed",
                        };
                        let _ = self.index.upsert_peer(&node_id, None, status, false);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Built-in event handlers
// ---------------------------------------------------------------------------

pub struct FetchHandler;
impl EventHandler for FetchHandler {
    fn handle(
        &self,
        event: &EventEnvelope,
        ctx: &DispatchContext,
    ) -> Result<(), StorageError> {
        if event.event_type == OpEventType::FetchRequested.as_i32() {
            if let Some(payload) = &event.payload_object {
                let target_id = edgerun_core::util::bytes_to_hex(&payload.object_id);
                let _ = ctx.index.enqueue_fetch("object", &target_id, 0);
            }
        }
        Ok(())
    }
}

pub struct PeerDiscoveryHandler;
impl EventHandler for PeerDiscoveryHandler {
    fn handle(
        &self,
        event: &EventEnvelope,
        ctx: &DispatchContext,
    ) -> Result<(), StorageError> {
        if event.event_type == OpEventType::PeerDiscovered.as_i32() {
            let attempt_event = EventEnvelope {
                envelope_version: 1,
                stream_id: event.stream_id.clone(),
                seq: 0,
                prev_event_hash: None,
                event_type: OpEventType::ConnectionAttempt.as_i32(),
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
            };
            let _ = ctx.produce_event(attempt_event);
        }
        Ok(())
    }
}

pub struct PeerStatusHandler;
impl EventHandler for PeerStatusHandler {
    fn handle(
        &self,
        _event: &EventEnvelope,
        _ctx: &DispatchContext,
    ) -> Result<(), StorageError> {
        Ok(())
    }
}

pub struct CredentialHandler;
impl EventHandler for CredentialHandler {
    fn handle(
        &self,
        _event: &EventEnvelope,
        _ctx: &DispatchContext,
    ) -> Result<(), StorageError> {
        Ok(())
    }
}

pub struct CredentialDeleteHandler;
impl EventHandler for CredentialDeleteHandler {
    fn handle(
        &self,
        _event: &EventEnvelope,
        _ctx: &DispatchContext,
    ) -> Result<(), StorageError> {
        Ok(())
    }
}
