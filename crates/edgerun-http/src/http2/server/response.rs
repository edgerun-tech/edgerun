//! Response helper methods for the HTTP/2 server.

use crate::http2::frame::{
    GoawayFrame, HeadersFrame, PingFrame, RstStreamFrame, SettingsFrame, WindowUpdateFrame,
};
use crate::http2::hpack::Encoder;
use crate::http2::stream::StreamManager;
use crate::http2::ErrorCode;
use super::FrameAction;

/// Send a 200 OK response.
pub fn respond_with_200(stream_id: u32, encoder: &mut Encoder) -> FrameAction {
    let response_headers = vec![
        (b":status".to_vec(), b"200".to_vec()),
        (b"content-type".to_vec(), b"text/plain".to_vec()),
        (b"content-length".to_vec(), b"2".to_vec()),
    ];

    let header_block = encoder.encode(
        response_headers
            .iter()
            .map(|(k, v)| (k.as_slice(), v.as_slice())),
    );

    let hf = HeadersFrame::new(stream_id, header_block, true);
    FrameAction::WriteFrames(vec![hf.to_frame()])
}

/// Send a RST_STREAM frame for the given stream.
pub fn rst_stream(
    stream_id: u32,
    error_code: u32,
    stream_manager: &mut StreamManager,
) -> FrameAction {
    let rst = RstStreamFrame::new(stream_id, error_code);
    let frames = vec![rst.to_frame()];
    if let Some(s) = stream_manager.get_stream_mut(stream_id) {
        s.close();
    }
    FrameAction::WriteFrames(frames)
}

/// Send SETTINGS ACK.
pub fn send_settings_ack() -> FrameAction {
    FrameAction::WriteFrames(vec![SettingsFrame::ack().to_frame()])
}

/// Send PING ACK with the given data.
pub fn send_ping_ack(data: [u8; 8]) -> FrameAction {
    let ack = PingFrame::ack(data);
    FrameAction::WriteFrames(vec![ack.to_frame()])
}

/// Send a GOAWAY frame.
pub fn send_goaway(last_stream_id: u32, error_code: u32, debug: &[u8]) -> FrameAction {
    let goaway = GoawayFrame::new(last_stream_id, error_code, debug.to_vec());
    FrameAction::Goaway {
        last_stream_id,
        error_code,
        debug_data: debug.to_vec(),
    }
}

/// Send WINDOW_UPDATE for consumed bytes.
pub fn send_window_update(stream_id: u32, increment: u32) -> WindowUpdateFrame {
    WindowUpdateFrame::new(stream_id, increment)
}

/// Half-close a stream's remote side.
pub fn half_close_remote(stream_id: u32, stream_manager: &mut StreamManager) {
    if let Some(s) = stream_manager.get_stream_mut(stream_id) {
        let _ = s.half_close_remote();
    }
}
