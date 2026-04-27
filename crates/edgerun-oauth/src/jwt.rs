//! Minimal JWT (RFC 7519) parsing and verification.
//!
//! Supports:
//! - HS256, ES256, RS256 signature algorithms
//! - All standard OIDC claims (`iss`, `sub`, `aud`, `exp`, `iat`, `nonce`)
//! - JWK (RFC 7517) parsing for ES256 and RS256 verification
//!
//! Does NOT support:
//! - JWT encryption (JWE)
//! - Signature creation (only verification)
//! - Non-standard algorithms

use crate::errors::{OAuthError, OAuthResult};
use edgerun_encoding::base64::base64url_decode;
use edgerun_json::{from_str, JsonValue, Map};

// ---------------------------------------------------------------------------
// JWT Header
// ---------------------------------------------------------------------------

/// Parsed JWT header.
#[derive(Debug, Clone)]
pub struct JwtHeader {
    pub alg: String,
    pub typ: Option<String>,
    pub kid: Option<String>,
}

impl JwtHeader {
    pub fn from_base64url(s: &str) -> OAuthResult<Self> {
        let bytes = base64url_decode(s)
            .map_err(|e| OAuthError::JwtError(format!("header base64url decode: {e}")))?;
        let text = String::from_utf8(bytes)
            .map_err(|e| OAuthError::JwtError(format!("header UTF-8: {e}")))?;
        let value: JsonValue =
            from_str(&text).map_err(|e| OAuthError::JwtError(format!("header JSON: {e}")))?;
        let alg = value
            .get("alg")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OAuthError::JwtError("missing 'alg' in JWT header".into()))?
            .to_string();
        let typ = value
            .get("typ")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let kid = value
            .get("kid")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Ok(Self { alg, typ, kid })
    }
}

// ---------------------------------------------------------------------------
// JWT Payload (OIDC ID Token claims)
// ---------------------------------------------------------------------------

/// Parsed JWT payload with OIDC ID Token claims.
#[derive(Debug, Clone)]
pub struct JwtPayload {
    pub iss: String,
    pub sub: String,
    pub aud: Vec<String>,
    pub exp: u64,
    pub iat: u64,
    pub auth_time: Option<u64>,
    pub nonce: Option<String>,
    pub email: Option<String>,
    pub email_verified: Option<bool>,
    pub name: Option<String>,
    pub picture: Option<String>,
    pub azp: Option<String>,
    pub acr: Option<String>,
    pub amr: Option<Vec<String>>,
    pub sid: Option<String>,
    /// Access token hash — present in code flow ID tokens (OIDC Core 1.0 §3.1.3.6).
    pub at_hash: Option<String>,
    /// Any additional claims not mapped to known fields.
    pub extra: Map,
}

impl JwtPayload {
    pub fn from_base64url(s: &str) -> OAuthResult<Self> {
        let bytes = base64url_decode(s)
            .map_err(|e| OAuthError::JwtError(format!("payload base64url decode: {e}")))?;
        let text = String::from_utf8(bytes)
            .map_err(|e| OAuthError::JwtError(format!("payload UTF-8: {e}")))?;
        let value: JsonValue =
            from_str(&text).map_err(|e| OAuthError::JwtError(format!("payload JSON: {e}")))?;

        let iss = value
            .get("iss")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let sub = value
            .get("sub")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // aud can be string or array
        let aud = match value.get("aud") {
            Some(v) if v.is_string() => vec![v.as_str().unwrap().to_string()],
            Some(v) if v.is_array() => v
                .as_array()
                .iter()
                .flat_map(|arr| arr.iter().filter_map(|x| x.as_str()).map(|s| s.to_string()))
                .collect(),
            _ => vec![],
        };

        let exp = value.get("exp").and_then(|v| v.as_u64()).unwrap_or(0);
        let iat = value.get("iat").and_then(|v| v.as_u64()).unwrap_or(0);
        let auth_time = value.get("auth_time").and_then(|v| v.as_u64());
        let nonce = value
            .get("nonce")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let email = value
            .get("email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let email_verified = value.get("email_verified").and_then(|v| v.as_bool());
        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let picture = value
            .get("picture")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let azp = value
            .get("azp")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let acr = value
            .get("acr")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let amr = value.get("amr").and_then(|v| v.as_array()).map(|arr| {
            arr.iter()
                .filter_map(|x| x.as_str())
                .map(|s| s.to_string())
                .collect()
        });
        let sid = value
            .get("sid")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let at_hash = value
            .get("at_hash")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        // Collect extra claims
        let known_keys = [
            "iss",
            "sub",
            "aud",
            "exp",
            "iat",
            "auth_time",
            "nonce",
            "email",
            "email_verified",
            "name",
            "picture",
            "azp",
            "acr",
            "amr",
            "sid",
            "at_hash",
        ];
        let mut extra = Map::new();
        if let Some(obj) = value.as_object() {
            for (k, v) in obj.iter() {
                if !known_keys.contains(&k.as_str()) {
                    extra.insert(k.clone(), v.clone());
                }
            }
        }

        Ok(Self {
            iss,
            sub,
            aud,
            exp,
            iat,
            auth_time,
            nonce,
            email,
            email_verified,
            name,
            picture,
            azp,
            acr,
            amr,
            sid,
            at_hash,
            extra,
        })
    }

    /// Check if the token is expired (with grace period).
    pub fn is_expired(&self, grace_secs: u64) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now + grace_secs >= self.exp
    }

