use alloc::vec::Vec;
use core::fmt;

use edgerun_crypto::sha256;
use edgerun_wire::api::high::{HighDeserializer, HighSerializer, HighValidator};
use edgerun_wire::bytecheck::CheckBytes;
use edgerun_wire::ser::allocator::ArenaHandle;
use edgerun_wire::{Portable, Serialize, WireError, access, deserialize, to_bytes, util};

use crate::channel::{ChannelEnvelope, ChannelId, RouteBinding};
use crate::protocol::*;
use crate::route_binding::route_hash;

pub type AlignedWorkPacketBytes = util::AlignedVec<16>;

pub struct EncodedWorkPacket {
    pub bytes: AlignedWorkPacketBytes,
    pub hash: Hash,
}

pub struct EncodedChannelEnvelope {
    pub envelope: ChannelEnvelope,
    pub packet: EncodedWorkPacket,
}

impl fmt::Debug for EncodedWorkPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EncodedWorkPacket")
            .field("len", &self.bytes.as_slice().len())
            .field("hash", &self.hash)
            .finish()
    }
}

impl EncodedWorkPacket {
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }
}

pub struct ArchivedWorkPacketFrame {
    pub bytes: AlignedWorkPacketBytes,
    pub hash: Hash,
}

impl fmt::Debug for ArchivedWorkPacketFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ArchivedWorkPacketFrame")
            .field("len", &self.bytes.as_slice().len())
            .field("hash", &self.hash)
            .finish()
    }
}

