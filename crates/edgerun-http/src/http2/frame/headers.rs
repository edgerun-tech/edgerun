use super::flags;
use super::Frame;
use super::FrameType;
use super::{Http2Error, Result};

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
            return Err(Http2Error::FrameParse("Expected HEADERS frame".to_string()));
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
