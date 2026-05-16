use alloc::vec::Vec;

use crate::codec::blake3_hash;
use crate::preimage::HashBuilder;
use crate::protocol::{Hash, NodeId, WORK_WIRE_ABI_VERSION};

const CAPABILITY_INVOCATION_DOMAIN: &[u8] = b"edgerun:v1:work:capability-invocation";

pub const CAPABILITY_PACKET_INVOKE: u16 = 2;

pub const CAPABILITY_CONTENT_OBJECT: u16 = 6;

pub const CAPABILITY_OPERATION_OBJECT_GET: u16 = 10;
pub const CAPABILITY_OPERATION_OBJECT_PUT: u16 = 11;
pub const CAPABILITY_OPERATION_OBJECT_DELETE: u16 = 12;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityEnvelope {
    pub abi_version: u16,
    pub session_id: Hash,
    pub invocation_id: Hash,
    pub capability_id: Hash,
    pub source_node_id: NodeId,
    pub target_node_id: NodeId,
    pub kind: u16,
    pub operation: u16,
    pub content_type: u16,
    pub sequence: u64,
    pub timestamp_unix_ms: u64,
    pub payload_hash: Hash,
    pub payload: Vec<u8>,
}

#[allow(clippy::too_many_arguments)]
pub fn capability_envelope(
    session_id: Hash,
    invocation_id: Hash,
    capability_id: Hash,
    source_node_id: NodeId,
    target_node_id: NodeId,
    kind: u16,
    operation: u16,
    content_type: u16,
    sequence: u64,
    timestamp_unix_ms: u64,
    payload: Vec<u8>,
) -> CapabilityEnvelope {
    let payload_hash = blake3_hash(&payload);
    CapabilityEnvelope {
        abi_version: WORK_WIRE_ABI_VERSION,
        session_id,
        invocation_id,
        capability_id,
        source_node_id,
        target_node_id,
        kind,
        operation,
        content_type,
        sequence,
        timestamp_unix_ms,
        payload_hash,
        payload,
    }
}

pub fn capability_invocation_id(
    session_id: Hash,
    operation: u16,
    sequence: u64,
    payload_hash: Hash,
) -> Hash {
    HashBuilder::domain(CAPABILITY_INVOCATION_DOMAIN)
        .hash(&session_id)
        .u16(operation)
        .u64(sequence)
        .hash(&payload_hash)
        .finish()
}

pub fn capability_envelope_hash(envelope: &CapabilityEnvelope) -> Hash {
    HashBuilder::domain(b"edgerun:v1:work:capability-envelope")
        .u16(envelope.abi_version)
        .hash(&envelope.session_id)
        .hash(&envelope.invocation_id)
        .hash(&envelope.capability_id)
        .node_id(&envelope.source_node_id)
        .node_id(&envelope.target_node_id)
        .u16(envelope.kind)
        .u16(envelope.operation)
        .u16(envelope.content_type)
        .u64(envelope.sequence)
        .u64(envelope.timestamp_unix_ms)
        .hash(&envelope.payload_hash)
        .bytes(&envelope.payload)
        .finish()
}

pub fn capability_operation_matches_content(operation: u16, content_type: u16) -> bool {
    match operation {
        CAPABILITY_OPERATION_OBJECT_GET
        | CAPABILITY_OPERATION_OBJECT_PUT
        | CAPABILITY_OPERATION_OBJECT_DELETE => content_type == CAPABILITY_CONTENT_OBJECT,
        _ => false,
    }
}

pub fn verify_capability_envelope_shape(envelope: &CapabilityEnvelope) -> bool {
    envelope.abi_version == WORK_WIRE_ABI_VERSION
        && envelope.kind == CAPABILITY_PACKET_INVOKE
        && capability_operation_matches_content(envelope.operation, envelope.content_type)
        && envelope.payload_hash == blake3_hash(&envelope.payload)
}
