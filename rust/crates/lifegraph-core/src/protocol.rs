//! Core protocol types — re-exported from protobuf as the canonical types.
//!
//! Every domain type is a protobuf-generated struct with `prost::Message`.
//! Canonical encoding = `prost::Message::encode()` directly.
//! No intermediate encoding layer, no conversion, no CBOR.

// Timestamp / Duration from prost-types (protobuf standard)
pub use prost_types::{Duration, Timestamp};

// Re-export proto types as the canonical protocol types.
pub use lifegraph_proto::lifegraph::v0::{
    access::{
        CostLimit, FederatedAggregateDescriptor, QueryRequest, QueryResultFragment,
        SnapshotDescriptor,
    },
    common::{
        CommandRef, DelegationRef, Digest, EventRef, HeadRef, IdentityRef, NodeRef, ObjectRef,
        RateLimit, RepresentationRef, RevocationRef, Signature, SnapshotRef, StreamRef,
        TimeWindow,
    },
    identity::IdentityRecord,
    network::{ReachabilityHint, RouteAdvertisement, RelayEnvelope, SessionAccept, SessionHello},
    object::{ChunkEntry, ChunkManifest, LogicalObjectDescriptor, StoredRepresentationHeader},
    stream::{
        ActionLifecyclePayload, CommandEnvelope, CommandResultPayload, CommandSentPayload,
        EventEnvelope, NodeGenesisPayload,
    },
    trust::{
        AggregateTrustPolicy, AssuranceClaim, AssuranceRequirement, CapabilityDescriptor,
        ConstraintSet, DelegationRecord, RevocationRecord, RouteSelectionPolicy,
        RouteTrustAssignment, RouteTrustAssignments, ScopeDescriptor,
    },
};

// Proof types
pub use lifegraph_proto::lifegraph::v0::access::{
    AggregateSummaryProof, EventSetProof, ObjectAssertionProof, ProofBundle,
    ResultFragmentProof, SnapshotSetProof, StreamHeadsProof, TrustPolicyProof,
};

// CheckpointRef
pub use lifegraph_proto::lifegraph::v0::common::CheckpointRef;

// Enum types from proto (needed for event/command type fields)
pub use lifegraph_proto::lifegraph::v0::stream::{
    ActionStatus, CommandDecision, CommandType, EventType,
};

// ---------------------------------------------------------------------------
// Unified ProtocolRecord enum — wraps all canonical types for hashing/signing
// ---------------------------------------------------------------------------

/// A protocol record that can be canonicalized, hashed, or signed.
///
/// Each variant holds a protobuf-generated type that already implements
/// `prost::Message`, so canonical bytes = `prost::Message::encode()`.
#[derive(Clone, Debug)]
pub enum ProtocolRecord {
    IdentityRef(IdentityRef),
    NodeRef(NodeRef),
    StreamRef(StreamRef),
    EventRef(EventRef),
    HeadRef(HeadRef),
    CheckpointRef(CheckpointRef),
    ObjectRef(ObjectRef),
    RepresentationRef(RepresentationRef),
    CommandRef(CommandRef),
    DelegationRef(DelegationRef),
    RevocationRef(RevocationRef),
    SnapshotRef(SnapshotRef),
    Digest(Digest),
    Signature(Signature),
    TimeWindow(TimeWindow),
    CostLimit(CostLimit),
    ScopeDescriptor(ScopeDescriptor),
    DelegationRecord(DelegationRecord),
    CommandEnvelope(CommandEnvelope),
    EventEnvelope(EventEnvelope),
    SnapshotDescriptor(SnapshotDescriptor),
    QueryRequest(QueryRequest),
    QueryResultFragment(QueryResultFragment),
    FederatedAggregateDescriptor(FederatedAggregateDescriptor),
    IdentityRecord(IdentityRecord),
    AssuranceClaim(AssuranceClaim),
    RevocationRecord(RevocationRecord),
    RelayEnvelope(RelayEnvelope),
    ReachabilityHint(ReachabilityHint),
    RouteAdvertisement(RouteAdvertisement),
    SessionHello(SessionHello),
    SessionAccept(SessionAccept),
    NodeGenesisPayload(NodeGenesisPayload),
    CommandSentPayload(CommandSentPayload),
    CommandResultPayload(CommandResultPayload),
    ActionLifecyclePayload(ActionLifecyclePayload),
    LogicalObjectDescriptor(LogicalObjectDescriptor),
    StoredRepresentationHeader(StoredRepresentationHeader),
    ChunkEntry(ChunkEntry),
    ChunkManifest(ChunkManifest),
    AggregateTrustPolicy(AggregateTrustPolicy),
    RouteTrustAssignment(RouteTrustAssignment),
    RouteTrustAssignments(RouteTrustAssignments),
    RouteSelectionPolicy(RouteSelectionPolicy),
    AssuranceRequirement(AssuranceRequirement),
    ConstraintSet(ConstraintSet),
    CapabilityDescriptor(CapabilityDescriptor),
    StreamHeadsProof(StreamHeadsProof),
    SnapshotSetProof(SnapshotSetProof),
    EventSetProof(EventSetProof),
    ObjectAssertionProof(ObjectAssertionProof),
    ResultFragmentProof(ResultFragmentProof),
    AggregateSummaryProof(AggregateSummaryProof),
    TrustPolicyProof(TrustPolicyProof),
    ProofBundle(ProofBundle),
}

