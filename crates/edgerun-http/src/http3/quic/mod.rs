//! QUIC transport protocol (RFC 9000)

pub mod crypto;
pub mod frame;
pub mod packet;
pub mod transport;

pub use crypto::QuicCrypto;
pub use frame::QuicFrame;
pub use packet::{PacketType, QuicPacket};
pub use transport::QuicTransport;

use std::net::UdpSocket;

/// QUIC connection
pub struct QuicConnection {
    /// UDP socket
    socket: UdpSocket,
    /// Server address
    server_addr: String,
    /// Transport layer
    transport: Option<QuicTransport>,
    /// Crypto layer
    crypto: QuicCrypto,
    /// Connection established
    established: bool,
}

impl QuicConnection {
    /// Create client connection
    pub fn client(socket: UdpSocket, server: &str) -> Result<Self, String> {
        let crypto = QuicCrypto::new();

        Ok(QuicConnection {
            socket,
            server_addr: server.to_string(),
            transport: None,
            crypto,
            established: false,
        })
    }

    /// Create a dummy connection for testing
    pub fn dummy() -> Self {
        // This is only used in tests where we don't need actual networking
        use std::net::{IpAddr, Ipv4Addr};
        let socket = UdpSocket::bind("127.0.0.1:0").expect("Cannot bind test socket");
        QuicConnection {
            socket,
            server_addr: "dummy".to_string(),
            transport: None,
            crypto: QuicCrypto::new(),
            established: false,
        }
    }

    /// Check if connection is established
    pub fn is_established(&self) -> bool {
        self.established
    }

    /// Get server name
    pub fn server_name(&self) -> &str {
        &self.server_addr
    }
}

/// QUIC version
pub const QUIC_VERSION_V1: u32 = 0x00000001;

/// Initial salt for QUIC v1 (RFC 9001)
pub const INITIAL_SALT_V1: &[u8] = &[
    0x38, 0x76, 0x2c, 0xf7, 0xf5, 0x59, 0x34, 0xb3, 0x4d, 0x17, 0x9a, 0xe6, 0xa4, 0xc8, 0x0c,
    0xad, 0xcc, 0xbb, 0x7f, 0x0e,
];

/// QUIC connection ID
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnectionId {
    data: Vec<u8>,
}

impl ConnectionId {
    /// Create new connection ID
    pub fn new(data: Vec<u8>) -> Self {
        assert!(data.len() <= 20);
        ConnectionId { data }
    }

    /// Generate random connection ID
    pub fn random() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let mut data = Vec::with_capacity(8);
        for i in 0..8 {
            data.push(((ts >> (i * 8)) & 0xFF) as u8);
        }
        ConnectionId { data }
    }

    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Get length
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

/// QUIC packet number space
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketNumberSpace {
    Initial,
    Handshake,
    ApplicationData,
}

/// QUIC transport parameters
#[derive(Debug, Clone)]
pub struct TransportParameters {
    /// Original destination connection ID
    pub original_destination_connection_id: Option<ConnectionId>,
    /// Max idle timeout (ms)
    pub max_idle_timeout: u64,
    /// Stateless reset token
    pub stateless_reset_token: Option<[u8; 16]>,
    /// Max UDP payload size
    pub max_udp_payload_size: u64,
    /// Initial max data (connection level)
    pub initial_max_data: u64,
    /// Initial max stream data (bidirectional)
    pub initial_max_stream_data_bidi_local: u64,
    /// Initial max stream data (bidirectional, remote)
    pub initial_max_stream_data_bidi_remote: u64,
    /// Initial max stream data (unidirectional)
    pub initial_max_stream_data_uni: u64,
    /// Initial max bidirectional streams
    pub initial_max_streams_bidi: u64,
    /// Initial max unidirectional streams
    pub initial_max_streams_uni: u64,
    /// Ack delay exponent
    pub ack_delay_exponent: u64,
    /// Max ack delay (ms)
    pub max_ack_delay: u64,
    /// Disable active migration
    pub disable_active_migration: bool,
    /// Active connection ID limit
    pub active_connection_id_limit: u64,
    /// Max TLS data size
    pub max_tls_data_size: u64,
}

impl Default for TransportParameters {
    fn default() -> Self {
        TransportParameters {
            original_destination_connection_id: None,
            max_idle_timeout: 30000,
            stateless_reset_token: None,
            max_udp_payload_size: 1200,
            initial_max_data: 65535,
            initial_max_stream_data_bidi_local: 65535,
            initial_max_stream_data_bidi_remote: 65535,
            initial_max_stream_data_uni: 65535,
            initial_max_streams_bidi: 100,
            initial_max_streams_uni: 100,
            ack_delay_exponent: 3,
            max_ack_delay: 25,
            disable_active_migration: false,
            active_connection_id_limit: 2,
            max_tls_data_size: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quic_dummy() {
        let conn = QuicConnection::dummy();
        assert!(!conn.is_established());
        assert_eq!(conn.server_name(), "dummy");
    }

    #[test]
    fn test_connection_id_random() {
        let cid1 = ConnectionId::random();
        let cid2 = ConnectionId::random();
        assert_eq!(cid1.len(), 8);
        assert_ne!(cid1, cid2);
    }

    #[test]
    fn test_transport_parameters_default() {
        let params = TransportParameters::default();
        assert_eq!(params.max_idle_timeout, 30000);
        assert_eq!(params.initial_max_streams_bidi, 100);
    }
}
