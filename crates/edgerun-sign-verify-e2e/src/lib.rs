#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use edgerun_core::crypto::{self, SigningKey};
use edgerun_core::protocol::{EventEnvelope, EventType, ProtocolRecord, Signature};
use edgerun_keygen::MemoryKeyStore;
use edgerun_node_bootstrap::{bootstrap_new_node, BootstrapConfig};
use edgerun_sign_p256::P256ProtocolSigner;
use edgerun_stream::{build_signed_event, validate_stream, EventDraft};
use edgerun_verify::{verify_event_envelope, ProtocolFamily, ProtocolSignerRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum E2eError {
    Keygen,
    Bootstrap,
    Sign,
    Verify,
    Stream,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignVerifyReport {
    pub signature_len: usize,
    pub record_hash_len: usize,
    pub public_key_len: usize,
    pub stream_id_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapReport {
    pub node_id_len: usize,
    pub stored_key_len: usize,
    pub genesis_signature_len: usize,
    pub genesis_stream_id_len: usize,
    pub genesis_seq: u64,
    pub store_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamReport {
    pub event_count: usize,
    pub genesis_seq: u64,
    pub next_seq: u64,
    pub next_has_prev_hash: bool,
    pub next_signature_len: usize,
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

pub fn bootstrap_roundtrip() -> Result<BootstrapReport, E2eError> {
    let mut store = MemoryKeyStore::new();
    let result = bootstrap_new_node(&mut store, BootstrapConfig::default())
        .map_err(|_| E2eError::Bootstrap)?;
    let signature_len = result
        .genesis_event
        .signature
        .as_ref()
        .map(|signature| signature.value.len())
        .ok_or(E2eError::Bootstrap)?;

    verify_event_envelope(
        &result.genesis_event,
        ProtocolSignerRef::P256Raw64(&result.node_id),
    )
    .map_err(|_| E2eError::Verify)?;

    Ok(BootstrapReport {
        node_id_len: result.node_id.len(),
        stored_key_len: result.stored_key.len(),
        genesis_signature_len: signature_len,
        genesis_stream_id_len: result.genesis_event.stream_id.len(),
        genesis_seq: result.genesis_event.seq,
        store_len: store.len(),
    })
}

pub fn stream_roundtrip() -> Result<StreamReport, E2eError> {
    let signing_key = deterministic_signing_key(21);
    let signer = P256ProtocolSigner::new(signing_key);
    let node_id = crypto::verifying_key_to_node_id(&signer.verifying_key());

    let mut genesis = edgerun_stream::genesis_event(&node_id, 0);
    edgerun_stream::sign_event(&mut genesis, &signer).map_err(|_| E2eError::Stream)?;

    let next = build_signed_event(
        &node_id,
        Some(&genesis),
        EventDraft {
            event_type: EventType::ActionStarted as i32,
            event_version: 1,
            ..EventDraft::default()
        },
        &signer,
    )
    .map_err(|_| E2eError::Stream)?;

    let events = alloc::vec![genesis.clone(), next.clone()];
    validate_stream(&events, &node_id).map_err(|_| E2eError::Stream)?;

    Ok(StreamReport {
        event_count: events.len(),
        genesis_seq: genesis.seq,
        next_seq: next.seq,
        next_has_prev_hash: next.prev_event_hash.is_some(),
        next_signature_len: next
            .signature
            .as_ref()
            .map(|signature| signature.value.len())
            .unwrap_or(0),
    })
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
        Err(E2eError::Keygen) => 9,
        Err(E2eError::Bootstrap) => 10,
        Err(E2eError::Sign) => 11,
        Err(E2eError::Verify) => 12,
        Err(E2eError::Stream) => 13,
    }
}

#[no_mangle]
pub extern "C" fn edgerun_bootstrap_e2e_roundtrip() -> u32 {
    match bootstrap_roundtrip() {
        Ok(report) => {
            if report.node_id_len == crypto::ECDSA_P256_PUBLIC_KEY_LEN
                && report.stored_key_len == crypto::ECDSA_P256_PUBLIC_KEY_LEN
                && report.genesis_signature_len == crypto::ECDSA_P256_SIGNATURE_LEN
                && report.genesis_stream_id_len == crypto::ECDSA_P256_PUBLIC_KEY_LEN
                && report.genesis_seq == 0
                && report.store_len == 1
            {
                0
            } else {
                2
            }
        }
        Err(E2eError::Keygen) => 9,
        Err(E2eError::Bootstrap) => 10,
        Err(E2eError::Sign) => 11,
        Err(E2eError::Verify) => 12,
        Err(E2eError::Stream) => 13,
    }
}

#[no_mangle]
pub extern "C" fn edgerun_stream_e2e_roundtrip() -> u32 {
    match stream_roundtrip() {
        Ok(report) => {
            if report.event_count == 2
                && report.genesis_seq == 0
                && report.next_seq == 1
                && report.next_has_prev_hash
                && report.next_signature_len == crypto::ECDSA_P256_SIGNATURE_LEN
            {
                0
            } else {
                2
            }
        }
        Err(E2eError::Keygen) => 9,
        Err(E2eError::Bootstrap) => 10,
        Err(E2eError::Sign) => 11,
        Err(E2eError::Verify) => 12,
        Err(E2eError::Stream) => 13,
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

pub fn bootstrap_only(iterations: usize) -> Result<usize, E2eError> {
    let mut ok = 0usize;
    for _ in 0..iterations {
        bootstrap_roundtrip()?;
        ok += 1;
    }
    Ok(ok)
}

pub fn stream_only(iterations: usize) -> Result<usize, E2eError> {
    let mut ok = 0usize;
    for _ in 0..iterations {
        stream_roundtrip()?;
        ok += 1;
    }
    Ok(ok)
}

pub fn signed_events(iterations: usize) -> Result<(Vec<EventEnvelope>, [u8; 64]), E2eError> {
    let signing_key = deterministic_signing_key(14);
    let signer = P256ProtocolSigner::new(signing_key);
    let public_key = crypto::verifying_key_to_node_id(&signer.verifying_key());
    let mut events = Vec::with_capacity(iterations);

    for seq in 0..iterations as u64 {
        let mut event = sample_event(&public_key, seq);
        let signed = signer
            .sign_record(
                &ProtocolRecord::EventEnvelope(event.clone()),
                ProtocolFamily::EventEnvelope,
            )
            .map_err(|_| E2eError::Sign)?;
        event.signature = Some(signed.signature);
        events.push(event);
    }

    Ok((events, public_key))
}

pub fn verify_prebuilt_events(
    events: &[EventEnvelope],
    public_key: &[u8; 64],
) -> Result<usize, E2eError> {
    let mut ok = 0usize;
    for event in events {
        verify_event_envelope(event, ProtocolSignerRef::P256Raw64(public_key))
            .map_err(|_| E2eError::Verify)?;
        ok += 1;
    }
    Ok(ok)
}

pub fn canonicalize_event_only(iterations: usize) -> usize {
    let signing_key = deterministic_signing_key(15);
    let signer = P256ProtocolSigner::new(signing_key);
    let public_key = crypto::verifying_key_to_node_id(&signer.verifying_key());
    let mut total = 0usize;

    for seq in 0..iterations as u64 {
        let event = sample_event(&public_key, seq);
        let bytes = edgerun_verify::protocol_signable_bytes(
            &ProtocolRecord::EventEnvelope(event),
            ProtocolFamily::EventEnvelope,
        )
        .expect("event canonicalization should be implemented");
        total += bytes.len();
    }

    total
}

pub fn hash_event_only(iterations: usize) -> usize {
    let signing_key = deterministic_signing_key(16);
    let signer = P256ProtocolSigner::new(signing_key);
    let public_key = crypto::verifying_key_to_node_id(&signer.verifying_key());
    let mut total = 0usize;

    for seq in 0..iterations as u64 {
        let event = sample_event(&public_key, seq);
        let hash = edgerun_verify::protocol_record_hash(
            &ProtocolRecord::EventEnvelope(event),
            ProtocolFamily::EventEnvelope,
        )
        .expect("event hashing should be implemented");
        total += hash.len();
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_roundtrip_works() {
        let report = stream_roundtrip().unwrap();
        assert_eq!(report.event_count, 2);
        assert_eq!(report.genesis_seq, 0);
        assert_eq!(report.next_seq, 1);
        assert!(report.next_has_prev_hash);
        assert_eq!(report.next_signature_len, 64);
    }

    #[test]
    fn bootstrap_roundtrip_works() {
        let report = bootstrap_roundtrip().unwrap();
        assert_eq!(report.node_id_len, 64);
        assert_eq!(report.stored_key_len, 64);
        assert_eq!(report.genesis_signature_len, 64);
        assert_eq!(report.genesis_stream_id_len, 64);
        assert_eq!(report.genesis_seq, 0);
        assert_eq!(report.store_len, 1);
    }

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
        assert_eq!(bootstrap_only(2).unwrap(), 2);
        assert_eq!(stream_only(2).unwrap(), 2);
    }
}
