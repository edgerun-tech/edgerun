//! Filesystem event-log backend.

use crate::prelude::v1::*;
use edgerun_protocols::core_protocol::protocol::EventEnvelope;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::core::{
    canonical_event_hash, encode_event_frame, validate_event_location, AppendReceipt,
    EventLocation, EventLog, ScannedEvent,
};
use crate::error::StorageError;

fn varint_is_unexpected_eof(err: &std::io::Error) -> bool {
    err.kind() == std::io::ErrorKind::UnexpectedEof
}

fn varint_io_to_storage_io(err: std::io::Error) -> StorageError {
    StorageError::Io(err)
}

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
        append_event_to_file(&self.events_dir, &mut file, event)
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

    fn sync(&mut self) -> Result<(), StorageError> {
        fs::create_dir_all(&self.events_dir).map_err(StorageError::Io)
    }
}

pub fn open_stream_file(events_dir: &Path, stream_id: &[u8]) -> Result<File, StorageError> {
    let log_path = stream_log_path(events_dir, stream_id);
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent).map_err(StorageError::Io)?;
    }
    Ok(OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .map_err(StorageError::Io)?)
}

fn stream_log_path(events_dir: &Path, stream_id: &[u8]) -> PathBuf {
    let stream_id_hex = edgerun_protocols::core_protocol::util::bytes_to_hex(stream_id);
    events_dir.join(format!("{stream_id_hex}.log"))
}

pub fn append_event_to_file(
    events_dir: &Path,
    file: &mut File,
    event: &EventEnvelope,
) -> Result<AppendReceipt, StorageError> {
    let event_hash = canonical_event_hash(event).value;
    let scanned = scan_one_stream_log(events_dir, &event.stream_id)?;

    if let Some(existing) = scanned
        .iter()
        .find(|scanned| scanned.location.seq == event.seq)
    {
        if existing.location.event_hash == event_hash {
            return Ok(AppendReceipt {
                stream_id: existing.location.stream_id.clone(),
                seq: existing.location.seq,
                event_hash: existing.location.event_hash.clone(),
                file_offset: existing.location.file_offset,
                envelope_version: existing.location.envelope_version,
            });
        }

        return Err(StorageError::Stream(format!(
            "refusing to append stream {} seq {}: seq already exists with a different hash",
            edgerun_protocols::core_protocol::util::bytes_to_hex(&event.stream_id),
            event.seq
        )));
    }

    validate_event_follows_head(event, scanned.last())?;

    let offset = write_event_to_file(file, event)?;
    Ok(AppendReceipt {
        stream_id: event.stream_id.clone(),
        seq: event.seq,
        event_hash,
        file_offset: offset,
        envelope_version: event.envelope_version,
    })
}

fn write_event_to_file(file: &mut File, event: &EventEnvelope) -> Result<u64, StorageError> {
    let (len_prefix, event_bytes) = encode_event_frame(event)?;

    let offset = file.metadata().map_err(StorageError::Io)?.len();

    file.write_all(&len_prefix).map_err(StorageError::Io)?;
    file.write_all(&event_bytes).map_err(StorageError::Io)?;
    file.sync_all().map_err(StorageError::Io)?;

    Ok(offset)
}

fn validate_event_follows_head(
    event: &EventEnvelope,
    head: Option<&ScannedEvent>,
) -> Result<(), StorageError> {
    let stream_id_hex = edgerun_protocols::core_protocol::util::bytes_to_hex(&event.stream_id);

    match head {
        None => {
            if event.seq != 0 {
                return Err(StorageError::Stream(format!(
                    "refusing to append stream {stream_id_hex} seq {}: first event must have seq 0",
                    event.seq
                )));
            }
            if event.prev_event_hash.is_some() {
                return Err(StorageError::Stream(format!(
                    "refusing to append stream {stream_id_hex} seq 0: genesis event must not have prev_event_hash"
                )));
            }
            Ok(())
        }
        Some(head) => {
            let expected_seq = head.location.seq.saturating_add(1);
            if event.seq != expected_seq {
                return Err(StorageError::Stream(format!(
                    "refusing to append stream {stream_id_hex} seq {}: expected next seq {expected_seq}",
                    event.seq
                )));
            }

            let Some(prev_event_hash) = &event.prev_event_hash else {
                return Err(StorageError::Stream(format!(
                    "refusing to append stream {stream_id_hex} seq {}: missing prev_event_hash",
                    event.seq
                )));
            };
            if prev_event_hash.algorithm != 1 || prev_event_hash.value != head.location.event_hash {
                return Err(StorageError::Stream(format!(
                    "refusing to append stream {stream_id_hex} seq {}: prev_event_hash does not match seq {}",
                    event.seq, head.location.seq
                )));
            }
            Ok(())
        }
    }
}

