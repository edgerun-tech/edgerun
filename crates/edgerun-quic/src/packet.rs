//! QUIC packet format (RFC 9000 Section 17)

use crate::std;
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use edgerun_encoding::byteorder::{read_u32_be, read_u64_be};
use edgerun_encoding::quic_varint::{
    decode_varint as quic_decode_varint, encode_varint as quic_encode_varint,
};

/// QUIC packet types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    /// Initial packet
    Initial,
    /// 0-RTT protected packet
    ZeroRtt,
    /// Handshake packet
    Handshake,
    /// Retry packet
    Retry,
    /// 1-RTT protected packet (short header)
    OneRtt,
}

impl PacketType {
    /// Encode to byte value (RFC 9000 §17.2 long header format)
    pub fn to_byte(self) -> u8 {
        match self {
            PacketType::Initial => 0xC0,
            PacketType::ZeroRtt => 0xD0,
            PacketType::Handshake => 0xE0,
            PacketType::Retry => 0xF0,
            PacketType::OneRtt => 0x40,
        }
    }

    /// Decode from byte value
    pub fn from_byte(byte: u8) -> Option<Self> {
        if byte & 0x80 != 0 {
            match (byte >> 4) & 0x03 {
                0x0 => Some(PacketType::Initial),
                0x1 => Some(PacketType::ZeroRtt),
                0x2 => Some(PacketType::Handshake),
                0x3 => Some(PacketType::Retry),
                _ => None,
            }
        } else {
            Some(PacketType::OneRtt)
        }
    }
}

/// QUIC packet header
#[derive(Debug, Clone)]
pub struct QuicPacketHeader {
    /// Packet type
    pub packet_type: PacketType,
    /// QUIC version
    pub version: u32,
    /// Destination connection ID
    pub dst_cid: Vec<u8>,
    /// Source connection ID
    pub src_cid: Vec<u8>,
    /// Token (for Initial packets)
    pub token: Vec<u8>,
    /// Packet number length (for 1-RTT)
    pub pn_length: usize,
    /// Packet number
    pub packet_number: u64,
    /// QUIC short-header key phase bit.
    pub key_phase: bool,
    /// Payload length
    pub payload_length: usize,
}

/// QUIC packet
#[derive(Debug, Clone)]
pub struct QuicPacket {
    /// Header
    pub header: QuicPacketHeader,
    /// Encrypted payload
    pub payload: Vec<u8>,
}

impl QuicPacket {
    /// Create a new Initial packet
    pub fn initial(
        version: u32,
        dst_cid: Vec<u8>,
        src_cid: Vec<u8>,
        token: Vec<u8>,
        packet_number: u64,
        payload: Vec<u8>,
    ) -> Self {
        QuicPacket {
            header: QuicPacketHeader {
                packet_type: PacketType::Initial,
                version,
                dst_cid,
                src_cid,
                token,
                pn_length: 4,
                packet_number,
                key_phase: false,
                payload_length: payload.len(),
            },
            payload,
        }
    }

    /// Create a Retry packet (RFC 9000 §17.2.5).
    ///
    /// Sent by a server to request the client retry with a different
    /// connection ID or to perform address validation.
    pub fn retry(
        version: u32,
        dst_cid: Vec<u8>,
        src_cid: Vec<u8>,
        token: Vec<u8>,
        integrity_tag: [u8; 16],
    ) -> Vec<u8> {
        let mut output = Vec::new();
        // First byte: 0xF0 | fixed bit (1)
        output.push(0xF1);
        // Version
        output.extend_from_slice(&version.to_be_bytes());
        // DCID length + data
        output.push(dst_cid.len() as u8);
        output.extend_from_slice(&dst_cid);
        // SCID length + data
        output.push(src_cid.len() as u8);
        output.extend_from_slice(&src_cid);
        // Retry token
        output.extend_from_slice(&token);
        // Integrity tag (16 bytes)
        output.extend_from_slice(&integrity_tag);
        output
    }

