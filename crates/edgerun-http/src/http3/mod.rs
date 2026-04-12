//! HTTP/3 implementation (RFC 9114) with QUIC transport (RFC 9000) and QPACK compression (RFC 9204)

pub mod connection;
pub mod http3;
pub mod qpack;
pub mod quic;
pub mod server;
pub mod varint;

pub use connection::Http3Connection;
pub use qpack::{QpackDecoder, QpackEncoder};
pub use quic::QuicConnection;
pub use server::Http3Server;

/// HTTP/3 error types
#[derive(Debug)]
pub enum Http3Error {
    /// I/O error
    Io(std::io::Error),
    /// QUIC protocol error
    QuicError(String),
    /// QPACK encoding/decoding error
    QpackError(String),
    /// HTTP/3 protocol violation
    ProtocolViolation(String),
    /// Stream error
    StreamError { stream_id: u64, error_code: u64 },
    /// Connection error
    ConnectionError { error_code: u64, reason: String },
    /// TLS handshake error
    TlsError(String),
    /// Connection timeout
    Timeout,
}

impl std::fmt::Display for Http3Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Http3Error::Io(err) => write!(f, "I/O error: {}", err),
            Http3Error::QuicError(msg) => write!(f, "QUIC error: {}", msg),
            Http3Error::QpackError(msg) => write!(f, "QPACK error: {}", msg),
            Http3Error::ProtocolViolation(msg) => {
                write!(f, "Protocol violation: {}", msg)
            }
            Http3Error::StreamError {
                stream_id,
                error_code,
            } => write!(f, "Stream {} error: {}", stream_id, error_code),
            Http3Error::ConnectionError {
                error_code,
                reason,
            } => write!(f, "Connection error {}: {}", error_code, reason),
            Http3Error::TlsError(msg) => write!(f, "TLS error: {}", msg),
            Http3Error::Timeout => write!(f, "Connection timeout"),
        }
    }
}

impl std::error::Error for Http3Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Http3Error::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for Http3Error {
    fn from(err: std::io::Error) -> Self {
        Http3Error::Io(err)
    }
}

impl From<String> for Http3Error {
    fn from(err: String) -> Self {
        Http3Error::QuicError(err)
    }
}

/// HTTP/3 result type
pub type Result<T> = std::result::Result<T, Http3Error>;

/// HTTP/3 error codes (RFC 9114 Section 8.1)
pub mod error_codes {
    /// No error
    pub const H3_NO_ERROR: u64 = 0x0100;
    /// General protocol error
    pub const H3_GENERAL_PROTOCOL_ERROR: u64 = 0x0101;
    /// Internal error
    pub const H3_INTERNAL_ERROR: u64 = 0x0102;
    /// Stream creation error
    pub const H3_STREAM_CREATION_ERROR: u64 = 0x0103;
    /// Closed critical stream
    pub const H3_CLOSED_CRITICAL_STREAM: u64 = 0x0104;
    /// Frame was unexpected
    pub const H3_FRAME_UNEXPECTED: u64 = 0x0105;
    /// Frame error
    pub const H3_FRAME_ERROR: u64 = 0x0106;
    /// Excessive load
    pub const H3_EXCESSIVE_LOAD: u64 = 0x0107;
    /// ID error
    pub const H3_ID_ERROR: u64 = 0x0108;
    /// Settings error
    pub const H3_SETTINGS_ERROR: u64 = 0x0109;
    /// Missing settings
    pub const H3_MISSING_SETTINGS: u64 = 0x010A;
    /// Request rejected
    pub const H3_REQUEST_REJECTED: u64 = 0x010B;
    /// Request cancelled
    pub const H3_REQUEST_CANCELLED: u64 = 0x010C;
    /// Request incomplete
    pub const H3_REQUEST_INCOMPLETE: u64 = 0x010D;
    /// CONNECT error
    pub const H3_CONNECT_ERROR: u64 = 0x010E;
    /// Version fallback
    pub const H3_VERSION_FALLBACK: u64 = 0x010F;
}

/// QUIC transport error codes (RFC 9000 Section 20.1)
pub mod quic_error_codes {
    /// No error
    pub const QUIC_NO_ERROR: u64 = 0x0;
    /// Internal error
    pub const QUIC_INTERNAL_ERROR: u64 = 0x1;
    /// Connection refused
    pub const QUIC_CONNECTION_REFUSED: u64 = 0x2;
    /// Flow control error
    pub const QUIC_FLOW_CONTROL_ERROR: u64 = 0x3;
    /// Stream limit error
    pub const QUIC_STREAM_LIMIT_ERROR: u64 = 0x4;
    /// Stream state error
    pub const QUIC_STREAM_STATE_ERROR: u64 = 0x5;
    /// Final size error
    pub const QUIC_FINAL_SIZE_ERROR: u64 = 0x6;
    /// Frame encoding error
    pub const QUIC_FRAME_ENCODING_ERROR: u64 = 0x7;
    /// Transport parameter error
    pub const QUIC_TRANSPORT_PARAMETER_ERROR: u64 = 0x8;
    /// Connection ID limit error
    pub const QUIC_CONNECTION_ID_LIMIT_ERROR: u64 = 0x9;
    /// Protocol violation
    pub const QUIC_PROTOCOL_VIOLATION: u64 = 0xA;
    /// Invalid token
    pub const QUIC_INVALID_TOKEN: u64 = 0xB;
    /// Application error
    pub const QUIC_APPLICATION_ERROR: u64 = 0xC;
    /// Crypto buffer exceeded
    pub const QUIC_CRYPTO_BUFFER_EXCEEDED: u64 = 0xD;
    /// Key update error
    pub const QUIC_KEY_UPDATE_ERROR: u64 = 0xE;
    /// AEAD limit reached
    pub const QUIC_AEAD_LIMIT_REACHED: u64 = 0xF;
    /// Crypto error
    pub const QUIC_CRYPTO_ERROR: u64 = 0x100;
}

/// HTTP/3 version identifiers
pub mod versions {
    /// HTTP/3 version 1 (RFC 9114)
    pub const HTTP3_VERSION: u32 = 0x00000001;
    /// Grease version (for testing)
    pub const GREASE_VERSION: u32 = 0x0a0a0a0a;
}

/// ALPN protocol identifiers
pub mod alpn {
    /// HTTP/3 ALPN
    pub const H3: &[u8] = b"h3";
    /// HTTP/3 (with version)
    pub const H3_29: &[u8] = b"h3-29";
}

/// Well-known UDP port for HTTP/3
pub const HTTP3_DEFAULT_PORT: u16 = 443;
