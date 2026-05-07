use super::flags;
use super::Frame;
use super::FrameType;
use super::{Http2Error, Result};
use alloc::string::ToString;
use alloc::vec::Vec;

/// PUSH_PROMISE frame (RFC 7540 Sec6.6)
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
            frame.payload[0],
            frame.payload[1],
            frame.payload[2],
            frame.payload[3],
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
