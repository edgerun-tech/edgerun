//! Filesystem event-log backend.

use crate::prelude::v1::*;
use edgerun_core::protocol::EventEnvelope;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::core::{
    canonical_event_hash, encode_event_frame, validate_event_location, AppendReceipt,
    EventLocation, EventLog, ScannedEvent,
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
        let mut file = open_stream_file(&self.events_dir, &event.stream_id)?;
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
        let Some(event) = read_event_at(&self.events_dir, stream_id, seq, location.file_offset)?
        else {
            return Ok(None);
        };

        validate_event_location(location, &event)?;

        Ok(Some(event))
    }

    fn scan(&self) -> Result<Vec<ScannedEvent>, StorageError> {
        scan_event_logs(&self.events_dir)
    }
}

pub fn open_stream_file(events_dir: &Path, stream_id: &[u8]) -> Result<File, StorageError> {
    let stream_id_hex = edgerun_core::util::bytes_to_hex(stream_id);
    let log_path = events_dir.join(format!("{stream_id_hex}.log"));
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(StorageError::Io)?;
    }
    Ok(OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(StorageError::Io)?)
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

    let Some(len) = decode_varint_from_read(&mut file)
        .map_err(varint_io_to_storage_io)?
    else {
        return Ok(None);
    };

    let mut event_bytes = vec![0u8; len as usize];
    file.read_exact(&mut event_bytes)?;

    let event = decode_event_envelope_wire(&event_bytes[..])?;
    if event.stream_id != stream_id {
        return Err(StorageError::Decode(format!(
            "event stream mismatch at offset {file_offset}: expected {}, got {}",
            stream_id_hex,
            edgerun_core::util::bytes_to_hex(&event.stream_id),
        )));
    }
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
        if path.extension().and_then(path_part_to_string) != Some("log".to_string()) {
            continue;
        }

        let stream_id_hex = path
            .file_stem()
            .and_then(path_part_to_string)
            .unwrap_or_else(String::new);
        let stream_id = match edgerun_core::util::hex_to_bytes(&stream_id_hex) {
            Ok(id) => id,
            Err(_) => continue,
        };

        let mut file = File::open(&path)?;
        loop {
            let record_start = file.stream_position()?;
            let len = match decode_varint_from_read(&mut file) {
                Ok(Some(v)) => v,
                Ok(None) => break,
                Err(e) if varint_is_unexpected_eof(&e) => break,
                Err(e) => return Err(varint_io_to_storage_io(e)),
            };

            let mut event_bytes = vec![0u8; len as usize];
            file.read_exact(&mut event_bytes)?;

            let event = decode_event_envelope_wire(&event_bytes[..])?;
            if event.stream_id != stream_id {
                return Err(StorageError::Decode(format!(
                    "event stream mismatch at offset {record_start}: expected {}, got {}",
                    stream_id_hex,
                    edgerun_core::util::bytes_to_hex(&event.stream_id),
                )));
            }
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

#[cfg(target_os = "none")]
fn path_part_to_string(part: crate::std_compat::path::PathPart) -> Option<String> {
    Some(part.into_string())
}

#[cfg(not(target_os = "none"))]
fn path_part_to_string(part: &std::ffi::OsStr) -> Option<String> {
    part.to_str().map(ToOwned::to_owned)
}

#[cfg(target_os = "none")]
fn varint_is_unexpected_eof(err: &edgerun_core::io::Error) -> bool {
    err.kind() == edgerun_core::io::ErrorKind::UnexpectedEof
}

#[cfg(not(target_os = "none"))]
fn varint_is_unexpected_eof(err: &std::io::Error) -> bool {
    err.kind() == std::io::ErrorKind::UnexpectedEof
}

#[cfg(target_os = "none")]
fn varint_io_to_storage_io(err: edgerun_core::io::Error) -> StorageError {
    let kind = match err.kind() {
        edgerun_core::io::ErrorKind::UnexpectedEof => std::io::ErrorKind::UnexpectedEof,
        edgerun_core::io::ErrorKind::InvalidData => std::io::ErrorKind::InvalidData,
        edgerun_core::io::ErrorKind::NotFound => std::io::ErrorKind::NotFound,
        edgerun_core::io::ErrorKind::Other => std::io::ErrorKind::Other,
    };
    StorageError::Io(std::io::Error::new(kind, err))
}

#[cfg(not(target_os = "none"))]
fn varint_io_to_storage_io(err: std::io::Error) -> StorageError {
    StorageError::Io(err)
}


fn decode_varint_from_read<R: Read>(r: &mut R) -> std::io::Result<Option<u64>> {
    let mut buf = [0u8; 1];
    let mut result: u64 = 0;
    let mut shift: u32 = 0;

    loop {
        match r.read_exact(&mut buf) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
                if shift == 0 {
                    return Ok(None);
                }
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "truncated varint",
                ));
            }
            Err(e) => return Err(e),
        }

        let byte = buf[0];
        result |= ((byte & 0x7f) as u64) << shift;

        if byte & 0x80 == 0 {
            return Ok(Some(result));
        }

        shift += 7;
        if shift >= 64 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "varint too long",
            ));
        }
    }
}

