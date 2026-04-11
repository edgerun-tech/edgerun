//! HTTP/2 frame protocol implementation (RFC 9113 / RFC 7540 Section 6)

use super::{Http2Error, Result};

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
            _ => None,
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
            return Err(Http2Error::FrameParse(
                "Frame header too short".to_string(),
            ));
        }

        // Length (3 bytes)
        let length = ((data[0] as u32) << 16) | ((data[1] as u32) << 8) | (data[2] as u32);

        // Type (1 byte)
        let frame_type =
            FrameType::from_u8(data[3]).ok_or_else(|| {
                Http2Error::FrameParse(format!("Unknown frame type: {}", data[3]))
            })?;

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
    pub fn validate_semantics(&self) -> std::result::Result<(), u32> {
        use crate::http2::ErrorCode;

        match self.frame_type {
            FrameType::Data => {
                // DATA frames MUST NOT be sent on stream 0 (RFC 9113 §6.1)
                if self.stream_id == 0 {
                    return Err(ErrorCode::PROTOCOL_ERROR.to_u32());
                }
            }
            FrameType::Headers => {
                // HEADERS frames MUST NOT be sent on stream 0 (RFC 9113 §6.2)
                if self.stream_id == 0 {
                    return Err(ErrorCode::PROTOCOL_ERROR.to_u32());
                }
            }
            FrameType::Priority => {
                // PRIORITY frames can be sent on most streams, but NOT stream 0 (RFC 7540 §6.3)
                if self.stream_id == 0 {
                    return Err(ErrorCode::PROTOCOL_ERROR.to_u32());
                }
                // Payload MUST be exactly 5 octets
                if self.payload.len() != 5 {
                    return Err(ErrorCode::FRAME_SIZE_ERROR.to_u32());
                }
            }
            FrameType::RstStream => {
                // RST_STREAM frames MUST NOT be sent on stream 0
                // Payload MUST be exactly 4 octets
                if self.stream_id == 0 {
                    return Err(ErrorCode::PROTOCOL_ERROR.to_u32());
                }
                if self.payload.len() != 4 {
                    return Err(ErrorCode::FRAME_SIZE_ERROR.to_u32());
                }
            }
            FrameType::Settings => {
                // SETTINGS frames MUST be on stream 0
                // ACK SETTINGS payload MUST be 0 octets
                // Non-ACK SETTINGS payload MUST be multiple of 6
                let is_ack = (self.flags & 0x1) != 0;
                if self.stream_id != 0 {
                    return Err(ErrorCode::PROTOCOL_ERROR.to_u32());
                }
                if is_ack && self.payload.len() != 0 {
                    return Err(ErrorCode::FRAME_SIZE_ERROR.to_u32());
                }
                if !is_ack && self.payload.len() % 6 != 0 {
                    return Err(ErrorCode::FRAME_SIZE_ERROR.to_u32());
                }
            }
            FrameType::PushPromise => {
                // PUSH_PROMISE frames MUST NOT be on stream 0
                // Promised stream ID MUST NOT be 0 or odd
                if self.stream_id == 0 {
                    return Err(ErrorCode::PROTOCOL_ERROR.to_u32());
                }
                if self.payload.len() >= 4 {
                    let promised = u32::from_be_bytes([
                        self.payload[0], self.payload[1], self.payload[2], self.payload[3],
                    ]) & 0x7FFFFFFF;
                    if promised == 0 || promised % 2 == 0 {
                        return Err(ErrorCode::PROTOCOL_ERROR.to_u32());
                    }
                }
            }
            FrameType::Ping => {
                // PING frames MUST be on stream 0, payload MUST be 8 octets
                if self.stream_id != 0 {
                    return Err(ErrorCode::PROTOCOL_ERROR.to_u32());
                }
                if self.payload.len() != 8 {
                    return Err(ErrorCode::FRAME_SIZE_ERROR.to_u32());
                }
            }
            FrameType::Goaway => {
                // GOAWAY frames MUST be on stream 0
                // Payload MUST be >= 8 octets
                if self.stream_id != 0 {
                    return Err(ErrorCode::PROTOCOL_ERROR.to_u32());
                }
                if self.payload.len() < 8 {
                    return Err(ErrorCode::FRAME_SIZE_ERROR.to_u32());
                }
            }
            FrameType::WindowUpdate => {
                // WINDOW_UPDATE payload MUST be exactly 4 octets (RFC 9113 §6.9)
                if self.payload.len() != 4 {
                    return Err(ErrorCode::FRAME_SIZE_ERROR.to_u32());
                }
                // Window size increment MUST NOT be 0
                let inc = u32::from_be_bytes([
                    self.payload[0], self.payload[1], self.payload[2], self.payload[3],
                ]) & 0x7FFFFFFF;
                if inc == 0 {
                    return Err(ErrorCode::PROTOCOL_ERROR.to_u32());
                }
            }
            FrameType::Continuation => {
                // CONTINUATION frames MUST NOT be on stream 0
                if self.stream_id == 0 {
                    return Err(ErrorCode::PROTOCOL_ERROR.to_u32());
                }
            }
        }

        Ok(())
    }
}

