//! HTTP/2 protocol helpers.

use alloc::string::String;
use core::fmt;

pub mod flow_control;
pub mod frame;
pub mod headers;
#[cfg(feature = "http2-server")]
pub mod hpack;
#[cfg(feature = "http2-server")]
pub mod server;
pub mod settings;
pub mod stream;

pub use frame::{Frame, FrameType};
#[cfg(feature = "http2-server")]
pub use hpack::{Decoder, Encoder};

/// HTTP/2 protocol-layer errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Http2Error {
    /// Frame parsing error.
    FrameParse(String),
    /// Protocol violation.
    ProtocolViolation(String),
    /// Flow-control violation.
    FlowControl(String),
}

impl fmt::Display for Http2Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Http2Error::FrameParse(message) => write!(f, "Frame parse error: {message}"),
            Http2Error::ProtocolViolation(message) => write!(f, "Protocol violation: {message}"),
            Http2Error::FlowControl(message) => write!(f, "Flow control error: {message}"),
        }
    }
}

pub type Result<T> = core::result::Result<T, Http2Error>;

/// HTTP/2 error codes (RFC 9113 / RFC 7540 Section 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// No error.
    NoError = 0x0,
    /// Protocol error detected.
    ProtocolError = 0x1,
    /// Internal error.
    InternalError = 0x2,
    /// Flow control error.
    FlowControlError = 0x3,
    /// Settings not acknowledged in time.
    SettingsTimeout = 0x4,
    /// Stream closed.
    StreamClosed = 0x5,
    /// Frame size error.
    FrameSizeError = 0x6,
    /// Refused to process the stream.
    RefusedStream = 0x7,
    /// Stream cancelled.
    Cancel = 0x8,
    /// Connection compression context no longer valid.
    CompressionError = 0x9,
    /// Endpoint detected excess load.
    EnhanceYourCalm = 0xb,
    /// Client sent request with inadequate security.
    InadequateSecurity = 0xc,
    /// Request must be sent over HTTP/1.1.
    Http11Required = 0xd,
}

impl ErrorCode {
    pub fn from_u32(code: u32) -> Self {
        match code {
            0x0 => ErrorCode::NoError,
            0x1 => ErrorCode::ProtocolError,
            0x2 => ErrorCode::InternalError,
            0x3 => ErrorCode::FlowControlError,
            0x4 => ErrorCode::SettingsTimeout,
            0x5 => ErrorCode::StreamClosed,
            0x6 => ErrorCode::FrameSizeError,
            0x7 => ErrorCode::RefusedStream,
            0x8 => ErrorCode::Cancel,
            0x9 => ErrorCode::CompressionError,
            0xb => ErrorCode::EnhanceYourCalm,
            0xc => ErrorCode::InadequateSecurity,
            0xd => ErrorCode::Http11Required,
            _ => ErrorCode::InternalError,
        }
    }

    pub fn to_u32(self) -> u32 {
        self as u32
    }
}
