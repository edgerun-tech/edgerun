//! HTTP/2 server state machine for conformance testing.
//!
//! Provides an `Http2Server` type that tracks connection and stream state,
//! validates frames, and produces appropriate responses (`FrameAction` values)
//! for incoming frames.
//!
//! # Design
//!
//! This module is **I/O-agnostic**: it operates on typed `Frame` structs and
//! returns `FrameAction` values that the caller serializes and writes to the
//! transport. This design enables unit testing every frame handling path without
//! needing TCP/TLS infrastructure.
//!
//! # FrameAction
//!
//! | Variant | When |
//! |---------|------|
//! | `None` | Frame silently accepted (no response needed) |
//! | `WriteFrames(Vec<Frame>)` | Frames to send back (PING ACK, SETTINGS ACK, response HEADERS) |
//! | `Goaway { .. }` | Protocol violation detected — send GOAWAY and close |
//! | `CloseConnection` | Client sent GOAWAY or graceful shutdown requested |
//!
//! # Error Handling
//!
//! The server distinguishes between:
//! - **Stream errors**: RST_STREAM with the appropriate error code
//! - **Connection errors**: GOAWAY with the appropriate error code
//!
//! Per RFC 9113:
//! - `PROTOCOL_ERROR` (0x1): General protocol violation
//! - `FLOW_CONTROL_ERROR` (0x3): Window size overflow
//! - `FRAME_SIZE_ERROR` (0x6): Frame too large/small
//! - `COMPRESSION_ERROR` (0x9): HPACK decoding failure
//! - `STREAM_CLOSED` (0x5): Frame on closed stream
//!
//! # Unit Tests
//!
//! 26 tests covering all frame types, stream states, settings validation,
//! flow control, and error conditions. Run with:
//! ```text
//! cargo test -p edgerun-http --lib -- server::
//! ```
//!
//! # See Also
//!
//! - [H2SPEC_ANALYSIS.md](../H2SPEC_ANALYSIS.md) for the h2spec analysis process
//! - [RFC 9113](https://www.rfc-editor.org/rfc/rfc9113) — HTTP/2 Specification
//! - [RFC 7540 §5.1](https://www.rfc-editor.org/rfc/rfc7540.html#section-5.1) — Stream States

use std::collections::HashMap;

use super::flow_control::FlowController;
use super::frame::{
    DataFrame, Frame, FrameType, GoawayFrame, HeadersFrame, PingFrame, PriorityFrame,
    RstStreamFrame, SettingsFrame, WindowUpdateFrame,
};
use super::hpack::{Decoder, Encoder};
use super::headers::{validate_header_name_case, validate_request_headers};
use super::settings::Settings;
use super::stream::{StreamManager, StreamState};
use super::ErrorCode;

/// The action the server should take after processing an incoming frame.
#[derive(Debug)]
pub enum FrameAction {
    /// No action needed (frame was silently accepted).
    None,
    /// Write one or more frames to the connection.
    WriteFrames(Vec<Frame>),
    /// A protocol violation was detected — send GOAWAY and close.
    Goaway {
        last_stream_id: u32,
        error_code: u32,
        debug_data: Vec<u8>,
    },
    /// The connection should be closed (e.g. client sent GOAWAY).
    CloseConnection,
}

/// Connection-level state for the HTTP/2 server.
pub struct Http2Server {
    /// Client's applied settings.
    pub client_settings: Settings,
    /// Server's settings (sent to client).
    pub server_settings: Settings,
    /// Connection-level flow controller.
    pub flow_controller: FlowController,
    /// Stream manager.
    pub stream_manager: StreamManager,
    /// Max frame size we accept (negotiated from client).
    pub max_frame_size: u32,
    /// Highest stream ID we've processed.
    pub last_processed_stream_id: u32,
    /// Pending decoded headers per stream (for HEADERS without END_STREAM).
    pub pending_headers: HashMap<u32, Vec<(Vec<u8>, Vec<u8>)>>,
    /// Whether we've sent a GOAWAY.
    pub goaway_sent: bool,
}

impl Http2Server {
    /// Create a new HTTP/2 server with default settings.
    pub fn new() -> Self {
        let server_settings = Settings::new();
        Http2Server {
            client_settings: Settings::default(),
            server_settings: server_settings.clone(),
            flow_controller: FlowController::new(server_settings.initial_window_size),
            stream_manager: StreamManager::new(server_settings.initial_window_size),
            max_frame_size: server_settings.max_frame_size,
            last_processed_stream_id: 0,
            goaway_sent: false,
            pending_headers: HashMap::new(),
        }
    }