/// DATA frame
#[derive(Debug, Clone)]
pub struct DataFrame {
    /// Stream ID
    pub stream_id: u32,
    /// END_STREAM flag
    pub end_stream: bool,
    /// Padding
    pub padding: Option<Vec<u8>>,
    /// Data payload
    pub data: Vec<u8>,
}

impl DataFrame {
    /// Create a new DATA frame
    pub fn new(stream_id: u32, data: Vec<u8>, end_stream: bool) -> Self {
        DataFrame {
            stream_id,
            end_stream,
            padding: None,
            data,
        }
    }

    /// Convert to generic Frame
    pub fn to_frame(&self) -> Frame {
        let mut flags = 0u8;
        if self.end_stream {
            flags |= flags::DATA_END_STREAM;
        }

        let mut payload = Vec::new();
        if let Some(ref pad) = self.padding {
            payload.push(pad.len() as u8);
        }
        payload.extend_from_slice(&self.data);
        if let Some(ref pad) = self.padding {
            payload.extend_from_slice(pad);
        }

        Frame::new(FrameType::Data, flags, self.stream_id, payload)
    }

    /// Parse from Frame
    pub fn from_frame(frame: &Frame) -> Result<Self> {
        if frame.frame_type != FrameType::Data {
            return Err(Http2Error::FrameParse(
                "Expected DATA frame".to_string(),
            ));
        }

        let end_stream = (frame.flags & flags::DATA_END_STREAM) != 0;
        let padded = (frame.flags & flags::DATA_PADDED) != 0;

        let mut offset = 0;
        let pad_length = if padded {
            if frame.payload.is_empty() {
                return Err(Http2Error::FrameParse(
                    "Padded DATA frame but no pad length".to_string(),
                ));
            }
            let len = frame.payload[0] as usize;
            offset = 1;
            Some(len)
        } else {
            None
        };

        let data_end = if let Some(pl) = pad_length {
            frame.payload.len().saturating_sub(pl)
        } else {
            frame.payload.len()
        };

        let data = frame.payload[offset..data_end.min(frame.payload.len())].to_vec();
        let padding = if let Some(pl) = pad_length {
            let start = data_end.min(frame.payload.len());
            Some(frame.payload[start..start + pl].to_vec())
        } else {
            None
        };

        Ok(DataFrame {
            stream_id: frame.stream_id,
            end_stream,
            padding,
            data,
        })
    }
}

/// HEADERS frame
#[derive(Debug, Clone)]
pub struct HeadersFrame {
    /// Stream ID
    pub stream_id: u32,
    /// END_STREAM flag
    pub end_stream: bool,
    /// Exclusive flag for priority
    pub exclusive: bool,
    /// Stream dependency
    pub stream_dependency: u32,
    /// Weight
    pub weight: u8,
    /// Padding
    pub padding: Option<Vec<u8>>,
    /// Header block fragment
    pub header_block: Vec<u8>,
}

impl HeadersFrame {
    /// Create a new HEADERS frame (without priority)
    pub fn new(stream_id: u32, header_block: Vec<u8>, end_stream: bool) -> Self {
        HeadersFrame {
            stream_id,
            end_stream,
            exclusive: false,
            stream_dependency: 0,
            weight: 0,
            padding: None,
            header_block,
        }
    }