    /// Verify the `aud` claim contains the expected client ID.
    pub fn verify_aud(&self, expected_client_id: &str) -> bool {
        self.aud.is_empty() || self.aud.iter().any(|a| a == expected_client_id)
    }

    /// Verify the `iss` claim matches the expected issuer.
    pub fn verify_iss(&self, expected_iss: &str) -> bool {
        self.iss == expected_iss
    }

    /// Verify the `nonce` claim matches the expected value.
    pub fn verify_nonce(&self, expected_nonce: Option<&str>) -> bool {
        match (expected_nonce, &self.nonce) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(a), Some(b)) => a == b,
        }
    }

    /// Verify the `at_hash` claim matches the access token.
    ///
    /// OIDC Core 1.0 §3.1.3.6: `at_hash` is the base64url encoding of the
    /// left half of the SHA-256 hash of the ASCII representation of the access token.
    pub fn verify_at_hash(&self, access_token: &str) -> bool {
        let expected_at_hash = match &self.at_hash {
            Some(h) => h,
            None => return true, // Not present — skip verification
        };

        use edgerun_crypto::sha256;
        use edgerun_encoding::base64::base64url_nopad_encode;

        let hash = sha256(access_token.as_bytes());
        let computed = base64url_nopad_encode(&hash[..16]); // left half
        computed == *expected_at_hash
    }
}

// ---------------------------------------------------------------------------
// IdToken — parsed JWT with raw components
// ---------------------------------------------------------------------------

/// A parsed OIDC ID Token (JWT).
#[derive(Debug, Clone)]
pub struct IdToken {
    pub raw: String,
    pub header: JwtHeader,
    pub payload: JwtPayload,
}

impl IdToken {
    /// Parse a raw JWT string into header + payload (no signature verification).
    pub fn parse_unverified(raw: &str) -> OAuthResult<Self> {
        let parts: Vec<&str> = raw.split('.').collect();
        if parts.len() != 3 {
            return Err(OAuthError::JwtError(format!(
                "JWT must have 3 parts, got {}",
                parts.len()
            )));
        }

        let header = JwtHeader::from_base64url(parts[0])?;
        let payload = JwtPayload::from_base64url(parts[1])?;

        Ok(Self {
            raw: raw.to_string(),
            header,
            payload,
        })
    }

