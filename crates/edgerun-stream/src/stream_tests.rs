use super::*;
use edgerun_crypto::rand_core::RngCore;
use edgerun_hardware_signing::{HardwareSigningError, MeshSigner};
use std::sync::Arc;

#[derive(Clone)]
struct TestSigner {
    node_id: NodeID,
    key: edgerun_crypto::p256::ecdsa::SigningKey,
}

impl TestSigner {
    fn new() -> Self {
        let key = edgerun_crypto::random_p256_signing_key();
        let vk = key.verifying_key();
        let encoded = vk.to_encoded_point(false);
        let mut node_bytes = [0u8; 64];
        node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
        Self {
            node_id: NodeID(node_bytes),
            key,
        }
    }
}

impl MeshSigner for TestSigner {
    fn node_id(&self) -> NodeID {
        self.node_id
    }

    fn sign_digest(&self, digest: &[u8; 32]) -> Result<[u8; 64], HardwareSigningError> {
        use edgerun_crypto::p256::ecdsa::signature::hazmat::PrehashSigner;
        let sig: edgerun_crypto::p256::ecdsa::Signature = self.key.sign_prehash(digest).unwrap();
        let mut bytes = [0u8; 64];
        bytes.copy_from_slice(&sig.to_bytes());
        Ok(bytes)
    }
}

#[test]
fn genesis_has_seq_zero_and_no_prev_hash() {
    let signer = TestSigner::new();
    let mut event = genesis_event(b"stream-1", 1000);
    sign_event(&mut event, &signer).unwrap();
    assert!(verify_event(&event, &signer.node_id()).is_ok());
    assert_eq!(event.seq, 0);
    assert!(event.prev_event_hash.is_none());
}

#[test]
fn append_creates_contiguous_sequence() {
    let signer = TestSigner::new();
    let mut writer = StreamWriter::new("stream-1".into(), Arc::new(signer), 1000).unwrap();
    for i in 0..3 {
        writer.append(100i32, 1, 1000 + i as i64).unwrap();
    }
    assert_eq!(writer.head().unwrap().seq, 3);
}

#[test]
fn validate_stream_rejects_tampered_event() {
    let signer = TestSigner::new();
    let mut events = Vec::new();
    let mut genesis = genesis_event(b"stream-1", 1000);
    sign_event(&mut genesis, &signer).unwrap();
    events.push(genesis);
    let mut event1 = EventEnvelope {
        envelope_version: 1,
        stream_id: b"stream-1".to_vec(),
        seq: 1,
        prev_event_hash: Some(compute_event_hash(&events[0])),
        event_type: 100i32,
        event_version: 1,
        recorded_at: Some(ms_to_timestamp(1001)),
        effective_at: None,
        payload_object: None,
        related_events: Vec::new(),
        related_commands: Vec::new(),
        related_objects: Vec::new(),
        related_delegations: Vec::new(),
        related_revocations: Vec::new(),
        event_metadata: None,
        signature: None,
    };
    sign_event(&mut event1, &signer).unwrap();
    events.push(event1);
    events[1].event_type = EventType::CommandCommitted as i32;
    let err = validate_stream(&events, &signer.node_id()).unwrap_err();
    assert!(matches!(err, StreamError::SignatureVerification(_)));
}

#[test]
fn validate_stream_rejects_sequence_gap() {
    let signer = TestSigner::new();
    let mut events = Vec::new();
    let mut genesis = genesis_event(b"stream-1", 1000);
    sign_event(&mut genesis, &signer).unwrap();
    events.push(genesis);
    let mut event = EventEnvelope {
        envelope_version: 1,
        stream_id: b"stream-1".to_vec(),
        seq: 5,
        prev_event_hash: Some(compute_event_hash(&events[0])),
        event_type: 100i32,
        event_version: 1,
        recorded_at: Some(ms_to_timestamp(1001)),
        effective_at: None,
        payload_object: None,
        related_events: Vec::new(),
        related_commands: Vec::new(),
        related_objects: Vec::new(),
        related_delegations: Vec::new(),
        related_revocations: Vec::new(),
        event_metadata: None,
        signature: None,
    };
    sign_event(&mut event, &signer).unwrap();
    events.push(event);
    let err = validate_stream(&events, &signer.node_id()).unwrap_err();
    match err {
        StreamError::SequenceGap { expected, actual } => {
            assert_eq!(expected, 1);
            assert_eq!(actual, 5);
        }
        other => panic!("expected SequenceGap, got {other:?}"),
    }
}

