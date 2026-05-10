//! TLS protocol primitives and state machines.

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

#[cfg(feature = "tls")]
pub mod alert;
#[cfg(feature = "tls")]
pub mod certificate;
pub mod certificate_gen;
#[cfg(feature = "tls")]
pub mod cipher;
#[cfg(feature = "tls")]
pub mod handshake;
#[cfg(feature = "tls")]
pub mod key_exchange;
#[cfg(feature = "tls")]
pub mod name_match;
#[cfg(feature = "tls")]
pub mod prf;
#[cfg(feature = "tls")]
pub mod record;
#[cfg(feature = "tls")]
pub mod server;
#[cfg(feature = "tls")]
pub mod session_ticket;
#[cfg(feature = "tls")]
pub mod tls_alpn;

#[cfg(feature = "tls")]
pub use alert::{Alert, AlertLevel};
pub use certificate_gen::{
    CertificateAndKey, cert_from_pem, generate_csr, generate_self_signed, generate_self_signed_pem,
    signing_key_from_pem, signing_key_to_pem,
};
#[cfg(feature = "tls")]
pub use name_match::{normalize_tls_dns_name, tls_dns_name_matches};
#[cfg(feature = "tls")]
pub use tls_alpn::ACME_TLS_ALPN_PROTOCOL;

#[cfg(feature = "tls")]
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

#[cfg(feature = "tls")]
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

#[cfg(feature = "tls")]
impl core::error::Error for TlsError {}

#[cfg(feature = "tls")]
impl From<String> for TlsError {
    fn from(m: String) -> Self {
        TlsError::Protocol(m)
    }
}

#[cfg(feature = "tls")]
pub type Result<T> = core::result::Result<T, TlsError>;

#[cfg(feature = "tls")]
pub use prf::{hmac_sha256, hmac_sha384};
