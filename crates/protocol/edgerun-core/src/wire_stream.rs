//! Rkyv-only stream wire boundary.

use alloc::vec::Vec;

use crate::protocol::{
    protocol_wire_bytes, CommandEnvelope, CommandResultPayload, EventEnvelope, ProtocolRecord,
};

pub fn event_signable_wire_bytes(event: &EventEnvelope) -> Vec<u8> {
    protocol_wire_bytes(&ProtocolRecord::EventEnvelope(event.clone()), true)
}

pub fn event_full_wire_bytes(event: &EventEnvelope) -> Vec<u8> {
    edgerun_wire::to_bytes::<edgerun_wire::WireError>(event)
        .expect("event envelope must serialize through the rkyv wire boundary")
        .into_vec()
}

pub fn decode_event_full_wire_bytes(bytes: &[u8]) -> Result<EventEnvelope, &'static str> {
    edgerun_wire::from_bytes::<EventEnvelope, edgerun_wire::WireError>(bytes)
        .map_err(|_| "invalid rkyv event envelope")
}

pub fn command_signable_wire_bytes(command: &CommandEnvelope) -> Vec<u8> {
    protocol_wire_bytes(&ProtocolRecord::CommandEnvelope(command.clone()), true)
}

pub fn command_full_wire_bytes(command: &CommandEnvelope) -> Vec<u8> {
    protocol_wire_bytes(&ProtocolRecord::CommandEnvelope(command.clone()), false)
}

pub fn command_result_wire_bytes(result: &CommandResultPayload) -> Vec<u8> {
    protocol_wire_bytes(&ProtocolRecord::CommandResultPayload(result.clone()), false)
}