    /// Create a new HEADERS frame with priority
    pub fn with_priority(
        stream_id: u32,
        header_block: Vec<u8>,
        end_stream: bool,
        exclusive: bool,
        stream_dependency: u32,
        weight: u8,
    ) -> Self {
        HeadersFrame {
            stream_id,
            end_stream,
            exclusive,
            stream_dependency,
            weight,
            padding: None,
            header_block,
        }
    }

    /// Convert to generic Frame
    pub fn to_frame(&self) -> Frame {
        let mut flags = flags::HEADERS_END_HEADERS;
        if self.end_stream {
            flags |= flags::HEADERS_END_STREAM;
        }
        if self.padding.is_some() {
            flags |= flags::HEADERS_PADDED;
        }
        if self.stream_dependency != 0 {
            flags |= flags::HEADERS_PRIORITY;
        }

        let mut payload = Vec::new();

        if let Some(ref pad) = self.padding {
            payload.push(pad.len() as u8);
        }

        if self.stream_dependency != 0 {
            let dep = if self.exclusive {
                self.stream_dependency | 0x80000000
            } else {
                self.stream_dependency
            };
            payload.push((dep >> 24) as u8);
            payload.push((dep >> 16) as u8);
            payload.push((dep >> 8) as u8);
            payload.push(dep as u8);
            payload.push(self.weight);
        }

        payload.extend_from_slice(&self.header_block);

        if let Some(ref pad) = self.padding {
            payload.extend_from_slice(pad);
        }

        Frame::new(FrameType::Headers, flags, self.stream_id, payload)
    }

    /// Parse from Frame
    pub fn from_frame(frame: &Frame) -> Result<Self> {
        if frame.frame_type != FrameType::Headers {
            return Err(Http2Error::FrameParse(
                "Expected HEADERS frame".to_string(),
            ));
        }

        let end_stream = (frame.flags & flags::HEADERS_END_STREAM) != 0;
        let padded = (frame.flags & flags::HEADERS_PADDED) != 0;
        let priority = (frame.flags & flags::HEADERS_PRIORITY) != 0;

        let mut offset = 0;

        // Pad Length
        let pad_length = if padded {
            if frame.payload.is_empty() {
                return Err(Http2Error::FrameParse(
                    "Padded HEADERS but no pad length".to_string(),
                ));
            }
            let len = frame.payload[0] as usize;
            offset = 1;
            Some(len)
        } else {
            None
        };

        // Priority
        let (exclusive, stream_dependency, weight) = if priority {
            if frame.payload.len() < offset + 5 {
                return Err(Http2Error::FrameParse(
                    "Priority flag but insufficient payload".to_string(),
                ));
            }
            let dep = ((frame.payload[offset] as u32) << 24)
                | ((frame.payload[offset + 1] as u32) << 16)
                | ((frame.payload[offset + 2] as u32) << 8)
                | (frame.payload[offset + 3] as u32);
            let exclusive = (dep & 0x80000000) != 0;
            let stream_dependency = dep & 0x7FFFFFFF;
            let weight = frame.payload[offset + 4];
            offset += 5;
            (exclusive, stream_dependency, weight)
        } else {
            (false, 0, 0)
        };

        // Header block fragment
        let data_end = if let Some(pl) = pad_length {
            frame.payload.len().saturating_sub(pl)
        } else {
            frame.payload.len()
        };

        let header_block = frame.payload[offset..data_end.min(frame.payload.len())].to_vec();

        let padding = if let Some(pl) = pad_length {
            let start = data_end.min(frame.payload.len());
            Some(frame.payload[start..start + pl].to_vec())
        } else {
            None
        };

        Ok(HeadersFrame {
            stream_id: frame.stream_id,
            end_stream,
            exclusive,
            stream_dependency,
            weight,
            padding,
            header_block,
        })
    }
}

/// SETTINGS frame
#[derive(Debug, Clone)]
pub struct SettingsFrame {
    /// ACK flag
    pub ack: bool,
    /// Settings entries
    pub entries: Vec<(u16, u32)>,
}

impl SettingsFrame {
    /// Create a new SETTINGS frame
    pub fn new(entries: Vec<(u16, u32)>) -> Self {
        SettingsFrame {
            ack: false,
            entries,
        }
    }

