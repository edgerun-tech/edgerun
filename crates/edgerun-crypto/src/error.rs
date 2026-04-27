use alloc::string::String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    InvalidKey,
    EncryptionFailed,
    DecryptionFailed,
    SignatureVerificationFailed,
    SigningError,
    RandomGenerationFailed,
    DigestMismatch,
    TpmUnavailable,
    UnsupportedAlgorithm,
    InvalidPoint,
    InvalidSignature,
    CpuFeatureUnavailable,
}

impl core::fmt::Display for CryptoError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CryptoError::InvalidKey => write!(f, "invalid cryptographic key"),
            CryptoError::EncryptionFailed => write!(f, "encryption failed"),
            CryptoError::DecryptionFailed => write!(f, "decryption failed"),
            CryptoError::SignatureVerificationFailed => write!(f, "signature verification failed"),
            CryptoError::SigningError => write!(f, "signing operation failed"),
            CryptoError::RandomGenerationFailed => write!(f, "random number generation failed"),
            CryptoError::DigestMismatch => write!(f, "digest mismatch"),
            CryptoError::TpmUnavailable => write!(f, "TPM is not available"),
            CryptoError::UnsupportedAlgorithm => write!(f, "unsupported algorithm"),
            CryptoError::InvalidPoint => write!(f, "invalid elliptic curve point"),
            CryptoError::InvalidSignature => write!(f, "invalid signature format"),
            CryptoError::CpuFeatureUnavailable => write!(f, "required CPU feature not available"),
        }
    }
}

impl core::error::Error for CryptoError {}

pub type Result<T> = core::result::Result<T, CryptoError>;

#[cfg(feature = "std")]
impl From<CryptoError> for std::io::Error {
    fn from(e: CryptoError) -> Self {
        std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
    }
}