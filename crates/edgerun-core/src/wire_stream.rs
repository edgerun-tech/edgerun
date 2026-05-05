//! Rkyv-only stream wire boundary.
//!
//! The previous transitional stream encoder has been removed. Call sites must
//! archive the concrete protocol type at the rkyv boundary instead of routing
//! through a second canonical format.

use alloc::vec::Vec;

use crate::protocol::{CommandEnvelope, CommandResultPayload, EventEnvelope};

#[cold]
fn removed_stream_wire_path() -> ! {
    panic!("stream wire helpers were removed; use the rkyv wire boundary")
}

pub fn event_signable_wire_bytes(_event: &EventEnvelope) -> Vec<u8> {
    removed_stream_wire_path()
}

pub fn event_full_wire_bytes(_event: &EventEnvelope) -> Vec<u8> {
    removed_stream_wire_path()
}

pub fn command_signable_wire_bytes(_command: &CommandEnvelope) -> Vec<u8> {
    removed_stream_wire_path()
}

pub fn command_full_wire_bytes(_command: &CommandEnvelope) -> Vec<u8> {
    removed_stream_wire_path()
}

pub fn command_result_wire_bytes(_result: &CommandResultPayload) -> Vec<u8> {
    removed_stream_wire_path()
}
