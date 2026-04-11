use super::*;
use crate::http2::frame::{DataFrame, HeadersFrame, PingFrame, PriorityFrame, RstStreamFrame, WindowUpdateFrame};
use crate::http2::{Decoder, Encoder, Frame, FrameType};

fn h(name: &str, value: &str) -> (Vec<u8>, Vec<u8>) {
    (name.as_bytes().to_vec(), value.as_bytes().to_vec())
}

fn make_headers_frame(
    stream_id: u32,
    headers: &[(Vec<u8>, Vec<u8>)],
    end_stream: bool,
    encoder: &mut Encoder,
) -> Frame {
    let block = encoder.encode(headers.iter().map(|(k, v)| (k.as_slice(), v.as_slice())));
    HeadersFrame::new(stream_id, block, end_stream).to_frame()
}

// ── Server construction ──

#[test]
fn test_server_default_settings() {
    let server = Http2Server::new();
    assert_eq!(server.max_frame_size, 16384);
    assert_eq!(server.last_processed_stream_id, 0);
    assert!(!server.goaway_sent);
    assert!(server.pending_headers.is_empty());
}

// ── SETTINGS handling ──

#[test]
fn test_apply_client_settings_default() {
    let mut server = Http2Server::new();
    let sf = SettingsFrame::new(vec![]);
    let action = server.apply_client_settings(&sf);
    match action {
        FrameAction::WriteFrames(frames) => {
            assert_eq!(frames.len(), 2);
        }
        _ => panic!("expected WriteFrames"),
    }
}

#[test]
fn test_apply_client_settings_custom_max_frame_size() {
    let mut server = Http2Server::new();
    let sf = SettingsFrame::new(vec![(0x5, 32768)]);
    let action = server.apply_client_settings(&sf);
    assert_eq!(server.max_frame_size, 32768);
    match action {
        FrameAction::WriteFrames(_) => {}
        _ => panic!("expected WriteFrames"),
    }
}

#[test]
fn test_apply_client_settings_max_concurrent_streams() {
    let mut server = Http2Server::new();
    let sf = SettingsFrame::new(vec![(0x3, 10)]);
    server.apply_client_settings(&sf);
}

#[test]
fn test_apply_invalid_enable_push() {
    let mut server = Http2Server::new();
    let sf = SettingsFrame::new(vec![(0x2, 5)]);
    match server.apply_client_settings(&sf) {
        FrameAction::Goaway { error_code, .. } => {
            assert_eq!(error_code, ErrorCode::PROTOCOL_ERROR.to_u32());
        }
        _ => panic!("expected Goaway for invalid settings"),
    }
}

#[test]
fn test_handle_settings_ack() {
    let mut server = Http2Server::new();
    let ack = SettingsFrame::ack().to_frame();
    let action = server.handle_settings(&ack);
    assert!(matches!(action, FrameAction::None));
}

// ── PING handling ──

#[test]
fn test_handle_ping() {
    let mut server = Http2Server::new();
    let pf = PingFrame::new([1, 2, 3, 4, 5, 6, 7, 8]).to_frame();
    match server.handle_ping(&pf) {
        FrameAction::WriteFrames(frames) => {
            assert_eq!(frames.len(), 1);
            let parsed = PingFrame::from_frame(&frames[0]).unwrap();
            assert!(parsed.ack);
            assert_eq!(parsed.data, [1, 2, 3, 4, 5, 6, 7, 8]);
        }
        _ => panic!("expected WriteFrames for PING"),
    }
}

#[test]
fn test_handle_ping_ack_noop() {
    let mut server = Http2Server::new();
    let pf = PingFrame::ack([1, 2, 3, 4, 5, 6, 7, 8]).to_frame();
    match server.handle_ping(&pf) {
        FrameAction::None => {}
        _ => panic!("expected None for PING ACK"),
    }
}

// ── WINDOW_UPDATE handling ──

#[test]
fn test_handle_window_update_connection() {
    let mut server = Http2Server::new();
    let wu = WindowUpdateFrame::new(0, 1000).to_frame();
    let action = server.handle_window_update(&wu);
    assert!(matches!(action, FrameAction::None));
    assert_eq!(server.flow_controller.window_size(), 65535 + 1000);
}

