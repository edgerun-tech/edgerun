//! TLS 1.3 implementation — async only.
//!
//! # Architecture
//! - **AsyncTlsStream** — async TLS 1.3 client wrapping any `AsyncRead + AsyncWrite`
//! - **AsyncTlsServerStream** — async TLS 1.3 server wrapping any `AsyncRead + AsyncWrite`
//!
//! Consumers should NOT use this crate directly. Use `edgerun_http::HttpClient`
//! for HTTP/HTTPS operations. The TLS types here are internal building blocks.
//!
//! # TLS 1.3 Handshake (1-RTT)
//! ```text
//! Client                                          Server
//! ------                                          ------
//! ClientHello (key_share, supported_versions, ...)
//!                                       ←  ServerHello (key_share)
//!                                       ←  {EncryptedExtensions}
//!                                       ←  {Certificate}
//!                                       ←  {CertificateVerify}
//!                                       ←  {Finished}
//! {Finished}                          →
//!
//! [Application Data]      ↔     [Application Data]
//! ```
//!
//! All messages after ServerHello are encrypted with handshake keys.

#![no_std]

extern crate alloc;
#[cfg(not(target_os = "none"))]
extern crate std as host_std;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
pub use edgerun_rt as rt;

pub mod alert;
pub mod async_tls;
pub mod certificate;
pub mod certificate_gen;
pub mod cipher;
pub mod compat;
pub mod handshake;
pub mod key_exchange;
pub mod prf;
pub mod record;
pub mod server;
pub mod session_cache;
pub mod std;
pub mod tls_alpn;

pub use async_tls::{AsyncTlsServerStream, AsyncTlsStream};
pub use compat::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
pub use session_cache::{parse_new_session_ticket, SessionCache, SessionTicket};

pub use alert::{Alert, AlertLevel};
pub use certificate_gen::{
    cert_from_pem, generate_csr, generate_self_signed, generate_self_signed_pem,
    signing_key_from_pem, signing_key_to_pem, CertificateAndKey,
};
pub use tls_alpn::ACME_TLS_ALPN_PROTOCOL;

/// TLS error types
#[derive(Debug)]
pub enum TlsError {
    /// Protocol error (malformed message)
    Protocol(String),
    /// Handshake failure
    HandshakeFailure(String),
    /// Certificate validation error
    Certificate(String),
    /// AEAD encryption/decryption error
    Cipher(String),
    /// Alert received from peer
    Alert(AlertLevel, Alert),
    /// HelloRetryRequest received — retry with this group and optional cookie
    HelloRetryRequest(crate::key_exchange::KeyExchangeGroup, Vec<u8>),
    /// Underlying I/O error from the async transport.
    Io(std::io::Error),
}

impl fmt::Display for TlsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TlsError::Protocol(m) => write!(f, "Protocol error: {m}"),
            TlsError::HandshakeFailure(m) => write!(f, "Handshake failure: {m}"),
            TlsError::Certificate(m) => write!(f, "Certificate error: {m}"),
            TlsError::Cipher(m) => write!(f, "Cipher error: {m}"),
            TlsError::Alert(lv, a) => write!(f, "TLS alert: {lv:?} {a}"),
            TlsError::HelloRetryRequest(g, _) => write!(f, "HelloRetryRequest: group={g:?}"),
            TlsError::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl core::error::Error for TlsError {}

impl From<String> for TlsError {
    fn from(m: String) -> Self {
        TlsError::Protocol(m)
    }
}

impl From<std::io::Error> for TlsError {
    fn from(e: std::io::Error) -> Self {
        TlsError::Io(e)
    }
}

/// Result type
pub type Result<T> = core::result::Result<T, TlsError>;

/// Re-export HMAC functions from prf module for Finished verification
pub use crate::prf::{hmac_sha256, hmac_sha384};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::async_tls::generate_random;
    use crate::certificate_gen::generate_self_signed;
    use crate::cipher::NamedGroup;
    use crate::handshake::ClientHelloBuilder;
    use crate::server::ClientHello;
    use alloc::string::ToString;
    use edgerun_crypto::CipherSuite;

    #[test]
    fn test_tls_error_display() {
        let e = TlsError::Protocol("test".into());
        assert!(e.to_string().contains("test"));

        let e = TlsError::Certificate("expired".into());
        assert!(e.to_string().contains("expired"));
    }

    #[test]
    fn test_certificate_generation() {
        let cert = generate_self_signed(&["localhost", "example.com"]).unwrap();
        assert!(!cert.cert_der.is_empty());
        assert!(cert.cert_der.len() > 100);

        let parsed = crate::certificate::Certificate::from_der(&cert.cert_der)
            .expect("failed to parse generated cert");
        assert!(parsed.is_valid_now());
    }

    #[test]
    fn test_server_client_hello_parsing() {
        let random = generate_random();
        let ch = ClientHelloBuilder::new(random, "localhost")
            .key_share(&[0x04u8; 65], NamedGroup::SECP256R1)
            .build()
            .expect("build ClientHello");

        let parsed = ClientHello::parse(&ch).expect("parse ClientHello");
        assert_eq!(parsed.server_name, Some("localhost".to_string()));
        assert!(!parsed.cipher_suites.is_empty());
        assert!(parsed.client_key_share.is_some());
        assert!(parsed.supported_versions.contains(&0x0304));
    }
}
