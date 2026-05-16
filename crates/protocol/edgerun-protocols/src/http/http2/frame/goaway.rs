use super::Frame;
use super::FrameType;
use super::{Http2Error, Result};
use alloc::string::ToString;
use alloc::vec::Vec;

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
            return Err(Http2Error::FrameParse("Expected GOAWAY frame".to_string()));
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
