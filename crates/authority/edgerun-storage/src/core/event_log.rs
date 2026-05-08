//! Shared event-log framing and hashing.

use crate::prelude::v1::*;
use edgerun_protocols::core_protocol::protocol::{Digest, EventEnvelope};

use crate::error::StorageError;

/// Result returned by durable event-log append implementations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppendReceipt {
    pub stream_id: Vec<u8>,
    pub seq: u64,
    pub event_hash: Vec<u8>,
    pub file_offset: u64,
    pub envelope_version: u32,
}

/// Current durable head for a stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamHead {
    pub seq: u64,
    pub event_hash: Vec<u8>,
}

/// Physical location of an event record in an append log.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventLocation {
    pub stream_id: Vec<u8>,
    pub seq: u64,
    pub event_hash: Vec<u8>,
    pub file_offset: u64,
    pub envelope_version: u32,
}

/// One record yielded while scanning a durable event log.
#[derive(Clone, Debug, PartialEq)]
pub struct ScannedEvent {
    pub location: EventLocation,
    pub event: EventEnvelope,
}

/// Backend contract for durable append-only event logs.
///
/// Implementations may be filesystem-backed, block-device-backed, or in-memory
/// test doubles, but they must preserve deterministic edgerun-wire event hashes.
pub trait EventLog {
    fn append_event(&mut self, event: &EventEnvelope) -> Result<AppendReceipt, StorageError>;

    fn read_event(
        &self,
        stream_id: &[u8],
        seq: u64,
        location: &EventLocation,
    ) -> Result<Option<EventEnvelope>, StorageError>;

    fn scan(&self) -> Result<Vec<ScannedEvent>, StorageError>;

    fn sync(&mut self) -> Result<(), StorageError>;
}

/// Computes the deterministic edgerun-wire event hash used for stream linkage and heads.
#[must_use]
pub fn canonical_event_hash(event: &EventEnvelope) -> Digest {
    let canonical = edgerun_protocols::core_protocol::wire_stream::event_signable_wire_bytes(event);
    let hash = edgerun_protocols::core_protocol::crypto::record_hash(
        edgerun_protocols::core_protocol::crypto::HASH_DOMAIN_EVENT_ENVELOPE,
        &canonical,
    );
    Digest {
        algorithm: 1,
        value: hash.to_vec(),
    }
}

/// Verifies an event read from a log still matches its indexed location.
pub fn validate_event_location(
    location: &EventLocation,
    event: &EventEnvelope,
) -> Result<(), StorageError> {
    if location.stream_id != event.stream_id {
        return Err(StorageError::Decode(format!(
            "event location stream mismatch at offset {}: location has {}, event has {}",
            location.file_offset,
            edgerun_protocols::core_protocol::util::bytes_to_hex(&location.stream_id),
            edgerun_protocols::core_protocol::util::bytes_to_hex(&event.stream_id),
        )));
    }
    if location.seq != event.seq {
        return Err(StorageError::Decode(format!(
            "event location seq mismatch at offset {}: location has {}, event has {}",
            location.file_offset, location.seq, event.seq
        )));
    }
    let event_hash = canonical_event_hash(event).value;
    if location.event_hash != event_hash {
        return Err(StorageError::Decode(format!(
            "event hash mismatch at offset {}: location has {}, event has {}",
            location.file_offset,
            edgerun_protocols::core_protocol::util::bytes_to_hex(&location.event_hash),
            edgerun_protocols::core_protocol::util::bytes_to_hex(&event_hash),
        )));
    }

    Ok(())
}

/// Encodes an event into the current hosted append-log frame:
/// `[varint wire_len][edgerun-wire EventEnvelope bytes]`.
///
/// The record body is edgerun-wire. Integrity/index hashes are computed through
/// `canonical_event_hash`.
pub fn encode_event_frame(event: &EventEnvelope) -> Result<(Vec<u8>, Vec<u8>), StorageError> {
    let event_bytes = edgerun_protocols::core_protocol::wire_stream::event_full_wire_bytes(event);

    let len_prefix = edgerun_encoding::varint::encode_varint(event_bytes.len() as u64);
    Ok((len_prefix, event_bytes))
}

#[cfg(test)]
#[path = "event_log_tests.rs"]
mod event_log_tests;