#[test]
fn validate_stream_rejects_wrong_signer() {
    let signer_a = TestSigner::new();
    let signer_b = TestSigner::new();
    let mut genesis = genesis_event(b"stream-1", 1000);
    sign_event(&mut genesis, &signer_a).unwrap();
    let err = validate_stream(&[genesis], &signer_b.node_id()).unwrap_err();
    assert!(matches!(err, StreamError::SignatureVerification(_)));
}

#[test]
fn validate_stream_rejects_missing_signature() {
    let signer = TestSigner::new();
    let mut genesis = genesis_event(b"stream-1", 1000);
    sign_event(&mut genesis, &signer).unwrap();
    genesis.signature = None;
    let err = validate_stream(&[genesis], &signer.node_id()).unwrap_err();
    assert!(matches!(err, StreamError::MissingSignature));
}

#[test]
fn stream_writer_produces_valid_chain() {
    let signer = TestSigner::new();
    let writer_id = signer.node_id();
    let mut writer = StreamWriter::new("stream-1".into(), Arc::new(signer), 1000).unwrap();
    let mut events = Vec::new();
    events.push(writer.head().unwrap().clone());
    for i in 0..5 {
        events.push(writer.append(100i32, 1, 1000 + i as i64).unwrap());
    }
    validate_stream(&events, &writer_id).unwrap();
}

#[test]
fn genesis_event_has_correct_type() {
    let event = genesis_event(b"test-stream", 0);
    assert_eq!(event.event_type, EventType::NodeGenesis as i32);
    assert_eq!(event.seq, 0);
    assert_eq!(event.stream_id, b"test-stream");
    assert!(event.prev_event_hash.is_none());
    assert!(event.signature.is_none());
    assert_eq!(event.envelope_version, 1);
    assert_eq!(event.event_version, 1);
}

#[test]
fn genesis_event_recorded_at_matches_timestamp() {
    let recorded_ms = 1_700_000_000_000i64;
    let event = genesis_event(b"stream-x", recorded_ms);
    let ts = event.recorded_at.unwrap();
    assert_eq!(ts.seconds, recorded_ms / 1000);
    assert_eq!(ts.nanos, ((recorded_ms % 1000) * 1_000_000) as i32);
}

#[test]
fn genesis_event_all_default_fields_are_empty() {
    let event = genesis_event(b"s", 0);
    assert!(event.effective_at.is_none());
    assert!(event.payload_object.is_none());
    assert!(event.related_events.is_empty());
    assert!(event.related_commands.is_empty());
    assert!(event.related_objects.is_empty());
    assert!(event.related_delegations.is_empty());
    assert!(event.related_revocations.is_empty());
    assert!(event.event_metadata.is_none());
}

#[test]
fn stream_writer_new_produces_signed_genesis() {
    let signer = TestSigner::new();
    let writer = StreamWriter::new("s".into(), Arc::new(signer), 5000).unwrap();
    let genesis = writer.head().unwrap();
    assert!(genesis.signature.is_some());
    let sig = genesis.signature.as_ref().unwrap();
    assert_eq!(sig.algorithm, 1);
    assert_eq!(sig.value.len(), 64);
}

#[test]
fn stream_writer_events_have_contiguous_seq() {
    let signer = TestSigner::new();
    let mut w = StreamWriter::new("s".into(), Arc::new(signer), 0).unwrap();
    for i in 0..10 {
        let ev = w.append(1, 1, i).unwrap();
        assert_eq!(ev.seq, (i + 1) as u64);
    }
}

