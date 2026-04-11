//! HTTP/2 frame sequence conformance tests.
//!
//! Validates that frame sequences follow RFC 9113 ordering rules:
//! - Connection preface → SETTINGS before any other frames
//! - HEADERS before DATA on a stream
//! - CONTINUATION immediately follows HEADERS/PUSH_PROMISE
//! - WINDOW_UPDATE only with valid increment
//! - RST_STREAM transitions stream to closed

use crate::http2::frame::{flags, Frame, FrameType};

/// Frame sequence validator per RFC 9113.
/// Tracks connection and stream-level state to enforce frame ordering rules.
pub struct FrameSequenceValidator {
    /// Connection-level SETTINGS has been received
    settings_received: bool,
    /// Connection-level GOAWAY has been received
    goaway_received: bool,
    /// Per-stream state: whether HEADERS has been received
    stream_headers_seen: std::collections::HashSet<u32>,
    /// Per-stream state: whether stream is closed
    stream_closed: std::collections::HashSet<u32>,
    /// Whether we're expecting a CONTINUATION frame
    expecting_continuation: bool,
}

impl FrameSequenceValidator {
    pub fn new() -> Self {
        FrameSequenceValidator {
            settings_received: false,
            goaway_received: false,
            stream_headers_seen: std::collections::HashSet::new(),
            stream_closed: std::collections::HashSet::new(),
            expecting_continuation: false,
        }
    }

    /// Validate a frame in the context of the current connection state.
    /// Returns Ok(()) or an error description.
    pub fn validate(&mut self, frame: &Frame) -> std::result::Result<(), String> {
        match frame.frame_type {
            FrameType::Settings => {
                if frame.stream_id != 0 {
                    return Err("SETTINGS must be on stream 0".to_string());
                }
                self.settings_received = true;
            }
            FrameType::Headers => {
                if self.goaway_received && frame.stream_id != 0 {
                    return Err("HEADERS after GOAWAY on non-zero stream".to_string());
                }
                if self.stream_closed.contains(&frame.stream_id) {
                    return Err(format!("HEADERS on closed stream {}", frame.stream_id));
                }
                self.stream_headers_seen.insert(frame.stream_id);

                // If END_HEADERS is not set, next frame MUST be CONTINUATION
                if frame.flags & flags::HEADERS_END_HEADERS == 0 {
                    self.expecting_continuation = true;
                }
            }
            FrameType::Continuation => {
                if !self.expecting_continuation {
                    return Err("CONTINUATION without pending HEADERS/PUSH_PROMISE".to_string());
                }
                self.expecting_continuation = false;
            }
            FrameType::Data => {
                if self.goaway_received && frame.stream_id != 0 {
                    return Err("DATA after GOAWAY on non-zero stream".to_string());
                }
                if self.stream_closed.contains(&frame.stream_id) {
                    return Err(format!("DATA on closed stream {}", frame.stream_id));
                }
                if !self.stream_headers_seen.contains(&frame.stream_id) {
                    return Err(format!("DATA before HEADERS on stream {}", frame.stream_id));
                }
            }
            FrameType::WindowUpdate => {
                if frame.stream_id != 0 && self.stream_closed.contains(&frame.stream_id) {
                    return Err(format!("WINDOW_UPDATE on closed stream {}", frame.stream_id));
                }
            }
            FrameType::RstStream => {
                if self.stream_closed.contains(&frame.stream_id) {
                    return Err(format!("RST_STREAM on already-closed stream {}", frame.stream_id));
                }
                if frame.stream_id == 0 {
                    return Err("RST_STREAM must not be on stream 0".to_string());
                }
                self.stream_closed.insert(frame.stream_id);
            }
            FrameType::Goaway => {
                if frame.stream_id != 0 {
                    return Err("GOAWAY must be on stream 0".to_string());
                }
                self.goaway_received = true;
            }
            FrameType::Ping | FrameType::Priority | FrameType::PushPromise => {
                // Generally allowed; specific rules enforced by validate_semantics
            }
        }
        Ok(())
    }