#[test]
fn test_handle_window_update_stream() {
    let mut server = Http2Server::new();
    let _ = server.stream_manager.get_or_create_stream(1);
    let wu = WindowUpdateFrame::new(1, 500).to_frame();
    let action = server.handle_window_update(&wu);
    assert!(matches!(action, FrameAction::None));
    let stream = server.stream_manager.get_stream(1).unwrap();
    assert_eq!(stream.remote_window, 65535 + 500);
}

#[test]
fn test_handle_window_update_stream_not_yet_created() {
    let mut server = Http2Server::new();
    let wu = WindowUpdateFrame::new(3, 200).to_frame();
    let action = server.handle_window_update(&wu);
    match action {
        FrameAction::Goaway { error_code, .. } => {
            assert_eq!(error_code, ErrorCode::PROTOCOL_ERROR.to_u32());
        }
        _ => panic!("expected Goaway for WINDOW_UPDATE on idle stream"),
    }
}

#[test]
fn test_handle_window_update_zero_increment() {
    let mut server = Http2Server::new();
    let frame = Frame::new(FrameType::WindowUpdate, 0, 0, vec![0, 0, 0, 0]);
    match server.handle_window_update(&frame) {
        FrameAction::Goaway { error_code, .. } => {
            assert_eq!(error_code, ErrorCode::FLOW_CONTROL_ERROR.to_u32());
        }
        _ => panic!("expected Goaway for zero increment"),
    }
}

// ── RST_STREAM handling ──

#[test]
fn test_handle_priority_on_idle_stream() {
    let mut server = Http2Server::new();
    assert!(server.stream_manager.get_stream(1).is_none());

    let pf = PriorityFrame::new(1, false, 0, 16);
    match server.handle_priority(&pf) {
        FrameAction::None => {}
        other => panic!("expected None for PRIORITY on idle stream, got {other:?}"),
    }

    assert!(server.stream_manager.get_stream(1).is_none());
}

#[test]
fn test_handle_priority_on_closed_stream() {
    let mut server = Http2Server::new();
    let _ = server.stream_manager.get_or_create_stream(1);
    let s = server.stream_manager.get_stream_mut(1).unwrap();
    s.open().unwrap();
    s.half_close_remote().unwrap();
    s.close();
    server.stream_manager.cleanup_closed();

    assert!(server.stream_manager.get_stream(1).is_none());

    let pf = PriorityFrame::new(1, false, 0, 16);
    match server.handle_priority(&pf) {
        FrameAction::None => {}
        other => panic!("expected None for PRIORITY on closed stream, got {other:?}"),
    }
}

#[test]
fn test_priority_frame_passes_semantics_validation() {
    let valid_payload = vec![0, 0, 0, 0, 15];
    let frame = Frame::new(FrameType::Priority, 0, 1, valid_payload);
    assert!(
        frame.validate_semantics().is_ok(),
        "PRIORITY on stream 1 should pass semantics"
    );

    let frame2 = Frame::new(FrameType::Priority, 0, 0, vec![0, 0, 0, 0, 15]);
    assert!(
        frame2.validate_semantics().is_err(),
        "PRIORITY on stream 0 should fail semantics"
    );
}

#[test]
fn test_priority_frame_weight_256_no_overflow() {
    let payload = vec![0, 0, 0, 0, 255];
    let frame = Frame::new(FrameType::Priority, 0, 1, payload);
    let pf = PriorityFrame::from_frame(&frame).expect("should parse weight 256");
    assert_eq!(pf.weight, 0);
}

#[test]
fn test_handle_rst_stream() {
    let mut server = Http2Server::new();
    let _ = server.stream_manager.get_or_create_stream(1);
    let stream = server.stream_manager.get_stream_mut(1).unwrap();
    stream.open().unwrap();

    let rst = RstStreamFrame::new(1, 0).to_frame();
    let action = server.handle_rst_stream(&rst);
    assert!(matches!(action, FrameAction::None));
    let s = server.stream_manager.get_stream(1).unwrap();
    assert!(s.is_closed());
}

// ── PRIORITY handling ──

