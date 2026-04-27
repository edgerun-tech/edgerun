//! Continuation tracking state for HEADERS + CONTINUATION frame sequences.

#[cfg(target_os = "none")]
use crate::prelude::v1::*;

/// Tracks whether we are expecting CONTINUATION frames after a partial HEADERS frame.
#[derive(Debug, Default)]
pub struct ContinuationState {
    /// Whether we are currently expecting CONTINUATION frames.
    pub expecting: bool,
    /// The stream ID that the CONTINUATION frames must target.
    pub stream_id: u32,
    /// Accumulated header block bytes from partial HEADERS + CONTINUATION frames.
    pub header_block_buf: Vec<u8>,
}

impl ContinuationState {
    /// Create a new empty continuation state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start expecting CONTINUATION frames for the given stream.
    pub fn start(&mut self, stream_id: u32, initial_buf: &[u8]) {
        self.expecting = true;
        self.stream_id = stream_id;
        self.header_block_buf.clear();
        self.header_block_buf.extend_from_slice(initial_buf);
    }

    /// Mark continuation as complete and return the accumulated header block.
    pub fn finish(&mut self) -> Vec<u8> {
        self.expecting = false;
        std::mem::take(&mut self.header_block_buf)
    }

    /// Abort continuation (e.g. on error) and clear state.
    pub fn abort(&mut self) {
        self.expecting = false;
        self.header_block_buf.clear();
    }
}