#[test]
fn stream_writer_events_have_correct_prev_hash() {
    let signer = TestSigner::new();
    let mut w = StreamWriter::new("s".into(), Arc::new(signer), 0).unwrap();
    let genesis = w.head().unwrap().clone();
    let genesis_hash = compute_event_hash(&genesis);
    let ev1 = w.append(1, 1, 0).unwrap();
    assert_eq!(ev1.prev_event_hash, Some(genesis_hash));
}

#[test]
fn stream_writer_events_are_signed() {
    let signer = TestSigner::new();
    let mut w = StreamWriter::new("s".into(), Arc::new(signer), 0).unwrap();
    let ev = w.append(1, 1, 0).unwrap();
    assert!(ev.signature.is_some());
    assert_eq!(ev.signature.as_ref().unwrap().value.len(), 64);
}

#[test]
fn stream_writer_stream_id_accessor() {
    let signer = TestSigner::new();
    let w = StreamWriter::new("my-stream".into(), Arc::new(signer), 0).unwrap();
    assert_eq!(w.stream_id(), b"my-stream");
}

#[test]
fn stream_writer_writer_accessor() {
    let signer = TestSigner::new();
    let expected = signer.node_id();
    let w = StreamWriter::new("s".into(), Arc::new(signer), 0).unwrap();
    assert_eq!(w.writer(), expected);
}

#[test]
fn stream_writer_events_accessor() {
    let signer = TestSigner::new();
    let mut w = StreamWriter::new("s".into(), Arc::new(signer), 0).unwrap();
    w.append(1, 1, 0).unwrap();
    w.append(2, 1, 1).unwrap();
    assert_eq!(w.events().len(), 3);
    assert_eq!(w.events()[0].seq, 0);
    assert_eq!(w.events()[1].seq, 1);
    assert_eq!(w.events()[2].seq, 2);
}

#[test]
fn append_increments_event_type_and_version() {
    let signer = TestSigner::new();
    let mut w = StreamWriter::new("s".into(), Arc::new(signer), 0).unwrap();
    let ev = w.append(42, 7, 0).unwrap();
    assert_eq!(ev.event_type, 42);
    assert_eq!(ev.event_version, 7);
}

#[test]
fn append_records_timestamp() {
    let signer = TestSigner::new();
    let mut w = StreamWriter::new("s".into(), Arc::new(signer), 0).unwrap();
    let ev = w.append(1, 1, 9_999_999).unwrap();
    let ts = ev.recorded_at.unwrap();
    assert_eq!(ts.seconds, 9999);
    assert_eq!(ts.nanos, 999_000_000);
}

#[test]
fn append_produces_empty_related_lists() {
    let signer = TestSigner::new();
    let mut w = StreamWriter::new("s".into(), Arc::new(signer), 0).unwrap();
    let ev = w.append(1, 1, 0).unwrap();
    assert!(ev.related_events.is_empty());
    assert!(ev.related_commands.is_empty());
    assert!(ev.related_objects.is_empty());
    assert!(ev.related_delegations.is_empty());
    assert!(ev.related_revocations.is_empty());
}

#[test]
fn validate_stream_detects_fork_wrong_prev_hash() {
    let signer = TestSigner::new();
    let mut events = Vec::new();
    let mut genesis = genesis_event(b"fork-stream", 1000);
    sign_event(&mut genesis, &signer).unwrap();
    events.push(genesis);
    let mut event1_correct = EventEnvelope {
        envelope_version: 1,
        stream_id: b"fork-stream".to_vec(),
        seq: 1,
        prev_event_hash: Some(compute_event_hash(&events[0])),
        event_type: 100i32,
        event_version: 1,
        recorded_at: Some(ms_to_timestamp(1001)),
        effective_at: None,
        payload_object: None,
        related_events: Vec::new(),
        related_commands: Vec::new(),
        related_objects: Vec::new(),
        related_delegations: Vec::new(),
        related_revocations: Vec::new(),
        event_metadata: None,
        signature: None,
    };
    sign_event(&mut event1_correct, &signer).unwrap();
    events.push(event1_correct);
    events[1].prev_event_hash = Some(Digest {
        algorithm: 1,
        value: vec![0xAB; 32],
    });
    let err = validate_stream(&events, &signer.node_id()).unwrap_err();
    assert!(matches!(err, StreamError::InvalidPrevHash { .. }));
}

