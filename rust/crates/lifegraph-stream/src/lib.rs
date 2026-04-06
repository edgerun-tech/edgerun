//! Lifegraph Stream — the append-only, single-writer event log.
//!
//! ## Protocol invariants (§4 Stream Model)
//!
//! - Every stream has exactly one writer for its entire lifetime
//! - An event in a stream is always authored by that stream's writer
//! - `seq` is the strictly increasing per-stream sequence number — no gaps
//! - Every event after genesis carries `prev_hash`, which **MUST** equal the
//!   canonical hash of the immediately preceding event
//! - Every event **MUST** be signed by the stream's writer identity
//! - Genesis event at `seq = 0` has no `prev_hash`
//! - Stream validation: genesis exists, seq contiguous, prev_hash correct,
//!   signatures verify, writer identity consistent
//!
//! ## Signing
//!
//! Events are signed using ECDSA P-256 with SHA-256 via secure hardware
//! (TPM, YubiKey, Android Keystore). The signature covers the canonical
//! serialized event envelope with the `signature` field omitted.
//! Canonical encoding is deterministic protobuf wire format (protocol §17).

use lifegraph_core::protocol::{
    canonical_bytes, Digest, EventEnvelope, EventType, ProtocolRecord, Signature,
};
use lifegraph_hardware_signing::{HardwareSigningError, MeshSigner, NodeID};
use prost_types::Timestamp;
use sha2::{Digest as ShaDigest, Sha256};

// ---------------------------------------------------------------------------
// Stream writer
// ---------------------------------------------------------------------------

/// A stream writer maintains the current stream head and produces signed events.
pub struct StreamWriter {
    stream_id: Vec<u8>,
    head: Option<EventEnvelope>,
    events: Vec<EventEnvelope>,
    signer: Box<dyn MeshSigner>,
}

impl StreamWriter {
    /// Creates a new stream writer, producing the genesis event.
    pub fn new(
        stream_id: String,
        signer: Box<dyn MeshSigner>,
        recorded_at_ms: i64,
    ) -> Result<Self, StreamError> {
        let stream_id_bytes = stream_id.as_bytes().to_vec();
        let mut event = genesis_event(&stream_id_bytes, recorded_at_ms);
        sign_event(&mut event, signer.as_ref())?;

        Ok(Self {
            stream_id: stream_id_bytes,
            head: Some(event.clone()),
            events: vec![event],
            signer,
        })
    }

    /// Appends a new event to the stream and signs it.
    pub fn append(
        &mut self,
        event_type: i32,
        event_version: u32,
        recorded_at_ms: i64,
    ) -> Result<EventEnvelope, StreamError> {
        let prev = self.head.as_ref().ok_or(StreamError::EmptyStream)?;
        let prev_hash = compute_event_hash(prev);
        let mut event = EventEnvelope {
            envelope_version: 1,
            stream_id: self.stream_id.clone(),
            seq: prev.seq + 1,
            prev_event_hash: Some(prev_hash),
            event_type,
            event_version,
            recorded_at: Some(ms_to_timestamp(recorded_at_ms)),
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
        sign_event(&mut event, self.signer.as_ref())?;
        self.events.push(event.clone());
        self.head = Some(event.clone());
        Ok(event)
    }

    /// Returns all events in the stream.
    #[must_use]
    pub fn events(&self) -> &[EventEnvelope] {
        &self.events
    }

    /// Returns the current head event (latest event in the stream).
    #[must_use]
    pub fn head(&self) -> Option<&EventEnvelope> {
        self.head.as_ref()
    }

    /// Returns the stream ID.
    #[must_use]
    pub fn stream_id(&self) -> &[u8] {
        &self.stream_id
    }

    /// Returns the writer's NodeID.
    #[must_use]
    pub fn writer(&self) -> NodeID {
        self.signer.node_id()
    }
}

// ---------------------------------------------------------------------------
// Genesis event
// ---------------------------------------------------------------------------

fn genesis_event(stream_id: &[u8], recorded_at_ms: i64) -> EventEnvelope {
    EventEnvelope {
        envelope_version: 1,
        stream_id: stream_id.to_vec(),
        seq: 0,
        prev_event_hash: None,
        event_type: EventType::NodeGenesis as i32,
        event_version: 1,
        recorded_at: Some(ms_to_timestamp(recorded_at_ms)),
        effective_at: None,
        payload_object: None,
        related_events: Vec::new(),
        related_commands: Vec::new(),
        related_objects: Vec::new(),
        related_delegations: Vec::new(),
        related_revocations: Vec::new(),
        event_metadata: None,
        signature: None,
    }
}

fn ms_to_timestamp(ms: i64) -> Timestamp {
    Timestamp {
        seconds: ms / 1000,
        nanos: ((ms % 1000) * 1_000_000) as i32,
    }
}

// ---------------------------------------------------------------------------
// Event signing helpers
// ---------------------------------------------------------------------------

/// Signs an event envelope using the canonical representation with signature
/// field omitted. Returns the 64-byte ECDSA P-256 signature.
pub fn sign_event(event: &mut EventEnvelope, signer: &dyn MeshSigner) -> Result<(), StreamError> {
    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, true);
    let digest = Sha256::digest(&canonical);
    let mut digest_bytes = [0u8; 32];
    digest_bytes.copy_from_slice(&digest);
    let sig = signer.sign_digest(&digest_bytes)?;
    event.signature = Some(Signature {
        algorithm: 2, // ECDSA_P256_SHA256
        value: sig.to_vec(),
    });
    Ok(())
}

