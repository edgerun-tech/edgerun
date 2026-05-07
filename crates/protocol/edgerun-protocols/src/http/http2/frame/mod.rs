//! HTTP/2 frame protocol implementation (RFC 9113 / RFC 7540 Section 6)

mod continuation;
mod data;
mod goaway;
mod headers;
mod ping;
mod priority;
mod push_promise;
mod rst_stream;
mod settings;
mod window_update;

use alloc::format;
use alloc::string::ToString;
use alloc::vec::Vec;

pub use super::{Http2Error, Result};
pub use continuation::ContinuationFrame;
pub use data::DataFrame;
pub use goaway::GoawayFrame;
pub use headers::HeadersFrame;
pub use ping::PingFrame;
pub use priority::PriorityFrame;
pub use push_promise::PushPromiseFrame;
pub use rst_stream::RstStreamFrame;
pub use settings::SettingsFrame;
pub use window_update::WindowUpdateFrame;

/// HTTP/2 frame types (RFC 9113 / RFC 7540 Section 6)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    /// DATA frame (0x0)
    Data = 0x0,
    /// HEADERS frame (0x1)
    Headers = 0x1,
    /// PRIORITY frame (0x2)
    Priority = 0x2,
    /// RST_STREAM frame (0x3)
    RstStream = 0x3,
    /// SETTINGS frame (0x4)
    Settings = 0x4,
    /// PUSH_PROMISE frame (0x5)
    PushPromise = 0x5,
    /// PING frame (0x6)
    Ping = 0x6,
    /// GOAWAY frame (0x7)
    Goaway = 0x7,
    /// WINDOW_UPDATE frame (0x8)
    WindowUpdate = 0x8,
    /// CONTINUATION frame (0x9)
    Continuation = 0x9,
    /// Extension frame for unknown types (RFC 9113 §4.1: MUST ignore)
    Extension = 0xFF,
}

impl FrameType {
    /// Parse from u8
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x0 => Some(FrameType::Data),
            0x1 => Some(FrameType::Headers),
            0x2 => Some(FrameType::Priority),
            0x3 => Some(FrameType::RstStream),
            0x4 => Some(FrameType::Settings),
            0x5 => Some(FrameType::PushPromise),
            0x6 => Some(FrameType::Ping),
            0x7 => Some(FrameType::Goaway),
            0x8 => Some(FrameType::WindowUpdate),
            0x9 => Some(FrameType::Continuation),
            // RFC 9113 §4.1: unknown frame types MUST be ignored
            _ => Some(FrameType::Extension),
        }
    }
}

/// HTTP/2 frame flags
pub mod flags {
    // DATA flags
    /// END_STREAM flag for DATA frame
    pub const DATA_END_STREAM: u8 = 0x1;
    /// PADDED flag for DATA frame
    pub const DATA_PADDED: u8 = 0x8;

    // HEADERS flags
    /// END_STREAM flag for HEADERS frame
    pub const HEADERS_END_STREAM: u8 = 0x1;
    /// END_HEADERS flag for HEADERS frame
    pub const HEADERS_END_HEADERS: u8 = 0x4;
    /// PADDED flag for HEADERS frame
    pub const HEADERS_PADDED: u8 = 0x8;
    /// PRIORITY flag for HEADERS frame
    pub const HEADERS_PRIORITY: u8 = 0x20;

    // SETTINGS flags
    /// ACK flag for SETTINGS frame
    pub const SETTINGS_ACK: u8 = 0x1;

    // PUSH_PROMISE flags
    /// END_HEADERS flag for PUSH_PROMISE frame
    pub const PUSH_PROMISE_END_HEADERS: u8 = 0x4;
    /// PADDED flag for PUSH_PROMISE frame
    pub const PUSH_PROMISE_PADDED: u8 = 0x8;

    // PING flags
    /// ACK flag for PING frame
    pub const PING_ACK: u8 = 0x1;

    // CONTINUATION flags
    /// END_HEADERS flag for CONTINUATION frame
    pub const CONTINUATION_END_HEADERS: u8 = 0x4;
}

/// HTTP/2 frame
#[derive(Debug, Clone)]
pub struct Frame {
    /// Frame type
    pub frame_type: FrameType,
    /// Frame flags
    pub flags: u8,
    /// Stream identifier (0 for connection-level frames)
    pub stream_id: u32,
    /// Frame payload
    pub payload: Vec<u8>,
}

