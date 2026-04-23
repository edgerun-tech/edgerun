//! Crypto error types.

use std::fmt;

/// All error types for edgerun-crypto operations.
#[derive(Debug)]
pub enum CryptoError {
    /// Invalid key length for the requested cipher.
    InvalidKeyLength { expected: usize, actual: usize },
    /// Invalid key bytes.
    InvalidKey,
    /// Encryption failed.
    EncryptionFailed,
    /// Decryption failed.
    DecryptionFailed,
    /// Invalid PEM format.
    InvalidPem(String),
    /// Invalid DER encoding.
    InvalidDer(String),
    /// Unsupported algorithm.
    UnsupportedAlgorithm(String),
    /// Certificate error.
    CertificateError(String),
    /// Key parsing error.
    KeyParseError(String),
    /// I/O error.
    Io(std::io::Error),
    /// Cipher suite error.
    CipherSuiteError(String),
}

impl fmt::Display for CryptoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CryptoError::InvalidKeyLength { expected, actual } => {
                write!(f, "invalid key length: expected {expected}, got {actual}")
            }
            CryptoError::InvalidKey => write!(f, "invalid key"),
            CryptoError::EncryptionFailed => write!(f, "encryption failed"),
            CryptoError::DecryptionFailed => write!(f, "decryption failed"),
            CryptoError::InvalidPem(msg) => write!(f, "invalid PEM: {msg}"),
            CryptoError::InvalidDer(msg) => write!(f, "invalid DER: {msg}"),
            CryptoError::UnsupportedAlgorithm(alg) => {
                write!(f, "unsupported algorithm: {alg}")
            }
            CryptoError::CertificateError(msg) => write!(f, "certificate error: {msg}"),
            CryptoError::KeyParseError(msg) => write!(f, "key parse error: {msg}"),
            CryptoError::Io(e) => write!(f, "I/O error: {e}"),
            CryptoError::CipherSuiteError(msg) => write!(f, "cipher suite error: {msg}"),
        }
    }
}

impl std::error::Error for CryptoError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CryptoError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for CryptoError {
    fn from(e: std::io::Error) -> Self {
        CryptoError::Io(e)
    }
}
