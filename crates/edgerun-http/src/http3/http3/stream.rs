//! HTTP/3 stream management

/// HTTP/3 stream types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Http3StreamType {
    /// Control stream (one per connection, carries SETTINGS)
    Control,
    /// Request stream (bidirectional)
    Request,
    /// Push stream (server-initiated, unidirectional)
    Push,
    /// QPACK encoder stream
    QpackEncoder,
    /// QPACK decoder stream
    QpackDecoder,
    /// Unknown/reserved
    Unknown,
}

/// HTTP/3 stream state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Http3StreamState {
    /// Idle
    Idle,
    /// Open
    Open,
    /// Half-closed (local)
    HalfClosedLocal,
    /// Half-closed (remote)
    HalfClosedRemote,
    /// Closed
    Closed,
}

/// HTTP/3 stream
pub struct Http3Stream {
    /// Stream ID
    pub id: u64,
    /// Stream type
    pub stream_type: Http3StreamType,
    /// State
    pub state: Http3StreamState,
    /// Frame type currently being received
    pub current_frame_type: Option<u64>,
    /// Frame payload being accumulated
    pub frame_payload: Vec<u8>,
    /// Expected frame length
    pub expected_frame_length: Option<u64>,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// End-of-stream received
    pub fin_received: bool,
}

impl Http3Stream {
    /// Create a new stream
    pub fn new(id: u64, stream_type: Http3StreamType) -> Self {
        Http3Stream {
            id,
            stream_type,
            state: Http3StreamState::Idle,
            current_frame_type: None,
            frame_payload: Vec::new(),
            expected_frame_length: None,
            bytes_sent: 0,
            bytes_received: 0,
            fin_received: false,
        }
    }

    /// Check if this is a unidirectional stream
    pub fn is_unidirectional(&self) -> bool {
        matches!(
            self.stream_type,
            Http3StreamType::QpackEncoder | Http3StreamType::QpackDecoder | Http3StreamType::Push
        )
    }

    /// Open the stream
    pub fn open(&mut self) {
        if self.state == Http3StreamState::Idle {
            self.state = Http3StreamState::Open;
        }
    }

    /// Mark half-closed local
    pub fn half_close_local(&mut self) {
        self.state = match self.state {
            Http3StreamState::Open => Http3StreamState::HalfClosedLocal,
            Http3StreamState::HalfClosedRemote => Http3StreamState::Closed,
            s => s,
        };
    }

    /// Mark half-closed remote
    pub fn half_close_remote(&mut self) {
        self.state = match self.state {
            Http3StreamState::Open => Http3StreamState::HalfClosedRemote,
            Http3StreamState::HalfClosedLocal => Http3StreamState::Closed,
            s => s,
        };
        self.fin_received = true;
    }

    /// Check if stream is closed
    pub fn is_closed(&self) -> bool {
        self.state == Http3StreamState::Closed
    }

    /// Process incoming data for frame parsing
    pub fn process_data(&mut self, data: &[u8]) -> Vec<(Option<u64>, Vec<u8>)> {
        let mut frames = Vec::new();
        self.bytes_received += data.len() as u64;

        if self.current_frame_type.is_none() {
            // Need to read frame type and length
            if data.len() < 2 {
                self.frame_payload.extend_from_slice(data);
                return frames;
            }

            let mut pos = 0;

            // Read frame type
            let (frame_type, ft_len) = Self::decode_varint(&data[pos..]).unwrap_or((0, 1));
            pos += ft_len;

            // Read frame length
            let (frame_len, fl_len) = Self::decode_varint(&data[pos..]).unwrap_or((0, 1));
            pos += fl_len;

            self.current_frame_type = Some(frame_type);
            self.expected_frame_length = Some(frame_len);

            // Process remaining data as frame payload
            let remaining = &data[pos..];
            if remaining.len() >= frame_len as usize {
                // Complete frame
                frames.push((Some(frame_type), remaining[..frame_len as usize].to_vec()));
                self.frame_payload.clear();
                self.current_frame_type = None;
                self.expected_frame_length = None;

                // Check if more frames follow
                let after_frame = &remaining[frame_len as usize..];
                if !after_frame.is_empty() {
                    frames.extend(self.process_data(after_frame));
                }
            } else {
                // Partial frame
                self.frame_payload.extend_from_slice(remaining);
            }
        } else {
            // Accumulating frame payload
            let expected = self.expected_frame_length.unwrap_or(0) as usize;
            self.frame_payload.extend_from_slice(data);

            if self.frame_payload.len() >= expected {
                let frame_type = self.current_frame_type;
                let payload = self.frame_payload[..expected].to_vec();
                frames.push((frame_type, payload));
                self.frame_payload = self.frame_payload[expected..].to_vec();
                self.current_frame_type = None;
                self.expected_frame_length = None;

                // Process remaining
                if !self.frame_payload.is_empty() {
                    let remaining = self.frame_payload.clone();
                    self.frame_payload.clear();
                    frames.extend(self.process_data(&remaining));
                }
            }
        }

        frames
    }

    fn decode_varint(data: &[u8]) -> Result<(u64, usize), String> {
        edgerun_encoding::quic_varint::decode_varint(data)
            .map_err(|e| format!("{e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_new() {
        let stream = Http3Stream::new(0, Http3StreamType::Request);
        assert_eq!(stream.id, 0);
        assert!(!stream.is_unidirectional());
    }

    #[test]
    fn test_stream_state_transitions() {
        let mut stream = Http3Stream::new(0, Http3StreamType::Request);
        stream.open();
        assert_eq!(stream.state, Http3StreamState::Open);

        stream.half_close_local();
        assert_eq!(stream.state, Http3StreamState::HalfClosedLocal);

        stream.half_close_remote();
        assert!(stream.is_closed());
    }

    #[test]
    fn test_unidirectional_stream() {
        let stream = Http3Stream::new(2, Http3StreamType::QpackEncoder);
        assert!(stream.is_unidirectional());
    }

    #[test]
    fn test_stream_process_complete_frame() {
        let mut stream = Http3Stream::new(0, Http3StreamType::Request);
        // DATA frame: type=0, len=5, payload="hello"
        let data = vec![0x00, 0x05, 0x68, 0x65, 0x6c, 0x6c, 0x6f];
        let frames = stream.process_data(&data);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].0, Some(0)); // DATA frame type
        assert_eq!(frames[0].1, b"hello");
    }
}
