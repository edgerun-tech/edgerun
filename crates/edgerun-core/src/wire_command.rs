//! Rkyv-only command wire boundary.
//!
//! Command byte materialization now has a single valid direction: archive the
//! concrete protocol object at the rkyv boundary. Transitional byte helpers are
//! explicit breakpoints until their callers are moved.

use alloc::vec::Vec;

use crate::protocol::{CommandEnvelope, CommandResultPayload, Digest};

#[cold]
fn removed_command_wire_path() -> ! {
    panic!("command wire helpers were removed; use the rkyv wire boundary")
}

#[must_use]
pub fn command_signable_bytes(_command: &CommandEnvelope) -> Vec<u8> {
    removed_command_wire_path()
}

#[must_use]
pub fn command_full_bytes(_command: &CommandEnvelope) -> Vec<u8> {
    removed_command_wire_path()
}

#[must_use]
pub fn command_result_bytes(_result: &CommandResultPayload) -> Vec<u8> {
    removed_command_wire_path()
}

#[must_use]
pub fn command_hash(_command: &CommandEnvelope) -> Digest {
    removed_command_wire_path()
}
