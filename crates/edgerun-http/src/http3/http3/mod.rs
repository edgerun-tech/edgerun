//! HTTP/3 protocol (RFC 9114)

pub mod frame;
pub mod settings;
pub mod stream;

pub use frame::Http3Frame;
pub use settings::Http3Settings;
pub use stream::Http3Stream;

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
