//! HTTP/3 frame types (RFC 9114 Section 7.1)

/// HTTP/3 frame types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Http3FrameType {
    /// DATA frame
    Data = 0x00,
    /// HEADERS frame
    Headers = 0x01,
    /// Reserved frame (for greasing)
    Reserved = 0x02,
    /// CANCEL_PUSH frame
    CancelPush = 0x03,
    /// SETTINGS frame
    Settings = 0x04,
    /// PUSH_PROMISE frame
    PushPromise = 0x05,
    /// Reserved frame 2 (greasing)
    Reserved2 = 0x06,
    /// MAX_PUSH_ID frame
    MaxPushId = 0x07,
    /// GOAWAY frame
    Goaway = 0x08,
    /// STREAMS_BLOCKED frame
    StreamsBlocked = 0x09,
}

impl Http3FrameType {
    pub fn from_u64(value: u64) -> Option<Self> {
        match value {
            0x00 => Some(Http3FrameType::Data),
            0x01 => Some(Http3FrameType::Headers),
            0x03 => Some(Http3FrameType::CancelPush),
            0x04 => Some(Http3FrameType::Settings),
            0x05 => Some(Http3FrameType::PushPromise),
            0x07 => Some(Http3FrameType::MaxPushId),
            0x08 => Some(Http3FrameType::Goaway),
            0x09 => Some(Http3FrameType::StreamsBlocked),
            v if v % 0x1F == 0x02 || v % 0x1F == 0x06 => Some(Http3FrameType::Reserved),
            _ => None,
        }
    }
}

/// HTTP/3 frame
#[derive(Debug, Clone)]
pub enum Http3Frame {
    /// DATA frame
    Data { payload: Vec<u8> },
    /// HEADERS frame
    Headers { header_block: Vec<u8> },
    /// CANCEL_PUSH frame
    CancelPush { push_id: u64 },
    /// SETTINGS frame
    Settings { entries: Vec<(u64, u64)> },
    /// PUSH_PROMISE frame
    PushPromise { push_id: u64, header_block: Vec<u8> },
    /// MAX_PUSH_ID frame
    MaxPushId { push_id: u64 },
    /// GOAWAY frame
    Goaway { stream_id: u64 },
    /// STREAMS_BLOCKED frame
    StreamsBlocked { limit: u64 },
}

impl Http3Frame {
    /// Get frame type
    pub fn frame_type(&self) -> Http3FrameType {
        match self {
            Http3Frame::Data { .. } => Http3FrameType::Data,
            Http3Frame::Headers { .. } => Http3FrameType::Headers,
            Http3Frame::CancelPush { .. } => Http3FrameType::CancelPush,
            Http3Frame::Settings { .. } => Http3FrameType::Settings,
            Http3Frame::PushPromise { .. } => Http3FrameType::PushPromise,
            Http3Frame::MaxPushId { .. } => Http3FrameType::MaxPushId,
            Http3Frame::Goaway { .. } => Http3FrameType::Goaway,
            Http3Frame::StreamsBlocked { .. } => Http3FrameType::StreamsBlocked,
        }
    }

    /// Serialize frame to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = Vec::new();

        // Frame type (variable-length integer)
        Self::encode_varint(self.frame_type() as u64, &mut output);

        // Payload
        let payload = match self {
            Http3Frame::Data { payload } => payload.clone(),
            Http3Frame::Headers { header_block } => header_block.clone(),
            Http3Frame::CancelPush { push_id } => {
                let mut p = Vec::new();
                Self::encode_varint(*push_id, &mut p);
                p
            }
            Http3Frame::Settings { entries } => {
                let mut p = Vec::new();
                for &(id, value) in entries {
                    Self::encode_varint(id, &mut p);
                    Self::encode_varint(value, &mut p);
                }
                p
            }
            Http3Frame::PushPromise {
                push_id,
                header_block,
            } => {
                let mut p = Vec::new();
                Self::encode_varint(*push_id, &mut p);
                p.extend_from_slice(header_block);
                p
            }
            Http3Frame::MaxPushId { push_id } => {
                let mut p = Vec::new();
                Self::encode_varint(*push_id, &mut p);
                p
            }
            Http3Frame::Goaway { stream_id } => {
                let mut p = Vec::new();
                Self::encode_varint(*stream_id, &mut p);
                p
            }
            Http3Frame::StreamsBlocked { limit } => {
                let mut p = Vec::new();
                Self::encode_varint(*limit, &mut p);
                p
            }
        };

        // Frame length
        Self::encode_varint(payload.len() as u64, &mut output);
        output.extend_from_slice(&payload);