    /// Check if a packet is a Retry packet.
    pub fn is_retry(data: &[u8]) -> bool {
        if data.is_empty() {
            return false;
        }
        // First byte: 0xF0-0xFF (type 0x30 with fixed bit)
        (data[0] & 0xF0) == 0xF0
    }

    /// Create a 1-RTT packet
    pub fn one_rtt(dst_cid: Vec<u8>, packet_number: u64, payload: Vec<u8>) -> Self {
        Self::one_rtt_with_key_phase(dst_cid, packet_number, false, payload)
    }

    /// Create a 1-RTT packet with an explicit key phase bit.
    pub fn one_rtt_with_key_phase(
        dst_cid: Vec<u8>,
        packet_number: u64,
        key_phase: bool,
        payload: Vec<u8>,
    ) -> Self {
        let pn_length = packet_number_length_for_value(packet_number);
        QuicPacket {
            header: QuicPacketHeader {
                packet_type: PacketType::OneRtt,
                version: 0,
                dst_cid,
                src_cid: Vec::new(),
                token: Vec::new(),
                pn_length,
                packet_number,
                key_phase,
                payload_length: payload.len(),
            },
            payload,
        }
    }

    /// Serialize packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = self.header_to_bytes_aad_with_payload_len(self.payload.len());
        // Append payload
        output.extend_from_slice(&self.payload);
        output
    }

    /// Serialize the packet header for use as AEAD AAD (RFC 9001 §5.2).
    ///
    /// The AAD is the unprotected packet header — everything before the
    /// encrypted payload. For long-header packets, this includes the
    /// packet number (since we don't implement header protection yet).
    pub fn header_to_bytes_aad(&self) -> Vec<u8> {
        self.header_to_bytes_aad_with_payload_len(self.payload.len())
    }

    /// Serialize packet header using an explicit payload length.
    ///
    /// For protected long-header packets, callers know the encrypted payload
    /// length before encryption because AEAD adds a fixed 16-byte tag. This
    /// helper lets AAD encode the wire length while the packet still holds
    /// plaintext.
    pub fn header_to_bytes_aad_with_payload_len(&self, payload_len: usize) -> Vec<u8> {
        let mut output = Vec::new();

        match self.header.packet_type {
            PacketType::Initial | PacketType::ZeroRtt | PacketType::Handshake => {
                let pn_length = self.long_header_packet_number_length();
                let first_byte = self.header.packet_type.to_byte() | 0x0C | (pn_length as u8 - 1);
                output.push(first_byte);
                output.extend_from_slice(&self.header.version.to_be_bytes());
                output.push(self.header.dst_cid.len() as u8);
                output.extend_from_slice(&self.header.dst_cid);
                output.push(self.header.src_cid.len() as u8);
                output.extend_from_slice(&self.header.src_cid);

                if self.header.packet_type == PacketType::Initial {
                    // Token length as varint
                    quic_encode_varint(self.header.token.len() as u64, &mut output);
                    output.extend_from_slice(&self.header.token);
                }

                // QUIC long-header length includes packet number bytes.
                quic_encode_varint((pn_length + payload_len) as u64, &mut output);

                let pn_bytes = self.header.packet_number.to_be_bytes();
                output.extend_from_slice(&pn_bytes[8 - pn_length..]);
            }
            PacketType::OneRtt => {
                let key_phase = if self.header.key_phase { 0x04 } else { 0x00 };
                let first_byte = 0x40 | key_phase | (self.header.pn_length as u8 - 1);
                output.push(first_byte);
                output.extend_from_slice(&self.header.dst_cid);
                let pn_bytes = self.header.packet_number.to_be_bytes();
                output.extend_from_slice(&pn_bytes[8 - self.header.pn_length..]);
            }
            PacketType::Retry => {
                output.push(0xF1);
                output.extend_from_slice(&self.header.version.to_be_bytes());
                output.push(self.header.dst_cid.len() as u8);
                output.extend_from_slice(&self.header.dst_cid);
                output.push(self.header.src_cid.len() as u8);
                output.extend_from_slice(&self.header.src_cid);
                output.extend_from_slice(&self.header.token);
            }
        }

        output
    }

    fn long_header_packet_number_length(&self) -> usize {
        self.header.pn_length.clamp(1, 4)
    }

    /// Parse packet from bytes
    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), String> {
        Self::from_bytes_with_short_dcid_len(data, 8)
    }

    /// Parse packet from bytes, using the supplied Destination CID length for
    /// short-header packets.
    ///
    /// QUIC short headers do not carry a CID length on the wire; callers must
    /// know the expected local Destination CID length from connection state.
    pub fn from_bytes_with_short_dcid_len(
        data: &[u8],
        short_dcid_len: usize,
    ) -> Result<(Self, usize), String> {
        if data.is_empty() {
            return Err("Empty data".to_string());
        }

        let first_byte = data[0];
        if first_byte & 0x40 == 0 {
            return Err("QUIC fixed bit is not set".to_string());
        }

        if first_byte & 0x80 != 0 {
            // Long header
            Self::parse_long_header(data)
        } else {
            // Short header (1-RTT)
            Self::parse_short_header(data, short_dcid_len)
        }
    }

    /// Parse long header packet
    fn parse_long_header(data: &[u8]) -> Result<(Self, usize), String> {
        if data.len() < 7 {
            return Err("Header too short".to_string());
        }

        let packet_type =
            PacketType::from_byte(data[0]).ok_or_else(|| "Invalid packet type".to_string())?;

        let version = read_u32_be(data, 1);

        let mut pos = 5;

        // Destination CID
        let dst_cid_len = data[pos] as usize;
        pos += 1;
        if dst_cid_len > 20 {
            return Err("DST CID too long".to_string());
        }
        if pos + dst_cid_len > data.len() {
            return Err("DST CID too long".to_string());
        }
        let dst_cid = data[pos..pos + dst_cid_len].to_vec();
        pos += dst_cid_len;

        // Source CID
        let src_cid_len = data[pos] as usize;
        pos += 1;
        if src_cid_len > 20 {
            return Err("SRC CID too long".to_string());
        }
        if pos + src_cid_len > data.len() {
            return Err("SRC CID too long".to_string());
        }
        let src_cid = data[pos..pos + src_cid_len].to_vec();
        pos += src_cid_len;

        if packet_type == PacketType::Retry {
            if data.len().saturating_sub(pos) < 16 {
                return Err("Retry packet missing integrity tag".to_string());
            }
            let token_end = data.len() - 16;
            let token = data[pos..token_end].to_vec();
            let payload = data[token_end..].to_vec();
            return Ok((
                QuicPacket {
                    header: QuicPacketHeader {
                        packet_type,
                        version,
                        dst_cid,
                        src_cid,
                        token,
                        pn_length: 0,
                        packet_number: 0,
                        key_phase: false,
                        payload_length: payload.len(),
                    },
                    payload,
                },
                data.len(),
            ));
        }

        // Token (Initial only)
        let mut token = Vec::new();
        if packet_type == PacketType::Initial {
            let (token_len, bytes_read) =
                quic_decode_varint(&data[pos..]).map_err(|e| e.to_string())?;
            let token_len = token_len as usize;
            pos += bytes_read;
            if pos + token_len > data.len() {
                return Err("Token too long".to_string());
            }
            token = data[pos..pos + token_len].to_vec();
            pos += token_len;
        }

        // Payload length
        let (payload_len, bytes_read) =
            quic_decode_varint(&data[pos..]).map_err(|e| e.to_string())?;
        pos += bytes_read;

        let pn_length = get_packet_number_length(data[0]);
        if payload_len < pn_length as u64 {
            return Err("Payload length shorter than packet number".to_string());
        }

        if pos + pn_length > data.len() {
            return Err("Packet number missing".to_string());
        }
        let mut pn_bytes = [0u8; 8];
        pn_bytes[8 - pn_length..].copy_from_slice(&data[pos..pos + pn_length]);
        let packet_number = read_u64_be(&pn_bytes, 0);
        pos += pn_length;

        let payload_len = payload_len as usize - pn_length;

        // Payload
        if pos + payload_len > data.len() {
            return Err("Payload incomplete".to_string());
        }
        let payload = data[pos..pos + payload_len].to_vec();

        let total_len = pos + payload_len;

        Ok((
            QuicPacket {
                header: QuicPacketHeader {
                    packet_type,
                    version,
                    dst_cid,
                    src_cid,
                    token,
                    pn_length,
                    packet_number,
                    key_phase: false,
                    payload_length: payload_len,
                },
                payload,
            },
            total_len,
        ))
    }

    /// Parse short header (1-RTT)
    fn parse_short_header(data: &[u8], dst_cid_len: usize) -> Result<(Self, usize), String> {
        if data.len() < 3 {
            return Err("Short header too short".to_string());
        }
        if dst_cid_len > 20 {
            return Err("Short header CID too long".to_string());
        }

        let pn_length = ((data[0] & 0x03) + 1) as usize;
        let key_phase = data[0] & 0x04 != 0;
        let mut pos = 1;

        if pos + dst_cid_len > data.len() {
            return Err("Short header CID missing".to_string());
        }
        let dst_cid = data[pos..pos + dst_cid_len].to_vec();
        pos += dst_cid_len;

        // Packet number
        if pos + pn_length > data.len() {
            return Err("Packet number missing".to_string());
        }
        let mut pn_bytes = [0u8; 8];
        pn_bytes[8 - pn_length..].copy_from_slice(&data[pos..pos + pn_length]);
        let packet_number = read_u64_be(&pn_bytes, 0);
        pos += pn_length;

        let payload = data[pos..].to_vec();

        Ok((
            QuicPacket {
                header: QuicPacketHeader {
                    packet_type: PacketType::OneRtt,
                    version: 0,
                    dst_cid,
                    src_cid: Vec::new(),
                    token: Vec::new(),
                    pn_length,
                    packet_number,
                    key_phase,
                    payload_length: payload.len(),
                },
                payload,
            },
            data.len(),
        ))
    }
}