pub fn read_event_at(
    events_dir: &Path,
    stream_id: &[u8],
    expected_seq: u64,
    file_offset: u64,
) -> Result<Option<EventEnvelope>, StorageError> {
    let stream_id_hex = edgerun_protocols::core_protocol::util::bytes_to_hex(stream_id);
    let log_path = events_dir.join(format!("{stream_id_hex}.log"));
    let mut file = File::open(&log_path)?;
    file.seek(SeekFrom::Start(file_offset))?;

    let Some(len) = decode_varint_from_read(&mut file).map_err(varint_io_to_storage_io)? else {
        return Ok(None);
    };

    let mut event_bytes = vec![0u8; len as usize];
    file.read_exact(&mut event_bytes)?;

    let event = decode_event_envelope_wire(&event_bytes[..])?;
    if event.stream_id != stream_id {
        return Err(StorageError::Decode(format!(
            "event stream mismatch at offset {file_offset}: expected {}, got {}",
            stream_id_hex,
            edgerun_protocols::core_protocol::util::bytes_to_hex(&event.stream_id),
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
        let stream_id = match edgerun_protocols::core_protocol::util::hex_to_bytes(&stream_id_hex) {
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
                    edgerun_protocols::core_protocol::util::bytes_to_hex(&event.stream_id),
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

fn scan_one_stream_log(
    events_dir: &Path,
    stream_id: &[u8],
) -> Result<Vec<ScannedEvent>, StorageError> {
    let log_path = stream_log_path(events_dir, stream_id);
    if !log_path.exists() {
        return Ok(Vec::new());
    }

    let mut scanned = Vec::new();
    let mut file = File::open(&log_path)?;
    let stream_id_hex = edgerun_protocols::core_protocol::util::bytes_to_hex(stream_id);

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
                edgerun_protocols::core_protocol::util::bytes_to_hex(&event.stream_id),
            )));
        }
        validate_event_follows_head(&event, scanned.last())?;

        let event_hash = canonical_event_hash(&event).value;
        scanned.push(ScannedEvent {
            location: EventLocation {
                stream_id: stream_id.to_vec(),
                seq: event.seq,
                event_hash,
                file_offset: record_start,
                envelope_version: event.envelope_version,
            },
            event,
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_protocols::core_protocol::protocol::Digest;

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

    fn with_prev_hash(mut event: EventEnvelope, prev: &[u8]) -> EventEnvelope {
        event.prev_event_hash = Some(Digest {
            algorithm: 1,
            value: prev.to_vec(),
        });
        event
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

    #[test]
    fn append_rejects_non_genesis_gap() {
        let events_dir = tmp_events_dir();
        let mut log = FsEventLog::new(events_dir.clone());
        let result = log.append_event(&event(b"stream", 1));

        assert!(matches!(result, Err(StorageError::Stream(_))));

        let _ = std::fs::remove_dir_all(events_dir);
    }

    #[test]
    fn append_rejects_missing_prev_hash() {
        let events_dir = tmp_events_dir();
        let mut log = FsEventLog::new(events_dir.clone());
        let genesis = event(b"stream", 0);
        log.append_event(&genesis).unwrap();

        let result = log.append_event(&event(b"stream", 1));

        assert!(matches!(result, Err(StorageError::Stream(_))));

        let _ = std::fs::remove_dir_all(events_dir);
    }

    #[test]
    fn append_rejects_wrong_prev_hash() {
        let events_dir = tmp_events_dir();
        let mut log = FsEventLog::new(events_dir.clone());
        let genesis = event(b"stream", 0);
        log.append_event(&genesis).unwrap();

        let next = with_prev_hash(event(b"stream", 1), &[0xff; 32]);
        let result = log.append_event(&next);

        assert!(matches!(result, Err(StorageError::Stream(_))));

        let _ = std::fs::remove_dir_all(events_dir);
    }

    #[test]
    fn append_accepts_hash_linked_next_event() {
        let events_dir = tmp_events_dir();
        let mut log = FsEventLog::new(events_dir.clone());
        let genesis = event(b"stream", 0);
        let genesis_hash = log.append_event(&genesis).unwrap().event_hash;

        let next = with_prev_hash(event(b"stream", 1), &genesis_hash);
        let receipt = log.append_event(&next).unwrap();

        assert_eq!(receipt.seq, 1);
        assert_eq!(log.scan().unwrap().len(), 2);

        let _ = std::fs::remove_dir_all(events_dir);
    }

    #[test]
    fn append_is_idempotent_for_same_seq_same_hash() {
        let events_dir = tmp_events_dir();
        let mut log = FsEventLog::new(events_dir.clone());
        let genesis = event(b"stream", 0);

        let first = log.append_event(&genesis).unwrap();
        let second = log.append_event(&genesis).unwrap();

        assert_eq!(first, second);
        assert_eq!(log.scan().unwrap().len(), 1);

        let _ = std::fs::remove_dir_all(events_dir);
    }

    #[test]
    fn append_rejects_same_seq_different_hash() {
        let events_dir = tmp_events_dir();
        let mut log = FsEventLog::new(events_dir.clone());
        let genesis = event(b"stream", 0);
        log.append_event(&genesis).unwrap();

        let mut conflicting = event(b"stream", 0);
        conflicting.event_type = 2;
        let result = log.append_event(&conflicting);

        assert!(matches!(result, Err(StorageError::Stream(_))));
        assert_eq!(log.scan().unwrap().len(), 1);

        let _ = std::fs::remove_dir_all(events_dir);
    }
}

fn decode_event_envelope_wire(
    bytes: &[u8],
) -> Result<edgerun_protocols::core_protocol::protocol::EventEnvelope, StorageError> {
    edgerun_protocols::core_protocol::wire_stream::decode_event_full_wire_bytes(bytes)
        .map_err(|e| StorageError::Decode(e.to_string()))
}
