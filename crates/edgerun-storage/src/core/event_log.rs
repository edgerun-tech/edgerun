//! Shared event-log framing and hashing.

use edgerun_core::protocol::{canonical_bytes, Digest, EventEnvelope, ProtocolRecord};
use edgerun_proto::edgerun::v0::stream as proto_stream;
use prost::Message;

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
/// test doubles, but they must preserve protocol canonical event hashes.
pub trait EventLog {
    fn append_event(&mut self, event: &EventEnvelope) -> Result<AppendReceipt, StorageError>;

    fn read_event(
        &self,
        stream_id: &[u8],
        seq: u64,
        location: &EventLocation,
    ) -> Result<Option<EventEnvelope>, StorageError>;

    fn scan(&self) -> Result<Vec<ScannedEvent>, StorageError>;
}

/// Computes the protocol canonical event hash used for stream linkage and heads.
#[must_use]
pub fn canonical_event_hash(event: &EventEnvelope) -> Digest {
    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, false);
    Digest {
        algorithm: 1,
        value: edgerun_core::crypto::sha256(&canonical).to_vec(),
    }
}

/// Encodes an event into the current hosted append-log frame:
/// `[varint protobuf_len][protobuf EventEnvelope bytes]`.
///
/// The record body remains protobuf for compatibility with the existing log
/// files. Integrity/index hashes are computed through `canonical_event_hash`.
pub fn encode_event_frame(event: &EventEnvelope) -> Result<(Vec<u8>, Vec<u8>), StorageError> {
    let proto: proto_stream::EventEnvelope = event.clone();
    let mut event_bytes = Vec::new();
    proto_stream::EventEnvelope::encode(&proto, &mut event_bytes)
        .map_err(|e| StorageError::Encode(format!("encode failed: {e}")))?;

    let len_prefix = edgerun_core::varint::encode_varint(event_bytes.len() as u64);
    Ok((len_prefix, event_bytes))
}