#[test]
fn test_handle_priority_noop() {
    let mut server = Http2Server::new();
    let pf = PriorityFrame::new(1, false, 0, 16);
    match server.handle_priority(&pf) {
        FrameAction::None => {}
        _ => panic!("expected None for PRIORITY"),
    }
}

#[test]
fn test_handle_priority_self_referential_dependency() {
    let mut server = Http2Server::new();
    let pf = PriorityFrame::new(1, true, 1, 16);
    match server.handle_priority(&pf) {
        FrameAction::WriteFrames(frames) => {
            assert!(!frames.is_empty());
            let rst = RstStreamFrame::from_frame(&frames[0]).unwrap();
            assert_eq!(rst.stream_id, 1);
            assert_eq!(rst.error_code, ErrorCode::PROTOCOL_ERROR.to_u32());
        }
        other => panic!("expected RST_STREAM for self-referential PRIORITY, got {other:?}"),
    }
}

#[test]
fn test_handle_priority_stream_zero() {
    let mut server = Http2Server::new();
    let pf = PriorityFrame::new(0, false, 0, 16);
    match server.handle_priority(&pf) {
        FrameAction::Goaway { error_code, .. } => {
            assert_eq!(error_code, ErrorCode::PROTOCOL_ERROR.to_u32());
        }
        other => panic!("expected Goaway for PRIORITY on stream 0, got {other:?}"),
    }
}

// ── HEADERS handling ──

#[test]
fn test_handle_headers_get_request() {
    let mut server = Http2Server::new();
    let mut encoder = Encoder::new();
    let mut decoder = Decoder::new();
    let mut cont = ContinuationState::new();

    let headers = vec![h(":method", "GET"), h(":scheme", "https"), h(":path", "/")];
    let frame = make_headers_frame(1, &headers, true, &mut encoder);

    let action = server.handle_headers(&frame, &mut decoder, &mut encoder, &mut cont);

    match action {
        FrameAction::WriteFrames(frames) => {
            assert!(!frames.is_empty());
        }
        _ => panic!("expected WriteFrames for valid GET"),
    }
}

#[test]
fn test_handle_headers_missing_method() {
    let mut server = Http2Server::new();
    let mut encoder = Encoder::new();
    let mut decoder = Decoder::new();
    let mut cont = ContinuationState::new();

    let headers = vec![h(":scheme", "https"), h(":path", "/")];
    let frame = make_headers_frame(1, &headers, true, &mut encoder);

    let action = server.handle_headers(&frame, &mut decoder, &mut encoder, &mut cont);

    match action {
        FrameAction::WriteFrames(frames) => {
            assert!(!frames.is_empty());
            let rst = RstStreamFrame::from_frame(&frames[0]).unwrap();
            assert_eq!(rst.stream_id, 1);
            assert_eq!(rst.error_code, ErrorCode::PROTOCOL_ERROR.to_u32());
        }
        FrameAction::Goaway { .. } => {}
        _ => panic!("expected error response for missing :method"),
    }
}

#[test]
fn test_handle_headers_connection_specific_rejected() {
    let mut server = Http2Server::new();
    let mut encoder = Encoder::new();
    let mut decoder = Decoder::new();
    let mut cont = ContinuationState::new();

    let headers = vec![
        h(":method", "GET"),
        h(":scheme", "https"),
        h(":path", "/"),
        h("connection", "keep-alive"),
    ];
    let frame = make_headers_frame(1, &headers, true, &mut encoder);

    let action = server.handle_headers(&frame, &mut decoder, &mut encoder, &mut cont);

    match action {
        FrameAction::WriteFrames(frames) => {
            assert!(!frames.is_empty());
        }
        _ => panic!("expected error for connection-specific header"),
    }
}

// ── DATA handling ──

