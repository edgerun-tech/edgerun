//! Native protocol-to-wire bridge. No edgerun-proto dependency.

use crate::protocol::*;
use crate::wire_boundary::{
    CommandEnvelopeWire, CommandPayloadWire, CommandRefWire, CommandResultPayloadWire, DigestWire,
    EventEnvelopeWire, EventRefWire, IdentityRefWire, NodeRefWire, ObjectRefWire, SignatureWire,
};

pub fn digest_wire(value: &Digest) -> DigestWire {
    DigestWire {
        algorithm: value.algorithm as u32,
        value: value.value.clone(),
    }
}

pub fn signature_wire(value: &Signature) -> SignatureWire {
    SignatureWire {
        algorithm: value.algorithm as u32,
        value: value.value.clone(),
    }
}

pub fn identity_ref_wire(value: &IdentityRef) -> IdentityRefWire {
    IdentityRefWire {
        identity_id: value.identity_id.clone(),
        identity_kind: value.identity_kind.map(|v| v as u32),
        key_hint: value.key_hint.clone(),
    }
}

pub fn node_ref_wire(value: &NodeRef) -> NodeRefWire {
    NodeRefWire {
        node_id: value.node_id.clone(),
    }
}

pub fn object_ref_wire(value: &ObjectRef) -> ObjectRefWire {
    ObjectRefWire {
        object_id: value.object_id.clone(),
        object_kind: value.object_kind.map(|v| v as u32),
    }
}

pub fn command_ref_wire(value: &CommandRef) -> CommandRefWire {
    CommandRefWire {
        command_id: value.command_id.clone(),
        command_hash: value.command_hash.as_ref().map(digest_wire),
    }
}

pub fn event_ref_wire(value: &EventRef) -> EventRefWire {
    EventRefWire {
        stream_id: value.stream_id.clone(),
        seq: value.seq,
        event_hash: value.event_hash.as_ref().map(digest_wire),
    }
}

pub fn command_envelope_wire_from_command(value: &CommandEnvelope) -> CommandEnvelopeWire {
    let payload = value.payload.as_ref().map(|payload| match payload {
        command_envelope::Payload::PayloadObject(value) => {
            CommandPayloadWire::Object(object_ref_wire(value))
        }
        command_envelope::Payload::InlinePayload(value) => {
            CommandPayloadWire::Inline(value.clone())
        }
    });
    CommandEnvelopeWire {
        envelope_version: value.envelope_version,
        command_id: value.command_id.clone(),
        target_node: value.target_node.as_ref().map(node_ref_wire),
        issuer: value.issuer.as_ref().map(identity_ref_wire),
        command_type: value.command_type as u32,
        command_version: value.command_version,
        idempotency_key: value.idempotency_key.clone(),
        payload,
        command_metadata: value.command_metadata.as_ref().map(object_ref_wire),
        signature: value.signature.as_ref().map(signature_wire),
        app_intent: value.app_intent.clone(),
    }
}

pub fn command_result_wire_from_result(value: &CommandResultPayload) -> CommandResultPayloadWire {
    CommandResultPayloadWire {
        payload_version: value.payload_version,
        command: value.command.as_ref().map(command_ref_wire),
        issuer: value.issuer.as_ref().map(identity_ref_wire),
        decision: value.decision as u32,
        decision_basis: value.decision_basis.as_ref().map(object_ref_wire),
        reason_code: value.reason_code.clone(),
        effect_summary_object: value.effect_summary_object.as_ref().map(object_ref_wire),
        result_object: value.result_object.as_ref().map(object_ref_wire),
    }
}

pub fn event_envelope_wire_from_event(value: &EventEnvelope) -> EventEnvelopeWire {
    EventEnvelopeWire {
        envelope_version: value.envelope_version,
        stream_id: value.stream_id.clone(),
        seq: value.seq,
        prev_event_hash: value.prev_event_hash.as_ref().map(digest_wire),
        event_type: value.event_type as u32,
        event_version: value.event_version,
        payload_object: value.payload_object.as_ref().map(object_ref_wire),
        related_events: value.related_events.iter().map(event_ref_wire).collect(),
        related_commands: value
            .related_commands
            .iter()
            .map(command_ref_wire)
            .collect(),
        related_objects: value.related_objects.iter().map(object_ref_wire).collect(),
        event_metadata: value.event_metadata.as_ref().map(object_ref_wire),
        signature: value.signature.as_ref().map(signature_wire),
    }
}
