//! Codex-facing bridge records archived through Edgerun's rkyv wire boundary.
//!
//! This crate is deliberately small. It gives local Codex integrations a
//! concrete Edgerun-backed context record without adding a second wire format
//! or a compatibility path.

use std::sync::Arc;

use edgerun_capabilities::CapabilityDescriptor;
use edgerun_capabilities::CapabilityError;
use edgerun_capabilities::CapabilityGrant;
use edgerun_capabilities::CapabilityInvocation;
use edgerun_capabilities::CapabilityRequest;
use edgerun_capability_policy::PolicyContext;
use edgerun_capability_policy::PolicyDecision;
use edgerun_capability_policy::PolicyEngine;
use edgerun_capability_policy::SimplePolicyEngine;
use edgerun_core::protocol::EventType;
use edgerun_core::protocol::IdentityKind;
use edgerun_core::protocol::IdentityRef;
use edgerun_core::protocol::NodeRef;
use edgerun_hardware_signing::MeshSigner;
use edgerun_hardware_signing::NodeID;
use edgerun_storage::core::AppendReceipt;
use edgerun_storage::core::EventLog;
use edgerun_storage::DurableStreamWriter;
use edgerun_storage::StorageError;
use edgerun_wire::Archive;
use edgerun_wire::Deserialize;
use edgerun_wire::Serialize;
use edgerun_wire::WireError;

pub const WIRE_PROTOCOL: &str = edgerun_wire::WIRE_PROTOCOL;

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct CodexBoostRecord {
    pub schema_version: u16,
    pub kind: CodexBoostRecordKind,
    pub source: String,
    pub body: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub enum CodexBoostRecordKind {
    MemorySummary,
    RepoContext,
    ToolObservation,
}

impl CodexBoostRecord {
    pub fn repo_context(source: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            schema_version: 1,
            kind: CodexBoostRecordKind::RepoContext,
            source: source.into(),
            body: body.into(),
        }
    }
}

pub fn archive_record(record: &CodexBoostRecord) -> Result<Vec<u8>, WireError> {
    Ok(edgerun_wire::to_bytes::<WireError>(record)?.into_vec())
}

pub fn decode_record(bytes: &[u8]) -> Result<CodexBoostRecord, WireError> {
    edgerun_wire::from_bytes::<CodexBoostRecord, WireError>(bytes)
}

pub fn access_record(bytes: &[u8]) -> Result<&edgerun_wire::Archived<CodexBoostRecord>, WireError> {
    edgerun_wire::access::<edgerun_wire::Archived<CodexBoostRecord>, WireError>(bytes)
}

#[derive(Clone, Debug, PartialEq)]
pub struct AgentIdentity {
    node_id: NodeID,
    identity_ref: IdentityRef,
    node_ref: NodeRef,
}

impl AgentIdentity {
    pub fn from_node_id(node_id: NodeID) -> Self {
        let identity_id = node_id.0.to_vec();
        Self {
            node_id,
            identity_ref: IdentityRef {
                identity_id: identity_id.clone(),
                identity_kind: Some(IdentityKind::Agent as i32),
                key_hint: Some(identity_id[..8].to_vec()),
            },
            node_ref: NodeRef {
                node_id: node_id.0.to_vec(),
            },
        }
    }

    #[must_use]
    pub fn node_id(&self) -> NodeID {
        self.node_id
    }

    #[must_use]
    pub fn identity_ref(&self) -> &IdentityRef {
        &self.identity_ref
    }

    #[must_use]
    pub fn node_ref(&self) -> &NodeRef {
        &self.node_ref
    }
}

#[derive(Debug)]
pub enum AgentRuntimeError {
    Storage(StorageError),
    Capability(CapabilityError),
    CapabilityDenied(&'static str),
    CapabilityRequiresInteraction(&'static str),
}

impl core::fmt::Display for AgentRuntimeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Storage(err) => write!(f, "{err}"),
            Self::Capability(err) => write!(f, "{err}"),
            Self::CapabilityDenied(reason) => write!(f, "capability denied: {reason}"),
            Self::CapabilityRequiresInteraction(reason) => {
                write!(f, "capability requires interaction: {reason}")
            }
        }
    }
}

impl std::error::Error for AgentRuntimeError {}

impl From<StorageError> for AgentRuntimeError {
    fn from(value: StorageError) -> Self {
        Self::Storage(value)
    }
}

impl From<CapabilityError> for AgentRuntimeError {
    fn from(value: CapabilityError) -> Self {
        Self::Capability(value)
    }
}

pub struct AgentRuntime<L> {
    identity: AgentIdentity,
    stream: DurableStreamWriter<L>,
    policy: SimplePolicyEngine,
}

