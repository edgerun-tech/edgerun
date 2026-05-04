//! Stream-specific canonical wire helpers.
//!
//! This module is the migration seam for code that still receives generated
//! protobuf structs at public boundaries. New storage and hashing code should
//! call these helpers instead of treating prost bytes as canonical.

use alloc::vec::Vec;

use crate::wire_boundary::proto_boundary;
use edgerun_core::protocol::{CommandEnvelope, CommandResultPayload, EventEnvelope};

pub fn event_signable_wire_bytes(event: &EventEnvelope) -> Vec<u8> {
    proto_boundary::event_envelope_from_proto(event.clone()).signable_bytes()
}

pub fn event_full_wire_bytes(event: &EventEnvelope) -> Vec<u8> {
    edgerun_wire::canonical_bytes(&proto_boundary::event_envelope_from_proto(event.clone()))
}

pub fn command_signable_wire_bytes(command: &CommandEnvelope) -> Vec<u8> {
    proto_boundary::command_envelope_from_proto(command.clone()).signable_bytes()
}

pub fn command_full_wire_bytes(command: &CommandEnvelope) -> Vec<u8> {
    edgerun_wire::canonical_bytes(&proto_boundary::command_envelope_from_proto(
        command.clone(),
    ))
}

pub fn command_result_wire_bytes(result: &CommandResultPayload) -> Vec<u8> {
    edgerun_wire::canonical_bytes(&proto_boundary::command_result_from_proto(result.clone()))
}
