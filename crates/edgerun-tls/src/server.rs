//! TLS 1.3 server-side handshake implementation.
//!
//! Handles accepting ClientHello, sending ServerHello + encrypted handshake
//! messages, and completing the TLS 1.3 server handshake.
//!
//! # Server Handshake Flow
//! ```text
//! Client                                          Server
//! ------                                          ------
//! ClientHello (key_share, supported_versions, ...)  →
//!                                       ←  ServerHello (key_share)
//!                                       ←  {EncryptedExtensions}
//!                                       ←  {Certificate}
//!                                       ←  {CertificateVerify}
//!                                       ←  {Finished}
//! {Finished}                          →
//!
//! [Application Data]      ↔     [Application Data]
//! ```

pub mod client_hello;
pub mod message_builder;
pub mod server_handshake;

// Re-export public types
pub use client_hello::ClientHello;
pub use server_handshake::TlsServerStream;

// Re-export public builder functions for consumers
pub use message_builder::{
    build_server_hello,
    build_encrypted_extensions,
    build_certificate_message,
    build_certificate_verify,
    build_finished_message,
};