impl<L: EventLog> AgentRuntime<L> {
    pub fn new(
        signer: Arc<dyn MeshSigner>,
        recorded_at_ms: i64,
        event_log: L,
    ) -> Result<Self, AgentRuntimeError> {
        let identity = AgentIdentity::from_node_id(signer.node_id());
        let stream =
            DurableStreamWriter::new(signer.node_id().0, signer, recorded_at_ms, event_log)?;
        let policy = SimplePolicyEngine::new(Some(identity.identity_ref.clone()));
        Ok(Self {
            identity,
            stream,
            policy,
        })
    }

    #[must_use]
    pub fn identity(&self) -> &AgentIdentity {
        &self.identity
    }

    #[must_use]
    pub fn stream(&self) -> &DurableStreamWriter<L> {
        &self.stream
    }

    #[must_use]
    pub fn policy(&self) -> &SimplePolicyEngine {
        &self.policy
    }

    pub fn record_action_started(
        &mut self,
        recorded_at_ms: i64,
    ) -> Result<AppendReceipt, AgentRuntimeError> {
        self.append_event(EventType::ActionStarted, recorded_at_ms)
            .map_err(Into::into)
    }

    pub fn record_action_completed(
        &mut self,
        recorded_at_ms: i64,
    ) -> Result<AppendReceipt, AgentRuntimeError> {
        self.append_event(EventType::ActionCompleted, recorded_at_ms)
            .map_err(Into::into)
    }

    pub fn record_action_failed(
        &mut self,
        recorded_at_ms: i64,
    ) -> Result<AppendReceipt, AgentRuntimeError> {
        self.append_event(EventType::ActionFailed, recorded_at_ms)
            .map_err(Into::into)
    }

    pub fn evaluate_capability_request(
        &mut self,
        descriptor: &CapabilityDescriptor,
        mut request: CapabilityRequest,
        context: &PolicyContext,
        recorded_at_ms: i64,
    ) -> Result<CapabilityGrant, AgentRuntimeError> {
        if request.requester.is_none() {
            request.requester = Some(self.identity.identity_ref.clone());
        }
        if request.requester_node.is_none() {
            request.requester_node = Some(self.identity.node_ref.clone());
        }

        match self
            .policy
            .evaluate_request(descriptor, &request, context)?
        {
            PolicyDecision::Allow { grant } => {
                self.append_event(EventType::CapabilityGranted, recorded_at_ms)?;
                Ok(*grant)
            }
            PolicyDecision::Deny { reason } => {
                self.append_event(EventType::ActionFailed, recorded_at_ms)?;
                Err(AgentRuntimeError::CapabilityDenied(reason))
            }
            PolicyDecision::RequireInteraction { reason } => {
                self.append_event(EventType::ActionFailed, recorded_at_ms)?;
                Err(AgentRuntimeError::CapabilityRequiresInteraction(reason))
            }
        }
    }

    pub fn authorize_capability_invocation(
        &mut self,
        invocation: &CapabilityInvocation,
        context: &PolicyContext,
        recorded_at_ms: i64,
    ) -> Result<AppendReceipt, AgentRuntimeError> {
        self.policy.authorize_invocation(invocation, context)?;
        self.append_event(EventType::ActionStarted, recorded_at_ms)
            .map_err(Into::into)
    }

