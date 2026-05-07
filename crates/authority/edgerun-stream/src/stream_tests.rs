use super::*;

use edgerun_protocols::core_protocol::crypto::{
    ECDSA_P256_SIGNATURE_LEN, SIGNATURE_ALGORITHM_ECDSA_P256,
};
use edgerun_protocols::keygen::{node_signing_key_from_bytes, NodeSigningKey};
use edgerun_protocols::sign_p256::P256ProtocolSigner;

fn signer(seed: u8) -> P256ProtocolSigner {
    P256ProtocolSigner::new(signing_key(seed))
}

fn signing_key(seed: u8) -> NodeSigningKey {
    node_signing_key_from_bytes([seed; 32]).unwrap()
}

fn stream_id(seed: u8) -> StreamId {
    signer(seed).raw_public_key()
}

fn signed_genesis(seed: u8) -> EventEnvelope {
    let signer = signer(seed);
    let mut event = genesis_event(&signer.raw_public_key(), 1_000);
    sign_event(&mut event, &signer).unwrap();
    event
}

#[test]
fn genesis_has_seq_zero_no_prev_hash_and_node_genesis_type() {
    let id = stream_id(7);
    let event = genesis_event(&id, 1_700_000_000_123);

    assert_eq!(event.stream_id, id.to_vec());
    assert_eq!(event.seq, 0);
    assert!(event.prev_event_hash.is_none());
    assert_eq!(event.event_type, EventType::NodeGenesis as i32);
    assert_eq!(event.event_version, 1);
    assert_eq!(event.envelope_version, 1);
    assert!(event.signature.is_none());

    let ts = event.recorded_at.unwrap();
    assert_eq!(ts.seconds, 1_700_000_000);
    assert_eq!(ts.nanos, 123_000_000);
}

#[test]
fn sign_event_uses_protocol_signer_and_verifier_accepts_it() {
    let signer = signer(8);
    let id = signer.raw_public_key();
    let mut event = genesis_event(&id, 1_000);

    sign_event(&mut event, &signer).unwrap();
    let signature = event.signature.as_ref().unwrap();

    assert_eq!(signature.algorithm, SIGNATURE_ALGORITHM_ECDSA_P256 as i32);
    assert_eq!(signature.value.len(), ECDSA_P256_SIGNATURE_LEN);
    verify_event(&event, &id).unwrap();
}

#[test]
fn stream_writer_appends_contiguous_signed_hash_linked_events() {
    let signer = signer(9);
    let id = signer.raw_public_key();
    let mut writer = StreamWriter::new(id, signer, 1_000).unwrap();
    let genesis = writer.head().unwrap().clone();
    let genesis_hash = compute_event_hash(&genesis);

    let first = writer.append(100, 1, 1_001).unwrap();
    let second = writer.append(101, 2, 1_002).unwrap();

    assert_eq!(first.seq, 1);
    assert_eq!(second.seq, 2);
    assert_eq!(first.prev_event_hash, Some(genesis_hash));
    assert_eq!(second.prev_event_hash, Some(compute_event_hash(&first)));
    assert!(writer
        .events()
        .iter()
        .all(|event| event.signature.is_some()));
    validate_stream(writer.events(), writer.stream_id()).unwrap();
}

#[test]
fn build_signed_event_assigns_seq_prev_hash_and_preserves_draft_fields() {
    let signer = signer(10);
    let id = signer.raw_public_key();
    let genesis = signed_genesis(10);

    let event = build_signed_event(
        &id,
        Some(&genesis),
        EventDraft {
            event_type: EventType::ActionCompleted as i32,
            event_version: 7,
            recorded_at: Some(ms_to_timestamp(2_345)),
            ..Default::default()
        },
        &signer,
    )
    .unwrap();

    assert_eq!(event.seq, 1);
    assert_eq!(event.prev_event_hash, Some(compute_event_hash(&genesis)));
    assert_eq!(event.event_type, EventType::ActionCompleted as i32);
    assert_eq!(event.event_version, 7);
    assert!(event.signature.is_some());
    validate_stream(&[genesis, event], &id).unwrap();
}

#[test]
fn build_unsigned_event_rejects_previous_event_from_different_stream() {
    let id = stream_id(11);
    let other = signed_genesis(12);

    let err = build_unsigned_event(
        &id,
        Some(&other),
        EventDraft {
            event_type: 100,
            event_version: 1,
            ..Default::default()
        },
    )
    .unwrap_err();

    assert_eq!(err, StreamError::StreamMismatch);
}

#[test]
fn validate_stream_rejects_empty_stream() {
    let id = stream_id(13);
    let err = validate_stream(&[], &id).unwrap_err();
    assert_eq!(err, StreamError::EmptyStream);
}