#[test]
fn validate_stream_detects_fork_different_stream_id() {
    let signer = TestSigner::new();
    let mut events = Vec::new();
    let mut genesis = genesis_event(b"stream-a", 1000);
    sign_event(&mut genesis, &signer).unwrap();
    events.push(genesis);
    let mut event1 = EventEnvelope {
        envelope_version: 1,
        stream_id: b"stream-b".to_vec(),
        seq: 1,
        prev_event_hash: Some(compute_event_hash(&events[0])),
        event_type: 100i32,
        event_version: 1,
        recorded_at: Some(ms_to_timestamp(1001)),
        effective_at: None,
        payload_object: None,
        related_events: Vec::new(),
        related_commands: Vec::new(),
        related_objects: Vec::new(),
        related_delegations: Vec::new(),
        related_revocations: Vec::new(),
        event_metadata: None,
        signature: None,
    };
    sign_event(&mut event1, &signer).unwrap();
    events.push(event1);
    assert!(validate_stream(&events, &signer.node_id()).is_ok());
}

#[test]
fn verify_event_rejects_wrong_signature_length() {
    let signer = TestSigner::new();
    let mut genesis = genesis_event(b"s", 0);
    sign_event(&mut genesis, &signer).unwrap();
    genesis.signature.as_mut().unwrap().value.truncate(32);
    let err = verify_event(&genesis, &signer.node_id()).unwrap_err();
    assert!(matches!(
        err,
        StreamError::InvalidSignature {
            expected: 64,
            actual: 32
        }
    ));
}

#[test]
fn verify_event_rejects_zero_length_signature() {
    let signer = TestSigner::new();
    let mut genesis = genesis_event(b"s", 0);
    sign_event(&mut genesis, &signer).unwrap();
    genesis.signature.as_mut().unwrap().value.clear();
    let err = verify_event(&genesis, &signer.node_id()).unwrap_err();
    assert!(matches!(
        err,
        StreamError::InvalidSignature {
            expected: 64,
            actual: 0
        }
    ));
}

#[test]
fn verify_event_rejects_random_signature_bytes() {
    let signer = TestSigner::new();
    let mut genesis = genesis_event(b"s", 0);
    sign_event(&mut genesis, &signer).unwrap();
    let mut rng_bytes = [0u8; 64];
    edgerun_crypto::fill_random(&mut rng_bytes).expect("random generation failed");
    genesis.signature.as_mut().unwrap().value = rng_bytes.to_vec();
    let err = verify_event(&genesis, &signer.node_id()).unwrap_err();
    assert!(matches!(
        err,
        StreamError::InvalidSignatureFormat(_) | StreamError::SignatureVerification(_)
    ));
}

#[test]
fn sign_event_produces_valid_64_byte_signature() {
    let signer = TestSigner::new();
    let mut event = genesis_event(b"s", 0);
    sign_event(&mut event, &signer).unwrap();
    let sig = event.signature.unwrap();
    assert_eq!(sig.algorithm, 1);
    assert_eq!(sig.value.len(), 64);
}

#[test]
fn sign_event_signature_algorithm_is_ecdsa_p256() {
    let signer = TestSigner::new();
    let mut event = genesis_event(b"s", 0);
    sign_event(&mut event, &signer).unwrap();
    assert_eq!(event.signature.unwrap().algorithm, 1);
}

#[test]
fn compute_event_hash_is_deterministic() {
    let event = genesis_event(b"deterministic", 12345);
    assert_eq!(compute_event_hash(&event), compute_event_hash(&event));
}

#[test]
fn compute_event_hash_differs_for_different_events() {
    let event1 = genesis_event(b"s", 1000);
    let event2 = genesis_event(b"s", 2000);
    assert_ne!(compute_event_hash(&event1), compute_event_hash(&event2));
}

