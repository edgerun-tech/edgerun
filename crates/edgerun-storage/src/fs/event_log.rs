//! Filesystem event-log backend.

use edgerun_core::protocol::EventEnvelope;
use edgerun_proto::edgerun::v0::stream as proto_stream;
use prost::Message;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::core::{
    canonical_event_hash, encode_event_frame, AppendReceipt, EventLocation, EventLog, ScannedEvent,
};
use crate::error::StorageError;

/// Hosted filesystem event log using `{events_dir}/{stream_id_hex}.log`.
#[derive(Clone, Debug)]
pub struct FsEventLog {
    events_dir: PathBuf,
}

impl FsEventLog {
    pub fn new(events_dir: PathBuf) -> Self {
        Self { events_dir }
    }

    pub fn events_dir(&self) -> &Path {
        &self.events_dir
    }
}

impl EventLog for FsEventLog {
    fn append_event(&mut self, event: &EventEnvelope) -> Result<AppendReceipt, StorageError> {
        let mut file = open_stream_file(&self.events_dir, &event.stream_id);
        let offset = write_event_to_file(&mut file, event)?;
        let event_hash = canonical_event_hash(event).value;
        Ok(AppendReceipt {
            stream_id: event.stream_id.clone(),
            seq: event.seq,
            event_hash,
            file_offset: offset,
            envelope_version: event.envelope_version,
        })
    }

    fn read_event(
        &self,
        stream_id: &[u8],
        seq: u64,
        location: &EventLocation,
    ) -> Result<Option<EventEnvelope>, StorageError> {
        read_event_at(&self.events_dir, stream_id, seq, location.file_offset)
    }

    fn scan(&self) -> Result<Vec<ScannedEvent>, StorageError> {
        scan_event_logs(&self.events_dir)
    }
}

pub fn open_stream_file(events_dir: &Path, stream_id: &[u8]) -> File {
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

pub fn write_event_to_file(file: &mut File, event: &EventEnvelope) -> Result<u64, StorageError> {
    let (len_prefix, event_bytes) = encode_event_frame(event)?;

    let offset = file.metadata().map_err(StorageError::Io)?.len();

    file.write_all(&len_prefix).map_err(StorageError::Io)?;
    file.write_all(&event_bytes).map_err(StorageError::Io)?;
    file.sync_all().map_err(StorageError::Io)?;

    Ok(offset)
}

pub fn read_event_at(
    events_dir: &Path,
    stream_id: &[u8],
    expected_seq: u64,
    file_offset: u64,
) -> Result<Option<EventEnvelope>, StorageError> {
    let stream_id_hex = edgerun_core::util::bytes_to_hex(stream_id);
    let log_path = events_dir.join(format!("{stream_id_hex}.log"));
    let mut file = File::open(&log_path)?;
    file.seek(SeekFrom::Start(file_offset))?;

    let Some(len) =
        edgerun_core::varint::decode_varint_from_read(&mut file).map_err(StorageError::Io)?
    else {
        return Ok(None);
    };

    let mut event_bytes = vec![0u8; len as usize];
    file.read_exact(&mut event_bytes)?;

    let event = proto_stream::EventEnvelope::decode(&event_bytes[..])
        .map_err(|e| StorageError::Decode(format!("event protobuf decode failed: {e}")))?;
    if event.seq != expected_seq {
        return Err(StorageError::Decode(format!(
            "event seq mismatch at offset {file_offset}: expected {expected_seq}, got {}",
            event.seq
        )));
    }

    Ok(Some(event))
}

pub fn scan_event_logs(events_dir: &Path) -> Result<Vec<ScannedEvent>, StorageError> {
    let mut scanned = Vec::new();

    if !events_dir.exists() {
        return Ok(scanned);
    }

    for entry in fs::read_dir(events_dir)? {
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
        let stream_id = match edgerun_core::util::hex_to_bytes(&stream_id_hex) {
            Ok(id) => id,
            Err(_) => continue,
        };

        let mut file = File::open(&path)?;
        loop {
            let record_start = file.stream_position()?;
            let len = match edgerun_core::varint::decode_varint_from_read(&mut file) {
                Ok(Some(v)) => v,
                Ok(None) => break,
                Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(e) => return Err(StorageError::Io(e)),
            };

            let mut event_bytes = vec![0u8; len as usize];
            file.read_exact(&mut event_bytes)?;

            let event = match proto_stream::EventEnvelope::decode(&event_bytes[..]) {
                Ok(event) => event,
                Err(_) => continue,
            };
            let event_hash = canonical_event_hash(&event).value;
            scanned.push(ScannedEvent {
                location: EventLocation {
                    stream_id: stream_id.clone(),
                    seq: event.seq,
                    event_hash,
                    file_offset: record_start,
                    envelope_version: event.envelope_version,
                },
                event,
            });
        }
    }

    Ok(scanned)
}
