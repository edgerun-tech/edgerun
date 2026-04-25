//! HTTP/3 protocol (RFC 9114)

pub mod connection;
pub mod frame;
pub mod qpack;
pub mod quic;
pub mod server;
pub mod settings;
pub mod stream;
pub mod varint;

pub use connection::Http3Connection;
pub use frame::Http3Frame;
pub use qpack::{QpackDecoder, QpackEncoder};
pub use quic::QuicConnection;
pub use server::Http3Server;
pub use settings::Http3Settings;
pub use stream::Http3Stream;

/// Result type
pub type Result<T> = std::result::Result<T, Http3Error>;

/// HTTP/3 error
#[derive(Debug)]
pub enum Http3Error {
    Io(std::io::Error),
    QuicError(String),
    QpackError(String),
    FrameUnexpected(String),
    ProtocolViolation(String),
}

impl std::fmt::Display for Http3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Http3Error::Io(e) => write!(f, "IO: {}", e),
            Http3Error::QuicError(e) => write!(f, "QUIC: {}", e),
            Http3Error::QpackError(e) => write!(f, "QPACK: {}", e),
            Http3Error::FrameUnexpected(e) => write!(f, "Frame: {}", e),
            Http3Error::ProtocolViolation(e) => write!(f, "Protocol: {}", e),
        }
    }
}

impl std::error::Error for Http3Error {}

impl From<std::io::Error> for Http3Error {
    fn from(e: std::io::Error) -> Self {
        Http3Error::Io(e)
    }
}

impl From<String> for Http3Error {
    fn from(e: String) -> Self {
        Http3Error::ProtocolViolation(e)
    }
}

impl From<&str> for Http3Error {
    fn from(e: &str) -> Self {
        Http3Error::ProtocolViolation(e.to_string())
    }
}

mod error_codes {
    pub const HTTP_NO_ERROR: u32 = 0x0;
    pub const HTTP_GENERAL_PROTOCOL_ERROR: u32 = 0x1;
    pub const HTTP_INTERNAL_ERROR: u32 = 0x2;
    pub const HTTP_STREAM_CREATION_ERROR: u32 = 0x3;
    pub const HTTP_CLOSED_CRITICAL_STREAM: u32 = 0x4;
    pub const HTTP_FRAME_UNSUPPORTED: u32 = 0x5;
    pub const HTTP_FRAME_ERROR: u32 = 0x6;
    pub const HTTP_EXCESSIVE_LOAD: u32 = 0x7;
    pub const HTTP_SETTINGS_ERROR: u32 = 0x8;
    pub const HTTP_MISSING_SETTINGS: u32 = 0x9;
    pub const HTTP_REQUEST_REJECTED: u32 = 0xa;
    pub const HTTP_CONNECTION_ERROR: u32 = 0xb;
    pub const HTTP_VERSION_FALLBACK: u32 = 0xc;
}

/// HTTP/3 stream type identifiers
pub mod stream_types {
    /// Control stream
    pub const CONTROL: u64 = 0x00;
    /// Push stream
    pub const PUSH: u64 = 0x01;
    /// QPACK encoder stream
    pub const QPACK_ENCODER: u64 = 0x02;
    /// QPACK decoder stream
    pub const QPACK_DECODER: u64 = 0x03;
}

/// Stream ID helpers
pub fn is_client_initiated_bidi(id: u64) -> bool {
    id.is_multiple_of(4)
}

pub fn is_server_initiated_bidi(id: u64) -> bool {
    id % 4 == 1
}

pub fn is_client_initiated_uni(id: u64) -> bool {
    id % 4 == 2
}

pub fn is_server_initiated_uni(id: u64) -> bool {
    id % 4 == 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stream_types() {
        assert!(is_client_initiated_bidi(0));
        assert!(is_server_initiated_bidi(1));
        assert!(is_client_initiated_uni(2));
        assert!(is_server_initiated_uni(3));
    }
}
