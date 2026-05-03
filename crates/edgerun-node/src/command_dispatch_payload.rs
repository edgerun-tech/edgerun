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
    let bytes = inline_payload_bytes(command).ok_or_else(|| "missing_inline_payload".to_string())?;
    M::decode(bytes).map_err(|e| format!("invalid_inline_payload: {e}"))
}

pub fn decode_named_inline_payload<M>(
    command: &CommandEnvelope,
    payload_name: &str,
) -> Result<M, String>
where
    M: Message + Default,
{
    let bytes = inline_payload_bytes(command)
        .ok_or_else(|| format!("missing_{payload_name}"))?;
    M::decode(bytes).map_err(|e| format!("invalid_{payload_name}: {e}"))
}
