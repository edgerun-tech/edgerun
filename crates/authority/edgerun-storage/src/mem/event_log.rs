//! Memory-only event-log backend used for trait validation and tests.

use crate::prelude::v1::*;
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use edgerun_protocols::core_protocol::protocol::EventEnvelope;

use crate::core::{
    canonical_event_hash, encode_event_frame, validate_event_location, AppendReceipt,
    EventLocation, EventLog, ScannedEvent,
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
        let event_hash = canonical_event_hash(event).value;

        if let Some(existing) = stream_buffer
            .events
            .iter()
            .find(|stored| stored.event.seq == event.seq)
        {
            if existing.hash == event_hash {
                return Ok(AppendReceipt {
                    stream_id: event.stream_id.clone(),
                    seq: event.seq,
                    event_hash,
                    file_offset: existing.offset,
                    envelope_version: existing.event.envelope_version,
                });
            }

            return Err(StorageError::Stream(format!(
                "refusing to append stream {} seq {}: seq already exists with a different hash",
                edgerun_protocols::core_protocol::util::bytes_to_hex(&event.stream_id),
                event.seq
            )));
        }

        validate_event_follows_head(event, stream_buffer.events.back())?;

        let offset = stream_buffer.next_offset;
        stream_buffer.next_offset = stream_buffer.next_offset.saturating_add(frame_size as u64);

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

fn validate_event_follows_head(
    event: &EventEnvelope,
    head: Option<&StoredEvent>,
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
            let expected_seq = head.event.seq.saturating_add(1);
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
            if prev_event_hash.algorithm != 1 || prev_event_hash.value != head.hash {
                return Err(StorageError::Stream(format!(
                    "refusing to append stream {stream_id_hex} seq {}: prev_event_hash does not match seq {}",
                    event.seq, head.event.seq
                )));
            }
            Ok(())
        }
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

    fn with_prev_hash(mut event: EventEnvelope, prev: &[u8]) -> EventEnvelope {
        event.prev_event_hash = Some(edgerun_protocols::core_protocol::protocol::Digest {
            algorithm: 1,
            value: prev.to_vec(),
        });
        event
    }

    #[test]
    fn append_read_and_scan_are_consistent() {
        let mut log = MemEventLog::new();
        let s1 = b"stream-alpha";
        let s2 = b"stream-beta";

        let e1 = event(s1, 0);
        let e1_hash = canonical_event_hash(&e1).value;
        let e2 = with_prev_hash(event(s1, 1), &e1_hash);
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
    fn append_rejects_non_genesis_gap() {
        let mut log = MemEventLog::new();
        let event = event(b"stream", 42);

        let result = log.append_event(&event);
        assert!(matches!(result, Err(StorageError::Stream(_))));
        assert!(log.scan().unwrap().is_empty());
    }

    #[test]
    fn append_rejects_wrong_prev_hash() {
        let mut log = MemEventLog::new();
        let genesis = event(b"stream", 0);
        let next = with_prev_hash(event(b"stream", 1), &[0xAA; 32]);

        log.append_event(&genesis).unwrap();
        assert!(matches!(
            log.append_event(&next),
            Err(StorageError::Stream(_))
        ));
        assert_eq!(log.scan().unwrap().len(), 1);
    }

    #[test]
    fn append_is_idempotent_for_same_seq_same_hash() {
        let mut log = MemEventLog::new();
        let event = event(b"stream", 0);

        let first = log.append_event(&event).unwrap();
        let second = log.append_event(&event).unwrap();

        assert_eq!(first, second);
        assert_eq!(log.scan().unwrap().len(), 1);
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