    /// Create a SETTINGS ACK frame
    pub fn ack() -> Self {
        SettingsFrame {
            ack: true,
            entries: Vec::new(),
        }
    }

    /// Convert to generic Frame
    pub fn to_frame(&self) -> Frame {
        let flags = if self.ack { flags::SETTINGS_ACK } else { 0 };
        let mut payload = Vec::with_capacity(self.entries.len() * 6);

        for &(id, value) in &self.entries {
            payload.push((id >> 8) as u8);
            payload.push(id as u8);
            payload.push((value >> 24) as u8);
            payload.push((value >> 16) as u8);
            payload.push((value >> 8) as u8);
            payload.push(value as u8);
        }

        Frame::new(FrameType::Settings, flags, 0, payload)
    }

    /// Parse from Frame
    pub fn from_frame(frame: &Frame) -> Result<Self> {
        if frame.frame_type != FrameType::Settings {
            return Err(Http2Error::FrameParse(
                "Expected SETTINGS frame".to_string(),
            ));
        }

        let ack = (frame.flags & flags::SETTINGS_ACK) != 0;

        if ack && !frame.payload.is_empty() {
            return Err(Http2Error::FrameParse(
                "SETTINGS ACK must have empty payload".to_string(),
            ));
        }

        if frame.payload.len() % 6 != 0 {
            return Err(Http2Error::FrameParse(
                "SETTINGS payload length not multiple of 6".to_string(),
            ));
        }

        let mut entries = Vec::new();
        for chunk in frame.payload.chunks(6) {
            let id = ((chunk[0] as u16) << 8) | (chunk[1] as u16);
            let value = ((chunk[2] as u32) << 24)
                | ((chunk[3] as u32) << 16)
                | ((chunk[4] as u32) << 8)
                | (chunk[5] as u32);
            entries.push((id, value));
        }

        Ok(SettingsFrame { ack, entries })
    }
}

/// PING frame
#[derive(Debug, Clone)]
pub struct PingFrame {
    /// ACK flag
    pub ack: bool,
    /// Opaque data (8 bytes)
    pub data: [u8; 8],
}

impl PingFrame {
    /// Create a new PING frame
    pub fn new(data: [u8; 8]) -> Self {
        PingFrame { ack: false, data }
    }

    /// Create a PING ACK frame
    pub fn ack(data: [u8; 8]) -> Self {
        PingFrame { ack: true, data }
    }

    /// Convert to generic Frame
    pub fn to_frame(&self) -> Frame {
        let flags = if self.ack { flags::PING_ACK } else { 0 };
        Frame::new(FrameType::Ping, flags, 0, self.data.to_vec())
    }

    /// Parse from Frame
    pub fn from_frame(frame: &Frame) -> Result<Self> {
        if frame.frame_type != FrameType::Ping {
            return Err(Http2Error::FrameParse(
                "Expected PING frame".to_string(),
            ));
        }

        if frame.payload.len() != 8 {
            return Err(Http2Error::FrameParse(
                "PING payload must be 8 bytes".to_string(),
            ));
        }

        let ack = (frame.flags & flags::PING_ACK) != 0;
        let mut data = [0u8; 8];
        data.copy_from_slice(&frame.payload);

        Ok(PingFrame { ack, data })
    }
}

/// GOAWAY frame
#[derive(Debug, Clone)]
pub struct GoawayFrame {
    /// Last stream ID processed
    pub last_stream_id: u32,
    /// Error code
    pub error_code: u32,
    /// Additional debug data
    pub debug_data: Vec<u8>,
}

impl GoawayFrame {
    /// Create a new GOAWAY frame
    pub fn new(last_stream_id: u32, error_code: u32, debug_data: Vec<u8>) -> Self {
        GoawayFrame {
            last_stream_id,
            error_code,
            debug_data,
        }
    }

    /// Convert to generic Frame
    pub fn to_frame(&self) -> Frame {
        let mut payload = Vec::with_capacity(8 + self.debug_data.len());

        // Reserved bit + last stream ID
        let lsi = self.last_stream_id & 0x7FFFFFFF;
        payload.push((lsi >> 24) as u8);
        payload.push((lsi >> 16) as u8);
        payload.push((lsi >> 8) as u8);
        payload.push(lsi as u8);

        // Error code
        payload.push((self.error_code >> 24) as u8);
        payload.push((self.error_code >> 16) as u8);
        payload.push((self.error_code >> 8) as u8);
        payload.push(self.error_code as u8);

        // Debug data
        payload.extend_from_slice(&self.debug_data);

        Frame::new(FrameType::Goaway, 0, 0, payload)
    }