// ---------------------------------------------------------------------------
// Canonical encoding — signable form clears the signature field
// ---------------------------------------------------------------------------

/// Returns canonical protobuf wire bytes for a protocol record.
/// When `signable` is `true`, the signature field is cleared before encoding
/// so that the same bytes are produced regardless of which signature is attached.
pub fn canonical_bytes(record: &ProtocolRecord, signable: bool) -> Vec<u8> {
    macro_rules! encode_signable {
        ($msg:expr) => {{
            let mut msg = ($msg).clone();
            if signable {
                msg.signature = None;
            }
            let mut buf = Vec::new();
            prost::Message::encode(&msg, &mut buf).expect("prost encode failed");
            buf
        }};
    }

    match record {
        ProtocolRecord::EventEnvelope(v) => encode_signable!(v),
        ProtocolRecord::CommandEnvelope(v) => encode_signable!(v),
        ProtocolRecord::DelegationRecord(v) => encode_signable!(v),
        ProtocolRecord::RevocationRecord(v) => encode_signable!(v),
        ProtocolRecord::SnapshotDescriptor(v) => encode_signable!(v),
        ProtocolRecord::IdentityRecord(v) => encode_signable!(v),
        ProtocolRecord::AssuranceClaim(v) => encode_signable!(v),
        ProtocolRecord::QueryResultFragment(v) => encode_signable!(v),
        ProtocolRecord::RouteAdvertisement(v) => encode_signable!(v),
        ProtocolRecord::SessionHello(v) => encode_signable!(v),
        ProtocolRecord::SessionAccept(v) => encode_signable!(v),
        ProtocolRecord::RelayEnvelope(v) => encode_signable!(v),
        // Non-signable types — encode directly
        _ => {
            let mut buf = Vec::new();
            match record {
                ProtocolRecord::IdentityRef(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::NodeRef(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::ObjectRef(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::EventRef(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::HeadRef(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::CheckpointRef(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::CommandRef(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::DelegationRef(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::RevocationRef(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::SnapshotRef(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::StreamRef(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::RepresentationRef(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::Digest(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::Signature(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::TimeWindow(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::CostLimit(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::ScopeDescriptor(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::QueryRequest(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::FederatedAggregateDescriptor(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::NodeGenesisPayload(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::CommandSentPayload(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::CommandResultPayload(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::ActionLifecyclePayload(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::LogicalObjectDescriptor(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::StoredRepresentationHeader(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::ChunkEntry(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::ChunkManifest(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::AggregateTrustPolicy(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::RouteTrustAssignment(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::RouteTrustAssignments(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::RouteSelectionPolicy(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::AssuranceRequirement(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::ConstraintSet(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::CapabilityDescriptor(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::StreamHeadsProof(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::SnapshotSetProof(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::EventSetProof(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::ObjectAssertionProof(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::ResultFragmentProof(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::AggregateSummaryProof(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::TrustPolicyProof(v) => prost::Message::encode(v, &mut buf).unwrap(),
                ProtocolRecord::ProofBundle(v) => prost::Message::encode(v, &mut buf).unwrap(),
                _ => unreachable!(),
            }
            buf
        }
    }
}
