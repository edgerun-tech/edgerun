//! edgerun-stream — single-writer event stream sequencing and validation.
//!
//! This crate owns stream order and prev-hash linkage. It does not own a
//! separate signing implementation. Signing goes through `edgerun-sign` and
//! verification goes through `edgerun-verify`.

#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

use edgerun_protocols::core_protocol::prelude::v1::*;
use edgerun_protocols::core_protocol::protocol::Timestamp;
use edgerun_protocols::core_protocol::protocol::{
    CommandRef, DelegationRef, Digest, EventEnvelope, EventRef, EventType, ObjectRef,
    ProtocolRecord, RevocationRef,
};
use edgerun_protocols::sign::{ProtocolSignError, ProtocolSigner};
use edgerun_protocols::verify::{verify_event_envelope, ProtocolFamily, ProtocolSignerRef};

pub type StreamId = [u8; 64];

/// A stream writer maintains the current stream head and produces signed events.
pub struct StreamWriter<S> {
    stream_id: StreamId,
    head: Option<EventEnvelope>,
    events: Vec<EventEnvelope>,
    signer: S,
}

impl<S: ProtocolSigner> StreamWriter<S> {
    /// Creates a new stream writer, producing the genesis event.
    pub fn new(stream_id: StreamId, signer: S, recorded_at_ms: i64) -> Result<Self, StreamError> {
        Self::new_with_genesis_metadata(stream_id, signer, recorded_at_ms, None)
    }

    /// Creates a new stream writer with a native payload object attached to
    /// the genesis event.
    pub fn new_with_genesis_payload(
        stream_id: StreamId,
        signer: S,
        recorded_at_ms: i64,
        payload_object: ObjectRef,
    ) -> Result<Self, StreamError> {
        Self::new_with_genesis_metadata(stream_id, signer, recorded_at_ms, Some(payload_object))
    }

    fn new_with_genesis_metadata(
        stream_id: StreamId,
        signer: S,
        recorded_at_ms: i64,
        payload_object: Option<ObjectRef>,
    ) -> Result<Self, StreamError> {
        let mut event = genesis_event_with_payload(&stream_id, recorded_at_ms, payload_object);
        sign_event(&mut event, &signer)?;
        Ok(Self {
            stream_id,
            head: Some(event.clone()),
            events: vec![event],
            signer,
        })
    }

