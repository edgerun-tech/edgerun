//! QUIC packet format (RFC 9000 Section 17)

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
    /// Encode to byte value
    pub fn to_byte(self) -> u8 {
        match self {
            PacketType::Initial => 0x00,
            PacketType::ZeroRtt => 0x10,
            PacketType::Handshake => 0x20,
            PacketType::Retry => 0x30,
            PacketType::OneRtt => 0x40,
        }
    }

    /// Decode from byte value
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte & 0xF0 {
            0x00 => Some(PacketType::Initial),
            0x10 => Some(PacketType::ZeroRtt),
            0x20 => Some(PacketType::Handshake),
            0x30 => Some(PacketType::Retry),
            0x40..=0xFF => Some(PacketType::OneRtt),
            _ => None,
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
                pn_length: 0,
                packet_number,
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
    pub fn one_rtt(
        dst_cid: Vec<u8>,
        packet_number: u64,
        payload: Vec<u8>,
    ) -> Self {
        QuicPacket {
            header: QuicPacketHeader {
                packet_type: PacketType::OneRtt,
                version: 0,
                dst_cid,
                src_cid: Vec::new(),
                token: Vec::new(),
                pn_length: 1,
                packet_number,
                payload_length: payload.len(),
            },
            payload,
        }
    }

    /// Serialize packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = self.header_to_bytes_aad();
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
        let mut output = Vec::new();

        match self.header.packet_type {
            PacketType::Initial | PacketType::ZeroRtt | PacketType::Handshake => {
                let first_byte = self.header.packet_type.to_byte() | 0x0C;
                output.push(first_byte);
                output.extend_from_slice(&self.header.version.to_be_bytes());
                output.push(self.header.dst_cid.len() as u8);
                output.extend_from_slice(&self.header.dst_cid);
                output.push(self.header.src_cid.len() as u8);
                output.extend_from_slice(&self.header.src_cid);

                if self.header.packet_type == PacketType::Initial {
                    // Token length as varint
                    self.encode_varint(self.header.token.len() as u64, &mut output);
                    output.extend_from_slice(&self.header.token);
                }

                // Payload length as varint
                self.encode_varint(self.header.payload_length as u64, &mut output);

                // Packet number (2 bytes, truncated)
                let pn_bytes = self.header.packet_number.to_be_bytes();
                output.extend_from_slice(&pn_bytes[6..]);
            }
            PacketType::OneRtt => {
                let first_byte = 0x40 | (self.header.pn_length as u8 - 1);
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

    /// Parse packet from bytes
    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), String> {
        if data.is_empty() {
            return Err("Empty data".to_string());
        }

        let first_byte = data[0];

        if first_byte & 0x80 != 0 {
            // Long header
            Self::parse_long_header(data)
        } else {
            // Short header (1-RTT)
            Self::parse_short_header(data)
        }
    }

    /// Parse long header packet
    fn parse_long_header(data: &[u8]) -> Result<(Self, usize), String> {
        if data.len() < 7 {
            return Err("Header too short".to_string());
        }

        let packet_type = PacketType::from_byte(data[0])
            .ok_or_else(|| "Invalid packet type".to_string())?;

        let version = u32::from_be_bytes([data[1], data[2], data[3], data[4]]);

        let mut pos = 5;

        // Destination CID
        let dst_cid_len = data[pos] as usize;
        pos += 1;
        if pos + dst_cid_len > data.len() {
            return Err("DST CID too long".to_string());
        }
        let dst_cid = data[pos..pos + dst_cid_len].to_vec();
        pos += dst_cid_len;

        // Source CID
        let src_cid_len = data[pos] as usize;
        pos += 1;
        if pos + src_cid_len > data.len() {
            return Err("SRC CID too long".to_string());
        }
        let src_cid = data[pos..pos + src_cid_len].to_vec();
        pos += src_cid_len;

        // Token (Initial only)
        let mut token = Vec::new();
        if packet_type == PacketType::Initial {
            if pos + 8 > data.len() {
                return Err("Token length missing".to_string());
            }
            let token_len = u64::from_be_bytes(
                data[pos..pos + 8].try_into().map_err(|_| "Token len parse error")?,
            ) as usize;
            pos += 8;
            if pos + token_len > data.len() {
                return Err("Token too long".to_string());
            }
            token = data[pos..pos + token_len].to_vec();
            pos += token_len;
        }

        // Payload length
        let (payload_len, bytes_read) = Self::decode_varint(&data[pos..])
            .map_err(|e| e.to_string())?;
        pos += bytes_read;

        // Packet number (4 bytes)
        if pos + 4 > data.len() {
            return Err("Packet number missing".to_string());
        }
        let packet_number = u64::from_be_bytes([
            0, 0, 0, 0, data[pos], data[pos + 1], data[pos + 2], data[pos + 3],
        ]);
        pos += 4;

        // Payload
        if pos + payload_len as usize > data.len() {
            return Err("Payload incomplete".to_string());
        }
        let payload = data[pos..pos + payload_len as usize].to_vec();

        let total_len = pos + payload_len as usize;

        Ok((
            QuicPacket {
                header: QuicPacketHeader {
                    packet_type,
                    version,
                    dst_cid,
                    src_cid,
                    token,
                    pn_length: 4,
                    packet_number,
                    payload_length: payload_len as usize,
                },
                payload,
            },
            total_len,
        ))
    }

    /// Parse short header (1-RTT)
    fn parse_short_header(data: &[u8]) -> Result<(Self, usize), String> {
        if data.len() < 3 {
            return Err("Short header too short".to_string());
        }

        let pn_length = ((data[0] & 0x03) + 1) as usize;
        let mut pos = 1;

        // Assume 8-byte CID for simplicity
        let dst_cid = data[pos..pos + 8].to_vec();
        pos += 8;

        // Packet number
        if pos + pn_length > data.len() {
            return Err("Packet number missing".to_string());
        }
        let mut pn_bytes = [0u8; 8];
        pn_bytes[8 - pn_length..].copy_from_slice(&data[pos..pos + pn_length]);
        let packet_number = u64::from_be_bytes(pn_bytes);
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
                    payload_length: payload.len(),
                },
                payload,
            },
            data.len(),
        ))
    }

    /// Encode variable-length integer
    fn encode_varint(&self, value: u64, output: &mut Vec<u8>) {
        edgerun_encoding::quic_varint::encode_varint(value, output)
    }

    /// Decode variable-length integer
    fn decode_varint(data: &[u8]) -> Result<(u64, usize), std::io::Error> {
        edgerun_encoding::quic_varint::decode_varint(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::UnexpectedEof, format!("{e}")))
    }
}

#[cfg(test)]
mod tests {
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

        pkt.encode_varint(0, &mut output);
        assert_eq!(output, vec![0]);

        output.clear();
        pkt.encode_varint(63, &mut output);
        assert_eq!(output, vec![63]);

        output.clear();
        pkt.encode_varint(64, &mut output);
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
}
