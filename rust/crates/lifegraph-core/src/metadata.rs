#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageKind {
    IdentityRef,
    NodeRef,
    StreamRef,
    EventRef,
    HeadRef,
    CheckpointRef,
    ObjectRef,
    RepresentationRef,
    CommandRef,
    DelegationRef,
    RevocationRef,
    SnapshotRef,
    Digest,
    Signature,
    TimeWindow,
    CostLimit,
    ScopeDescriptor,
    Capability,
    DelegationRecord,
    CommandEnvelope,
    EventEnvelope,
    SnapshotDescriptor,
    QueryRequest,
    QueryResultFragment,
    FederatedAggregateDescriptor,
    StreamHeadsProof,
    SnapshotSetProof,
    EventSetProof,
    ObjectAssertionProof,
    ResultFragmentProof,
    AggregateSummaryProof,
    TrustPolicyProof,
    ProofBundle,
    ReachabilityHint,
    RouteAdvertisement,
    SessionHello,
    SessionAccept,
    RelayEnvelope,
    RateLimit,
    IdentityRecord,
    AssuranceClaim,
    AssuranceRequirement,
    ConstraintSet,
    CapabilityDescriptor,
    RevocationRecord,
    LogicalObjectDescriptor,
    StoredRepresentationHeader,
    ChunkEntry,
    ChunkManifest,
    AggregateTrustPolicy,
    RouteTrustAssignment,
    RouteTrustAssignments,
    RouteSelectionPolicy,
    NodeGenesisPayload,
    CommandSentPayload,
    CommandResultPayload,
    ActionLifecyclePayload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Scalar,
    Bytes,
    Timestamp,
    Message,
    RepeatedScalar,
    RepeatedMessage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldSpec {
    pub number: usize,
    pub name: &'static str,
    pub kind: FieldKind,
}

pub fn message_kind_by_name(name: &str) -> Option<MessageKind> {
    match name {
        "IdentityRef" => Some(MessageKind::IdentityRef),
        "NodeRef" => Some(MessageKind::NodeRef),
        "StreamRef" => Some(MessageKind::StreamRef),
        "EventRef" => Some(MessageKind::EventRef),
        "HeadRef" => Some(MessageKind::HeadRef),
        "CheckpointRef" => Some(MessageKind::CheckpointRef),
        "ObjectRef" => Some(MessageKind::ObjectRef),
        "RepresentationRef" => Some(MessageKind::RepresentationRef),
        "CommandRef" => Some(MessageKind::CommandRef),
        "DelegationRef" => Some(MessageKind::DelegationRef),
        "RevocationRef" => Some(MessageKind::RevocationRef),
        "SnapshotRef" => Some(MessageKind::SnapshotRef),
        "Digest" => Some(MessageKind::Digest),
        "Signature" => Some(MessageKind::Signature),
        "TimeWindow" => Some(MessageKind::TimeWindow),
        "CostLimit" => Some(MessageKind::CostLimit),
        "ScopeDescriptor" => Some(MessageKind::ScopeDescriptor),
        "Capability" => Some(MessageKind::Capability),
        "DelegationRecord" => Some(MessageKind::DelegationRecord),
        "CommandEnvelope" => Some(MessageKind::CommandEnvelope),
        "EventEnvelope" => Some(MessageKind::EventEnvelope),
        "SnapshotDescriptor" => Some(MessageKind::SnapshotDescriptor),
        "QueryRequest" => Some(MessageKind::QueryRequest),
        "QueryResultFragment" => Some(MessageKind::QueryResultFragment),
        "FederatedAggregateDescriptor" => Some(MessageKind::FederatedAggregateDescriptor),
        "StreamHeadsProof" => Some(MessageKind::StreamHeadsProof),
        "SnapshotSetProof" => Some(MessageKind::SnapshotSetProof),
        "EventSetProof" => Some(MessageKind::EventSetProof),
        "ObjectAssertionProof" => Some(MessageKind::ObjectAssertionProof),
        "ResultFragmentProof" => Some(MessageKind::ResultFragmentProof),
        "AggregateSummaryProof" => Some(MessageKind::AggregateSummaryProof),
        "TrustPolicyProof" => Some(MessageKind::TrustPolicyProof),
        "ProofBundle" => Some(MessageKind::ProofBundle),
        "ReachabilityHint" => Some(MessageKind::ReachabilityHint),
        "RouteAdvertisement" => Some(MessageKind::RouteAdvertisement),
        "SessionHello" => Some(MessageKind::SessionHello),
        "SessionAccept" => Some(MessageKind::SessionAccept),
        "RelayEnvelope" => Some(MessageKind::RelayEnvelope),
        "RateLimit" => Some(MessageKind::RateLimit),
        "IdentityRecord" => Some(MessageKind::IdentityRecord),
        "AssuranceClaim" => Some(MessageKind::AssuranceClaim),
        "AssuranceRequirement" => Some(MessageKind::AssuranceRequirement),
        "ConstraintSet" => Some(MessageKind::ConstraintSet),
        "CapabilityDescriptor" => Some(MessageKind::CapabilityDescriptor),
        "RevocationRecord" => Some(MessageKind::RevocationRecord),
        "LogicalObjectDescriptor" => Some(MessageKind::LogicalObjectDescriptor),
        "StoredRepresentationHeader" => Some(MessageKind::StoredRepresentationHeader),
        "ChunkEntry" => Some(MessageKind::ChunkEntry),
        "ChunkManifest" => Some(MessageKind::ChunkManifest),
        "AggregateTrustPolicy" => Some(MessageKind::AggregateTrustPolicy),
        "RouteTrustAssignment" => Some(MessageKind::RouteTrustAssignment),
        "RouteTrustAssignments" => Some(MessageKind::RouteTrustAssignments),
        "RouteSelectionPolicy" => Some(MessageKind::RouteSelectionPolicy),
        "NodeGenesisPayload" => Some(MessageKind::NodeGenesisPayload),
        "CommandSentPayload" => Some(MessageKind::CommandSentPayload),
        "CommandResultPayload" => Some(MessageKind::CommandResultPayload),
        "ActionLifecyclePayload" => Some(MessageKind::ActionLifecyclePayload),
        _ => None,
    }
}

