use alloc::vec::Vec;

use crate::channel::{ArchivedChannelEnvelope, ChannelEnvelope};
use crate::codec::{wire_bytes, wire_from_bytes};
use crate::protocol::WorkProtocolError;

pub const MAX_CHANNEL_FRAME_LEN: usize = 1024 * 1024;

pub fn channel_envelope_bytes(envelope: &ChannelEnvelope) -> Result<Vec<u8>, WorkProtocolError> {
    let bytes = wire_bytes(envelope)?;
    if bytes.len() > MAX_CHANNEL_FRAME_LEN {
        return Err(WorkProtocolError::PacketTooLarge);
    }
    Ok(bytes)
}

pub fn channel_envelope_from_bytes(bytes: &[u8]) -> Result<ChannelEnvelope, WorkProtocolError> {
    if bytes.is_empty() {
        return Err(WorkProtocolError::EmptyPacket);
    }
    if bytes.len() > MAX_CHANNEL_FRAME_LEN {
        return Err(WorkProtocolError::PacketTooLarge);
    }
    wire_from_bytes::<ChannelEnvelope, ArchivedChannelEnvelope>(bytes)
}