    /// Appends a new event and signs it.
    pub fn append(
        &mut self,
        event_type: i32,
        event_version: u32,
        recorded_at_ms: i64,
    ) -> Result<EventEnvelope, StreamError> {
        let draft = EventDraft {
            event_type,
            event_version,
            recorded_at: Some(ms_to_timestamp(recorded_at_ms)),
            ..EventDraft::default()
        };
        let event = build_signed_event(&self.stream_id, self.head.as_ref(), draft, &self.signer)?;
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
    pub fn stream_id(&self) -> &StreamId {
        &self.stream_id
    }

    #[must_use]
    pub fn signer(&self) -> &S {
        &self.signer
    }
}

/// Caller-supplied event content before stream sequencing and signing.
#[derive(Clone, Debug, Default)]
pub struct EventDraft {
    pub event_type: i32,
    pub event_version: u32,
    pub recorded_at: Option<Timestamp>,
    pub effective_at: Option<Timestamp>,
    pub payload_object: Option<ObjectRef>,
    pub related_events: Vec<EventRef>,
    pub related_commands: Vec<CommandRef>,
    pub related_objects: Vec<ObjectRef>,
    pub related_delegations: Vec<DelegationRef>,
    pub related_revocations: Vec<RevocationRef>,
    pub event_metadata: Option<ObjectRef>,
}

/// Builds and signs the next event for `stream_id`.
///
/// The stream layer owns sequence assignment and prev-event hash linkage.
/// The protocol signer owns only signing.
pub fn build_signed_event<S: ProtocolSigner + ?Sized>(
    stream_id: &StreamId,
    previous: Option<&EventEnvelope>,
    draft: EventDraft,
    signer: &S,
) -> Result<EventEnvelope, StreamError> {
    let mut event = build_unsigned_event(stream_id, previous, draft)?;
    sign_event(&mut event, signer)?;
    Ok(event)
}

/// Builds the next unsigned event for `stream_id`.
///
/// This is useful for stores/runtimes that want to inspect or enrich the event
/// before signing through their selected signer implementation.
pub fn build_unsigned_event(
    stream_id: &StreamId,
    previous: Option<&EventEnvelope>,
    draft: EventDraft,
) -> Result<EventEnvelope, StreamError> {
    let (seq, prev_event_hash) = match previous {
        Some(prev) => {
            if prev.stream_id != stream_id.as_slice() {
                return Err(StreamError::StreamMismatch);
            }
            (prev.seq + 1, Some(compute_event_hash(prev)))
        }
        None => (0, None),
    };

    Ok(EventEnvelope {
        envelope_version: 1,
        stream_id: stream_id.to_vec(),
        seq,
        prev_event_hash,
        event_type: draft.event_type,
        event_version: draft.event_version,
        recorded_at: draft.recorded_at,
        effective_at: draft.effective_at,
        payload_object: draft.payload_object,
        related_events: draft.related_events,
        related_commands: draft.related_commands,
        related_objects: draft.related_objects,
        related_delegations: draft.related_delegations,
        related_revocations: draft.related_revocations,
        event_metadata: draft.event_metadata,
        signature: None,
    })
}

pub fn genesis_event(stream_id: &StreamId, recorded_at_ms: i64) -> EventEnvelope {
    genesis_event_with_payload(stream_id, recorded_at_ms, None)
}

pub fn genesis_event_with_payload(
    stream_id: &StreamId,
    recorded_at_ms: i64,
    payload_object: Option<ObjectRef>,
) -> EventEnvelope {
    EventEnvelope {
        envelope_version: 1,
        stream_id: stream_id.to_vec(),
        seq: 0,
        prev_event_hash: None,
        event_type: EventType::NodeGenesis as i32,
        event_version: 1,
        recorded_at: Some(ms_to_timestamp(recorded_at_ms)),
        effective_at: None,
        payload_object,
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
    edgerun_protocols::core_protocol::wire_stream::event_signable_wire_bytes(event)
}

/// Signs an event envelope using the shared protocol signer path.
pub fn sign_event<S: ProtocolSigner + ?Sized>(
    event: &mut EventEnvelope,
    signer: &S,
) -> Result<(), StreamError> {
    let signed = signer.sign_protocol_record(
        &ProtocolRecord::EventEnvelope(event.clone()),
        ProtocolFamily::EventEnvelope,
    )?;
    event.signature = Some(signed.signature);
    Ok(())
}

/// Computes the deterministic event hash for prev-hash linkage.
#[must_use]
pub fn compute_event_hash(event: &EventEnvelope) -> Digest {
    let canonical = event_signable_bytes(event);
    let hash = edgerun_protocols::core_protocol::crypto::record_hash(
        edgerun_protocols::core_protocol::crypto::HASH_DOMAIN_EVENT_ENVELOPE,
        &canonical,
    );
    Digest {
        algorithm: 1,
        value: hash.to_vec(),
    }
}

/// Verifies the event signature against the given writer identity.
pub fn verify_event(event: &EventEnvelope, writer: &StreamId) -> Result<(), StreamError> {
    verify_event_envelope(event, ProtocolSignerRef::P256Raw64(writer))?;
    Ok(())
}

/// Validates an entire stream from genesis.
pub fn validate_stream(events: &[EventEnvelope], writer: &StreamId) -> Result<(), StreamError> {
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
    if genesis.stream_id != writer.as_slice() {
        return Err(StreamError::StreamMismatch);
    }
    verify_event(genesis, writer)?;

    for i in 1..events.len() {
        let prev = &events[i - 1];
        let curr = &events[i];

        if curr.stream_id != writer.as_slice() {
            return Err(StreamError::StreamMismatch);
        }
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

#[derive(Debug, Clone, PartialEq)]
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
    InvalidPublicKey,
    InvalidSignatureValue,
    UnsupportedAlgorithm,
    SignatureVerification,
    SignerFailed,
    StreamMismatch,
    UnsupportedFamily,
}

impl From<ProtocolSignError> for StreamError {
    fn from(value: ProtocolSignError) -> Self {
        match value {
            ProtocolSignError::UnsupportedFamily => Self::UnsupportedFamily,
            ProtocolSignError::InvalidKey => Self::InvalidPublicKey,
            ProtocolSignError::SignerFailed => Self::SignerFailed,
        }
    }
}

impl From<edgerun_protocols::verify::ProtocolVerifyError> for StreamError {
    fn from(value: edgerun_protocols::verify::ProtocolVerifyError) -> Self {
        match value {
            edgerun_protocols::verify::ProtocolVerifyError::MissingSignature => {
                Self::MissingSignature
            }
            edgerun_protocols::verify::ProtocolVerifyError::UnsupportedSignatureAlgorithm => {
                Self::UnsupportedAlgorithm
            }
            edgerun_protocols::verify::ProtocolVerifyError::InvalidPublicKey => {
                Self::InvalidPublicKey
            }
            edgerun_protocols::verify::ProtocolVerifyError::InvalidSignatureLength => {
                Self::InvalidSignature {
                    expected: 64,
                    actual: 0,
                }
            }
            edgerun_protocols::verify::ProtocolVerifyError::InvalidSignature => {
                Self::SignatureVerification
            }
            edgerun_protocols::verify::ProtocolVerifyError::UnsupportedFamily => {
                Self::UnsupportedFamily
            }
            edgerun_protocols::verify::ProtocolVerifyError::MissingWriterIdentity
            | edgerun_protocols::verify::ProtocolVerifyError::MissingIssuerIdentity => {
                Self::InvalidPublicKey
            }
        }
    }
}

impl core::fmt::Display for StreamError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::EmptyStream => write!(f, "stream is empty; no genesis event found"),
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
            Self::InvalidPublicKey => write!(f, "invalid public key"),
            Self::InvalidSignatureValue => write!(f, "invalid signature value"),
            Self::UnsupportedAlgorithm => write!(f, "unsupported signature algorithm"),
            Self::SignatureVerification => write!(f, "signature verification failed"),
            Self::SignerFailed => write!(f, "signer failed"),
            Self::StreamMismatch => write!(f, "event belongs to a different stream"),
            Self::UnsupportedFamily => write!(f, "unsupported protocol family"),
        }
    }
}

impl core::error::Error for StreamError {}

#[cfg(test)]
mod stream_tests;
