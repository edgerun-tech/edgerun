use lifegraph_core::protocol::{
    ActionLifecyclePayload, AggregateSummaryProof, AggregateTrustPolicy, AssuranceClaim,
    AssuranceRequirement, AssuranceSubject, Capability, CapabilityDescriptor, CheckpointRef,
    ChunkEntry, ChunkManifest, CommandEnvelope, CommandRef, CommandResultPayload,
    CommandSentPayload, ConstraintSet, CostLimit, DelegationRecord, DelegationRef, Digest,
    Duration, EventEnvelope, EventRef, EventSetProof, FederatedAggregateDescriptor, HeadRef,
    IdentityRecord, IdentityRef, LogicalObjectDescriptor, NodeGenesisPayload, NodeRef,
    ObjectAssertionProof, ObjectRef, ProofBundle, ProtocolRecord, QueryRequest,
    QueryResultFragment, RateLimit, ReachabilityHint, RelayEnvelope, RepresentationRef,
    ResultFragmentProof, RevocationRecord, RevocationRef, RevocationTarget, RouteAdvertisement,
    RouteSelectionPolicy, RouteTrustAssignment, RouteTrustAssignments, ScopeDescriptor,
    SessionAccept, SessionHello, Signature, SnapshotDescriptor, SnapshotRef, SnapshotSetProof,
    StoredRepresentationHeader, StreamHeadsProof, StreamRef, TimeWindow, Timestamp,
    TrustPolicyProof,
};

use crate::lifegraph::v0::{access, common, identity, network, object, stream, trust};

#[derive(Debug)]
pub struct ConvertError(pub String);

impl std::fmt::Display for ConvertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ConvertError {}

pub trait IntoCore<T> {
    fn into_core(&self) -> Result<T, ConvertError>;
}

fn timestamp_from_proto(ts: &prost_types::Timestamp) -> Timestamp {
    Timestamp {
        seconds: ts.seconds,
        nanos: i64::from(ts.nanos),
    }
}

fn timestamp_to_proto(ts: &Timestamp) -> prost_types::Timestamp {
    prost_types::Timestamp {
        seconds: ts.seconds,
        nanos: ts.nanos as i32,
    }
}

fn duration_from_proto(value: &prost_types::Duration) -> Duration {
    Duration {
        seconds: value.seconds,
        nanos: i64::from(value.nanos),
    }
}

fn duration_to_proto(value: &Duration) -> prost_types::Duration {
    prost_types::Duration {
        seconds: value.seconds,
        nanos: value.nanos as i32,
    }
}

fn optional_timestamp_from_proto(ts: &Option<prost_types::Timestamp>) -> Option<Timestamp> {
    ts.as_ref().map(timestamp_from_proto)
}

fn optional_timestamp_to_proto(ts: &Option<Timestamp>) -> Option<prost_types::Timestamp> {
    ts.as_ref().map(timestamp_to_proto)
}

fn optional_duration_from_proto(value: &Option<prost_types::Duration>) -> Option<Duration> {
    value.as_ref().map(duration_from_proto)
}

fn optional_duration_to_proto(value: &Option<Duration>) -> Option<prost_types::Duration> {
    value.as_ref().map(duration_to_proto)
}

macro_rules! enum_name_from_proto {
    ($ty:ty, $value:expr, $trim_prefix:expr) => {
        <$ty>::try_from($value).ok().map(|v| {
            v.as_str_name()
                .trim_start_matches($trim_prefix)
                .to_ascii_lowercase()
        })
    };
}

macro_rules! enum_name_to_proto {
    ($ty:ty, $value:expr, $prefix:expr) => {{
        let value = $value;
        let full = if value.starts_with($prefix) {
            value.to_string()
        } else {
            format!(
                "{}{}",
                $prefix,
                value.to_ascii_uppercase().replace('-', "_")
            )
        };
        <$ty>::from_str_name(&full).map(|v| v as i32)
    }};
}

fn identity_kind_from_proto(kind: Option<i32>) -> Option<String> {
    kind.and_then(|v| enum_name_from_proto!(common::IdentityKind, v, "IDENTITY_KIND_"))
}

fn identity_kind_to_proto(kind: &Option<String>) -> Option<i32> {
    kind.as_deref()
        .and_then(|v| enum_name_to_proto!(common::IdentityKind, v, "IDENTITY_KIND_"))
}

fn signature_algorithm_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(common::signature::Algorithm, value, "SIGNATURE_ALGORITHM_")
}

fn signature_algorithm_to_proto(value: &str) -> Option<i32> {
    enum_name_to_proto!(common::signature::Algorithm, value, "SIGNATURE_ALGORITHM_")
}

fn digest_algorithm_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(common::digest::Algorithm, value, "DIGEST_ALGORITHM_")
}

fn digest_algorithm_to_proto(value: &str) -> Option<i32> {
    enum_name_to_proto!(common::digest::Algorithm, value, "DIGEST_ALGORITHM_")
}

fn object_kind_from_proto(value: Option<i32>) -> Option<String> {
    value.and_then(|v| enum_name_from_proto!(common::ObjectKind, v, "OBJECT_KIND_"))
}

fn object_kind_to_proto(value: &Option<String>) -> Option<i32> {
    value
        .as_deref()
        .and_then(|v| enum_name_to_proto!(common::ObjectKind, v, "OBJECT_KIND_"))
}

fn query_class_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(access::QueryClass, value, "QUERY_CLASS_")
}

fn query_class_to_proto(value: &Option<String>) -> Option<i32> {
    value
        .as_deref()
        .and_then(|v| enum_name_to_proto!(access::QueryClass, v, "QUERY_CLASS_"))
}

fn proof_class_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(access::ProofClass, value, "PROOF_CLASS_")
}

fn proof_class_to_proto(value: &str) -> Option<i32> {
    enum_name_to_proto!(access::ProofClass, value, "PROOF_CLASS_")
}

fn result_completeness_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(access::ResultCompleteness, value, "RESULT_COMPLETENESS_")
}

fn result_completeness_to_proto(value: &Option<String>) -> Option<i32> {
    value
        .as_deref()
        .and_then(|v| enum_name_to_proto!(access::ResultCompleteness, v, "RESULT_COMPLETENESS_"))
}

fn proof_payload_type_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(access::ProofPayloadType, value, "PROOF_PAYLOAD_TYPE_")
}

fn proof_payload_type_to_proto(value: &Option<String>) -> Option<i32> {
    value
        .as_deref()
        .and_then(|v| enum_name_to_proto!(access::ProofPayloadType, v, "PROOF_PAYLOAD_TYPE_"))
}

fn snapshot_completeness_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(
        access::SnapshotCompleteness,
        value,
        "SNAPSHOT_COMPLETENESS_"
    )
}

fn snapshot_completeness_to_proto(value: &Option<String>) -> Option<i32> {
    value.as_deref().and_then(|v| {
        enum_name_to_proto!(access::SnapshotCompleteness, v, "SNAPSHOT_COMPLETENESS_")
    })
}

fn transport_class_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(common::TransportClass, value, "TRANSPORT_CLASS_")
}

fn transport_class_to_proto(value: &Option<String>) -> Option<i32> {
    value
        .as_deref()
        .and_then(|v| enum_name_to_proto!(common::TransportClass, v, "TRANSPORT_CLASS_"))
}

fn directness_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(common::Directness, value, "DIRECTNESS_")
}

fn directness_to_proto(value: &Option<String>) -> Option<i32> {
    value
        .as_deref()
        .and_then(|v| enum_name_to_proto!(common::Directness, v, "DIRECTNESS_"))
}

fn payload_kind_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(network::PayloadKind, value, "PAYLOAD_KIND_")
}

fn payload_kind_to_proto(value: &Option<String>) -> Option<i32> {
    value
        .as_deref()
        .and_then(|v| enum_name_to_proto!(network::PayloadKind, v, "PAYLOAD_KIND_"))
}

fn assurance_class_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(common::AssuranceClass, value, "ASSURANCE_CLASS_")
}

fn assurance_class_to_proto(value: &Option<String>) -> Option<i32> {
    value
        .as_deref()
        .and_then(|v| enum_name_to_proto!(common::AssuranceClass, v, "ASSURANCE_CLASS_"))
}

fn export_policy_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(trust::ExportPolicy, value, "EXPORT_POLICY_")
}

fn export_policy_to_proto(value: &Option<String>) -> Option<i32> {
    value
        .as_deref()
        .and_then(|v| enum_name_to_proto!(trust::ExportPolicy, v, "EXPORT_POLICY_"))
}

fn delegation_policy_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(trust::DelegationPolicy, value, "DELEGATION_POLICY_")
}

fn delegation_policy_to_proto(value: &Option<String>) -> Option<i32> {
    value
        .as_deref()
        .and_then(|v| enum_name_to_proto!(trust::DelegationPolicy, v, "DELEGATION_POLICY_"))
}

fn revocation_kind_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(trust::RevocationKind, value, "REVOCATION_KIND_")
}

fn revocation_kind_to_proto(value: &Option<String>) -> Option<i32> {
    value
        .as_deref()
        .and_then(|v| enum_name_to_proto!(trust::RevocationKind, v, "REVOCATION_KIND_"))
}

fn key_algorithm_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(identity::KeyAlgorithm, value, "KEY_ALGORITHM_")
}

fn key_algorithm_to_proto(value: &Option<String>) -> Option<i32> {
    value
        .as_deref()
        .and_then(|v| enum_name_to_proto!(identity::KeyAlgorithm, v, "KEY_ALGORITHM_"))
}

fn chunking_mode_from_proto(value: i32) -> Option<String> {
    enum_name_from_proto!(object::ChunkingMode, value, "CHUNKING_MODE_")
}

fn chunking_mode_to_proto(value: &Option<String>) -> Option<i32> {
    value
        .as_deref()
        .and_then(|v| enum_name_to_proto!(object::ChunkingMode, v, "CHUNKING_MODE_"))
}

fn scope_from_proto(scope: &trust::ScopeDescriptor) -> ScopeDescriptor {
    let scope_kind = trust::ScopeKind::try_from(scope.scope_kind)
        .ok()
        .map(|v| v.as_str_name().to_string())
        .unwrap_or_else(|| "SCOPE_KIND_UNSPECIFIED".to_string());
    let mut segments = Vec::new();
    if !scope.target_domains.is_empty() {
        segments.extend(scope.target_domains.clone());
    } else if !scope.target_view_types.is_empty() {
        segments.extend(scope.target_view_types.clone());
    }
    ScopeDescriptor {
        scope_version: i64::from(scope.scope_version),
        scope_kind,
        target_nodes: scope
            .target_nodes
            .iter()
            .map(IntoCore::into_core)
            .collect::<Result<_, _>>()
            .unwrap_or_default(),
        target_streams: scope
            .target_streams
            .iter()
            .map(IntoCore::into_core)
            .collect::<Result<_, _>>()
            .unwrap_or_default(),
        target_object_kinds: scope
            .target_object_kinds
            .iter()
            .filter_map(|v| object_kind_from_proto(Some(*v)))
            .collect(),
        target_view_types: scope.target_view_types.clone(),
        target_domains: scope.target_domains.clone(),
        time_bounds: scope
            .time_bounds
            .as_ref()
            .map(IntoCore::into_core)
            .transpose()
            .ok()
            .flatten(),
        scope_metadata: scope
            .scope_metadata
            .as_ref()
            .map(IntoCore::into_core)
            .transpose()
            .ok()
            .flatten(),
        segments,
    }
}

fn scope_to_proto(scope: &ScopeDescriptor) -> trust::ScopeDescriptor {
    let kind = trust::ScopeKind::from_str_name(&scope.scope_kind)
        .unwrap_or(trust::ScopeKind::Unspecified) as i32;
    trust::ScopeDescriptor {
        scope_version: scope.scope_version as u32,
        scope_kind: kind,
        target_nodes: scope.target_nodes.iter().map(Into::into).collect(),
        target_streams: scope.target_streams.iter().map(Into::into).collect(),
        target_object_kinds: scope
            .target_object_kinds
            .iter()
            .filter_map(|v| object_kind_to_proto(&Some(v.clone())))
            .collect(),
        target_view_types: if !scope.target_view_types.is_empty() {
            scope.target_view_types.clone()
        } else if scope.scope_kind == "SCOPE_KIND_VIEW" {
            scope.segments.clone()
        } else {
            vec![]
        },
        target_domains: if !scope.target_domains.is_empty() {
            scope.target_domains.clone()
        } else if scope.scope_kind == "SCOPE_KIND_DOMAIN"
            || scope.scope_kind == "SCOPE_KIND_QUERY_CLASS"
            || scope.scope_kind == "SCOPE_KIND_GLOBAL_WITH_CONSTRAINTS"
        {
            scope.segments.clone()
        } else {
            vec![]
        },
        time_bounds: scope.time_bounds.as_ref().map(Into::into),
        scope_metadata: scope.scope_metadata.as_ref().map(Into::into),
    }
}

