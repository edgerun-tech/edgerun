//! TLS protocol primitives and state machines.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

pub mod alert;
pub mod certificate;
pub mod certificate_gen;
pub mod cipher;
pub mod handshake;
pub mod key_exchange;
pub mod name_match;
pub mod prf;
pub mod record;
pub mod server;
pub mod session_ticket;
pub mod tls_alpn;

pub use alert::{Alert, AlertLevel};
pub use certificate_gen::{
    cert_from_pem, generate_csr, generate_self_signed, generate_self_signed_pem,
    signing_key_from_pem, signing_key_to_pem, CertificateAndKey,
};
pub use name_match::{normalize_tls_dns_name, tls_dns_name_matches};
pub use tls_alpn::ACME_TLS_ALPN_PROTOCOL;

#[derive(Debug)]
pub enum TlsError {
    Protocol(String),
    HandshakeFailure(String),
    Certificate(String),
    Cipher(String),
    Alert(AlertLevel, Alert),
    HelloRetryRequest(key_exchange::KeyExchangeGroup, Vec<u8>),
    Io(String),
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
            TlsError::Io(m) => write!(f, "I/O error: {m}"),
        }
    }
}

impl core::error::Error for TlsError {}

impl From<String> for TlsError {
    fn from(m: String) -> Self {
        TlsError::Protocol(m)
    }
}

pub type Result<T> = core::result::Result<T, TlsError>;

pub use prf::{hmac_sha256, hmac_sha384};
