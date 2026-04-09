//! QUIC frame types (RFC 9000 Section 19)

/// QUIC frame types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuicFrameType {
    Padding = 0x00,
    Ping = 0x01,
    Ack = 0x02,
    AckECN = 0x03,
    ResetStream = 0x04,
    StopSending = 0x05,
    Crypto = 0x06,
    NewToken = 0x07,
    Stream = 0x08,
    MaxData = 0x10,
    MaxStreamData = 0x11,
    MaxStreamsBidi = 0x12,
    MaxStreamsUni = 0x13,
    DataBlocked = 0x14,
    StreamDataBlocked = 0x15,
    StreamsBlockedBidi = 0x16,
    StreamsBlockedUni = 0x17,
    NewConnectionId = 0x18,
    RetireConnectionId = 0x19,
    PathChallenge = 0x1A,
    PathResponse = 0x1B,
    ConnectionClose = 0x1C,
    ConnectionCloseApplication = 0x1D,
    HandshakeDone = 0x1E,
}

impl QuicFrameType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(QuicFrameType::Padding),
            0x01 => Some(QuicFrameType::Ping),
            0x02 => Some(QuicFrameType::Ack),
            0x03 => Some(QuicFrameType::AckECN),
            0x04 => Some(QuicFrameType::ResetStream),
            0x05 => Some(QuicFrameType::StopSending),
            0x06 => Some(QuicFrameType::Crypto),
            0x07 => Some(QuicFrameType::NewToken),
            0x10 => Some(QuicFrameType::MaxData),
            0x11 => Some(QuicFrameType::MaxStreamData),
            0x12 => Some(QuicFrameType::MaxStreamsBidi),
            0x13 => Some(QuicFrameType::MaxStreamsUni),
            0x14 => Some(QuicFrameType::DataBlocked),
            0x15 => Some(QuicFrameType::StreamDataBlocked),
            0x16 => Some(QuicFrameType::StreamsBlockedBidi),
            0x17 => Some(QuicFrameType::StreamsBlockedUni),
            0x18 => Some(QuicFrameType::NewConnectionId),
            0x19 => Some(QuicFrameType::RetireConnectionId),
            0x1A => Some(QuicFrameType::PathChallenge),
            0x1B => Some(QuicFrameType::PathResponse),
            0x1C => Some(QuicFrameType::ConnectionClose),
            0x1D => Some(QuicFrameType::ConnectionCloseApplication),
            0x1E => Some(QuicFrameType::HandshakeDone),
            v if v >= 0x08 && v <= 0x0F => Some(QuicFrameType::Stream),
            _ => None,
        }
    }
}

/// QUIC frame
#[derive(Debug, Clone)]
pub enum QuicFrame {
    /// PADDING frame
    Padding { length: usize },
    /// PING frame
    Ping,
    /// ACK frame
    Ack {
        largest_acknowledged: u64,
        ack_delay: u64,
        ack_range_count: u64,
        first_ack_range: u64,
    },
    /// CRYPTO frame
    Crypto { offset: u64, data: Vec<u8> },
    /// STREAM frame
    Stream {
        stream_id: u64,
        offset: u64,
        fin: bool,
        data: Vec<u8>,
    },
    /// MAX_DATA frame
    MaxData { max_data: u64 },
    /// MAX_STREAM_DATA frame
    MaxStreamData { stream_id: u64, max_stream_data: u64 },
    /// NEW_CONNECTION_ID frame
    NewConnectionId {
        sequence_number: u64,
        retire_prior_to: u64,
        connection_id: Vec<u8>,
        stateless_reset_token: [u8; 16],
    },
    /// CONNECTION_CLOSE frame
    ConnectionClose {
        error_code: u64,
        frame_type: u64,
        reason: Vec<u8>,
    },
    /// HANDSHAKE_DONE frame
    HandshakeDone,
}

