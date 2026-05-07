//! Memory-only event-log backend used for trait validation and tests.

use crate::prelude::v1::*;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use edgerun_protocols::core_protocol::protocol::EventEnvelope;

use crate::core::{
    AppendReceipt, EventLocation, EventLog, ScannedEvent, canonical_event_hash, encode_event_frame,
    validate_event_location,
};
use crate::error::StorageError;

#[derive(Clone, Debug)]
struct StoredEvent {
    event: EventEnvelope,
    offset: u64,
    hash: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
struct StreamBuffer {
    events: VecDeque<StoredEvent>,
    next_offset: u64,
}

/// Thread-safe in-memory event log for tests and lightweight backends.
#[derive(Clone, Default)]
pub struct MemEventLog {
    streams: Arc<Mutex<BTreeMap<Vec<u8>, StreamBuffer>>>,
}

impl MemEventLog {
    pub fn new() -> Self {
        Self::default()
    }
}

impl EventLog for MemEventLog {
    fn append_event(&mut self, event: &EventEnvelope) -> Result<AppendReceipt, StorageError> {
        let mut streams = self
            .streams
            .lock()
            .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?;

        let frame = encode_event_frame(event)?;
        let (frame_prefix, frame_body) = frame;
        let frame_size = frame_prefix.len() + frame_body.len();

        let stream_buffer = streams.entry(event.stream_id.clone()).or_default();
        let offset = stream_buffer.next_offset;
        stream_buffer.next_offset = stream_buffer.next_offset.saturating_add(frame_size as u64);

        let event_hash = canonical_event_hash(event).value;
        stream_buffer.events.push_back(StoredEvent {
            event: event.clone(),
            offset,
            hash: event_hash.clone(),
        });

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
        let streams = self
            .streams
            .lock()
            .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?;

        let Some(stream) = streams.get(stream_id) else {
            return Ok(None);
        };

        let Some(found) = stream.events.iter().find(|event| event.event.seq == seq) else {
            return Ok(None);
        };

        if found.offset != location.file_offset {
            return Err(StorageError::Decode(format!(
                "event offset mismatch at seq {seq}: expected {}, got {}",
                location.file_offset, found.offset
            )));
        }
        validate_event_location(location, &found.event)?;

        Ok(Some(found.event.clone()))
    }

    fn scan(&self) -> Result<Vec<ScannedEvent>, StorageError> {
        let streams = self
            .streams
            .lock()
            .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?;

        let mut scanned = Vec::new();
        for (stream_id, stream) in streams.iter() {
            for event in &stream.events {
                scanned.push(ScannedEvent {
                    location: EventLocation {
                        stream_id: stream_id.clone(),
                        seq: event.event.seq,
                        event_hash: event.hash.clone(),
                        file_offset: event.offset,
                        envelope_version: event.event.envelope_version,
                    },
                    event: event.event.clone(),
                });
            }
        }

        Ok(scanned)
    }

    fn sync(&mut self) -> Result<(), StorageError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(stream_id: &[u8], seq: u64) -> EventEnvelope {
        EventEnvelope {
            envelope_version: 1,
            event_version: 1,
            stream_id: stream_id.to_vec(),
            seq,
            prev_event_hash: None,
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

    #[test]
    fn append_read_and_scan_are_consistent() {
        let mut log = MemEventLog::new();
        let s1 = b"stream-alpha";
        let s2 = b"stream-beta";

        let e1 = event(s1, 0);
        let e2 = event(s1, 1);
        let e3 = event(s2, 0);

        let r1 = log.append_event(&e1).unwrap();
        let r2 = log.append_event(&e2).unwrap();
        let r3 = log.append_event(&e3).unwrap();

        let location = EventLocation {
            stream_id: e1.stream_id.clone(),
            seq: 0,
            event_hash: r1.event_hash,
            file_offset: r1.file_offset,
            envelope_version: e1.envelope_version,
        };
        let read = log
            .read_event(&e1.stream_id, 0, &location)
            .unwrap()
            .unwrap();
        assert_eq!(read.seq, 0);
        assert_eq!(read.event_version, 1);

        let scanned = log.scan().unwrap();
        assert_eq!(scanned.len(), 3);

        assert_eq!(
            scanned
                .iter()
                .map(|s| s.location.file_offset)
                .collect::<Vec<_>>(),
            vec![r1.file_offset, r2.file_offset, r3.file_offset]
        );
    }

    #[test]
    fn append_does_not_author_stream_state() {
        let mut log = MemEventLog::new();
        let event = event(b"stream", 42);

        let receipt = log.append_event(&event).unwrap();
        let scanned = log.scan().unwrap();

        assert_eq!(receipt.seq, 42);
        assert_eq!(scanned[0].event.seq, 42);
        assert!(scanned[0].event.prev_event_hash.is_none());
        assert!(scanned[0].event.signature.is_none());
    }

    #[test]
    fn read_mismatched_offset_is_error() {
        let mut log = MemEventLog::new();
        let e1 = event(b"stream", 0);
        let receipt = log.append_event(&e1).unwrap();
        let bad = EventLocation {
            stream_id: e1.stream_id.clone(),
            seq: 0,
            event_hash: receipt.event_hash,
            file_offset: receipt.file_offset + 1,
            envelope_version: e1.envelope_version,
        };
        let result = log.read_event(&e1.stream_id, 0, &bad);
        assert!(result.is_err());
    }

    #[test]
    fn read_mismatched_hash_is_error() {
        let mut log = MemEventLog::new();
        let e1 = event(b"stream", 0);
        let receipt = log.append_event(&e1).unwrap();
        let bad = EventLocation {
            stream_id: e1.stream_id.clone(),
            seq: 0,
            event_hash: vec![0xff; 32],
            file_offset: receipt.file_offset,
            envelope_version: e1.envelope_version,
        };
        let result = log.read_event(&e1.stream_id, 0, &bad);
        assert!(result.is_err());
    }
}
