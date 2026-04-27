#[cfg(target_os = "none")]
use crate::prelude::v1::*;

use super::Frame;
use super::FrameType;
use super::{Http2Error, Result};

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