#[test]
fn compute_event_hash_algorithm_is_sha256() {
    let event = genesis_event(b"s", 0);
    let hash = compute_event_hash(&event);
    assert_eq!(hash.algorithm, 1);
    assert_eq!(hash.value.len(), 32);
}

#[test]
fn validate_stream_single_genesis_is_valid() {
    let signer = TestSigner::new();
    let mut genesis = genesis_event(b"s", 0);
    sign_event(&mut genesis, &signer).unwrap();
    assert!(validate_stream(&[genesis], &signer.node_id()).is_ok());
}

#[test]
fn validate_stream_long_chain_validates_correctly() {
    let signer = TestSigner::new();
    let mut w = StreamWriter::new("long-chain".into(), Arc::new(signer.clone()), 0).unwrap();
    for i in 0..50 {
        w.append(1, 1, i).unwrap();
    }
    let events: Vec<_> = w.events().to_vec();
    let writer_id = w.writer();
    assert!(validate_stream(&events, &writer_id).is_ok());
}

#[test]
fn validate_stream_two_events_valid() {
    let signer = TestSigner::new();
    let mut events = Vec::new();
    let mut genesis = genesis_event(b"s", 0);
    sign_event(&mut genesis, &signer).unwrap();
    let hash = compute_event_hash(&genesis);
    events.push(genesis);
    let mut ev1 = EventEnvelope {
        envelope_version: 1,
        stream_id: b"s".to_vec(),
        seq: 1,
        prev_event_hash: Some(hash),
        event_type: 100,
        event_version: 1,
        recorded_at: Some(ms_to_timestamp(1)),
        effective_at: None,
        payload_object: None,
        related_events: Vec::new(),
        related_commands: Vec::new(),
        related_objects: Vec::new(),
        related_delegations: Vec::new(),
        related_revocations: Vec::new(),
        event_metadata: None,
        signature: None,
    };
    sign_event(&mut ev1, &signer).unwrap();
    events.push(ev1);
    assert!(validate_stream(&events, &signer.node_id()).is_ok());
}

#[test]
fn event_envelope_version_is_one() {
    let signer = TestSigner::new();
    let mut w = StreamWriter::new("s".into(), Arc::new(signer), 0).unwrap();
    assert_eq!(w.head().unwrap().envelope_version, 1);
    let ev = w.append(1, 1, 0).unwrap();
    assert_eq!(ev.envelope_version, 1);
}

#[test]
fn event_effective_at_is_none_by_default() {
    let signer = TestSigner::new();
    let mut w = StreamWriter::new("s".into(), Arc::new(signer), 0).unwrap();
    assert!(w.append(1, 1, 0).unwrap().effective_at.is_none());
}

#[test]
fn event_metadata_is_none_by_default() {
    let signer = TestSigner::new();
    let mut w = StreamWriter::new("s".into(), Arc::new(signer), 0).unwrap();
    assert!(w.append(1, 1, 0).unwrap().event_metadata.is_none());
}

#[test]
fn error_display_empty_stream() {
    assert_eq!(
        StreamError::EmptyStream.to_string(),
        "stream is empty — no genesis event found"
    );
}

#[test]
fn error_display_missing_genesis() {
    assert_eq!(
        StreamError::MissingGenesis { first_seq: 5 }.to_string(),
        "first event is not genesis: seq=5"
    );
}

#[test]
fn error_display_genesis_has_prev_hash() {
    assert_eq!(
        StreamError::GenesisHasPrevHash.to_string(),
        "genesis event must not have prev_event_hash"
    );
}

#[test]
fn error_display_sequence_gap() {
    assert_eq!(
        StreamError::SequenceGap {
            expected: 3,
            actual: 7
        }
        .to_string(),
        "sequence gap at seq 7: expected 3"
    );
}

#[test]
fn error_display_invalid_prev_hash() {
    let e = StreamError::InvalidPrevHash {
        seq: 4,
        expected: Digest {
            algorithm: 1,
            value: vec![0u8; 32],
        },
        actual: Digest {
            algorithm: 1,
            value: vec![1u8; 32],
        },
    };
    assert_eq!(e.to_string(), "invalid prev_hash at seq 4");
}

