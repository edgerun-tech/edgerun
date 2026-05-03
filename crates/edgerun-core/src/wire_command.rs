//! Command-specific canonical wire helpers.
//!
//! This module is the migration seam away from protobuf canonicalization for
//! command identity, replay keys, command signatures, and command-result bytes.

use alloc::vec::Vec;

use crate::protocol::{CommandEnvelope, CommandResultPayload, Digest};

#[must_use]
pub fn command_signable_bytes(command: &CommandEnvelope) -> Vec<u8> {
    crate::wire_stream::command_signable_wire_bytes(command)
}

#[must_use]
pub fn command_full_bytes(command: &CommandEnvelope) -> Vec<u8> {
    crate::wire_stream::command_full_wire_bytes(command)
}

#[must_use]
pub fn command_result_bytes(result: &CommandResultPayload) -> Vec<u8> {
    crate::wire_stream::command_result_wire_bytes(result)
}

#[must_use]
pub fn command_hash(command: &CommandEnvelope) -> Digest {
    let canonical = command_signable_bytes(command);
    let hash = crate::crypto::record_hash(crate::crypto::HASH_DOMAIN_COMMAND_ENVELOPE, &canonical);
    Digest { algorithm: 1, value: hash.to_vec() }
}
