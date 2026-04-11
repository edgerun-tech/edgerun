//! Frame handler implementations for the HTTP/2 server.
//!
//! Each handler is a method on `Http2Server` in this module.

use crate::http2::flow_control::FlowController;
use crate::http2::frame::{
    DataFrame, Frame, GoawayFrame, HeadersFrame, PingFrame, PriorityFrame, RstStreamFrame,
    SettingsFrame, WindowUpdateFrame,
};
use crate::http2::headers::{validate_header_name_case, validate_request_headers};
use crate::http2::hpack::{Decoder, Encoder};
use super::continuation::ContinuationState;
use super::response;
use crate::http2::settings::Settings;
use crate::http2::stream::StreamState;
use crate::http2::ErrorCode;
use super::FrameAction;
use super::Http2Server;

impl Http2Server {
    /// Process an incoming SETTINGS frame (after preface).
    pub fn handle_settings(&mut self, frame: &Frame) -> FrameAction {
        let sf = match SettingsFrame::from_frame(frame) {
            Ok(sf) => sf,
            Err(_) => {
                self.goaway_sent = true;
                return response::send_goaway(
                    self.last_processed_stream_id,
                    ErrorCode::FRAME_SIZE_ERROR.to_u32(),
                    b"Invalid SETTINGS payload",
                );
            }
        };

        if !sf.ack {
            match Settings::from_entries(&sf.entries) {
                Ok(new_settings) => {
                    self.client_settings = new_settings;
                    self.max_frame_size = self.client_settings.max_frame_size;
                    if let Some(max) = self.client_settings.max_concurrent_streams {
                        self.stream_manager.set_max_concurrent_streams(max);
                    }
                }
                Err(e) => {
                    let error_code = match &e {
                        crate::http2::Http2Error::FlowControl(_) => ErrorCode::FLOW_CONTROL_ERROR.to_u32(),
                        _ => ErrorCode::PROTOCOL_ERROR.to_u32(),
                    };
                    return response::send_goaway(
                        self.last_processed_stream_id,
                        error_code,
                        b"Invalid SETTINGS values",
                    );
                }
            }
            return response::send_settings_ack();
        }
        FrameAction::None
    }

    /// Process an incoming PING frame.
    pub fn handle_ping(&mut self, frame: &Frame) -> FrameAction {
        let pf = match PingFrame::from_frame(frame) {
            Ok(pf) => pf,
            Err(_) => return FrameAction::None,
        };
        if !pf.ack {
            return response::send_ping_ack(pf.data);
        }
        FrameAction::None
    }

    /// Process an incoming WINDOW_UPDATE frame.
    pub fn handle_window_update(&mut self, frame: &Frame) -> FrameAction {
        let wu = match WindowUpdateFrame::from_frame(frame) {
            Ok(wu) => wu,
            Err(_) => {
                self.goaway_sent = true;
                return response::send_goaway(
                    self.last_processed_stream_id,
                    ErrorCode::FLOW_CONTROL_ERROR.to_u32(),
                    b"Invalid WINDOW_UPDATE",
                );
            }
        };

        if frame.stream_id == 0 {
            if self.flow_controller.increment(wu.window_increment).is_err() {
                self.goaway_sent = true;
                return response::send_goaway(
                    self.last_processed_stream_id,
                    ErrorCode::FLOW_CONTROL_ERROR.to_u32(),
                    b"Window overflow",
                );
            }
        } else {
            let stream_exists = self.stream_manager.get_stream(wu.stream_id).is_some();
            let was_seen = wu.stream_id <= self.last_processed_stream_id;
            if !stream_exists && !was_seen {
                self.goaway_sent = true;
                return response::send_goaway(
                    self.last_processed_stream_id,
                    ErrorCode::PROTOCOL_ERROR.to_u32(),
                    b"WINDOW_UPDATE on idle stream",
                );
            }
            if let Some(s) = self.stream_manager.get_stream_mut(wu.stream_id) {
                if wu.window_increment as i64 > FlowController::MAX_WINDOW_SIZE - s.remote_window {
                    return response::rst_stream(
                        wu.stream_id,
                        ErrorCode::FLOW_CONTROL_ERROR.to_u32(),
                        &mut self.stream_manager,
                    );
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
        let stream_exists = self.stream_manager.get_stream(rst.stream_id).is_some();
        let was_seen = rst.stream_id <= self.last_processed_stream_id;
        if !stream_exists && !was_seen {
            self.goaway_sent = true;
            return response::send_goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"RST_STREAM on idle stream",
            );
        }
        if let Some(s) = self.stream_manager.get_stream_mut(rst.stream_id) {
            s.close();
        }
        FrameAction::None
    }

    /// Process an incoming PRIORITY frame.
    pub fn handle_priority(&mut self, frame: &PriorityFrame) -> FrameAction {
        if frame.stream_id == 0 {
            self.goaway_sent = true;
            return response::send_goaway(
                self.last_processed_stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                b"PRIORITY on stream 0",
            );
        }

        if frame.stream_dependency == frame.stream_id {
            return response::rst_stream(
                frame.stream_id,
                ErrorCode::PROTOCOL_ERROR.to_u32(),
                &mut self.stream_manager,
            );
        }

        FrameAction::None
    }

    /// Process an incoming GOAWAY frame (from client).
    pub fn handle_goaway(&mut self) -> FrameAction {
        self.goaway_sent = true;
        FrameAction::CloseConnection
    }
}