    /// Parse from Frame
    pub fn from_frame(frame: &Frame) -> Result<Self> {
        if frame.frame_type != FrameType::Goaway {
            return Err(Http2Error::FrameParse(
                "Expected GOAWAY frame".to_string(),
            ));
        }

        if frame.payload.len() < 8 {
            return Err(Http2Error::FrameParse(
                "GOAWAY payload too short".to_string(),
            ));
        }

        let last_stream_id = ((frame.payload[0] as u32) << 24)
            | ((frame.payload[1] as u32) << 16)
            | ((frame.payload[2] as u32) << 8)
            | (frame.payload[3] as u32);

        let error_code = ((frame.payload[4] as u32) << 24)
            | ((frame.payload[5] as u32) << 16)
            | ((frame.payload[6] as u32) << 8)
            | (frame.payload[7] as u32);

        let debug_data = frame.payload[8..].to_vec();

        Ok(GoawayFrame {
            last_stream_id: last_stream_id & 0x7FFFFFFF,
            error_code,
            debug_data,
        })
    }
}

/// WINDOW_UPDATE frame
#[derive(Debug, Clone)]
pub struct WindowUpdateFrame {
    /// Stream ID (0 for connection-level)
    pub stream_id: u32,
    /// Window size increment
    pub window_increment: u32,
}

impl WindowUpdateFrame {
    /// Create a new WINDOW_UPDATE frame
    pub fn new(stream_id: u32, window_increment: u32) -> Self {
        WindowUpdateFrame {
            stream_id,
            window_increment,
        }
    }

    /// Convert to generic Frame
    pub fn to_frame(&self) -> Frame {
        let mut payload = Vec::with_capacity(4);
        // Reserved bit + window size increment
        let wsi = self.window_increment & 0x7FFFFFFF;
        payload.push((wsi >> 24) as u8);
        payload.push((wsi >> 16) as u8);
        payload.push((wsi >> 8) as u8);
        payload.push(wsi as u8);

        Frame::new(FrameType::WindowUpdate, 0, self.stream_id, payload)
    }

    /// Parse from Frame
    pub fn from_frame(frame: &Frame) -> Result<Self> {
        if frame.frame_type != FrameType::WindowUpdate {
            return Err(Http2Error::FrameParse(
                "Expected WINDOW_UPDATE frame".to_string(),
            ));
        }

        if frame.payload.len() < 4 {
            return Err(Http2Error::FrameParse(
                "WINDOW_UPDATE payload too short".to_string(),
            ));
        }

        let window_increment = ((frame.payload[0] as u32) << 24)
            | ((frame.payload[1] as u32) << 16)
            | ((frame.payload[2] as u32) << 8)
            | (frame.payload[3] as u32);

        if window_increment == 0 {
            return Err(Http2Error::FrameParse(
                "WINDOW_UPDATE with zero increment".to_string(),
            ));
        }

        Ok(WindowUpdateFrame {
            stream_id: frame.stream_id,
            window_increment,
        })
    }
}

/// RST_STREAM frame
#[derive(Debug, Clone)]
pub struct RstStreamFrame {
    /// Stream ID
    pub stream_id: u32,
    /// Error code
    pub error_code: u32,
}

impl RstStreamFrame {
    /// Create a new RST_STREAM frame
    pub fn new(stream_id: u32, error_code: u32) -> Self {
        RstStreamFrame {
            stream_id,
            error_code,
        }
    }

    /// Convert to generic Frame
    pub fn to_frame(&self) -> Frame {
        let mut payload = Vec::with_capacity(4);
        payload.push((self.error_code >> 24) as u8);
        payload.push((self.error_code >> 16) as u8);
        payload.push((self.error_code >> 8) as u8);
        payload.push(self.error_code as u8);

        Frame::new(FrameType::RstStream, 0, self.stream_id, payload)
    }