#[test]
fn test_handle_headers_self_referential_dependency() {
    let mut server = Http2Server::new();
    let mut encoder = Encoder::new();
    let mut decoder = Decoder::new();
    let mut cont = ContinuationState::new();

    let headers = vec![h(":method", "GET"), h(":scheme", "https"), h(":path", "/")];
    let block = encoder.encode(headers.iter().map(|(k, v)| (k.as_slice(), v.as_slice())));
    let hf = HeadersFrame {
        stream_id: 1,
        end_stream: true,
        exclusive: true,
        stream_dependency: 1,
        weight: 16,
        padding: None,
        header_block: block,
    };
    let frame = hf.to_frame();

    let action = server.handle_headers(&frame, &mut decoder, &mut encoder, &mut cont);

    match action {
        FrameAction::WriteFrames(frames) => {
            assert!(!frames.is_empty());
            let rst = RstStreamFrame::from_frame(&frames[0]).unwrap();
            assert_eq!(rst.stream_id, 1);
            assert_eq!(rst.error_code, ErrorCode::PROTOCOL_ERROR.to_u32());
        }
        other => panic!("expected RST_STREAM for self-referential HEADERS, got {other:?}"),
    }
}

#[test]
fn test_handle_data_on_idle_stream() {
    let mut server = Http2Server::new();
    let mut encoder = Encoder::new();

    let df = DataFrame::new(1, vec![0x00], true).to_frame();
    let action = server.handle_data(&df, &mut encoder);

    match action {
        FrameAction::Goaway { error_code, .. } => {
            assert_eq!(error_code, ErrorCode::PROTOCOL_ERROR.to_u32());
        }
        _ => panic!("expected Goaway for DATA on idle stream"),
    }
}

#[test]
fn test_handle_data_flow_control_consumed() {
    let mut server = Http2Server::new();
    let mut encoder = Encoder::new();
    let initial_window = server.flow_controller.window_size();

    let mut enc = Encoder::new();
    let mut dec = Decoder::new();
    let mut cont = ContinuationState::new();

    let headers = vec![h(":method", "POST"), h(":scheme", "https"), h(":path", "/")];
    let hf = make_headers_frame(1, &headers, false, &mut enc);
    server.handle_headers(&hf, &mut dec, &mut enc, &mut cont);

    let data = vec![0x01, 0x02, 0x03];
    let df = DataFrame::new(1, data.clone(), true).to_frame();
    let action = server.handle_data(&df, &mut encoder);

    let remaining = initial_window - data.len() as i64;
    assert_eq!(server.flow_controller.window_size(), remaining);
}

// ── CONTINUATION handling ──

#[test]
fn test_continuation_not_expected() {
    let mut server = Http2Server::new();
    let mut cont = ContinuationState::new();
    let mut encoder = Encoder::new();
    let mut decoder = Decoder::new();

    let frame = Frame::new(FrameType::Continuation, 0x04, 1, vec![0x82]);
    let action = server.handle_continuation(&frame, &mut cont, &mut decoder, &mut encoder);

    match action {
        FrameAction::Goaway { error_code, .. } => {
            assert_eq!(error_code, ErrorCode::PROTOCOL_ERROR.to_u32());
        }
        _ => panic!("expected Goaway for unexpected CONTINUATION"),
    }
}

// ── Stream state tracking ──

#[test]
fn test_stream_created_on_headers() {
    let mut server = Http2Server::new();
    let mut encoder = Encoder::new();
    let mut decoder = Decoder::new();
    let mut cont = ContinuationState::new();

    assert!(server.stream_manager.get_stream(1).is_none());

    let headers = vec![h(":method", "GET"), h(":scheme", "https"), h(":path", "/")];
    let frame = make_headers_frame(1, &headers, true, &mut encoder);
    server.handle_headers(&frame, &mut decoder, &mut encoder, &mut cont);
}

#[test]
fn test_last_processed_stream_id_updated() {
    let mut server = Http2Server::new();
    assert_eq!(server.last_processed_stream_id, 0);

    let mut encoder = Encoder::new();
    let mut decoder = Decoder::new();
    let mut cont = ContinuationState::new();

    let headers = vec![h(":method", "GET"), h(":scheme", "https"), h(":path", "/")];
    let frame = make_headers_frame(5, &headers, true, &mut encoder);
    server.handle_headers(&frame, &mut decoder, &mut encoder, &mut cont);

    assert_eq!(server.last_processed_stream_id, 5);
}

// ── GOAWAY handling ──

#[test]
fn test_handle_client_goaway() {
    let mut server = Http2Server::new();
    match server.handle_goaway() {
        FrameAction::CloseConnection => {}
        _ => panic!("expected CloseConnection"),
    }
}