/// Get packet number length from first byte (RFC 9000 bits 0-1 encode length - 1).
pub fn get_packet_number_length(first_byte: u8) -> usize {
    ((first_byte & 0x03) + 1) as usize
}

/// Choose the shortest packet-number encoding that preserves the value.
pub fn packet_number_length_for_value(packet_number: u64) -> usize {
    if packet_number <= 0xff {
        1
    } else if packet_number <= 0xffff {
        2
    } else if packet_number <= 0x00ff_ffff {
        3
    } else {
        4
    }
}

/// Get AAD for AEAD - use this instead of manual parsing
pub fn get_packet_aad(packet: &QuicPacket) -> Vec<u8> {
    packet.header_to_bytes_aad()
}

/// Get byte offset where payload starts in a long-header packet
pub fn get_long_header_payload_offset(data: &[u8]) -> Result<usize, String> {
    if data.len() < 6 {
        return Err("Data too short".to_string());
    }

    let first_byte = data[0];
    if first_byte & 0x40 == 0 {
        return Err("QUIC fixed bit is not set".to_string());
    }
    let packet_type = PacketType::from_byte(first_byte).ok_or("Invalid packet type")?;
    if packet_type == PacketType::Retry {
        return Err("Retry packets do not have a protected payload offset".to_string());
    }

    // Start after first byte and version.
    let mut pos = 5;

    if pos >= data.len() {
        return Err("DCID length missing".to_string());
    }
    let dst_cid_len = data[pos] as usize;
    if dst_cid_len > 20 {
        return Err("DCID length exceeds QUIC maximum".to_string());
    }
    pos += 1 + dst_cid_len;

    if pos >= data.len() {
        return Err("SCID length missing".to_string());
    }
    let src_cid_len = data[pos] as usize;
    if src_cid_len > 20 {
        return Err("SCID length exceeds QUIC maximum".to_string());
    }
    pos += 1 + src_cid_len;

    if pos > data.len() {
        return Err("Connection ID exceeds packet length".to_string());
    }

    if packet_type == PacketType::Initial {
        let (token_len, consumed) =
            quic_decode_varint(&data[pos..]).map_err(|_| "Invalid token length")?;
        pos = pos
            .checked_add(consumed)
            .and_then(|pos| pos.checked_add(token_len as usize))
            .ok_or("Token length overflows packet")?;
        if pos > data.len() {
            return Err("Token exceeds packet length".to_string());
        }
    }

    let (_, consumed) = quic_decode_varint(&data[pos..]).map_err(|_| "Invalid length varint")?;
    pos += consumed;

    let pn_length = get_packet_number_length(first_byte);
    pos += pn_length;

    if pos > data.len() {
        return Err("Packet number exceeds packet length".to_string());
    }

    Ok(pos)
}