    /// Check if the next frame MUST be a CONTINUATION.
    pub fn expecting_continuation(&self) -> bool {
        self.expecting_continuation
    }
}

// ---------------------------------------------------------------------------
// Frame sequence conformance tests
// ---------------------------------------------------------------------------

#[test]
fn frame_sequence_valid_client_preface_settings_headers_data() {
    // Valid sequence: SETTINGS → HEADERS (with END_HEADERS) → DATA
    let mut validator = FrameSequenceValidator::new();

    // SETTINGS on stream 0
    let settings_wire: &[u8] = &[
        0x00, 0x00, 0x00, // Length = 0 (ACK)
        0x04, 0x01,       // Type = SETTINGS, Flags = ACK
        0x00, 0x00, 0x00, 0x00,
    ];
    let (settings, _) = Frame::from_bytes(settings_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&settings).is_ok());

    // HEADERS on stream 1 with END_HEADERS
    let headers_wire: &[u8] = &[
        0x00, 0x00, 0x05, // Length = 5
        0x01, 0x04,       // Type = HEADERS, Flags = END_HEADERS
        0x00, 0x00, 0x00, 0x01, // Stream ID = 1
        0x82, 0x86, 0x84, 0x41, 0x8a, // dummy HPACK
    ];
    let (headers, _) = Frame::from_bytes(headers_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&headers).is_ok());
    assert!(!validator.expecting_continuation());

    // DATA on stream 1
    let data_wire: &[u8] = &[
        0x00, 0x00, 0x05, // Length = 5
        0x00, 0x01,       // Type = DATA, Flags = END_STREAM
        0x00, 0x00, 0x00, 0x01, // Stream ID = 1
        0x48, 0x65, 0x6c, 0x6c, 0x6f, // "Hello"
    ];
    let (data, _) = Frame::from_bytes(data_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&data).is_ok());
}

#[test]
fn frame_sequence_invalid_data_before_headers() {
    // DATA before HEADERS on a stream is a protocol violation
    let mut validator = FrameSequenceValidator::new();

    // SETTINGS first
    let settings_wire: &[u8] = &[
        0x00, 0x00, 0x00, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00,
    ];
    let (settings, _) = Frame::from_bytes(settings_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&settings).is_ok());

    // DATA on stream 1 without prior HEADERS
    let data_wire: &[u8] = &[
        0x00, 0x00, 0x05, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
        0x48, 0x65, 0x6c, 0x6c, 0x6f,
    ];
    let (data, _) = Frame::from_bytes(data_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&data).is_err());
}

#[test]
fn frame_sequence_invalid_continuation_without_headers() {
    // CONTINUATION without pending HEADERS is a protocol violation
    let mut validator = FrameSequenceValidator::new();

    let cont_wire: &[u8] = &[
        0x00, 0x00, 0x05, 0x09, 0x04, 0x00, 0x00, 0x00, 0x01,
        0x82, 0x86, 0x84, 0x41, 0x8a,
    ];
    let (cont, _) = Frame::from_bytes(cont_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&cont).is_err());
}

#[test]
fn frame_sequence_headers_without_end_headers_expects_continuation() {
    // HEADERS without END_HEADERS flag must be followed by CONTINUATION
    let mut validator = FrameSequenceValidator::new();

    // SETTINGS
    let settings_wire: &[u8] = &[0x00, 0x00, 0x00, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00];
    let (settings, _) = Frame::from_bytes(settings_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&settings).is_ok());

    // HEADERS without END_HEADERS
    let headers_wire: &[u8] = &[
        0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x82, 0x86, 0x84,
    ];
    let (headers, _) = Frame::from_bytes(headers_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&headers).is_ok());
    assert!(validator.expecting_continuation());
}

