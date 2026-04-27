//! QUIC types shared across modules

/// QUIC version
pub const QUIC_VERSION_V1: u32 = 0x00000001;

/// Initial salt for QUIC v1 (RFC 9001)
pub const INITIAL_SALT_V1: &[u8] = &[
    0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3, 0x4d, 0x17, 0x9a, 0xe6, 0xa4, 0xc8, 0x0c, 0xad,
    0xcc, 0xbb, 0x7f, 0x0e,
];

/// QUIC connection ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnectionId {
    data: Vec<u8>,
}

use edgerun_crypto::getrandom;

impl ConnectionId {
    pub fn new(data: Vec<u8>) -> Self {
        assert!(data.len() <= 20);
        ConnectionId { data }
    }

    pub fn random() -> Self {
        let mut data = [0u8; 8];
        getrandom::fill(&mut data).expect("random generation failed");
        ConnectionId {
            data: data.to_vec(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

/// Packet number space
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketNumberSpace {
    /// Initial keys
    Initial,
    /// Handshake keys
    Handshake,
    /// Application data keys
    ApplicationData,
}

/// Transport parameters
#[derive(Debug, Clone, Default)]
pub struct TransportParameters {
    pub max_idle_timeout: u64,
    pub max_packet_size: u64,
    pub ack_delay_exponent: u8,
    pub max_ack_delay: u64,
    pub active_connection_id_limit: u64,
}