#[cfg(test)]
mod tests {
    use crate::crypto::{PacketProtection, ProtectionKeys};

    use super::*;

    #[test]
    fn test_packet_type_roundtrip() {
        assert_eq!(
            PacketType::from_byte(PacketType::Initial.to_byte()),
            Some(PacketType::Initial)
        );
        assert_eq!(
            PacketType::from_byte(PacketType::Handshake.to_byte()),
            Some(PacketType::Handshake)
        );
        assert_eq!(
            PacketType::from_byte(PacketType::OneRtt.to_byte()),
            Some(PacketType::OneRtt)
        );
    }

    #[test]
    fn test_varint_encoding() {
        let mut output = Vec::new();
        let pkt = QuicPacket::one_rtt(vec![0u8; 8], 0, vec![]);

        quic_encode_varint(0, &mut output);
        assert_eq!(output, vec![0]);

        output.clear();
        quic_encode_varint(63, &mut output);
        assert_eq!(output, vec![63]);

        output.clear();
        quic_encode_varint(64, &mut output);
        assert_eq!(output, vec![0x40, 64]);
    }

    #[test]
    fn test_initial_packet_to_bytes() {
        let pkt = QuicPacket::initial(
            1,
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![0; 8],
            vec![],
            1,
            vec![0x06, 0x00, 0x40, 0x51],
        );

        let bytes = pkt.to_bytes();
        assert!(!bytes.is_empty());
        assert!(bytes.len() > 20);
    }