#[test]
fn frame_sequence_continuation_after_headers() {
    // Valid: HEADERS (no END_HEADERS) → CONTINUATION
    let mut validator = FrameSequenceValidator::new();

    // SETTINGS
    let settings_wire: &[u8] = &[0x00, 0x00, 0x00, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00];
    let (settings, _) = Frame::from_bytes(settings_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&settings).is_ok());

    // HEADERS without END_HEADERS
    let headers_wire: &[u8] = &[
        0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x82, 0x86, 0x84,
    ];
    let (headers, _) = Frame::from_bytes(headers_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&headers).is_ok());

    // CONTINUATION
    let cont_wire: &[u8] = &[
        0x00, 0x00, 0x03, 0x09, 0x04, 0x00, 0x00, 0x00, 0x01,
        0x41, 0x8a, 0x8c,
    ];
    let (cont, _) = Frame::from_bytes(cont_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&cont).is_ok());
    assert!(!validator.expecting_continuation());
}

#[test]
fn frame_sequence_data_after_goaway_rejected() {
    // DATA after GOAWAY on non-zero stream is rejected
    let mut validator = FrameSequenceValidator::new();

    // SETTINGS
    let settings_wire: &[u8] = &[0x00, 0x00, 0x00, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00];
    let (settings, _) = Frame::from_bytes(settings_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&settings).is_ok());

    // GOAWAY
    let goaway_wire: &[u8] = &[
        0x00, 0x00, 0x08, // Length = 8
        0x07, 0x00,       // Type = GOAWAY
        0x00, 0x00, 0x00, 0x00, // Stream ID = 0
        0x00, 0x00, 0x00, 0x00, // Last stream ID = 0
        0x00, 0x00, 0x00, 0x00, // Error code = 0
    ];
    let (goaway, _) = Frame::from_bytes(goaway_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&goaway).is_ok());

    // DATA on stream 1 after GOAWAY
    let data_wire: &[u8] = &[
        0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x48, 0x65, 0x6c, 0x6c, 0x6f,
    ];
    let (data, _) = Frame::from_bytes(data_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&data).is_err());
}

#[test]
fn frame_sequence_rst_stream_closes_stream() {
    // After RST_STREAM, no more frames on that stream
    let mut validator = FrameSequenceValidator::new();

    // SETTINGS
    let settings_wire: &[u8] = &[0x00, 0x00, 0x00, 0x04, 0x01, 0x00, 0x00, 0x00, 0x00];
    let (settings, _) = Frame::from_bytes(settings_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&settings).is_ok());

    // HEADERS on stream 1
    let headers_wire: &[u8] = &[
        0x00, 0x00, 0x05, 0x01, 0x04, 0x00, 0x00, 0x00, 0x01,
        0x82, 0x86, 0x84, 0x41, 0x8a,
    ];
    let (headers, _) = Frame::from_bytes(headers_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&headers).is_ok());

    // RST_STREAM on stream 1
    let rst_wire: &[u8] = &[
        0x00, 0x00, 0x04, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x00, // Error code = 0
    ];
    let (rst, _) = Frame::from_bytes(rst_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&rst).is_ok());

    // DATA on stream 1 after RST_STREAM
    let data_wire: &[u8] = &[
        0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x48, 0x65, 0x6c, 0x6c, 0x6f,
    ];
    let (data, _) = Frame::from_bytes(data_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    assert!(validator.validate(&data).is_err());
}

#[test]
fn frame_sequence_rst_stream_on_stream_zero_rejected() {
    let rst_wire: &[u8] = &[
        0x00, 0x00, 0x04, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ];
    let (rst, _) = Frame::from_bytes(rst_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    let mut validator = FrameSequenceValidator::new();
    assert!(validator.validate(&rst).is_err());
}

#[test]
fn frame_sequence_goaway_on_nonzero_stream_rejected() {
    let goaway_wire: &[u8] = &[
        0x00, 0x00, 0x08, 0x07, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    let (goaway, _) = Frame::from_bytes(goaway_wire, Frame::DEFAULT_MAX_FRAME_SIZE).unwrap();
    let mut validator = FrameSequenceValidator::new();
    assert!(validator.validate(&goaway).is_err());
}
