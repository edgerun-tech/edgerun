//! edgerun Stream — the append-only, single-writer event log.
//!
//! Events are signed over deterministic edgerun-wire bytes with the signature
//! field omitted. Generated protobuf structs remain boundary types only.

#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

use alloc::sync::Arc;
use edgerun_core::prelude::v1::*;
use edgerun_core::protocol::{Digest, EventEnvelope, EventType, Signature};
use edgerun_hardware_signing::{HardwareSigningError, MeshSigner, NodeID};
use prost_types::Timestamp;

/// A stream writer maintains the current stream head and produces signed events.
pub struct StreamWriter {
    stream_id: Vec<u8>,
    head: Option<EventEnvelope>,
    events: Vec<EventEnvelope>,
    signer: Arc<dyn MeshSigner>,
}

impl StreamWriter {
    /// Creates a new stream writer, producing the genesis event.
    pub fn new(
        stream_id: String,
        signer: Arc<dyn MeshSigner>,
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

    #[must_use]
    pub fn events(&self) -> &[EventEnvelope] {
        &self.events
    }

    #[must_use]
    pub fn head(&self) -> Option<&EventEnvelope> {
        self.head.as_ref()
    }

    #[must_use]
    pub fn stream_id(&self) -> &[u8] {
        &self.stream_id
    }

    #[must_use]
    pub fn writer(&self) -> NodeID {
        self.signer.node_id()
    }
}

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

/// Returns deterministic signable edgerun-wire bytes for an event.
#[must_use]
pub fn event_signable_bytes(event: &EventEnvelope) -> Vec<u8> {
    edgerun_core::wire_stream::event_signable_wire_bytes(event)
}

/// Signs an event envelope using edgerun-wire bytes with the signature omitted.
pub fn sign_event(event: &mut EventEnvelope, signer: &dyn MeshSigner) -> Result<(), StreamError> {
    use edgerun_core::crypto::SIG_DOMAIN_EVENT_ENVELOPE;
    let canonical = event_signable_bytes(event);
    let sig = signer.sign_record(SIG_DOMAIN_EVENT_ENVELOPE, &canonical)?;
    event.signature = Some(Signature {
        algorithm: 1,
        value: sig.to_vec(),
    });
    Ok(())
}

/// Computes the deterministic event hash for prev_hash linkage.
#[must_use]
pub fn compute_event_hash(event: &EventEnvelope) -> Digest {
    let canonical = event_signable_bytes(event);
    let hash = edgerun_core::crypto::record_hash(
        edgerun_core::crypto::HASH_DOMAIN_EVENT_ENVELOPE,
        &canonical,
    );
    Digest {
        algorithm: 1,
        value: hash.to_vec(),
    }
}

/// Verifies the event signature against the given writer identity.
pub fn verify_event(event: &EventEnvelope, writer: &NodeID) -> Result<(), StreamError> {
    use edgerun_core::crypto::{
        verify_canonical_record, verify_canonical_record_hw, SIG_DOMAIN_EVENT_ENVELOPE,
    };
    let sig = event
        .signature
        .as_ref()
        .ok_or(StreamError::MissingSignature)?;
    if sig.value.len() != 64 {
        return Err(StreamError::InvalidSignature {
            expected: 64,
            actual: sig.value.len(),
        });
    }

    let canonical = event_signable_bytes(event);

    let mut sec1 = [0u8; 65];
    sec1[0] = 0x04;
    sec1[1..].copy_from_slice(&writer.0);
    let vk = edgerun_crypto::p256::ecdsa::VerifyingKey::from_sec1_bytes(&sec1)
        .map_err(|e| StreamError::InvalidPublicKey(e.to_string()))?;

    if !verify_canonical_record(&vk, SIG_DOMAIN_EVENT_ENVELOPE, &canonical, &sig.value)
        && !verify_canonical_record_hw(&vk, SIG_DOMAIN_EVENT_ENVELOPE, &canonical, &sig.value)
    {
        return Err(StreamError::SignatureVerification(
            "invalid signature".into(),
        ));
    }
    Ok(())
}

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

#[derive(Debug)]
pub enum StreamError {
    EmptyStream,
    MissingGenesis {
        first_seq: u64,
    },
    GenesisHasPrevHash,
    SequenceGap {
        expected: u64,
        actual: u64,
    },
    InvalidPrevHash {
        seq: u64,
        expected: Digest,
        actual: Digest,
    },
    MissingSignature,
    InvalidSignature {
        expected: usize,
        actual: usize,
    },
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

impl core::fmt::Display for StreamError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyStream => write!(f, "stream is empty — no genesis event found"),
            Self::MissingGenesis { first_seq } => {
                write!(f, "first event is not genesis: seq={first_seq}")
            }
            Self::GenesisHasPrevHash => write!(f, "genesis event must not have prev_event_hash"),
            Self::SequenceGap { expected, actual } => {
                write!(f, "sequence gap at seq {actual}: expected {expected}")
            }
            Self::InvalidPrevHash { seq, .. } => write!(f, "invalid prev_hash at seq {seq}"),
            Self::MissingSignature => write!(f, "event signature is missing"),
            Self::InvalidSignature { expected, actual } => write!(
                f,
                "invalid signature length: expected {expected}, got {actual}"
            ),
            Self::InvalidSignatureFormat(e) => write!(f, "invalid signature format: {e}"),
            Self::InvalidPublicKey(e) => write!(f, "invalid public key: {e}"),
            Self::SignatureVerification(e) => write!(f, "signature verification failed: {e}"),
            Self::HardwareSigning(e) => write!(f, "hardware signing failed: {e}"),
        }
    }
}

impl core::error::Error for StreamError {}

#[cfg(test)]
mod stream_tests;
