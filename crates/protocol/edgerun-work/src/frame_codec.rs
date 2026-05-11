use alloc::vec::Vec;

use edgerun_wire::{access, deserialize, to_bytes, util, WireError};

use crate::channel::{ChannelEnvelope, ArchivedChannelEnvelope};
use crate::protocol::WorkProtocolError;

pub const MAX_CHANNEL_FRAME_LEN: usize = 1024 * 1024;

pub fn channel_envelope_bytes(envelope: &ChannelEnvelope) -> Result<Vec<u8>, WorkProtocolError> {
    let bytes = to_bytes::<WireError>(envelope).map_err(|_| WorkProtocolError::InvalidPacket)?;
    if bytes.len() > MAX_CHANNEL_FRAME_LEN {
        return Err(WorkProtocolError::PacketTooLarge);
    }
    Ok(bytes.to_vec())
}

pub fn channel_envelope_from_bytes(bytes: &[u8]) -> Result<ChannelEnvelope, WorkProtocolError> {
    if bytes.is_empty() {
        return Err(WorkProtocolError::EmptyPacket);
    }
    if bytes.len() > MAX_CHANNEL_FRAME_LEN {
        return Err(WorkProtocolError::PacketTooLarge);
    }
    if bytes
        .as_ptr()
        .align_offset(core::mem::align_of::<ArchivedChannelEnvelope>())
        != 0
    {
        let mut aligned = util::AlignedVec::<16>::with_capacity(bytes.len());
        aligned.extend_from_slice(bytes);
        return channel_envelope_from_aligned_bytes(aligned.as_slice());
    }
    channel_envelope_from_aligned_bytes(bytes)
}

fn channel_envelope_from_aligned_bytes(bytes: &[u8]) -> Result<ChannelEnvelope, WorkProtocolError> {
    let archived = access::<ArchivedChannelEnvelope, WireError>(bytes)
        .map_err(|_| WorkProtocolError::InvalidPacket)?;
    deserialize::<ChannelEnvelope, WireError>(archived).map_err(|_| WorkProtocolError::InvalidPacket)
}
