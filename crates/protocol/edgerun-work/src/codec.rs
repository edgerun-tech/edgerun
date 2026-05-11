use alloc::vec::Vec;
use core::fmt;

use edgerun_crypto::sha256;
use edgerun_wire::{access, deserialize, to_bytes, util, WireError};

use crate::protocol::*;

pub use crate::identity::{derive_node_id, node_identity_from_key, verify_node_identity};
pub use crate::signing::{
    empty_signature, network_message_preimage, node_available_preimage,
    node_heartbeat_preimage, relay_assignment_preimage, sign_ed25519,
    sign_network_message, sign_node_available, sign_node_heartbeat,
    sign_relay_assignment, sign_work_admission, sign_work_receipt,
    verify_network_message, verify_node_available, verify_node_heartbeat,
    verify_relay_assignment, verify_signature, verify_solana_ed25519,
    verify_work_admission, verify_work_receipt, work_admission_preimage,
    work_receipt_preimage,
};

pub type AlignedWorkPacketBytes = util::AlignedVec<16>;

pub struct EncodedWorkPacket {
    pub bytes: AlignedWorkPacketBytes,
    pub hash: Hash,
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
    *blake3::hash(bytes).as_bytes()
}

pub fn sha256_hash(bytes: &[u8]) -> Hash {
    sha256(bytes)
}

pub fn packet_hash(packet: &WorkPacket) -> Result<Hash, WorkProtocolError> {
    Ok(encode_work_packet_once(packet)?.hash)
}

pub fn encode_work_packet_once(packet: &WorkPacket) -> Result<EncodedWorkPacket, WorkProtocolError> {
    let bytes = packet_aligned_bytes(packet)?;
    let hash = blake3_hash(bytes.as_slice());
    Ok(EncodedWorkPacket { bytes, hash })
}

pub fn packet_aligned_bytes(packet: &WorkPacket) -> Result<AlignedWorkPacketBytes, WorkProtocolError> {
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

pub fn archived_packet_frame_from_bytes(bytes: &[u8]) -> Result<ArchivedWorkPacketFrame, WorkProtocolError> {
    if bytes.is_empty() {
        return Err(WorkProtocolError::EmptyPacket);
    }
    if bytes.len() > MAX_WORK_FRAME_LEN {
        return Err(WorkProtocolError::PacketTooLarge);
    }
    let mut aligned = util::AlignedVec::<16>::with_capacity(bytes.len());
    aligned.extend_from_slice(bytes);
    archived_work_packet_from_aligned_bytes(aligned.as_slice())?;
    let hash = blake3_hash(aligned.as_slice());
    Ok(ArchivedWorkPacketFrame { bytes: aligned, hash })
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
    if bytes
        .as_ptr()
        .align_offset(core::mem::align_of::<ArchivedWorkPacket>())
        != 0
    {
        let mut aligned = util::AlignedVec::<16>::with_capacity(bytes.len());
        aligned.extend_from_slice(bytes);
        return packet_from_aligned_bytes(aligned.as_slice());
    }
    packet_from_aligned_bytes(bytes)
}

pub fn packet_from_aligned_bytes(bytes: &[u8]) -> Result<WorkPacket, WorkProtocolError> {
    let archived = archived_work_packet_from_aligned_bytes(bytes)?;
    deserialize::<WorkPacket, WireError>(archived).map_err(|_| WorkProtocolError::InvalidPacket)
}
