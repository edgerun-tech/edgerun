//! Core protocol compatibility surface backed by edgerun-wire canonicalization.
//!
//! This module intentionally replaces the old protobuf-canonical `protocol.rs`.
//! Generated protobuf structs may still appear at process boundaries while the
//! remaining crates are migrated, but canonical bytes and hashes must route
//! through edgerun-wire helpers, not `prost::Message::encode`.

use crate::prelude::v1::*;

pub use prost_types::{Duration, Timestamp};

pub use edgerun_proto::edgerun::v0::{
    access::{
        AggregateSummaryProof, CostLimit, EventSetProof, FederatedAggregateDescriptor,
        ObjectAssertionProof, ProofBundle, QueryRequest, QueryResultFragment, ResultFragmentProof,
        SnapshotDescriptor, SnapshotSetProof, StreamHeadsProof, TrustPolicyProof,
    },
    app::{CapabilityCheck, CapabilityResult, ExecutionContext},
    common::{
        CheckpointRef, CipherSuite, CommandRef, DelegationRef, Digest, EncryptedEnvelope, EventRef,
        HeadRef, IdentityRef, NodeRef, ObjectKind, ObjectRef, RateLimit, RepresentationRef,
        RevocationRef, Signature, SnapshotRef, StreamRef, TimeWindow,
    },
    identity::IdentityRecord,
    network::{ReachabilityHint, RelayEnvelope, RouteAdvertisement, SessionAccept, SessionHello},
    object::{ChunkEntry, ChunkManifest, LogicalObjectDescriptor, StoredRepresentationHeader},
    stream::{
        ActionLifecyclePayload, ActionStatus, AppExecutionPayload, AppPackage,
        CollectionCreatedPayload, CollectionDeletedPayload, CommandDecision, CommandEnvelope,
        CommandResultPayload, CommandSentPayload, CommandType, EventEnvelope, EventType,
        InstallAppPayload, NodeGenesisPayload, SecretDeletePayload, SecretPutPayload,
        UninstallAppPayload,
    },
    trust::{
        AggregateTrustPolicy, AssuranceClaim, AssuranceRequirement, CapabilityDescriptor,
        ConstraintSet, DelegationRecord, RevocationRecord, RouteSelectionPolicy,
        RouteTrustAssignment, RouteTrustAssignments, ScopeDescriptor,
    },
    ui::{UiActionEvent, UiNode, UiRenderRequest},
};

#[derive(Clone, Debug)]
pub enum ProtocolRecord {
    CommandEnvelope(CommandEnvelope),
    EventEnvelope(EventEnvelope),
    CommandResultPayload(CommandResultPayload),
    DelegationRecord(DelegationRecord),
    RevocationRecord(RevocationRecord),
    SnapshotDescriptor(SnapshotDescriptor),
    IdentityRecord(IdentityRecord),
    AssuranceClaim(AssuranceClaim),
    QueryResultFragment(QueryResultFragment),
    QueryRequest(QueryRequest),
    FederatedAggregateDescriptor(FederatedAggregateDescriptor),
    RouteAdvertisement(RouteAdvertisement),
    SessionHello(SessionHello),
    SessionAccept(SessionAccept),
    RelayEnvelope(RelayEnvelope),
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
    NodeGenesisPayload(NodeGenesisPayload),
    CommandSentPayload(CommandSentPayload),
    ActionLifecyclePayload(ActionLifecyclePayload),
    SecretPutPayload(SecretPutPayload),
    SecretDeletePayload(SecretDeletePayload),
    CollectionCreatedPayload(CollectionCreatedPayload),
    CollectionDeletedPayload(CollectionDeletedPayload),
    InstallAppPayload(InstallAppPayload),
    UninstallAppPayload(UninstallAppPayload),
    AppExecutionPayload(AppExecutionPayload),
    AppPackage(AppPackage),
    ExecutionContext(ExecutionContext),
    CapabilityCheck(CapabilityCheck),
    CapabilityResultMsg(CapabilityResult),
    UiNode(UiNode),
    UiActionEvent(UiActionEvent),
    UiRenderRequest(UiRenderRequest),
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
    ReachabilityHint(ReachabilityHint),
}

pub fn canonical_bytes(record: &ProtocolRecord, signable: bool) -> Vec<u8> {
    match record {
        ProtocolRecord::EventEnvelope(event) => {
            if signable {
                crate::wire_stream::event_signable_wire_bytes(event)
            } else {
                crate::wire_stream::event_full_wire_bytes(event)
            }
        }
        ProtocolRecord::CommandEnvelope(command) => {
            if signable {
                crate::wire_command::command_signable_bytes(command)
            } else {
                crate::wire_command::command_full_bytes(command)
            }
        }
        ProtocolRecord::CommandResultPayload(result) => {
            crate::wire_command::command_result_bytes(result)
        }
        other => fallback_structural_bytes(other, signable),
    }
}