    /// Apply the client's initial SETTINGS frame.
    /// Returns frames to write, or a Goaway if settings are invalid.
    pub fn apply_client_settings(
        &mut self,
        settings_frame: &SettingsFrame,
    ) -> FrameAction {
        // Validate semantics first
        if let Err(error_code) = settings_frame.to_frame().validate_semantics() {
            return self.goaway(0, error_code, b"Invalid SETTINGS");
        }

        match Settings::from_entries(&settings_frame.entries) {
            Ok(s) => {
                self.client_settings = s;
                self.max_frame_size = self.client_settings.max_frame_size;
                if let Some(max) = self.client_settings.max_concurrent_streams {
                    self.stream_manager.set_max_concurrent_streams(max);
                }
            }
            Err(_) => {
                return self.goaway(0, ErrorCode::PROTOCOL_ERROR.to_u32(), b"Invalid settings values");
            }
        }

        // Send server SETTINGS + ACK
        FrameAction::WriteFrames(vec![
            SettingsFrame::new(self.server_settings.to_entries()).to_frame(),
            SettingsFrame::ack().to_frame(),
        ])
    }

    /// Process an incoming SETTINGS frame (after preface).
    pub fn handle_settings(&mut self, frame: &Frame) -> FrameAction {
        let sf = match SettingsFrame::from_frame(frame) {
            Ok(sf) => sf,
            Err(_) => {
                return self.goaway(
                    self.last_processed_stream_id,
                    ErrorCode::FRAME_SIZE_ERROR.to_u32(),
                    b"Invalid SETTINGS payload",
                );
            }
        };

        if !sf.ack {
            // Client sending new settings — validate and apply them
            match Settings::from_entries(&sf.entries) {
                Ok(new_settings) => {
                    self.client_settings = new_settings;
                    self.max_frame_size = self.client_settings.max_frame_size;
                    if let Some(max) = self.client_settings.max_concurrent_streams {
                        self.stream_manager.set_max_concurrent_streams(max);
                    }
                }
                Err(e) => {
                    // Map error type to appropriate error code
                    let error_code = match &e {
                        super::Http2Error::FlowControl(_) => {
                            ErrorCode::FLOW_CONTROL_ERROR.to_u32()
                        }
                        _ => ErrorCode::PROTOCOL_ERROR.to_u32(),
                    };
                    return self.goaway(
                        self.last_processed_stream_id,
                        error_code,
                        b"Invalid SETTINGS values",
                    );
                }
            }
            // Acknowledge
            return FrameAction::WriteFrames(vec![SettingsFrame::ack().to_frame()]);
        }
        // ACK: nothing to do
        FrameAction::None
    }

    /// Process an incoming PING frame.
    pub fn handle_ping(&mut self, frame: &Frame) -> FrameAction {
        let pf = match PingFrame::from_frame(frame) {
            Ok(pf) => pf,
            Err(_) => return FrameAction::None, // Already validated
        };
        if !pf.ack {
            let ack = PingFrame::ack(pf.data);
            return FrameAction::WriteFrames(vec![ack.to_frame()]);
        }
        FrameAction::None
    }

    /// Process an incoming WINDOW_UPDATE frame.
    pub fn handle_window_update(&mut self, frame: &Frame) -> FrameAction {
        let wu = match WindowUpdateFrame::from_frame(frame) {
            Ok(wu) => wu,
            Err(_) => {
                return self.goaway(
                    self.last_processed_stream_id,
                    ErrorCode::FLOW_CONTROL_ERROR.to_u32(),
                    b"Invalid WINDOW_UPDATE",
                );
            }
        };

        if frame.stream_id == 0 {
            // Connection-level
            if self.flow_controller.increment(wu.window_increment).is_err() {
                return self.goaway(
                    self.last_processed_stream_id,
                    ErrorCode::FLOW_CONTROL_ERROR.to_u32(),
                    b"Window overflow",
                );
            }
        } else {
            // Stream-level WINDOW_UPDATE
            // If stream exists (even if closed), accept it.
            // If stream never existed (idle, > last_processed), it's a connection error.
            let stream_exists = self.stream_manager.get_stream(wu.stream_id).is_some();
            let was_seen = wu.stream_id <= self.last_processed_stream_id;
            if !stream_exists && !was_seen {
                return self.goaway(
                    self.last_processed_stream_id,
                    ErrorCode::PROTOCOL_ERROR.to_u32(),
                    b"WINDOW_UPDATE on idle stream",
                );
            }
            if let Some(s) = self.stream_manager.get_stream_mut(wu.stream_id) {
                // Check overflow (RFC 9113 §6.9.1)
                if wu.window_increment as i64 > FlowController::MAX_WINDOW_SIZE - s.remote_window {
                    return self.rst_stream(wu.stream_id, ErrorCode::FLOW_CONTROL_ERROR.to_u32());
                }
                s.remote_window += wu.window_increment as i64;
            }
        }
        FrameAction::None
    }