    #[test]
    fn test_get_packet_number_length() {
        assert_eq!(get_packet_number_length(0xC0), 1);
        assert_eq!(get_packet_number_length(0xC1), 2);
        assert_eq!(get_packet_number_length(0xC2), 3);
        assert_eq!(get_packet_number_length(0xC3), 4);
        assert_eq!(get_packet_number_length(0xCF), 4);
    }

    #[test]
    fn test_packet_number_length_for_value() {
        assert_eq!(packet_number_length_for_value(0), 1);
        assert_eq!(packet_number_length_for_value(0xff), 1);
        assert_eq!(packet_number_length_for_value(0x100), 2);
        assert_eq!(packet_number_length_for_value(0xffff), 2);
        assert_eq!(packet_number_length_for_value(0x1_0000), 3);
        assert_eq!(packet_number_length_for_value(0xff_ffff), 3);
        assert_eq!(packet_number_length_for_value(0x1_000000), 4);
    }

    #[test]
    fn test_get_long_header_payload_offset() {
        let packet = vec![
            0xCF, 0x00, 0x00, 0x00, 0x01, // first_byte + version
            8, 1, 2, 3, 4, 5, 6, 7, 8, // dst_cid
            8, 9, 10, 11, 12, 13, 14, 15, 16, // src_cid
            0,  // token_len = 0
            7,  // length = packet number + payload
            0, 0, 0, 0, // packet number
            1, 2, 3, // payload
        ];

        let offset = get_long_header_payload_offset(&packet).unwrap();

        // Just verify offset is valid (within packet bounds)
        assert!(offset >= 5, "Offset should be >= 5");
        assert!(offset < packet.len(), "Offset should be < packet len");
    }