/// Computes the canonical hash of an event (for prev_hash linkage).
#[must_use]
pub fn compute_event_hash(event: &EventEnvelope) -> Digest {
    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, false);
    let hash = Sha256::digest(&canonical);
    Digest {
        algorithm: 1, // DIGEST_ALGORITHM_SHA256
        value: hash.to_vec(),
    }
}

/// Verifies the event signature against the given writer identity.
pub fn verify_event(event: &EventEnvelope, writer: &NodeID) -> Result<(), StreamError> {
    let sig = event.signature.as_ref().ok_or(StreamError::MissingSignature)?;
    if sig.value.len() != 64 {
        return Err(StreamError::InvalidSignature {
            expected: 64,
            actual: sig.value.len(),
        });
    }

    let record = ProtocolRecord::EventEnvelope(event.clone());
    let canonical = canonical_bytes(&record, true);
    let digest = Sha256::digest(&canonical);
    let mut digest_bytes = [0u8; 32];
    digest_bytes.copy_from_slice(&digest);

    // Build verifying key from NodeID
    let mut sec1 = [0u8; 65];
    sec1[0] = 0x04;
    sec1[1..].copy_from_slice(&writer.0);
    let vk = p256::ecdsa::VerifyingKey::from_sec1_bytes(&sec1)
        .map_err(|e| StreamError::InvalidPublicKey(e.to_string()))?;

    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&sig.value);
    let r = p256::FieldBytes::from_slice(&sig_bytes[..32]);
    let s = p256::FieldBytes::from_slice(&sig_bytes[32..]);
    let ecdsa_sig = p256::ecdsa::Signature::from_scalars(*r, *s)
        .map_err(|e| StreamError::InvalidSignatureFormat(e.to_string()))?;

    use p256::ecdsa::signature::hazmat::PrehashVerifier;
    vk.verify_prehash(&digest_bytes, &ecdsa_sig)
        .map_err(|e| StreamError::SignatureVerification(e.to_string()))
}

// ---------------------------------------------------------------------------
// Stream validation
// ---------------------------------------------------------------------------