fn fallback_structural_bytes(record: &ProtocolRecord, signable: bool) -> Vec<u8> {
    let name = match record {
        ProtocolRecord::DelegationRecord(_) => "DelegationRecord",
        ProtocolRecord::RevocationRecord(_) => "RevocationRecord",
        ProtocolRecord::SnapshotDescriptor(_) => "SnapshotDescriptor",
        ProtocolRecord::IdentityRecord(_) => "IdentityRecord",
        ProtocolRecord::AssuranceClaim(_) => "AssuranceClaim",
        ProtocolRecord::QueryResultFragment(_) => "QueryResultFragment",
        ProtocolRecord::QueryRequest(_) => "QueryRequest",
        ProtocolRecord::FederatedAggregateDescriptor(_) => "FederatedAggregateDescriptor",
        ProtocolRecord::RouteAdvertisement(_) => "RouteAdvertisement",
        ProtocolRecord::SessionHello(_) => "SessionHello",
        ProtocolRecord::SessionAccept(_) => "SessionAccept",
        ProtocolRecord::RelayEnvelope(_) => "RelayEnvelope",
        ProtocolRecord::IdentityRef(_) => "IdentityRef",
        ProtocolRecord::NodeRef(_) => "NodeRef",
        ProtocolRecord::StreamRef(_) => "StreamRef",
        ProtocolRecord::EventRef(_) => "EventRef",
        ProtocolRecord::HeadRef(_) => "HeadRef",
        ProtocolRecord::CheckpointRef(_) => "CheckpointRef",
        ProtocolRecord::ObjectRef(_) => "ObjectRef",
        ProtocolRecord::RepresentationRef(_) => "RepresentationRef",
        ProtocolRecord::CommandRef(_) => "CommandRef",
        ProtocolRecord::DelegationRef(_) => "DelegationRef",
        ProtocolRecord::RevocationRef(_) => "RevocationRef",
        ProtocolRecord::SnapshotRef(_) => "SnapshotRef",
        ProtocolRecord::Digest(_) => "Digest",
        ProtocolRecord::Signature(_) => "Signature",
        ProtocolRecord::TimeWindow(_) => "TimeWindow",
        ProtocolRecord::CostLimit(_) => "CostLimit",
        ProtocolRecord::ScopeDescriptor(_) => "ScopeDescriptor",
        ProtocolRecord::NodeGenesisPayload(_) => "NodeGenesisPayload",
        ProtocolRecord::CommandSentPayload(_) => "CommandSentPayload",
        ProtocolRecord::ActionLifecyclePayload(_) => "ActionLifecyclePayload",
        ProtocolRecord::SecretPutPayload(_) => "SecretPutPayload",
        ProtocolRecord::SecretDeletePayload(_) => "SecretDeletePayload",
        ProtocolRecord::CollectionCreatedPayload(_) => "CollectionCreatedPayload",
        ProtocolRecord::CollectionDeletedPayload(_) => "CollectionDeletedPayload",
        ProtocolRecord::InstallAppPayload(_) => "InstallAppPayload",
        ProtocolRecord::UninstallAppPayload(_) => "UninstallAppPayload",
        ProtocolRecord::AppExecutionPayload(_) => "AppExecutionPayload",
        ProtocolRecord::AppPackage(_) => "AppPackage",
        ProtocolRecord::ExecutionContext(_) => "ExecutionContext",
        ProtocolRecord::CapabilityCheck(_) => "CapabilityCheck",
        ProtocolRecord::CapabilityResultMsg(_) => "CapabilityResult",
        ProtocolRecord::UiNode(_) => "UiNode",
        ProtocolRecord::UiActionEvent(_) => "UiActionEvent",
        ProtocolRecord::UiRenderRequest(_) => "UiRenderRequest",
        ProtocolRecord::LogicalObjectDescriptor(_) => "LogicalObjectDescriptor",
        ProtocolRecord::StoredRepresentationHeader(_) => "StoredRepresentationHeader",
        ProtocolRecord::ChunkEntry(_) => "ChunkEntry",
        ProtocolRecord::ChunkManifest(_) => "ChunkManifest",
        ProtocolRecord::AggregateTrustPolicy(_) => "AggregateTrustPolicy",
        ProtocolRecord::RouteTrustAssignment(_) => "RouteTrustAssignment",
        ProtocolRecord::RouteTrustAssignments(_) => "RouteTrustAssignments",
        ProtocolRecord::RouteSelectionPolicy(_) => "RouteSelectionPolicy",
        ProtocolRecord::AssuranceRequirement(_) => "AssuranceRequirement",
        ProtocolRecord::ConstraintSet(_) => "ConstraintSet",
        ProtocolRecord::CapabilityDescriptor(_) => "CapabilityDescriptor",
        ProtocolRecord::StreamHeadsProof(_) => "StreamHeadsProof",
        ProtocolRecord::SnapshotSetProof(_) => "SnapshotSetProof",
        ProtocolRecord::EventSetProof(_) => "EventSetProof",
        ProtocolRecord::ObjectAssertionProof(_) => "ObjectAssertionProof",
        ProtocolRecord::ResultFragmentProof(_) => "ResultFragmentProof",
        ProtocolRecord::AggregateSummaryProof(_) => "AggregateSummaryProof",
        ProtocolRecord::TrustPolicyProof(_) => "TrustPolicyProof",
        ProtocolRecord::ProofBundle(_) => "ProofBundle",
        ProtocolRecord::ReachabilityHint(_) => "ReachabilityHint",
        ProtocolRecord::EventEnvelope(_)
        | ProtocolRecord::CommandEnvelope(_)
        | ProtocolRecord::CommandResultPayload(_) => unreachable!(),
    };
    edgerun_wire::canonical_bytes(&edgerun_wire::struct_value(vec![
        edgerun_wire::field(1, edgerun_wire::text(name)),
        edgerun_wire::field(2, edgerun_wire::boolv(signable)),
    ]))
}
