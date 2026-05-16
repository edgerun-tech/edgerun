use alloc::vec::Vec;

use crate::codec::{blake3_hash, wire_bytes, wire_from_bytes};
use crate::generated_wire::ArchivedCapabilityEnvelope;
use crate::preimage::HashBuilder;
use crate::protocol::*;

const CAPABILITY_ID_DOMAIN: &[u8] = b"edgerun:v1:work:capability-id";
const CAPABILITY_SESSION_DOMAIN: &[u8] = b"edgerun:v1:work:capability-session";
const CAPABILITY_INVOCATION_DOMAIN: &[u8] = b"edgerun:v1:work:capability-invocation";

pub const CAPABILITY_PACKET_REQUEST: u16 = 1;
pub const CAPABILITY_PACKET_INVOKE: u16 = 2;
pub const CAPABILITY_PACKET_EVENT: u16 = 3;
pub const CAPABILITY_PACKET_CLOSE: u16 = 4;

pub const CAPABILITY_CONTENT_OPAQUE: u16 = 0;
pub const CAPABILITY_CONTENT_CONTROL: u16 = 1;
pub const CAPABILITY_CONTENT_VIDEO: u16 = 2;
pub const CAPABILITY_CONTENT_AUDIO: u16 = 3;
pub const CAPABILITY_CONTENT_INPUT: u16 = 4;
pub const CAPABILITY_CONTENT_RENDER: u16 = 5;
pub const CAPABILITY_CONTENT_OBJECT: u16 = 6;

pub const CAPABILITY_OPERATION_CONTROL_OPEN: u16 = 1;
pub const CAPABILITY_OPERATION_CONTROL_CLOSE: u16 = 2;
pub const CAPABILITY_OPERATION_OBJECT_GET: u16 = 10;
pub const CAPABILITY_OPERATION_OBJECT_PUT: u16 = 11;
pub const CAPABILITY_OPERATION_OBJECT_DELETE: u16 = 12;
pub const CAPABILITY_OPERATION_STREAM_CHUNK: u16 = 20;
pub const CAPABILITY_OPERATION_STREAM_END: u16 = 21;
pub const CAPABILITY_OPERATION_INPUT_EVENT: u16 = 30;
pub const CAPABILITY_OPERATION_RENDER_COMMAND: u16 = 40;

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

pub fn encode_capability_envelope(
    envelope: &CapabilityEnvelope,
) -> Result<Vec<u8>, WorkProtocolError> {
    wire_bytes(envelope)
}

pub fn decode_capability_envelope(bytes: &[u8]) -> Result<CapabilityEnvelope, WorkProtocolError> {
    wire_from_bytes::<CapabilityEnvelope, ArchivedCapabilityEnvelope>(bytes)
}

pub fn capability_id_from_descriptor(provider_node_id: NodeId, descriptor: &[u8]) -> Hash {
    HashBuilder::domain(CAPABILITY_ID_DOMAIN)
        .node_id(&provider_node_id)
        .bytes(descriptor)
        .finish()
}