fn varint_is_unexpected_eof(err: &std::io::Error) -> bool {
    err.kind() == std::io::ErrorKind::UnexpectedEof
}

fn varint_io_to_storage_io(err: std::io::Error) -> StorageError {
    StorageError::Io(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_events_dir() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("fs_event_log_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn event(stream_id: &[u8], seq: u64) -> EventEnvelope {
        EventEnvelope {
            envelope_version: 1,
            stream_id: stream_id.to_vec(),
            seq,
            prev_event_hash: None,
            event_type: 1,
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
            ..Default::default()
        }
    }

    #[test]
    fn scan_rejects_malformed_event_frame() {
        let events_dir = tmp_events_dir();
        let log_path = events_dir.join("aa.log");
        std::fs::write(&log_path, [3u8, 0xff, 0xff, 0xff]).unwrap();

        let result = scan_event_logs(&events_dir);
        assert!(matches!(result, Err(StorageError::Decode(_))));

        let _ = std::fs::remove_dir_all(events_dir);
    }

    #[test]
    fn scan_rejects_event_in_wrong_stream_file() {
        let events_dir = tmp_events_dir();
        let event = event(&[0xbb], 0);
        let (len_prefix, event_bytes) = encode_event_frame(&event).unwrap();
        let mut frame = Vec::new();
        frame.extend_from_slice(&len_prefix);
        frame.extend_from_slice(&event_bytes);
        std::fs::write(events_dir.join("aa.log"), frame).unwrap();

        let result = scan_event_logs(&events_dir);
        assert!(matches!(result, Err(StorageError::Decode(_))));

        let _ = std::fs::remove_dir_all(events_dir);
    }

    #[test]
    fn read_event_rejects_location_hash_mismatch() {
        let events_dir = tmp_events_dir();
        let mut log = FsEventLog::new(events_dir.clone());
        let event = event(b"stream", 0);
        let receipt = log.append_event(&event).unwrap();

        let bad_location = EventLocation {
            stream_id: event.stream_id.clone(),
            seq: event.seq,
            event_hash: vec![0xff; 32],
            file_offset: receipt.file_offset,
            envelope_version: event.envelope_version,
        };
        let result = log.read_event(&event.stream_id, event.seq, &bad_location);
        assert!(matches!(result, Err(StorageError::Decode(_))));

        let _ = std::fs::remove_dir_all(events_dir);
    }
}

fn decode_event_envelope_wire(
    _bytes: &[u8],
) -> Result<edgerun_core::protocol::EventEnvelope, StorageError> {
    Err(StorageError::Decode(
        "native event wire decode not wired yet".to_string(),
    ))
}