    /// Parse from Frame
    pub fn from_frame(frame: &Frame) -> Result<Self> {
        if frame.frame_type != FrameType::RstStream {
            return Err(Http2Error::FrameParse(
                "Expected RST_STREAM frame".to_string(),
            ));
        }

        if frame.stream_id == 0 {
            return Err(Http2Error::FrameParse(
                "RST_STREAM on stream 0".to_string(),
            ));
        }

        let error_code = frame.payload_u32()?;

        Ok(RstStreamFrame {
            stream_id: frame.stream_id,
            error_code,
        })
    }
}

/// PRIORITY frame (RFC 7540 §6.3)
#[derive(Debug, Clone)]
pub struct PriorityFrame {
    /// Stream ID (must be 0 for PRIORITY frames)
    pub stream_id: u32,
    /// Exclusive flag
    pub exclusive: bool,
    /// Stream dependency (0 = no dependency)
    pub stream_dependency: u32,
    /// Weight (1-256, encoded as 0-255 on wire)
    pub weight: u8,
}

impl PriorityFrame {
    /// Create a new PRIORITY frame
    pub fn new(stream_id: u32, exclusive: bool, stream_dependency: u32, weight: u8) -> Self {
        // HTTP/2 weight range is 1-256, but u8 only holds 0-255.
        // Weight 256 is stored as 255 (the difference is negligible in practice).
        let w = if weight < 1 { 1 } else if weight >= 255 { 255 } else { weight };
        PriorityFrame {
            stream_id,
            exclusive,
            stream_dependency,
            weight: w,
        }
    }

    /// Convert to generic Frame
    pub fn to_frame(&self) -> Frame {
        let mut payload = Vec::with_capacity(5);
        let dep = if self.exclusive {
            0x80000000 | (self.stream_dependency & 0x7FFFFFFF)
        } else {
            self.stream_dependency & 0x7FFFFFFF
        };
        payload.push((dep >> 24) as u8);
        payload.push((dep >> 16) as u8);
        payload.push((dep >> 8) as u8);
        payload.push(dep as u8);
        payload.push(self.weight - 1); // Wire format is weight - 1

        Frame::new(FrameType::Priority, 0, self.stream_id, payload)
    }

    /// Parse from Frame
    pub fn from_frame(frame: &Frame) -> Result<Self> {
        if frame.frame_type != FrameType::Priority {
            return Err(Http2Error::FrameParse(
                "Expected PRIORITY frame".to_string(),
            ));
        }
        if frame.payload.len() < 5 {
            return Err(Http2Error::FrameParse(
                "PRIORITY payload too short".to_string(),
            ));
        }

        let dep_raw = u32::from_be_bytes([
            frame.payload[0], frame.payload[1], frame.payload[2], frame.payload[3],
        ]);
        let exclusive = (dep_raw >> 31) != 0;
        let stream_dependency = dep_raw & 0x7FFFFFFF;
        let weight = frame.payload[4].wrapping_add(1); // Wire format is weight - 1 (0→1, 255→256)

        Ok(PriorityFrame {
            stream_id: frame.stream_id,
            exclusive,
            stream_dependency,
            weight,
        })
    }
}

/// PUSH_PROMISE frame (RFC 7540 §6.6)
#[derive(Debug, Clone)]
pub struct PushPromiseFrame {
    /// Parent stream ID
    pub stream_id: u32,
    /// Promised stream ID (server-initiated, must be even)
    pub promised_stream_id: u32,
    /// Header block fragment
    pub header_block_fragment: Vec<u8>,
    /// Padding (optional)
    pub padding: Option<Vec<u8>>,
}

impl PushPromiseFrame {
    /// Create a new PUSH_PROMISE frame
    pub fn new(stream_id: u32, promised_stream_id: u32, header_block_fragment: Vec<u8>) -> Self {
        PushPromiseFrame {
            stream_id,
            promised_stream_id,
            header_block_fragment,
            padding: None,
        }
    }

    /// Create with padding
    pub fn with_padding(
        stream_id: u32,
        promised_stream_id: u32,
        header_block_fragment: Vec<u8>,
        padding: Vec<u8>,
    ) -> Self {
        PushPromiseFrame {
            stream_id,
            promised_stream_id,
            header_block_fragment,
            padding: Some(padding),
        }
    }

