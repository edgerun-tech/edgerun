//! Shared command payload helpers.

use edgerun_proto::edgerun::v0::stream::{command_envelope, CommandEnvelope};
use prost::Message;

pub fn inline_payload_bytes(command: &CommandEnvelope) -> Option<&[u8]> {
    match command.payload.as_ref()? {
        command_envelope::Payload::InlinePayload(bytes) => Some(bytes.as_slice()),
        command_envelope::Payload::PayloadObject(_) => None,
    }
}

pub fn decode_inline_payload<M>(command: &CommandEnvelope) -> Result<M, String>
where
    M: Message + Default,
{
    let bytes =
        inline_payload_bytes(command).ok_or_else(|| "missing_inline_payload".to_string())?;
    M::decode(bytes).map_err(|e| format!("invalid_inline_payload: {e}"))
}

pub fn decode_named_inline_payload<M>(
    command: &CommandEnvelope,
    payload_name: &str,
) -> Result<M, String>
where
    M: Message + Default,
{
    let bytes = inline_payload_bytes(command).ok_or_else(|| format!("missing_{payload_name}"))?;
    M::decode(bytes).map_err(|e| format!("invalid_{payload_name}: {e}"))
}

/// Extract and decode an inline payload, or return a CommandDispatchResult with rejection.
///
/// This consolidates the repetitive pattern of "extract payload, decode, or reject command"
/// used across many dispatch handlers.
///
/// Returns the decoded payload on success, or returns early from the calling function
/// with a rejected command result on failure.
#[inline]
pub fn extract_payload_or_reject<M>(
    command: &CommandEnvelope,
    payload_name: &str,
    reject: &dyn Fn(String) -> CommandDispatchResult,
) -> Result<M, CommandDispatchResult>
where
    M: Message + Default,
{
    match decode_named_inline_payload::<M>(command, payload_name) {
        Ok(p) => Ok(p),
        Err(e) => Err(reject(e)),
    }
}