impl IntoCore<IdentityRef> for common::IdentityRef {
    fn into_core(&self) -> Result<IdentityRef, ConvertError> {
        Ok(IdentityRef {
            identity_id: self.identity_id.clone(),
            identity_kind: identity_kind_from_proto(self.identity_kind),
            key_hint: self.key_hint.clone(),
        })
    }
}

impl From<&IdentityRef> for common::IdentityRef {
    fn from(value: &IdentityRef) -> Self {
        Self {
            identity_id: value.identity_id.clone(),
            identity_kind: identity_kind_to_proto(&value.identity_kind),
            key_hint: value.key_hint.clone(),
        }
    }
}

impl IntoCore<NodeRef> for common::NodeRef {
    fn into_core(&self) -> Result<NodeRef, ConvertError> {
        Ok(NodeRef {
            node_id: self.node_id.clone(),
        })
    }
}

impl From<&NodeRef> for common::NodeRef {
    fn from(value: &NodeRef) -> Self {
        Self {
            node_id: value.node_id.clone(),
        }
    }
}

impl IntoCore<StreamRef> for common::StreamRef {
    fn into_core(&self) -> Result<StreamRef, ConvertError> {
        Ok(StreamRef {
            stream_id: self.stream_id.clone(),
        })
    }
}

impl From<&StreamRef> for common::StreamRef {
    fn from(value: &StreamRef) -> Self {
        Self {
            stream_id: value.stream_id.clone(),
        }
    }
}

impl IntoCore<Digest> for common::Digest {
    fn into_core(&self) -> Result<Digest, ConvertError> {
        Ok(Digest {
            algorithm: digest_algorithm_from_proto(self.algorithm)
                .ok_or_else(|| ConvertError("unsupported digest algorithm".into()))?,
            value: self.value.clone(),
        })
    }
}

impl From<&Digest> for common::Digest {
    fn from(value: &Digest) -> Self {
        Self {
            algorithm: digest_algorithm_to_proto(&value.algorithm)
                .unwrap_or(common::digest::Algorithm::DigestAlgorithmUnspecified as i32),
            value: value.value.clone(),
        }
    }
}

impl IntoCore<Signature> for common::Signature {
    fn into_core(&self) -> Result<Signature, ConvertError> {
        Ok(Signature {
            algorithm: signature_algorithm_from_proto(self.algorithm)
                .ok_or_else(|| ConvertError("unsupported signature algorithm".into()))?,
            value: self.value.clone(),
        })
    }
}

impl From<&Signature> for common::Signature {
    fn from(value: &Signature) -> Self {
        Self {
            algorithm: signature_algorithm_to_proto(&value.algorithm)
                .unwrap_or(common::signature::Algorithm::SignatureAlgorithmUnspecified as i32),
            value: value.value.clone(),
        }
    }
}

