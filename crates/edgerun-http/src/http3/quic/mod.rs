//! QUIC transport protocol (RFC 9000)

pub mod crypto;
pub mod frame;
pub mod packet;
pub mod transport;

pub use crypto::{PacketProtection, ProtectionKeys, QuicCrypto};
pub use frame::QuicFrame;
pub use packet::{PacketType, QuicPacket};
pub use transport::QuicTransport;

use crypto::{CryptoPhase, ProtectionKeys as ProtKeys};

use std::net::UdpSocket;

/// QUIC connection
pub struct QuicConnection {
    /// UDP socket
    socket: UdpSocket,
    /// Server address
    server_addr: String,
    /// Transport layer
    transport: QuicTransport,
    /// Crypto layer
    crypto: QuicCrypto,
    /// Packet protection
    protection: Option<crypto::PacketProtection>,
    /// Connection established
    established: bool,
    /// Receive buffer
    recv_buffer: Vec<u8>,
    /// Offset into recv_buffer for partial reads
    recv_offset: usize,
}

impl QuicConnection {
    /// Create client connection (does not perform handshake)
    pub fn client(socket: UdpSocket, server: &str) -> Result<Self, String> {
        let local_cid = ConnectionId::random();
        let remote_cid = ConnectionId::random();
        let transport = QuicTransport::new(local_cid.clone(), remote_cid.clone());
        let crypto = QuicCrypto::new();

        Ok(QuicConnection {
            socket,
            server_addr: server.to_string(),
            transport,
            crypto,
            protection: None,
            established: false,
            recv_buffer: Vec::new(),
            recv_offset: 0,
        })
    }

    /// Create a dummy connection for testing
    pub fn dummy() -> Self {
        let socket = UdpSocket::bind("127.0.0.1:0").expect("Cannot bind test socket");
        let local_cid = ConnectionId::random();
        let remote_cid = ConnectionId::random();
        let transport = QuicTransport::new(local_cid.clone(), remote_cid.clone());

        QuicConnection {
            socket,
            server_addr: "dummy".to_string(),
            transport,
            crypto: QuicCrypto::new(),
            protection: None,
            established: false,
            recv_buffer: Vec::new(),
            recv_offset: 0,
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

    /// Send data on a stream
    pub fn send_stream_data(&mut self, stream_id: u64, data: &[u8], fin: bool) -> Result<(), String> {
        let frame = self.transport.create_stream_frame(stream_id, data.to_vec(), fin);
        self.send_frame(frame)
    }

    /// Send a single QUIC frame
    fn send_frame(&mut self, frame: QuicFrame) -> Result<(), String> {
        let pn = self.transport.next_packet_number();

        // Build packet payload (frame serialized)
        let payload = frame.to_bytes();

        // Create Initial packet
        let pkt = QuicPacket::initial(
            0x00000001,
            self.transport.remote_cid.as_bytes().to_vec(),
            self.transport.local_cid.as_bytes().to_vec(),
            vec![],
            pn,
            payload,
        );

        let packet_bytes = pkt.to_bytes();

        // Encrypt if we have protection
        let send_bytes = if let Some(ref mut prot) = self.protection {
            prot.protect(&packet_bytes[..9], &packet_bytes[9..])
                .unwrap_or_else(|e| panic!("Packet protection failed: {}", e))
        } else {
            packet_bytes
        };

        // Send via UDP
        let addr = format!("{}:443", self.server_addr);
        self.socket
            .send_to(&send_bytes, &addr)
            .map_err(|e| format!("UDP send failed: {}", e))?;

        self.transport.update_activity();
        Ok(())
    }

    /// Receive data, returning (stream_id, data, fin)
    pub fn recv_stream_data(&mut self) -> Result<Option<(u64, Vec<u8>, bool)>, String> {
        // Try to read from UDP if buffer is empty
        if self.recv_buffer.is_empty() || self.recv_offset >= self.recv_buffer.len() {
            let mut buf = [0u8; 4096];
            match self.socket.recv(&mut buf) {
                Ok(n) => {
                    self.recv_buffer = buf[..n].to_vec();
                    self.recv_offset = 0;
                    self.transport.update_activity();
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    return Ok(None);
                }
                Err(e) => return Err(format!("UDP recv failed: {}", e)),
            }
        }

        // Parse packet
        if self.recv_offset >= self.recv_buffer.len() {
            return Ok(None);
        }

        let data = &self.recv_buffer[self.recv_offset..];
        match QuicPacket::from_bytes(data) {
            Ok((packet, consumed)) => {
                self.recv_offset += consumed;
                let pn = packet.header.packet_number;

                // Try to decrypt if we have protection
                let plaintext = if let Some(ref mut prot) = self.protection {
                    match prot.unprotect(&[], pn, &packet.payload) {
                        Ok(pt) => pt,
                        Err(_) => return Err("Packet authentication failed".to_string()),
                    }
                } else {
                    packet.payload
                };

                // Parse frames from plaintext
                self.transport.update_activity();
                // For now, treat the whole payload as a single STREAM frame
                // In a full impl, we'd parse frame headers here
                Ok(Some((0, plaintext, false)))
            }
            Err(_) => Ok(None),
        }
    }

    /// Get mutable crypto
    pub fn crypto_mut(&mut self) -> &mut QuicCrypto {
        &mut self.crypto
    }

    /// Set protection keys after handshake
    pub fn set_protection_keys(&mut self, keys: &ProtKeys) {
        self.crypto.set_keys(CryptoPhase::Application, keys.clone());
        self.protection = Some(PacketProtection::new(keys));
    }

    /// Set non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), String> {
        self.socket
            .set_nonblocking(nonblocking)
            .map_err(|e| format!("set_nonblocking: {}", e))
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

    /// Generate random connection ID using CSPRNG
    pub fn random() -> Self {
        let mut data = [0u8; 8];
        edgerun_crypto::getrandom::getrandom(&mut data)
            .expect("CSPRNG failure");
        ConnectionId { data: data.to_vec() }
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
