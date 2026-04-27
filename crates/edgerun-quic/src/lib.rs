//! QUIC transport protocol (RFC 9000)

pub mod crypto;
pub mod frame;
pub mod handshake;
pub mod handshake_unified;
pub mod packet;
pub mod server_handshake;
pub mod transport;
pub mod types;

// Re-exports
pub use crypto::{PacketProtection, ProtectionKeys, QuicCrypto};
pub use frame::QuicFrame;
pub use handshake::{HandshakeResult, QuicTlsHandshaker};
pub use packet::{get_long_header_payload_offset, PacketType, QuicPacket};
pub use server_handshake::{QuicTlsServerHandshaker, ServerHandshakeResult};
pub use transport::QuicTransport;

// Types
pub use types::{ConnectionId, PacketNumberSpace, TransportParameters, QUIC_VERSION_V1};

pub type QuicConnection = QuicTransport;
