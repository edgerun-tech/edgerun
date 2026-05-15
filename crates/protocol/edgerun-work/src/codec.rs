use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

use edgerun_crypto::sha256;

use crate::channel::{ChannelEnvelope, ChannelId, RouteBinding};
use crate::generated_wire::ArchivedWorkPacket;
use crate::protocol::*;
use crate::route_binding::route_hash;

pub type AlignedWorkPacketBytes = Vec<u8>;

pub struct WireWriter {
    bytes: Vec<u8>,
}

impl WireWriter {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn push(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl Default for WireWriter {
    fn default() -> Self {
        Self::new()
    }
}

pub struct WireCursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> WireCursor<'a> {
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.pos)
    }

    pub fn take(&mut self, len: usize) -> Result<&'a [u8], WorkProtocolError> {
        if self.remaining() < len {
            return Err(WorkProtocolError::InvalidPacket);
        }
        let start = self.pos;
        self.pos += len;
        Ok(&self.bytes[start..self.pos])
    }

    pub fn finish(self) -> Result<(), WorkProtocolError> {
        if self.pos == self.bytes.len() {
            Ok(())
        } else {
            Err(WorkProtocolError::InvalidShape)
        }
    }
}

pub trait EdgeWire: Sized {
    fn encode_wire(&self, out: &mut WireWriter);
    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError>;
}

macro_rules! impl_wire_int {
    ($ty:ty) => {
        impl EdgeWire for $ty {
            fn encode_wire(&self, out: &mut WireWriter) {
                out.push(&self.to_le_bytes());
            }

            fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
                let bytes = input.take(core::mem::size_of::<$ty>())?;
                let mut array = [0u8; core::mem::size_of::<$ty>()];
                array.copy_from_slice(bytes);
                Ok(<$ty>::from_le_bytes(array))
            }
        }
    };
}

impl_wire_int!(u16);
impl_wire_int!(u32);
impl_wire_int!(u64);
impl_wire_int!(i32);

impl EdgeWire for bool {
    fn encode_wire(&self, out: &mut WireWriter) {
        out.push(&[*self as u8]);
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        match input.take(1)?[0] {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(WorkProtocolError::InvalidShape),
        }
    }
}

impl<const N: usize> EdgeWire for [u8; N] {
    fn encode_wire(&self, out: &mut WireWriter) {
        out.push(self);
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        let mut out = [0u8; N];
        out.copy_from_slice(input.take(N)?);
        Ok(out)
    }
}

impl<T: EdgeWire> EdgeWire for Option<T> {
    fn encode_wire(&self, out: &mut WireWriter) {
        match self {
            Some(value) => {
                true.encode_wire(out);
                value.encode_wire(out);
            }
            None => false.encode_wire(out),
        }
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        if bool::decode_wire(input)? {
            Ok(Some(T::decode_wire(input)?))
        } else {
            Ok(None)
        }
    }
}

impl<T: EdgeWire> EdgeWire for Vec<T> {
    fn encode_wire(&self, out: &mut WireWriter) {
        (self.len() as u64).encode_wire(out);
        for item in self {
            item.encode_wire(out);
        }
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        let len = u64::decode_wire(input)?;
        if len as usize > input.remaining() {
            return Err(WorkProtocolError::InvalidPacket);
        }
        let mut out = Vec::with_capacity(len as usize);
        for _ in 0..len {
            out.push(T::decode_wire(input)?);
        }
        Ok(out)
    }
}

impl EdgeWire for u8 {
    fn encode_wire(&self, out: &mut WireWriter) {
        out.push(&[*self]);
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        Ok(input.take(1)?[0])
    }
}

impl EdgeWire for String {
    fn encode_wire(&self, out: &mut WireWriter) {
        self.as_bytes().to_vec().encode_wire(out);
    }

    fn decode_wire(input: &mut WireCursor<'_>) -> Result<Self, WorkProtocolError> {
        String::from_utf8(Vec::<u8>::decode_wire(input)?)
            .map_err(|_| WorkProtocolError::InvalidShape)
    }
}

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

    pub fn archived(&self) -> Result<ArchivedWorkPacket, WorkProtocolError> {
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
    let bytes = wire_bytes(packet)?;
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
    T: EdgeWire,
{
    let mut out = WireWriter::new();
    value.encode_wire(&mut out);
    Ok(out.into_bytes())
}

pub fn aligned_copy(bytes: &[u8]) -> AlignedWorkPacketBytes {
    bytes.to_vec()
}

pub fn wire_from_bytes<T, A>(bytes: &[u8]) -> Result<T, WorkProtocolError>
where
    T: EdgeWire,
{
    let _ = core::marker::PhantomData::<A>;
    wire_from_aligned_bytes::<T, A>(bytes)
}

pub fn wire_from_aligned_bytes<T, A>(bytes: &[u8]) -> Result<T, WorkProtocolError>
where
    T: EdgeWire,
{
    let _ = core::marker::PhantomData::<A>;
    let mut input = WireCursor::new(bytes);
    let value = T::decode_wire(&mut input)?;
    input.finish()?;
    Ok(value)
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
) -> Result<ArchivedWorkPacket, WorkProtocolError> {
    if bytes.is_empty() {
        return Err(WorkProtocolError::EmptyPacket);
    }
    if bytes.len() > MAX_WORK_FRAME_LEN {
        return Err(WorkProtocolError::PacketTooLarge);
    }
    packet_from_aligned_bytes(bytes)
}

pub fn packet_from_bytes(bytes: &[u8]) -> Result<WorkPacket, WorkProtocolError> {
    if bytes.is_empty() {
        return Err(WorkProtocolError::EmptyPacket);
    }
    if bytes.len() > MAX_WORK_FRAME_LEN {
        return Err(WorkProtocolError::PacketTooLarge);
    }
    packet_from_aligned_bytes(bytes)
}

pub fn packet_from_aligned_bytes(bytes: &[u8]) -> Result<WorkPacket, WorkProtocolError> {
    wire_from_aligned_bytes::<WorkPacket, ArchivedWorkPacket>(bytes)
}