#[test]
fn validate_stream_rejects_missing_genesis_sequence() {
    let signer = signer(14);
    let id = signer.raw_public_key();
    let mut event = genesis_event(&id, 1_000);
    event.seq = 1;
    sign_event(&mut event, &signer).unwrap();

    let err = validate_stream(&[event], &id).unwrap_err();
    assert_eq!(err, StreamError::MissingGenesis { first_seq: 1 });
}

#[test]
fn validate_stream_rejects_genesis_prev_hash() {
    let signer = signer(15);
    let id = signer.raw_public_key();
    let mut event = genesis_event(&id, 1_000);
    event.prev_event_hash = Some(Digest {
        algorithm: 1,
        value: vec![0; 32],
    });
    sign_event(&mut event, &signer).unwrap();

    let err = validate_stream(&[event], &id).unwrap_err();
    assert_eq!(err, StreamError::GenesisHasPrevHash);
}

#[test]
fn validate_stream_rejects_sequence_gap() {
    let signer = signer(16);
    let id = signer.raw_public_key();
    let genesis = signed_genesis(16);
    let mut event = build_signed_event(
        &id,
        Some(&genesis),
        EventDraft {
            event_type: 100,
            event_version: 1,
            ..Default::default()
        },
        &signer,
    )
    .unwrap();
    event.seq = 5;

    let err = validate_stream(&[genesis, event], &id).unwrap_err();
    assert_eq!(
        err,
        StreamError::SequenceGap {
            expected: 1,
            actual: 5
        }
    );
}

#[test]
fn validate_stream_rejects_bad_prev_hash_before_signature_check() {
    let signer = signer(17);
    let id = signer.raw_public_key();
    let genesis = signed_genesis(17);
    let mut event = build_signed_event(
        &id,
        Some(&genesis),
        EventDraft {
            event_type: 100,
            event_version: 1,
            ..Default::default()
        },
        &signer,
    )
    .unwrap();
    event.prev_event_hash = Some(Digest {
        algorithm: 1,
        value: vec![0xAB; 32],
    });

    let err = validate_stream(&[genesis, event], &id).unwrap_err();
    assert!(matches!(err, StreamError::InvalidPrevHash { seq: 1, .. }));
}

#[test]
fn validate_stream_rejects_tampered_signed_event() {
    let signer = signer(18);
    let id = signer.raw_public_key();
    let mut writer = StreamWriter::new(id, signer, 1_000).unwrap();
    writer.append(100, 1, 1_001).unwrap();
    let mut events = writer.events().to_vec();
    events[1].event_type = 101;

    let err = validate_stream(&events, writer.stream_id()).unwrap_err();
    assert_eq!(err, StreamError::SignatureVerification);
}

#[test]
fn validate_stream_rejects_wrong_writer_identity() {
    let genesis = signed_genesis(19);
    let wrong_id = stream_id(20);

    let err = validate_stream(&[genesis], &wrong_id).unwrap_err();
    assert_eq!(err, StreamError::StreamMismatch);
}

#[test]
fn verify_event_rejects_missing_signature() {
    let id = stream_id(21);
    let event = genesis_event(&id, 1_000);

    let err = verify_event(&event, &id).unwrap_err();
    assert_eq!(err, StreamError::MissingSignature);
}

#[test]
fn verify_event_rejects_bad_signature_length() {
    let signer = signer(22);
    let id = signer.raw_public_key();
    let mut event = genesis_event(&id, 1_000);
    sign_event(&mut event, &signer).unwrap();
    event.signature.as_mut().unwrap().value.truncate(32);

    let err = verify_event(&event, &id).unwrap_err();
    assert_eq!(
        err,
        StreamError::InvalidSignature {
            expected: 64,
            actual: 0
        }
    );
}

#[test]
fn compute_event_hash_is_deterministic_and_uses_sha256() {
    let event = genesis_event(&stream_id(23), 1_000);
    let hash = compute_event_hash(&event);

    assert_eq!(hash, compute_event_hash(&event));
    assert_eq!(hash.algorithm, 1);
    assert_eq!(hash.value.len(), 32);
}

#[test]
fn compute_event_hash_differs_for_different_signable_fields() {
    let id = stream_id(24);
    let event = genesis_event(&id, 1_000);
    let mut changed = genesis_event(&id, 1_000);
    changed.event_version = 2;

    assert_ne!(compute_event_hash(&event), compute_event_hash(&changed));
}

#[test]
fn error_display_matches_current_error_shape() {
    assert_eq!(
        StreamError::EmptyStream.to_string(),
        "stream is empty; no genesis event found"
    );
    assert_eq!(
        StreamError::SignatureVerification.to_string(),
        "signature verification failed"
    );
    assert_eq!(
        StreamError::InvalidSignature {
            expected: 64,
            actual: 0
        }
        .to_string(),
        "invalid signature length: expected 64, got 0"
    );
}
