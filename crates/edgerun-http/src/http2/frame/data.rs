use super::flags;
use super::Frame;
use super::FrameType;
use super::{Http2Error, Result};
use alloc::string::ToString;
use alloc::vec::Vec;

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
            return Err(Http2Error::FrameParse("Expected DATA frame".to_string()));
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