impl IntoCore<EventRef> for common::EventRef {
    fn into_core(&self) -> Result<EventRef, ConvertError> {
        Ok(EventRef {
            stream_id: self.stream_id.clone(),
            seq: self.seq as i64,
            event_hash: self
                .event_hash
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&EventRef> for common::EventRef {
    fn from(value: &EventRef) -> Self {
        Self {
            stream_id: value.stream_id.clone(),
            seq: value.seq as u64,
            event_hash: value.event_hash.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<HeadRef> for common::HeadRef {
    fn into_core(&self) -> Result<HeadRef, ConvertError> {
        Ok(HeadRef {
            stream_id: self.stream_id.clone(),
            seq: self.seq as i64,
            event_hash: self
                .event_hash
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&HeadRef> for common::HeadRef {
    fn from(value: &HeadRef) -> Self {
        Self {
            stream_id: value.stream_id.clone(),
            seq: value.seq as u64,
            event_hash: value.event_hash.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<CheckpointRef> for common::CheckpointRef {
    fn into_core(&self) -> Result<CheckpointRef, ConvertError> {
        Ok(CheckpointRef {
            checkpoint_id: self.checkpoint_id.clone(),
            heads: self
                .heads
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<&CheckpointRef> for common::CheckpointRef {
    fn from(value: &CheckpointRef) -> Self {
        Self {
            checkpoint_id: value.checkpoint_id.clone(),
            heads: value.heads.iter().map(Into::into).collect(),
        }
    }
}

impl IntoCore<ObjectRef> for common::ObjectRef {
    fn into_core(&self) -> Result<ObjectRef, ConvertError> {
        Ok(ObjectRef {
            object_id: self.object_id.clone(),
            object_kind: object_kind_from_proto(self.object_kind),
        })
    }
}

impl From<&ObjectRef> for common::ObjectRef {
    fn from(value: &ObjectRef) -> Self {
        Self {
            object_id: value.object_id.clone(),
            object_kind: object_kind_to_proto(&value.object_kind),
        }
    }
}

impl IntoCore<RepresentationRef> for common::RepresentationRef {
    fn into_core(&self) -> Result<RepresentationRef, ConvertError> {
        Ok(RepresentationRef {
            representation_id: self.representation_id.clone(),
            object_id: self.object_id.clone(),
        })
    }
}

impl From<&RepresentationRef> for common::RepresentationRef {
    fn from(value: &RepresentationRef) -> Self {
        Self {
            representation_id: value.representation_id.clone(),
            object_id: value.object_id.clone(),
        }
    }
}

impl IntoCore<CommandRef> for common::CommandRef {
    fn into_core(&self) -> Result<CommandRef, ConvertError> {
        Ok(CommandRef {
            command_id: self.command_id.clone(),
            command_hash: self
                .command_hash
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&CommandRef> for common::CommandRef {
    fn from(value: &CommandRef) -> Self {
        Self {
            command_id: value.command_id.clone(),
            command_hash: value.command_hash.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<DelegationRef> for common::DelegationRef {
    fn into_core(&self) -> Result<DelegationRef, ConvertError> {
        Ok(DelegationRef {
            delegation_id: self.delegation_id.clone(),
            delegation_hash: self
                .delegation_hash
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&DelegationRef> for common::DelegationRef {
    fn from(value: &DelegationRef) -> Self {
        Self {
            delegation_id: value.delegation_id.clone(),
            delegation_hash: value.delegation_hash.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<RevocationRef> for common::RevocationRef {
    fn into_core(&self) -> Result<RevocationRef, ConvertError> {
        Ok(RevocationRef {
            revocation_id: self.revocation_id.clone(),
            revocation_hash: self
                .revocation_hash
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&RevocationRef> for common::RevocationRef {
    fn from(value: &RevocationRef) -> Self {
        Self {
            revocation_id: value.revocation_id.clone(),
            revocation_hash: value.revocation_hash.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<SnapshotRef> for common::SnapshotRef {
    fn into_core(&self) -> Result<SnapshotRef, ConvertError> {
        Ok(SnapshotRef {
            snapshot_id: self.snapshot_id.clone(),
            object_id: self.object_id.clone(),
        })
    }
}

impl From<&SnapshotRef> for common::SnapshotRef {
    fn from(value: &SnapshotRef) -> Self {
        Self {
            snapshot_id: value.snapshot_id.clone(),
            object_id: value.object_id.clone(),
        }
    }
}

impl IntoCore<DelegationRecord> for trust::DelegationRecord {
    fn into_core(&self) -> Result<DelegationRecord, ConvertError> {
        let capability = self
            .capability
            .as_ref()
            .ok_or_else(|| ConvertError("missing capability".into()))?;
        Ok(DelegationRecord {
            record_version: i64::from(self.record_version),
            delegation_id: self.delegation_id.clone(),
            issuer: self
                .issuer
                .as_ref()
                .ok_or_else(|| ConvertError("missing issuer".into()))?
                .into_core()?,
            recipient: self
                .recipient
                .as_ref()
                .ok_or_else(|| ConvertError("missing recipient".into()))?
                .into_core()?,
            capability: Capability {
                capability_version: i64::from(capability.capability_version),
                kind: trust::CapabilityKind::try_from(capability.capability_kind)
                    .ok()
                    .map(|v| v.as_str_name().to_string())
                    .unwrap_or_else(|| "CAPABILITY_KIND_UNSPECIFIED".to_string()),
                actions: capability.actions.clone(),
                scope: capability.scope.as_ref().map(scope_from_proto),
                constraints: capability
                    .constraints
                    .as_ref()
                    .map(IntoCore::into_core)
                    .transpose()?,
                delegation_policy: delegation_policy_from_proto(capability.delegation_policy),
                minimum_assurance: capability
                    .minimum_assurance
                    .as_ref()
                    .map(IntoCore::into_core)
                    .transpose()?,
                capability_metadata: capability
                    .capability_metadata
                    .as_ref()
                    .map(IntoCore::into_core)
                    .transpose()?,
            },
            issued_at: optional_timestamp_from_proto(&self.issued_at),
            not_before: optional_timestamp_from_proto(&self.not_before),
            expires_at: optional_timestamp_from_proto(&self.expires_at),
            parent_delegation: self
                .parent_delegation
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            revocation_authorities: self
                .revocation_authorities
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            delegation_metadata: self
                .delegation_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&DelegationRecord> for trust::DelegationRecord {
    fn from(value: &DelegationRecord) -> Self {
        let capability_kind = trust::CapabilityKind::from_str_name(&value.capability.kind)
            .unwrap_or(trust::CapabilityKind::Unspecified) as i32;
        Self {
            record_version: value.record_version as u32,
            delegation_id: value.delegation_id.clone(),
            issuer: Some((&value.issuer).into()),
            recipient: Some((&value.recipient).into()),
            issued_at: optional_timestamp_to_proto(&value.issued_at),
            not_before: optional_timestamp_to_proto(&value.not_before),
            expires_at: optional_timestamp_to_proto(&value.expires_at),
            capability: Some(trust::CapabilityDescriptor {
                capability_version: value.capability.capability_version as u32,
                capability_kind,
                actions: value.capability.actions.clone(),
                scope: value.capability.scope.as_ref().map(scope_to_proto),
                constraints: value.capability.constraints.as_ref().map(Into::into),
                delegation_policy: delegation_policy_to_proto(&value.capability.delegation_policy)
                    .unwrap_or(trust::DelegationPolicy::Unspecified as i32),
                minimum_assurance: value.capability.minimum_assurance.as_ref().map(Into::into),
                capability_metadata: value
                    .capability
                    .capability_metadata
                    .as_ref()
                    .map(Into::into),
            }),
            parent_delegation: value.parent_delegation.as_ref().map(Into::into),
            revocation_authorities: value
                .revocation_authorities
                .iter()
                .map(Into::into)
                .collect(),
            delegation_metadata: value.delegation_metadata.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<EventEnvelope> for stream::EventEnvelope {
    fn into_core(&self) -> Result<EventEnvelope, ConvertError> {
        Ok(EventEnvelope {
            envelope_version: i64::from(self.envelope_version),
            stream_id: self.stream_id.clone(),
            seq: self.seq as i64,
            prev_event_hash: self
                .prev_event_hash
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            event_type: stream::EventType::try_from(self.event_type)
                .ok()
                .map(|v| v.as_str_name().to_string()),
            event_version: i64::from(self.event_version),
            recorded_at: optional_timestamp_from_proto(&self.recorded_at),
            effective_at: optional_timestamp_from_proto(&self.effective_at),
            payload_object: self
                .payload_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            related_events: self
                .related_events
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            related_commands: self
                .related_commands
                .iter()
                .map(|v| hex::encode(&v.command_id))
                .collect(),
            related_objects: self
                .related_objects
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            related_delegations: self
                .related_delegations
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            related_revocations: self
                .related_revocations
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            event_metadata: self
                .event_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&EventEnvelope> for stream::EventEnvelope {
    fn from(value: &EventEnvelope) -> Self {
        Self {
            envelope_version: value.envelope_version as u32,
            stream_id: value.stream_id.clone(),
            seq: value.seq as u64,
            prev_event_hash: value.prev_event_hash.as_ref().map(Into::into),
            event_type: value
                .event_type
                .as_deref()
                .and_then(stream::EventType::from_str_name)
                .unwrap_or(stream::EventType::Unspecified) as i32,
            event_version: value.event_version as u32,
            recorded_at: optional_timestamp_to_proto(&value.recorded_at),
            effective_at: optional_timestamp_to_proto(&value.effective_at),
            payload_object: value.payload_object.as_ref().map(Into::into),
            related_events: value.related_events.iter().map(Into::into).collect(),
            related_commands: value
                .related_commands
                .iter()
                .map(|id| common::CommandRef {
                    command_id: hex::decode(id).unwrap_or_else(|_| id.as_bytes().to_vec()),
                    command_hash: None,
                })
                .collect(),
            related_objects: value.related_objects.iter().map(Into::into).collect(),
            related_delegations: value.related_delegations.iter().map(Into::into).collect(),
            related_revocations: value.related_revocations.iter().map(Into::into).collect(),
            event_metadata: value.event_metadata.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<CommandEnvelope> for stream::CommandEnvelope {
    fn into_core(&self) -> Result<CommandEnvelope, ConvertError> {
        Ok(CommandEnvelope {
            envelope_version: i64::from(self.envelope_version),
            command_id: Some(self.command_id.clone()),
            target_node: self
                .target_node
                .as_ref()
                .ok_or_else(|| ConvertError("missing target_node".into()))?
                .into_core()?,
            issuer: self
                .issuer
                .as_ref()
                .ok_or_else(|| ConvertError("missing issuer".into()))?
                .into_core()?,
            command_type: stream::CommandType::try_from(self.command_type)
                .ok()
                .map(|v| v.as_str_name().to_string()),
            command_version: i64::from(self.command_version),
            issued_at: optional_timestamp_from_proto(&self.issued_at),
            not_before: optional_timestamp_from_proto(&self.not_before),
            expires_at: optional_timestamp_from_proto(&self.expires_at),
            idempotency_key: if self.idempotency_key.is_empty() {
                None
            } else {
                Some(self.idempotency_key.clone())
            },
            payload_object: match &self.payload {
                Some(stream::command_envelope::Payload::PayloadObject(v)) => Some(v.into_core()?),
                _ => None,
            },
            inline_payload: match &self.payload {
                Some(stream::command_envelope::Payload::InlinePayload(v)) => Some(v.clone()),
                _ => None,
            },
            delegation_chain: self
                .delegation_chain
                .iter()
                .map(|v| hex::encode(&v.delegation_id))
                .collect(),
            requested_assurance: self
                .requested_assurance
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            command_metadata: self
                .command_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&CommandEnvelope> for stream::CommandEnvelope {
    fn from(value: &CommandEnvelope) -> Self {
        Self {
            envelope_version: value.envelope_version as u32,
            command_id: value.command_id.clone().unwrap_or_default(),
            target_node: Some((&value.target_node).into()),
            issuer: Some((&value.issuer).into()),
            command_type: value
                .command_type
                .as_deref()
                .and_then(stream::CommandType::from_str_name)
                .unwrap_or(stream::CommandType::Unspecified) as i32,
            command_version: value.command_version as u32,
            issued_at: optional_timestamp_to_proto(&value.issued_at),
            not_before: optional_timestamp_to_proto(&value.not_before),
            expires_at: optional_timestamp_to_proto(&value.expires_at),
            idempotency_key: value.idempotency_key.clone().unwrap_or_default(),
            payload: match (&value.payload_object, &value.inline_payload) {
                (Some(v), _) => Some(stream::command_envelope::Payload::PayloadObject(v.into())),
                (None, Some(v)) => {
                    Some(stream::command_envelope::Payload::InlinePayload(v.clone()))
                }
                (None, None) => None,
            },
            delegation_chain: value
                .delegation_chain
                .iter()
                .map(|id| trust::DelegationRecord {
                    record_version: 1,
                    delegation_id: hex::decode(id).unwrap_or_else(|_| id.as_bytes().to_vec()),
                    issuer: None,
                    recipient: None,
                    issued_at: None,
                    not_before: None,
                    expires_at: None,
                    capability: None,
                    parent_delegation: None,
                    revocation_authorities: vec![],
                    delegation_metadata: None,
                    signature: None,
                })
                .collect(),
            requested_assurance: value.requested_assurance.as_ref().map(Into::into),
            command_metadata: value.command_metadata.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<NodeGenesisPayload> for stream::NodeGenesisPayload {
    fn into_core(&self) -> Result<NodeGenesisPayload, ConvertError> {
        Ok(NodeGenesisPayload {
            payload_version: i64::from(self.payload_version),
            node_id: self.node_id.clone(),
            primary_node_identity: self
                .primary_node_identity
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            initial_controllers: self
                .initial_controllers
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            initial_policy_object: self
                .initial_policy_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            bootstrap_records: self
                .bootstrap_records
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            assurance_claims: self
                .assurance_claims
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            node_roles: self.node_roles.clone(),
            genesis_metadata: self
                .genesis_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&NodeGenesisPayload> for stream::NodeGenesisPayload {
    fn from(value: &NodeGenesisPayload) -> Self {
        Self {
            payload_version: value.payload_version as u32,
            node_id: value.node_id.clone(),
            primary_node_identity: value.primary_node_identity.as_ref().map(Into::into),
            initial_controllers: value.initial_controllers.iter().map(Into::into).collect(),
            initial_policy_object: value.initial_policy_object.as_ref().map(Into::into),
            bootstrap_records: value.bootstrap_records.iter().map(Into::into).collect(),
            assurance_claims: value.assurance_claims.iter().map(Into::into).collect(),
            node_roles: value.node_roles.clone(),
            genesis_metadata: value.genesis_metadata.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<CommandSentPayload> for stream::CommandSentPayload {
    fn into_core(&self) -> Result<CommandSentPayload, ConvertError> {
        Ok(CommandSentPayload {
            payload_version: i64::from(self.payload_version),
            command: self.command.as_ref().map(IntoCore::into_core).transpose()?,
            target_node: self
                .target_node
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            send_metadata: self
                .send_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&CommandSentPayload> for stream::CommandSentPayload {
    fn from(value: &CommandSentPayload) -> Self {
        Self {
            payload_version: value.payload_version as u32,
            command: value.command.as_ref().map(Into::into),
            target_node: value.target_node.as_ref().map(Into::into),
            send_metadata: value.send_metadata.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<CommandResultPayload> for stream::CommandResultPayload {
    fn into_core(&self) -> Result<CommandResultPayload, ConvertError> {
        Ok(CommandResultPayload {
            payload_version: i64::from(self.payload_version),
            command: self.command.as_ref().map(IntoCore::into_core).transpose()?,
            issuer: self.issuer.as_ref().map(IntoCore::into_core).transpose()?,
            decision: stream::CommandDecision::try_from(self.decision)
                .ok()
                .map(|v| v.as_str_name().to_string()),
            decision_basis: self
                .decision_basis
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            reason_code: if self.reason_code.is_empty() {
                None
            } else {
                Some(self.reason_code.clone())
            },
            effect_summary_object: self
                .effect_summary_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            result_object: self
                .result_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&CommandResultPayload> for stream::CommandResultPayload {
    fn from(value: &CommandResultPayload) -> Self {
        Self {
            payload_version: value.payload_version as u32,
            command: value.command.as_ref().map(Into::into),
            issuer: value.issuer.as_ref().map(Into::into),
            decision: value
                .decision
                .as_deref()
                .and_then(stream::CommandDecision::from_str_name)
                .unwrap_or(stream::CommandDecision::Unspecified) as i32,
            decision_basis: value.decision_basis.as_ref().map(Into::into),
            reason_code: value.reason_code.clone().unwrap_or_default(),
            effect_summary_object: value.effect_summary_object.as_ref().map(Into::into),
            result_object: value.result_object.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<ActionLifecyclePayload> for stream::ActionLifecyclePayload {
    fn into_core(&self) -> Result<ActionLifecyclePayload, ConvertError> {
        Ok(ActionLifecyclePayload {
            payload_version: i64::from(self.payload_version),
            origin_command: self
                .origin_command
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            action_instance_id: self.action_instance_id.clone(),
            status: stream::ActionStatus::try_from(self.status)
                .ok()
                .map(|v| v.as_str_name().to_string()),
            result_object: self
                .result_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            error_object: self
                .error_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            progress_object: self
                .progress_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            action_metadata: self
                .action_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&ActionLifecyclePayload> for stream::ActionLifecyclePayload {
    fn from(value: &ActionLifecyclePayload) -> Self {
        Self {
            payload_version: value.payload_version as u32,
            origin_command: value.origin_command.as_ref().map(Into::into),
            action_instance_id: value.action_instance_id.clone(),
            status: value
                .status
                .as_deref()
                .and_then(stream::ActionStatus::from_str_name)
                .unwrap_or(stream::ActionStatus::Unspecified) as i32,
            result_object: value.result_object.as_ref().map(Into::into),
            error_object: value.error_object.as_ref().map(Into::into),
            progress_object: value.progress_object.as_ref().map(Into::into),
            action_metadata: value.action_metadata.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<TimeWindow> for common::TimeWindow {
    fn into_core(&self) -> Result<TimeWindow, ConvertError> {
        Ok(TimeWindow {
            not_before: optional_timestamp_from_proto(&self.not_before),
            expires_at: optional_timestamp_from_proto(&self.expires_at),
        })
    }
}

impl From<&TimeWindow> for common::TimeWindow {
    fn from(value: &TimeWindow) -> Self {
        Self {
            not_before: optional_timestamp_to_proto(&value.not_before),
            expires_at: optional_timestamp_to_proto(&value.expires_at),
        }
    }
}

impl IntoCore<CostLimit> for access::CostLimit {
    fn into_core(&self) -> Result<CostLimit, ConvertError> {
        Ok(CostLimit {
            max_results: self.max_results.map(|v| v as i64),
            max_total_bytes: self.max_total_bytes.map(|v| v as i64),
            max_wall_time: optional_duration_from_proto(&self.max_wall_time),
            max_federated_responders: self.max_federated_responders.map(i64::from),
        })
    }
}

impl From<&CostLimit> for access::CostLimit {
    fn from(value: &CostLimit) -> Self {
        Self {
            max_results: value.max_results.map(|v| v as u64),
            max_total_bytes: value.max_total_bytes.map(|v| v as u64),
            max_wall_time: optional_duration_to_proto(&value.max_wall_time),
            max_federated_responders: value.max_federated_responders.map(|v| v as u32),
        }
    }
}

impl IntoCore<SnapshotDescriptor> for access::SnapshotDescriptor {
    fn into_core(&self) -> Result<SnapshotDescriptor, ConvertError> {
        Ok(SnapshotDescriptor {
            descriptor_version: i64::from(self.descriptor_version),
            snapshot_id: self.snapshot_id.clone(),
            view_type: if self.view_type.is_empty() {
                None
            } else {
                Some(self.view_type.clone())
            },
            view_version: i64::from(self.view_version),
            producer: self
                .producer
                .as_ref()
                .ok_or_else(|| ConvertError("missing producer".into()))?
                .into_core()?,
            produced_at: optional_timestamp_from_proto(&self.produced_at),
            base_heads: self
                .base_heads
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            base_checkpoints: self
                .base_checkpoints
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            scope: self.scope.as_ref().map(scope_from_proto),
            completeness: snapshot_completeness_from_proto(self.completeness),
            payload_object: self
                .payload_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            supersedes: self
                .supersedes
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            snapshot_metadata: self
                .snapshot_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&SnapshotDescriptor> for access::SnapshotDescriptor {
    fn from(value: &SnapshotDescriptor) -> Self {
        Self {
            descriptor_version: value.descriptor_version as u32,
            snapshot_id: value.snapshot_id.clone(),
            view_type: value.view_type.clone().unwrap_or_default(),
            view_version: value.view_version as u32,
            producer: Some((&value.producer).into()),
            produced_at: optional_timestamp_to_proto(&value.produced_at),
            base_heads: value.base_heads.iter().map(Into::into).collect(),
            base_checkpoints: value.base_checkpoints.iter().map(Into::into).collect(),
            scope: value.scope.as_ref().map(scope_to_proto),
            completeness: snapshot_completeness_to_proto(&value.completeness)
                .unwrap_or(access::SnapshotCompleteness::Unspecified as i32),
            payload_object: value.payload_object.as_ref().map(Into::into),
            supersedes: value.supersedes.as_ref().map(Into::into),
            snapshot_metadata: value.snapshot_metadata.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<QueryRequest> for access::QueryRequest {
    fn into_core(&self) -> Result<QueryRequest, ConvertError> {
        Ok(QueryRequest {
            request_version: i64::from(self.request_version),
            query_id: self.query_id.clone(),
            requester: self
                .requester
                .as_ref()
                .ok_or_else(|| ConvertError("missing requester".into()))?
                .into_core()?,
            target_scope: self.target_scope.as_ref().map(scope_from_proto),
            query_class: query_class_from_proto(self.query_class),
            time_window: self
                .time_window
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            checkpoint_base: self
                .checkpoint_base
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            result_limit: self.result_limit.map(|v| v as i64),
            cost_limit: self
                .cost_limit
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            required_proof_classes: self
                .required_proof_classes
                .iter()
                .filter_map(|v| proof_class_from_proto(*v))
                .collect(),
            query_payload_object: self
                .query_payload_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&QueryRequest> for access::QueryRequest {
    fn from(value: &QueryRequest) -> Self {
        Self {
            request_version: value.request_version as u32,
            query_id: value.query_id.clone(),
            requester: Some((&value.requester).into()),
            target_scope: value.target_scope.as_ref().map(scope_to_proto),
            query_class: query_class_to_proto(&value.query_class)
                .unwrap_or(access::QueryClass::Unspecified as i32),
            time_window: value.time_window.as_ref().map(Into::into),
            checkpoint_base: value.checkpoint_base.as_ref().map(Into::into),
            result_limit: value.result_limit.map(|v| v as u64),
            cost_limit: value.cost_limit.as_ref().map(Into::into),
            required_proof_classes: value
                .required_proof_classes
                .iter()
                .filter_map(|v| proof_class_to_proto(v))
                .collect(),
            query_payload_object: value.query_payload_object.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<QueryResultFragment> for access::QueryResultFragment {
    fn into_core(&self) -> Result<QueryResultFragment, ConvertError> {
        Ok(QueryResultFragment {
            fragment_version: i64::from(self.fragment_version),
            query_id: self.query_id.clone(),
            responder: self
                .responder
                .as_ref()
                .ok_or_else(|| ConvertError("missing responder".into()))?
                .into_core()?,
            answered_at: optional_timestamp_from_proto(&self.answered_at),
            completeness: result_completeness_from_proto(self.completeness),
            snapshot_refs: self
                .snapshot_refs
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            event_refs: self
                .event_refs
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            object_refs: self
                .object_refs
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            proof_objects: self
                .proof_objects
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            omission_reason: if self.omission_reason.is_empty() {
                None
            } else {
                Some(self.omission_reason.clone())
            },
            bundled_result_object: self
                .bundled_result_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            result_metadata: self
                .result_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&QueryResultFragment> for access::QueryResultFragment {
    fn from(value: &QueryResultFragment) -> Self {
        Self {
            fragment_version: value.fragment_version as u32,
            query_id: value.query_id.clone(),
            responder: Some((&value.responder).into()),
            answered_at: optional_timestamp_to_proto(&value.answered_at),
            completeness: result_completeness_to_proto(&value.completeness)
                .unwrap_or(access::ResultCompleteness::Unspecified as i32),
            snapshot_refs: value.snapshot_refs.iter().map(Into::into).collect(),
            event_refs: value.event_refs.iter().map(Into::into).collect(),
            object_refs: value.object_refs.iter().map(Into::into).collect(),
            proof_objects: value.proof_objects.iter().map(Into::into).collect(),
            omission_reason: value.omission_reason.clone().unwrap_or_default(),
            bundled_result_object: value.bundled_result_object.as_ref().map(Into::into),
            result_metadata: value.result_metadata.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<FederatedAggregateDescriptor> for access::FederatedAggregateDescriptor {
    fn into_core(&self) -> Result<FederatedAggregateDescriptor, ConvertError> {
        Ok(FederatedAggregateDescriptor {
            descriptor_version: i64::from(self.descriptor_version),
            aggregate_id: self.aggregate_id.clone(),
            source_query_id: self.source_query_id.clone(),
            aggregator: self
                .aggregator
                .as_ref()
                .ok_or_else(|| ConvertError("missing aggregator".into()))?
                .into_core()?,
            aggregated_at: optional_timestamp_from_proto(&self.aggregated_at),
            input_fragments: self
                .input_fragments
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            aggregation_policy_object: self
                .aggregation_policy_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            payload_object: self
                .payload_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&FederatedAggregateDescriptor> for access::FederatedAggregateDescriptor {
    fn from(value: &FederatedAggregateDescriptor) -> Self {
        Self {
            descriptor_version: value.descriptor_version as u32,
            aggregate_id: value.aggregate_id.clone(),
            source_query_id: value.source_query_id.clone(),
            aggregator: Some((&value.aggregator).into()),
            aggregated_at: optional_timestamp_to_proto(&value.aggregated_at),
            input_fragments: value.input_fragments.iter().map(Into::into).collect(),
            aggregation_policy_object: value.aggregation_policy_object.as_ref().map(Into::into),
            payload_object: value.payload_object.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<StreamHeadsProof> for access::StreamHeadsProof {
    fn into_core(&self) -> Result<StreamHeadsProof, ConvertError> {
        Ok(StreamHeadsProof {
            source_query_id: self.source_query_id.clone(),
            heads: self
                .heads
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<&StreamHeadsProof> for access::StreamHeadsProof {
    fn from(value: &StreamHeadsProof) -> Self {
        Self {
            source_query_id: value.source_query_id.clone(),
            heads: value.heads.iter().map(Into::into).collect(),
        }
    }
}

impl IntoCore<SnapshotSetProof> for access::SnapshotSetProof {
    fn into_core(&self) -> Result<SnapshotSetProof, ConvertError> {
        Ok(SnapshotSetProof {
            source_query_id: self.source_query_id.clone(),
            snapshots: self
                .snapshots
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<&SnapshotSetProof> for access::SnapshotSetProof {
    fn from(value: &SnapshotSetProof) -> Self {
        Self {
            source_query_id: value.source_query_id.clone(),
            snapshots: value.snapshots.iter().map(Into::into).collect(),
        }
    }
}

impl IntoCore<EventSetProof> for access::EventSetProof {
    fn into_core(&self) -> Result<EventSetProof, ConvertError> {
        Ok(EventSetProof {
            source_query_id: self.source_query_id.clone(),
            events: self
                .events
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            related_objects: self
                .related_objects
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<&EventSetProof> for access::EventSetProof {
    fn from(value: &EventSetProof) -> Self {
        Self {
            source_query_id: value.source_query_id.clone(),
            events: value.events.iter().map(Into::into).collect(),
            related_objects: value.related_objects.iter().map(Into::into).collect(),
        }
    }
}

impl IntoCore<ObjectAssertionProof> for access::ObjectAssertionProof {
    fn into_core(&self) -> Result<ObjectAssertionProof, ConvertError> {
        Ok(ObjectAssertionProof {
            source_query_id: self.source_query_id.clone(),
            object_ref: self
                .object_ref
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            exists: Some(self.exists),
            bundled_result_object: self
                .bundled_result_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&ObjectAssertionProof> for access::ObjectAssertionProof {
    fn from(value: &ObjectAssertionProof) -> Self {
        Self {
            source_query_id: value.source_query_id.clone(),
            object_ref: value.object_ref.as_ref().map(Into::into),
            exists: value.exists.unwrap_or(false),
            bundled_result_object: value.bundled_result_object.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<ResultFragmentProof> for access::ResultFragmentProof {
    fn into_core(&self) -> Result<ResultFragmentProof, ConvertError> {
        Ok(ResultFragmentProof {
            fragment: self
                .fragment
                .as_ref()
                .ok_or_else(|| ConvertError("missing fragment".into()))?
                .into_core()?,
        })
    }
}

impl From<&ResultFragmentProof> for access::ResultFragmentProof {
    fn from(value: &ResultFragmentProof) -> Self {
        Self {
            fragment: Some((&value.fragment).into()),
        }
    }
}

impl IntoCore<AggregateSummaryProof> for access::AggregateSummaryProof {
    fn into_core(&self) -> Result<AggregateSummaryProof, ConvertError> {
        Ok(AggregateSummaryProof {
            source_query_id: self.source_query_id.clone(),
            included_responders: self
                .included_responders
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            excluded_responders: self
                .excluded_responders
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            total_trust_score: self.total_trust_score as i64,
            trust_policy_object: self
                .trust_policy_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&AggregateSummaryProof> for access::AggregateSummaryProof {
    fn from(value: &AggregateSummaryProof) -> Self {
        Self {
            source_query_id: value.source_query_id.clone(),
            included_responders: value.included_responders.iter().map(Into::into).collect(),
            excluded_responders: value.excluded_responders.iter().map(Into::into).collect(),
            total_trust_score: value.total_trust_score as u64,
            trust_policy_object: value.trust_policy_object.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<TrustPolicyProof> for access::TrustPolicyProof {
    fn into_core(&self) -> Result<TrustPolicyProof, ConvertError> {
        Ok(TrustPolicyProof {
            source_query_id: self.source_query_id.clone(),
            policy_object: self
                .policy_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            assignments_object: self
                .assignments_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&TrustPolicyProof> for access::TrustPolicyProof {
    fn from(value: &TrustPolicyProof) -> Self {
        Self {
            source_query_id: value.source_query_id.clone(),
            policy_object: value.policy_object.as_ref().map(Into::into),
            assignments_object: value.assignments_object.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<ProofBundle> for access::ProofBundle {
    fn into_core(&self) -> Result<ProofBundle, ConvertError> {
        Ok(ProofBundle {
            bundle_version: i64::from(self.bundle_version),
            payload_type: proof_payload_type_from_proto(self.payload_type),
            source_query_id: self.source_query_id.clone(),
            payload_object: self
                .payload_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            supporting_objects: self
                .supporting_objects
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&ProofBundle> for access::ProofBundle {
    fn from(value: &ProofBundle) -> Self {
        Self {
            bundle_version: value.bundle_version as u32,
            payload_type: proof_payload_type_to_proto(&value.payload_type)
                .unwrap_or(access::ProofPayloadType::Unspecified as i32),
            source_query_id: value.source_query_id.clone(),
            payload_object: value.payload_object.as_ref().map(Into::into),
            supporting_objects: value.supporting_objects.iter().map(Into::into).collect(),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<ReachabilityHint> for network::ReachabilityHint {
    fn into_core(&self) -> Result<ReachabilityHint, ConvertError> {
        Ok(ReachabilityHint {
            hint_version: i64::from(self.hint_version),
            subject_node: self
                .subject_node
                .as_ref()
                .ok_or_else(|| ConvertError("missing subject_node".into()))?
                .into_core()?,
            transport_class: transport_class_from_proto(self.transport_class),
            locator_payload: self.locator_payload.clone(),
            directness: directness_from_proto(self.directness),
            valid_after: optional_timestamp_from_proto(&self.valid_after),
            valid_until: optional_timestamp_from_proto(&self.valid_until),
            cost_hint: self.cost_hint.map(|v| v as i64),
            quality_hint: self.quality_hint.map(|v| v as i64),
            issuer: self.issuer.as_ref().map(IntoCore::into_core).transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&ReachabilityHint> for network::ReachabilityHint {
    fn from(value: &ReachabilityHint) -> Self {
        Self {
            hint_version: value.hint_version as u32,
            subject_node: Some((&value.subject_node).into()),
            transport_class: transport_class_to_proto(&value.transport_class)
                .unwrap_or(common::TransportClass::Unspecified as i32),
            locator_payload: value.locator_payload.clone(),
            directness: directness_to_proto(&value.directness)
                .unwrap_or(common::Directness::Unspecified as i32),
            valid_after: optional_timestamp_to_proto(&value.valid_after),
            valid_until: optional_timestamp_to_proto(&value.valid_until),
            cost_hint: value.cost_hint.map(|v| v as u64),
            quality_hint: value.quality_hint.map(|v| v as u64),
            issuer: value.issuer.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<RouteAdvertisement> for network::RouteAdvertisement {
    fn into_core(&self) -> Result<RouteAdvertisement, ConvertError> {
        Ok(RouteAdvertisement {
            advertisement_version: i64::from(self.advertisement_version),
            target_node: self
                .target_node
                .as_ref()
                .ok_or_else(|| ConvertError("missing target_node".into()))?
                .into_core()?,
            advertiser: self
                .advertiser
                .as_ref()
                .ok_or_else(|| ConvertError("missing advertiser".into()))?
                .into_core()?,
            next_hop_node: self
                .next_hop_node
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            reachability: self
                .reachability
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            metric_hint: self
                .metric_hint
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            advertised_at: optional_timestamp_from_proto(&self.advertised_at),
            expires_at: optional_timestamp_from_proto(&self.expires_at),
            route_metadata: self
                .route_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&RouteAdvertisement> for network::RouteAdvertisement {
    fn from(value: &RouteAdvertisement) -> Self {
        Self {
            advertisement_version: value.advertisement_version as u32,
            target_node: Some((&value.target_node).into()),
            advertiser: Some((&value.advertiser).into()),
            next_hop_node: value.next_hop_node.as_ref().map(Into::into),
            reachability: value.reachability.iter().map(Into::into).collect(),
            metric_hint: value.metric_hint.as_ref().map(Into::into),
            advertised_at: optional_timestamp_to_proto(&value.advertised_at),
            expires_at: optional_timestamp_to_proto(&value.expires_at),
            route_metadata: value.route_metadata.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<SessionHello> for network::SessionHello {
    fn into_core(&self) -> Result<SessionHello, ConvertError> {
        Ok(SessionHello {
            message_version: i64::from(self.message_version),
            initiator: self
                .initiator
                .as_ref()
                .ok_or_else(|| ConvertError("missing initiator".into()))?
                .into_core()?,
            target_node: self
                .target_node
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            supported_transport_features: self.supported_transport_features.clone(),
            supported_protocol_versions: self
                .supported_protocol_versions
                .iter()
                .map(|v| i64::from(*v))
                .collect(),
            session_nonce: self.session_nonce.clone(),
            initiator_locators: self
                .initiator_locators
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            hello_metadata: self
                .hello_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&SessionHello> for network::SessionHello {
    fn from(value: &SessionHello) -> Self {
        Self {
            message_version: value.message_version as u32,
            initiator: Some((&value.initiator).into()),
            target_node: value.target_node.as_ref().map(Into::into),
            supported_transport_features: value.supported_transport_features.clone(),
            supported_protocol_versions: value
                .supported_protocol_versions
                .iter()
                .map(|v| *v as u32)
                .collect(),
            session_nonce: value.session_nonce.clone(),
            initiator_locators: value.initiator_locators.iter().map(Into::into).collect(),
            hello_metadata: value.hello_metadata.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<SessionAccept> for network::SessionAccept {
    fn into_core(&self) -> Result<SessionAccept, ConvertError> {
        Ok(SessionAccept {
            message_version: i64::from(self.message_version),
            responder: self
                .responder
                .as_ref()
                .ok_or_else(|| ConvertError("missing responder".into()))?
                .into_core()?,
            echoed_session_nonce: self.echoed_session_nonce.clone(),
            selected_protocol_version: i64::from(self.selected_protocol_version),
            selected_transport_features: self.selected_transport_features.clone(),
            responder_locators: self
                .responder_locators
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            accept_metadata: self
                .accept_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&SessionAccept> for network::SessionAccept {
    fn from(value: &SessionAccept) -> Self {
        Self {
            message_version: value.message_version as u32,
            responder: Some((&value.responder).into()),
            echoed_session_nonce: value.echoed_session_nonce.clone(),
            selected_protocol_version: value.selected_protocol_version as u32,
            selected_transport_features: value.selected_transport_features.clone(),
            responder_locators: value.responder_locators.iter().map(Into::into).collect(),
            accept_metadata: value.accept_metadata.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<RelayEnvelope> for network::RelayEnvelope {
    fn into_core(&self) -> Result<RelayEnvelope, ConvertError> {
        let (payload_object, inline_payload) = match &self.payload {
            Some(network::relay_envelope::Payload::PayloadObject(v)) => {
                (Some(v.into_core()?), None)
            }
            Some(network::relay_envelope::Payload::InlinePayload(v)) => (None, Some(v.clone())),
            None => (None, None),
        };
        Ok(RelayEnvelope {
            envelope_version: i64::from(self.envelope_version),
            relay_message_id: self.relay_message_id.clone(),
            original_sender: self
                .original_sender
                .as_ref()
                .ok_or_else(|| ConvertError("missing original_sender".into()))?
                .into_core()?,
            intended_recipient_node: self
                .intended_recipient_node
                .as_ref()
                .ok_or_else(|| ConvertError("missing intended_recipient_node".into()))?
                .into_core()?,
            relay_chain: self
                .relay_chain
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            payload_kind: payload_kind_from_proto(self.payload_kind),
            payload_object,
            inline_payload,
            store_until: optional_timestamp_from_proto(&self.store_until),
            relay_metadata: self
                .relay_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&RelayEnvelope> for network::RelayEnvelope {
    fn from(value: &RelayEnvelope) -> Self {
        let payload = match (&value.payload_object, &value.inline_payload) {
            (Some(v), _) => Some(network::relay_envelope::Payload::PayloadObject(v.into())),
            (None, Some(v)) => Some(network::relay_envelope::Payload::InlinePayload(v.clone())),
            (None, None) => None,
        };
        Self {
            envelope_version: value.envelope_version as u32,
            relay_message_id: value.relay_message_id.clone(),
            original_sender: Some((&value.original_sender).into()),
            intended_recipient_node: Some((&value.intended_recipient_node).into()),
            relay_chain: value.relay_chain.iter().map(Into::into).collect(),
            payload_kind: payload_kind_to_proto(&value.payload_kind)
                .unwrap_or(network::PayloadKind::Unspecified as i32),
            payload,
            store_until: optional_timestamp_to_proto(&value.store_until),
            relay_metadata: value.relay_metadata.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<RateLimit> for common::RateLimit {
    fn into_core(&self) -> Result<RateLimit, ConvertError> {
        Ok(RateLimit {
            max_operations: self.max_operations as i64,
            per: optional_duration_from_proto(&self.per),
        })
    }
}

impl From<&RateLimit> for common::RateLimit {
    fn from(value: &RateLimit) -> Self {
        Self {
            max_operations: value.max_operations as u64,
            per: optional_duration_to_proto(&value.per),
        }
    }
}

impl IntoCore<IdentityRecord> for identity::IdentityRecord {
    fn into_core(&self) -> Result<IdentityRecord, ConvertError> {
        Ok(IdentityRecord {
            record_version: i64::from(self.record_version),
            identity_id: self.identity_id.clone(),
            identity_kind: Some(
                common::IdentityKind::try_from(self.identity_kind)
                    .ok()
                    .map(|v| {
                        v.as_str_name()
                            .trim_start_matches("IDENTITY_KIND_")
                            .to_ascii_lowercase()
                    })
                    .unwrap_or_else(|| "unspecified".into()),
            ),
            key_algorithm: key_algorithm_from_proto(self.key_algorithm),
            public_key: self.public_key.clone(),
            created_at: optional_timestamp_from_proto(&self.created_at),
            supersedes_identity: self
                .supersedes_identity
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            assurance_claim_objects: self
                .assurance_claim_objects
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            metadata_object: self
                .metadata_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&IdentityRecord> for identity::IdentityRecord {
    fn from(value: &IdentityRecord) -> Self {
        Self {
            record_version: value.record_version as u32,
            identity_id: value.identity_id.clone(),
            identity_kind: identity_kind_to_proto(&value.identity_kind)
                .unwrap_or(common::IdentityKind::Unspecified as i32),
            key_algorithm: key_algorithm_to_proto(&value.key_algorithm)
                .unwrap_or(identity::KeyAlgorithm::Unspecified as i32),
            public_key: value.public_key.clone(),
            created_at: optional_timestamp_to_proto(&value.created_at),
            supersedes_identity: value.supersedes_identity.as_ref().map(Into::into),
            assurance_claim_objects: value
                .assurance_claim_objects
                .iter()
                .map(Into::into)
                .collect(),
            metadata_object: value.metadata_object.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<AssuranceRequirement> for trust::AssuranceRequirement {
    fn into_core(&self) -> Result<AssuranceRequirement, ConvertError> {
        Ok(AssuranceRequirement {
            assurance_version: i64::from(self.assurance_version),
            required_class: assurance_class_from_proto(self.required_class),
            acceptable_attesters: self
                .acceptable_attesters
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            max_evidence_age: optional_duration_from_proto(&self.max_evidence_age),
            assurance_metadata: self
                .assurance_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&AssuranceRequirement> for trust::AssuranceRequirement {
    fn from(value: &AssuranceRequirement) -> Self {
        Self {
            assurance_version: value.assurance_version as u32,
            required_class: assurance_class_to_proto(&value.required_class)
                .unwrap_or(common::AssuranceClass::Unspecified as i32),
            acceptable_attesters: value.acceptable_attesters.iter().map(Into::into).collect(),
            max_evidence_age: optional_duration_to_proto(&value.max_evidence_age),
            assurance_metadata: value.assurance_metadata.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<AssuranceClaim> for trust::AssuranceClaim {
    fn into_core(&self) -> Result<AssuranceClaim, ConvertError> {
        let subject = match &self.subject {
            Some(trust::assurance_claim::Subject::SubjectIdentity(v)) => {
                Some(AssuranceSubject::Identity(v.into_core()?))
            }
            Some(trust::assurance_claim::Subject::SubjectNode(v)) => {
                Some(AssuranceSubject::Node(v.into_core()?))
            }
            None => None,
        };
        Ok(AssuranceClaim {
            claim_version: i64::from(self.claim_version),
            subject,
            assurance_class: assurance_class_from_proto(self.assurance_class),
            attester: self
                .attester
                .as_ref()
                .ok_or_else(|| ConvertError("missing attester".into()))?
                .into_core()?,
            issued_at: optional_timestamp_from_proto(&self.issued_at),
            expires_at: optional_timestamp_from_proto(&self.expires_at),
            evidence_object: self
                .evidence_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            claim_note: if self.claim_note.is_empty() {
                None
            } else {
                Some(self.claim_note.clone())
            },
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&AssuranceClaim> for trust::AssuranceClaim {
    fn from(value: &AssuranceClaim) -> Self {
        let subject = match &value.subject {
            Some(AssuranceSubject::Identity(v)) => {
                Some(trust::assurance_claim::Subject::SubjectIdentity(v.into()))
            }
            Some(AssuranceSubject::Node(v)) => {
                Some(trust::assurance_claim::Subject::SubjectNode(v.into()))
            }
            None => None,
        };
        Self {
            claim_version: value.claim_version as u32,
            subject,
            assurance_class: assurance_class_to_proto(&value.assurance_class)
                .unwrap_or(common::AssuranceClass::Unspecified as i32),
            attester: Some((&value.attester).into()),
            issued_at: optional_timestamp_to_proto(&value.issued_at),
            expires_at: optional_timestamp_to_proto(&value.expires_at),
            evidence_object: value.evidence_object.as_ref().map(Into::into),
            claim_note: value.claim_note.clone().unwrap_or_default(),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<ConstraintSet> for trust::ConstraintSet {
    fn into_core(&self) -> Result<ConstraintSet, ConvertError> {
        Ok(ConstraintSet {
            constraint_version: i64::from(self.constraint_version),
            not_before: optional_timestamp_from_proto(&self.not_before),
            expires_at: optional_timestamp_from_proto(&self.expires_at),
            max_uses: self.max_uses.map(|v| v as i64),
            rate_limit: self
                .rate_limit
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            requires_local_session: self.requires_local_session,
            requires_user_presence: self.requires_user_presence,
            requires_transport_classes: self
                .requires_transport_classes
                .iter()
                .filter_map(|v| transport_class_from_proto(*v))
                .collect(),
            requires_location_classes: self.requires_location_classes.clone(),
            export_policy: export_policy_from_proto(self.export_policy),
            execution_class_limits: self
                .execution_class_limits
                .iter()
                .map(|v| format!("{v}"))
                .collect(),
            storage_class_limits: self
                .storage_class_limits
                .iter()
                .map(|v| format!("{v}"))
                .collect(),
            constraint_metadata: self
                .constraint_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&ConstraintSet> for trust::ConstraintSet {
    fn from(value: &ConstraintSet) -> Self {
        Self {
            constraint_version: value.constraint_version as u32,
            not_before: optional_timestamp_to_proto(&value.not_before),
            expires_at: optional_timestamp_to_proto(&value.expires_at),
            max_uses: value.max_uses.map(|v| v as u64),
            rate_limit: value.rate_limit.as_ref().map(Into::into),
            requires_local_session: value.requires_local_session,
            requires_user_presence: value.requires_user_presence,
            requires_transport_classes: value
                .requires_transport_classes
                .iter()
                .filter_map(|v| transport_class_to_proto(&Some(v.clone())))
                .collect(),
            requires_location_classes: value.requires_location_classes.clone(),
            export_policy: export_policy_to_proto(&value.export_policy)
                .unwrap_or(trust::ExportPolicy::Unspecified as i32),
            execution_class_limits: vec![],
            storage_class_limits: vec![],
            constraint_metadata: value.constraint_metadata.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<CapabilityDescriptor> for trust::CapabilityDescriptor {
    fn into_core(&self) -> Result<CapabilityDescriptor, ConvertError> {
        Ok(CapabilityDescriptor {
            capability_version: i64::from(self.capability_version),
            capability_kind: Some(
                trust::CapabilityKind::try_from(self.capability_kind)
                    .ok()
                    .map(|v| v.as_str_name().to_string())
                    .unwrap_or_else(|| "CAPABILITY_KIND_UNSPECIFIED".into()),
            ),
            actions: self.actions.clone(),
            scope: self.scope.as_ref().map(scope_from_proto),
            constraints: self
                .constraints
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            delegation_policy: delegation_policy_from_proto(self.delegation_policy),
            minimum_assurance: self
                .minimum_assurance
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            capability_metadata: self
                .capability_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&CapabilityDescriptor> for trust::CapabilityDescriptor {
    fn from(value: &CapabilityDescriptor) -> Self {
        Self {
            capability_version: value.capability_version as u32,
            capability_kind: value
                .capability_kind
                .as_deref()
                .and_then(trust::CapabilityKind::from_str_name)
                .unwrap_or(trust::CapabilityKind::Unspecified) as i32,
            actions: value.actions.clone(),
            scope: value.scope.as_ref().map(scope_to_proto),
            constraints: value.constraints.as_ref().map(Into::into),
            delegation_policy: delegation_policy_to_proto(&value.delegation_policy)
                .unwrap_or(trust::DelegationPolicy::Unspecified as i32),
            minimum_assurance: value.minimum_assurance.as_ref().map(Into::into),
            capability_metadata: value.capability_metadata.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<RevocationRecord> for trust::RevocationRecord {
    fn into_core(&self) -> Result<RevocationRecord, ConvertError> {
        let target = match &self.target {
            Some(trust::revocation_record::Target::TargetDelegation(v)) => {
                Some(RevocationTarget::Delegation(v.into_core()?))
            }
            Some(trust::revocation_record::Target::TargetIdentity(v)) => {
                Some(RevocationTarget::Identity(v.into_core()?))
            }
            Some(trust::revocation_record::Target::TargetNode(v)) => {
                Some(RevocationTarget::Node(v.into_core()?))
            }
            Some(trust::revocation_record::Target::TargetObject(v)) => {
                Some(RevocationTarget::Object(v.into_core()?))
            }
            None => None,
        };
        Ok(RevocationRecord {
            record_version: i64::from(self.record_version),
            revocation_id: self.revocation_id.clone(),
            issuer: self
                .issuer
                .as_ref()
                .ok_or_else(|| ConvertError("missing issuer".into()))?
                .into_core()?,
            issued_at: optional_timestamp_from_proto(&self.issued_at),
            effective_at: optional_timestamp_from_proto(&self.effective_at),
            revocation_kind: revocation_kind_from_proto(self.revocation_kind),
            target,
            scope_override: self.scope_override.as_ref().map(scope_from_proto),
            reason_code: if self.reason_code.is_empty() {
                None
            } else {
                Some(self.reason_code.clone())
            },
            replacement_id: if self.replacement_id.is_empty() {
                None
            } else {
                Some(self.replacement_id.clone())
            },
            revocation_metadata: self
                .revocation_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            signature: self
                .signature
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&RevocationRecord> for trust::RevocationRecord {
    fn from(value: &RevocationRecord) -> Self {
        let target = match &value.target {
            Some(RevocationTarget::Delegation(v)) => {
                Some(trust::revocation_record::Target::TargetDelegation(v.into()))
            }
            Some(RevocationTarget::Identity(v)) => {
                Some(trust::revocation_record::Target::TargetIdentity(v.into()))
            }
            Some(RevocationTarget::Node(v)) => {
                Some(trust::revocation_record::Target::TargetNode(v.into()))
            }
            Some(RevocationTarget::Object(v)) => {
                Some(trust::revocation_record::Target::TargetObject(v.into()))
            }
            None => None,
        };
        Self {
            record_version: value.record_version as u32,
            revocation_id: value.revocation_id.clone(),
            issuer: Some((&value.issuer).into()),
            issued_at: optional_timestamp_to_proto(&value.issued_at),
            effective_at: optional_timestamp_to_proto(&value.effective_at),
            revocation_kind: revocation_kind_to_proto(&value.revocation_kind)
                .unwrap_or(trust::RevocationKind::Unspecified as i32),
            target,
            scope_override: value.scope_override.as_ref().map(scope_to_proto),
            reason_code: value.reason_code.clone().unwrap_or_default(),
            replacement_id: value.replacement_id.clone().unwrap_or_default(),
            revocation_metadata: value.revocation_metadata.as_ref().map(Into::into),
            signature: value.signature.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<LogicalObjectDescriptor> for object::LogicalObjectDescriptor {
    fn into_core(&self) -> Result<LogicalObjectDescriptor, ConvertError> {
        Ok(LogicalObjectDescriptor {
            descriptor_version: i64::from(self.descriptor_version),
            object_id: self.object_id.clone(),
            object_kind: object_kind_from_proto(Some(self.object_kind)),
            object_schema_version: i64::from(self.object_schema_version),
            canonicalization_id: if self.canonicalization_id.is_empty() {
                None
            } else {
                Some(self.canonicalization_id.clone())
            },
            canonical_digest: self
                .canonical_digest
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            canonical_size: self.canonical_size as i64,
            created_at: optional_timestamp_from_proto(&self.created_at),
            producer: self
                .producer
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            describes_object: self
                .describes_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            object_metadata: self
                .object_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&LogicalObjectDescriptor> for object::LogicalObjectDescriptor {
    fn from(value: &LogicalObjectDescriptor) -> Self {
        Self {
            descriptor_version: value.descriptor_version as u32,
            object_id: value.object_id.clone(),
            object_kind: object_kind_to_proto(&value.object_kind)
                .unwrap_or(common::ObjectKind::Unspecified as i32),
            object_schema_version: value.object_schema_version as u32,
            canonicalization_id: value.canonicalization_id.clone().unwrap_or_default(),
            canonical_digest: value.canonical_digest.as_ref().map(Into::into),
            canonical_size: value.canonical_size as u64,
            created_at: optional_timestamp_to_proto(&value.created_at),
            producer: value.producer.as_ref().map(Into::into),
            describes_object: value.describes_object.as_ref().map(Into::into),
            object_metadata: value.object_metadata.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<StoredRepresentationHeader> for object::StoredRepresentationHeader {
    fn into_core(&self) -> Result<StoredRepresentationHeader, ConvertError> {
        Ok(StoredRepresentationHeader {
            header_version: i64::from(self.header_version),
            representation_id: self.representation_id.clone(),
            object: self.object.as_ref().map(IntoCore::into_core).transpose()?,
            representation_digest: self
                .representation_digest
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            plaintext_size: self.plaintext_size.map(|v| v as i64),
            stored_size: self.stored_size as i64,
            encryption_scheme: if self.encryption_scheme.is_empty() {
                None
            } else {
                Some(self.encryption_scheme.clone())
            },
            compression_scheme: if self.compression_scheme.is_empty() {
                None
            } else {
                Some(self.compression_scheme.clone())
            },
            chunking_mode: chunking_mode_from_proto(self.chunking_mode),
            chunk_manifest_object: self
                .chunk_manifest_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            access_package_object: self
                .access_package_object
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            created_at: optional_timestamp_from_proto(&self.created_at),
            representation_metadata: self
                .representation_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&StoredRepresentationHeader> for object::StoredRepresentationHeader {
    fn from(value: &StoredRepresentationHeader) -> Self {
        Self {
            header_version: value.header_version as u32,
            representation_id: value.representation_id.clone(),
            object: value.object.as_ref().map(Into::into),
            representation_digest: value.representation_digest.as_ref().map(Into::into),
            plaintext_size: value.plaintext_size.map(|v| v as u64),
            stored_size: value.stored_size as u64,
            encryption_scheme: value.encryption_scheme.clone().unwrap_or_default(),
            compression_scheme: value.compression_scheme.clone().unwrap_or_default(),
            chunking_mode: chunking_mode_to_proto(&value.chunking_mode)
                .unwrap_or(object::ChunkingMode::Unspecified as i32),
            chunk_manifest_object: value.chunk_manifest_object.as_ref().map(Into::into),
            access_package_object: value.access_package_object.as_ref().map(Into::into),
            created_at: optional_timestamp_to_proto(&value.created_at),
            representation_metadata: value.representation_metadata.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<ChunkEntry> for object::ChunkEntry {
    fn into_core(&self) -> Result<ChunkEntry, ConvertError> {
        Ok(ChunkEntry {
            index: i64::from(self.index),
            chunk_representation_id: self.chunk_representation_id.clone(),
            chunk_digest: self
                .chunk_digest
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            offset: self.offset as i64,
            length: self.length as i64,
        })
    }
}

impl From<&ChunkEntry> for object::ChunkEntry {
    fn from(value: &ChunkEntry) -> Self {
        Self {
            index: value.index as u32,
            chunk_representation_id: value.chunk_representation_id.clone(),
            chunk_digest: value.chunk_digest.as_ref().map(Into::into),
            offset: value.offset as u64,
            length: value.length as u64,
        }
    }
}

impl IntoCore<ChunkManifest> for object::ChunkManifest {
    fn into_core(&self) -> Result<ChunkManifest, ConvertError> {
        Ok(ChunkManifest {
            manifest_version: i64::from(self.manifest_version),
            object: self.object.as_ref().map(IntoCore::into_core).transpose()?,
            representation: self
                .representation
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
            chunk_count: i64::from(self.chunk_count),
            total_stored_size: self.total_stored_size as i64,
            chunk_entries: self
                .chunk_entries
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            manifest_metadata: self
                .manifest_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&ChunkManifest> for object::ChunkManifest {
    fn from(value: &ChunkManifest) -> Self {
        Self {
            manifest_version: value.manifest_version as u32,
            object: value.object.as_ref().map(Into::into),
            representation: value.representation.as_ref().map(Into::into),
            chunk_count: value.chunk_count as u32,
            total_stored_size: value.total_stored_size as u64,
            chunk_entries: value.chunk_entries.iter().map(Into::into).collect(),
            manifest_metadata: value.manifest_metadata.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<AggregateTrustPolicy> for trust::AggregateTrustPolicy {
    fn into_core(&self) -> Result<AggregateTrustPolicy, ConvertError> {
        Ok(AggregateTrustPolicy {
            policy_version: i64::from(self.policy_version),
            minimum_trust_score: self.minimum_trust_score.map(|v| v as i64),
            preferred_aggregators: self
                .preferred_aggregators
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            allowed_responders: self
                .allowed_responders
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            policy_metadata: self
                .policy_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&AggregateTrustPolicy> for trust::AggregateTrustPolicy {
    fn from(value: &AggregateTrustPolicy) -> Self {
        Self {
            policy_version: value.policy_version as u32,
            minimum_trust_score: value.minimum_trust_score.map(|v| v as u64),
            preferred_aggregators: value.preferred_aggregators.iter().map(Into::into).collect(),
            allowed_responders: value.allowed_responders.iter().map(Into::into).collect(),
            policy_metadata: value.policy_metadata.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<RouteTrustAssignment> for trust::RouteTrustAssignment {
    fn into_core(&self) -> Result<RouteTrustAssignment, ConvertError> {
        Ok(RouteTrustAssignment {
            subject: self
                .subject
                .as_ref()
                .ok_or_else(|| ConvertError("missing subject".into()))?
                .into_core()?,
            trust_score: self.trust_score as i64,
            source: if self.source.is_empty() {
                None
            } else {
                Some(self.source.clone())
            },
        })
    }
}

impl From<&RouteTrustAssignment> for trust::RouteTrustAssignment {
    fn from(value: &RouteTrustAssignment) -> Self {
        Self {
            subject: Some((&value.subject).into()),
            trust_score: value.trust_score as u64,
            source: value.source.clone().unwrap_or_default(),
        }
    }
}

impl IntoCore<RouteTrustAssignments> for trust::RouteTrustAssignments {
    fn into_core(&self) -> Result<RouteTrustAssignments, ConvertError> {
        Ok(RouteTrustAssignments {
            assignments: self
                .assignments
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
        })
    }
}

impl From<&RouteTrustAssignments> for trust::RouteTrustAssignments {
    fn from(value: &RouteTrustAssignments) -> Self {
        Self {
            assignments: value.assignments.iter().map(Into::into).collect(),
        }
    }
}

impl IntoCore<RouteSelectionPolicy> for trust::RouteSelectionPolicy {
    fn into_core(&self) -> Result<RouteSelectionPolicy, ConvertError> {
        Ok(RouteSelectionPolicy {
            policy_version: i64::from(self.policy_version),
            minimum_quality_hint: self.minimum_quality_hint.map(|v| v as i64),
            maximum_cost_hint: self.maximum_cost_hint.map(|v| v as i64),
            preferred_advertisers: self
                .preferred_advertisers
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            preferred_next_hops: self
                .preferred_next_hops
                .iter()
                .map(IntoCore::into_core)
                .collect::<Result<_, _>>()?,
            require_active_session: self.require_active_session,
            policy_metadata: self
                .policy_metadata
                .as_ref()
                .map(IntoCore::into_core)
                .transpose()?,
        })
    }
}

impl From<&RouteSelectionPolicy> for trust::RouteSelectionPolicy {
    fn from(value: &RouteSelectionPolicy) -> Self {
        Self {
            policy_version: value.policy_version as u32,
            minimum_quality_hint: value.minimum_quality_hint.map(|v| v as u64),
            maximum_cost_hint: value.maximum_cost_hint.map(|v| v as u64),
            preferred_advertisers: value.preferred_advertisers.iter().map(Into::into).collect(),
            preferred_next_hops: value.preferred_next_hops.iter().map(Into::into).collect(),
            require_active_session: value.require_active_session,
            policy_metadata: value.policy_metadata.as_ref().map(Into::into),
        }
    }
}

impl IntoCore<ProtocolRecord> for stream::EventEnvelope {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::EventEnvelope(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for stream::CommandEnvelope {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::CommandEnvelope(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for trust::DelegationRecord {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::DelegationRecord(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for access::SnapshotDescriptor {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::SnapshotDescriptor(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for access::QueryRequest {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::QueryRequest(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for access::QueryResultFragment {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::QueryResultFragment(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for access::FederatedAggregateDescriptor {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::FederatedAggregateDescriptor(
            self.into_core()?,
        ))
    }
}

impl IntoCore<ProtocolRecord> for access::StreamHeadsProof {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::StreamHeadsProof(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for access::SnapshotSetProof {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::SnapshotSetProof(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for access::EventSetProof {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::EventSetProof(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for access::ObjectAssertionProof {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::ObjectAssertionProof(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for access::ResultFragmentProof {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::ResultFragmentProof(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for access::AggregateSummaryProof {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::AggregateSummaryProof(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for access::TrustPolicyProof {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::TrustPolicyProof(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for access::ProofBundle {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::ProofBundle(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for network::ReachabilityHint {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::ReachabilityHint(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for network::RouteAdvertisement {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::RouteAdvertisement(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for network::SessionHello {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::SessionHello(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for network::SessionAccept {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::SessionAccept(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for network::RelayEnvelope {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::RelayEnvelope(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for identity::IdentityRecord {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::IdentityRecord(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for trust::AssuranceClaim {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::AssuranceClaim(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for trust::AssuranceRequirement {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::AssuranceRequirement(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for trust::ConstraintSet {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::ConstraintSet(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for trust::CapabilityDescriptor {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::CapabilityDescriptor(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for trust::RevocationRecord {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::RevocationRecord(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for object::LogicalObjectDescriptor {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::LogicalObjectDescriptor(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for object::StoredRepresentationHeader {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::StoredRepresentationHeader(
            self.into_core()?,
        ))
    }
}

impl IntoCore<ProtocolRecord> for object::ChunkManifest {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::ChunkManifest(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for trust::AggregateTrustPolicy {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::AggregateTrustPolicy(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for trust::RouteTrustAssignments {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::RouteTrustAssignments(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for trust::RouteSelectionPolicy {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::RouteSelectionPolicy(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for stream::NodeGenesisPayload {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::NodeGenesisPayload(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for stream::CommandSentPayload {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::CommandSentPayload(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for stream::CommandResultPayload {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::CommandResultPayload(self.into_core()?))
    }
}

impl IntoCore<ProtocolRecord> for stream::ActionLifecyclePayload {
    fn into_core(&self) -> Result<ProtocolRecord, ConvertError> {
        Ok(ProtocolRecord::ActionLifecyclePayload(self.into_core()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_ref_roundtrip_subset() {
        let core = IdentityRef {
            identity_id: vec![1, 2, 3],
            identity_kind: Some("user".to_string()),
            key_hint: Some(vec![9, 9]),
        };
        let proto: common::IdentityRef = (&core).into();
        let back: IdentityRef = proto.into_core().expect("into core");
        assert_eq!(back, core);
    }

    #[test]
    fn command_envelope_roundtrip_subset() {
        let core = CommandEnvelope {
            envelope_version: 1,
            command_id: Some(vec![1, 2, 3, 4]),
            target_node: NodeRef {
                node_id: vec![0xaa],
            },
            issuer: IdentityRef {
                identity_id: vec![0x10],
                identity_kind: Some("node".to_string()),
                key_hint: Some(vec![0x20]),
            },
            command_type: Some("COMMAND_TYPE_QUERY".to_string()),
            command_version: 1,
            issued_at: Some(Timestamp {
                seconds: 100,
                nanos: 5,
            }),
            not_before: None,
            expires_at: None,
            idempotency_key: Some(vec![7, 8]),
            payload_object: None,
            inline_payload: None,
            delegation_chain: vec!["64656c6567".to_string()],
            requested_assurance: None,
            command_metadata: None,
            signature: Some(Signature {
                algorithm: "ed25519".to_string(),
                value: vec![1; 64],
            }),
        };
        let proto: stream::CommandEnvelope = (&core).into();
        let back: CommandEnvelope = proto.into_core().expect("into core");
        assert_eq!(back.command_id, core.command_id);
        assert_eq!(back.target_node, core.target_node);
        assert_eq!(back.issuer, core.issuer);
        assert_eq!(back.command_type, core.command_type);
        assert_eq!(back.delegation_chain, core.delegation_chain);
    }

    #[test]
    fn snapshot_descriptor_roundtrip_subset() {
        let core = SnapshotDescriptor {
            descriptor_version: 1,
            snapshot_id: vec![0xaa, 0xbb],
            view_type: Some("timeline".into()),
            view_version: 1,
            producer: IdentityRef {
                identity_id: vec![1],
                identity_kind: Some("node".into()),
                key_hint: None,
            },
            produced_at: Some(Timestamp {
                seconds: 5,
                nanos: 6,
            }),
            base_heads: vec![HeadRef {
                stream_id: vec![2],
                seq: 3,
                event_hash: None,
            }],
            base_checkpoints: vec![],
            scope: Some(ScopeDescriptor {
                scope_version: 1,
                scope_kind: "SCOPE_KIND_VIEW".into(),
                target_nodes: vec![],
                target_streams: vec![],
                target_object_kinds: vec![],
                target_view_types: vec!["timeline".into()],
                target_domains: vec![],
                time_bounds: None,
                scope_metadata: None,
                segments: vec!["timeline".into()],
            }),
            completeness: Some("full".into()),
            payload_object: Some(ObjectRef {
                object_id: vec![9],
                object_kind: Some("snapshot".into()),
            }),
            supersedes: None,
            snapshot_metadata: None,
            signature: Some(Signature {
                algorithm: "ed25519".into(),
                value: vec![7; 64],
            }),
        };
        let proto: access::SnapshotDescriptor = (&core).into();
        let back: SnapshotDescriptor = proto.into_core().expect("into core");
        assert_eq!(back.snapshot_id, core.snapshot_id);
        assert_eq!(back.view_type, core.view_type);
        assert_eq!(back.base_heads, core.base_heads);
        assert_eq!(back.completeness, core.completeness);
    }

    #[test]
    fn route_advertisement_roundtrip_subset() {
        let core = RouteAdvertisement {
            advertisement_version: 1,
            target_node: NodeRef {
                node_id: vec![0x01],
            },
            advertiser: IdentityRef {
                identity_id: vec![0x02],
                identity_kind: Some("node".into()),
                key_hint: None,
            },
            next_hop_node: Some(NodeRef {
                node_id: vec![0x03],
            }),
            reachability: vec![ReachabilityHint {
                hint_version: 1,
                subject_node: NodeRef {
                    node_id: vec![0x01],
                },
                transport_class: Some("quic".into()),
                locator_payload: vec![0x77],
                directness: Some("direct".into()),
                valid_after: None,
                valid_until: None,
                cost_hint: Some(1),
                quality_hint: Some(9),
                issuer: None,
                signature: None,
            }],
            metric_hint: None,
            advertised_at: Some(Timestamp {
                seconds: 9,
                nanos: 0,
            }),
            expires_at: None,
            route_metadata: None,
            signature: Some(Signature {
                algorithm: "ed25519".into(),
                value: vec![3; 64],
            }),
        };
        let proto: network::RouteAdvertisement = (&core).into();
        let back: RouteAdvertisement = proto.into_core().expect("into core");
        assert_eq!(back.target_node, core.target_node);
        assert_eq!(back.reachability.len(), 1);
        assert_eq!(back.reachability[0].transport_class, Some("quic".into()));
    }

    #[test]
    fn identity_record_roundtrip_subset() {
        let core = IdentityRecord {
            record_version: 1,
            identity_id: vec![1, 2],
            identity_kind: Some("user".into()),
            key_algorithm: Some("ed25519".into()),
            public_key: vec![9; 32],
            created_at: Some(Timestamp {
                seconds: 1,
                nanos: 2,
            }),
            supersedes_identity: None,
            assurance_claim_objects: vec![ObjectRef {
                object_id: vec![4],
                object_kind: Some("proof".into()),
            }],
            metadata_object: None,
            signature: Some(Signature {
                algorithm: "ed25519".into(),
                value: vec![5; 64],
            }),
        };
        let proto: identity::IdentityRecord = (&core).into();
        let back: IdentityRecord = proto.into_core().expect("into core");
        assert_eq!(back.identity_id, core.identity_id);
        assert_eq!(back.key_algorithm, Some("ed25519".into()));
    }

    #[test]
    fn chunk_manifest_roundtrip_subset() {
        let core = ChunkManifest {
            manifest_version: 1,
            object: Some(ObjectRef {
                object_id: vec![1],
                object_kind: Some("payload".into()),
            }),
            representation: Some(RepresentationRef {
                representation_id: vec![2],
                object_id: vec![1],
            }),
            chunk_count: 1,
            total_stored_size: 10,
            chunk_entries: vec![ChunkEntry {
                index: 0,
                chunk_representation_id: vec![3],
                chunk_digest: Some(Digest {
                    algorithm: "blake3_256".into(),
                    value: vec![7; 32],
                }),
                offset: 0,
                length: 10,
            }],
            manifest_metadata: None,
        };
        let proto: object::ChunkManifest = (&core).into();
        let back: ChunkManifest = proto.into_core().expect("into core");
        assert_eq!(back.chunk_count, 1);
        assert_eq!(back.chunk_entries.len(), 1);
        assert_eq!(back.chunk_entries[0].length, 10);
    }

    #[test]
    fn delegation_record_roundtrip_richer_fields() {
        let core = DelegationRecord {
            record_version: 2,
            delegation_id: vec![0xaa],
            issuer: IdentityRef {
                identity_id: vec![1],
                identity_kind: Some("user".into()),
                key_hint: None,
            },
            recipient: IdentityRef {
                identity_id: vec![2],
                identity_kind: Some("agent".into()),
                key_hint: Some(vec![9]),
            },
            capability: Capability {
                capability_version: 3,
                kind: "CAPABILITY_KIND_QUERY".into(),
                actions: vec!["read".into()],
                scope: Some(ScopeDescriptor {
                    scope_version: 4,
                    scope_kind: "SCOPE_KIND_DOMAIN".into(),
                    target_nodes: vec![],
                    target_streams: vec![],
                    target_object_kinds: vec![],
                    target_view_types: vec![],
                    target_domains: vec!["local".into()],
                    time_bounds: None,
                    scope_metadata: None,
                    segments: vec!["local".into()],
                }),
                constraints: None,
                delegation_policy: Some("delegable_with_attenuation".into()),
                minimum_assurance: None,
                capability_metadata: None,
            },
            issued_at: Some(Timestamp {
                seconds: 1,
                nanos: 0,
            }),
            not_before: Some(Timestamp {
                seconds: 2,
                nanos: 0,
            }),
            expires_at: Some(Timestamp {
                seconds: 3,
                nanos: 0,
            }),
            parent_delegation: Some(DelegationRef {
                delegation_id: vec![0xbb],
                delegation_hash: None,
            }),
            revocation_authorities: vec![IdentityRef {
                identity_id: vec![4],
                identity_kind: Some("service".into()),
                key_hint: None,
            }],
            delegation_metadata: Some(ObjectRef {
                object_id: vec![5],
                object_kind: Some("proof".into()),
            }),
            signature: Some(Signature {
                algorithm: "ed25519".into(),
                value: vec![6; 64],
            }),
        };
        let proto: trust::DelegationRecord = (&core).into();
        let back: DelegationRecord = proto.into_core().expect("into core");
        assert_eq!(back.record_version, core.record_version);
        assert_eq!(back.not_before, core.not_before);
        assert_eq!(back.parent_delegation, core.parent_delegation);
        assert_eq!(back.revocation_authorities.len(), 1);
    }

    #[test]
    fn command_envelope_roundtrip_payload_and_assurance() {
        let core = CommandEnvelope {
            envelope_version: 1,
            command_id: Some(vec![1, 2, 3]),
            target_node: NodeRef { node_id: vec![8] },
            issuer: IdentityRef {
                identity_id: vec![9],
                identity_kind: Some("user".into()),
                key_hint: None,
            },
            command_type: Some("COMMAND_TYPE_QUERY".into()),
            command_version: 2,
            issued_at: Some(Timestamp {
                seconds: 1,
                nanos: 2,
            }),
            not_before: None,
            expires_at: None,
            idempotency_key: None,
            payload_object: Some(ObjectRef {
                object_id: vec![7],
                object_kind: Some("command".into()),
            }),
            inline_payload: None,
            delegation_chain: vec!["abcd".into()],
            requested_assurance: Some(AssuranceRequirement {
                assurance_version: 1,
                required_class: Some("software".into()),
                acceptable_attesters: vec![],
                max_evidence_age: Some(Duration {
                    seconds: 60,
                    nanos: 0,
                }),
                assurance_metadata: None,
            }),
            command_metadata: Some(ObjectRef {
                object_id: vec![6],
                object_kind: Some("payload".into()),
            }),
            signature: Some(Signature {
                algorithm: "ed25519".into(),
                value: vec![1; 64],
            }),
        };
        let proto: stream::CommandEnvelope = (&core).into();
        let back: CommandEnvelope = proto.into_core().expect("into core");
        assert_eq!(back.payload_object, core.payload_object);
        assert!(back.inline_payload.is_none());
        assert!(back.requested_assurance.is_some());
        assert_eq!(back.command_metadata, core.command_metadata);
    }

    #[test]
    fn event_envelope_roundtrip_related_fields() {
        let core = EventEnvelope {
            envelope_version: 1,
            stream_id: vec![1],
            seq: 2,
            prev_event_hash: None,
            event_type: Some("EVENT_TYPE_COMMAND_COMMITTED".into()),
            event_version: 3,
            recorded_at: Some(Timestamp {
                seconds: 4,
                nanos: 0,
            }),
            effective_at: None,
            payload_object: Some(ObjectRef {
                object_id: vec![2],
                object_kind: Some("payload".into()),
            }),
            related_events: vec![EventRef {
                stream_id: vec![1],
                seq: 1,
                event_hash: None,
            }],
            related_commands: vec!["beef".into()],
            related_objects: vec![ObjectRef {
                object_id: vec![3],
                object_kind: Some("proof".into()),
            }],
            related_delegations: vec![DelegationRef {
                delegation_id: vec![4],
                delegation_hash: None,
            }],
            related_revocations: vec![RevocationRef {
                revocation_id: vec![5],
                revocation_hash: None,
            }],
            event_metadata: Some(ObjectRef {
                object_id: vec![6],
                object_kind: Some("payload".into()),
            }),
            signature: Some(Signature {
                algorithm: "ed25519".into(),
                value: vec![7; 64],
            }),
        };
        let proto: stream::EventEnvelope = (&core).into();
        let back: EventEnvelope = proto.into_core().expect("into core");
        assert_eq!(back.payload_object, core.payload_object);
        assert_eq!(back.related_events, core.related_events);
        assert_eq!(back.related_objects, core.related_objects);
        assert_eq!(back.related_delegations, core.related_delegations);
        assert_eq!(back.related_revocations, core.related_revocations);
        assert_eq!(back.event_metadata, core.event_metadata);
    }

    #[test]
    fn proof_bundle_roundtrip_subset() {
        let core = ProofBundle {
            bundle_version: 1,
            payload_type: Some("PROOF_PAYLOAD_TYPE_RESULT_FRAGMENT".into()),
            source_query_id: vec![1, 2, 3],
            payload_object: Some(ObjectRef {
                object_id: vec![9, 9],
                object_kind: Some("OBJECT_KIND_PAYLOAD".into()),
            }),
            supporting_objects: vec![ObjectRef {
                object_id: vec![8, 8],
                object_kind: None,
            }],
            signature: None,
        };
        let proto: access::ProofBundle = (&core).into();
        let back: ProofBundle = proto.into_core().expect("into core");
        assert!(back.payload_type.is_some());
        assert_eq!(back.supporting_objects.len(), 1);
    }

    #[test]
    fn stream_heads_proof_roundtrip_subset() {
        let core = StreamHeadsProof {
            source_query_id: vec![4, 5, 6],
            heads: vec![HeadRef {
                stream_id: vec![7],
                seq: 3,
                event_hash: Some(Digest {
                    algorithm: "blake3".into(),
                    value: vec![0xAB],
                }),
            }],
        };
        let proto: access::StreamHeadsProof = (&core).into();
        let back: StreamHeadsProof = proto.into_core().expect("into core");
        assert_eq!(back.source_query_id, core.source_query_id);
        assert_eq!(back.heads[0].seq, 3);
    }

    #[test]
    fn snapshot_set_proof_roundtrip_subset() {
        let core = SnapshotSetProof {
            source_query_id: vec![1, 2, 3],
            snapshots: vec![SnapshotRef {
                snapshot_id: vec![4, 5],
                object_id: Some(vec![6, 7]),
            }],
        };
        let proto: access::SnapshotSetProof = (&core).into();
        let back: SnapshotSetProof = proto.into_core().expect("into core");
        assert_eq!(back.source_query_id, core.source_query_id);
        assert_eq!(back.snapshots, core.snapshots);
    }

    #[test]
    fn event_set_proof_roundtrip_subset() {
        let core = EventSetProof {
            source_query_id: vec![7],
            events: vec![EventRef {
                stream_id: vec![8],
                seq: 9,
                event_hash: Some(Digest {
                    algorithm: "blake3_256".into(),
                    value: vec![0xaa],
                }),
            }],
            related_objects: vec![ObjectRef {
                object_id: vec![1, 2],
                object_kind: Some("payload".into()),
            }],
        };
        let proto: access::EventSetProof = (&core).into();
        let back: EventSetProof = proto.into_core().expect("into core");
        assert_eq!(back.events, core.events);
        assert_eq!(back.related_objects, core.related_objects);
    }

    #[test]
    fn object_assertion_proof_roundtrip_subset() {
        let core = ObjectAssertionProof {
            source_query_id: vec![3, 1, 4],
            object_ref: Some(ObjectRef {
                object_id: vec![2, 7],
                object_kind: Some("payload".into()),
            }),
            exists: Some(true),
            bundled_result_object: Some(ObjectRef {
                object_id: vec![5, 8],
                object_kind: None,
            }),
        };
        let proto: access::ObjectAssertionProof = (&core).into();
        let back: ObjectAssertionProof = proto.into_core().expect("into core");
        assert_eq!(back, core);
    }

    #[test]
    fn aggregate_summary_proof_roundtrip_subset() {
        let core = AggregateSummaryProof {
            source_query_id: vec![6, 2],
            included_responders: vec![IdentityRef {
                identity_id: vec![1],
                identity_kind: Some("node".into()),
                key_hint: None,
            }],
            excluded_responders: vec![IdentityRef {
                identity_id: vec![2],
                identity_kind: Some("node".into()),
                key_hint: Some(vec![3]),
            }],
            total_trust_score: 42,
            trust_policy_object: Some(ObjectRef {
                object_id: vec![9],
                object_kind: None,
            }),
        };
        let proto: access::AggregateSummaryProof = (&core).into();
        let back: AggregateSummaryProof = proto.into_core().expect("into core");
        assert_eq!(back.included_responders, core.included_responders);
        assert_eq!(back.excluded_responders, core.excluded_responders);
        assert_eq!(back.total_trust_score, 42);
    }

    #[test]
    fn trust_policy_proof_roundtrip_subset() {
        let core = TrustPolicyProof {
            source_query_id: vec![0x10],
            policy_object: Some(ObjectRef {
                object_id: vec![0x11],
                object_kind: None,
            }),
            assignments_object: Some(ObjectRef {
                object_id: vec![0x12],
                object_kind: Some("payload".into()),
            }),
        };
        let proto: access::TrustPolicyProof = (&core).into();
        let back: TrustPolicyProof = proto.into_core().expect("into core");
        assert_eq!(back, core);
    }

    #[test]
    fn node_genesis_payload_roundtrip_subset() {
        let core = NodeGenesisPayload {
            payload_version: 1,
            node_id: vec![0xAA],
            primary_node_identity: Some(IdentityRef {
                identity_id: vec![0x01],
                identity_kind: Some("node".into()),
                key_hint: None,
            }),
            initial_controllers: vec![IdentityRef {
                identity_id: vec![0x02],
                identity_kind: Some("user".into()),
                key_hint: Some(vec![0x03]),
            }],
            initial_policy_object: Some(ObjectRef {
                object_id: vec![0x04],
                object_kind: None,
            }),
            bootstrap_records: vec![ObjectRef {
                object_id: vec![0x05],
                object_kind: Some("payload".into()),
            }],
            assurance_claims: vec![ObjectRef {
                object_id: vec![0x06],
                object_kind: Some("proof".into()),
            }],
            node_roles: vec!["storage".into(), "relay".into()],
            genesis_metadata: None,
        };
        let proto: stream::NodeGenesisPayload = (&core).into();
        let back: NodeGenesisPayload = proto.into_core().expect("into core");
        assert_eq!(back, core);
    }

    #[test]
    fn action_lifecycle_payload_roundtrip_subset() {
        let core = ActionLifecyclePayload {
            payload_version: 2,
            origin_command: Some(CommandRef {
                command_id: vec![0x10],
                command_hash: Some(Digest {
                    algorithm: "blake3_256".into(),
                    value: vec![0x11],
                }),
            }),
            action_instance_id: vec![0x12, 0x13],
            status: Some("ACTION_STATUS_COMPLETED".into()),
            result_object: Some(ObjectRef {
                object_id: vec![0x14],
                object_kind: Some("payload".into()),
            }),
            error_object: None,
            progress_object: Some(ObjectRef {
                object_id: vec![0x15],
                object_kind: None,
            }),
            action_metadata: Some(ObjectRef {
                object_id: vec![0x16],
                object_kind: Some("payload".into()),
            }),
        };
        let proto: stream::ActionLifecyclePayload = (&core).into();
        let back: ActionLifecyclePayload = proto.into_core().expect("into core");
        assert_eq!(back, core);
    }

    #[test]
    fn command_sent_payload_roundtrip_subset() {
        let core = CommandSentPayload {
            payload_version: 1,
            command: Some(CommandRef {
                command_id: vec![0x21],
                command_hash: Some(Digest {
                    algorithm: "blake3_256".into(),
                    value: vec![0x22],
                }),
            }),
            target_node: Some(NodeRef {
                node_id: vec![0x23],
            }),
            send_metadata: Some(ObjectRef {
                object_id: vec![0x24],
                object_kind: Some("payload".into()),
            }),
        };
        let proto: stream::CommandSentPayload = (&core).into();
        let back: CommandSentPayload = proto.into_core().expect("into core");
        assert_eq!(back, core);
    }

    #[test]
    fn command_result_payload_roundtrip_subset() {
        let core = CommandResultPayload {
            payload_version: 3,
            command: Some(CommandRef {
                command_id: vec![0x31],
                command_hash: Some(Digest {
                    algorithm: "blake3_256".into(),
                    value: vec![0x32],
                }),
            }),
            issuer: Some(IdentityRef {
                identity_id: vec![0x33],
                identity_kind: Some("node".into()),
                key_hint: None,
            }),
            decision: Some("COMMAND_DECISION_COMMITTED".into()),
            decision_basis: Some(ObjectRef {
                object_id: vec![0x34],
                object_kind: None,
            }),
            reason_code: Some("ok".into()),
            effect_summary_object: Some(ObjectRef {
                object_id: vec![0x35],
                object_kind: Some("payload".into()),
            }),
            result_object: Some(ObjectRef {
                object_id: vec![0x36],
                object_kind: Some("payload".into()),
            }),
        };
        let proto: stream::CommandResultPayload = (&core).into();
        let back: CommandResultPayload = proto.into_core().expect("into core");
        assert_eq!(back, core);
    }
}