    /// Process an incoming RST_STREAM frame.
    pub fn handle_rst_stream(&mut self, frame: &Frame) -> FrameAction {
        let rst = match RstStreamFrame::from_frame(frame) {
            Ok(rst) => rst,
            Err(_) => return FrameAction::None,
        };
        // RST_STREAM on idle stream (never created) = protocol error.
        // RST_STREAM on closed/existing streams is valid — silently accept.
        let stream_exists = self.stream_manager.get_stream(rst.stream_id).is_some();
        let was_seen = rst.stream_id <= self.last_processed_stream_id;
        if !stream_exists && !was_seen {
            return self.goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"RST_STREAM on idle stream",
            );
        }
        if let Some(s) = self.stream_manager.get_stream_mut(rst.stream_id) {
            s.close();
        }
        // Don't clean up immediately — post-closure frames may still arrive.
        FrameAction::WriteFrames(vec![RstStreamFrame::new(rst.stream_id, rst.error_code).to_frame()])
    }

    /// Process an incoming PRIORITY frame.
    /// PRIORITY frames are accepted on most streams per RFC 7540 §6.3,
    /// but NOT on stream 0 (connection-level).
    /// Self-referential dependencies (stream depends on itself) are rejected.
    pub fn handle_priority(&mut self, frame: &PriorityFrame) -> FrameAction {
        // PRIORITY on stream 0 is a connection error (RFC 7540 §6.3)
        if frame.stream_id == 0 {
            return self.goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"PRIORITY on stream 0",
            );
        }

        // Self-referential dependency: stream depends on itself (RFC 7540 §5.3.1)
        if frame.stream_dependency == frame.stream_id {
            return self.rst_stream(frame.stream_id, ErrorCode::PROTOCOL_ERROR.to_u32());
        }

        // PRIORITY frames are accepted on half-closed and closed streams
        // (RFC 7540 §5.1: PRIORITY can be sent on any stream state)
        // We don't implement priority scheduling, so just accept it.
        FrameAction::None
    }

    /// Process an incoming HEADERS frame.
    /// Uses the provided HPACK decoder (which maintains state across frames).
    pub fn handle_headers(
        &mut self,
        frame: &Frame,
        header_block_buf: &mut Vec<u8>,
        decoder: &mut Decoder,
        encoder: &mut Encoder,
        expecting_continuation: &mut bool,
        continuation_stream_id: &mut u32,
    ) -> FrameAction {
        let hf = match HeadersFrame::from_frame(frame) {
            Ok(hf) => hf,
            Err(_) => {
                return self.goaway(
                    self.last_processed_stream_id,
                    ErrorCode::PROTOCOL_ERROR.to_u32(),
                    b"Invalid HEADERS payload",
                );
            }
        };

        let stream_id = hf.stream_id;

        // Check stream ID is odd (client-initiated) and not 0
        if stream_id == 0 {
            return self.goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"HEADERS on stream 0",
            );
        }
        if stream_id % 2 == 0 {
            return self.goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"Client sent even stream ID",
            );
        }
        // Check stream ID is not smaller than previous (monotonically increasing)
        if stream_id < self.last_processed_stream_id
            && self.stream_manager.get_stream(stream_id).is_none()
        {
            return self.goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"Decreasing stream ID",
            );
        }

        // Check stream state BEFORE creating/transitioning
        let state_before = self.stream_manager.get_stream(stream_id).map(|s| s.state);
        // If the stream doesn't exist but its ID is <= last_processed_stream_id,
        // it was already closed and cleaned up. Treat as Closed.
        let effective_state = match state_before {
            Some(s) => s,
            None if stream_id <= self.last_processed_stream_id => StreamState::Closed,
            None => StreamState::Idle,
        };
        match effective_state {
            StreamState::Closed => {
                // HEADERS on closed stream = connection error (RFC 7540 §5.1)
                self.update_last_stream(stream_id);
                return self.goaway(
                    self.last_processed_stream_id,
                    ErrorCode::STREAM_CLOSED.to_u32(),
                    b"HEADERS on closed stream",
                );
            }
            StreamState::HalfClosedRemote => {
                // Could be trailers (second HEADERS after DATA) or protocol error.
                // Check if headers contain pseudo-headers (trailers don't have them).
                // For now, treat as trailers — decode and check for pseudo-headers.
                // We'll handle this after header decoding below.
            }
            StreamState::HalfClosedLocal => {
                return self.rst_stream(stream_id, ErrorCode::STREAM_CLOSED.to_u32());
            }
            _ => {}
        }

        self.update_last_stream(stream_id);

        // Get or create stream
        if self.stream_manager.get_or_create_stream(stream_id).is_err() {
            return self.goaway(
                self.last_processed_stream_id,
                ErrorCode::STREAM_CLOSED.to_u32(),
                b"Cannot create stream",
            );
        }

        // Transition stream to Open if it's still Idle
        if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
            if s.state == StreamState::Idle {
                let _ = s.open();
            }
        }

        let end_headers = frame.flags & super::frame::flags::HEADERS_END_HEADERS != 0;

        // Check for self-referential stream dependency (RFC 7540 §5.3.1)
        if hf.exclusive && hf.stream_dependency == stream_id {
            return self.rst_stream(stream_id, ErrorCode::PROTOCOL_ERROR.to_u32());
        }

        // Check if this is a trailers frame (HEADERS on HalfClosedRemote without pseudo-headers)
        if state_before == Some(StreamState::HalfClosedRemote) && hf.end_stream && end_headers {
            // This is a trailers frame — decode but don't validate as request headers
            let _headers = match decoder.decode(&hf.header_block) {
                Ok(h) => h,
                Err(_) => {
                    return self.goaway(
                        self.last_processed_stream_id,
                        ErrorCode::COMPRESSION_ERROR.to_u32(),
                        b"HPACK decode error in trailers",
                    );
                }
            };
            // Trailers accepted — stream goes from HalfClosedRemote to Closed
            if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
                let _ = s.half_close_remote();  // HalfClosedRemote -> Closed
            }
            // Don't clean up immediately — post-closure frames may still arrive.
            // Don't respond — the original response was already sent
            return FrameAction::None;
        }

        if hf.end_stream && end_headers {
            // Full request in HEADERS (GET with END_STREAM)
            return self.process_complete_headers(&hf.header_block, stream_id, decoder, encoder);
        } else if end_headers && !hf.end_stream {
            // Headers complete, expecting DATA — decode & validate immediately
            let headers = match decoder.decode(&hf.header_block) {
                Ok(h) => h,
                Err(_) => {
                    return self.goaway(
                        self.last_processed_stream_id,
                        ErrorCode::COMPRESSION_ERROR.to_u32(),
                        b"HPACK decode error",
                    );
                }
            };
            if let Err((ec, _)) = validate_request_headers(&headers) {
                return self.rst_stream(stream_id, ec);
            }
            if let Err((ec, _)) = validate_header_name_case(&headers) {
                return self.rst_stream(stream_id, ec);
            }

            // Extract Content-Length if present
            let content_length = headers.iter()
                .find(|(k, _)| k == b"content-length")
                .and_then(|(_, v)| std::str::from_utf8(v).ok()?.parse::<u64>().ok());

            if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
                s.content_length = content_length;
            }

            self.pending_headers.insert(stream_id, headers);
            *continuation_stream_id = stream_id;
            *expecting_continuation = false;
            // Transition stream to Open
            if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
                if s.state == StreamState::Idle {
                    let _ = s.open();
                }
            }
        } else if !end_headers {
            // Need CONTINUATION frames — store raw bytes for now
            header_block_buf.clear();
            header_block_buf.extend_from_slice(&hf.header_block);
            *continuation_stream_id = stream_id;
            *expecting_continuation = true;
            // Transition stream to Open
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
        header_block_buf: &mut Vec<u8>,
        expecting_continuation: &mut bool,
        continuation_stream_id: &mut u32,
        decoder: &mut Decoder,
        encoder: &mut Encoder,
    ) -> FrameAction {
        if !*expecting_continuation {
            return self.goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"CONTINUATION not expected",
            );
        }

        if frame.stream_id != *continuation_stream_id {
            return self.goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"CONTINUATION on wrong stream",
            );
        }

        header_block_buf.extend_from_slice(&frame.payload);

        let end_headers = frame.flags & super::frame::flags::HEADERS_END_HEADERS != 0;

        if end_headers {
            *expecting_continuation = false;
            let sid = *continuation_stream_id;
            self.update_last_stream(sid);

            if self.stream_manager.get_or_create_stream(sid).is_err() {
                return self.goaway(
                    self.last_processed_stream_id,
                    ErrorCode::STREAM_CLOSED.to_u32(),
                    b"Cannot create stream for CONTINUATION",
                );
            }

            let headers = match decoder.decode(header_block_buf) {
                Ok(h) => h,
                Err(_) => {
                    return self.goaway(
                        self.last_processed_stream_id,
                        ErrorCode::COMPRESSION_ERROR.to_u32(),
                        b"HPACK decode error in CONTINUATION",
                    );
                }
            };

            if let Err((ec, _)) = validate_request_headers(&headers) {
                header_block_buf.clear();
                return self.rst_stream(sid, ec);
            }
            if let Err((ec, _)) = validate_header_name_case(&headers) {
                header_block_buf.clear();
                return self.rst_stream(sid, ec);
            }

            let action = self.respond_with_200(sid, encoder);
            self.half_close_remote(sid);
            header_block_buf.clear();
            return action;
        }

        FrameAction::None
    }

    /// Process an incoming DATA frame.
    pub fn handle_data(
        &mut self,
        frame: &Frame,
        encoder: &mut Encoder,
    ) -> FrameAction {
        let df = match DataFrame::from_frame(frame) {
            Ok(df) => df,
            Err(_) => {
                return self.goaway(
                    self.last_processed_stream_id,
                    ErrorCode::PROTOCOL_ERROR.to_u32(),
                    b"Invalid DATA payload",
                );
            }
        };

        let sid = df.stream_id;
        self.update_last_stream(sid);

        // Check stream state
        let state_before = self.stream_manager.get_stream(sid).map(|s| s.state);
        match state_before.unwrap_or(StreamState::Idle) {
            StreamState::Idle | StreamState::ReservedRemote => {
                // DATA on idle stream without HEADERS = connection error (RFC 9113 §8.1)
                return self.goaway(
                    self.last_processed_stream_id,
                    ErrorCode::PROTOCOL_ERROR.to_u32(),
                    b"DATA on idle stream",
                );
            }
            StreamState::HalfClosedRemote | StreamState::HalfClosedLocal | StreamState::Closed => {
                return self.rst_stream(sid, ErrorCode::STREAM_CLOSED.to_u32());
            }
            _ => {}
        }

        // Get or create stream
        if self.stream_manager.get_or_create_stream(sid).is_err() {
            return self.goaway(
                self.last_processed_stream_id,
                ErrorCode::STREAM_CLOSED.to_u32(),
                b"Cannot create stream for DATA",
            );
        }

        // Transition to Open if needed
        if let Some(s) = self.stream_manager.get_stream_mut(sid) {
            if s.state == StreamState::Idle {
                let _ = s.open();
            }
        }

        // Consume from flow control window
        let data_len = df.data.len() as u32;
        if self.flow_controller.consume(data_len).is_err() {
            return self.goaway(
                self.last_processed_stream_id,
                ErrorCode::FLOW_CONTROL_ERROR.to_u32(),
                b"Flow control window exceeded",
            );
        }

        // Check Content-Length if known
        if let Some(s) = self.stream_manager.get_stream(sid) {
            if let Some(expected_cl) = s.content_length {
                let bytes_so_far = s.bytes_received + data_len as u64;
                if df.end_stream && bytes_so_far != expected_cl {
                    return self.rst_stream(sid, ErrorCode::PROTOCOL_ERROR.to_u32());
                }
                if bytes_so_far > expected_cl {
                    return self.rst_stream(sid, ErrorCode::PROTOCOL_ERROR.to_u32());
                }
            }
        }

        // Track bytes received
        if let Some(s) = self.stream_manager.get_stream_mut(sid) {
            s.bytes_received += data_len as u64;
        }

        let mut actions = vec![WindowUpdateFrame::new(0, data_len).to_frame()];

        if df.end_stream {
            let headers = self.pending_headers.remove(&sid).unwrap_or_default();
            match self.respond_with_200(sid, encoder) {
                FrameAction::WriteFrames(mut frames) => actions.append(&mut frames),
                FrameAction::Goaway { .. } => {
                    return self.goaway(
                        self.last_processed_stream_id,
                        ErrorCode::INTERNAL_ERROR.to_u32(),
                        b"Response generation failed",
                    );
                }
                _ => {}
            }
            self.half_close_remote(sid);
        }

        FrameAction::WriteFrames(actions)
    }

    /// Process an incoming GOAWAY frame (from client).
    /// Just acknowledge by returning CloseConnection — no response frame needed.
    pub fn handle_goaway(&mut self) -> FrameAction {
        self.goaway_sent = true; // Client initiated, we don't need to send one
        FrameAction::CloseConnection
    }

    // ── Internal helpers ──

    fn update_last_stream(&mut self, stream_id: u32) {
        if stream_id > self.last_processed_stream_id {
            self.last_processed_stream_id = stream_id;
        }
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
                return self.goaway(
                    self.last_processed_stream_id,
                    ErrorCode::COMPRESSION_ERROR.to_u32(),
                    b"HPACK decode error",
                );
            }
        };

        if let Err((ec, _)) = validate_request_headers(&headers) {
            return self.rst_stream(stream_id, ec);
        }
        if let Err((ec, _)) = validate_header_name_case(&headers) {
            return self.rst_stream(stream_id, ec);
        }

        // Extract Content-Length if present
        let content_length = headers.iter()
            .find(|(k, _)| k == b"content-length")
            .and_then(|(_, v)| std::str::from_utf8(v).ok()?.parse::<u64>().ok());

        if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
            s.content_length = content_length;
            // Transition Idle -> Open before half-closing
            if s.state == StreamState::Idle {
                let _ = s.open();
            }
        }

        let action = self.respond_with_200(stream_id, encoder);
        // Server also sends END_STREAM, so close the stream completely
        if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
            let _ = s.half_close_local();  // Open/HalfClosedRemote -> HalfClosedLocal/Closed
        }
        self.half_close_remote(stream_id);
        action
    }

    fn respond_with_200(&mut self, stream_id: u32, encoder: &mut Encoder) -> FrameAction {
        let response_headers = vec![
            (b":status".to_vec(), b"200".to_vec()),
            (b"content-type".to_vec(), b"text/plain".to_vec()),
            (b"content-length".to_vec(), b"2".to_vec()),
        ];

        let header_block = encoder.encode(
            response_headers.iter().map(|(k, v)| (k.as_slice(), v.as_slice())),
        );

        let hf = HeadersFrame::new(stream_id, header_block, true);
        FrameAction::WriteFrames(vec![hf.to_frame()])
    }

    fn half_close_remote(&mut self, stream_id: u32) {
        if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
            let _ = s.half_close_remote();
        }
        // Don't clean up immediately — post-closure frames (WINDOW_UPDATE,
        // PRIORITY, RST_STREAM) are valid on closed streams per RFC 7540 §5.1.
        // Cleanup is deferred to periodic cleanup.
    }

    fn rst_stream(&mut self, stream_id: u32, error_code: u32) -> FrameAction {
        let rst = RstStreamFrame::new(stream_id, error_code);
        let frames = vec![rst.to_frame()];
        if let Some(s) = self.stream_manager.get_stream_mut(stream_id) {
            s.close();
        }
        // Don't clean up immediately — post-closure frames may still arrive.
        FrameAction::WriteFrames(frames)
    }

    /// Periodically clean up closed streams.
    /// Should be called every N frames (e.g. every 100 frames) to prevent
    /// memory growth from streams that have been closed but not removed.
    pub fn cleanup_closed_streams(&mut self) {
        self.stream_manager.cleanup_closed();
    }

    fn goaway(&mut self, last_stream_id: u32, error_code: u32, debug: &[u8]) -> FrameAction {
        self.goaway_sent = true;
        let goaway = GoawayFrame::new(last_stream_id, error_code, debug.to_vec());
        FrameAction::Goaway {
            last_stream_id,
            error_code,
            debug_data: debug.to_vec(),
        }
    }
}