    fn append_event(
        &mut self,
        event_type: EventType,
        recorded_at_ms: i64,
    ) -> Result<AppendReceipt, StorageError> {
        self.stream.append(event_type as i32, 1, recorded_at_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_capabilities::capability_descriptor;
    use edgerun_capabilities::CapabilityAccessClass;
    use edgerun_capabilities::CapabilityEventKind;
    use edgerun_capabilities::CapabilityModality;
    use edgerun_capabilities::CapabilityOperation;
    use edgerun_capabilities::CapabilityRole;
    use edgerun_capabilities::CapabilitySelector;
    use edgerun_hardware_signing::HardwareSigningError;
    use edgerun_storage::MemEventLog;

    #[test]
    fn archives_codex_context_through_edgerun_wire() {
        let record = CodexBoostRecord::repo_context(
            "AGENTS.md",
            "internal Edgerun wire protocol is rkyv only",
        );

        let bytes = archive_record(&record).expect("archive record");
        let archived = access_record(&bytes).expect("access archived record");
        let decoded = decode_record(&bytes).expect("decode record");

        assert_eq!(archived.schema_version, record.schema_version);
        assert_eq!(decoded, record);
        assert_eq!(WIRE_PROTOCOL, "rkyv");
    }

    #[test]
    fn agent_runtime_owns_identity_and_records_signed_agency_events() {
        let signer = Arc::new(FixedSigner::new(7));
        let mut runtime =
            AgentRuntime::new(signer.clone(), 1_000, MemEventLog::new()).expect("create runtime");

        assert_eq!(runtime.identity().node_id(), signer.node_id());
        assert_eq!(
            runtime.identity().identity_ref().identity_kind,
            Some(IdentityKind::Agent as i32)
        );

        let started = runtime
            .record_action_started(1_001)
            .expect("record action start");
        let completed = runtime
            .record_action_completed(1_002)
            .expect("record action completion");
        let scanned = runtime.stream().event_log().scan().expect("scan events");

        assert_eq!(started.seq, 1);
        assert_eq!(completed.seq, 2);
        assert_eq!(scanned.len(), 3);
        assert_eq!(scanned[0].event.event_type, EventType::NodeGenesis as i32);
        assert_eq!(scanned[1].event.event_type, EventType::ActionStarted as i32);
        assert_eq!(
            scanned[2].event.event_type,
            EventType::ActionCompleted as i32
        );
        assert!(scanned.iter().all(|event| event.event.signature.is_some()));
    }

    #[test]
    fn agent_runtime_turns_allowed_capability_request_into_grant_event() {
        let signer = Arc::new(FixedSigner::new(9));
        let mut runtime =
            AgentRuntime::new(signer, 1_000, MemEventLog::new()).expect("create runtime");
        let descriptor = capability_descriptor(
            "codex-tool",
            "local-shell",
            CapabilityRole::Execution,
            &[CapabilityModality::Computational],
            &[CapabilityEventKind::State],
            &[CapabilityOperation::Invoke],
            Vec::new(),
        );
        let request = CapabilityRequest {
            request_version: 1,
            request_id: b"request-1".to_vec(),
            selector: Some(CapabilitySelector {
                role: CapabilityRole::Execution as i32,
                modalities: vec![CapabilityModality::Computational as i32],
                event_kinds: vec![CapabilityEventKind::State as i32],
                operations: vec![CapabilityOperation::Invoke as i32],
                access_class: CapabilityAccessClass::Derived as i32,
                provider_instance_id: "local-shell".to_string(),
                ..Default::default()
            }),
            requested_operations: vec![CapabilityOperation::Invoke as i32],
            purpose: "exercise agent grant path".to_string(),
            ..Default::default()
        };
        let context = PolicyContext {
            requester: Some(runtime.identity().identity_ref().clone()),
            requester_node: Some(runtime.identity().node_ref().clone()),
            ..Default::default()
        };

        let grant = runtime
            .evaluate_capability_request(&descriptor, request, &context, 1_001)
            .expect("grant capability");
        let invocation = CapabilityInvocation {
            invocation_version: 1,
            invocation_id: b"invoke-1".to_vec(),
            grant_id: grant.grant_id.clone(),
            invoker: Some(runtime.identity().identity_ref().clone()),
            operation: CapabilityOperation::Invoke as i32,
            requested_access_class: CapabilityAccessClass::Derived as i32,
            ..Default::default()
        };
        let receipt = runtime
            .authorize_capability_invocation(&invocation, &context, 1_002)
            .expect("authorize invocation");
        let scanned = runtime.stream().event_log().scan().expect("scan events");

        assert_eq!(receipt.seq, 2);
        assert_eq!(
            grant.grantee,
            Some(runtime.identity().identity_ref().clone())
        );
        assert_eq!(
            scanned[1].event.event_type,
            EventType::CapabilityGranted as i32
        );
        assert_eq!(scanned[2].event.event_type, EventType::ActionStarted as i32);
    }

    struct FixedSigner {
        node_id: NodeID,
    }

    impl FixedSigner {
        fn new(seed: u8) -> Self {
            Self {
                node_id: NodeID([seed; 64]),
            }
        }
    }

    impl MeshSigner for FixedSigner {
        fn node_id(&self) -> NodeID {
            self.node_id
        }

        fn sign_digest(&self, digest: &[u8; 32]) -> Result<[u8; 64], HardwareSigningError> {
            let mut signature = [0u8; 64];
            signature[..32].copy_from_slice(digest);
            signature[32..].copy_from_slice(&self.node_id.0[..32]);
            Ok(signature)
        }

        fn sign_message_var(&self, message: &[u8]) -> Result<[u8; 64], HardwareSigningError> {
            let digest = edgerun_core::crypto::sha256(message);
            let mut digest_array = [0u8; 32];
            digest_array.copy_from_slice(&digest);
            self.sign_digest(&digest_array)
        }
    }
}
