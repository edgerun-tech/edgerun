#[cfg(target_os = "none")]
use crate::prelude::v1::*;

use super::Frame;
use super::FrameType;
use super::{Http2Error, Result};

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
        let payload = vec![
            (self.error_code >> 24) as u8,
            (self.error_code >> 16) as u8,
            (self.error_code >> 8) as u8,
            self.error_code as u8,
        ];

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
            return Err(Http2Error::FrameParse("RST_STREAM on stream 0".to_string()));
        }

        let error_code = frame.payload_u32()?;

        Ok(RstStreamFrame {
            stream_id: frame.stream_id,
            error_code,
        })
    }
}