    /// Verify the JWT signature using the provided verifier.
    pub fn verify(&self, verifier: &JwtVerifier) -> OAuthResult<()> {
        let parts: Vec<&str> = self.raw.split('.').collect();
        let signing_input = format!("{}.{}", parts[0], parts[1]);
        let signature_bytes = base64url_decode(parts[2])
            .map_err(|e| OAuthError::JwtError(format!("signature base64url decode: {e}")))?;

        match self.header.alg.as_str() {
            "HS256" => {
                let JwtVerifier::Hmac { key } = verifier else {
                    return Err(OAuthError::JwtError(
                        "expected HMAC verifier but got asymmetric".into(),
                    ));
                };
                let expected = edgerun_crypto::hmac_sha256(key, signing_input.as_bytes());
                if !constant_time_eq(&signature_bytes, &expected) {
                    return Err(OAuthError::JwtError(
                        "HS256 signature verification failed".into(),
                    ));
                }
            }
            "ES256" => {
                let JwtVerifier::Es256 { verifying_key } = verifier else {
                    return Err(OAuthError::JwtError(
                        "expected ES256 verifier but got HMAC".into(),
                    ));
                };
                use edgerun_crypto::p256::ecdsa::signature::Verifier;
                let sig = edgerun_crypto::p256::ecdsa::Signature::from_der(&signature_bytes)
                    .map_err(|e| OAuthError::JwtError(format!("ES256 DER parse: {e}")))?;
                verifying_key
                    .verify(signing_input.as_bytes(), &sig)
                    .map_err(|e| OAuthError::JwtError(format!("ES256 verification failed: {e}")))?;
            }
            "RS256" => {
                let JwtVerifier::Rs256 { verifying_key } = verifier else {
                    return Err(OAuthError::JwtError(
                        "expected RS256 verifier but got HMAC".into(),
                    ));
                };
                use edgerun_crypto::digest::Digest;
                use edgerun_crypto::sha2::Sha256;
                use rsa::pkcs1v15::Pkcs1v15Sign;

                let mut hasher = Sha256::new();
                hasher.update(signing_input.as_bytes());
                let digest = hasher.finalize();

                verifying_key
                    .verify(Pkcs1v15Sign::new::<Sha256>(), &digest, &signature_bytes)
                    .map_err(|e| OAuthError::JwtError(format!("RS256 verification failed: {e}")))?;
            }
            other => {
                return Err(OAuthError::JwtError(format!(
                    "unsupported JWT algorithm: {other}"
                )));
            }
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// JWT Verifier
// ---------------------------------------------------------------------------

/// A verifier for JWT signatures.
pub enum JwtVerifier {
    /// HMAC-based verification (HS256).
    Hmac { key: Vec<u8> },
    /// ECDSA P-256 SHA-256 verification (ES256).
    Es256 {
        verifying_key: edgerun_crypto::p256::ecdsa::VerifyingKey,
    },
    /// RSA PKCS#1 v1.5 SHA-256 verification (RS256).
    Rs256 {
        verifying_key: rsa::RsaPublicKey,
    },
}

impl JwtVerifier {
    /// Create an HMAC-SHA256 verifier from a shared secret.
    pub fn hmac_from_bytes(key: &[u8]) -> Self {
        JwtVerifier::Hmac { key: key.to_vec() }
    }

    /// Create an ES256 verifier from a raw P-256 public key (uncompressed, 65 bytes starting with 0x04).
    pub fn es256_from_raw_bytes(bytes: &[u8]) -> OAuthResult<Self> {
        use edgerun_crypto::elliptic_curve::sec1::FromEncodedPoint;
        use edgerun_crypto::p256::ecdsa::VerifyingKey;
        use edgerun_crypto::p256::{EncodedPoint, PublicKey};

        if bytes.len() != 65 || bytes[0] != 0x04 {
            return Err(OAuthError::JwtError(
                "ES256 public key must be 65 bytes uncompressed starting with 0x04".into(),
            ));
        }

        let x = &bytes[1..33];
        let y = &bytes[33..65];
        let point = EncodedPoint::from_affine_coordinates(
            edgerun_crypto::p256::FieldBytes::from_slice(x),
            edgerun_crypto::p256::FieldBytes::from_slice(y),
            false,
        );

        let pub_key = PublicKey::from_encoded_point(&point)
            .into_option()
            .ok_or_else(|| OAuthError::JwtError("ES256 point decode failed".into()))?;
        let verifying_key = VerifyingKey::from(&pub_key);

        Ok(JwtVerifier::Es256 { verifying_key })
    }

    /// Create an ES256 verifier from a PEM-encoded P-256 public key.
    pub fn es256_from_pem(pem: &str) -> OAuthResult<Self> {
        use edgerun_crypto::elliptic_curve::pkcs8::DecodePublicKey;
        use edgerun_crypto::p256::ecdsa::VerifyingKey;

        let verifying_key = VerifyingKey::from_public_key_pem(pem)
            .map_err(|e| OAuthError::JwtError(format!("ES256 PEM parse: {e}")))?;

        Ok(JwtVerifier::Es256 { verifying_key })
    }

    /// Create an RS256 verifier from a PEM-encoded RSA public key.
    pub fn rs256_from_pem(pem: &str) -> OAuthResult<Self> {
        use rsa::pkcs8::DecodePublicKey;
        use rsa::RsaPublicKey;

        let verifying_key = RsaPublicKey::from_public_key_pem(pem)
            .map_err(|e| OAuthError::JwtError(format!("RS256 PEM parse: {e}")))?;

        Ok(JwtVerifier::Rs256 { verifying_key })
    }

    /// Create an RS256 verifier from DER-encoded RSA public key (PKCS#1 or PKCS#8).
    pub fn rs256_from_der(der: &[u8]) -> OAuthResult<Self> {
        use rsa::pkcs8::DecodePublicKey;
        use rsa::RsaPublicKey;

        let verifying_key = RsaPublicKey::from_public_key_der(der)
            .map_err(|e| OAuthError::JwtError(format!("RS256 DER parse: {e}")))?;

        Ok(JwtVerifier::Rs256 { verifying_key })
    }
}

// ---------------------------------------------------------------------------
// JWK parsing (for JWKS endpoint)
// ---------------------------------------------------------------------------

/// Parse a JWK (RFC 7517) into a `JwtVerifier`.
///
/// Supports:
/// - `"kty": "EC"` with `"crv": "P-256"` → ES256
/// - `"kty": "RSA"` → RS256
/// - `"kty": "oct"` → HMAC
pub fn verifier_from_jwk(jwk: &JsonValue) -> OAuthResult<(String, JwtVerifier)> {
    let kty = jwk
        .get("kty")
        .and_then(|v| v.as_str())
        .ok_or_else(|| OAuthError::JwtError("JWK missing 'kty'".into()))?;
    let kid = jwk
        .get("kid")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    match kty {
        "EC" => {
            let crv = jwk
                .get("crv")
                .and_then(|v| v.as_str())
                .ok_or_else(|| OAuthError::JwtError("EC JWK missing 'crv'".into()))?;
            if crv != "P-256" {
                return Err(OAuthError::JwtError(format!("unsupported EC curve: {crv}")));
            }
            let x = jwk
                .get("x")
                .and_then(|v| v.as_str())
                .ok_or_else(|| OAuthError::JwtError("EC JWK missing 'x'".into()))?;
            let y = jwk
                .get("y")
                .and_then(|v| v.as_str())
                .ok_or_else(|| OAuthError::JwtError("EC JWK missing 'y'".into()))?;

            let x_bytes = base64url_decode(x)
                .map_err(|e| OAuthError::JwtError(format!("JWK x decode: {e}")))?;
            let y_bytes = base64url_decode(y)
                .map_err(|e| OAuthError::JwtError(format!("JWK y decode: {e}")))?;

            // Reconstruct uncompressed point: 0x04 || x || y
            let mut raw = vec![0x04];
            raw.extend_from_slice(&x_bytes);
            raw.extend_from_slice(&y_bytes);

            let verifier = JwtVerifier::es256_from_raw_bytes(&raw)?;
            Ok((kid, verifier))
        }
        "RSA" => {
            let n = jwk
                .get("n")
                .and_then(|v| v.as_str())
                .ok_or_else(|| OAuthError::JwtError("RSA JWK missing 'n'".into()))?;
            let e = jwk.get("e").and_then(|v| v.as_str()).unwrap_or("AQAB");

            let n_bytes = base64url_decode(n)
                .map_err(|e| OAuthError::JwtError(format!("JWK n decode: {e}")))?;
            let e_bytes = base64url_decode(e)
                .map_err(|e| OAuthError::JwtError(format!("JWK e decode: {e}")))?;

            // RSA PKCS#1 DER encoding of SubjectPublicKeyInfo
            // SEQUENCE { SEQUENCE { OID rsaEncryption, NULL }, BIT STRING { SEQUENCE { INTEGER n, INTEGER e } } }
            let rsa_pub_key = build_rsa_spki_der(&n_bytes, &e_bytes)
                .map_err(|e| OAuthError::JwtError(format!("RSA SPKI build: {e}")))?;

            let verifier = JwtVerifier::rs256_from_der(&rsa_pub_key)?;
            Ok((kid, verifier))
        }
        "oct" => {
            let k = jwk
                .get("k")
                .and_then(|v| v.as_str())
                .ok_or_else(|| OAuthError::JwtError("oct JWK missing 'k'".into()))?;
            let key_bytes = base64url_decode(k)
                .map_err(|e| OAuthError::JwtError(format!("JWK k decode: {e}")))?;
            Ok((kid, JwtVerifier::hmac_from_bytes(&key_bytes)))
        }
        other => Err(OAuthError::JwtError(format!(
            "unsupported JWK kty: {other}"
        ))),
    }
}

/// Build a DER-encoded RSA SubjectPublicKeyInfo from raw modulus and exponent.
fn build_rsa_spki_der(modulus: &[u8], exponent: &[u8]) -> Result<Vec<u8>, String> {
    // Manually build DER encoding since der crate's SequenceOf requires heapless feature.
    //
    // SubjectPublicKeyInfo ::= SEQUENCE {
    //     algorithm  AlgorithmIdentifier { rsaEncryption OID, NULL },
    //     subjectPublicKey BIT STRING { SEQUENCE { INTEGER n, INTEGER e } }
    // }

    // RSA OID: 1.2.840.113549.1.1.1 → encoded as 06 09 2a 86 48 86 f7 0d 01 01 01
    let rsa_oid: &[u8] = &[
        0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01,
    ];
    let null_bytes: &[u8] = &[0x05, 0x00];

    // AlgorithmIdentifier SEQUENCE
    let alg_inner_len = (rsa_oid.len() + null_bytes.len()) as u8;
    let mut alg_seq = vec![0x30, alg_inner_len];
    alg_seq.extend_from_slice(rsa_oid);
    alg_seq.extend_from_slice(null_bytes);

    // INTEGER n (modulus) — prepend 0x00 if high bit set to keep it positive
    let n_bytes = if modulus[0] & 0x80 != 0 {
        let mut tmp = vec![0x00];
        tmp.extend_from_slice(modulus);
        tmp
    } else {
        modulus.to_vec()
    };
    let n_der = der_integer(&n_bytes);

    // INTEGER e (exponent)
    let e_der = der_integer(exponent);

    // SEQUENCE { INTEGER n, INTEGER e }
    let seq_content_len = n_der.len() + e_der.len();
    let (seq_tag, seq_len) = der_tag_len(0x30, seq_content_len);
    let mut inner_seq = Vec::with_capacity(seq_tag + seq_len + seq_content_len);
    inner_seq.push(0x30);
    der_write_len(&mut inner_seq, seq_content_len);
    inner_seq.extend_from_slice(&n_der);
    inner_seq.extend_from_slice(&e_der);

    // BIT STRING wrapping the SEQUENCE
    let bs_content_len = inner_seq.len() + 1; // +1 for unused-bits byte
    let (bs_tag, bs_len) = der_tag_len(0x03, bs_content_len);
    let mut bit_string = Vec::with_capacity(bs_tag + bs_len + bs_content_len);
    bit_string.push(0x03);
    der_write_len(&mut bit_string, bs_content_len);
    bit_string.push(0x00); // 0 unused bits
    bit_string.extend_from_slice(&inner_seq);

    // Outer SEQUENCE { AlgorithmIdentifier, BIT STRING }
    let outer_len = alg_seq.len() + bit_string.len();
    let mut spki = Vec::with_capacity(2 + outer_len);
    spki.push(0x30);
    der_write_len(&mut spki, outer_len);
    spki.extend_from_slice(&alg_seq);
    spki.extend_from_slice(&bit_string);

    Ok(spki)
}

fn der_integer(bytes: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(2 + bytes.len());
    result.push(0x02); // INTEGER tag
    der_write_len(&mut result, bytes.len());
    result.extend_from_slice(bytes);
    result
}

fn der_write_len(buf: &mut Vec<u8>, len: usize) {
    if len < 0x80 {
        buf.push(len as u8);
    } else if len < 0x100 {
        buf.push(0x81);
        buf.push(len as u8);
    } else if len < 0x10000 {
        buf.push(0x82);
        buf.push((len >> 8) as u8);
        buf.push(len as u8);
    } else {
        buf.push(0x83);
        buf.push((len >> 16) as u8);
        buf.push((len >> 8) as u8);
        buf.push(len as u8);
    }
}

fn der_tag_len(_tag: u8, len: usize) -> (usize, usize) {
    let len_bytes = if len < 0x80 {
        1
    } else if len < 0x100 {
        2
    } else if len < 0x10000 {
        3
    } else {
        4
    };
    (1, len_bytes)
}

// ---------------------------------------------------------------------------
// Constant-time equality
// ---------------------------------------------------------------------------

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_encoding::base64::base64url_nopad_encode;

    // Manually constructed JWT with valid base64url encoding
    const TEST_JWT: &str = "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.sig";

    #[test]
    fn test_parse_unverified_header() {
        let token = IdToken::parse_unverified(TEST_JWT).unwrap();
        assert_eq!(token.header.alg, "HS256");
        assert!(token.header.kid.is_none());
    }

    #[test]
    fn test_parse_unverified_payload() {
        let token = IdToken::parse_unverified(TEST_JWT).unwrap();
        assert_eq!(token.payload.sub, "1234567890");
        assert_eq!(token.payload.name.as_deref(), Some("John Doe"));
        assert_eq!(token.payload.iat, 1516239022);
        assert!(token.payload.iss.is_empty());
        assert!(token.payload.aud.is_empty());
    }

    #[test]
    fn test_expired_check() {
        let header = base64url_nopad_encode(br#"{"alg":"HS256"}"#);
        let payload = base64url_nopad_encode(br#"{"sub":"test","exp":1516239022}"#);
        let jwt = format!("{header}.{payload}.sig");
        let token = IdToken::parse_unverified(&jwt).unwrap();
        assert!(token.payload.is_expired(0));
    }

    #[test]
    fn test_not_expired_with_far_future_exp() {
        let header = base64url_nopad_encode(br#"{"alg":"HS256"}"#);
        let payload = base64url_nopad_encode(br#"{"sub":"test","exp":9999999999}"#);
        let jwt = format!("{header}.{payload}.sig");
        let token = IdToken::parse_unverified(&jwt).unwrap();
        assert!(!token.payload.is_expired(0));
    }

    #[test]
    fn test_invalid_jwt_wrong_parts() {
        // Valid base64url with valid JSON but missing required JWT fields
        let header = base64url_nopad_encode(br#"{"a":1}"#);
        let payload = base64url_nopad_encode(br#"{"sub":"test"}"#);
        let jwt = format!("{header}.{payload}.sig");
        assert!(IdToken::parse_unverified(&jwt).is_ok());
        assert!(IdToken::parse_unverified("only.two").is_err());
        assert!(IdToken::parse_unverified("one").is_err());
    }

    #[test]
    fn test_aud_string_claim() {
        let header = base64url_nopad_encode(br#"{"alg":"HS256"}"#);
        let payload =
            base64url_nopad_encode(br#"{"sub":"user1","aud":"my-client","exp":9999999999}"#);
        let jwt = format!("{header}.{payload}.sig");
        let token = IdToken::parse_unverified(&jwt).unwrap();
        assert_eq!(token.payload.aud, vec!["my-client"]);
        assert!(token.payload.verify_aud("my-client"));
        assert!(!token.payload.verify_aud("other-client"));
    }

    #[test]
    fn test_aud_array_claim() {
        let header = base64url_nopad_encode(br#"{"alg":"HS256"}"#);
        let payload = base64url_nopad_encode(
            br#"{"sub":"user1","aud":["client-a","client-b"],"exp":9999999999}"#,
        );
        let jwt = format!("{header}.{payload}.sig");
        let token = IdToken::parse_unverified(&jwt).unwrap();
        assert_eq!(token.payload.aud, vec!["client-a", "client-b"]);
        assert!(token.payload.verify_aud("client-a"));
        assert!(token.payload.verify_aud("client-b"));
        assert!(!token.payload.verify_aud("client-c"));
    }

    #[test]
    fn test_nonce_verification() {
        let header = base64url_nopad_encode(br#"{"alg":"HS256"}"#);
        let payload =
            base64url_nopad_encode(br#"{"sub":"user1","nonce":"abc123","exp":9999999999}"#);
        let jwt = format!("{header}.{payload}.sig");
        let token = IdToken::parse_unverified(&jwt).unwrap();

        assert!(token.payload.verify_nonce(Some("abc123")));
        assert!(!token.payload.verify_nonce(Some("wrong")));
        // If caller didn't expect a nonce, verification passes
        // (can't verify what wasn't requested)
        assert!(token.payload.verify_nonce(None));
    }

    #[test]
    fn test_nonce_absent() {
        let header = base64url_nopad_encode(br#"{"alg":"HS256"}"#);
        let payload = base64url_nopad_encode(br#"{"sub":"user1","exp":9999999999}"#);
        let jwt = format!("{header}.{payload}.sig");
        let token = IdToken::parse_unverified(&jwt).unwrap();

        assert!(token.payload.verify_nonce(None));
        assert!(!token.payload.verify_nonce(Some("any")));
    }

    #[test]
    fn test_at_hash_parsing() {
        let header = base64url_nopad_encode(br#"{"alg":"ES256"}"#);
        let payload =
            base64url_nopad_encode(br#"{"sub":"user1","at_hash":"abc123","exp":9999999999}"#);
        let jwt = format!("{header}.{payload}.sig");
        let token = IdToken::parse_unverified(&jwt).unwrap();
        assert_eq!(token.payload.at_hash.as_deref(), Some("abc123"));
    }

    #[test]
    fn test_at_hash_verification() {
        use edgerun_crypto::sha256;

        let access_token = "test-access-token-12345";
        let hash = sha256(access_token.as_bytes());
        let at_hash = base64url_nopad_encode(&hash[..16]);

        let header = base64url_nopad_encode(br#"{"alg":"ES256"}"#);
        let payload_str = format!(r#"{{"sub":"user1","at_hash":"{at_hash}","exp":9999999999}}"#);
        let payload_b64 = base64url_nopad_encode(payload_str.as_bytes());
        let header_b64 = base64url_nopad_encode(br#"{"alg":"ES256"}"#);
        let jwt = format!("{header_b64}.{payload_b64}.sig");
        let token = IdToken::parse_unverified(&jwt).unwrap();

        assert!(token.payload.verify_at_hash(access_token));
        assert!(!token.payload.verify_at_hash("wrong-token"));
    }

    #[test]
    fn test_at_hash_absent_allows_anything() {
        let header = base64url_nopad_encode(br#"{"alg":"ES256"}"#);
        let payload = base64url_nopad_encode(br#"{"sub":"user1","exp":9999999999}"#);
        let jwt = format!("{header}.{payload}.sig");
        let token = IdToken::parse_unverified(&jwt).unwrap();

        assert!(token.payload.verify_at_hash("anything"));
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"hello", b"hello"));
        assert!(!constant_time_eq(b"hello", b"world"));
        assert!(!constant_time_eq(b"hello", b"hello!"));
        assert!(!constant_time_eq(b"", b"a"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn test_extra_claims_captured() {
        let header = base64url_nopad_encode(br#"{"alg":"HS256"}"#);
        let payload =
            base64url_nopad_encode(br#"{"sub":"u","exp":9999999999,"custom_key":"custom_val"}"#);
        let jwt = format!("{header}.{payload}.sig");
        let token = IdToken::parse_unverified(&jwt).unwrap();
        assert!(token.payload.extra.get("custom_key").is_some());
    }
}