    #[test]
    fn test_get_long_header_payload_offset_rejects_truncated_token() {
        let packet = vec![
            0xcf, 0, 0, 0, 1, // first byte + version
            0, // dst_cid_len
            0, // src_cid_len
            4, // token_len
            0xaa, 0xbb, // truncated token
        ];

        assert!(get_long_header_payload_offset(&packet).is_err());
    }

    #[test]
    fn test_rejects_long_header_without_fixed_bit() {
        let mut bytes = QuicPacket::initial(
            1,
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![9, 10, 11, 12],
            Vec::new(),
            0,
            vec![0x06, 0x00],
        )
        .to_bytes();
        bytes[0] &= !0x40;

        assert!(QuicPacket::from_bytes(&bytes).is_err());
        assert!(get_long_header_payload_offset(&bytes).is_err());
    }

    #[test]
    fn test_rejects_short_header_without_fixed_bit() {
        let mut bytes = QuicPacket::one_rtt(vec![1, 2, 3, 4], 0, vec![0x01]).to_bytes();
        bytes[0] &= !0x40;

        assert!(QuicPacket::from_bytes_with_short_dcid_len(&bytes, 4).is_err());
    }

    #[test]
    fn test_rejects_long_header_connection_ids_over_20_bytes() {
        let bytes = vec![
            0xcf, 0, 0, 0, 1, // first byte + version
            21, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // dcid
            0, // scid_len
            0, // token_len
            4, // length
            0, 0, 0, 0, // packet number
        ];

        assert!(QuicPacket::from_bytes(&bytes).is_err());
        assert!(get_long_header_payload_offset(&bytes).is_err());
    }

    #[test]
    fn test_rejects_short_header_connection_id_length_over_20_bytes() {
        let bytes = vec![0x40, 0, 0, 0, 0];
        assert!(QuicPacket::from_bytes_with_short_dcid_len(&bytes, 21).is_err());
    }

