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

mod continuation;
mod data;
mod handlers;
mod headers;
mod response;

pub use continuation::ContinuationState;
pub use response::{respond_with_200, rst_stream, send_goaway, send_ping_ack, send_settings_ack};

use std::collections::HashMap;

use super::flow_control::FlowController;
use crate::http2::frame::{
    Frame, SettingsFrame,
};
use super::settings::Settings;
use crate::http2::stream::StreamManager;
use super::ErrorCode;

/// A sync callback invoked when complete request headers arrive on a stream.
/// Takes the decoded request headers and returns frames to write back.
/// The handler is responsible for HPACK-encoding response headers via the
/// provided encoder and may include DATA frames for the response body.
pub type ResponseBuilder = Box<dyn FnMut(u32, &mut Vec<(Vec<u8>, Vec<u8>)>) -> FrameAction>;

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
    /// Track closed stream IDs to handle post-closure frames gracefully.
    /// Stores stream IDs that were recently closed, so we can accept
    /// WINDOW_UPDATE/PRIORITY/RST_STREAM on them without connection errors.
    closed_stream_ids: std::collections::HashSet<u32>,
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
            closed_stream_ids: std::collections::HashSet::new(),
        }
    }

    /// Apply the client's initial SETTINGS frame.
    /// Returns frames to write, or a Goaway if settings are invalid.
    pub fn apply_client_settings(&mut self, settings_frame: &SettingsFrame) -> FrameAction {
        if let Err(error_code) = settings_frame.to_frame().validate_semantics() {
            self.goaway_sent = true;
            return response::send_goaway(0, error_code, b"Invalid SETTINGS");
        }

        match Settings::from_entries(&settings_frame.entries) {
            Ok(s) => {
                let old_initial_window = self.client_settings.initial_window_size;
                self.client_settings = s;
                self.max_frame_size = self.client_settings.max_frame_size;
                if let Some(max) = self.client_settings.max_concurrent_streams {
                    self.stream_manager.set_max_concurrent_streams(max);
                }
                // RFC 7540 §6.9.2: When INITIAL_WINDOW_SIZE changes, adjust all active streams
                if self.client_settings.initial_window_size != old_initial_window {
                    self.stream_manager.update_initial_window_size(
                        self.client_settings.initial_window_size,
                    );
                }
            }
            Err(_) => {
                self.goaway_sent = true;
                return response::send_goaway(
                    0,
                    ErrorCode::PROTOCOL_ERROR.to_u32(),
                    b"Invalid settings values",
                );
            }
        }

        FrameAction::WriteFrames(vec![
            SettingsFrame::new(self.server_settings.to_entries()).to_frame(),
            SettingsFrame::ack().to_frame(),
        ])
    }

    // ── Internal helpers ──

    pub(crate) fn update_last_stream(&mut self, stream_id: u32) {
        if stream_id > self.last_processed_stream_id {
            self.last_processed_stream_id = stream_id;
        }
    }

    /// Check if a stream ID was recently closed (so we can accept frames on it).
    fn is_closed_stream(&self, stream_id: u32) -> bool {
        self.closed_stream_ids.contains(&stream_id)
    }

    /// Record a stream as closed for post-closure frame handling.
    fn record_closed_stream(&mut self, stream_id: u32) {
        self.closed_stream_ids.insert(stream_id);
    }

    /// Periodically clean up closed streams.
    /// Should be called every N frames (e.g. every 100 frames) to prevent
    /// memory growth from streams that have been closed but not removed.
    pub fn cleanup_closed_streams(&mut self) {
        self.stream_manager.cleanup_closed();
        // Also clear the closed_stream_ids tracking set
        self.closed_stream_ids.clear();
    }
}

impl Default for Http2Server {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests;
