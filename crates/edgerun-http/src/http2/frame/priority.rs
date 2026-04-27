use super::Frame;
use super::FrameType;
use super::{Http2Error, Result};
use alloc::string::ToString;
use alloc::vec::Vec;

/// PRIORITY frame (RFC 7540 Sec6.3)
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
        let w = if weight == 0 { 1 } else { weight };
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
            frame.payload[0],
            frame.payload[1],
            frame.payload[2],
            frame.payload[3],
        ]);
        let exclusive = (dep_raw >> 31) != 0;
        let stream_dependency = dep_raw & 0x7FFFFFFF;
        let weight = frame.payload[4].wrapping_add(1); // Wire format is weight - 1 (0->1, 255->256)

        Ok(PriorityFrame {
            stream_id: frame.stream_id,
            exclusive,
            stream_dependency,
            weight,
        })
    }
}
