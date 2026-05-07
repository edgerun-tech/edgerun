//! TLS 1.3 implementation — async only.
//!
//! # Architecture
//! - **AsyncTlsStream** — async TLS 1.3 client wrapping any `AsyncRead + AsyncWrite`
//! - **AsyncTlsServerStream** — async TLS 1.3 server wrapping any `AsyncRead + AsyncWrite`
//!
//! Runtime-owned TLS stream support for node-managed transports.
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

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

pub mod alert {
    pub use edgerun_protocols::tls::alert::*;
}
pub mod async_tls;
pub mod certificate {
    pub use edgerun_protocols::tls::certificate::*;
}
pub mod certificate_gen {
    pub use edgerun_protocols::tls::certificate_gen::*;
}
pub mod cipher {
    pub use edgerun_protocols::tls::cipher::*;
}
pub mod handshake {
    pub use edgerun_protocols::tls::handshake::*;
}
pub mod key_exchange {
    pub use edgerun_protocols::tls::key_exchange::*;
}
pub mod name_match {
    pub use edgerun_protocols::tls::name_match::*;
}
pub mod prf {
    pub use edgerun_protocols::tls::prf::*;
}
pub mod record {
    pub use edgerun_protocols::tls::record::*;
}
pub mod server {
    pub mod client_hello {
        pub use edgerun_protocols::tls::server::client_hello::*;
    }
    pub mod message_builder {
        pub use edgerun_protocols::tls::server::message_builder::*;
    }

    pub use client_hello::ClientHello;
    pub use message_builder::{
        build_certificate_message, build_certificate_verify, build_encrypted_extensions,
        build_finished_message, build_server_hello,
    };
}
pub mod session_cache;
pub mod tls_alpn {
    pub use edgerun_protocols::tls::tls_alpn::*;
}

pub use crate::rt::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
pub use async_tls::{AsyncTlsServerStream, AsyncTlsStream};
pub use name_match::{normalize_tls_dns_name, tls_dns_name_matches};
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
    HelloRetryRequest(crate::tls::key_exchange::KeyExchangeGroup, Vec<u8>),
    /// Underlying I/O error from the async transport.
    Io(crate::rt::IoError),
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

impl From<edgerun_protocols::tls::TlsError> for TlsError {
    fn from(error: edgerun_protocols::tls::TlsError) -> Self {
        match error {
            edgerun_protocols::tls::TlsError::Protocol(message) => TlsError::Protocol(message),
            edgerun_protocols::tls::TlsError::HandshakeFailure(message) => {
                TlsError::HandshakeFailure(message)
            }
            edgerun_protocols::tls::TlsError::Certificate(message) => {
                TlsError::Certificate(message)
            }
            edgerun_protocols::tls::TlsError::Cipher(message) => TlsError::Cipher(message),
            edgerun_protocols::tls::TlsError::Alert(level, alert) => TlsError::Alert(level, alert),
            edgerun_protocols::tls::TlsError::HelloRetryRequest(group, cookie) => {
                TlsError::HelloRetryRequest(group, cookie)
            }
            edgerun_protocols::tls::TlsError::Io(_) => {
                TlsError::Io(crate::rt::IoError::Other("protocol TLS I/O error"))
            }
        }
    }
}

#[cfg(not(target_os = "none"))]
impl From<::std::io::Error> for TlsError {
    fn from(_: ::std::io::Error) -> Self {
        TlsError::Io(crate::rt::IoError::Other("host TLS I/O error"))
    }
}

impl From<crate::rt::IoError> for TlsError {
    fn from(error: crate::rt::IoError) -> Self {
        TlsError::Io(error)
    }
}

/// Result type
pub type Result<T> = core::result::Result<T, TlsError>;

/// Re-export HMAC functions from prf module for Finished verification
pub use self::prf::{hmac_sha256, hmac_sha384};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tls::async_tls::generate_random;
    use crate::tls::certificate_gen::generate_self_signed;
    use crate::tls::cipher::NamedGroup;
    use crate::tls::handshake::ClientHelloBuilder;
    use crate::tls::server::ClientHello;
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

        let parsed = crate::tls::certificate::Certificate::from_der(&cert.cert_der)
            .expect("failed to parse generated cert");
        assert!(parsed.not_before < parsed.not_after);
        assert!(parsed.is_valid_at_unix_secs(parsed.not_before));
        assert!(parsed.is_valid_at_unix_secs(parsed.not_after));
        if parsed.not_before > 0 {
            assert!(!parsed.is_valid_at_unix_secs(parsed.not_before - 1));
        }
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
