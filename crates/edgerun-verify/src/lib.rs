#![no_std]

extern crate alloc;

use alloc::vec::Vec;

/// EdgeRun canonical signature envelope magic.
pub const SIGNATURE_ENVELOPE_MAGIC: &[u8; 6] = b"ERSIG1";

/// Domain used for EdgeRun signed payload verification.
pub const DEFAULT_DOMAIN: &[u8] = b"edgerun:v0:signed-envelope";

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureAlgorithm {
    Ed25519 = 1,
    P256Sha256Asn1Der = 2,
    P256Sha256Fixed = 3,
    RsaPssSha256 = 4,
    RsaPkcs1Sha256 = 5,
    HmacSha256 = 6,
    DevInsecureSha256 = 255,
}

impl SignatureAlgorithm {
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            1 => Some(Self::Ed25519),
            2 => Some(Self::P256Sha256Asn1Der),
            3 => Some(Self::P256Sha256Fixed),
            4 => Some(Self::RsaPssSha256),
            5 => Some(Self::RsaPkcs1Sha256),
            6 => Some(Self::HmacSha256),
            255 => Some(Self::DevInsecureSha256),
            _ => None,
        }
    }

    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignaturePurpose {
    Unknown = 0,
    Event = 1,
    ObjectDescriptor = 2,
    CapabilityGrant = 3,
    CapabilityUse = 4,
    NodeProvisioning = 5,
    AppManifest = 6,
    BinaryRelease = 7,
    DevTest = 65535,
}

impl SignaturePurpose {
    pub fn from_u16(value: u16) -> Self {
        match value {
            1 => Self::Event,
            2 => Self::ObjectDescriptor,
            3 => Self::CapabilityGrant,
            4 => Self::CapabilityUse,
            5 => Self::NodeProvisioning,
            6 => Self::AppManifest,
            7 => Self::BinaryRelease,
            65535 => Self::DevTest,
            _ => Self::Unknown,
        }
    }

