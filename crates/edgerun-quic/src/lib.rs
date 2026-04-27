//! QUIC transport protocol (RFC 9000)

#![no_std]

#[macro_use]
extern crate alloc;

pub mod compat {
    pub use edgerun_rt::{sleep, spawn, timeout, Duration, Instant};
}

pub mod std {
    pub mod collections {
        pub use alloc::collections::{BTreeMap as HashMap, BTreeSet as HashSet};
    }

    pub mod time {
        pub use edgerun_rt::{Duration, Instant};
    }

    pub mod io {
        use alloc::{format, string::String};
        use core::fmt;

        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum ErrorKind {
            UnexpectedEof,
            InvalidData,
            Other,
        }

        #[derive(Clone, Debug, PartialEq, Eq)]
        pub struct Error {
            kind: ErrorKind,
            message: String,
        }

        impl Error {
            pub fn new(kind: ErrorKind, message: impl fmt::Display) -> Self {
                Self {
                    kind,
                    message: format!("{message}"),
                }
            }

            pub fn kind(&self) -> ErrorKind {
                self.kind
            }
        }

        impl fmt::Display for Error {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.message)
            }
        }

        impl core::error::Error for Error {}
    }
}

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