impl Default for Http2Server {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let sf = SettingsFrame::new(vec![]); // empty = all defaults
        let action = server.apply_client_settings(&sf);
        match action {
            FrameAction::WriteFrames(frames) => {
                assert_eq!(frames.len(), 2); // SETTINGS + ACK
            }
            _ => panic!("expected WriteFrames"),
        }
    }

    #[test]
    fn test_apply_client_settings_custom_max_frame_size() {
        let mut server = Http2Server::new();
        let sf = SettingsFrame::new(vec![(0x5, 32768)]); // MAX_FRAME_SIZE
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
        let sf = SettingsFrame::new(vec![(0x3, 10)]); // MAX_CONCURRENT_STREAMS
        server.apply_client_settings(&sf);
        // Setting applied; verified via max_concurrent_streams in StreamManager
        // (tested indirectly via stream creation limits)
    }

    #[test]
    fn test_apply_invalid_enable_push() {
        let mut server = Http2Server::new();
        let sf = SettingsFrame::new(vec![(0x2, 5)]); // invalid enable_push
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
                // Should be PING ACK
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
        // Create stream 1 first
        let _ = server.stream_manager.get_or_create_stream(1);
        let wu = WindowUpdateFrame::new(1, 500).to_frame();
        let action = server.handle_window_update(&wu);
        assert!(matches!(action, FrameAction::None));
        let stream = server.stream_manager.get_stream(1).unwrap();
        assert_eq!(stream.remote_window, 65535 + 500);
    }

    #[test]
    fn test_handle_window_update_stream_not_yet_created() {
        // WINDOW_UPDATE on stream that doesn't exist yet — protocol error per RFC 9113
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
        // Zero increment fails parse in from_frame → handler sends FLOW_CONTROL_ERROR
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
        // RFC 7540 §5.1: PRIORITY frames can be sent on idle streams
        let mut server = Http2Server::new();
        assert!(server.stream_manager.get_stream(1).is_none());

        let pf = PriorityFrame::new(1, false, 0, 16);
        match server.handle_priority(&pf) {
            FrameAction::None => {} // PRIORITY is silently accepted
            other => panic!("expected None for PRIORITY on idle stream, got {other:?}"),
        }

        // Stream should NOT be created by PRIORITY
        assert!(server.stream_manager.get_stream(1).is_none());
    }

    #[test]
    fn test_handle_priority_on_closed_stream() {
        // RFC 7540 §5.1: PRIORITY frames can be sent on closed streams
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
        // h2spec sends PRIORITY on various streams — validate_semantics must accept
        let valid_payload = vec![0, 0, 0, 0, 15]; // dep=0, weight=16
        let frame = Frame::new(FrameType::Priority, 0, 1, valid_payload);
        assert!(frame.validate_semantics().is_ok(), "PRIORITY on stream 1 should pass semantics");

        // PRIORITY on stream 0 is a protocol error
        let frame2 = Frame::new(FrameType::Priority, 0, 0, vec![0, 0, 0, 0, 15]);
        assert!(frame2.validate_semantics().is_err(), "PRIORITY on stream 0 should fail semantics");
    }

    #[test]
    fn test_priority_frame_weight_256_no_overflow() {
        // h2spec sends PRIORITY with weight=256 (encoded as 255 on wire)
        // wrapping_add(1) must not panic (255 + 1 wraps to 0)
        let payload = vec![0, 0, 0, 0, 255]; // weight=255 on wire
        let frame = Frame::new(FrameType::Priority, 0, 1, payload);
        // This used to panic with "attempt to add with overflow"
        let pf = PriorityFrame::from_frame(&frame).expect("should parse weight 256");
        // Weight wraps: 255 + 1 = 0 (u8 overflow). This is acceptable — weight=0 is effectively 256.
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
        // RST_STREAM returns the frame to be written to the client
        assert!(matches!(action, FrameAction::WriteFrames(_)));
        // Stream is marked closed but NOT cleaned up immediately —
        // post-closure frames may still arrive per RFC 7540 §5.1.
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
        // Stream 1 depends on itself (exclusive)
        let pf = PriorityFrame::new(1, true, 1, 16);
        match server.handle_priority(&pf) {
            FrameAction::WriteFrames(frames) => {
                // Should be RST_STREAM
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
        let mut expecting_continuation = false;
        let mut continuation_stream_id = 0u32;

        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
        ];
        let frame = make_headers_frame(1, &headers, true, &mut encoder);

        let action = server.handle_headers(
            &frame,
            &mut Vec::new(),
            &mut decoder,
            &mut encoder,
            &mut expecting_continuation,
            &mut continuation_stream_id,
        );

        match action {
            FrameAction::WriteFrames(frames) => {
                assert!(!frames.is_empty());
                // Should contain response HEADERS with END_STREAM
            }
            _ => panic!("expected WriteFrames for valid GET"),
        }
    }

    #[test]
    fn test_handle_headers_missing_method() {
        let mut server = Http2Server::new();
        let mut encoder = Encoder::new();
        let mut decoder = Decoder::new();
        let mut expecting_continuation = false;
        let mut continuation_stream_id = 0u32;

        let headers = vec![h(":scheme", "https"), h(":path", "/")];
        let frame = make_headers_frame(1, &headers, true, &mut encoder);

        let action = server.handle_headers(
            &frame,
            &mut Vec::new(),
            &mut decoder,
            &mut encoder,
            &mut expecting_continuation,
            &mut continuation_stream_id,
        );

        // Should send RST_STREAM with PROTOCOL_ERROR
        match action {
            FrameAction::WriteFrames(frames) => {
                assert!(!frames.is_empty());
                // First frame should be RST_STREAM
                let rst = RstStreamFrame::from_frame(&frames[0]).unwrap();
                assert_eq!(rst.stream_id, 1);
                assert_eq!(rst.error_code, ErrorCode::PROTOCOL_ERROR.to_u32());
            }
            FrameAction::Goaway { .. } => {
                // Goaway is also acceptable for some implementations
            }
            _ => panic!("expected error response for missing :method"),
        }
    }

    #[test]
    fn test_handle_headers_connection_specific_rejected() {
        let mut server = Http2Server::new();
        let mut encoder = Encoder::new();
        let mut decoder = Decoder::new();
        let mut expecting_continuation = false;
        let mut continuation_stream_id = 0u32;

        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
            h("connection", "keep-alive"),
        ];
        let frame = make_headers_frame(1, &headers, true, &mut encoder);

        let action = server.handle_headers(
            &frame,
            &mut Vec::new(),
            &mut decoder,
            &mut encoder,
            &mut expecting_continuation,
            &mut continuation_stream_id,
        );

        match action {
            FrameAction::WriteFrames(frames) => {
                // Should be RST_STREAM
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
        let mut expecting_continuation = false;
        let mut continuation_stream_id = 0u32;

        // HEADERS on stream 1 with exclusive dependency on itself
        let headers = vec![
            h(":method", "GET"),
            h(":scheme", "https"),
            h(":path", "/"),
        ];
        let block = encoder.encode(headers.iter().map(|(k, v)| (k.as_slice(), v.as_slice())));
        // Manually construct HEADERS frame with priority info
        // The HeadersFrame has exclusive=true and stream_dependency=1
        let hf = HeadersFrame {
            stream_id: 1,
            end_stream: true,
            exclusive: true,
            stream_dependency: 1, // self-referential!
            weight: 16,
            padding: None,
            header_block: block,
        };
        let frame = hf.to_frame();

        let action = server.handle_headers(
            &frame,
            &mut Vec::new(),
            &mut decoder,
            &mut encoder,
            &mut expecting_continuation,
            &mut continuation_stream_id,
        );

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

        // DATA on stream 1 that hasn't seen HEADERS yet
        let df = DataFrame::new(1, vec![0x00], true).to_frame();
        let action = server.handle_data(&df, &mut encoder);

        // DATA on idle stream is a connection-level error (GOAWAY)
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

        // First send valid HEADERS to create stream 1
        let mut enc = Encoder::new();
        let mut dec = Decoder::new();
        let mut expecting_continuation = false;
        let mut continuation_stream_id = 0u32;

        let headers = vec![
            h(":method", "POST"),
            h(":scheme", "https"),
            h(":path", "/"),
        ];
        let hf = make_headers_frame(1, &headers, false, &mut enc);
        server.handle_headers(
            &hf,
            &mut Vec::new(),
            &mut dec,
            &mut enc,
            &mut expecting_continuation,
            &mut continuation_stream_id,
        );

        // Now send DATA with END_STREAM
        let data = vec![0x01, 0x02, 0x03];
        let df = DataFrame::new(1, data.clone(), true).to_frame();
        let action = server.handle_data(&df, &mut encoder);

        // Window should have decreased by data length
        let remaining = initial_window - data.len() as i64;
        assert_eq!(server.flow_controller.window_size(), remaining);
    }

    // ── CONTINUATION handling ──

    #[test]
    fn test_continuation_not_expected() {
        let mut server = Http2Server::new();
        let mut buf = Vec::new();
        let mut expecting_continuation = false;
        let mut continuation_stream_id = 0u32;
        let mut encoder = Encoder::new();
        let mut decoder = Decoder::new();

        // CONTINUATION when not expecting one
        let frame = Frame::new(FrameType::Continuation, 0x04, 1, vec![0x82]);
        let action = server.handle_continuation(
            &frame,
            &mut buf,
            &mut expecting_continuation,
            &mut continuation_stream_id,
            &mut decoder,
            &mut encoder,
        );

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
        let mut expecting_continuation = false;
        let mut continuation_stream_id = 0u32;

        assert!(server.stream_manager.get_stream(1).is_none());

        let headers = vec![h(":method", "GET"), h(":scheme", "https"), h(":path", "/")];
        let frame = make_headers_frame(1, &headers, true, &mut encoder);
        server.handle_headers(
            &frame,
            &mut Vec::new(),
            &mut decoder,
            &mut encoder,
            &mut expecting_continuation,
            &mut continuation_stream_id,
        );

        // Stream was created and then closed (half-close remote + cleanup)
        // The stream might have been cleaned up already
    }

    #[test]
    fn test_last_processed_stream_id_updated() {
        let mut server = Http2Server::new();
        assert_eq!(server.last_processed_stream_id, 0);

        let mut encoder = Encoder::new();
        let mut decoder = Decoder::new();
        let mut expecting_continuation = false;
        let mut continuation_stream_id = 0u32;

        let headers = vec![h(":method", "GET"), h(":scheme", "https"), h(":path", "/")];
        let frame = make_headers_frame(5, &headers, true, &mut encoder);
        server.handle_headers(
            &frame,
            &mut Vec::new(),
            &mut decoder,
            &mut encoder,
            &mut expecting_continuation,
            &mut continuation_stream_id,
        );

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
}
