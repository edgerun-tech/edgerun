//! PKCE (Proof Key for Code Exchange) helpers — RFC 7636.

use crate::prelude::*;
use alloc::format;
use core::fmt;
use edgerun_crypto::{fill_random, sha256};
use edgerun_encoding::base64::base64url_nopad_encode;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PkceError {
    Random(String),
    InvalidVerifierLength(usize),
}

impl fmt::Display for PkceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Random(message) => write!(f, "PKCE random generation failed: {message}"),
            Self::InvalidVerifierLength(len) => {
                write!(f, "PKCE code_verifier must be 43-128 chars, got {len}")
            }
        }
    }
}

impl core::error::Error for PkceError {}

/// A PKCE code-verifier / code-challenge pair.
#[derive(Debug, Clone)]
pub struct PkcePair {
    pub code_verifier: String,
    pub code_challenge: String,
}

impl PkcePair {
    /// Generate a new PKCE pair with 32 bytes of randomness (256-bit entropy).
    ///
    /// RFC 7636: `code_verifier` must be 43-128 characters of unreserved
    /// characters. 32 random bytes encode to 43 base64url characters with no
    /// padding.
    pub fn generate() -> Result<Self, PkceError> {
        let mut code_verifier_bytes = [0u8; 32];
        fill_random(&mut code_verifier_bytes).map_err(|e| PkceError::Random(format!("{e:?}")))?;
        Ok(Self::from_verifier_bytes(&code_verifier_bytes))
    }

    pub fn from_verifier_bytes(bytes: &[u8]) -> Self {
        let code_verifier = base64url_nopad_encode(bytes);
        let code_challenge = code_challenge_s256(&code_verifier);
        Self {
            code_verifier,
            code_challenge,
        }
    }

    pub fn from_verifier(code_verifier: impl Into<String>) -> Result<Self, PkceError> {
        let code_verifier = code_verifier.into();
        validate_code_verifier(&code_verifier)?;
        let code_challenge = code_challenge_s256(&code_verifier);
        Ok(Self {
            code_verifier,
            code_challenge,
        })
    }
}

pub fn code_challenge_s256(code_verifier: &str) -> String {
    let hash = sha256(code_verifier.as_bytes());
    base64url_nopad_encode(&hash)
}

pub fn validate_code_verifier(code_verifier: &str) -> Result<(), PkceError> {
    let len = code_verifier.len();
    if !(43..=128).contains(&len) {
        return Err(PkceError::InvalidVerifierLength(len));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = PkcePair::generate().unwrap();
        assert_eq!(pkce.code_verifier.len(), 43);
        assert_eq!(pkce.code_challenge.len(), 43);
        assert!(!pkce.code_verifier.contains('='));
        assert!(!pkce.code_challenge.contains('='));
    }

    #[test]
    fn test_deterministic_challenge() {
        let pkce = PkcePair::generate().unwrap();
        let expected_challenge = code_challenge_s256(&pkce.code_verifier);
        assert_eq!(pkce.code_challenge, expected_challenge);
    }

    #[test]
    fn test_uniqueness() {
        let pkce1 = PkcePair::generate().unwrap();
        let pkce2 = PkcePair::generate().unwrap();
        assert_ne!(pkce1.code_verifier, pkce2.code_verifier);
        assert_ne!(pkce1.code_challenge, pkce2.code_challenge);
    }

    #[test]
    fn test_rfc7636_length_requirement() {
        for _ in 0..10 {
            let pkce = PkcePair::generate().unwrap();
            assert!(pkce.code_verifier.len() >= 43);
            assert!(pkce.code_verifier.len() <= 128);
        }
    }

    #[test]
    fn test_only_unreserved_chars() {
        let pkce = PkcePair::generate().unwrap();
        for c in pkce.code_verifier.chars() {
            assert!(
                c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~',
                "code_verifier contains invalid char: {c}"
            );
        }
    }
}