impl ArchivedWorkPacketFrame {
    pub fn as_bytes(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    pub fn archived(&self) -> Result<&ArchivedWorkPacket, WorkProtocolError> {
        archived_work_packet_from_aligned_bytes(self.bytes.as_slice())
    }

    pub fn into_packet(self) -> Result<WorkPacket, WorkProtocolError> {
        packet_from_aligned_bytes(self.bytes.as_slice())
    }
}

pub fn blake3_hash(bytes: &[u8]) -> Hash {
    *crate::blake3::hash(bytes).as_bytes()
}

pub fn sha256_hash(bytes: &[u8]) -> Hash {
    sha256(bytes)
}

pub fn packet_hash(packet: &WorkPacket) -> Result<Hash, WorkProtocolError> {
    Ok(encode_work_packet_once(packet)?.hash)
}

pub fn encode_work_packet_once(
    packet: &WorkPacket,
) -> Result<EncodedWorkPacket, WorkProtocolError> {
    let bytes = packet_aligned_bytes(packet)?;
    let hash = blake3_hash(bytes.as_slice());
    Ok(EncodedWorkPacket { bytes, hash })
}

pub fn encode_channel_envelope_for_route(
    route: &RouteBinding,
    from: NodeId,
    to: NodeId,
    packet: WorkPacket,
) -> Result<EncodedChannelEnvelope, WorkProtocolError> {
    encode_channel_envelope(
        route.endpoint.channel_id,
        route_hash(route),
        from,
        to,
        packet,
    )
}

pub fn encode_channel_envelope(
    channel_id: ChannelId,
    route_hash: Hash,
    from: NodeId,
    to: NodeId,
    packet: WorkPacket,
) -> Result<EncodedChannelEnvelope, WorkProtocolError> {
    let encoded = encode_work_packet_once(&packet)?;
    let envelope = ChannelEnvelope::new(channel_id, from, to, route_hash, encoded.hash, packet);
    Ok(EncodedChannelEnvelope {
        envelope,
        packet: encoded,
    })
}

pub fn packet_aligned_bytes(
    packet: &WorkPacket,
) -> Result<AlignedWorkPacketBytes, WorkProtocolError> {
    let bytes: AlignedWorkPacketBytes =
        to_bytes::<WireError>(packet).map_err(|_| WorkProtocolError::InvalidPacket)?;
    if bytes.len() > MAX_WORK_FRAME_LEN {
        return Err(WorkProtocolError::PacketTooLarge);
    }
    Ok(bytes)
}

pub fn packet_bytes(packet: &WorkPacket) -> Result<Vec<u8>, WorkProtocolError> {
    Ok(packet_aligned_bytes(packet)?.to_vec())
}

pub fn wire_bytes<T>(value: &T) -> Result<Vec<u8>, WorkProtocolError>
where
    T: for<'a> Serialize<HighSerializer<AlignedWorkPacketBytes, ArenaHandle<'a>, WireError>>,
{
    to_bytes::<WireError>(value)
        .map(|bytes| bytes.to_vec())
        .map_err(|_| WorkProtocolError::InvalidPacket)
}

pub fn aligned_copy(bytes: &[u8]) -> AlignedWorkPacketBytes {
    let mut aligned = util::AlignedVec::<16>::with_capacity(bytes.len());
    aligned.extend_from_slice(bytes);
    aligned
}

pub fn aligned_copy_if_needed_for<T>(bytes: &[u8]) -> Option<AlignedWorkPacketBytes> {
    if bytes.as_ptr().align_offset(core::mem::align_of::<T>()) != 0 {
        Some(aligned_copy(bytes))
    } else {
        None
    }
}

pub fn wire_from_bytes<T, A>(bytes: &[u8]) -> Result<T, WorkProtocolError>
where
    A: Portable
        + for<'a> CheckBytes<HighValidator<'a, WireError>>
        + edgerun_wire::Deserialize<T, HighDeserializer<WireError>>,
{
    if let Some(aligned) = aligned_copy_if_needed_for::<A>(bytes) {
        return wire_from_aligned_bytes::<T, A>(aligned.as_slice());
    }
    wire_from_aligned_bytes::<T, A>(bytes)
}

pub fn wire_from_aligned_bytes<T, A>(bytes: &[u8]) -> Result<T, WorkProtocolError>
where
    A: Portable
        + for<'a> CheckBytes<HighValidator<'a, WireError>>
        + edgerun_wire::Deserialize<T, HighDeserializer<WireError>>,
{
    let archived = access::<A, WireError>(bytes).map_err(|_| WorkProtocolError::InvalidPacket)?;
    deserialize::<T, WireError>(archived).map_err(|_| WorkProtocolError::InvalidPacket)
}

pub fn archived_packet_frame_from_bytes(
    bytes: &[u8],
) -> Result<ArchivedWorkPacketFrame, WorkProtocolError> {
    if bytes.is_empty() {
        return Err(WorkProtocolError::EmptyPacket);
    }
    if bytes.len() > MAX_WORK_FRAME_LEN {
        return Err(WorkProtocolError::PacketTooLarge);
    }
    let aligned = aligned_copy(bytes);
    archived_work_packet_from_aligned_bytes(aligned.as_slice())?;
    let hash = blake3_hash(aligned.as_slice());
    Ok(ArchivedWorkPacketFrame {
        bytes: aligned,
        hash,
    })
}

pub fn archived_work_packet_from_aligned_bytes(
    bytes: &[u8],
) -> Result<&ArchivedWorkPacket, WorkProtocolError> {
    if bytes.is_empty() {
        return Err(WorkProtocolError::EmptyPacket);
    }
    if bytes.len() > MAX_WORK_FRAME_LEN {
        return Err(WorkProtocolError::PacketTooLarge);
    }
    access::<ArchivedWorkPacket, WireError>(bytes).map_err(|_| WorkProtocolError::InvalidPacket)
}

pub fn packet_from_bytes(bytes: &[u8]) -> Result<WorkPacket, WorkProtocolError> {
    if bytes.is_empty() {
        return Err(WorkProtocolError::EmptyPacket);
    }
    if bytes.len() > MAX_WORK_FRAME_LEN {
        return Err(WorkProtocolError::PacketTooLarge);
    }
    if let Some(aligned) = aligned_copy_if_needed_for::<ArchivedWorkPacket>(bytes) {
        return packet_from_aligned_bytes(aligned.as_slice());
    }
    packet_from_aligned_bytes(bytes)
}

pub fn packet_from_aligned_bytes(bytes: &[u8]) -> Result<WorkPacket, WorkProtocolError> {
    let archived = archived_work_packet_from_aligned_bytes(bytes)?;
    deserialize::<WorkPacket, WireError>(archived).map_err(|_| WorkProtocolError::InvalidPacket)
}