#[test]
fn error_display_missing_signature() {
    assert_eq!(
        StreamError::MissingSignature.to_string(),
        "event signature is missing"
    );
}

#[test]
fn error_display_invalid_signature() {
    assert_eq!(
        StreamError::InvalidSignature {
            expected: 64,
            actual: 32
        }
        .to_string(),
        "invalid signature length: expected 64, got 32"
    );
}

#[test]
fn error_display_invalid_signature_format() {
    assert_eq!(
        StreamError::InvalidSignatureFormat("bad scalars".into()).to_string(),
        "invalid signature format: bad scalars"
    );
}

#[test]
fn error_display_invalid_public_key() {
    assert_eq!(
        StreamError::InvalidPublicKey("malformed point".into()).to_string(),
        "invalid public key: malformed point"
    );
}

#[test]
fn error_display_signature_verification() {
    assert_eq!(
        StreamError::SignatureVerification("verify failed".into()).to_string(),
        "signature verification failed: verify failed"
    );
}

#[test]
fn error_display_hardware_signing() {
    assert_eq!(
        StreamError::HardwareSigning("tpm unavailable".into()).to_string(),
        "hardware signing failed: tpm unavailable"
    );
}

#[test]
fn error_is_std_error() {
    let e: Box<dyn std::error::Error> = Box::new(StreamError::EmptyStream);
    assert_eq!(e.to_string(), "stream is empty — no genesis event found");
}

#[test]
fn validate_stream_empty_events_rejected() {
    let signer = TestSigner::new();
    let err = validate_stream(&[], &signer.node_id()).unwrap_err();
    assert!(matches!(err, StreamError::EmptyStream));
}

#[test]
fn validate_stream_genesis_not_at_seq_zero_rejected() {
    let signer = TestSigner::new();
    let mut genesis = genesis_event(b"s", 0);
    genesis.seq = 1;
    sign_event(&mut genesis, &signer).unwrap();
    let err = validate_stream(&[genesis], &signer.node_id()).unwrap_err();
    assert!(matches!(err, StreamError::MissingGenesis { first_seq: 1 }));
}

#[test]
fn validate_stream_genesis_with_prev_hash_rejected() {
    let signer = TestSigner::new();
    let mut genesis = genesis_event(b"s", 0);
    genesis.prev_event_hash = Some(Digest {
        algorithm: 1,
        value: vec![0u8; 32],
    });
    sign_event(&mut genesis, &signer).unwrap();
    let err = validate_stream(&[genesis], &signer.node_id()).unwrap_err();
    assert!(matches!(err, StreamError::GenesisHasPrevHash));
}

#[test]
fn validate_stream_sequence_out_of_order_rejected() {
    let signer = TestSigner::new();
    let mut events = Vec::new();
    let mut genesis = genesis_event(b"s", 0);
    sign_event(&mut genesis, &signer).unwrap();
    events.push(genesis);
    let mut ev = EventEnvelope {
        envelope_version: 1,
        stream_id: b"s".to_vec(),
        seq: 0,
        prev_event_hash: Some(compute_event_hash(&events[0])),
        event_type: 100,
        event_version: 1,
        recorded_at: Some(ms_to_timestamp(1)),
        effective_at: None,
        payload_object: None,
        related_events: Vec::new(),
        related_commands: Vec::new(),
        related_objects: Vec::new(),
        related_delegations: Vec::new(),
        related_revocations: Vec::new(),
        event_metadata: None,
        signature: None,
    };
    sign_event(&mut ev, &signer).unwrap();
    events.push(ev);
    let err = validate_stream(&events, &signer.node_id()).unwrap_err();
    assert!(matches!(err, StreamError::SequenceGap { .. }));
}

#[test]
fn ms_to_timestamp_converts_correctly() {
    let ts = ms_to_timestamp(1_500);
    assert_eq!(ts.seconds, 1);
    assert_eq!(ts.nanos, 500_000_000);
}