        output
    }

    /// Parse frame from bytes
    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), String> {
        let (frame_type, ft_len) = Self::decode_varint(data).map_err(|e| e.to_string())?;
        let (payload_len, pl_len) =
            Self::decode_varint(&data[ft_len..]).map_err(|e| e.to_string())?;
        let header_len = ft_len + pl_len;

        if header_len + payload_len as usize > data.len() {
            return Err("Frame payload incomplete".to_string());
        }

        let payload = &data[header_len..header_len + payload_len as usize];
        let total_len = header_len + payload_len as usize;

        let frame = match Http3FrameType::from_u64(frame_type) {
            Some(Http3FrameType::Data) => Http3Frame::Data {
                payload: payload.to_vec(),
            },
            Some(Http3FrameType::Headers) => Http3Frame::Headers {
                header_block: payload.to_vec(),
            },
            Some(Http3FrameType::CancelPush) => {
                let (push_id, _) = Self::decode_varint(payload).map_err(|e| e.to_string())?;
                Http3Frame::CancelPush { push_id }
            }
            Some(Http3FrameType::Settings) => {
                let mut entries = Vec::new();
                let mut pos = 0;
                while pos < payload.len() {
                    let (id, n) =
                        Self::decode_varint(&payload[pos..]).map_err(|e| e.to_string())?;
                    pos += n;
                    let (value, n) =
                        Self::decode_varint(&payload[pos..]).map_err(|e| e.to_string())?;
                    pos += n;
                    entries.push((id, value));
                }
                Http3Frame::Settings { entries }
            }
            Some(Http3FrameType::Goaway) => {
                let (stream_id, _) = Self::decode_varint(payload).map_err(|e| e.to_string())?;
                Http3Frame::Goaway { stream_id }
            }
            Some(Http3FrameType::MaxPushId) => {
                let (push_id, _) = Self::decode_varint(payload).map_err(|e| e.to_string())?;
                Http3Frame::MaxPushId { push_id }
            }
            Some(Http3FrameType::PushPromise) => {
                let (push_id, varint_len) =
                    Self::decode_varint(payload).map_err(|e| e.to_string())?;
                let header_block = payload[varint_len..].to_vec();
                Http3Frame::PushPromise {
                    push_id,
                    header_block,
                }
            }
            Some(Http3FrameType::StreamsBlocked) => {
                let (limit, _) = Self::decode_varint(payload).map_err(|e| e.to_string())?;
                Http3Frame::StreamsBlocked { limit }
            }
            Some(_) | None => {
                return Err(format!("Unknown or unsupported frame type: {}", frame_type));
            }
        };

        Ok((frame, total_len))
    }

    fn encode_varint(value: u64, output: &mut Vec<u8>) {
        edgerun_encoding::quic_varint::encode_varint(value, output)
    }

    pub(crate) fn decode_varint(data: &[u8]) -> Result<(u64, usize), std::io::Error> {
        edgerun_encoding::quic_varint::decode_varint(data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::UnexpectedEof, format!("{e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_frame_roundtrip() {
        let frame = Http3Frame::Data {
            payload: b"hello".to_vec(),
        };
        let bytes = frame.to_bytes();
        let (parsed, _) = Http3Frame::from_bytes(&bytes).unwrap();
        if let Http3Frame::Data { payload } = parsed {
            assert_eq!(payload, b"hello");
        } else {
            panic!("Expected DATA frame");
        }
    }

    #[test]
    fn test_settings_frame_roundtrip() {
        let frame = Http3Frame::Settings {
            entries: vec![
                (0x06, 4096), // MAX_TABLE_CAPACITY
            ],
        };
        let bytes = frame.to_bytes();
        let (parsed, _) = Http3Frame::from_bytes(&bytes).unwrap();
        if let Http3Frame::Settings { entries } = parsed {
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0], (0x06, 4096));
        } else {
            panic!("Expected SETTINGS frame");
        }
    }

    #[test]
    fn test_headers_frame_roundtrip() {
        let frame = Http3Frame::Headers {
            header_block: vec![0x82, 0x86, 0x84],
        };
        let bytes = frame.to_bytes();
        let (parsed, _) = Http3Frame::from_bytes(&bytes).unwrap();
        if let Http3Frame::Headers { header_block } = parsed {
            assert_eq!(header_block, vec![0x82, 0x86, 0x84]);
        } else {
            panic!("Expected HEADERS frame");
        }
    }

    #[test]
    fn test_goaway_frame_roundtrip() {
        let frame = Http3Frame::Goaway { stream_id: 7 };
        let bytes = frame.to_bytes();
        let (parsed, _) = Http3Frame::from_bytes(&bytes).unwrap();
        if let Http3Frame::Goaway { stream_id } = parsed {
            assert_eq!(stream_id, 7);
        }
    }

    #[test]
    fn test_frame_type_from_u64() {
        assert_eq!(Http3FrameType::from_u64(0), Some(Http3FrameType::Data));
        assert_eq!(Http3FrameType::from_u64(1), Some(Http3FrameType::Headers));
        assert_eq!(Http3FrameType::from_u64(4), Some(Http3FrameType::Settings));
        // 99 % 31 = 6, which matches Reserved2 pattern
        assert_eq!(Http3FrameType::from_u64(99), Some(Http3FrameType::Reserved));
        // 200 doesn't match any pattern
        assert!(Http3FrameType::from_u64(200).is_none());
    }
}
