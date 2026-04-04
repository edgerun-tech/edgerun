use crate::cbor::CValue;
use crate::metadata::{field_specs, FieldKind, MessageKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timestamp {
    pub seconds: i64,
    pub nanos: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Duration {
    pub seconds: i64,
    pub nanos: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Digest {
    pub algorithm: String,
    pub value: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Signature {
    pub algorithm: String,
    pub value: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityRef {
    pub identity_id: Vec<u8>,
    pub identity_kind: Option<String>,
    pub key_hint: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeRef {
    pub node_id: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamRef {
    pub stream_id: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventRef {
    pub stream_id: Vec<u8>,
    pub seq: i64,
    pub event_hash: Option<Digest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeadRef {
    pub stream_id: Vec<u8>,
    pub seq: i64,
    pub event_hash: Option<Digest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointRef {
    pub checkpoint_id: Option<Vec<u8>>,
    pub heads: Vec<HeadRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectRef {
    pub object_id: Vec<u8>,
    pub object_kind: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepresentationRef {
    pub representation_id: Vec<u8>,
    pub object_id: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandRef {
    pub command_id: Vec<u8>,
    pub command_hash: Option<Digest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DelegationRef {
    pub delegation_id: Vec<u8>,
    pub delegation_hash: Option<Digest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevocationRef {
    pub revocation_id: Vec<u8>,
    pub revocation_hash: Option<Digest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotRef {
    pub snapshot_id: Vec<u8>,
    pub object_id: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeWindow {
    pub not_before: Option<Timestamp>,
    pub expires_at: Option<Timestamp>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CostLimit {
    pub max_results: Option<i64>,
    pub max_total_bytes: Option<i64>,
    pub max_wall_time: Option<Duration>,
    pub max_federated_responders: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopeDescriptor {
    pub scope_version: i64,
    pub scope_kind: String,
    pub target_nodes: Vec<NodeRef>,
    pub target_streams: Vec<StreamRef>,
    pub target_object_kinds: Vec<String>,
    pub target_view_types: Vec<String>,
    pub target_domains: Vec<String>,
    pub time_bounds: Option<TimeWindow>,
    pub scope_metadata: Option<ObjectRef>,
    pub segments: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Capability {
    pub capability_version: i64,
    pub kind: String,
    pub actions: Vec<String>,
    pub scope: Option<ScopeDescriptor>,
    pub constraints: Option<ConstraintSet>,
    pub delegation_policy: Option<String>,
    pub minimum_assurance: Option<AssuranceRequirement>,
    pub capability_metadata: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DelegationRecord {
    pub record_version: i64,
    pub delegation_id: Vec<u8>,
    pub issuer: IdentityRef,
    pub recipient: IdentityRef,
    pub capability: Capability,
    pub issued_at: Option<Timestamp>,
    pub not_before: Option<Timestamp>,
    pub expires_at: Option<Timestamp>,
    pub parent_delegation: Option<DelegationRef>,
    pub revocation_authorities: Vec<IdentityRef>,
    pub delegation_metadata: Option<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandEnvelope {
    pub envelope_version: i64,
    pub command_id: Option<Vec<u8>>,
    pub target_node: NodeRef,
    pub issuer: IdentityRef,
    pub command_type: Option<String>,
    pub command_version: i64,
    pub issued_at: Option<Timestamp>,
    pub not_before: Option<Timestamp>,
    pub expires_at: Option<Timestamp>,
    pub idempotency_key: Option<Vec<u8>>,
    pub payload_object: Option<ObjectRef>,
    pub inline_payload: Option<Vec<u8>>,
    pub delegation_chain: Vec<String>,
    pub requested_assurance: Option<AssuranceRequirement>,
    pub command_metadata: Option<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventEnvelope {
    pub envelope_version: i64,
    pub stream_id: Vec<u8>,
    pub seq: i64,
    pub prev_event_hash: Option<Digest>,
    pub event_type: Option<String>,
    pub event_version: i64,
    pub recorded_at: Option<Timestamp>,
    pub effective_at: Option<Timestamp>,
    pub payload_object: Option<ObjectRef>,
    pub related_events: Vec<EventRef>,
    pub related_commands: Vec<String>,
    pub related_objects: Vec<ObjectRef>,
    pub related_delegations: Vec<DelegationRef>,
    pub related_revocations: Vec<RevocationRef>,
    pub event_metadata: Option<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotDescriptor {
    pub descriptor_version: i64,
    pub snapshot_id: Vec<u8>,
    pub view_type: Option<String>,
    pub view_version: i64,
    pub producer: IdentityRef,
    pub produced_at: Option<Timestamp>,
    pub base_heads: Vec<HeadRef>,
    pub base_checkpoints: Vec<CheckpointRef>,
    pub scope: Option<ScopeDescriptor>,
    pub completeness: Option<String>,
    pub payload_object: Option<ObjectRef>,
    pub supersedes: Option<SnapshotRef>,
    pub snapshot_metadata: Option<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryRequest {
    pub request_version: i64,
    pub query_id: Vec<u8>,
    pub requester: IdentityRef,
    pub target_scope: Option<ScopeDescriptor>,
    pub query_class: Option<String>,
    pub time_window: Option<TimeWindow>,
    pub checkpoint_base: Option<CheckpointRef>,
    pub result_limit: Option<i64>,
    pub cost_limit: Option<CostLimit>,
    pub required_proof_classes: Vec<String>,
    pub query_payload_object: Option<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueryResultFragment {
    pub fragment_version: i64,
    pub query_id: Vec<u8>,
    pub responder: IdentityRef,
    pub answered_at: Option<Timestamp>,
    pub completeness: Option<String>,
    pub snapshot_refs: Vec<SnapshotRef>,
    pub event_refs: Vec<EventRef>,
    pub object_refs: Vec<ObjectRef>,
    pub proof_objects: Vec<ObjectRef>,
    pub omission_reason: Option<String>,
    pub bundled_result_object: Option<ObjectRef>,
    pub result_metadata: Option<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FederatedAggregateDescriptor {
    pub descriptor_version: i64,
    pub aggregate_id: Vec<u8>,
    pub source_query_id: Vec<u8>,
    pub aggregator: IdentityRef,
    pub aggregated_at: Option<Timestamp>,
    pub input_fragments: Vec<ObjectRef>,
    pub aggregation_policy_object: Option<ObjectRef>,
    pub payload_object: Option<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StreamHeadsProof {
    pub source_query_id: Vec<u8>,
    pub heads: Vec<HeadRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SnapshotSetProof {
    pub source_query_id: Vec<u8>,
    pub snapshots: Vec<SnapshotRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventSetProof {
    pub source_query_id: Vec<u8>,
    pub events: Vec<EventRef>,
    pub related_objects: Vec<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectAssertionProof {
    pub source_query_id: Vec<u8>,
    pub object_ref: Option<ObjectRef>,
    pub exists: Option<bool>,
    pub bundled_result_object: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResultFragmentProof {
    pub fragment: QueryResultFragment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregateSummaryProof {
    pub source_query_id: Vec<u8>,
    pub included_responders: Vec<IdentityRef>,
    pub excluded_responders: Vec<IdentityRef>,
    pub total_trust_score: i64,
    pub trust_policy_object: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrustPolicyProof {
    pub source_query_id: Vec<u8>,
    pub policy_object: Option<ObjectRef>,
    pub assignments_object: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofBundle {
    pub bundle_version: i64,
    pub payload_type: Option<String>,
    pub source_query_id: Vec<u8>,
    pub payload_object: Option<ObjectRef>,
    pub supporting_objects: Vec<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReachabilityHint {
    pub hint_version: i64,
    pub subject_node: NodeRef,
    pub transport_class: Option<String>,
    pub locator_payload: Vec<u8>,
    pub directness: Option<String>,
    pub valid_after: Option<Timestamp>,
    pub valid_until: Option<Timestamp>,
    pub cost_hint: Option<i64>,
    pub quality_hint: Option<i64>,
    pub issuer: Option<IdentityRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteAdvertisement {
    pub advertisement_version: i64,
    pub target_node: NodeRef,
    pub advertiser: IdentityRef,
    pub next_hop_node: Option<NodeRef>,
    pub reachability: Vec<ReachabilityHint>,
    pub metric_hint: Option<ObjectRef>,
    pub advertised_at: Option<Timestamp>,
    pub expires_at: Option<Timestamp>,
    pub route_metadata: Option<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionHello {
    pub message_version: i64,
    pub initiator: IdentityRef,
    pub target_node: Option<NodeRef>,
    pub supported_transport_features: Vec<String>,
    pub supported_protocol_versions: Vec<i64>,
    pub session_nonce: Vec<u8>,
    pub initiator_locators: Vec<ReachabilityHint>,
    pub hello_metadata: Option<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionAccept {
    pub message_version: i64,
    pub responder: IdentityRef,
    pub echoed_session_nonce: Vec<u8>,
    pub selected_protocol_version: i64,
    pub selected_transport_features: Vec<String>,
    pub responder_locators: Vec<ReachabilityHint>,
    pub accept_metadata: Option<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelayEnvelope {
    pub envelope_version: i64,
    pub relay_message_id: Vec<u8>,
    pub original_sender: IdentityRef,
    pub intended_recipient_node: NodeRef,
    pub relay_chain: Vec<IdentityRef>,
    pub payload_kind: Option<String>,
    pub payload_object: Option<ObjectRef>,
    pub inline_payload: Option<Vec<u8>>,
    pub store_until: Option<Timestamp>,
    pub relay_metadata: Option<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RateLimit {
    pub max_operations: i64,
    pub per: Option<Duration>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityRecord {
    pub record_version: i64,
    pub identity_id: Vec<u8>,
    pub identity_kind: Option<String>,
    pub key_algorithm: Option<String>,
    pub public_key: Vec<u8>,
    pub created_at: Option<Timestamp>,
    pub supersedes_identity: Option<IdentityRef>,
    pub assurance_claim_objects: Vec<ObjectRef>,
    pub metadata_object: Option<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AssuranceSubject {
    Identity(IdentityRef),
    Node(NodeRef),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssuranceClaim {
    pub claim_version: i64,
    pub subject: Option<AssuranceSubject>,
    pub assurance_class: Option<String>,
    pub attester: IdentityRef,
    pub issued_at: Option<Timestamp>,
    pub expires_at: Option<Timestamp>,
    pub evidence_object: Option<ObjectRef>,
    pub claim_note: Option<String>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssuranceRequirement {
    pub assurance_version: i64,
    pub required_class: Option<String>,
    pub acceptable_attesters: Vec<IdentityRef>,
    pub max_evidence_age: Option<Duration>,
    pub assurance_metadata: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConstraintSet {
    pub constraint_version: i64,
    pub not_before: Option<Timestamp>,
    pub expires_at: Option<Timestamp>,
    pub max_uses: Option<i64>,
    pub rate_limit: Option<RateLimit>,
    pub requires_local_session: Option<bool>,
    pub requires_user_presence: Option<bool>,
    pub requires_transport_classes: Vec<String>,
    pub requires_location_classes: Vec<String>,
    pub export_policy: Option<String>,
    pub execution_class_limits: Vec<String>,
    pub storage_class_limits: Vec<String>,
    pub constraint_metadata: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityDescriptor {
    pub capability_version: i64,
    pub capability_kind: Option<String>,
    pub actions: Vec<String>,
    pub scope: Option<ScopeDescriptor>,
    pub constraints: Option<ConstraintSet>,
    pub delegation_policy: Option<String>,
    pub minimum_assurance: Option<AssuranceRequirement>,
    pub capability_metadata: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RevocationTarget {
    Delegation(DelegationRef),
    Identity(IdentityRef),
    Node(NodeRef),
    Object(ObjectRef),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RevocationRecord {
    pub record_version: i64,
    pub revocation_id: Vec<u8>,
    pub issuer: IdentityRef,
    pub issued_at: Option<Timestamp>,
    pub effective_at: Option<Timestamp>,
    pub revocation_kind: Option<String>,
    pub target: Option<RevocationTarget>,
    pub scope_override: Option<ScopeDescriptor>,
    pub reason_code: Option<String>,
    pub replacement_id: Option<Vec<u8>>,
    pub revocation_metadata: Option<ObjectRef>,
    pub signature: Option<Signature>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogicalObjectDescriptor {
    pub descriptor_version: i64,
    pub object_id: Vec<u8>,
    pub object_kind: Option<String>,
    pub object_schema_version: i64,
    pub canonicalization_id: Option<String>,
    pub canonical_digest: Option<Digest>,
    pub canonical_size: i64,
    pub created_at: Option<Timestamp>,
    pub producer: Option<IdentityRef>,
    pub describes_object: Option<ObjectRef>,
    pub object_metadata: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StoredRepresentationHeader {
    pub header_version: i64,
    pub representation_id: Vec<u8>,
    pub object: Option<ObjectRef>,
    pub representation_digest: Option<Digest>,
    pub plaintext_size: Option<i64>,
    pub stored_size: i64,
    pub encryption_scheme: Option<String>,
    pub compression_scheme: Option<String>,
    pub chunking_mode: Option<String>,
    pub chunk_manifest_object: Option<ObjectRef>,
    pub access_package_object: Option<ObjectRef>,
    pub created_at: Option<Timestamp>,
    pub representation_metadata: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkEntry {
    pub index: i64,
    pub chunk_representation_id: Vec<u8>,
    pub chunk_digest: Option<Digest>,
    pub offset: i64,
    pub length: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChunkManifest {
    pub manifest_version: i64,
    pub object: Option<ObjectRef>,
    pub representation: Option<RepresentationRef>,
    pub chunk_count: i64,
    pub total_stored_size: i64,
    pub chunk_entries: Vec<ChunkEntry>,
    pub manifest_metadata: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregateTrustPolicy {
    pub policy_version: i64,
    pub minimum_trust_score: Option<i64>,
    pub preferred_aggregators: Vec<IdentityRef>,
    pub allowed_responders: Vec<IdentityRef>,
    pub policy_metadata: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteTrustAssignment {
    pub subject: IdentityRef,
    pub trust_score: i64,
    pub source: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteTrustAssignments {
    pub assignments: Vec<RouteTrustAssignment>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RouteSelectionPolicy {
    pub policy_version: i64,
    pub minimum_quality_hint: Option<i64>,
    pub maximum_cost_hint: Option<i64>,
    pub preferred_advertisers: Vec<IdentityRef>,
    pub preferred_next_hops: Vec<NodeRef>,
    pub require_active_session: Option<bool>,
    pub policy_metadata: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeGenesisPayload {
    pub payload_version: i64,
    pub node_id: Vec<u8>,
    pub primary_node_identity: Option<IdentityRef>,
    pub initial_controllers: Vec<IdentityRef>,
    pub initial_policy_object: Option<ObjectRef>,
    pub bootstrap_records: Vec<ObjectRef>,
    pub assurance_claims: Vec<ObjectRef>,
    pub node_roles: Vec<String>,
    pub genesis_metadata: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandSentPayload {
    pub payload_version: i64,
    pub command: Option<CommandRef>,
    pub target_node: Option<NodeRef>,
    pub send_metadata: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandResultPayload {
    pub payload_version: i64,
    pub command: Option<CommandRef>,
    pub issuer: Option<IdentityRef>,
    pub decision: Option<String>,
    pub decision_basis: Option<ObjectRef>,
    pub reason_code: Option<String>,
    pub effect_summary_object: Option<ObjectRef>,
    pub result_object: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActionLifecyclePayload {
    pub payload_version: i64,
    pub origin_command: Option<CommandRef>,
    pub action_instance_id: Vec<u8>,
    pub status: Option<String>,
    pub result_object: Option<ObjectRef>,
    pub error_object: Option<ObjectRef>,
    pub progress_object: Option<ObjectRef>,
    pub action_metadata: Option<ObjectRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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
    Capability(Capability),
    DelegationRecord(DelegationRecord),
    CommandEnvelope(CommandEnvelope),
    EventEnvelope(EventEnvelope),
    SnapshotDescriptor(SnapshotDescriptor),
    QueryRequest(QueryRequest),
    QueryResultFragment(QueryResultFragment),
    FederatedAggregateDescriptor(FederatedAggregateDescriptor),
    StreamHeadsProof(StreamHeadsProof),
    SnapshotSetProof(SnapshotSetProof),
    EventSetProof(EventSetProof),
    ObjectAssertionProof(ObjectAssertionProof),
    ResultFragmentProof(ResultFragmentProof),
    AggregateSummaryProof(AggregateSummaryProof),
    TrustPolicyProof(TrustPolicyProof),
    ProofBundle(ProofBundle),
    ReachabilityHint(ReachabilityHint),
    RouteAdvertisement(RouteAdvertisement),
    SessionHello(SessionHello),
    SessionAccept(SessionAccept),
    RelayEnvelope(RelayEnvelope),
    RateLimit(RateLimit),
    IdentityRecord(IdentityRecord),
    AssuranceClaim(AssuranceClaim),
    AssuranceRequirement(AssuranceRequirement),
    ConstraintSet(ConstraintSet),
    CapabilityDescriptor(CapabilityDescriptor),
    RevocationRecord(RevocationRecord),
    LogicalObjectDescriptor(LogicalObjectDescriptor),
    StoredRepresentationHeader(StoredRepresentationHeader),
    ChunkEntry(ChunkEntry),
    ChunkManifest(ChunkManifest),
    AggregateTrustPolicy(AggregateTrustPolicy),
    RouteTrustAssignment(RouteTrustAssignment),
    RouteTrustAssignments(RouteTrustAssignments),
    RouteSelectionPolicy(RouteSelectionPolicy),
    NodeGenesisPayload(NodeGenesisPayload),
    CommandSentPayload(CommandSentPayload),
    CommandResultPayload(CommandResultPayload),
    ActionLifecyclePayload(ActionLifecyclePayload),
}

impl ProtocolRecord {
    pub fn kind(&self) -> MessageKind {
        match self {
            Self::IdentityRef(_) => MessageKind::IdentityRef,
            Self::NodeRef(_) => MessageKind::NodeRef,
            Self::StreamRef(_) => MessageKind::StreamRef,
            Self::EventRef(_) => MessageKind::EventRef,
            Self::HeadRef(_) => MessageKind::HeadRef,
            Self::CheckpointRef(_) => MessageKind::CheckpointRef,
            Self::ObjectRef(_) => MessageKind::ObjectRef,
            Self::RepresentationRef(_) => MessageKind::RepresentationRef,
            Self::CommandRef(_) => MessageKind::CommandRef,
            Self::DelegationRef(_) => MessageKind::DelegationRef,
            Self::RevocationRef(_) => MessageKind::RevocationRef,
            Self::SnapshotRef(_) => MessageKind::SnapshotRef,
            Self::Digest(_) => MessageKind::Digest,
            Self::Signature(_) => MessageKind::Signature,
            Self::TimeWindow(_) => MessageKind::TimeWindow,
            Self::CostLimit(_) => MessageKind::CostLimit,
            Self::ScopeDescriptor(_) => MessageKind::ScopeDescriptor,
            Self::Capability(_) => MessageKind::Capability,
            Self::DelegationRecord(_) => MessageKind::DelegationRecord,
            Self::CommandEnvelope(_) => MessageKind::CommandEnvelope,
            Self::EventEnvelope(_) => MessageKind::EventEnvelope,
            Self::SnapshotDescriptor(_) => MessageKind::SnapshotDescriptor,
            Self::QueryRequest(_) => MessageKind::QueryRequest,
            Self::QueryResultFragment(_) => MessageKind::QueryResultFragment,
            Self::FederatedAggregateDescriptor(_) => MessageKind::FederatedAggregateDescriptor,
            Self::StreamHeadsProof(_) => MessageKind::StreamHeadsProof,
            Self::SnapshotSetProof(_) => MessageKind::SnapshotSetProof,
            Self::EventSetProof(_) => MessageKind::EventSetProof,
            Self::ObjectAssertionProof(_) => MessageKind::ObjectAssertionProof,
            Self::ResultFragmentProof(_) => MessageKind::ResultFragmentProof,
            Self::AggregateSummaryProof(_) => MessageKind::AggregateSummaryProof,
            Self::TrustPolicyProof(_) => MessageKind::TrustPolicyProof,
            Self::ProofBundle(_) => MessageKind::ProofBundle,
            Self::ReachabilityHint(_) => MessageKind::ReachabilityHint,
            Self::RouteAdvertisement(_) => MessageKind::RouteAdvertisement,
            Self::SessionHello(_) => MessageKind::SessionHello,
            Self::SessionAccept(_) => MessageKind::SessionAccept,
            Self::RelayEnvelope(_) => MessageKind::RelayEnvelope,
            Self::RateLimit(_) => MessageKind::RateLimit,
            Self::IdentityRecord(_) => MessageKind::IdentityRecord,
            Self::AssuranceClaim(_) => MessageKind::AssuranceClaim,
            Self::AssuranceRequirement(_) => MessageKind::AssuranceRequirement,
            Self::ConstraintSet(_) => MessageKind::ConstraintSet,
            Self::CapabilityDescriptor(_) => MessageKind::CapabilityDescriptor,
            Self::RevocationRecord(_) => MessageKind::RevocationRecord,
            Self::LogicalObjectDescriptor(_) => MessageKind::LogicalObjectDescriptor,
            Self::StoredRepresentationHeader(_) => MessageKind::StoredRepresentationHeader,
            Self::ChunkEntry(_) => MessageKind::ChunkEntry,
            Self::ChunkManifest(_) => MessageKind::ChunkManifest,
            Self::AggregateTrustPolicy(_) => MessageKind::AggregateTrustPolicy,
            Self::RouteTrustAssignment(_) => MessageKind::RouteTrustAssignment,
            Self::RouteTrustAssignments(_) => MessageKind::RouteTrustAssignments,
            Self::RouteSelectionPolicy(_) => MessageKind::RouteSelectionPolicy,
            Self::NodeGenesisPayload(_) => MessageKind::NodeGenesisPayload,
            Self::CommandSentPayload(_) => MessageKind::CommandSentPayload,
            Self::CommandResultPayload(_) => MessageKind::CommandResultPayload,
            Self::ActionLifecyclePayload(_) => MessageKind::ActionLifecyclePayload,
        }
    }

    pub fn to_cvalue(&self, signable: bool) -> CValue {
        let specs = field_specs(self.kind());
        let mut slots = vec![CValue::Null; specs.iter().map(|s| s.number).max().unwrap_or(0)];
        for spec in specs {
            if signable && spec.name == "signature" {
                slots[spec.number - 1] = CValue::Null;
                continue;
            }
            slots[spec.number - 1] = self.field_value(spec.name, spec.kind, signable);
        }
        CValue::Array(slots)
    }

    fn field_value(&self, name: &str, kind: FieldKind, signable: bool) -> CValue {
        match self {
            Self::IdentityRef(v) => match name {
                "identity_id" => CValue::Bytes(v.identity_id.clone()),
                "identity_kind" => opt_text(&v.identity_kind),
                "key_hint" => opt_bytes(&v.key_hint),
                _ => CValue::Null,
            },
            Self::NodeRef(v) => match name {
                "node_id" => CValue::Bytes(v.node_id.clone()),
                _ => CValue::Null,
            },
            Self::StreamRef(v) => match name {
                "stream_id" => CValue::Bytes(v.stream_id.clone()),
                _ => CValue::Null,
            },
            Self::EventRef(v) => match name {
                "stream_id" => CValue::Bytes(v.stream_id.clone()),
                "seq" => CValue::Int(v.seq),
                "event_hash" => opt_digest(&v.event_hash),
                _ => CValue::Null,
            },
            Self::HeadRef(v) => match name {
                "stream_id" => CValue::Bytes(v.stream_id.clone()),
                "seq" => CValue::Int(v.seq),
                "event_hash" => opt_digest(&v.event_hash),
                _ => CValue::Null,
            },
            Self::CheckpointRef(v) => match name {
                "checkpoint_id" => opt_bytes(&v.checkpoint_id),
                "heads" => repeated(v.heads.iter().cloned().map(ProtocolRecord::HeadRef)),
                _ => CValue::Null,
            },
            Self::ObjectRef(v) => match name {
                "object_id" => CValue::Bytes(v.object_id.clone()),
                "object_kind" => opt_text(&v.object_kind),
                _ => CValue::Null,
            },
            Self::RepresentationRef(v) => match name {
                "representation_id" => CValue::Bytes(v.representation_id.clone()),
                "object_id" => CValue::Bytes(v.object_id.clone()),
                _ => CValue::Null,
            },
            Self::CommandRef(v) => match name {
                "command_id" => CValue::Bytes(v.command_id.clone()),
                "command_hash" => opt_digest(&v.command_hash),
                _ => CValue::Null,
            },
            Self::DelegationRef(v) => match name {
                "delegation_id" => CValue::Bytes(v.delegation_id.clone()),
                "delegation_hash" => opt_digest(&v.delegation_hash),
                _ => CValue::Null,
            },
            Self::RevocationRef(v) => match name {
                "revocation_id" => CValue::Bytes(v.revocation_id.clone()),
                "revocation_hash" => opt_digest(&v.revocation_hash),
                _ => CValue::Null,
            },
            Self::SnapshotRef(v) => match name {
                "snapshot_id" => CValue::Bytes(v.snapshot_id.clone()),
                "object_id" => opt_bytes(&v.object_id),
                _ => CValue::Null,
            },
            Self::Digest(v) => match name {
                "algorithm" => CValue::Text(v.algorithm.clone()),
                "value" => CValue::Bytes(v.value.clone()),
                _ => CValue::Null,
            },
            Self::Signature(v) => match name {
                "algorithm" => CValue::Text(v.algorithm.clone()),
                "value" => CValue::Bytes(v.value.clone()),
                _ => CValue::Null,
            },
            Self::TimeWindow(v) => match name {
                "not_before" => timestamp_value(v.not_before.as_ref()),
                "expires_at" => timestamp_value(v.expires_at.as_ref()),
                _ => CValue::Null,
            },
            Self::CostLimit(v) => match name {
                "max_results" => opt_int(&v.max_results),
                "max_total_bytes" => opt_int(&v.max_total_bytes),
                "max_wall_time" => duration_value(v.max_wall_time.as_ref()),
                "max_federated_responders" => opt_int(&v.max_federated_responders),
                _ => CValue::Null,
            },
            Self::ScopeDescriptor(v) => match name {
                "scope_kind" => CValue::Text(v.scope_kind.clone()),
                "segments" => CValue::Array(v.segments.iter().cloned().map(CValue::Text).collect()),
                _ => CValue::Null,
            },
            Self::Capability(v) => match name {
                "kind" => CValue::Text(v.kind.clone()),
                "actions" => CValue::Array(v.actions.iter().cloned().map(CValue::Text).collect()),
                "scope" => opt_record(v.scope.clone().map(ProtocolRecord::ScopeDescriptor)),
                _ => CValue::Null,
            },
            Self::DelegationRecord(v) => match name {
                "delegation_id" => CValue::Bytes(v.delegation_id.clone()),
                "issuer" => ProtocolRecord::IdentityRef(v.issuer.clone()).to_cvalue(false),
                "recipient" => ProtocolRecord::IdentityRef(v.recipient.clone()).to_cvalue(false),
                "capability" => ProtocolRecord::Capability(v.capability.clone()).to_cvalue(false),
                "issued_at" => timestamp_value(v.issued_at.as_ref()),
                "not_before" => timestamp_value(v.not_before.as_ref()),
                "expires_at" => timestamp_value(v.expires_at.as_ref()),
                "parent_delegation" => opt_record(
                    v.parent_delegation
                        .clone()
                        .map(ProtocolRecord::DelegationRef),
                ),
                "revocation_authorities" => repeated(
                    v.revocation_authorities
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::IdentityRef),
                ),
                "delegation_metadata" => {
                    opt_record(v.delegation_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                "signature" => opt_signature(&v.signature, signable),
                _ => CValue::Null,
            },
            Self::CommandEnvelope(v) => match name {
                "envelope_version" => CValue::Int(v.envelope_version),
                "command_id" => opt_bytes(&v.command_id),
                "target_node" => ProtocolRecord::NodeRef(v.target_node.clone()).to_cvalue(false),
                "issuer" => ProtocolRecord::IdentityRef(v.issuer.clone()).to_cvalue(false),
                "command_type" => opt_text(&v.command_type),
                "command_version" => CValue::Int(v.command_version),
                "issued_at" => timestamp_value(v.issued_at.as_ref()),
                "not_before" => timestamp_value(v.not_before.as_ref()),
                "expires_at" => timestamp_value(v.expires_at.as_ref()),
                "idempotency_key" => opt_bytes(&v.idempotency_key),
                "payload_object" => {
                    opt_record(v.payload_object.clone().map(ProtocolRecord::ObjectRef))
                }
                "inline_payload" => opt_bytes(&v.inline_payload),
                "delegation_chain" => CValue::Array(
                    v.delegation_chain
                        .iter()
                        .cloned()
                        .map(CValue::Text)
                        .collect(),
                ),
                "requested_assurance" => opt_record(
                    v.requested_assurance
                        .clone()
                        .map(ProtocolRecord::AssuranceRequirement),
                ),
                "command_metadata" => {
                    opt_record(v.command_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::EventEnvelope(v) => match name {
                "envelope_version" => CValue::Int(v.envelope_version),
                "stream_id" => CValue::Bytes(v.stream_id.clone()),
                "seq" => CValue::Int(v.seq),
                "prev_event_hash" => opt_digest(&v.prev_event_hash),
                "event_type" => opt_text(&v.event_type),
                "event_version" => CValue::Int(v.event_version),
                "recorded_at" => timestamp_value(v.recorded_at.as_ref()),
                "effective_at" => timestamp_value(v.effective_at.as_ref()),
                "payload_object" => {
                    opt_record(v.payload_object.clone().map(ProtocolRecord::ObjectRef))
                }
                "related_events" => repeated(
                    v.related_events
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::EventRef),
                ),
                "related_commands" => CValue::Array(
                    v.related_commands
                        .iter()
                        .cloned()
                        .map(CValue::Text)
                        .collect(),
                ),
                "related_objects" => repeated(
                    v.related_objects
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "related_delegations" => repeated(
                    v.related_delegations
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::DelegationRef),
                ),
                "related_revocations" => repeated(
                    v.related_revocations
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::RevocationRef),
                ),
                "event_metadata" => {
                    opt_record(v.event_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::SnapshotDescriptor(v) => match name {
                "descriptor_version" => CValue::Int(v.descriptor_version),
                "snapshot_id" => CValue::Bytes(v.snapshot_id.clone()),
                "view_type" => opt_text(&v.view_type),
                "view_version" => CValue::Int(v.view_version),
                "producer" => ProtocolRecord::IdentityRef(v.producer.clone()).to_cvalue(false),
                "produced_at" => timestamp_value(v.produced_at.as_ref()),
                "base_heads" => repeated(v.base_heads.iter().cloned().map(ProtocolRecord::HeadRef)),
                "base_checkpoints" => repeated(
                    v.base_checkpoints
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::CheckpointRef),
                ),
                "scope" => opt_record(v.scope.clone().map(ProtocolRecord::ScopeDescriptor)),
                "completeness" => opt_text(&v.completeness),
                "payload_object" => {
                    opt_record(v.payload_object.clone().map(ProtocolRecord::ObjectRef))
                }
                "supersedes" => opt_record(v.supersedes.clone().map(ProtocolRecord::SnapshotRef)),
                "snapshot_metadata" => {
                    opt_record(v.snapshot_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::QueryRequest(v) => match name {
                "request_version" => CValue::Int(v.request_version),
                "query_id" => CValue::Bytes(v.query_id.clone()),
                "requester" => ProtocolRecord::IdentityRef(v.requester.clone()).to_cvalue(false),
                "target_scope" => {
                    opt_record(v.target_scope.clone().map(ProtocolRecord::ScopeDescriptor))
                }
                "query_class" => opt_text(&v.query_class),
                "time_window" => opt_record(v.time_window.clone().map(ProtocolRecord::TimeWindow)),
                "checkpoint_base" => {
                    opt_record(v.checkpoint_base.clone().map(ProtocolRecord::CheckpointRef))
                }
                "result_limit" => opt_int(&v.result_limit),
                "cost_limit" => opt_record(v.cost_limit.clone().map(ProtocolRecord::CostLimit)),
                "required_proof_classes" => CValue::Array(
                    v.required_proof_classes
                        .iter()
                        .cloned()
                        .map(CValue::Text)
                        .collect(),
                ),
                "query_payload_object" => opt_record(
                    v.query_payload_object
                        .clone()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::QueryResultFragment(v) => match name {
                "fragment_version" => CValue::Int(v.fragment_version),
                "query_id" => CValue::Bytes(v.query_id.clone()),
                "responder" => ProtocolRecord::IdentityRef(v.responder.clone()).to_cvalue(false),
                "answered_at" => timestamp_value(v.answered_at.as_ref()),
                "completeness" => opt_text(&v.completeness),
                "snapshot_refs" => repeated(
                    v.snapshot_refs
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::SnapshotRef),
                ),
                "event_refs" => {
                    repeated(v.event_refs.iter().cloned().map(ProtocolRecord::EventRef))
                }
                "object_refs" => {
                    repeated(v.object_refs.iter().cloned().map(ProtocolRecord::ObjectRef))
                }
                "proof_objects" => repeated(
                    v.proof_objects
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "omission_reason" => opt_text(&v.omission_reason),
                "bundled_result_object" => opt_record(
                    v.bundled_result_object
                        .clone()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "result_metadata" => {
                    opt_record(v.result_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::FederatedAggregateDescriptor(v) => match name {
                "descriptor_version" => CValue::Int(v.descriptor_version),
                "aggregate_id" => CValue::Bytes(v.aggregate_id.clone()),
                "source_query_id" => CValue::Bytes(v.source_query_id.clone()),
                "aggregator" => ProtocolRecord::IdentityRef(v.aggregator.clone()).to_cvalue(false),
                "aggregated_at" => timestamp_value(v.aggregated_at.as_ref()),
                "input_fragments" => repeated(
                    v.input_fragments
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "aggregation_policy_object" => opt_record(
                    v.aggregation_policy_object
                        .clone()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "payload_object" => {
                    opt_record(v.payload_object.clone().map(ProtocolRecord::ObjectRef))
                }
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::StreamHeadsProof(v) => match name {
                "source_query_id" => CValue::Bytes(v.source_query_id.clone()),
                "heads" => repeated(v.heads.iter().cloned().map(ProtocolRecord::HeadRef)),
                _ => default_for_kind(kind),
            },
            Self::SnapshotSetProof(v) => match name {
                "source_query_id" => CValue::Bytes(v.source_query_id.clone()),
                "snapshots" => {
                    repeated(v.snapshots.iter().cloned().map(ProtocolRecord::SnapshotRef))
                }
                _ => default_for_kind(kind),
            },
            Self::EventSetProof(v) => match name {
                "source_query_id" => CValue::Bytes(v.source_query_id.clone()),
                "events" => repeated(v.events.iter().cloned().map(ProtocolRecord::EventRef)),
                "related_objects" => repeated(
                    v.related_objects
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::ObjectRef),
                ),
                _ => default_for_kind(kind),
            },
            Self::ObjectAssertionProof(v) => match name {
                "source_query_id" => CValue::Bytes(v.source_query_id.clone()),
                "object_ref" => opt_record(v.object_ref.clone().map(ProtocolRecord::ObjectRef)),
                "exists" => match v.exists {
                    Some(b) => CValue::Bool(b),
                    None => CValue::Null,
                },
                "bundled_result_object" => opt_record(
                    v.bundled_result_object
                        .clone()
                        .map(ProtocolRecord::ObjectRef),
                ),
                _ => default_for_kind(kind),
            },
            Self::ResultFragmentProof(v) => match name {
                "fragment" => {
                    ProtocolRecord::QueryResultFragment(v.fragment.clone()).to_cvalue(false)
                }
                _ => default_for_kind(kind),
            },
            Self::AggregateSummaryProof(v) => match name {
                "source_query_id" => CValue::Bytes(v.source_query_id.clone()),
                "included_responders" => repeated(
                    v.included_responders
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::IdentityRef),
                ),
                "excluded_responders" => repeated(
                    v.excluded_responders
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::IdentityRef),
                ),
                "total_trust_score" => CValue::Int(v.total_trust_score),
                "trust_policy_object" => {
                    opt_record(v.trust_policy_object.clone().map(ProtocolRecord::ObjectRef))
                }
                _ => default_for_kind(kind),
            },
            Self::TrustPolicyProof(v) => match name {
                "source_query_id" => CValue::Bytes(v.source_query_id.clone()),
                "policy_object" => {
                    opt_record(v.policy_object.clone().map(ProtocolRecord::ObjectRef))
                }
                "assignments_object" => {
                    opt_record(v.assignments_object.clone().map(ProtocolRecord::ObjectRef))
                }
                _ => default_for_kind(kind),
            },
            Self::ProofBundle(v) => match name {
                "bundle_version" => CValue::Int(v.bundle_version),
                "payload_type" => opt_text(&v.payload_type),
                "source_query_id" => CValue::Bytes(v.source_query_id.clone()),
                "payload_object" => {
                    opt_record(v.payload_object.clone().map(ProtocolRecord::ObjectRef))
                }
                "supporting_objects" => repeated(
                    v.supporting_objects
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::ReachabilityHint(v) => match name {
                "hint_version" => CValue::Int(v.hint_version),
                "subject_node" => ProtocolRecord::NodeRef(v.subject_node.clone()).to_cvalue(false),
                "transport_class" => opt_text(&v.transport_class),
                "locator_payload" => CValue::Bytes(v.locator_payload.clone()),
                "directness" => opt_text(&v.directness),
                "valid_after" => timestamp_value(v.valid_after.as_ref()),
                "valid_until" => timestamp_value(v.valid_until.as_ref()),
                "cost_hint" => opt_int(&v.cost_hint),
                "quality_hint" => opt_int(&v.quality_hint),
                "issuer" => opt_record(v.issuer.clone().map(ProtocolRecord::IdentityRef)),
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::RouteAdvertisement(v) => match name {
                "advertisement_version" => CValue::Int(v.advertisement_version),
                "target_node" => ProtocolRecord::NodeRef(v.target_node.clone()).to_cvalue(false),
                "advertiser" => ProtocolRecord::IdentityRef(v.advertiser.clone()).to_cvalue(false),
                "next_hop_node" => opt_record(v.next_hop_node.clone().map(ProtocolRecord::NodeRef)),
                "reachability" => repeated(
                    v.reachability
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::ReachabilityHint),
                ),
                "metric_hint" => opt_record(v.metric_hint.clone().map(ProtocolRecord::ObjectRef)),
                "advertised_at" => timestamp_value(v.advertised_at.as_ref()),
                "expires_at" => timestamp_value(v.expires_at.as_ref()),
                "route_metadata" => {
                    opt_record(v.route_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::SessionHello(v) => match name {
                "message_version" => CValue::Int(v.message_version),
                "initiator" => ProtocolRecord::IdentityRef(v.initiator.clone()).to_cvalue(false),
                "target_node" => opt_record(v.target_node.clone().map(ProtocolRecord::NodeRef)),
                "supported_transport_features" => CValue::Array(
                    v.supported_transport_features
                        .iter()
                        .cloned()
                        .map(CValue::Text)
                        .collect(),
                ),
                "supported_protocol_versions" => CValue::Array(
                    v.supported_protocol_versions
                        .iter()
                        .copied()
                        .map(CValue::Int)
                        .collect(),
                ),
                "session_nonce" => CValue::Bytes(v.session_nonce.clone()),
                "initiator_locators" => repeated(
                    v.initiator_locators
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::ReachabilityHint),
                ),
                "hello_metadata" => {
                    opt_record(v.hello_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::SessionAccept(v) => match name {
                "message_version" => CValue::Int(v.message_version),
                "responder" => ProtocolRecord::IdentityRef(v.responder.clone()).to_cvalue(false),
                "echoed_session_nonce" => CValue::Bytes(v.echoed_session_nonce.clone()),
                "selected_protocol_version" => CValue::Int(v.selected_protocol_version),
                "selected_transport_features" => CValue::Array(
                    v.selected_transport_features
                        .iter()
                        .cloned()
                        .map(CValue::Text)
                        .collect(),
                ),
                "responder_locators" => repeated(
                    v.responder_locators
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::ReachabilityHint),
                ),
                "accept_metadata" => {
                    opt_record(v.accept_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::RelayEnvelope(v) => match name {
                "envelope_version" => CValue::Int(v.envelope_version),
                "relay_message_id" => CValue::Bytes(v.relay_message_id.clone()),
                "original_sender" => {
                    ProtocolRecord::IdentityRef(v.original_sender.clone()).to_cvalue(false)
                }
                "intended_recipient_node" => {
                    ProtocolRecord::NodeRef(v.intended_recipient_node.clone()).to_cvalue(false)
                }
                "relay_chain" => repeated(
                    v.relay_chain
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::IdentityRef),
                ),
                "payload_kind" => opt_text(&v.payload_kind),
                "payload_object" => {
                    opt_record(v.payload_object.clone().map(ProtocolRecord::ObjectRef))
                }
                "inline_payload" => opt_bytes(&v.inline_payload),
                "store_until" => timestamp_value(v.store_until.as_ref()),
                "relay_metadata" => {
                    opt_record(v.relay_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::RateLimit(v) => match name {
                "max_operations" => CValue::Int(v.max_operations),
                "per" => duration_value(v.per.as_ref()),
                _ => default_for_kind(kind),
            },
            Self::IdentityRecord(v) => match name {
                "record_version" => CValue::Int(v.record_version),
                "identity_id" => CValue::Bytes(v.identity_id.clone()),
                "identity_kind" => opt_text(&v.identity_kind),
                "key_algorithm" => opt_text(&v.key_algorithm),
                "public_key" => CValue::Bytes(v.public_key.clone()),
                "created_at" => timestamp_value(v.created_at.as_ref()),
                "supersedes_identity" => opt_record(
                    v.supersedes_identity
                        .clone()
                        .map(ProtocolRecord::IdentityRef),
                ),
                "assurance_claim_objects" => repeated(
                    v.assurance_claim_objects
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "metadata_object" => {
                    opt_record(v.metadata_object.clone().map(ProtocolRecord::ObjectRef))
                }
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::AssuranceClaim(v) => match name {
                "claim_version" => CValue::Int(v.claim_version),
                "subject_identity" => match &v.subject {
                    Some(AssuranceSubject::Identity(value)) => {
                        ProtocolRecord::IdentityRef(value.clone()).to_cvalue(false)
                    }
                    _ => CValue::Null,
                },
                "subject_node" => match &v.subject {
                    Some(AssuranceSubject::Node(value)) => {
                        ProtocolRecord::NodeRef(value.clone()).to_cvalue(false)
                    }
                    _ => CValue::Null,
                },
                "assurance_class" => opt_text(&v.assurance_class),
                "attester" => ProtocolRecord::IdentityRef(v.attester.clone()).to_cvalue(false),
                "issued_at" => timestamp_value(v.issued_at.as_ref()),
                "expires_at" => timestamp_value(v.expires_at.as_ref()),
                "evidence_object" => {
                    opt_record(v.evidence_object.clone().map(ProtocolRecord::ObjectRef))
                }
                "claim_note" => opt_text(&v.claim_note),
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::AssuranceRequirement(v) => match name {
                "assurance_version" => CValue::Int(v.assurance_version),
                "required_class" => opt_text(&v.required_class),
                "acceptable_attesters" => repeated(
                    v.acceptable_attesters
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::IdentityRef),
                ),
                "max_evidence_age" => duration_value(v.max_evidence_age.as_ref()),
                "assurance_metadata" => {
                    opt_record(v.assurance_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                _ => default_for_kind(kind),
            },
            Self::ConstraintSet(v) => match name {
                "constraint_version" => CValue::Int(v.constraint_version),
                "not_before" => timestamp_value(v.not_before.as_ref()),
                "expires_at" => timestamp_value(v.expires_at.as_ref()),
                "max_uses" => opt_int(&v.max_uses),
                "rate_limit" => opt_record(v.rate_limit.clone().map(ProtocolRecord::RateLimit)),
                "requires_local_session" => v
                    .requires_local_session
                    .map(CValue::Bool)
                    .unwrap_or(CValue::Null),
                "requires_user_presence" => v
                    .requires_user_presence
                    .map(CValue::Bool)
                    .unwrap_or(CValue::Null),
                "requires_transport_classes" => CValue::Array(
                    v.requires_transport_classes
                        .iter()
                        .cloned()
                        .map(CValue::Text)
                        .collect(),
                ),
                "requires_location_classes" => CValue::Array(
                    v.requires_location_classes
                        .iter()
                        .cloned()
                        .map(CValue::Text)
                        .collect(),
                ),
                "export_policy" => opt_text(&v.export_policy),
                "execution_class_limits" => CValue::Array(
                    v.execution_class_limits
                        .iter()
                        .cloned()
                        .map(CValue::Text)
                        .collect(),
                ),
                "storage_class_limits" => CValue::Array(
                    v.storage_class_limits
                        .iter()
                        .cloned()
                        .map(CValue::Text)
                        .collect(),
                ),
                "constraint_metadata" => {
                    opt_record(v.constraint_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                _ => default_for_kind(kind),
            },
            Self::CapabilityDescriptor(v) => match name {
                "capability_version" => CValue::Int(v.capability_version),
                "capability_kind" => opt_text(&v.capability_kind),
                "actions" => CValue::Array(v.actions.iter().cloned().map(CValue::Text).collect()),
                "scope" => opt_record(v.scope.clone().map(ProtocolRecord::ScopeDescriptor)),
                "constraints" => {
                    opt_record(v.constraints.clone().map(ProtocolRecord::ConstraintSet))
                }
                "delegation_policy" => opt_text(&v.delegation_policy),
                "minimum_assurance" => opt_record(
                    v.minimum_assurance
                        .clone()
                        .map(ProtocolRecord::AssuranceRequirement),
                ),
                "capability_metadata" => {
                    opt_record(v.capability_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                _ => default_for_kind(kind),
            },
            Self::RevocationRecord(v) => match name {
                "record_version" => CValue::Int(v.record_version),
                "revocation_id" => CValue::Bytes(v.revocation_id.clone()),
                "issuer" => ProtocolRecord::IdentityRef(v.issuer.clone()).to_cvalue(false),
                "issued_at" => timestamp_value(v.issued_at.as_ref()),
                "effective_at" => timestamp_value(v.effective_at.as_ref()),
                "revocation_kind" => opt_text(&v.revocation_kind),
                "target_delegation" => match &v.target {
                    Some(RevocationTarget::Delegation(value)) => {
                        ProtocolRecord::DelegationRef(value.clone()).to_cvalue(false)
                    }
                    _ => CValue::Null,
                },
                "target_identity" => match &v.target {
                    Some(RevocationTarget::Identity(value)) => {
                        ProtocolRecord::IdentityRef(value.clone()).to_cvalue(false)
                    }
                    _ => CValue::Null,
                },
                "target_node" => match &v.target {
                    Some(RevocationTarget::Node(value)) => {
                        ProtocolRecord::NodeRef(value.clone()).to_cvalue(false)
                    }
                    _ => CValue::Null,
                },
                "target_object" => match &v.target {
                    Some(RevocationTarget::Object(value)) => {
                        ProtocolRecord::ObjectRef(value.clone()).to_cvalue(false)
                    }
                    _ => CValue::Null,
                },
                "scope_override" => opt_record(
                    v.scope_override
                        .clone()
                        .map(ProtocolRecord::ScopeDescriptor),
                ),
                "reason_code" => opt_text(&v.reason_code),
                "replacement_id" => opt_bytes(&v.replacement_id),
                "revocation_metadata" => {
                    opt_record(v.revocation_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                "signature" => opt_signature(&v.signature, signable),
                _ => default_for_kind(kind),
            },
            Self::LogicalObjectDescriptor(v) => match name {
                "descriptor_version" => CValue::Int(v.descriptor_version),
                "object_id" => CValue::Bytes(v.object_id.clone()),
                "object_kind" => opt_text(&v.object_kind),
                "object_schema_version" => CValue::Int(v.object_schema_version),
                "canonicalization_id" => opt_text(&v.canonicalization_id),
                "canonical_digest" => opt_digest(&v.canonical_digest),
                "canonical_size" => CValue::Int(v.canonical_size),
                "created_at" => timestamp_value(v.created_at.as_ref()),
                "producer" => opt_record(v.producer.clone().map(ProtocolRecord::IdentityRef)),
                "describes_object" => {
                    opt_record(v.describes_object.clone().map(ProtocolRecord::ObjectRef))
                }
                "object_metadata" => {
                    opt_record(v.object_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                _ => default_for_kind(kind),
            },
            Self::StoredRepresentationHeader(v) => match name {
                "header_version" => CValue::Int(v.header_version),
                "representation_id" => CValue::Bytes(v.representation_id.clone()),
                "object" => opt_record(v.object.clone().map(ProtocolRecord::ObjectRef)),
                "representation_digest" => opt_digest(&v.representation_digest),
                "plaintext_size" => opt_int(&v.plaintext_size),
                "stored_size" => CValue::Int(v.stored_size),
                "encryption_scheme" => opt_text(&v.encryption_scheme),
                "compression_scheme" => opt_text(&v.compression_scheme),
                "chunking_mode" => opt_text(&v.chunking_mode),
                "chunk_manifest_object" => opt_record(
                    v.chunk_manifest_object
                        .clone()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "access_package_object" => opt_record(
                    v.access_package_object
                        .clone()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "created_at" => timestamp_value(v.created_at.as_ref()),
                "representation_metadata" => opt_record(
                    v.representation_metadata
                        .clone()
                        .map(ProtocolRecord::ObjectRef),
                ),
                _ => default_for_kind(kind),
            },
            Self::ChunkEntry(v) => match name {
                "index" => CValue::Int(v.index),
                "chunk_representation_id" => CValue::Bytes(v.chunk_representation_id.clone()),
                "chunk_digest" => opt_digest(&v.chunk_digest),
                "offset" => CValue::Int(v.offset),
                "length" => CValue::Int(v.length),
                _ => default_for_kind(kind),
            },
            Self::ChunkManifest(v) => match name {
                "manifest_version" => CValue::Int(v.manifest_version),
                "object" => opt_record(v.object.clone().map(ProtocolRecord::ObjectRef)),
                "representation" => opt_record(
                    v.representation
                        .clone()
                        .map(ProtocolRecord::RepresentationRef),
                ),
                "chunk_count" => CValue::Int(v.chunk_count),
                "total_stored_size" => CValue::Int(v.total_stored_size),
                "chunk_entries" => repeated(
                    v.chunk_entries
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::ChunkEntry),
                ),
                "manifest_metadata" => {
                    opt_record(v.manifest_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                _ => default_for_kind(kind),
            },
            Self::AggregateTrustPolicy(v) => match name {
                "policy_version" => CValue::Int(v.policy_version),
                "minimum_trust_score" => opt_int(&v.minimum_trust_score),
                "preferred_aggregators" => repeated(
                    v.preferred_aggregators
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::IdentityRef),
                ),
                "allowed_responders" => repeated(
                    v.allowed_responders
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::IdentityRef),
                ),
                "policy_metadata" => {
                    opt_record(v.policy_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                _ => default_for_kind(kind),
            },
            Self::RouteTrustAssignment(v) => match name {
                "subject" => ProtocolRecord::IdentityRef(v.subject.clone()).to_cvalue(false),
                "trust_score" => CValue::Int(v.trust_score),
                "source" => opt_text(&v.source),
                _ => default_for_kind(kind),
            },
            Self::RouteTrustAssignments(v) => match name {
                "assignments" => repeated(
                    v.assignments
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::RouteTrustAssignment),
                ),
                _ => default_for_kind(kind),
            },
            Self::RouteSelectionPolicy(v) => match name {
                "policy_version" => CValue::Int(v.policy_version),
                "minimum_quality_hint" => opt_int(&v.minimum_quality_hint),
                "maximum_cost_hint" => opt_int(&v.maximum_cost_hint),
                "preferred_advertisers" => repeated(
                    v.preferred_advertisers
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::IdentityRef),
                ),
                "preferred_next_hops" => repeated(
                    v.preferred_next_hops
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::NodeRef),
                ),
                "require_active_session" => v
                    .require_active_session
                    .map(CValue::Bool)
                    .unwrap_or(CValue::Null),
                "policy_metadata" => {
                    opt_record(v.policy_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                _ => default_for_kind(kind),
            },
            Self::NodeGenesisPayload(v) => match name {
                "payload_version" => CValue::Int(v.payload_version),
                "node_id" => CValue::Bytes(v.node_id.clone()),
                "primary_node_identity" => opt_record(
                    v.primary_node_identity
                        .clone()
                        .map(ProtocolRecord::IdentityRef),
                ),
                "initial_controllers" => repeated(
                    v.initial_controllers
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::IdentityRef),
                ),
                "initial_policy_object" => opt_record(
                    v.initial_policy_object
                        .clone()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "bootstrap_records" => repeated(
                    v.bootstrap_records
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "assurance_claims" => repeated(
                    v.assurance_claims
                        .iter()
                        .cloned()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "node_roles" => {
                    CValue::Array(v.node_roles.iter().cloned().map(CValue::Text).collect())
                }
                "genesis_metadata" => {
                    opt_record(v.genesis_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                _ => default_for_kind(kind),
            },
            Self::CommandSentPayload(v) => match name {
                "payload_version" => CValue::Int(v.payload_version),
                "command" => opt_record(v.command.clone().map(ProtocolRecord::CommandRef)),
                "target_node" => opt_record(v.target_node.clone().map(ProtocolRecord::NodeRef)),
                "send_metadata" => {
                    opt_record(v.send_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                _ => default_for_kind(kind),
            },
            Self::CommandResultPayload(v) => match name {
                "payload_version" => CValue::Int(v.payload_version),
                "command" => opt_record(v.command.clone().map(ProtocolRecord::CommandRef)),
                "issuer" => opt_record(v.issuer.clone().map(ProtocolRecord::IdentityRef)),
                "decision" => opt_text(&v.decision),
                "decision_basis" => {
                    opt_record(v.decision_basis.clone().map(ProtocolRecord::ObjectRef))
                }
                "reason_code" => opt_text(&v.reason_code),
                "effect_summary_object" => opt_record(
                    v.effect_summary_object
                        .clone()
                        .map(ProtocolRecord::ObjectRef),
                ),
                "result_object" => {
                    opt_record(v.result_object.clone().map(ProtocolRecord::ObjectRef))
                }
                _ => default_for_kind(kind),
            },
            Self::ActionLifecyclePayload(v) => match name {
                "payload_version" => CValue::Int(v.payload_version),
                "origin_command" => {
                    opt_record(v.origin_command.clone().map(ProtocolRecord::CommandRef))
                }
                "action_instance_id" => CValue::Bytes(v.action_instance_id.clone()),
                "status" => opt_text(&v.status),
                "result_object" => {
                    opt_record(v.result_object.clone().map(ProtocolRecord::ObjectRef))
                }
                "error_object" => opt_record(v.error_object.clone().map(ProtocolRecord::ObjectRef)),
                "progress_object" => {
                    opt_record(v.progress_object.clone().map(ProtocolRecord::ObjectRef))
                }
                "action_metadata" => {
                    opt_record(v.action_metadata.clone().map(ProtocolRecord::ObjectRef))
                }
                _ => default_for_kind(kind),
            },
        }
    }
}

fn timestamp_value(value: Option<&Timestamp>) -> CValue {
    value
        .map(|v| CValue::Array(vec![CValue::Int(v.seconds), CValue::Int(v.nanos)]))
        .unwrap_or(CValue::Null)
}

fn duration_value(value: Option<&Duration>) -> CValue {
    value
        .map(|v| CValue::Array(vec![CValue::Int(v.seconds), CValue::Int(v.nanos)]))
        .unwrap_or(CValue::Null)
}

fn opt_text(value: &Option<String>) -> CValue {
    value
        .as_ref()
        .cloned()
        .map(CValue::Text)
        .unwrap_or(CValue::Null)
}

fn opt_bytes(value: &Option<Vec<u8>>) -> CValue {
    value
        .as_ref()
        .cloned()
        .map(CValue::Bytes)
        .unwrap_or(CValue::Null)
}

fn opt_int(value: &Option<i64>) -> CValue {
    value.map(CValue::Int).unwrap_or(CValue::Null)
}

fn opt_record(value: Option<ProtocolRecord>) -> CValue {
    value.map(|v| v.to_cvalue(false)).unwrap_or(CValue::Null)
}

fn opt_digest(value: &Option<Digest>) -> CValue {
    value
        .as_ref()
        .map(|d| ProtocolRecord::Digest(d.clone()).to_cvalue(false))
        .unwrap_or(CValue::Null)
}

fn opt_signature(value: &Option<Signature>, signable: bool) -> CValue {
    value
        .as_ref()
        .map(|s| ProtocolRecord::Signature(s.clone()).to_cvalue(signable))
        .unwrap_or(CValue::Null)
}

fn repeated(values: impl Iterator<Item = ProtocolRecord>) -> CValue {
    CValue::Array(values.map(|v| v.to_cvalue(false)).collect())
}

fn default_for_kind(kind: FieldKind) -> CValue {
    match kind {
        FieldKind::RepeatedScalar | FieldKind::RepeatedMessage => CValue::Array(vec![]),
        FieldKind::Message | FieldKind::Scalar | FieldKind::Bytes | FieldKind::Timestamp => {
            CValue::Null
        }
    }
}