    #[test]
    fn test_retry_packet_roundtrip() {
        let token = vec![0xaa, 0xbb, 0xcc, 0xdd];
        let tag = [0x11; 16];
        let bytes = QuicPacket::retry(1, vec![1, 2, 3, 4], vec![5, 6, 7, 8], token.clone(), tag);

        assert!(QuicPacket::is_retry(&bytes));
        let (parsed, consumed) = QuicPacket::from_bytes(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(parsed.header.packet_type, PacketType::Retry);
        assert_eq!(parsed.header.version, 1);
        assert_eq!(parsed.header.dst_cid, vec![1, 2, 3, 4]);
        assert_eq!(parsed.header.src_cid, vec![5, 6, 7, 8]);
        assert_eq!(parsed.header.token, token);
        assert_eq!(parsed.header.pn_length, 0);
        assert_eq!(parsed.header.packet_number, 0);
        assert_eq!(parsed.payload, tag.to_vec());
        assert_eq!(parsed.to_bytes(), bytes);
        assert!(get_long_header_payload_offset(&bytes).is_err());
    }

    #[test]
    fn test_retry_packet_rejects_missing_integrity_tag() {
        let bytes = vec![
            0xf1, 0, 0, 0, 1, // first byte + version
            4, 1, 2, 3, 4, // dcid
            4, 5, 6, 7, 8, // scid
            0xaa, 0xbb, // token, but no 16-byte tag
        ];

        assert!(QuicPacket::from_bytes(&bytes).is_err());
    }

    #[test]
    fn test_initial_packet_roundtrip_with_token() {
        let payload = vec![0x06, 0x00, 0x04, 1, 2, 3, 4];
        let pkt = QuicPacket::initial(
            1,
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![9, 10, 11, 12],
            vec![0xaa, 0xbb, 0xcc],
            0x0102_0304,
            payload.clone(),
        );

        let bytes = pkt.to_bytes();
        let offset = get_long_header_payload_offset(&bytes).unwrap();
        assert_eq!(offset, bytes.len() - payload.len());

        let (parsed, consumed) = QuicPacket::from_bytes(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(parsed.header.packet_type, PacketType::Initial);
        assert_eq!(parsed.header.version, 1);
        assert_eq!(parsed.header.dst_cid, vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(parsed.header.src_cid, vec![9, 10, 11, 12]);
        assert_eq!(parsed.header.token, vec![0xaa, 0xbb, 0xcc]);
        assert_eq!(parsed.header.pn_length, 4);
        assert_eq!(parsed.header.packet_number, 0x0102_0304);
        assert_eq!(parsed.payload, payload);
    }

    #[test]
    fn test_handshake_packet_roundtrip() {
        let pkt = QuicPacket {
            header: QuicPacketHeader {
                packet_type: PacketType::Handshake,
                version: 1,
                dst_cid: vec![1, 2, 3, 4],
                src_cid: vec![5, 6, 7, 8],
                token: Vec::new(),
                pn_length: 2,
                packet_number: 0x1234,
                key_phase: false,
                payload_length: 3,
            },
            payload: vec![0xaa, 0xbb, 0xcc],
        };

        let bytes = pkt.to_bytes();
        let offset = get_long_header_payload_offset(&bytes).unwrap();
        assert_eq!(offset, bytes.len() - pkt.payload.len());

        let (parsed, consumed) = QuicPacket::from_bytes(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(parsed.header.packet_type, PacketType::Handshake);
        assert_eq!(parsed.header.pn_length, 2);
        assert_eq!(parsed.header.packet_number, 0x1234);
        assert_eq!(parsed.payload, pkt.payload);
    }

    fn protected_long_header_roundtrip(packet: QuicPacket) {
        let keys = ProtectionKeys::test_keys();
        let mut encryptor = PacketProtection::new(&keys);
        let aad = packet.header_to_bytes_aad_with_payload_len(packet.payload.len() + 16);
        let encrypted = encryptor
            .protect_with_packet_number(packet.header.packet_number, &aad, &packet.payload)
            .expect("encrypt packet");
        let mut packet_bytes = aad.clone();
        packet_bytes.extend_from_slice(&encrypted);

        let (parsed, consumed) = QuicPacket::from_bytes(&packet_bytes).expect("parse packet");
        assert_eq!(consumed, packet_bytes.len());
        assert_eq!(parsed.header.packet_type, packet.header.packet_type);
        assert_eq!(parsed.header.packet_number, packet.header.packet_number);
        assert_eq!(parsed.header_to_bytes_aad(), aad);
        assert_eq!(parsed.payload, encrypted);

        let mut wrong_decryptor = PacketProtection::new(&keys);
        assert!(wrong_decryptor
            .unprotect(&parsed.header_to_bytes_aad(), 0, &parsed.payload)
            .is_err());

        let mut decryptor = PacketProtection::new(&keys);
        let plaintext = decryptor
            .unprotect(
                &parsed.header_to_bytes_aad(),
                parsed.header.packet_number,
                &parsed.payload,
            )
            .expect("decrypt packet");
        assert_eq!(plaintext, packet.payload);
    }

    #[test]
    fn test_protected_initial_packet_roundtrip() {
        protected_long_header_roundtrip(QuicPacket::initial(
            1,
            vec![1, 2, 3, 4, 5, 6, 7, 8],
            vec![9, 10, 11, 12],
            vec![0xaa, 0xbb],
            7,
            vec![0x06, 0x00, 0x04, 1, 2, 3, 4],
        ));
    }

    #[test]
    fn test_protected_handshake_packet_roundtrip() {
        protected_long_header_roundtrip(QuicPacket {
            header: QuicPacketHeader {
                packet_type: PacketType::Handshake,
                version: 1,
                dst_cid: vec![1, 2, 3, 4],
                src_cid: vec![5, 6, 7, 8],
                token: Vec::new(),
                pn_length: 2,
                packet_number: 7,
                key_phase: false,
                payload_length: 3,
            },
            payload: vec![0xaa, 0xbb, 0xcc],
        });
    }

    #[test]
    fn test_protected_zero_rtt_packet_roundtrip() {
        protected_long_header_roundtrip(QuicPacket {
            header: QuicPacketHeader {
                packet_type: PacketType::ZeroRtt,
                version: 1,
                dst_cid: vec![1, 2, 3, 4],
                src_cid: vec![5, 6, 7, 8],
                token: Vec::new(),
                pn_length: 4,
                packet_number: 7,
                key_phase: false,
                payload_length: 3,
            },
            payload: vec![0x08, 0xaa, 0xbb],
        });
    }

    #[test]
    fn test_protected_one_rtt_packet_roundtrip() {
        let packet = QuicPacket::one_rtt(vec![1, 2, 3, 4, 5, 6, 7, 8], 7, vec![0x01]);
        let keys = ProtectionKeys::test_keys();
        let mut encryptor = PacketProtection::new(&keys);
        let aad = packet.header_to_bytes_aad();
        let encrypted = encryptor
            .protect_with_packet_number(packet.header.packet_number, &aad, &packet.payload)
            .expect("encrypt packet");
        let mut packet_bytes = aad.clone();
        packet_bytes.extend_from_slice(&encrypted);

        let (parsed, consumed) = QuicPacket::from_bytes(&packet_bytes).expect("parse packet");
        assert_eq!(consumed, packet_bytes.len());
        assert_eq!(parsed.header.packet_type, PacketType::OneRtt);
        assert_eq!(parsed.header.packet_number, 7);
        assert!(!parsed.header.key_phase);
        assert_eq!(parsed.header_to_bytes_aad(), aad);

        let mut wrong_decryptor = PacketProtection::new(&keys);
        assert!(wrong_decryptor
            .unprotect(&parsed.header_to_bytes_aad(), 0, &parsed.payload)
            .is_err());

        let mut decryptor = PacketProtection::new(&keys);
        let plaintext = decryptor
            .unprotect(
                &parsed.header_to_bytes_aad(),
                parsed.header.packet_number,
                &parsed.payload,
            )
            .expect("decrypt packet");
        assert_eq!(plaintext, packet.payload);
    }

    #[test]
    fn test_one_rtt_key_phase_roundtrip() {
        let packet =
            QuicPacket::one_rtt_with_key_phase(vec![1, 2, 3, 4, 5, 6, 7, 8], 9, true, vec![0x01]);
        let bytes = packet.to_bytes();
        assert_ne!(bytes[0] & 0x04, 0);

        let (parsed, consumed) = QuicPacket::from_bytes(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(parsed.header.packet_type, PacketType::OneRtt);
        assert_eq!(parsed.header.packet_number, 9);
        assert!(parsed.header.key_phase);
        assert_eq!(parsed.header_to_bytes_aad(), packet.header_to_bytes_aad());
        assert_eq!(parsed.payload, packet.payload);
    }

    #[test]
    fn test_one_rtt_parses_explicit_destination_cid_length() {
        let packet = QuicPacket::one_rtt(vec![1, 2, 3, 4], 9, vec![0x01, 0x02]);
        let bytes = packet.to_bytes();

        let (parsed, consumed) = QuicPacket::from_bytes_with_short_dcid_len(&bytes, 4).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(parsed.header.dst_cid, vec![1, 2, 3, 4]);
        assert_eq!(parsed.header.packet_number, 9);
        assert_eq!(parsed.payload, vec![0x01, 0x02]);

        assert!(QuicPacket::from_bytes(&bytes).is_err());
    }

    #[test]
    fn test_one_rtt_large_packet_number_is_not_truncated() {
        let packet = QuicPacket::one_rtt(vec![1, 2, 3, 4, 5, 6, 7, 8], 0x1234, vec![0x01]);
        assert_eq!(packet.header.pn_length, 2);

        let bytes = packet.to_bytes();
        let (parsed, consumed) = QuicPacket::from_bytes(&bytes).unwrap();
        assert_eq!(consumed, bytes.len());
        assert_eq!(parsed.header.packet_number, 0x1234);
        assert_eq!(parsed.header.pn_length, 2);
        assert_eq!(parsed.header_to_bytes_aad(), packet.header_to_bytes_aad());
    }
}
