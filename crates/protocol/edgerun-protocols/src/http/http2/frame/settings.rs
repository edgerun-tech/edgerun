use super::Frame;
use super::FrameType;
use super::flags;
use super::{Http2Error, Result};
use alloc::string::ToString;
use alloc::vec::Vec;

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

        if !frame.payload.len().is_multiple_of(6) {
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
