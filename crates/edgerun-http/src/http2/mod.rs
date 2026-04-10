//! HTTP/2 implementation (RFC 9113) with HPACK compression (RFC 7541)
//!
//! Provides frame protocol, stream multiplexing, flow control, and SETTINGS negotiation.

pub mod connection;
pub mod flow_control;
pub mod frame;
pub mod hpack;
pub mod settings;
pub mod stream;

#[cfg(test)]
mod hpack_conformance;
#[cfg(test)]
mod http2_frame_conformance;

pub use connection::Connection;
pub use flow_control::FlowController;
pub use frame::{Frame, FrameType};
pub use hpack::{Decoder, Encoder};
pub use settings::Settings;
pub use stream::Stream;

/// HTTP/2 error types
#[derive(Debug)]
pub enum Http2Error {
    /// I/O error
    Io(std::io::Error),
    /// Frame parsing error
    FrameParse(String),
    /// HPACK decoding error
    HpackDecode(String),
    /// HPACK encoding error
    HpackEncode(String),
    /// Protocol violation
    ProtocolViolation(String),
    /// Stream error
    StreamError {
        /// Stream ID
        stream_id: u32,
        /// Error code
        error_code: ErrorCode,
    },
    /// Connection error
    ConnectionError {
        /// Error code
        error_code: ErrorCode,
        /// Reason
        reason: String,
    },
    /// Flow control violation
    FlowControl(String),
    /// Settings timeout
    SettingsTimeout,
}

impl std::fmt::Display for Http2Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Http2Error::Io(err) => write!(f, "I/O error: {}", err),
            Http2Error::FrameParse(msg) => write!(f, "Frame parse error: {}", msg),
            Http2Error::HpackDecode(msg) => write!(f, "HPACK decode error: {}", msg),
            Http2Error::HpackEncode(msg) => write!(f, "HPACK encode error: {}", msg),
            Http2Error::ProtocolViolation(msg) => {
                write!(f, "Protocol violation: {}", msg)
            }
            Http2Error::StreamError {
                stream_id,
                error_code,
            } => write!(f, "Stream {} error: {:?}", stream_id, error_code),
            Http2Error::ConnectionError {
                error_code,
                reason,
            } => write!(f, "Connection error {:?}: {}", error_code, reason),
            Http2Error::FlowControl(msg) => write!(f, "Flow control error: {}", msg),
            Http2Error::SettingsTimeout => write!(f, "Settings timeout"),
        }
    }
}

impl std::error::Error for Http2Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Http2Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Http2Error {
    fn from(err: std::io::Error) -> Self {
        Http2Error::Io(err)
    }
}

impl From<Http2Error> for std::io::Error {
    fn from(err: Http2Error) -> Self {
        match err {
            Http2Error::Io(err) => err,
            other => std::io::Error::new(std::io::ErrorKind::Other, other.to_string()),
        }
    }
}

impl From<String> for Http2Error {
    fn from(err: String) -> Self {
        Http2Error::FrameParse(err)
    }
}

/// HTTP/2 result type
pub type Result<T> = std::result::Result<T, Http2Error>;

/// HTTP/2 error codes (RFC 9113 / RFC 7540 Section 7)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// No error
    NO_ERROR = 0x0,
    /// Protocol error detected
    PROTOCOL_ERROR = 0x1,
    /// Internal error
    INTERNAL_ERROR = 0x2,
    /// Flow control error
    FLOW_CONTROL_ERROR = 0x3,
    /// Settings not acknowledged in time
    SETTINGS_TIMEOUT = 0x4,
    /// Stream closed
    STREAM_CLOSED = 0x5,
    /// Frame size error
    FRAME_SIZE_ERROR = 0x6,
    /// Compressed header list too large
    REFUSED_STREAM = 0x7,
    /// Settings not acknowledged
    CANCEL = 0x8,
    /// Connection compression context no longer valid
    COMPRESSION_ERROR = 0x9,
    /// CONNECT request without :authority
    CONNECT_ERROR = 0xA,
    /// Endpoint detected excess load
    ENHANCE_YOUR_CALM = 0xB,
    /// Client sent request with inadequate security
    INADEQUATE_SECURITY = 0xC,
    /// Request must be sent over HTTP/1.1
    HTTP_1_1_REQUIRED = 0xD,
}

impl ErrorCode {
    /// Parse from u32
    pub fn from_u32(code: u32) -> Self {
        match code {
            0x0 => ErrorCode::NO_ERROR,
            0x1 => ErrorCode::PROTOCOL_ERROR,
            0x2 => ErrorCode::INTERNAL_ERROR,
            0x3 => ErrorCode::FLOW_CONTROL_ERROR,
            0x4 => ErrorCode::SETTINGS_TIMEOUT,
            0x5 => ErrorCode::STREAM_CLOSED,
            0x6 => ErrorCode::FRAME_SIZE_ERROR,
            0x7 => ErrorCode::REFUSED_STREAM,
            0x8 => ErrorCode::CANCEL,
            0x9 => ErrorCode::COMPRESSION_ERROR,
            0xA => ErrorCode::CONNECT_ERROR,
            0xB => ErrorCode::ENHANCE_YOUR_CALM,
            0xC => ErrorCode::INADEQUATE_SECURITY,
            0xD => ErrorCode::HTTP_1_1_REQUIRED,
            _ => ErrorCode::INTERNAL_ERROR,
        }
    }

    /// Convert to u32
    pub fn to_u32(self) -> u32 {
        self as u32
    }
}

/// HTTP/2 connection preface
pub const CONNECTION_PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
