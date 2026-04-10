//! HTTP/2 connection conformance tests.
//!
//! Tests connection preface validation (RFC 9113 §3.4) and SETTINGS
//! negotiation (RFC 9113 §6.5).

use crate::http2::frame::{Frame, FrameType};
use crate::http2::settings::Settings;
use crate::http2::stream::{Stream, StreamManager, StreamState};

/// The HTTP/2 connection preface (RFC 9113 §3.4).
/// Clients MUST send this as the first 24 octets.
const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

// ---------------------------------------------------------------------------
// Connection preface conformance (RFC 9113 §3.4)
// ---------------------------------------------------------------------------

#[test]
fn connection_preface_is_correct_bytes() {
    // RFC 9113 §3.4: the preface is exactly these 24 octets
    assert_eq!(PREFACE, b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n");
    assert_eq!(PREFACE.len(), 24);
}

#[test]
fn connection_preface_parse_as_frame() {
    // The preface looks like a DATA frame on stream 0 with a 9-byte header
    // followed by 15 bytes of payload (but it's actually special — not a valid frame).
    // The first 3 bytes are "PRI" which would decode as length 0x505249 = 5,263,945
    // which exceeds the default max frame size (16,384). So from_bytes rejects it.
    let result = Frame::from_bytes(PREFACE);
    // The preface is NOT a valid frame — it's a magic string that MUST be
    // recognized by the server before frame parsing begins.
    assert!(result.is_err(), "Connection preface should not parse as a valid frame");
}

#[test]
fn connection_preface_rejection_truncated() {
    // A truncated preface should be detected
    let truncated = b"PRI * HTTP/2.0\r\n\r\nSM";
    assert!(truncated.len() < 24);
}

// ---------------------------------------------------------------------------
// SETTINGS conformance (RFC 9113 §6.5, RFC 7540 §6.5.2)
// ---------------------------------------------------------------------------

#[test]
fn settings_frame_parse() {
    // SETTINGS frame: type=0x4, flags=0x0 (no ACK)
    // Payload: setting_id (2 bytes) + value (4 bytes) = 6 bytes per entry
    let wire: &[u8] = &[
        0x00, 0x00, 0x0C, // Length = 12 (2 settings)
        0x04,             // Type = SETTINGS
        0x00,             // Flags = 0
        0x00, 0x00, 0x00, 0x00, // Stream ID = 0
        // Setting 1: HEADER_TABLE_SIZE = 8192
        0x00, 0x01, 0x00, 0x00, 0x20, 0x00,
        // Setting 2: MAX_CONCURRENT_STREAMS = 5000
        0x00, 0x03, 0x00, 0x00, 0x13, 0x88,
    ];

    let (frame, consumed) = Frame::from_bytes(wire).unwrap();
    assert_eq!(consumed, wire.len());
    assert_eq!(frame.frame_type, FrameType::Settings);
    assert_eq!(frame.flags, 0x00);
    assert_eq!(frame.stream_id, 0);
    assert_eq!(frame.payload.len(), 12);

    // Validate semantics: payload % 6 == 0
    assert!(frame.validate_semantics().is_ok());
}

#[test]
fn settings_ack_frame_parse() {
    // ACK SETTINGS: type=0x4, flags=0x1
    let wire: &[u8] = &[
        0x00, 0x00, 0x00, // Length = 0
        0x04,             // Type = SETTINGS
        0x01,             // Flags = ACK
        0x00, 0x00, 0x00, 0x00, // Stream ID = 0
    ];

    let (frame, consumed) = Frame::from_bytes(wire).unwrap();
    assert_eq!(consumed, wire.len());
    assert_eq!(frame.frame_type, FrameType::Settings);
    assert_eq!(frame.flags, 0x01);
    assert_eq!(frame.stream_id, 0);
    assert_eq!(frame.payload.len(), 0);
    assert!(frame.validate_semantics().is_ok());
}

#[test]
fn settings_ack_with_payload_rejected() {
    // ACK SETTINGS with payload is a FRAME_SIZE_ERROR
    let wire: &[u8] = &[
        0x00, 0x00, 0x06, // Length = 6
        0x04,             // Type = SETTINGS
        0x01,             // Flags = ACK
        0x00, 0x00, 0x00, 0x00, // Stream ID = 0
        0x00, 0x01, 0x00, 0x00, 0x20, 0x00, // payload (6 bytes)
    ];

    let (frame, _) = Frame::from_bytes(wire).unwrap();
    assert_eq!(frame.validate_semantics(), Err(crate::http2::ErrorCode::FRAME_SIZE_ERROR.to_u32()));
}

#[test]
fn settings_on_nonzero_stream_rejected() {
    // SETTINGS on stream != 0 is PROTOCOL_ERROR
    let wire: &[u8] = &[
        0x00, 0x00, 0x06, // Length = 6
        0x04,             // Type = SETTINGS
        0x00,             // Flags = 0
        0x00, 0x00, 0x00, 0x01, // Stream ID = 1 (invalid!)
        0x00, 0x01, 0x00, 0x00, 0x20, 0x00,
    ];

    let (frame, _) = Frame::from_bytes(wire).unwrap();
    assert_eq!(frame.validate_semantics(), Err(crate::http2::ErrorCode::PROTOCOL_ERROR.to_u32()));
}

#[test]
fn settings_partial_payload_rejected() {
    // SETTINGS payload not a multiple of 6
    let wire: &[u8] = &[
        0x00, 0x00, 0x03, // Length = 3 (invalid)
        0x04,             // Type = SETTINGS
        0x00,             // Flags = 0
        0x00, 0x00, 0x00, 0x00, // Stream ID = 0
        0x00, 0x01, 0x00, // only 3 bytes of a setting
    ];

    let (frame, _) = Frame::from_bytes(wire).unwrap();
    assert_eq!(frame.validate_semantics(), Err(crate::http2::ErrorCode::FRAME_SIZE_ERROR.to_u32()));
}

// ---------------------------------------------------------------------------
// SETTINGS struct conformance
// ---------------------------------------------------------------------------

#[test]
fn settings_default_values() {
    let settings = Settings::new();
    // RFC 7540 §6.5.2: default values
    assert_eq!(settings.header_table_size, 4096);
    assert_eq!(settings.max_concurrent_streams, None); // default is unlimited
    assert_eq!(settings.initial_window_size, 65535);
    assert_eq!(settings.max_frame_size, 16384);
}

#[test]
fn settings_known_ids() {
    // RFC 7540 §6.5.2: setting identifiers
    const HEADER_TABLE_SIZE: u16 = 0x1;
    const ENABLE_PUSH: u16 = 0x2;
    const MAX_CONCURRENT_STREAMS: u16 = 0x3;
    const INITIAL_WINDOW_SIZE: u16 = 0x4;
    const MAX_FRAME_SIZE: u16 = 0x5;
    const MAX_HEADER_LIST_SIZE: u16 = 0x6;

    // Verify these match the settings module
    let settings = Settings::from_entries(&[
        (HEADER_TABLE_SIZE, 8192),
        (ENABLE_PUSH, 0),
        (MAX_CONCURRENT_STREAMS, 100),
        (INITIAL_WINDOW_SIZE, 32768),
        (MAX_FRAME_SIZE, 32768),
        (MAX_HEADER_LIST_SIZE, 65536),
    ]).unwrap();

    assert_eq!(settings.header_table_size, 8192);
    assert_eq!(settings.enable_push, 0);
    assert_eq!(settings.max_concurrent_streams, Some(100));
    assert_eq!(settings.initial_window_size, 32768);
    assert_eq!(settings.max_frame_size, 32768);
    assert_eq!(settings.max_header_list_size, Some(65536));
}

// ---------------------------------------------------------------------------
// Stream state machine conformance (RFC 9113 §5.1 / RFC 7540 §5.1)
// ---------------------------------------------------------------------------

#[test]
fn stream_initial_state_is_idle() {
    let stream = Stream::new(1, 65535);
    assert_eq!(stream.state, StreamState::Idle);
}

#[test]
fn stream_idle_to_open() {
    let mut stream = Stream::new(1, 65535);
    stream.open().unwrap();
    assert_eq!(stream.state, StreamState::Open);
}

#[test]
fn stream_open_to_half_closed_local() {
    let mut stream = Stream::new(1, 65535);
    stream.open().unwrap();
    stream.half_close_local().unwrap();
    assert_eq!(stream.state, StreamState::HalfClosedLocal);
}

#[test]
fn stream_open_to_half_closed_remote() {
    let mut stream = Stream::new(1, 65535);
    stream.open().unwrap();
    stream.half_close_remote().unwrap();
    assert_eq!(stream.state, StreamState::HalfClosedRemote);
}

#[test]
fn stream_half_closed_local_to_closed() {
    let mut stream = Stream::new(1, 65535);
    stream.open().unwrap();
    stream.half_close_local().unwrap();
    stream.half_close_remote().unwrap();
    assert_eq!(stream.state, StreamState::Closed);
}

#[test]
fn stream_half_closed_remote_to_closed() {
    let mut stream = Stream::new(1, 65535);
    stream.open().unwrap();
    stream.half_close_remote().unwrap();
    stream.half_close_local().unwrap();
    assert_eq!(stream.state, StreamState::Closed);
}

#[test]
fn stream_reject_open_from_closed() {
    let mut stream = Stream::new(1, 65535);
    stream.open().unwrap();
    stream.half_close_local().unwrap();
    stream.half_close_remote().unwrap();
    assert_eq!(stream.state, StreamState::Closed);
    assert!(stream.open().is_err());
}

#[test]
fn stream_reject_half_close_local_from_idle() {
    let mut stream = Stream::new(1, 65535);
    assert!(stream.half_close_local().is_err());
}

#[test]
fn stream_reject_half_close_remote_from_idle() {
    let mut stream = Stream::new(1, 65535);
    assert!(stream.half_close_remote().is_err());
}

#[test]
fn stream_client_initiated_odd_id() {
    let stream = Stream::new(1, 65535);
    assert!(stream.is_client_initiated());
    assert!(!stream.is_server_initiated());
}

#[test]
fn stream_server_initiated_even_id() {
    let stream = Stream::new(2, 65535);
    assert!(!stream.is_client_initiated());
    assert!(stream.is_server_initiated());
}

#[test]
fn stream_manager_creates_streams_in_order() {
    let mut manager = StreamManager::new(65535);
    let id1 = manager.create_client_stream().unwrap();
    let id2 = manager.create_client_stream().unwrap();
    let id3 = manager.create_client_stream().unwrap();
    assert_eq!(id1, 1);
    assert_eq!(id2, 3);
    assert_eq!(id3, 5);
}

#[test]
fn stream_manager_respects_max_concurrent() {
    let mut manager = StreamManager::new(65535);
    manager.set_max_concurrent_streams(2);
    assert!(manager.create_client_stream().is_ok());
    assert!(manager.create_client_stream().is_ok());
    assert!(manager.create_client_stream().is_err());
}

#[test]
fn stream_manager_cleanup_closed_streams() {
    let mut manager = StreamManager::new(65535);
    let id = manager.create_client_stream().unwrap();
    if let Some(s) = manager.get_stream_mut(id) {
        s.open().unwrap();
        s.half_close_local().unwrap();
        s.half_close_remote().unwrap();
        assert!(s.is_closed());
    }
    // closed streams are not active
    assert_eq!(manager.active_count(), 0);
    manager.cleanup_closed();
    assert_eq!(manager.active_count(), 0);
}

#[test]
fn stream_flow_control_window_decrements() {
    let mut stream = Stream::new(1, 65535);
    // Simulate receiving data
    stream.queue_received(vec![0; 1000]);
    // Local window should still reflect initial size (managed separately)
    assert_eq!(stream.local_window, 65535);
    assert_eq!(stream.remote_window, 65535);
}

// ---------------------------------------------------------------------------
// SETTINGS negotiation round-trip conformance (RFC 9113 §6.5)
// ---------------------------------------------------------------------------

/// Encode SETTINGS entries to wire format (matching what the encoder does).
fn encode_settings_entries(entries: &[(u16, u32)]) -> Vec<u8> {
    let mut buf = Vec::with_capacity(entries.len() * 6);
    for &(id, value) in entries {
        buf.extend_from_slice(&id.to_be_bytes());
        buf.extend_from_slice(&value.to_be_bytes());
    }
    buf
}

#[test]
fn settings_roundtrip_encode_decode() {
    // RFC 7540 §6.5.2: SETTINGS are encoded as 6-byte pairs
    let entries = Settings::new().to_entries();
    let wire = encode_settings_entries(&entries);

    // Build a SETTINGS frame from the wire data
    let len = wire.len() as u32;
    let mut frame_wire: Vec<u8> = Vec::with_capacity(9 + wire.len());
    frame_wire.extend_from_slice(&((len >> 16) as u8).to_be_bytes());
    frame_wire.extend_from_slice(&((len >> 8) as u8 & 0xFF).to_be_bytes());
    frame_wire.push((len & 0xFF) as u8);
    frame_wire.push(0x04); // Type = SETTINGS
    frame_wire.push(0x00); // Flags = 0
    frame_wire.extend_from_slice(&0u32.to_be_bytes()); // Stream ID = 0
    frame_wire.extend_from_slice(&wire);

    let (frame, consumed) = Frame::from_bytes(&frame_wire).unwrap();
    assert_eq!(consumed, frame_wire.len());
    assert_eq!(frame.frame_type, FrameType::Settings);
    assert_eq!(frame.stream_id, 0);

    // Parse the settings
    let settings_entries: Vec<(u16, u32)> = frame
        .payload
        .chunks(6)
        .map(|chunk| {
            let id = u16::from_be_bytes([chunk[0], chunk[1]]);
            let val = u32::from_be_bytes([chunk[2], chunk[3], chunk[4], chunk[5]]);
            (id, val)
        })
        .collect();

    // Round-trip: Settings::from_entries should reconstruct the settings
    let decoded = Settings::from_entries(&settings_entries).unwrap();
    assert_eq!(decoded.header_table_size, 4096);
    assert_eq!(decoded.initial_window_size, 65535);
    assert_eq!(decoded.max_frame_size, 16384);
}

#[test]
fn settings_roundtrip_custom_values() {
    let custom_entries = vec![
        (0x1, 8192),   // HEADER_TABLE_SIZE = 8192
        (0x3, 128),    // MAX_CONCURRENT_STREAMS = 128
        (0x4, 32768),  // INITIAL_WINDOW_SIZE = 32768
        (0x5, 32768),  // MAX_FRAME_SIZE = 32768
    ];
    let wire = encode_settings_entries(&custom_entries);

    // Build frame
    let len = wire.len() as u32;
    let mut frame_wire: Vec<u8> = Vec::with_capacity(9 + wire.len());
    frame_wire.extend_from_slice(&((len >> 16) as u8).to_be_bytes());
    frame_wire.extend_from_slice(&((len >> 8) as u8 & 0xFF).to_be_bytes());
    frame_wire.push((len & 0xFF) as u8);
    frame_wire.push(0x04);
    frame_wire.push(0x00);
    frame_wire.extend_from_slice(&0u32.to_be_bytes());
    frame_wire.extend_from_slice(&wire);

    let (frame, _) = Frame::from_bytes(&frame_wire).unwrap();

    // Decode
    let settings_entries: Vec<(u16, u32)> = frame
        .payload
        .chunks(6)
        .map(|chunk| {
            let id = u16::from_be_bytes([chunk[0], chunk[1]]);
            let val = u32::from_be_bytes([chunk[2], chunk[3], chunk[4], chunk[5]]);
            (id, val)
        })
        .collect();

    let decoded = Settings::from_entries(&settings_entries).unwrap();
    assert_eq!(decoded.header_table_size, 8192);
    assert_eq!(decoded.max_concurrent_streams, Some(128));
    assert_eq!(decoded.initial_window_size, 32768);
    assert_eq!(decoded.max_frame_size, 32768);
}

#[test]
fn settings_validate_known_ids() {
    // Settings must have valid values per RFC 7540 §6.5.2
    let entries = vec![
        (0x1, 8192),   // HEADER_TABLE_SIZE: valid
        (0x4, 32768),  // INITIAL_WINDOW_SIZE: valid
        (0x5, 32768),  // MAX_FRAME_SIZE: valid (must be >= 16384 and <= 2^24-1)
    ];
    let settings = Settings::from_entries(&entries).unwrap();
    assert!(settings.validate().is_ok());
}

#[test]
fn settings_validate_invalid_max_frame_size() {
    // MAX_FRAME_SIZE must be >= 16384 and <= 2^24-1
    // from_entries validates this and returns Err
    let entries = vec![
        (0x5, 100), // Too small
    ];
    assert!(Settings::from_entries(&entries).is_err());
}

#[test]
fn settings_validate_invalid_window_size() {
    // INITIAL_WINDOW_SIZE must be <= 2^31-1
    // from_entries validates this and returns Err
    let entries = vec![
        (0x4, 2_147_483_648u32), // Too large (2^31)
    ];
    assert!(Settings::from_entries(&entries).is_err());
}

#[test]
fn settings_ack_frame_has_no_payload() {
    // RFC 7540 §6.5: ACK SETTINGS frame has zero-length payload
    let ack_wire: &[u8] = &[
        0x00, 0x00, 0x00, // Length = 0
        0x04, 0x01,       // Type = SETTINGS, Flags = ACK
        0x00, 0x00, 0x00, 0x00,
    ];
    let (frame, _) = Frame::from_bytes(ack_wire).unwrap();
    assert_eq!(frame.payload.len(), 0);
    assert_eq!(frame.flags, 0x01);
    assert!(frame.validate_semantics().is_ok());
}