pub fn capability_session_id(
    admission_hash: Hash,
    source_node_id: NodeId,
    target_node_id: NodeId,
    capability_id: Hash,
    sequence: u64,
) -> Hash {
    HashBuilder::domain(CAPABILITY_SESSION_DOMAIN)
        .hash(&admission_hash)
        .node_id(&source_node_id)
        .node_id(&target_node_id)
        .hash(&capability_id)
        .u64(sequence)
        .finish()
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

pub fn capability_work_type_for_kind(kind: u16) -> Option<u16> {
    match kind {
        CAPABILITY_PACKET_REQUEST => Some(WORK_TYPE_CAPABILITY_REQUEST),
        CAPABILITY_PACKET_INVOKE => Some(WORK_TYPE_CAPABILITY_INVOKE),
        CAPABILITY_PACKET_EVENT => Some(WORK_TYPE_CAPABILITY_EVENT),
        CAPABILITY_PACKET_CLOSE => Some(WORK_TYPE_CAPABILITY_CLOSE),
        _ => None,
    }
}

pub fn capability_operation_matches_content(operation: u16, content_type: u16) -> bool {
    match operation {
        CAPABILITY_OPERATION_CONTROL_OPEN | CAPABILITY_OPERATION_CONTROL_CLOSE => {
            content_type == CAPABILITY_CONTENT_CONTROL || content_type == CAPABILITY_CONTENT_OPAQUE
        }
        CAPABILITY_OPERATION_OBJECT_GET
        | CAPABILITY_OPERATION_OBJECT_PUT
        | CAPABILITY_OPERATION_OBJECT_DELETE => content_type == CAPABILITY_CONTENT_OBJECT,
        CAPABILITY_OPERATION_STREAM_CHUNK | CAPABILITY_OPERATION_STREAM_END => matches!(
            content_type,
            CAPABILITY_CONTENT_VIDEO | CAPABILITY_CONTENT_AUDIO | CAPABILITY_CONTENT_OPAQUE
        ),
        CAPABILITY_OPERATION_INPUT_EVENT => content_type == CAPABILITY_CONTENT_INPUT,
        CAPABILITY_OPERATION_RENDER_COMMAND => content_type == CAPABILITY_CONTENT_RENDER,
        _ => false,
    }
}

pub fn verify_capability_envelope_shape(envelope: &CapabilityEnvelope) -> bool {
    envelope.abi_version == WORK_WIRE_ABI_VERSION
        && capability_work_type_for_kind(envelope.kind).is_some()
        && capability_operation_matches_content(envelope.operation, envelope.content_type)
        && envelope.payload_hash == blake3_hash(&envelope.payload)
}

pub fn verify_capability_message_payload(
    message: &NetworkMessage,
) -> Result<CapabilityEnvelope, WorkProtocolError> {
    if message.department != DEPARTMENT_CAPABILITY {
        return Err(WorkProtocolError::Unsupported);
    }
    let envelope = decode_capability_envelope(&message.payload)?;
    if !verify_capability_envelope_shape(&envelope) {
        return Err(WorkProtocolError::InvalidShape);
    }
    if capability_work_type_for_kind(envelope.kind) != Some(message.work_type)
        || envelope.source_node_id != message.from
        || envelope.target_node_id != message.to
        || message.payload_hash != blake3_hash(&message.payload)
    {
        return Err(WorkProtocolError::HashMismatch);
    }
    Ok(envelope)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signing::{empty_signature, sign_network_message_payload};

    #[test]
    fn capability_message_payload_verifies_source_target_kind_and_hash() {
        let source = [1u8; 32];
        let target = [2u8; 32];
        let capability_id = capability_id_from_descriptor(target, b"camera/front");
        let session_id = capability_session_id([3u8; 32], source, target, capability_id, 1);
        let payload = b"frame".to_vec();
        let invocation_id = capability_invocation_id(
            session_id,
            CAPABILITY_OPERATION_STREAM_CHUNK,
            8,
            blake3_hash(&payload),
        );
        let envelope = capability_envelope(
            session_id,
            invocation_id,
            capability_id,
            source,
            target,
            CAPABILITY_PACKET_EVENT,
            CAPABILITY_OPERATION_STREAM_CHUNK,
            CAPABILITY_CONTENT_VIDEO,
            8,
            9,
            payload,
        );
        let payload = encode_capability_envelope(&envelope).expect("encode capability envelope");
        let payload_hash = blake3_hash(&payload);
        let message = NetworkMessage {
            abi_version: WORK_WIRE_ABI_VERSION,
            message_id: [6u8; 32],
            prev_hash: [0u8; 32],
            from: source,
            to: target,
            via_relay: [7u8; 32],
            department: DEPARTMENT_CAPABILITY,
            work_type: WORK_TYPE_CAPABILITY_EVENT,
            sequence: 1,
            payload_hash,
            payload,
            signature: empty_signature(),
        };

        assert_eq!(
            verify_capability_message_payload(&message).expect("payload verifies"),
            envelope
        );

        let mut wrong_work_type = message.clone();
        wrong_work_type.work_type = WORK_TYPE_CAPABILITY_INVOKE;
        assert!(verify_capability_message_payload(&wrong_work_type).is_err());

        let mut wrong_hash = message;
        wrong_hash.payload_hash = [9u8; 32];
        assert!(verify_capability_message_payload(&wrong_hash).is_err());
    }

    #[test]
    fn signed_capability_message_payload_verifies() {
        let key = edgerun_crypto::Ed25519SigningKey::from_bytes(&[10u8; 32]);
        let source = *key.verifying_key().as_bytes();
        let target = [11u8; 32];
        let envelope = capability_envelope(
            [12u8; 32],
            [13u8; 32],
            [14u8; 32],
            source,
            target,
            CAPABILITY_PACKET_INVOKE,
            CAPABILITY_OPERATION_RENDER_COMMAND,
            CAPABILITY_CONTENT_RENDER,
            16,
            17,
            b"render".to_vec(),
        );
        let payload = encode_capability_envelope(&envelope).expect("encode capability envelope");
        let message = sign_network_message_payload(
            &key,
            [18u8; 32],
            [0u8; 32],
            source,
            target,
            [19u8; 32],
            DEPARTMENT_CAPABILITY,
            WORK_TYPE_CAPABILITY_INVOKE,
            1,
            payload,
        );

        assert_eq!(
            verify_capability_message_payload(&message).expect("payload verifies"),
            envelope
        );
    }
}