impl Frame {
    /// The maximum default frame size (16,384 bytes)
    pub const DEFAULT_MAX_FRAME_SIZE: u32 = 16384;

    /// Create a new frame
    pub fn new(frame_type: FrameType, flags: u8, stream_id: u32, payload: Vec<u8>) -> Self {
        Frame {
            frame_type,
            flags,
            stream_id: stream_id & 0x7FFFFFFF, // Clear reserved bit
            payload,
        }
    }

    /// Serialize frame to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let length = self.payload.len() as u32;
        let mut bytes = Vec::with_capacity(9 + self.payload.len());

        // Length (3 bytes)
        bytes.push((length >> 16) as u8);
        bytes.push((length >> 8) as u8);
        bytes.push(length as u8);

        // Type (1 byte)
        bytes.push(self.frame_type as u8);

        // Flags (1 byte)
        bytes.push(self.flags);

        // Stream ID (4 bytes, with reserved bit cleared)
        let sid = self.stream_id & 0x7FFFFFFF;
        bytes.push((sid >> 24) as u8);
        bytes.push((sid >> 16) as u8);
        bytes.push((sid >> 8) as u8);
        bytes.push(sid as u8);

        // Payload
        bytes.extend_from_slice(&self.payload);

        bytes
    }

    /// Parse frame from bytes
    ///
    /// Note: frame size validation uses `max_frame_size` parameter (from
    /// negotiated `SETTINGS_MAX_FRAME_SIZE`), not the hardcoded default.
    /// Callers should pass their current negotiated max frame size.
    pub fn from_bytes(data: &[u8], max_frame_size: u32) -> Result<(Self, usize)> {
        if data.len() < 9 {
            return Err(Http2Error::FrameParse("Frame header too short".to_string()));
        }

        // Length (3 bytes)
        let length = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | (data[2] as u32);

        // Type (1 byte)
        let frame_type = FrameType::from_u8(data[3])
            .ok_or_else(|| Http2Error::FrameParse(format!("Unknown frame type: {}", data[3])))?;

        // Flags (1 byte)
        let flags = data[4];

        // Stream ID (4 bytes)
        let stream_id = ((data[5] as u32) << 24)
            | ((data[6] as u32) << 16)
            | ((data[7] as u32) << 8)
            | (data[8] as u32);

        // Validate frame size against negotiated limit
        if length > max_frame_size {
            return Err(Http2Error::FrameParse(format!(
                "Frame size {} exceeds maximum {}",
                length, max_frame_size
            )));
        }

        // Check payload availability
        let total_len = 9 + length as usize;
        if data.len() < total_len {
            return Err(Http2Error::FrameParse(
                "Frame payload incomplete".to_string(),
            ));
        }

        // Extract payload
        let payload = data[9..total_len].to_vec();

        Ok((
            Frame {
                frame_type,
                flags,
                stream_id,
                payload,
            },
            total_len,
        ))
    }

    /// Get payload as u32 (for RST_STREAM, WINDOW_UPDATE, etc.)
    pub fn payload_u32(&self) -> Result<u32> {
        if self.payload.len() < 4 {
            return Err(Http2Error::FrameParse(
                "Payload too short for u32".to_string(),
            ));
        }
        Ok(((self.payload[0] as u32) << 24)
            | ((self.payload[1] as u32) << 16)
            | ((self.payload[2] as u32) << 8)
            | (self.payload[3] as u32))
    }

    /// Get payload as u64 (for PING)
    pub fn payload_u64(&self) -> Result<u64> {
        if self.payload.len() < 8 {
            return Err(Http2Error::FrameParse(
                "Payload too short for u64".to_string(),
            ));
        }
        Ok(((self.payload[0] as u64) << 56)
            | ((self.payload[1] as u64) << 48)
            | ((self.payload[2] as u64) << 40)
            | ((self.payload[3] as u64) << 32)
            | ((self.payload[4] as u64) << 24)
            | ((self.payload[5] as u64) << 16)
            | ((self.payload[6] as u64) << 8)
            | (self.payload[7] as u64))
    }

    /// Validate frame semantics per RFC 9113 (RFC 7540 Section 6).
    /// Returns the connection error code if a protocol violation is found.
    pub fn validate_semantics(&self) -> core::result::Result<(), u32> {
        use super::ErrorCode;

        match self.frame_type {
            FrameType::Data => {
                // DATA frames MUST NOT be sent on stream 0 (RFC 9113 Sec6.1)
                if self.stream_id == 0 {
                    return Err(ErrorCode::ProtocolError.to_u32());
                }
            }
            FrameType::Headers => {
                // HEADERS frames MUST NOT be sent on stream 0 (RFC 9113 Sec6.2)
                if self.stream_id == 0 {
                    return Err(ErrorCode::ProtocolError.to_u32());
                }
            }
            FrameType::Priority => {
                // PRIORITY frames can be sent on most streams, but NOT stream 0 (RFC 7540 Sec6.3)
                if self.stream_id == 0 {
                    return Err(ErrorCode::ProtocolError.to_u32());
                }
                // Payload MUST be exactly 5 octets
                if self.payload.len() != 5 {
                    return Err(ErrorCode::FrameSizeError.to_u32());
                }
            }
            FrameType::RstStream => {
                // RST_STREAM frames MUST NOT be sent on stream 0
                // Payload MUST be exactly 4 octets
                if self.stream_id == 0 {
                    return Err(ErrorCode::ProtocolError.to_u32());
                }
                if self.payload.len() != 4 {
                    return Err(ErrorCode::FrameSizeError.to_u32());
                }
            }
            FrameType::Settings => {
                // SETTINGS frames MUST be on stream 0
                // ACK SETTINGS payload MUST be 0 octets
                // Non-ACK SETTINGS payload MUST be multiple of 6
                let is_ack = (self.flags & 0x1) != 0;
                if self.stream_id != 0 {
                    return Err(ErrorCode::ProtocolError.to_u32());
                }
                if is_ack && !self.payload.is_empty() {
                    return Err(ErrorCode::FrameSizeError.to_u32());
                }
                if !is_ack && !self.payload.len().is_multiple_of(6) {
                    return Err(ErrorCode::FrameSizeError.to_u32());
                }
            }
            FrameType::PushPromise => {
                // PUSH_PROMISE frames MUST NOT be on stream 0
                // Promised stream ID MUST NOT be 0 or odd
                if self.stream_id == 0 {
                    return Err(ErrorCode::ProtocolError.to_u32());
                }
                if self.payload.len() >= 4 {
                    let promised = u32::from_be_bytes([
                        self.payload[0],
                        self.payload[1],
                        self.payload[2],
                        self.payload[3],
                    ]) & 0x7FFFFFFF;
                    if promised == 0 || promised.is_multiple_of(2) {
                        return Err(ErrorCode::ProtocolError.to_u32());
                    }
                }
            }
            FrameType::Ping => {
                // PING frames MUST be on stream 0, payload MUST be 8 octets
                if self.stream_id != 0 {
                    return Err(ErrorCode::ProtocolError.to_u32());
                }
                if self.payload.len() != 8 {
                    return Err(ErrorCode::FrameSizeError.to_u32());
                }
            }
            FrameType::Goaway => {
                // GOAWAY frames MUST be on stream 0
                // Payload MUST be >= 8 octets
                if self.stream_id != 0 {
                    return Err(ErrorCode::ProtocolError.to_u32());
                }
                if self.payload.len() < 8 {
                    return Err(ErrorCode::FrameSizeError.to_u32());
                }
            }
            FrameType::WindowUpdate => {
                // WINDOW_UPDATE payload MUST be exactly 4 octets (RFC 9113 Sec6.9)
                if self.payload.len() != 4 {
                    return Err(ErrorCode::FrameSizeError.to_u32());
                }
                // Window size increment MUST NOT be 0
                let inc = u32::from_be_bytes([
                    self.payload[0],
                    self.payload[1],
                    self.payload[2],
                    self.payload[3],
                ]) & 0x7FFFFFFF;
                if inc == 0 {
                    return Err(ErrorCode::ProtocolError.to_u32());
                }
            }
            FrameType::Continuation => {
                // CONTINUATION frames MUST NOT be on stream 0
                if self.stream_id == 0 {
                    return Err(ErrorCode::ProtocolError.to_u32());
                }
            }
            // RFC 9113 §4.1: unknown frame types MUST be ignored - no validation needed
            FrameType::Extension => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn test_frame_type_from_u8() {
        assert_eq!(FrameType::from_u8(0), Some(FrameType::Data));
        assert_eq!(FrameType::from_u8(1), Some(FrameType::Headers));
        assert_eq!(FrameType::from_u8(8), Some(FrameType::WindowUpdate));
        // RFC 9113 §4.1: unknown frame types are treated as Extension
        assert_eq!(FrameType::from_u8(10), Some(FrameType::Extension));
    }

    #[test]
    fn test_frame_to_bytes_roundtrip() {
        let frame = Frame::new(FrameType::Data, 0x1, 1, b"hello".to_vec());
        let bytes = frame.to_bytes();

        // Header (9 bytes) + payload (5 bytes) = 14
        assert_eq!(bytes.len(), 14);

        let (parsed, _) = Frame::from_bytes(&bytes, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
        assert_eq!(parsed.frame_type, FrameType::Data);
        assert_eq!(parsed.flags, 0x1);
        assert_eq!(parsed.stream_id, 1);
        assert_eq!(parsed.payload, b"hello");
    }

    #[test]
    fn test_frame_from_bytes_too_short() {
        assert!(Frame::from_bytes(&[0, 0], Frame::DEFAULT_MAX_FRAME_SIZE).is_err());
    }

    #[test]
    fn test_data_frame_roundtrip() {
        let data_frame = DataFrame::new(1, b"test data".to_vec(), true);
        let frame = data_frame.to_frame();
        let parsed = DataFrame::from_frame(&frame).unwrap();

        assert_eq!(parsed.stream_id, 1);
        assert!(parsed.end_stream);
        assert_eq!(parsed.data, b"test data");
    }

    #[test]
    fn test_headers_frame_roundtrip() {
        let headers_frame = HeadersFrame::new(1, vec![0x82, 0x86, 0x84], false);
        let frame = headers_frame.to_frame();
        let parsed = HeadersFrame::from_frame(&frame).unwrap();

        assert_eq!(parsed.stream_id, 1);
        assert!(!parsed.end_stream);
        assert_eq!(parsed.header_block, vec![0x82, 0x86, 0x84]);
    }

    #[test]
    fn test_settings_frame_roundtrip() {
        let settings = SettingsFrame::new(vec![
            (0x1, 4096),  // HEADER_TABLE_SIZE
            (0x3, 100),   // MAX_CONCURRENT_STREAMS
            (0x4, 65535), // INITIAL_WINDOW_SIZE
        ]);
        let frame = settings.to_frame();
        let parsed = SettingsFrame::from_frame(&frame).unwrap();

        assert!(!parsed.ack);
        assert_eq!(parsed.entries.len(), 3);
        assert_eq!(parsed.entries[0], (0x1, 4096));
    }

    #[test]
    fn test_settings_frame_ack_roundtrip() {
        let ack = SettingsFrame::ack();
        let frame = ack.to_frame();
        let parsed = SettingsFrame::from_frame(&frame).unwrap();

        assert!(parsed.ack);
        assert!(parsed.entries.is_empty());
    }

    #[test]
    fn test_ping_frame_roundtrip() {
        let ping = PingFrame::new([1, 2, 3, 4, 5, 6, 7, 8]);
        let frame = ping.to_frame();
        let parsed = PingFrame::from_frame(&frame).unwrap();

        assert!(!parsed.ack);
        assert_eq!(parsed.data, [1, 2, 3, 4, 5, 6, 7, 8]);
    }

    #[test]
    fn test_goaway_frame_roundtrip() {
        let goaway = GoawayFrame::new(5, 0, b"debug".to_vec());
        let frame = goaway.to_frame();
        let parsed = GoawayFrame::from_frame(&frame).unwrap();

        assert_eq!(parsed.last_stream_id, 5);
        assert_eq!(parsed.error_code, 0);
        assert_eq!(parsed.debug_data, b"debug");
    }

    #[test]
    fn test_window_update_frame_roundtrip() {
        let wu = WindowUpdateFrame::new(1, 1024);
        let frame = wu.to_frame();
        let parsed = WindowUpdateFrame::from_frame(&frame).unwrap();

        assert_eq!(parsed.stream_id, 1);
        assert_eq!(parsed.window_increment, 1024);
    }

    #[test]
    fn test_rst_stream_frame_roundtrip() {
        let rst = RstStreamFrame::new(3, 0x1);
        let frame = rst.to_frame();
        let parsed = RstStreamFrame::from_frame(&frame).unwrap();

        assert_eq!(parsed.stream_id, 3);
        assert_eq!(parsed.error_code, 0x1);
    }
}
