//! QUIC transport protocol (RFC 9000).

pub mod crypto;
pub mod frame;
pub mod handshake;
pub mod handshake_unified;
pub mod packet;
pub mod server_handshake;
pub mod transport;
pub mod types;

pub use crypto::{PacketProtection, ProtectionKeys, QuicCrypto};
pub use frame::QuicFrame;
pub use handshake::{HandshakeResult, QuicTlsHandshaker};
pub use packet::{PacketType, QuicPacket, get_long_header_payload_offset};
pub use server_handshake::{QuicTlsServerHandshaker, ServerHandshakeResult};
pub use transport::{QuicDuration, QuicInstant, QuicTransport};
pub use types::{ConnectionId, PacketNumberSpace, QUIC_VERSION_V1, TransportParameters};

pub type QuicConnection = QuicTransport;
