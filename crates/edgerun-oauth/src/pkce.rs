//! PKCE (Proof Key for Code Exchange) — RFC 7636.
//!
//! Generates a `code_verifier` and derives the `code_challenge` via SHA-256.

use crate::base64url::base64url_nopad_encode;
use edgerun_crypto::getrandom;
use edgerun_crypto::sha256;

/// A PKCE code-verifier / code-challenge pair.
#[derive(Debug, Clone)]
pub struct PkcePair {
    pub code_verifier: String,
    pub code_challenge: String,
}

impl PkcePair {
    /// Generate a new PKCE pair with 32 bytes of randomness (256-bit entropy).
    ///
    /// RFC 7636: code_verifier must be 43-128 characters of unreserved characters.
    /// 32 bytes → 43 base64url characters (no padding).
    pub fn generate() -> std::io::Result<Self> {
        let mut code_verifier_bytes = [0u8; 32];
        getrandom::fill(&mut code_verifier_bytes).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("random generation failed: {e}"))
        })?;
        let code_verifier = base64url_nopad_encode(&code_verifier_bytes);

        // code_challenge = BASE64URL(SHA256(code_verifier))
        let hash = sha256(code_verifier.as_bytes());
        let code_challenge = base64url_nopad_encode(&hash);

        Ok(PkcePair {
            code_verifier,
            code_challenge,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pkce_generation() {
        let pkce = PkcePair::generate().unwrap();
        // 32 bytes → 43 base64url chars (no padding)
        assert_eq!(pkce.code_verifier.len(), 43);
        assert_eq!(pkce.code_challenge.len(), 43);
        assert!(!pkce.code_verifier.contains('='));
        assert!(!pkce.code_challenge.contains('='));
    }

    #[test]
    fn test_deterministic_challenge() {
        let pkce = PkcePair::generate().unwrap();
        let expected_hash = sha256(pkce.code_verifier.as_bytes());
        let expected_challenge = base64url_nopad_encode(&expected_hash);
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
        // RFC 7636: code_verifier must be 43-128 characters
        for _ in 0..10 {
            let pkce = PkcePair::generate().unwrap();
            assert!(pkce.code_verifier.len() >= 43);
            assert!(pkce.code_verifier.len() <= 128);
        }
    }

    #[test]
    fn test_only_unreserved_chars() {
        // RFC 7636: ALPHA / DIGIT / "-" / "." / "_" / "~"
        let pkce = PkcePair::generate().unwrap();
        for c in pkce.code_verifier.chars() {
            assert!(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~',
                "code_verifier contains invalid char: {c}");
        }
    }
}