impl QuicFrame {
    /// Serialize frame to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            QuicFrame::Padding { length } => vec![0x00; *length],
            QuicFrame::Ping => vec![0x01],
            QuicFrame::HandshakeDone => vec![0x1E],
            QuicFrame::Crypto { offset, data } => {
                let mut output = vec![0x06];
                Self::encode_varint(*offset, &mut output);
                Self::encode_varint(data.len() as u64, &mut output);
                output.extend_from_slice(data);
                output
            }
            QuicFrame::Stream {
                stream_id,
                offset,
                fin,
                data,
            } => {
                let mut type_byte = 0x08u8;
                if *fin {
                    type_byte |= 0x01;
                }
                type_byte |= 0x02; // Offset present
                type_byte |= 0x04; // Length present

                let mut output = vec![type_byte];
                Self::encode_varint(*stream_id, &mut output);
                Self::encode_varint(*offset, &mut output);
                Self::encode_varint(data.len() as u64, &mut output);
                output.extend_from_slice(data);
                output
            }
            QuicFrame::MaxData { max_data } => {
                let mut output = vec![0x10];
                Self::encode_varint(*max_data, &mut output);
                output
            }
            QuicFrame::ConnectionClose {
                error_code,
                frame_type,
                reason,
            } => {
                let mut output = vec![0x1C];
                Self::encode_varint(*error_code, &mut output);
                Self::encode_varint(*frame_type, &mut output);
                Self::encode_varint(reason.len() as u64, &mut output);
                output.extend_from_slice(reason);
                output
            }
            _ => vec![],
        }
    }

    /// Parse frame from bytes
    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), String> {
        if data.is_empty() {
            return Err("Empty frame data".to_string());
        }

        let frame_type = QuicFrameType::from_u8(data[0])
            .ok_or_else(|| format!("Unknown frame type: 0x{:02x}", data[0]))?;

        match frame_type {
            QuicFrameType::Padding => {
                let mut length = 1;
                while length < data.len() && data[length] == 0x00 {
                    length += 1;
                }
                Ok((QuicFrame::Padding { length }, length))
            }
            QuicFrameType::Ping => Ok((QuicFrame::Ping, 1)),
            QuicFrameType::HandshakeDone => Ok((QuicFrame::HandshakeDone, 1)),
            QuicFrameType::Crypto => {
                let (offset, offset_len) = Self::decode_varint(&data[1..])?;
                let pos = 1 + offset_len;
                let (data_len, data_len_len) = Self::decode_varint(&data[pos..])?;
                let data_start = pos + data_len_len;
                let data_end = data_start + data_len as usize;
                if data_end > data.len() {
                    return Err("Crypto frame data incomplete".to_string());
                }
                let frame_data = data[data_start..data_end].to_vec();
                Ok((
                    QuicFrame::Crypto {
                        offset,
                        data: frame_data,
                    },
                    data_end,
                ))
            }
            QuicFrameType::Stream => {
                let type_byte = data[0];
                let fin = (type_byte & 0x01) != 0;
                let has_offset = (type_byte & 0x02) != 0;
                let has_length = (type_byte & 0x04) != 0;

                let mut pos = 1;
                let (stream_id, n) = Self::decode_varint(&data[pos..])?;
                pos += n;

                let offset = if has_offset {
                    let (v, n) = Self::decode_varint(&data[pos..])?;
                    pos += n;
                    v
                } else {
                    0
                };

                let (data_len, n) = if has_length {
                    let (v, n) = Self::decode_varint(&data[pos..])?;
                    pos += n;
                    (v, n)
                } else {
                    (data.len().saturating_sub(pos) as u64, 0)
                };

                let data_end = pos + data_len as usize;
                let frame_data = data[pos..data_end.min(data.len())].to_vec();

                Ok((
                    QuicFrame::Stream {
                        stream_id,
                        offset,
                        fin,
                        data: frame_data,
                    },
                    data_end.min(data.len()),
                ))
            }
            QuicFrameType::MaxData => {
                let (max_data, n) = Self::decode_varint(&data[1..])?;
                Ok((QuicFrame::MaxData { max_data }, 1 + n))
            }
            QuicFrameType::ConnectionClose => {
                let mut pos = 1;
                let (error_code, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let (frame_type, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let (reason_len, n) = Self::decode_varint(&data[pos..])?;
                pos += n;
                let reason = data[pos..pos + reason_len as usize].to_vec();
                pos += reason_len as usize;
                Ok((
                    QuicFrame::ConnectionClose {
                        error_code,
                        frame_type,
                        reason,
                    },
                    pos,
                ))
            }
            _ => Err(format!("Frame type {:?} not yet implemented", frame_type)),
        }
    }

    fn encode_varint(value: u64, output: &mut Vec<u8>) {
        if value < 64 {
            output.push(value as u8);
        } else if value < 16384 {
            output.push(((value >> 8) as u8) | 0x40);
            output.push(value as u8);
        } else if value < 1073741824 {
            output.push(((value >> 24) as u8) | 0x80);
            output.extend_from_slice(&(value as u32).to_be_bytes());
        } else {
            output.push(((value >> 56) as u8) | 0xC0);
            output.extend_from_slice(&value.to_be_bytes());
        }
    }

    fn decode_varint(data: &[u8]) -> Result<(u64, usize), String> {
        if data.is_empty() {
            return Err("Empty varint".to_string());
        }
        let first = data[0];
        let len = match first >> 6 {
            0 => 1,
            1 => 2,
            2 => 4,
            3 => 8,
            _ => return Err("Invalid varint".to_string()),
        };
        if data.len() < len {
            return Err("Incomplete varint".to_string());
        }
        let value = match len {
            1 => (first & 0x3F) as u64,
            2 => u16::from_be_bytes([first & 0x3F, data[1]]) as u64,
            4 => {
                let mut b = [first & 0x3F, data[1], data[2], data[3]];
                u32::from_be_bytes(b) as u64
            }
            8 => {
                let mut b: [u8; 8] = data[..8].try_into().unwrap();
                b[0] &= 0x3F;
                u64::from_be_bytes(b)
            }
            _ => unreachable!(),
        };
        Ok((value, len))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_frame() {
        let frame = QuicFrame::Ping;
        let bytes = frame.to_bytes();
        assert_eq!(bytes, vec![0x01]);

        let (parsed, len) = QuicFrame::from_bytes(&bytes).unwrap();
        assert!(matches!(parsed, QuicFrame::Ping));
        assert_eq!(len, 1);
    }

    #[test]
    fn test_padding_frame() {
        let frame = QuicFrame::Padding { length: 10 };
        let bytes = frame.to_bytes();
        assert_eq!(bytes.len(), 10);
        assert!(bytes.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_stream_frame() {
        let frame = QuicFrame::Stream {
            stream_id: 0,
            offset: 0,
            fin: true,
            data: b"hello".to_vec(),
        };
        let bytes = frame.to_bytes();
        assert!(!bytes.is_empty());

        let (parsed, _) = QuicFrame::from_bytes(&bytes).unwrap();
        if let QuicFrame::Stream { fin, data, .. } = parsed {
            assert!(fin);
            assert_eq!(data, b"hello");
        } else {
            panic!("Expected Stream frame");
        }
    }

    #[test]
    fn test_crypto_frame() {
        let frame = QuicFrame::Crypto {
            offset: 0,
            data: vec![0x01, 0x02, 0x03],
        };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x06);
    }

    #[test]
    fn test_max_data_frame() {
        let frame = QuicFrame::MaxData { max_data: 65535 };
        let bytes = frame.to_bytes();
        assert_eq!(bytes[0], 0x10);
    }
}
