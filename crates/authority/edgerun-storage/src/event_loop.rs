//! Durable event append dispatch loop.
//!
//! Architecture:
//! 1. Stream code creates an already-signed EventEnvelope.
//! 2. Callers send it through DurableEventAppender::append_event().
//! 2. Single background thread drains channel, writes to disk (fsync)
//! 3. After write confirms, FileIndex is updated as a rebuildable projection.
//! 4. Additional handlers may update derived indexes, but may not create
//!    admitted work or fabricate audit evidence.

use crate::prelude::v1::*;

use edgerun_protocols::core_protocol::protocol::EventEnvelope;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::JoinHandle;

use crate::error::StorageError;
use crate::file_index::FileIndex;
use crate::fs::{append_event_to_file, open_stream_file};
use crate::materializer::{OpEventType, materialize_event_to_index};

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

/// Durable event submission handle. Cloneable for multiple producers.
#[derive(Clone)]
pub struct DurableEventAppender {
    tx: Arc<std::sync::Mutex<SyncSender<WriteRequest>>>,
}

impl DurableEventAppender {
    /// Submit an already-valid signed event to be written.
    pub async fn append_event(&self, event: EventEnvelope) -> Result<u64, StorageError> {
        let (result_tx, result_rx) = mpsc::sync_channel(1);
        let request = WriteRequest { event, result_tx };

        {
            let tx = self
                .tx
                .lock()
                .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?;
            tx.send(request).map_err(|e| {
                StorageError::Io(std::io::Error::new(
                    std::io::ErrorKind::BrokenPipe,
                    format!("event writer channel closed: {e}"),
                ))
            })?;
        }

        result_rx.recv().map_err(|e| {
            StorageError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                format!("event writer result channel closed: {e}"),
            ))
        })?
    }

    /// Synchronous version — blocks the current thread until the write completes.
    pub fn append_event_blocking(&self, event: EventEnvelope) -> Result<u64, StorageError> {
        let (result_tx, result_rx) = mpsc::sync_channel(1);
        let request = WriteRequest { event, result_tx };

        let tx = self
            .tx
            .lock()
            .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?;
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
// Dispatcher context — for handlers running after durable append.
// ---------------------------------------------------------------------------

/// Shared access to the stream_files map on the writer thread.
/// Only used by the single writer thread — no actual concurrency.
type StreamFilesRef = Arc<std::sync::Mutex<alloc::collections::BTreeMap<Vec<u8>, File>>>;

pub struct DispatchContext {
    index: Arc<FileIndex>,
}

// ---------------------------------------------------------------------------
// Event handler trait
// ---------------------------------------------------------------------------

pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &EventEnvelope, ctx: &DispatchContext) -> Result<(), StorageError>;
}

// ---------------------------------------------------------------------------
// EventLoop builder
// ---------------------------------------------------------------------------

pub struct EventLoopBuilder {
    events_dir: PathBuf,
    index: Arc<FileIndex>,
    handlers: Vec<Box<dyn EventHandler>>,
}

impl EventLoopBuilder {
    pub fn new(events_dir: PathBuf, index: Arc<FileIndex>) -> Self {
        Self {
            events_dir,
            index,
            handlers: Vec::new(),
        }
    }

    pub fn register_handler(&mut self, handler: Box<dyn EventHandler>) {
        self.handlers.push(handler);
    }

    pub fn build(self) -> Result<(DurableEventAppender, JoinHandle<()>), StorageError> {
        // Fix #4: bounded channel instead of unbounded mpsc::channel()
        let (tx, rx) = mpsc::sync_channel(EVENT_CHANNEL_CAPACITY);
        let stream_files: StreamFilesRef =
            Arc::new(std::sync::Mutex::new(alloc::collections::BTreeMap::new()));

        let events_dir = self.events_dir.clone();
        let index = Arc::clone(&self.index);
        let handlers = self.handlers;
        let stream_files_clone = Arc::clone(&stream_files);

        let handle = std::thread::Builder::new()
            .name("event-writer".to_string())
            .spawn(move || {
                run_event_loop(rx, events_dir, index, handlers, stream_files_clone);
            })
            .map_err(StorageError::Io)?;

        let writer = DurableEventAppender {
            tx: Arc::new(std::sync::Mutex::new(tx)),
        };

        Ok((writer, handle))
    }
}