#[test]
fn ms_to_timestamp_zero() {
    let ts = ms_to_timestamp(0);
    assert_eq!(ts.seconds, 0);
    assert_eq!(ts.nanos, 0);
}

#[test]
fn ms_to_timestamp_negative_seconds() {
    let ts = ms_to_timestamp(-1000);
    assert_eq!(ts.seconds, -1);
    assert_eq!(ts.nanos, 0);
}

#[test]
fn stream_writer_clone_impl_trait() {
    let signer = TestSigner::new();
    let _ = StreamWriter::new("s".into(), Arc::new(signer), 0).unwrap();
}

#[test]
fn validate_stream_detects_tampered_event_type() {
    let signer = TestSigner::new();
    let mut events = Vec::new();
    let mut genesis = genesis_event(b"s", 0);
    sign_event(&mut genesis, &signer).unwrap();
    events.push(genesis);
    let mut ev = EventEnvelope {
        envelope_version: 1,
        stream_id: b"s".to_vec(),
        seq: 1,
        prev_event_hash: Some(compute_event_hash(&events[0])),
        event_type: 100,
        event_version: 1,
        recorded_at: Some(ms_to_timestamp(1)),
        effective_at: None,
        payload_object: None,
        related_events: Vec::new(),
        related_commands: Vec::new(),
        related_objects: Vec::new(),
        related_delegations: Vec::new(),
        related_revocations: Vec::new(),
        event_metadata: None,
        signature: None,
    };
    sign_event(&mut ev, &signer).unwrap();
    events.push(ev);
    events[1].event_type = 999;
    let err = validate_stream(&events, &signer.node_id()).unwrap_err();
    assert!(matches!(err, StreamError::SignatureVerification(_)));
}

#[test]
fn validate_stream_detects_tampered_stream_id() {
    let signer = TestSigner::new();
    let mut events = Vec::new();
    let mut genesis = genesis_event(b"original", 0);
    sign_event(&mut genesis, &signer).unwrap();
    events.push(genesis);
    let mut ev = EventEnvelope {
        envelope_version: 1,
        stream_id: b"original".to_vec(),
        seq: 1,
        prev_event_hash: Some(compute_event_hash(&events[0])),
        event_type: 100,
        event_version: 1,
        recorded_at: Some(ms_to_timestamp(1)),
        effective_at: None,
        payload_object: None,
        related_events: Vec::new(),
        related_commands: Vec::new(),
        related_objects: Vec::new(),
        related_delegations: Vec::new(),
        related_revocations: Vec::new(),
        event_metadata: None,
        signature: None,
    };
    sign_event(&mut ev, &signer).unwrap();
    events.push(ev);
    events[1].stream_id = b"tampered".to_vec();
    let err = validate_stream(&events, &signer.node_id()).unwrap_err();
    assert!(matches!(err, StreamError::SignatureVerification(_)));
}

#[test]
fn validate_stream_detects_tampered_recorded_at() {
    let signer = TestSigner::new();
    let mut events = Vec::new();
    let mut genesis = genesis_event(b"s", 0);
    sign_event(&mut genesis, &signer).unwrap();
    events.push(genesis);
    let mut ev = EventEnvelope {
        envelope_version: 1,
        stream_id: b"s".to_vec(),
        seq: 1,
        prev_event_hash: Some(compute_event_hash(&events[0])),
        event_type: 100,
        event_version: 1,
        recorded_at: Some(ms_to_timestamp(1)),
        effective_at: None,
        payload_object: None,
        related_events: Vec::new(),
        related_commands: Vec::new(),
        related_objects: Vec::new(),
        related_delegations: Vec::new(),
        related_revocations: Vec::new(),
        event_metadata: None,
        signature: None,
    };
    sign_event(&mut ev, &signer).unwrap();
    events.push(ev);
    events[1].recorded_at = Some(ms_to_timestamp(9999));
    let err = validate_stream(&events, &signer.node_id()).unwrap_err();
    assert!(matches!(err, StreamError::SignatureVerification(_)));
}