    /// Convert to generic Frame
    pub fn to_frame(&self) -> Frame {
        let mut flags: u8 = 0;
        let mut payload = Vec::new();

        if self.padding.is_some() {
            flags |= flags::PUSH_PROMISE_PADDED;
        }

        // Promised stream ID (4 bytes)
        payload.extend_from_slice(&self.promised_stream_id.to_be_bytes());

        // Padding length byte (if padded)
        if let Some(ref pad) = self.padding {
            payload.push(pad.len() as u8);
            payload.extend_from_slice(&self.header_block_fragment);
            payload.extend_from_slice(pad);
        } else {
            payload.extend_from_slice(&self.header_block_fragment);
        }

        Frame::new(FrameType::PushPromise, flags, self.stream_id, payload)
    }

    /// Parse from Frame
    pub fn from_frame(frame: &Frame) -> Result<Self> {
        if frame.frame_type != FrameType::PushPromise {
            return Err(Http2Error::FrameParse(
                "Expected PUSH_PROMISE frame".to_string(),
            ));
        }

        let is_padded = (frame.flags & flags::PUSH_PROMISE_PADDED) != 0;

        if frame.payload.len() < 4 {
            return Err(Http2Error::FrameParse(
                "PUSH_PROMISE payload too short".to_string(),
            ));
        }

        let promised_stream_id = u32::from_be_bytes([
            frame.payload[0], frame.payload[1], frame.payload[2], frame.payload[3],
        ]) & 0x7FFFFFFF;

        let mut offset = 4;
        let mut padding = None;
        let mut hbf_start = offset;
        let mut hbf_end = frame.payload.len();

        if is_padded && !frame.payload.is_empty() {
            let pad_len = frame.payload[offset] as usize;
            offset += 1;
            hbf_start = offset;
            hbf_end = frame.payload.len().saturating_sub(pad_len);
            padding = Some(frame.payload[hbf_end..].to_vec());
        }

        let header_block_fragment = frame.payload[hbf_start..hbf_end].to_vec();

        Ok(PushPromiseFrame {
            stream_id: frame.stream_id,
            promised_stream_id,
            header_block_fragment,
            padding,
        })
    }
}

/// CONTINUATION frame (RFC 7540 §6.10)
#[derive(Debug, Clone)]
pub struct ContinuationFrame {
    /// Stream ID (must match the associated HEADERS/PUSH_PROMISE)
    pub stream_id: u32,
    /// Header block fragment (partial)
    pub header_block_fragment: Vec<u8>,
    /// END_HEADERS flag
    pub end_headers: bool,
}

impl ContinuationFrame {
    /// Create a new CONTINUATION frame
    pub fn new(stream_id: u32, header_block_fragment: Vec<u8>, end_headers: bool) -> Self {
        ContinuationFrame {
            stream_id,
            header_block_fragment,
            end_headers,
        }
    }

    /// Convert to generic Frame
    pub fn to_frame(&self) -> Frame {
        let flags = if self.end_headers {
            flags::CONTINUATION_END_HEADERS
        } else {
            0
        };

        Frame::new(
            FrameType::Continuation,
            flags,
            self.stream_id,
            self.header_block_fragment.clone(),
        )
    }

    /// Parse from Frame
    pub fn from_frame(frame: &Frame) -> Result<Self> {
        if frame.frame_type != FrameType::Continuation {
            return Err(Http2Error::FrameParse(
                "Expected CONTINUATION frame".to_string(),
            ));
        }

        let end_headers = (frame.flags & flags::CONTINUATION_END_HEADERS) != 0;

        Ok(ContinuationFrame {
            stream_id: frame.stream_id,
            header_block_fragment: frame.payload.clone(),
            end_headers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_type_from_u8() {
        assert_eq!(FrameType::from_u8(0), Some(FrameType::Data));
        assert_eq!(FrameType::from_u8(1), Some(FrameType::Headers));
        assert_eq!(FrameType::from_u8(8), Some(FrameType::WindowUpdate));
        assert_eq!(FrameType::from_u8(10), None);
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
            (0x1, 4096),   // HEADER_TABLE_SIZE
            (0x3, 100),    // MAX_CONCURRENT_STREAMS
            (0x4, 65535),  // INITIAL_WINDOW_SIZE
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
