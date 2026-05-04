//! Stream-specific canonical wire helpers.
//!
//! This module is the migration seam for code that still receives generated
//! protobuf structs at public boundaries. New storage and hashing code should
//! call these helpers instead of treating prost bytes as canonical.

use alloc::vec::Vec;

use crate::protocol::{CommandEnvelope, CommandResultPayload, EventEnvelope};

pub fn event_signable_wire_bytes(event: &EventEnvelope) -> Vec<u8> {
    crate::wire_boundary_free::event_envelope_wire_from_event(event).signable_bytes()
}

pub fn event_full_wire_bytes(event: &EventEnvelope) -> Vec<u8> {
    edgerun_wire::canonical_bytes(&crate::wire_boundary_free::event_envelope_wire_from_event(
        event,
    ))
}

pub fn command_signable_wire_bytes(command: &CommandEnvelope) -> Vec<u8> {
    crate::wire_boundary_free::command_envelope_wire_from_command(command).signable_bytes()
}

pub fn command_full_wire_bytes(command: &CommandEnvelope) -> Vec<u8> {
    edgerun_wire::canonical_bytes(
        &crate::wire_boundary_free::command_envelope_wire_from_command(command),
    )
}

pub fn command_result_wire_bytes(result: &CommandResultPayload) -> Vec<u8> {
    edgerun_wire::canonical_bytes(&crate::wire_boundary_free::command_result_wire_from_result(
        result,
    ))
}
