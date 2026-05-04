//! Shared command payload helpers.
//!
//! Command payload decoding is intentionally not generic anymore. Each command
//! family owns one explicit edgerun-wire codec instead of using prost::Message.

use edgerun_core::protocol::{command_envelope, CommandEnvelope};

pub fn inline_payload_bytes(command: &CommandEnvelope) -> Option<&[u8]> {
    match command.payload.as_ref()? {
        command_envelope::Payload::InlinePayload(bytes) => Some(bytes.as_slice()),
        command_envelope::Payload::PayloadObject(_) => None,
    }
}

pub fn required_inline_payload_bytes<'a>(command: &'a CommandEnvelope, payload_name: &str) -> Result<&'a [u8], String> {
    inline_payload_bytes(command).ok_or_else(|| format!("missing_{payload_name}"))
}
