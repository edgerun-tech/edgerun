//! Internal protocol boundary for edgerun-wire.
//!
//! Generated protobuf structs are compatibility/import-export types only. These
//! structs are the first internal equivalents used by stream/node/exchange
//! migration work. They encode with edgerun-wire's deterministic hand-written
//! binary format and keep prost out of canonical storage decisions.

use alloc::string::String;
use alloc::vec::Vec;

use edgerun_wire::{field, struct_value, u64v, bytes, WireEncode, WireValue};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DigestWire {
    pub algorithm: u32,
    pub value: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignatureWire {
    pub algorithm: u32,
    pub value: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityRefWire {
    pub identity_id: Vec<u8>,
    pub identity_kind: Option<u32>,
    pub key_hint: Option<Vec<u8>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeRefWire {
    pub node_id: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StreamRefWire {
    pub stream_id: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectRefWire {
    pub object_id: Vec<u8>,
    pub object_kind: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandRefWire {
    pub command_id: Vec<u8>,
    pub command_hash: Option<DigestWire>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventRefWire {
    pub stream_id: Vec<u8>,
    pub seq: u64,
    pub event_hash: Option<DigestWire>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HeadRefWire {
    pub stream_id: Vec<u8>,
    pub seq: u64,
    pub event_hash: Option<DigestWire>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CommandPayloadWire {
    Object(ObjectRefWire),
    Inline(Vec<u8>),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandEnvelopeWire {
    pub envelope_version: u32,
    pub command_id: Vec<u8>,
    pub target_node: Option<NodeRefWire>,
    pub issuer: Option<IdentityRefWire>,
    pub command_type: u32,
    pub command_version: u32,
    pub idempotency_key: Vec<u8>,
    pub payload: Option<CommandPayloadWire>,
    pub command_metadata: Option<ObjectRefWire>,
    pub signature: Option<SignatureWire>,
    pub app_intent: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandResultPayloadWire {
    pub payload_version: u32,
    pub command: Option<CommandRefWire>,
    pub issuer: Option<IdentityRefWire>,
    pub decision: u32,
    pub decision_basis: Option<ObjectRefWire>,
    pub reason_code: String,
    pub effect_summary_object: Option<ObjectRefWire>,
    pub result_object: Option<ObjectRefWire>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventEnvelopeWire {
    pub envelope_version: u32,
    pub stream_id: Vec<u8>,
    pub seq: u64,
    pub prev_event_hash: Option<DigestWire>,
    pub event_type: u32,
    pub event_version: u32,
    pub payload_object: Option<ObjectRefWire>,
    pub related_events: Vec<EventRefWire>,
    pub related_commands: Vec<CommandRefWire>,
    pub related_objects: Vec<ObjectRefWire>,
    pub event_metadata: Option<ObjectRefWire>,
    pub signature: Option<SignatureWire>,
}

impl WireEncode for DigestWire {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        struct_value(vec![field(1, u64v(self.algorithm as u64)), field(2, bytes(&self.value))]).encode_wire(out);
    }
}

impl WireEncode for SignatureWire {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        struct_value(vec![field(1, u64v(self.algorithm as u64)), field(2, bytes(&self.value))]).encode_wire(out);
    }
}

impl WireEncode for IdentityRefWire {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        let mut fields = vec![field(1, bytes(&self.identity_id))];
        if let Some(kind) = self.identity_kind { fields.push(field(2, u64v(kind as u64))); }
        if let Some(key_hint) = &self.key_hint { fields.push(field(3, bytes(key_hint))); }
        struct_value(fields).encode_wire(out);
    }
}

impl WireEncode for NodeRefWire {
    fn encode_wire(&self, out: &mut Vec<u8>) { struct_value(vec![field(1, bytes(&self.node_id))]).encode_wire(out); }
}

impl WireEncode for StreamRefWire {
    fn encode_wire(&self, out: &mut Vec<u8>) { struct_value(vec![field(1, bytes(&self.stream_id))]).encode_wire(out); }
}

impl WireEncode for ObjectRefWire {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        let mut fields = vec![field(1, bytes(&self.object_id))];
        if let Some(kind) = self.object_kind { fields.push(field(2, u64v(kind as u64))); }
        struct_value(fields).encode_wire(out);
    }
}

impl WireEncode for CommandRefWire {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        let mut fields = vec![field(1, bytes(&self.command_id))];
        if let Some(hash) = &self.command_hash { fields.push(field(2, wire_value(hash))); }
        struct_value(fields).encode_wire(out);
    }
}

impl WireEncode for EventRefWire {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        let mut fields = vec![field(1, bytes(&self.stream_id)), field(2, u64v(self.seq))];
        if let Some(hash) = &self.event_hash { fields.push(field(3, wire_value(hash))); }
        struct_value(fields).encode_wire(out);
    }
}

impl WireEncode for HeadRefWire {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        let mut fields = vec![field(1, bytes(&self.stream_id)), field(2, u64v(self.seq))];
        if let Some(hash) = &self.event_hash { fields.push(field(3, wire_value(hash))); }
        struct_value(fields).encode_wire(out);
    }
}

impl WireEncode for CommandPayloadWire {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        match self {
            Self::Object(value) => struct_value(vec![field(1, wire_value(value))]).encode_wire(out),
            Self::Inline(value) => struct_value(vec![field(2, bytes(value))]).encode_wire(out),
        }
    }
}

impl WireEncode for CommandEnvelopeWire {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        let mut fields = vec![
            field(1, u64v(self.envelope_version as u64)),
            field(2, bytes(&self.command_id)),
            field(5, u64v(self.command_type as u64)),
            field(6, u64v(self.command_version as u64)),
            field(10, bytes(&self.idempotency_key)),
            field(17, bytes(&self.app_intent)),
        ];
        if let Some(value) = &self.target_node { fields.push(field(3, wire_value(value))); }
        if let Some(value) = &self.issuer { fields.push(field(4, wire_value(value))); }
        if let Some(value) = &self.payload { fields.push(field(11, wire_value(value))); }
        if let Some(value) = &self.command_metadata { fields.push(field(15, wire_value(value))); }
        if let Some(value) = &self.signature { fields.push(field(16, wire_value(value))); }
        struct_value(fields).encode_wire(out);
    }
}

impl CommandEnvelopeWire {
    pub fn signable_bytes(&self) -> Vec<u8> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        edgerun_wire::canonical_bytes(&unsigned)
    }
}

impl WireEncode for CommandResultPayloadWire {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        let mut fields = vec![
            field(1, u64v(self.payload_version as u64)),
            field(4, u64v(self.decision as u64)),
            field(6, edgerun_wire::text(&self.reason_code)),
        ];
        if let Some(value) = &self.command { fields.push(field(2, wire_value(value))); }
        if let Some(value) = &self.issuer { fields.push(field(3, wire_value(value))); }
        if let Some(value) = &self.decision_basis { fields.push(field(5, wire_value(value))); }
        if let Some(value) = &self.effect_summary_object { fields.push(field(7, wire_value(value))); }
        if let Some(value) = &self.result_object { fields.push(field(8, wire_value(value))); }
        struct_value(fields).encode_wire(out);
    }
}

impl WireEncode for EventEnvelopeWire {
    fn encode_wire(&self, out: &mut Vec<u8>) {
        let mut fields = vec![
            field(1, u64v(self.envelope_version as u64)),
            field(2, bytes(&self.stream_id)),
            field(3, u64v(self.seq)),
            field(5, u64v(self.event_type as u64)),
            field(6, u64v(self.event_version as u64)),
            field(10, list_value(&self.related_events)),
            field(11, list_value(&self.related_commands)),
            field(12, list_value(&self.related_objects)),
        ];
        if let Some(value) = &self.prev_event_hash { fields.push(field(4, wire_value(value))); }
        if let Some(value) = &self.payload_object { fields.push(field(9, wire_value(value))); }
        if let Some(value) = &self.event_metadata { fields.push(field(15, wire_value(value))); }
        if let Some(value) = &self.signature { fields.push(field(16, wire_value(value))); }
        struct_value(fields).encode_wire(out);
    }
}

impl EventEnvelopeWire {
    pub fn signable_bytes(&self) -> Vec<u8> {
        let mut unsigned = self.clone();
        unsigned.signature = None;
        edgerun_wire::canonical_bytes(&unsigned)
    }
}

fn wire_value<T: WireEncode>(value: &T) -> WireValue {
    let mut out = Vec::new();
    value.encode_wire(&mut out);
    WireValue::Bytes(out)
}

fn list_value<T: WireEncode>(values: &[T]) -> WireValue {
    let mut out = Vec::with_capacity(values.len());
    for value in values {
        out.push(wire_value(value));
    }
    WireValue::List(out)
}

pub mod proto_boundary {
    use super::*;
    use edgerun_proto::edgerun::v0::{common, stream};

    pub fn digest_from_proto(value: common::Digest) -> DigestWire {
        DigestWire { algorithm: value.algorithm as u32, value: value.value }
    }

    pub fn signature_from_proto(value: common::Signature) -> SignatureWire {
        SignatureWire { algorithm: value.algorithm as u32, value: value.value }
    }

    pub fn identity_ref_from_proto(value: common::IdentityRef) -> IdentityRefWire {
        IdentityRefWire { identity_id: value.identity_id, identity_kind: value.identity_kind.map(|v| v as u32), key_hint: value.key_hint }
    }

    pub fn node_ref_from_proto(value: common::NodeRef) -> NodeRefWire { NodeRefWire { node_id: value.node_id } }

    pub fn object_ref_from_proto(value: common::ObjectRef) -> ObjectRefWire {
        ObjectRefWire { object_id: value.object_id, object_kind: value.object_kind.map(|v| v as u32) }
    }

    pub fn command_ref_from_proto(value: common::CommandRef) -> CommandRefWire {
        CommandRefWire { command_id: value.command_id, command_hash: value.command_hash.map(digest_from_proto) }
    }

    pub fn event_ref_from_proto(value: common::EventRef) -> EventRefWire {
        EventRefWire { stream_id: value.stream_id, seq: value.seq, event_hash: value.event_hash.map(digest_from_proto) }
    }

    pub fn command_result_from_proto(value: stream::CommandResultPayload) -> CommandResultPayloadWire {
        CommandResultPayloadWire {
            payload_version: value.payload_version,
            command: value.command.map(command_ref_from_proto),
            issuer: value.issuer.map(identity_ref_from_proto),
            decision: value.decision as u32,
            decision_basis: value.decision_basis.map(object_ref_from_proto),
            reason_code: value.reason_code,
            effect_summary_object: value.effect_summary_object.map(object_ref_from_proto),
            result_object: value.result_object.map(object_ref_from_proto),
        }
    }

    pub fn command_envelope_from_proto(value: stream::CommandEnvelope) -> CommandEnvelopeWire {
        let payload = value.payload.map(|payload| match payload {
            stream::command_envelope::Payload::PayloadObject(value) => CommandPayloadWire::Object(object_ref_from_proto(value)),
            stream::command_envelope::Payload::InlinePayload(value) => CommandPayloadWire::Inline(value),
        });
        CommandEnvelopeWire {
            envelope_version: value.envelope_version,
            command_id: value.command_id,
            target_node: value.target_node.map(node_ref_from_proto),
            issuer: value.issuer.map(identity_ref_from_proto),
            command_type: value.command_type as u32,
            command_version: value.command_version,
            idempotency_key: value.idempotency_key,
            payload,
            command_metadata: value.command_metadata.map(object_ref_from_proto),
            signature: value.signature.map(signature_from_proto),
            app_intent: value.app_intent,
        }
    }

    pub fn event_envelope_from_proto(value: stream::EventEnvelope) -> EventEnvelopeWire {
        EventEnvelopeWire {
            envelope_version: value.envelope_version,
            stream_id: value.stream_id,
            seq: value.seq,
            prev_event_hash: value.prev_event_hash.map(digest_from_proto),
            event_type: value.event_type as u32,
            event_version: value.event_version,
            payload_object: value.payload_object.map(object_ref_from_proto),
            related_events: value.related_events.into_iter().map(event_ref_from_proto).collect(),
            related_commands: value.related_commands.into_iter().map(command_ref_from_proto).collect(),
            related_objects: value.related_objects.into_iter().map(object_ref_from_proto).collect(),
            event_metadata: value.event_metadata.map(object_ref_from_proto),
            signature: value.signature.map(signature_from_proto),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_wire::canonical_bytes;

    #[test]
    fn event_signable_bytes_drop_signature() {
        let event = EventEnvelopeWire {
            envelope_version: 1,
            stream_id: b"s".to_vec(),
            seq: 1,
            prev_event_hash: None,
            event_type: 1,
            event_version: 1,
            payload_object: None,
            related_events: Vec::new(),
            related_commands: Vec::new(),
            related_objects: Vec::new(),
            event_metadata: None,
            signature: Some(SignatureWire { algorithm: 1, value: vec![0xab; 64] }),
        };
        let full = canonical_bytes(&event);
        let signable = event.signable_bytes();
        assert!(full.len() > signable.len());
        assert!(!signable.contains(&0xab));
    }

    #[test]
    fn command_result_wire_is_deterministic() {
        let value = CommandResultPayloadWire {
            payload_version: 1,
            command: Some(CommandRefWire { command_id: b"cmd".to_vec(), command_hash: None }),
            issuer: None,
            decision: 1,
            decision_basis: None,
            reason_code: "ok".into(),
            effect_summary_object: None,
            result_object: None,
        };
        assert_eq!(canonical_bytes(&value), canonical_bytes(&value));
    }
}
