//! HEADERS and CONTINUATION frame handlers for the HTTP/2 server.

use super::frame::{Frame, HeadersFrame, RstStreamFrame};
use super::headers::{validate_header_name_case, validate_request_headers};
use super::hpack::{Decoder, Encoder};
use super::server::continuation::ContinuationState;
use super::server::response;
use super::stream::StreamState;
use super::ErrorCode;
use super::FrameAction;
use super::Http2Server;

impl Http2Server {
    /// Process an incoming HEADERS frame.
    /// Uses the provided HPACK decoder (which maintains state across frames).
    pub fn handle_headers(
        &mut self,
        frame: &Frame,
        decoder: &mut Decoder,
        encoder: &mut Encoder,
        cont: &mut ContinuationState,
    ) -> FrameAction {
        let hf = match HeadersFrame::from_frame(frame) {
            Ok(hf) => hf,
            Err(_) => {
                self.goaway_sent = true;
                return response::send_goaway(
                    self.last_processed_stream_id,
                    ErrorCode::PROTOCOL_ERROR.to_u32(),
                    b"Invalid HEADERS payload",
                );
            }
        };

        let stream_id = hf.stream_id;

        if stream_id == 0 {
            self.goaway_sent = true;
            return response::send_goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"HEADERS on stream 0",
            );
        }
        if stream_id % 2 == 0 {
            self.goaway_sent = true;
            return response::send_goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"Client sent even stream ID",
            );
        }
        if stream_id < self.last_processed_stream_id
            && self.stream_manager.get_stream(stream_id).is_none()
        {
            self.goaway_sent = true;
            return response::send_goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"Decreasing stream ID",
            );
        }

        let state_before = self.stream_manager.get_stream(stream_id).map(|s| s.state);
        let effective_state = match state_before {
            Some(s) => s,
            None if stream_id <= self.last_processed_stream_id => StreamState::Closed,
            None => StreamState::Idle,
        };
        match effective_state {
            StreamState::Closed => {
                self.update_last_stream(stream_id);
                self.goaway_sent = true;
                return response::send_goaway(
                    self.last_processed_stream_id,
                    ErrorCode::STREAM_CLOSED.to_u32(),
                    b"HEADERS on closed stream",
                );
            }
            StreamState::HalfClosedLocal => {
                return response::rst_stream(
                    stream_id,
                    ErrorCode::STREAM_CLOSED.to_u32(),
                    &mut self.stream_manager,
                );
            }
            _ => {}
        }

        self.update_last_stream(stream_id);

        if self.stream_manager.get_or_create_stream(stream_id).is_err() {
            self.goaway_sent = true;
            return response::send_goaway(
                self.last_processed_stream_id,
                ErrorCode::STREAM_CLOSED.to_u32(),
                b"Cannot create stream",
            );
        }

        if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
            if s.state == StreamState::Idle {
                let _ = s.open();
            }
        }

        let end_headers = frame.flags & super::frame::flags::HEADERS_END_HEADERS != 0;

        if hf.exclusive && hf.stream_dependency == stream_id {
            return response::rst_stream(
                stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                &mut self.stream_manager,
            );
        }

        if state_before == Some(StreamState::HalfClosedRemote) && hf.end_stream && end_headers {
            let _headers = match decoder.decode(&hf.header_block) {
                Ok(h) => h,
                Err(_) => {
                    self.goaway_sent = true;
                    return response::send_goaway(
                        self.last_processed_stream_id,
                        ErrorCode::COMPRESSION_ERROR.to_u32(),
                        b"HPACK decode error in trailers",
                    );
                }
            };
            if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
                let _ = s.half_close_remote();
            }
            return FrameAction::None;
        }

        if hf.end_stream && end_headers {
            return self.process_complete_headers(&hf.header_block, stream_id, decoder, encoder);
        } else if end_headers && !hf.end_stream {
            let headers = match decoder.decode(&hf.header_block) {
                Ok(h) => h,
                Err(_) => {
                    self.goaway_sent = true;
                    return response::send_goaway(
                        self.last_processed_stream_id,
                        ErrorCode::COMPRESSION_ERROR.to_u32(),
                        b"HPACK decode error",
                    );
                }
            };
            if let Err((ec, _)) = validate_request_headers(&headers) {
                return response::rst_stream(stream_id, ec, &mut self.stream_manager);
            }
            if let Err((ec, _)) = validate_header_name_case(&headers) {
                return response::rst_stream(stream_id, ec, &mut self.stream_manager);
            }

            let content_length = headers
                .iter()
                .find(|(k, _)| k == b"content-length")
                .and_then(|(_, v)| std::str::from_utf8(v).ok()?.parse::<u64>().ok());

            if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
                s.content_length = content_length;
            }

            self.pending_headers.insert(stream_id, headers);
        } else if !end_headers {
            cont.start(stream_id, &hf.header_block);
            if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
                if s.state == StreamState::Idle {
                    let _ = s.open();
                }
            }
        }

        FrameAction::None
    }

    /// Process an incoming CONTINUATION frame.
    pub fn handle_continuation(
        &mut self,
        frame: &Frame,
        cont: &mut ContinuationState,
        decoder: &mut Decoder,
        encoder: &mut Encoder,
    ) -> FrameAction {
        if !cont.expecting {
            self.goaway_sent = true;
            return response::send_goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"CONTINUATION not expected",
            );
        }

        if frame.stream_id != cont.stream_id {
            self.goaway_sent = true;
            return response::send_goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"CONTINUATION on wrong stream",
            );
        }

        cont.header_block_buf.extend_from_slice(&frame.payload);

        let end_headers = frame.flags & super::frame::flags::HEADERS_END_HEADERS != 0;

        if end_headers {
            let sid = cont.stream_id;
            self.update_last_stream(sid);

            if self.stream_manager.get_or_create_stream(sid).is_err() {
                cont.abort();
                self.goaway_sent = true;
                return response::send_goaway(
                    self.last_processed_stream_id,
                    ErrorCode::STREAM_CLOSED.to_u32(),
                    b"Cannot create stream for CONTINUATION",
                );
            }

            let headers = match decoder.decode(&cont.header_block_buf) {
                Ok(h) => h,
                Err(_) => {
                    cont.abort();
                    self.goaway_sent = true;
                    return response::send_goaway(
                        self.last_processed_stream_id,
                        ErrorCode::COMPRESSION_ERROR.to_u32(),
                        b"HPACK decode error in CONTINUATION",
                    );
                }
            };

            if let Err((ec, _)) = validate_request_headers(&headers) {
                cont.abort();
                return response::rst_stream(sid, ec, &mut self.stream_manager);
            }
            if let Err((ec, _)) = validate_header_name_case(&headers) {
                cont.abort();
                return response::rst_stream(sid, ec, &mut self.stream_manager);
            }

            let action = response::respond_with_200(sid, encoder);
            self.half_close_remote(sid);
            cont.finish();
            return action;
        }

        FrameAction::None
    }

    fn process_complete_headers(
        &mut self,
        header_block: &[u8],
        stream_id: u32,
        decoder: &mut Decoder,
        encoder: &mut Encoder,
    ) -> FrameAction {
        let headers = match decoder.decode(header_block) {
            Ok(h) => h,
            Err(_) => {
                self.goaway_sent = true;
                return response::send_goaway(
                    self.last_processed_stream_id,
                    ErrorCode::COMPRESSION_ERROR.to_u32(),
                    b"HPACK decode error",
                );
            }
        };

        if let Err((ec, _)) = validate_request_headers(&headers) {
            return response::rst_stream(stream_id, ec, &mut self.stream_manager);
        }
        if let Err((ec, _)) = validate_header_name_case(&headers) {
            return response::rst_stream(stream_id, ec, &mut self.stream_manager);
        }

        let content_length = headers
            .iter()
            .find(|(k, _)| k == b"content-length")
            .and_then(|(_, v)| std::str::from_utf8(v).ok()?.parse::<u64>().ok());

        if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
            s.content_length = content_length;
            if s.state == StreamState::Idle {
                let _ = s.open();
            }
        }

        let action = response::respond_with_200(stream_id, encoder);
        if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
            let _ = s.half_close_local();
        }
        self.half_close_remote(stream_id);
        action
    }

    fn half_close_remote(&mut self, stream_id: u32) {
        if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
            let _ = s.half_close_remote();
        }
    }
}
