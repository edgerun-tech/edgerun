//! DATA frame handler for the HTTP/2 server.

use super::FrameAction;
use super::Http2Server;
use super::response;
use crate::http::http2::ErrorCode;
use crate::http::http2::frame::{DataFrame, Frame};
use crate::http::http2::hpack::Encoder;
use alloc::vec;

impl Http2Server {
    /// Process an incoming DATA frame.
    pub fn handle_data(&mut self, frame: &Frame, encoder: &mut Encoder) -> FrameAction {
        let df = match DataFrame::from_frame(frame) {
            Ok(df) => df,
            Err(_) => {
                self.goaway_sent = true;
                return response::send_goaway(
                    self.last_processed_stream_id,
                    ErrorCode::ProtocolError.to_u32(),
                    b"Invalid DATA payload",
                );
            }
        };

        let sid = df.stream_id;
        self.update_last_stream(sid);

        let state_before = self.stream_manager.get_stream(sid).map(|s| s.state);
        match state_before.unwrap_or(crate::http::http2::stream::StreamState::Idle) {
            crate::http::http2::stream::StreamState::Idle
            | crate::http::http2::stream::StreamState::ReservedRemote => {
                self.goaway_sent = true;
                return response::send_goaway(
                    self.last_processed_stream_id,
                    ErrorCode::ProtocolError.to_u32(),
                    b"DATA on idle stream",
                );
            }
            crate::http::http2::stream::StreamState::HalfClosedRemote
            | crate::http::http2::stream::StreamState::Closed => {
                return response::rst_stream(
                    sid,
                    ErrorCode::StreamClosed.to_u32(),
                    &mut self.stream_manager,
                );
            }
            crate::http::http2::stream::StreamState::HalfClosedLocal => {}
            _ => {}
        }

        if self.stream_manager.get_or_create_stream(sid).is_err() {
            self.goaway_sent = true;
            return response::send_goaway(
                self.last_processed_stream_id,
                ErrorCode::StreamClosed.to_u32(),
                b"Cannot create stream for DATA",
            );
        }

        if let Some(s) = self.stream_manager.get_stream_mut(sid) {
            if s.state == crate::http::http2::stream::StreamState::Idle {
                let _ = s.open();
            }
        }

        let data_len = df.data.len() as u32;
        if self.flow_controller.consume(data_len).is_err() {
            self.goaway_sent = true;
            return response::send_goaway(
                self.last_processed_stream_id,
                ErrorCode::FlowControlError.to_u32(),
                b"Flow control window exceeded",
            );
        }

        if let Some(s) = self.stream_manager.get_stream(sid) {
            if let Some(expected_cl) = s.content_length {
                let bytes_so_far = s.bytes_received + data_len as u64;
                if df.end_stream && bytes_so_far != expected_cl {
                    return response::rst_stream(
                        sid,
                        ErrorCode::ProtocolError.to_u32(),
                        &mut self.stream_manager,
                    );
                }
                if bytes_so_far > expected_cl {
                    return response::rst_stream(
                        sid,
                        ErrorCode::ProtocolError.to_u32(),
                        &mut self.stream_manager,
                    );
                }
            }
        }

        if let Some(s) = self.stream_manager.get_stream_mut(sid) {
            s.bytes_received += data_len as u64;
        }

        let mut actions = vec![response::send_window_update(sid, data_len).to_frame()];

        if df.end_stream {
            let _headers = self.pending_headers.remove(&sid).unwrap_or_default();
            match response::respond_with_200(sid, encoder) {
                FrameAction::WriteFrames(mut frames) => actions.append(&mut frames),
                FrameAction::Goaway { .. } => {
                    self.goaway_sent = true;
                    return response::send_goaway(
                        self.last_processed_stream_id,
                        ErrorCode::InternalError.to_u32(),
                        b"Response generation failed",
                    );
                }
                _ => {}
            }
            self.record_closed_stream(sid);
            self.half_close_remote_for_data(sid);
        }

        FrameAction::WriteFrames(actions)
    }

    fn half_close_remote_for_data(&mut self, stream_id: u32) {
        if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
            let _ = s.half_close_remote();
            // If this closes the stream, record it
            if s.state == crate::http::http2::stream::StreamState::Closed {
                self.record_closed_stream(stream_id);
            }
        }
    }
}