/// Validates an entire stream from genesis.
pub fn validate_stream(events: &[EventEnvelope], writer: &NodeID) -> Result<(), StreamError> {
    if events.is_empty() {
        return Err(StreamError::EmptyStream);
    }

    let genesis = &events[0];
    if genesis.seq != 0 {
        return Err(StreamError::MissingGenesis {
            first_seq: genesis.seq,
        });
    }
    if genesis.prev_event_hash.is_some() {
        return Err(StreamError::GenesisHasPrevHash);
    }
    verify_event(genesis, writer)?;

    for i in 1..events.len() {
        let prev = &events[i - 1];
        let curr = &events[i];

        if curr.seq != prev.seq + 1 {
            return Err(StreamError::SequenceGap {
                expected: prev.seq + 1,
                actual: curr.seq,
            });
        }

        let expected_hash = compute_event_hash(prev);
        if curr.prev_event_hash.as_ref() != Some(&expected_hash) {
            return Err(StreamError::InvalidPrevHash {
                seq: curr.seq,
                expected: expected_hash,
                actual: curr.prev_event_hash.clone().unwrap_or(Digest {
                    algorithm: 0,
                    value: Vec::new(),
                }),
            });
        }

        verify_event(curr, writer)?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum StreamError {
    EmptyStream,
    MissingGenesis { first_seq: u64 },
    GenesisHasPrevHash,
    SequenceGap { expected: u64, actual: u64 },
    InvalidPrevHash { seq: u64, expected: Digest, actual: Digest },
    MissingSignature,
    InvalidSignature { expected: usize, actual: usize },
    InvalidSignatureFormat(String),
    InvalidPublicKey(String),
    SignatureVerification(String),
    HardwareSigning(String),
}

impl From<HardwareSigningError> for StreamError {
    fn from(e: HardwareSigningError) -> Self {
        Self::HardwareSigning(e.to_string())
    }
}

impl std::fmt::Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyStream => write!(f, "stream is empty — no genesis event found"),
            Self::MissingGenesis { first_seq } => {
                write!(f, "first event is not genesis: seq={first_seq}")
            }
            Self::GenesisHasPrevHash => write!(f, "genesis event must not have prev_event_hash"),
            Self::SequenceGap { expected, actual } => {
                write!(f, "sequence gap at seq {actual}: expected {expected}")
            }
            Self::InvalidPrevHash { seq, .. } => {
                write!(f, "invalid prev_hash at seq {seq}")
            }
            Self::MissingSignature => write!(f, "event signature is missing"),
            Self::InvalidSignature { expected, actual } => {
                write!(f, "invalid signature length: expected {expected}, got {actual}")
            }
            Self::InvalidSignatureFormat(e) => write!(f, "invalid signature format: {e}"),
            Self::InvalidPublicKey(e) => write!(f, "invalid public key: {e}"),
            Self::SignatureVerification(e) => write!(f, "signature verification failed: {e}"),
            Self::HardwareSigning(e) => write!(f, "hardware signing failed: {e}"),
        }
    }
}

impl std::error::Error for StreamError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use lifegraph_hardware_signing::{HardwareSigningError, MeshSigner};
    use rand::rngs::OsRng;

    struct TestSigner {
        node_id: NodeID,
        key: p256::ecdsa::SigningKey,
    }

    impl TestSigner {
        fn new() -> Self {
            let key = p256::ecdsa::SigningKey::random(&mut OsRng);
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

        fn sign_digest(
            &self,
            digest: &[u8; 32],
        ) -> Result<[u8; 64], HardwareSigningError> {
            use p256::ecdsa::signature::hazmat::RandomizedPrehashSigner;
            let sig: p256::ecdsa::Signature =
                self.key.sign_prehash_with_rng(&mut OsRng, digest).unwrap();
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
        let mut writer =
            StreamWriter::new("stream-1".into(), Box::new(signer), 1000).unwrap();

        for i in 0..3 {
            writer
                .append(100i32, 1, 1000 + i as i64)
                .unwrap();
        }

        let head = writer.head().unwrap();
        assert_eq!(head.seq, 3);
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

        // Tamper with event 1's type
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
            seq: 5, // Skip to seq 5
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
        let mut writer =
            StreamWriter::new("stream-1".into(), Box::new(signer), 1000).unwrap();

        let mut events = Vec::new();
        events.push(writer.head().unwrap().clone());

        for i in 0..5 {
            let event = writer
                .append(100i32, 1, 1000 + i as i64)
                .unwrap();
            events.push(event);
        }

        validate_stream(&events, &writer_id).unwrap();
    }
}