/// The event loop: writes to disk → materializes FileIndex → dispatches handlers.
fn run_event_loop(
    rx: Receiver<WriteRequest>,
    events_dir: PathBuf,
    index: Arc<FileIndex>,
    handlers: Vec<Box<dyn EventHandler>>,
    stream_files: StreamFilesRef,
) {
    loop {
        let request = match rx.recv() {
            Ok(req) => req,
            Err(_) => break,
        };

        let event = &request.event;
        let stream_id = event.stream_id.clone();

        // Get or open the log file for this stream
        let mut sf = match stream_files
            .lock()
            .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))
        {
            Ok(sf) => sf,
            Err(e) => {
                let _ = request.result_tx.send(Err(e));
                continue;
            }
        };
        if !sf.contains_key(&stream_id) {
            match open_stream_file(&events_dir, &stream_id) {
                Ok(file) => {
                    sf.insert(stream_id.clone(), file);
                }
                Err(e) => {
                    let _ = request.result_tx.send(Err(e));
                    continue;
                }
            }
        }
        let Some(file) = sf.get_mut(&stream_id) else {
            let _ = request
                .result_tx
                .send(Err(StorageError::Io(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "stream file missing after open",
                ))));
            continue;
        };

        // Validate the stream linkage and write the event to disk.
        let receipt = match append_event_to_file(&events_dir, file, event) {
            Ok(receipt) => receipt,
            Err(e) => {
                let _ = request.result_tx.send(Err(e));
                continue;
            }
        };
        let offset = receipt.file_offset;
        drop(sf);

        // Materialize: update FileIndex from the event
        if let Err(e) = materialize_event_to_index(&index, event, offset) {
            let _ = request.result_tx.send(Err(e));
            continue;
        }

        // Report success after the event is durable and indexed.
        let _ = request.result_tx.send(Ok(offset));

        // Dispatch to additional handlers
        let ctx = DispatchContext {
            index: Arc::clone(&index),
        };

        for handler in &handlers {
            if let Err(e) = handler.handle(event, &ctx) {
                edgerun_log::warn!("event handler error: {e}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Built-in event handlers
// ---------------------------------------------------------------------------

pub struct FetchHandler;
impl EventHandler for FetchHandler {
    fn handle(&self, event: &EventEnvelope, ctx: &DispatchContext) -> Result<(), StorageError> {
        if event.event_type == OpEventType::FetchRequested.as_i32() {
            if let Some(payload) = &event.payload_object {
                let target_id =
                    edgerun_protocols::core_protocol::util::bytes_to_hex(&payload.object_id);
                let _ = ctx.index.enqueue_fetch("object", &target_id, 0);
            }
        }
        Ok(())
    }
}

pub struct PeerDiscoveryHandler;
impl EventHandler for PeerDiscoveryHandler {
    fn handle(&self, event: &EventEnvelope, ctx: &DispatchContext) -> Result<(), StorageError> {
        if event.event_type == OpEventType::PeerDiscovered.as_i32() {
            if let Some(payload) = &event.payload_object {
                let node_id =
                    edgerun_protocols::core_protocol::util::bytes_to_hex(&payload.object_id);
                ctx.index.upsert_peer(&node_id, None, "discovered", false)?;
            }
        }
        Ok(())
    }
}

pub struct PeerStatusHandler;
impl EventHandler for PeerStatusHandler {
    fn handle(&self, _event: &EventEnvelope, _ctx: &DispatchContext) -> Result<(), StorageError> {
        Ok(())
    }
}

pub struct CredentialHandler;
impl EventHandler for CredentialHandler {
    fn handle(&self, _event: &EventEnvelope, _ctx: &DispatchContext) -> Result<(), StorageError> {
        Ok(())
    }
}

pub struct CredentialDeleteHandler;
impl EventHandler for CredentialDeleteHandler {
    fn handle(&self, _event: &EventEnvelope, _ctx: &DispatchContext) -> Result<(), StorageError> {
        Ok(())
    }
}