    pub fn as_u16(self) -> u16 {
        self as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CryptoStrength {
    None,
    DevOnly,
    WeakSymmetric,
    SoftwareAsymmetric,
    HardwareCompatibleAsymmetric,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyError {
    MalformedEnvelope,
    UnsupportedAlgorithm,
    InvalidKey,
    InvalidSignature,
    FeatureDisabled,
    LengthOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedEnvelopeRef<'a> {
    pub version: u8,
    pub algorithm: SignatureAlgorithm,
    pub purpose: SignaturePurpose,
    pub flags: u16,
    pub key_id: &'a [u8],
    pub public_key_or_ref: &'a [u8],
    pub signature: &'a [u8],
    pub payload: &'a [u8],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedSignature {
    pub algorithm: SignatureAlgorithm,
    pub purpose: SignaturePurpose,
    pub strength: CryptoStrength,
    pub key_id: Vec<u8>,
    pub signer_id: [u8; 32],
    pub payload_hash: [u8; 32],
}

/// Parse an EdgeRun canonical signature envelope.
///
/// Format:
/// - magic: 6 bytes = ERSIG1
/// - version: u8
/// - algorithm: u16 little-endian
/// - purpose: u16 little-endian
/// - flags: u16 little-endian
/// - key_id_len: u16 little-endian
/// - public_key_or_ref_len: u16 little-endian
/// - signature_len: u16 little-endian
/// - payload_len: u64 little-endian
/// - key_id bytes
/// - public_key_or_ref bytes
/// - signature bytes
/// - payload bytes
pub fn parse_envelope(input: &[u8]) -> Result<SignedEnvelopeRef<'_>, VerifyError> {
    const HEADER_LEN: usize = 6 + 1 + 2 + 2 + 2 + 2 + 2 + 2 + 8;
    if input.len() < HEADER_LEN {
        return Err(VerifyError::MalformedEnvelope);
    }
    if &input[..6] != SIGNATURE_ENVELOPE_MAGIC {
        return Err(VerifyError::MalformedEnvelope);
    }

    let version = input[6];
    let algorithm = SignatureAlgorithm::from_u16(read_u16_le(input, 7)?)
        .ok_or(VerifyError::UnsupportedAlgorithm)?;
    let purpose = SignaturePurpose::from_u16(read_u16_le(input, 9)?);
    let flags = read_u16_le(input, 11)?;
    let key_id_len = read_u16_le(input, 13)? as usize;
    let public_key_or_ref_len = read_u16_le(input, 15)? as usize;
    let signature_len = read_u16_le(input, 17)? as usize;
    let payload_len = read_u64_le(input, 19)? as usize;

    let body_len = key_id_len
        .checked_add(public_key_or_ref_len)
        .and_then(|n| n.checked_add(signature_len))
        .and_then(|n| n.checked_add(payload_len))
        .ok_or(VerifyError::LengthOverflow)?;
    let total_len = HEADER_LEN
        .checked_add(body_len)
        .ok_or(VerifyError::LengthOverflow)?;
    if input.len() != total_len {
        return Err(VerifyError::MalformedEnvelope);
    }

    let mut pos = HEADER_LEN;
    let key_id = take(input, &mut pos, key_id_len)?;
    let public_key_or_ref = take(input, &mut pos, public_key_or_ref_len)?;
    let signature = take(input, &mut pos, signature_len)?;
    let payload = take(input, &mut pos, payload_len)?;

    Ok(SignedEnvelopeRef {
        version,
        algorithm,
        purpose,
        flags,
        key_id,
        public_key_or_ref,
        signature,
        payload,
    })
}

/// Build the canonical signed bytes for an envelope.
///
/// The payload itself is not directly signed. The payload hash is signed, so
/// verifiers can stream/hash large payloads and then verify a compact message.
pub fn canonical_signed_message(
    domain: &[u8],
    envelope: &SignedEnvelopeRef<'_>,
    payload_hash: &[u8; 32],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(
        domain.len()
            + 1
            + 2
            + 2
            + 2
            + envelope.key_id.len()
            + envelope.public_key_or_ref.len()
            + payload_hash.len(),
    );
    out.extend_from_slice(domain);
    out.push(envelope.version);
    out.extend_from_slice(&envelope.algorithm.as_u16().to_le_bytes());
    out.extend_from_slice(&envelope.purpose.as_u16().to_le_bytes());
    out.extend_from_slice(&envelope.flags.to_le_bytes());
    out.extend_from_slice(envelope.key_id);
    out.extend_from_slice(envelope.public_key_or_ref);
    out.extend_from_slice(payload_hash);
    out
}

pub fn verify_envelope(input: &[u8]) -> Result<VerifiedSignature, VerifyError> {
    let envelope = parse_envelope(input)?;
    verify(&envelope, DEFAULT_DOMAIN)
}

pub fn verify(envelope: &SignedEnvelopeRef<'_>, domain: &[u8]) -> Result<VerifiedSignature, VerifyError> {
    let payload_hash = edgerun_crypto::sha256(envelope.payload);
    let message = canonical_signed_message(domain, envelope, &payload_hash);
    verify_prehashed_message(envelope, &payload_hash, &message)
}

pub fn verify_prehashed_message(
    envelope: &SignedEnvelopeRef<'_>,
    payload_hash: &[u8; 32],
    message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    match envelope.algorithm {
        SignatureAlgorithm::Ed25519 => verify_ed25519(envelope, payload_hash, message),
        SignatureAlgorithm::P256Sha256Asn1Der => verify_p256_der(envelope, payload_hash, message),
        SignatureAlgorithm::P256Sha256Fixed => verify_p256_fixed(envelope, payload_hash, message),
        SignatureAlgorithm::RsaPssSha256 => verify_rsa_pss_sha256(envelope, payload_hash, message),
        SignatureAlgorithm::RsaPkcs1Sha256 => verify_rsa_pkcs1_sha256(envelope, payload_hash, message),
        SignatureAlgorithm::HmacSha256 => verify_hmac_sha256(envelope, payload_hash, message),
        SignatureAlgorithm::DevInsecureSha256 => verify_dev_insecure_sha256(envelope, payload_hash, message),
    }
}

#[cfg(feature = "ed25519")]
fn verify_ed25519(
    envelope: &SignedEnvelopeRef<'_>,
    payload_hash: &[u8; 32],
    message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    let public_key: [u8; 32] = envelope
        .public_key_or_ref
        .try_into()
        .map_err(|_| VerifyError::InvalidKey)?;
    if envelope.signature.len() != 64 {
        return Err(VerifyError::InvalidSignature);
    }
    let verifying_key = edgerun_crypto::ed25519_dalek::VerifyingKey::from_bytes(&public_key)
        .map_err(|_| VerifyError::InvalidKey)?;
    let signature = edgerun_crypto::ed25519_dalek::Signature::from_slice(envelope.signature)
        .map_err(|_| VerifyError::InvalidSignature)?;
    use edgerun_crypto::ed25519_dalek::Verifier;
    verifying_key
        .verify(message, &signature)
        .map_err(|_| VerifyError::InvalidSignature)?;
    Ok(verified(
        envelope,
        payload_hash,
        CryptoStrength::SoftwareAsymmetric,
    ))
}

#[cfg(not(feature = "ed25519"))]
fn verify_ed25519(
    _envelope: &SignedEnvelopeRef<'_>,
    _payload_hash: &[u8; 32],
    _message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    Err(VerifyError::FeatureDisabled)
}

#[cfg(feature = "p256")]
fn verify_p256_der(
    envelope: &SignedEnvelopeRef<'_>,
    payload_hash: &[u8; 32],
    message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    use edgerun_crypto::p256::ecdsa::{Signature, VerifyingKey};
    use edgerun_crypto::p256::ecdsa::signature::Verifier;

    let verifying_key = VerifyingKey::from_sec1_bytes(envelope.public_key_or_ref)
        .map_err(|_| VerifyError::InvalidKey)?;
    let signature = Signature::from_der(envelope.signature)
        .map_err(|_| VerifyError::InvalidSignature)?;
    verifying_key
        .verify(message, &signature)
        .map_err(|_| VerifyError::InvalidSignature)?;
    Ok(verified(
        envelope,
        payload_hash,
        CryptoStrength::HardwareCompatibleAsymmetric,
    ))
}

#[cfg(not(feature = "p256"))]
fn verify_p256_der(
    _envelope: &SignedEnvelopeRef<'_>,
    _payload_hash: &[u8; 32],
    _message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    Err(VerifyError::FeatureDisabled)
}

#[cfg(feature = "p256")]
fn verify_p256_fixed(
    envelope: &SignedEnvelopeRef<'_>,
    payload_hash: &[u8; 32],
    message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    use edgerun_crypto::p256::ecdsa::{Signature, VerifyingKey};
    use edgerun_crypto::p256::ecdsa::signature::Verifier;

    let public_key = if envelope.public_key_or_ref.len() == 64 {
        let mut sec1 = Vec::with_capacity(65);
        sec1.push(0x04);
        sec1.extend_from_slice(envelope.public_key_or_ref);
        sec1
    } else {
        envelope.public_key_or_ref.to_vec()
    };
    let verifying_key = VerifyingKey::from_sec1_bytes(&public_key)
        .map_err(|_| VerifyError::InvalidKey)?;
    let signature = Signature::from_slice(envelope.signature)
        .map_err(|_| VerifyError::InvalidSignature)?;
    verifying_key
        .verify(message, &signature)
        .map_err(|_| VerifyError::InvalidSignature)?;
    Ok(verified(
        envelope,
        payload_hash,
        CryptoStrength::HardwareCompatibleAsymmetric,
    ))
}

#[cfg(not(feature = "p256"))]
fn verify_p256_fixed(
    _envelope: &SignedEnvelopeRef<'_>,
    _payload_hash: &[u8; 32],
    _message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    Err(VerifyError::FeatureDisabled)
}

#[cfg(feature = "rsa")]
fn verify_rsa_pss_sha256(
    envelope: &SignedEnvelopeRef<'_>,
    payload_hash: &[u8; 32],
    message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    use edgerun_crypto::rsa::pkcs1::DecodeRsaPublicKey;
    use edgerun_crypto::rsa::pss::{Signature, VerifyingKey};
    use edgerun_crypto::rsa::signature::Verifier;

    let public_key = edgerun_crypto::rsa::RsaPublicKey::from_pkcs1_der(envelope.public_key_or_ref)
        .map_err(|_| VerifyError::InvalidKey)?;
    let signature = Signature::try_from(envelope.signature)
        .map_err(|_| VerifyError::InvalidSignature)?;
    let verifying_key = VerifyingKey::<edgerun_crypto::sha2::Sha256>::new(public_key);
    verifying_key
        .verify(message, &signature)
        .map_err(|_| VerifyError::InvalidSignature)?;
    Ok(verified(
        envelope,
        payload_hash,
        CryptoStrength::HardwareCompatibleAsymmetric,
    ))
}

#[cfg(not(feature = "rsa"))]
fn verify_rsa_pss_sha256(
    _envelope: &SignedEnvelopeRef<'_>,
    _payload_hash: &[u8; 32],
    _message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    Err(VerifyError::FeatureDisabled)
}

#[cfg(feature = "rsa")]
fn verify_rsa_pkcs1_sha256(
    envelope: &SignedEnvelopeRef<'_>,
    payload_hash: &[u8; 32],
    message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    use edgerun_crypto::rsa::pkcs1::DecodeRsaPublicKey;
    use edgerun_crypto::rsa::pkcs1v15::{Signature, VerifyingKey};
    use edgerun_crypto::rsa::signature::Verifier;

    let public_key = edgerun_crypto::rsa::RsaPublicKey::from_pkcs1_der(envelope.public_key_or_ref)
        .map_err(|_| VerifyError::InvalidKey)?;
    let signature = Signature::try_from(envelope.signature)
        .map_err(|_| VerifyError::InvalidSignature)?;
    let verifying_key = VerifyingKey::<edgerun_crypto::sha2::Sha256>::new(public_key);
    verifying_key
        .verify(message, &signature)
        .map_err(|_| VerifyError::InvalidSignature)?;
    Ok(verified(
        envelope,
        payload_hash,
        CryptoStrength::HardwareCompatibleAsymmetric,
    ))
}

#[cfg(not(feature = "rsa"))]
fn verify_rsa_pkcs1_sha256(
    _envelope: &SignedEnvelopeRef<'_>,
    _payload_hash: &[u8; 32],
    _message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    Err(VerifyError::FeatureDisabled)
}

#[cfg(feature = "hmac")]
fn verify_hmac_sha256(
    envelope: &SignedEnvelopeRef<'_>,
    payload_hash: &[u8; 32],
    message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    // Symmetric verification: public_key_or_ref is the verification secret or
    // a target-specific resolved secret. This is intentionally low trust.
    let expected = edgerun_crypto::hmac_sha256(envelope.public_key_or_ref, message);
    if !constant_time_eq(&expected, envelope.signature) {
        return Err(VerifyError::InvalidSignature);
    }
    Ok(verified(envelope, payload_hash, CryptoStrength::WeakSymmetric))
}

#[cfg(not(feature = "hmac"))]
fn verify_hmac_sha256(
    _envelope: &SignedEnvelopeRef<'_>,
    _payload_hash: &[u8; 32],
    _message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    Err(VerifyError::FeatureDisabled)
}

fn verify_dev_insecure_sha256(
    envelope: &SignedEnvelopeRef<'_>,
    payload_hash: &[u8; 32],
    message: &[u8],
) -> Result<VerifiedSignature, VerifyError> {
    // Development-only mode. The signature is SHA-256(public_key_or_ref || message).
    // This is not a secure signature scheme; it exists so tiny/dev targets can
    // participate with an explicit low trust level.
    let mut input = Vec::with_capacity(envelope.public_key_or_ref.len() + message.len());
    input.extend_from_slice(envelope.public_key_or_ref);
    input.extend_from_slice(message);
    let expected = edgerun_crypto::sha256(&input);
    if !constant_time_eq(&expected, envelope.signature) {
        return Err(VerifyError::InvalidSignature);
    }
    Ok(verified(envelope, payload_hash, CryptoStrength::DevOnly))
}

fn verified(
    envelope: &SignedEnvelopeRef<'_>,
    payload_hash: &[u8; 32],
    strength: CryptoStrength,
) -> VerifiedSignature {
    let signer_id = edgerun_crypto::sha256(envelope.public_key_or_ref);
    VerifiedSignature {
        algorithm: envelope.algorithm,
        purpose: envelope.purpose,
        strength,
        key_id: envelope.key_id.to_vec(),
        signer_id,
        payload_hash: *payload_hash,
    }
}

fn read_u16_le(input: &[u8], offset: usize) -> Result<u16, VerifyError> {
    let bytes = input
        .get(offset..offset + 2)
        .ok_or(VerifyError::MalformedEnvelope)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u64_le(input: &[u8], offset: usize) -> Result<u64, VerifyError> {
    let bytes = input
        .get(offset..offset + 8)
        .ok_or(VerifyError::MalformedEnvelope)?;
    Ok(u64::from_le_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

fn take<'a>(input: &'a [u8], pos: &mut usize, len: usize) -> Result<&'a [u8], VerifyError> {
    let end = pos.checked_add(len).ok_or(VerifyError::LengthOverflow)?;
    let out = input.get(*pos..end).ok_or(VerifyError::MalformedEnvelope)?;
    *pos = end;
    Ok(out)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    fn encode_test_envelope(
        algorithm: SignatureAlgorithm,
        purpose: SignaturePurpose,
        key_id: &[u8],
        public_key_or_ref: &[u8],
        signature: &[u8],
        payload: &[u8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(SIGNATURE_ENVELOPE_MAGIC);
        out.push(1);
        out.extend_from_slice(&algorithm.as_u16().to_le_bytes());
        out.extend_from_slice(&purpose.as_u16().to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&(key_id.len() as u16).to_le_bytes());
        out.extend_from_slice(&(public_key_or_ref.len() as u16).to_le_bytes());
        out.extend_from_slice(&(signature.len() as u16).to_le_bytes());
        out.extend_from_slice(&(payload.len() as u64).to_le_bytes());
        out.extend_from_slice(key_id);
        out.extend_from_slice(public_key_or_ref);
        out.extend_from_slice(signature);
        out.extend_from_slice(payload);
        out
    }

    #[test]
    fn parse_roundtrip() {
        let encoded = encode_test_envelope(
            SignatureAlgorithm::DevInsecureSha256,
            SignaturePurpose::DevTest,
            b"key-1",
            b"dev-secret",
            &[0u8; 32],
            b"payload",
        );
        let envelope = parse_envelope(&encoded).unwrap();
        assert_eq!(envelope.version, 1);
        assert_eq!(envelope.algorithm, SignatureAlgorithm::DevInsecureSha256);
        assert_eq!(envelope.purpose, SignaturePurpose::DevTest);
        assert_eq!(envelope.key_id, b"key-1");
        assert_eq!(envelope.public_key_or_ref, b"dev-secret");
        assert_eq!(envelope.signature, &[0u8; 32]);
        assert_eq!(envelope.payload, b"payload");
    }

    #[test]
    fn rejects_bad_magic() {
        let encoded = b"BAD";
        assert_eq!(parse_envelope(encoded), Err(VerifyError::MalformedEnvelope));
    }

    #[test]
    fn dev_insecure_signature_verifies_with_low_strength() {
        let public_key_or_ref = b"dev-secret";
        let payload = b"payload";
        let mut placeholder = encode_test_envelope(
            SignatureAlgorithm::DevInsecureSha256,
            SignaturePurpose::DevTest,
            b"key-1",
            public_key_or_ref,
            &[],
            payload,
        );
        let envelope = parse_envelope(&placeholder).unwrap();
        let payload_hash = edgerun_crypto::sha256(envelope.payload);
        let message = canonical_signed_message(DEFAULT_DOMAIN, &envelope, &payload_hash);
        let mut input = Vec::new();
        input.extend_from_slice(public_key_or_ref);
        input.extend_from_slice(&message);
        let signature = edgerun_crypto::sha256(&input);
        placeholder = encode_test_envelope(
            SignatureAlgorithm::DevInsecureSha256,
            SignaturePurpose::DevTest,
            b"key-1",
            public_key_or_ref,
            &signature,
            payload,
        );
        let verified = verify_envelope(&placeholder).unwrap();
        assert_eq!(verified.strength, CryptoStrength::DevOnly);
        assert_eq!(verified.purpose, SignaturePurpose::DevTest);
    }
}
