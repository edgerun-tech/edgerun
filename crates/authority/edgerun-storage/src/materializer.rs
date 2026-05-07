//! Rebuildable projections derived from durable event-log records.
//!
//! This module may cache stream heads, event offsets, and operational lookup
//! tables. It does not create events, assign sequence numbers, sign envelopes,
//! or decide stream ordering.

use crate::prelude::v1::*;

use edgerun_protocols::core_protocol::protocol::EventEnvelope;
use std::sync::Arc;

use crate::core::canonical_event_hash;
use crate::error::StorageError;
use crate::file_index::FileIndex;

// ---------------------------------------------------------------------------
// Operational event types written as already-signed stream events.
// Protocol uses 1-7. Storage projections recognize 100+ for lookup caches.
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

pub(crate) fn materialize_event_to_index(
    index: &Arc<FileIndex>,
    event: &EventEnvelope,
    offset: u64,
) -> Result<(), StorageError> {
    let stream_id_hex = edgerun_protocols::core_protocol::util::bytes_to_hex(&event.stream_id);
    let event_hash = canonical_event_hash(event).value;

    index.put_event(
        &stream_id_hex,
        event.seq as i64,
        &event_hash,
        offset,
        event.envelope_version as i64,
    )?;

    index.set_head(&stream_id_hex, event.seq as i64, &event_hash)?;

    if let Some(op) = OpEventType::from_i32(event.event_type) {
        match op {
            OpEventType::CredentialStored => {
                if let Some(payload) = &event.payload_object {
                    let namespace =
                        edgerun_protocols::core_protocol::util::bytes_to_hex(&payload.object_id);
                    let name =
                        edgerun_protocols::core_protocol::util::bytes_to_hex(&event.stream_id);
                    let blob_id =
                        edgerun_protocols::core_protocol::util::bytes_to_hex(&payload.object_id);
                    let _ = index.put_credential(&namespace, &name, &blob_id, None);
                }
            }
            OpEventType::PeerDiscovered | OpEventType::PeerStatusChanged => {
                if let Some(payload) = &event.payload_object {
                    let node_id =
                        edgerun_protocols::core_protocol::util::bytes_to_hex(&payload.object_id);
                    let status = match op {
                        OpEventType::PeerDiscovered => "discovered",
                        _ => "status_changed",
                    };
                    let _ = index.upsert_peer(&node_id, None, status, false);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
