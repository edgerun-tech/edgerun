#[cfg(target_os = "none")]
use crate::prelude::v1::*;

use super::flags;
use super::Frame;
use super::FrameType;
use super::{Http2Error, Result};

/// CONTINUATION frame (RFC 7540 Sec6.10)
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