pub fn field_specs(kind: MessageKind) -> &'static [FieldSpec] {
    use FieldKind as K;
    use FieldSpec as F;
    match kind {
        MessageKind::IdentityRef => &[
            F {
                number: 1,
                name: "identity_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "identity_kind",
                kind: K::Scalar,
            },
            F {
                number: 3,
                name: "key_hint",
                kind: K::Bytes,
            },
        ],
        MessageKind::NodeRef => &[F {
            number: 1,
            name: "node_id",
            kind: K::Bytes,
        }],
        MessageKind::StreamRef => &[F {
            number: 1,
            name: "stream_id",
            kind: K::Bytes,
        }],
        MessageKind::EventRef => &[
            F {
                number: 1,
                name: "stream_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "seq",
                kind: K::Scalar,
            },
            F {
                number: 3,
                name: "event_hash",
                kind: K::Message,
            },
        ],
        MessageKind::HeadRef => &[
            F {
                number: 1,
                name: "stream_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "seq",
                kind: K::Scalar,
            },
            F {
                number: 3,
                name: "event_hash",
                kind: K::Message,
            },
        ],
        MessageKind::CheckpointRef => &[
            F {
                number: 1,
                name: "checkpoint_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "heads",
                kind: K::RepeatedMessage,
            },
        ],
        MessageKind::ObjectRef => &[
            F {
                number: 1,
                name: "object_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "object_kind",
                kind: K::Scalar,
            },
        ],
        MessageKind::RepresentationRef => &[
            F {
                number: 1,
                name: "representation_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "object_id",
                kind: K::Bytes,
            },
        ],
        MessageKind::CommandRef => &[
            F {
                number: 1,
                name: "command_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "command_hash",
                kind: K::Message,
            },
        ],
        MessageKind::DelegationRef => &[
            F {
                number: 1,
                name: "delegation_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "delegation_hash",
                kind: K::Message,
            },
        ],
        MessageKind::RevocationRef => &[
            F {
                number: 1,
                name: "revocation_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "revocation_hash",
                kind: K::Message,
            },
        ],
        MessageKind::SnapshotRef => &[
            F {
                number: 1,
                name: "snapshot_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "object_id",
                kind: K::Bytes,
            },
        ],
        MessageKind::Digest => &[
            F {
                number: 1,
                name: "algorithm",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "value",
                kind: K::Bytes,
            },
        ],
        MessageKind::Signature => &[
            F {
                number: 1,
                name: "algorithm",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "value",
                kind: K::Bytes,
            },
        ],
        MessageKind::TimeWindow => &[
            F {
                number: 1,
                name: "not_before",
                kind: K::Timestamp,
            },
            F {
                number: 2,
                name: "expires_at",
                kind: K::Timestamp,
            },
        ],
        MessageKind::CostLimit => &[
            F {
                number: 1,
                name: "max_results",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "max_total_bytes",
                kind: K::Scalar,
            },
            F {
                number: 3,
                name: "max_wall_time",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "max_federated_responders",
                kind: K::Scalar,
            },
        ],
        MessageKind::ScopeDescriptor => &[
            F {
                number: 1,
                name: "scope_kind",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "segments",
                kind: K::RepeatedScalar,
            },
        ],
        MessageKind::Capability => &[
            F {
                number: 1,
                name: "kind",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "actions",
                kind: K::RepeatedScalar,
            },
            F {
                number: 3,
                name: "scope",
                kind: K::Message,
            },
        ],
        MessageKind::DelegationRecord => &[
            F {
                number: 1,
                name: "delegation_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "issuer",
                kind: K::Message,
            },
            F {
                number: 3,
                name: "recipient",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "capability",
                kind: K::Message,
            },
            F {
                number: 5,
                name: "issued_at",
                kind: K::Timestamp,
            },
            F {
                number: 6,
                name: "expires_at",
                kind: K::Timestamp,
            },
            F {
                number: 7,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::CommandEnvelope => &[
            F {
                number: 1,
                name: "envelope_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "command_id",
                kind: K::Bytes,
            },
            F {
                number: 3,
                name: "target_node",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "issuer",
                kind: K::Message,
            },
            F {
                number: 5,
                name: "command_type",
                kind: K::Scalar,
            },
            F {
                number: 6,
                name: "command_version",
                kind: K::Scalar,
            },
            F {
                number: 7,
                name: "issued_at",
                kind: K::Timestamp,
            },
            F {
                number: 8,
                name: "not_before",
                kind: K::Timestamp,
            },
            F {
                number: 9,
                name: "expires_at",
                kind: K::Timestamp,
            },
            F {
                number: 10,
                name: "idempotency_key",
                kind: K::Bytes,
            },
            F {
                number: 11,
                name: "payload_object",
                kind: K::Message,
            },
            F {
                number: 12,
                name: "inline_payload",
                kind: K::Bytes,
            },
            F {
                number: 13,
                name: "delegation_chain",
                kind: K::RepeatedScalar,
            },
            F {
                number: 14,
                name: "requested_assurance",
                kind: K::Message,
            },
            F {
                number: 15,
                name: "command_metadata",
                kind: K::Message,
            },
            F {
                number: 16,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::EventEnvelope => &[
            F {
                number: 1,
                name: "envelope_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "stream_id",
                kind: K::Bytes,
            },
            F {
                number: 3,
                name: "seq",
                kind: K::Scalar,
            },
            F {
                number: 4,
                name: "prev_event_hash",
                kind: K::Message,
            },
            F {
                number: 5,
                name: "event_type",
                kind: K::Scalar,
            },
            F {
                number: 6,
                name: "event_version",
                kind: K::Scalar,
            },
            F {
                number: 7,
                name: "recorded_at",
                kind: K::Timestamp,
            },
            F {
                number: 8,
                name: "effective_at",
                kind: K::Timestamp,
            },
            F {
                number: 9,
                name: "payload_object",
                kind: K::Message,
            },
            F {
                number: 10,
                name: "related_events",
                kind: K::RepeatedScalar,
            },
            F {
                number: 11,
                name: "related_commands",
                kind: K::RepeatedScalar,
            },
            F {
                number: 12,
                name: "related_objects",
                kind: K::RepeatedScalar,
            },
            F {
                number: 13,
                name: "related_delegations",
                kind: K::RepeatedScalar,
            },
            F {
                number: 14,
                name: "related_revocations",
                kind: K::RepeatedScalar,
            },
            F {
                number: 15,
                name: "event_metadata",
                kind: K::Message,
            },
            F {
                number: 16,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::SnapshotDescriptor => &[
            F {
                number: 1,
                name: "descriptor_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "snapshot_id",
                kind: K::Bytes,
            },
            F {
                number: 3,
                name: "view_type",
                kind: K::Scalar,
            },
            F {
                number: 4,
                name: "view_version",
                kind: K::Scalar,
            },
            F {
                number: 5,
                name: "producer",
                kind: K::Message,
            },
            F {
                number: 6,
                name: "produced_at",
                kind: K::Timestamp,
            },
            F {
                number: 7,
                name: "base_heads",
                kind: K::RepeatedMessage,
            },
            F {
                number: 8,
                name: "base_checkpoints",
                kind: K::RepeatedMessage,
            },
            F {
                number: 9,
                name: "scope",
                kind: K::Message,
            },
            F {
                number: 10,
                name: "completeness",
                kind: K::Scalar,
            },
            F {
                number: 11,
                name: "payload_object",
                kind: K::Message,
            },
            F {
                number: 12,
                name: "supersedes",
                kind: K::Message,
            },
            F {
                number: 13,
                name: "snapshot_metadata",
                kind: K::Message,
            },
            F {
                number: 14,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::QueryRequest => &[
            F {
                number: 1,
                name: "request_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "query_id",
                kind: K::Bytes,
            },
            F {
                number: 3,
                name: "requester",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "target_scope",
                kind: K::Message,
            },
            F {
                number: 5,
                name: "query_class",
                kind: K::Scalar,
            },
            F {
                number: 6,
                name: "time_window",
                kind: K::Message,
            },
            F {
                number: 7,
                name: "checkpoint_base",
                kind: K::Message,
            },
            F {
                number: 8,
                name: "result_limit",
                kind: K::Scalar,
            },
            F {
                number: 9,
                name: "cost_limit",
                kind: K::Message,
            },
            F {
                number: 10,
                name: "required_proof_classes",
                kind: K::RepeatedScalar,
            },
            F {
                number: 11,
                name: "query_payload_object",
                kind: K::Message,
            },
            F {
                number: 12,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::QueryResultFragment => &[
            F {
                number: 1,
                name: "fragment_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "query_id",
                kind: K::Bytes,
            },
            F {
                number: 3,
                name: "responder",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "answered_at",
                kind: K::Timestamp,
            },
            F {
                number: 5,
                name: "completeness",
                kind: K::Scalar,
            },
            F {
                number: 6,
                name: "snapshot_refs",
                kind: K::RepeatedMessage,
            },
            F {
                number: 7,
                name: "event_refs",
                kind: K::RepeatedMessage,
            },
            F {
                number: 8,
                name: "object_refs",
                kind: K::RepeatedMessage,
            },
            F {
                number: 9,
                name: "proof_objects",
                kind: K::RepeatedMessage,
            },
            F {
                number: 10,
                name: "omission_reason",
                kind: K::Scalar,
            },
            F {
                number: 11,
                name: "bundled_result_object",
                kind: K::Message,
            },
            F {
                number: 12,
                name: "result_metadata",
                kind: K::Message,
            },
            F {
                number: 13,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::FederatedAggregateDescriptor => &[
            F {
                number: 1,
                name: "descriptor_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "aggregate_id",
                kind: K::Bytes,
            },
            F {
                number: 3,
                name: "source_query_id",
                kind: K::Bytes,
            },
            F {
                number: 4,
                name: "aggregator",
                kind: K::Message,
            },
            F {
                number: 5,
                name: "aggregated_at",
                kind: K::Timestamp,
            },
            F {
                number: 6,
                name: "input_fragments",
                kind: K::RepeatedMessage,
            },
            F {
                number: 7,
                name: "aggregation_policy_object",
                kind: K::Message,
            },
            F {
                number: 8,
                name: "payload_object",
                kind: K::Message,
            },
            F {
                number: 9,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::StreamHeadsProof => &[
            F {
                number: 1,
                name: "source_query_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "heads",
                kind: K::RepeatedMessage,
            },
        ],
        MessageKind::SnapshotSetProof => &[
            F {
                number: 1,
                name: "source_query_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "snapshots",
                kind: K::RepeatedMessage,
            },
        ],
        MessageKind::EventSetProof => &[
            F {
                number: 1,
                name: "source_query_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "events",
                kind: K::RepeatedMessage,
            },
            F {
                number: 3,
                name: "related_objects",
                kind: K::RepeatedMessage,
            },
        ],
        MessageKind::ObjectAssertionProof => &[
            F {
                number: 1,
                name: "source_query_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "object_ref",
                kind: K::Message,
            },
            F {
                number: 3,
                name: "exists",
                kind: K::Scalar,
            },
            F {
                number: 4,
                name: "bundled_result_object",
                kind: K::Message,
            },
        ],
        MessageKind::ResultFragmentProof => &[F {
            number: 1,
            name: "fragment",
            kind: K::Message,
        }],
        MessageKind::AggregateSummaryProof => &[
            F {
                number: 1,
                name: "source_query_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "included_responders",
                kind: K::RepeatedMessage,
            },
            F {
                number: 3,
                name: "excluded_responders",
                kind: K::RepeatedMessage,
            },
            F {
                number: 4,
                name: "total_trust_score",
                kind: K::Scalar,
            },
            F {
                number: 5,
                name: "trust_policy_object",
                kind: K::Message,
            },
        ],
        MessageKind::TrustPolicyProof => &[
            F {
                number: 1,
                name: "source_query_id",
                kind: K::Bytes,
            },
            F {
                number: 2,
                name: "policy_object",
                kind: K::Message,
            },
            F {
                number: 3,
                name: "assignments_object",
                kind: K::Message,
            },
        ],
        MessageKind::ProofBundle => &[
            F {
                number: 1,
                name: "bundle_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "payload_type",
                kind: K::Scalar,
            },
            F {
                number: 3,
                name: "source_query_id",
                kind: K::Bytes,
            },
            F {
                number: 4,
                name: "payload_object",
                kind: K::Message,
            },
            F {
                number: 5,
                name: "supporting_objects",
                kind: K::RepeatedMessage,
            },
            F {
                number: 6,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::ReachabilityHint => &[
            F {
                number: 1,
                name: "hint_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "subject_node",
                kind: K::Message,
            },
            F {
                number: 3,
                name: "transport_class",
                kind: K::Scalar,
            },
            F {
                number: 4,
                name: "locator_payload",
                kind: K::Bytes,
            },
            F {
                number: 5,
                name: "directness",
                kind: K::Scalar,
            },
            F {
                number: 6,
                name: "valid_after",
                kind: K::Timestamp,
            },
            F {
                number: 7,
                name: "valid_until",
                kind: K::Timestamp,
            },
            F {
                number: 8,
                name: "cost_hint",
                kind: K::Scalar,
            },
            F {
                number: 9,
                name: "quality_hint",
                kind: K::Scalar,
            },
            F {
                number: 10,
                name: "issuer",
                kind: K::Message,
            },
            F {
                number: 11,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::RouteAdvertisement => &[
            F {
                number: 1,
                name: "advertisement_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "target_node",
                kind: K::Message,
            },
            F {
                number: 3,
                name: "advertiser",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "next_hop_node",
                kind: K::Message,
            },
            F {
                number: 5,
                name: "reachability",
                kind: K::RepeatedMessage,
            },
            F {
                number: 6,
                name: "metric_hint",
                kind: K::Message,
            },
            F {
                number: 7,
                name: "advertised_at",
                kind: K::Timestamp,
            },
            F {
                number: 8,
                name: "expires_at",
                kind: K::Timestamp,
            },
            F {
                number: 9,
                name: "route_metadata",
                kind: K::Message,
            },
            F {
                number: 10,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::SessionHello => &[
            F {
                number: 1,
                name: "message_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "initiator",
                kind: K::Message,
            },
            F {
                number: 3,
                name: "target_node",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "supported_transport_features",
                kind: K::RepeatedScalar,
            },
            F {
                number: 5,
                name: "supported_protocol_versions",
                kind: K::RepeatedScalar,
            },
            F {
                number: 6,
                name: "session_nonce",
                kind: K::Bytes,
            },
            F {
                number: 7,
                name: "initiator_locators",
                kind: K::RepeatedMessage,
            },
            F {
                number: 8,
                name: "hello_metadata",
                kind: K::Message,
            },
            F {
                number: 9,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::SessionAccept => &[
            F {
                number: 1,
                name: "message_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "responder",
                kind: K::Message,
            },
            F {
                number: 3,
                name: "echoed_session_nonce",
                kind: K::Bytes,
            },
            F {
                number: 4,
                name: "selected_protocol_version",
                kind: K::Scalar,
            },
            F {
                number: 5,
                name: "selected_transport_features",
                kind: K::RepeatedScalar,
            },
            F {
                number: 6,
                name: "responder_locators",
                kind: K::RepeatedMessage,
            },
            F {
                number: 7,
                name: "accept_metadata",
                kind: K::Message,
            },
            F {
                number: 8,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::RelayEnvelope => &[
            F {
                number: 1,
                name: "envelope_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "relay_message_id",
                kind: K::Bytes,
            },
            F {
                number: 3,
                name: "original_sender",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "intended_recipient_node",
                kind: K::Message,
            },
            F {
                number: 5,
                name: "relay_chain",
                kind: K::RepeatedMessage,
            },
            F {
                number: 6,
                name: "payload_kind",
                kind: K::Scalar,
            },
            F {
                number: 7,
                name: "payload_object",
                kind: K::Message,
            },
            F {
                number: 8,
                name: "inline_payload",
                kind: K::Bytes,
            },
            F {
                number: 9,
                name: "store_until",
                kind: K::Timestamp,
            },
            F {
                number: 10,
                name: "relay_metadata",
                kind: K::Message,
            },
            F {
                number: 11,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::RateLimit => &[
            F {
                number: 1,
                name: "max_operations",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "per",
                kind: K::Message,
            },
        ],
        MessageKind::IdentityRecord => &[
            F {
                number: 1,
                name: "record_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "identity_id",
                kind: K::Bytes,
            },
            F {
                number: 3,
                name: "identity_kind",
                kind: K::Scalar,
            },
            F {
                number: 4,
                name: "key_algorithm",
                kind: K::Scalar,
            },
            F {
                number: 5,
                name: "public_key",
                kind: K::Bytes,
            },
            F {
                number: 6,
                name: "created_at",
                kind: K::Timestamp,
            },
            F {
                number: 7,
                name: "supersedes_identity",
                kind: K::Message,
            },
            F {
                number: 8,
                name: "assurance_claim_objects",
                kind: K::RepeatedMessage,
            },
            F {
                number: 9,
                name: "metadata_object",
                kind: K::Message,
            },
            F {
                number: 10,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::AssuranceClaim => &[
            F {
                number: 1,
                name: "claim_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "subject_identity",
                kind: K::Message,
            },
            F {
                number: 3,
                name: "subject_node",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "assurance_class",
                kind: K::Scalar,
            },
            F {
                number: 5,
                name: "attester",
                kind: K::Message,
            },
            F {
                number: 6,
                name: "issued_at",
                kind: K::Timestamp,
            },
            F {
                number: 7,
                name: "expires_at",
                kind: K::Timestamp,
            },
            F {
                number: 8,
                name: "evidence_object",
                kind: K::Message,
            },
            F {
                number: 9,
                name: "claim_note",
                kind: K::Scalar,
            },
            F {
                number: 10,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::AssuranceRequirement => &[
            F {
                number: 1,
                name: "assurance_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "required_class",
                kind: K::Scalar,
            },
            F {
                number: 3,
                name: "acceptable_attesters",
                kind: K::RepeatedMessage,
            },
            F {
                number: 4,
                name: "max_evidence_age",
                kind: K::Message,
            },
            F {
                number: 5,
                name: "assurance_metadata",
                kind: K::Message,
            },
        ],
        MessageKind::ConstraintSet => &[
            F {
                number: 1,
                name: "constraint_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "not_before",
                kind: K::Timestamp,
            },
            F {
                number: 3,
                name: "expires_at",
                kind: K::Timestamp,
            },
            F {
                number: 4,
                name: "max_uses",
                kind: K::Scalar,
            },
            F {
                number: 5,
                name: "rate_limit",
                kind: K::Message,
            },
            F {
                number: 6,
                name: "requires_local_session",
                kind: K::Scalar,
            },
            F {
                number: 7,
                name: "requires_user_presence",
                kind: K::Scalar,
            },
            F {
                number: 8,
                name: "requires_transport_classes",
                kind: K::RepeatedScalar,
            },
            F {
                number: 9,
                name: "requires_location_classes",
                kind: K::RepeatedScalar,
            },
            F {
                number: 10,
                name: "export_policy",
                kind: K::Scalar,
            },
            F {
                number: 11,
                name: "execution_class_limits",
                kind: K::RepeatedScalar,
            },
            F {
                number: 12,
                name: "storage_class_limits",
                kind: K::RepeatedScalar,
            },
            F {
                number: 13,
                name: "constraint_metadata",
                kind: K::Message,
            },
        ],
        MessageKind::CapabilityDescriptor => &[
            F {
                number: 1,
                name: "capability_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "capability_kind",
                kind: K::Scalar,
            },
            F {
                number: 3,
                name: "actions",
                kind: K::RepeatedScalar,
            },
            F {
                number: 4,
                name: "scope",
                kind: K::Message,
            },
            F {
                number: 5,
                name: "constraints",
                kind: K::Message,
            },
            F {
                number: 6,
                name: "delegation_policy",
                kind: K::Scalar,
            },
            F {
                number: 7,
                name: "minimum_assurance",
                kind: K::Message,
            },
            F {
                number: 8,
                name: "capability_metadata",
                kind: K::Message,
            },
        ],
        MessageKind::RevocationRecord => &[
            F {
                number: 1,
                name: "record_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "revocation_id",
                kind: K::Bytes,
            },
            F {
                number: 3,
                name: "issuer",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "issued_at",
                kind: K::Timestamp,
            },
            F {
                number: 5,
                name: "effective_at",
                kind: K::Timestamp,
            },
            F {
                number: 6,
                name: "revocation_kind",
                kind: K::Scalar,
            },
            F {
                number: 7,
                name: "target_delegation",
                kind: K::Message,
            },
            F {
                number: 8,
                name: "target_identity",
                kind: K::Message,
            },
            F {
                number: 9,
                name: "target_node",
                kind: K::Message,
            },
            F {
                number: 10,
                name: "target_object",
                kind: K::Message,
            },
            F {
                number: 11,
                name: "scope_override",
                kind: K::Message,
            },
            F {
                number: 12,
                name: "reason_code",
                kind: K::Scalar,
            },
            F {
                number: 13,
                name: "replacement_id",
                kind: K::Bytes,
            },
            F {
                number: 14,
                name: "revocation_metadata",
                kind: K::Message,
            },
            F {
                number: 15,
                name: "signature",
                kind: K::Message,
            },
        ],
        MessageKind::LogicalObjectDescriptor => &[
            F {
                number: 1,
                name: "descriptor_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "object_id",
                kind: K::Bytes,
            },
            F {
                number: 3,
                name: "object_kind",
                kind: K::Scalar,
            },
            F {
                number: 4,
                name: "object_schema_version",
                kind: K::Scalar,
            },
            F {
                number: 5,
                name: "canonicalization_id",
                kind: K::Scalar,
            },
            F {
                number: 6,
                name: "canonical_digest",
                kind: K::Message,
            },
            F {
                number: 7,
                name: "canonical_size",
                kind: K::Scalar,
            },
            F {
                number: 8,
                name: "created_at",
                kind: K::Timestamp,
            },
            F {
                number: 9,
                name: "producer",
                kind: K::Message,
            },
            F {
                number: 10,
                name: "describes_object",
                kind: K::Message,
            },
            F {
                number: 11,
                name: "object_metadata",
                kind: K::Message,
            },
        ],
        MessageKind::StoredRepresentationHeader => &[
            F {
                number: 1,
                name: "header_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "representation_id",
                kind: K::Bytes,
            },
            F {
                number: 3,
                name: "object",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "representation_digest",
                kind: K::Message,
            },
            F {
                number: 5,
                name: "plaintext_size",
                kind: K::Scalar,
            },
            F {
                number: 6,
                name: "stored_size",
                kind: K::Scalar,
            },
            F {
                number: 7,
                name: "encryption_scheme",
                kind: K::Scalar,
            },
            F {
                number: 8,
                name: "compression_scheme",
                kind: K::Scalar,
            },
            F {
                number: 9,
                name: "chunking_mode",
                kind: K::Scalar,
            },
            F {
                number: 10,
                name: "chunk_manifest_object",
                kind: K::Message,
            },
            F {
                number: 11,
                name: "access_package_object",
                kind: K::Message,
            },
            F {
                number: 12,
                name: "created_at",
                kind: K::Timestamp,
            },
            F {
                number: 13,
                name: "representation_metadata",
                kind: K::Message,
            },
        ],
        MessageKind::ChunkEntry => &[
            F {
                number: 1,
                name: "index",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "chunk_representation_id",
                kind: K::Bytes,
            },
            F {
                number: 3,
                name: "chunk_digest",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "offset",
                kind: K::Scalar,
            },
            F {
                number: 5,
                name: "length",
                kind: K::Scalar,
            },
        ],
        MessageKind::ChunkManifest => &[
            F {
                number: 1,
                name: "manifest_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "object",
                kind: K::Message,
            },
            F {
                number: 3,
                name: "representation",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "chunk_count",
                kind: K::Scalar,
            },
            F {
                number: 5,
                name: "total_stored_size",
                kind: K::Scalar,
            },
            F {
                number: 6,
                name: "chunk_entries",
                kind: K::RepeatedMessage,
            },
            F {
                number: 7,
                name: "manifest_metadata",
                kind: K::Message,
            },
        ],
        MessageKind::AggregateTrustPolicy => &[
            F {
                number: 1,
                name: "policy_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "minimum_trust_score",
                kind: K::Scalar,
            },
            F {
                number: 3,
                name: "preferred_aggregators",
                kind: K::RepeatedMessage,
            },
            F {
                number: 4,
                name: "allowed_responders",
                kind: K::RepeatedMessage,
            },
            F {
                number: 5,
                name: "policy_metadata",
                kind: K::Message,
            },
        ],
        MessageKind::RouteTrustAssignment => &[
            F {
                number: 1,
                name: "subject",
                kind: K::Message,
            },
            F {
                number: 2,
                name: "trust_score",
                kind: K::Scalar,
            },
            F {
                number: 3,
                name: "source",
                kind: K::Scalar,
            },
        ],
        MessageKind::RouteTrustAssignments => &[F {
            number: 1,
            name: "assignments",
            kind: K::RepeatedMessage,
        }],
        MessageKind::RouteSelectionPolicy => &[
            F {
                number: 1,
                name: "policy_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "minimum_quality_hint",
                kind: K::Scalar,
            },
            F {
                number: 3,
                name: "maximum_cost_hint",
                kind: K::Scalar,
            },
            F {
                number: 4,
                name: "preferred_advertisers",
                kind: K::RepeatedMessage,
            },
            F {
                number: 5,
                name: "preferred_next_hops",
                kind: K::RepeatedMessage,
            },
            F {
                number: 6,
                name: "require_active_session",
                kind: K::Scalar,
            },
            F {
                number: 7,
                name: "policy_metadata",
                kind: K::Message,
            },
        ],
        MessageKind::NodeGenesisPayload => &[
            F {
                number: 1,
                name: "payload_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "node_id",
                kind: K::Bytes,
            },
            F {
                number: 3,
                name: "primary_node_identity",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "initial_controllers",
                kind: K::RepeatedMessage,
            },
            F {
                number: 5,
                name: "initial_policy_object",
                kind: K::Message,
            },
            F {
                number: 6,
                name: "bootstrap_records",
                kind: K::RepeatedMessage,
            },
            F {
                number: 7,
                name: "assurance_claims",
                kind: K::RepeatedMessage,
            },
            F {
                number: 8,
                name: "node_roles",
                kind: K::RepeatedScalar,
            },
            F {
                number: 9,
                name: "genesis_metadata",
                kind: K::Message,
            },
        ],
        MessageKind::CommandSentPayload => &[
            F {
                number: 1,
                name: "payload_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "command",
                kind: K::Message,
            },
            F {
                number: 3,
                name: "target_node",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "send_metadata",
                kind: K::Message,
            },
        ],
        MessageKind::CommandResultPayload => &[
            F {
                number: 1,
                name: "payload_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "command",
                kind: K::Message,
            },
            F {
                number: 3,
                name: "issuer",
                kind: K::Message,
            },
            F {
                number: 4,
                name: "decision",
                kind: K::Scalar,
            },
            F {
                number: 5,
                name: "decision_basis",
                kind: K::Message,
            },
            F {
                number: 6,
                name: "reason_code",
                kind: K::Scalar,
            },
            F {
                number: 7,
                name: "effect_summary_object",
                kind: K::Message,
            },
            F {
                number: 8,
                name: "result_object",
                kind: K::Message,
            },
        ],
        MessageKind::ActionLifecyclePayload => &[
            F {
                number: 1,
                name: "payload_version",
                kind: K::Scalar,
            },
            F {
                number: 2,
                name: "origin_command",
                kind: K::Message,
            },
            F {
                number: 3,
                name: "action_instance_id",
                kind: K::Bytes,
            },
            F {
                number: 4,
                name: "status",
                kind: K::Scalar,
            },
            F {
                number: 5,
                name: "result_object",
                kind: K::Message,
            },
            F {
                number: 6,
                name: "error_object",
                kind: K::Message,
            },
            F {
                number: 7,
                name: "progress_object",
                kind: K::Message,
            },
            F {
                number: 8,
                name: "action_metadata",
                kind: K::Message,
            },
        ],
    }
}
