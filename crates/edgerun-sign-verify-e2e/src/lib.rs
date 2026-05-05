#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use edgerun_core::crypto::{self, SigningKey};
use edgerun_core::protocol::{EventEnvelope, ProtocolRecord, Signature};
use edgerun_sign_p256::P256ProtocolSigner;
use edgerun_verify::{verify_event_envelope, ProtocolFamily, ProtocolSignerRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E2eError {
    Sign,
    Verify,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignVerifyReport {
    pub signature_len: usize,
    pub record_hash_len: usize,
    pub public_key_len: usize,
    pub stream_id_len: usize,
}

pub fn deterministic_signing_key(seed_byte: u8) -> SigningKey {
    let bytes: [u8; 32] = [seed_byte; 32];
    SigningKey::from_bytes(&bytes.into()).expect("deterministic test key is valid")
}

pub fn sample_event(public_key_raw64: &[u8; 64], seq: u64) -> EventEnvelope {
    EventEnvelope {
        envelope_version: 1,
        stream_id: public_key_raw64.to_vec(),
        seq,
        event_version: 1,
        ..EventEnvelope::default()
    }
}

pub fn sign_verify_event_roundtrip() -> Result<SignVerifyReport, E2eError> {
    let signing_key = deterministic_signing_key(11);
    let signer = P256ProtocolSigner::new(signing_key);
    let public_key = crypto::verifying_key_to_node_id(&signer.verifying_key());
    let mut event = sample_event(&public_key, 1);

    let signed = signer
        .sign_record(
            &ProtocolRecord::EventEnvelope(event.clone()),
            ProtocolFamily::EventEnvelope,
        )
        .map_err(|_| E2eError::Sign)?;

    event.signature = Some(signed.signature.clone());

    let verified = verify_event_envelope(&event, ProtocolSignerRef::P256Raw64(&public_key))
        .map_err(|_| E2eError::Verify)?;

    if verified.record_hash != signed.record_hash {
        return Err(E2eError::Verify);
    }

    Ok(SignVerifyReport {
        signature_len: signed.signature.value.len(),
        record_hash_len: signed.record_hash.len(),
        public_key_len: public_key.len(),
        stream_id_len: event.stream_id.len(),
    })
}

#[no_mangle]
pub extern "C" fn edgerun_sign_verify_e2e_roundtrip() -> u32 {
    match sign_verify_event_roundtrip() {
        Ok(report) => {
            if report.signature_len == crypto::ECDSA_P256_SIGNATURE_LEN
                && report.record_hash_len == 32
                && report.public_key_len == crypto::ECDSA_P256_PUBLIC_KEY_LEN
                && report.stream_id_len == crypto::ECDSA_P256_PUBLIC_KEY_LEN
            {
                0
            } else {
                2
            }
        }
        Err(E2eError::Sign) => 10,
        Err(E2eError::Verify) => 11,
    }
}

#[no_mangle]
pub extern "C" fn edgerun_sign_verify_e2e_signature_len() -> u32 {
    crypto::ECDSA_P256_SIGNATURE_LEN as u32
}

#[no_mangle]
pub extern "C" fn edgerun_sign_verify_e2e_public_key_len() -> u32 {
    crypto::ECDSA_P256_PUBLIC_KEY_LEN as u32
}

pub fn sign_event_only(iterations: usize) -> Result<Vec<Signature>, E2eError> {
    let signing_key = deterministic_signing_key(12);
    let signer = P256ProtocolSigner::new(signing_key);
    let public_key = crypto::verifying_key_to_node_id(&signer.verifying_key());
    let mut signatures = Vec::with_capacity(iterations);

    for seq in 0..iterations as u64 {
        let event = sample_event(&public_key, seq);
        let signed = signer
            .sign_record(
                &ProtocolRecord::EventEnvelope(event),
                ProtocolFamily::EventEnvelope,
            )
            .map_err(|_| E2eError::Sign)?;
        signatures.push(signed.signature);
    }

    Ok(signatures)
}

pub fn verify_event_only(iterations: usize) -> Result<usize, E2eError> {
    let signing_key = deterministic_signing_key(13);
    let signer = P256ProtocolSigner::new(signing_key);
    let public_key = crypto::verifying_key_to_node_id(&signer.verifying_key());
    let mut ok = 0usize;

    for seq in 0..iterations as u64 {
        let mut event = sample_event(&public_key, seq);
        let signed = signer
            .sign_record(
                &ProtocolRecord::EventEnvelope(event.clone()),
                ProtocolFamily::EventEnvelope,
            )
            .map_err(|_| E2eError::Sign)?;
        event.signature = Some(signed.signature);
        verify_event_envelope(&event, ProtocolSignerRef::P256Raw64(&public_key))
            .map_err(|_| E2eError::Verify)?;
        ok += 1;
    }

    Ok(ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_works() {
        let report = sign_verify_event_roundtrip().unwrap();
        assert_eq!(report.signature_len, 64);
        assert_eq!(report.record_hash_len, 32);
        assert_eq!(report.public_key_len, 64);
        assert_eq!(report.stream_id_len, 64);
    }

    #[test]
    fn sign_and_verify_many() {
        assert_eq!(sign_event_only(8).unwrap().len(), 8);
        assert_eq!(verify_event_only(8).unwrap(), 8);
    }
}
