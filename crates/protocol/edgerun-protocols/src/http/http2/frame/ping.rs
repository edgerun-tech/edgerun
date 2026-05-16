use super::Frame;
use super::FrameType;
use super::flags;
use super::{Http2Error, Result};
use alloc::string::ToString;

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
            return Err(Http2Error::FrameParse("Expected PING frame".to_string()));
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
