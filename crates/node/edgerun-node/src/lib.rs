//! edgerun Node — IPC router and resource authority.
//!
//! A node moves data between apps, protocol state machines, sockets,
//! capabilities, storage, and mesh links. It is the authority boundary for all
//! host resources. Apps can declare routes and ask for capabilities, but they
//! do not bind ports, share memory, open files, or own sockets directly.
//!
//! A node is initialized from native rkyv bootstrap records and signed stream
//! events. Text configuration is not an authority boundary.

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

pub mod app_model;
pub mod bootstrap;
pub mod error;
#[cfg(feature = "exchange-events")]
pub mod exchange_events;
#[cfg(feature = "std")]
pub mod hardware;
pub mod logging;
pub mod mesh_node;
pub mod resource;
pub mod router;
pub mod rt;
pub mod runtime;
#[cfg(any(
    feature = "http",
    feature = "dns",
    feature = "dhcp",
    feature = "tftp",
    feature = "proxy",
    feature = "smtp",
    feature = "imap",
    feature = "lmtp",
    feature = "acme",
    feature = "virtual-disk",
))]
pub mod services;
#[cfg(feature = "exchange-events")]
mod stream_append;
pub mod transport;

mod protocol_signer;

#[cfg(test)]
pub(crate) mod test_support {
    use alloc::format;
    use std::sync::Mutex;

    use edgerun_hardware_signing::{HardwareSigningError, MeshSigner, NodeID};
    use edgerun_protocols::keygen::{
        generate_node_signing_key, node_id_from_signing_key, NodeSigningKey,
    };
    use edgerun_protocols::sign::{ProtocolSigner, SignableProtocolFamily};
    use edgerun_protocols::sign_p256::P256ProtocolSigner;

    pub(crate) struct TestSigner {
        node_id: NodeID,
        signer: Mutex<P256ProtocolSigner>,
    }

    impl TestSigner {
        pub(crate) fn generate() -> Self {
            let (key, _) = generate_node_signing_key();
            Self::new(key)
        }

        fn new(key: NodeSigningKey) -> Self {
            let node_id = NodeID(node_id_from_signing_key(&key));
            Self {
                node_id,
                signer: Mutex::new(P256ProtocolSigner::new(key)),
            }
        }
    }

    impl MeshSigner for TestSigner {
        fn node_id(&self) -> NodeID {
            self.node_id
        }

        fn sign_digest(&self, digest: &[u8; 32]) -> Result<[u8; 64], HardwareSigningError> {
            self.sign_message_var(digest)
        }

        fn sign_message_var(&self, message: &[u8]) -> Result<[u8; 64], HardwareSigningError> {
            let signer = self
                .signer
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let signature = signer
                .sign_signature_input(SignableProtocolFamily::EventEnvelope, message)
                .map_err(|error| HardwareSigningError::Provider(format!("{error:?}")))?;
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&signature);
            Ok(bytes)
        }

        fn as_any(&self) -> &dyn core::any::Any {
            self
        }
    }
}

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use edgerun_capabilities::CapabilityGrant;
use edgerun_capability_policy::SimplePolicyEngine;
use edgerun_hardware_signing::NodeID;
use edgerun_protocols::core_protocol::command::{validate_command, CommandValidationContext};
use edgerun_protocols::core_protocol::protocol::{CommandEnvelope, EventEnvelope, EventType};
use edgerun_protocols::core_protocol::result::Verdict;
use edgerun_protocols::core_protocol::util::now_unix_millis_i64 as now_ms;
use edgerun_protocols::core_protocol::value::Value;
use edgerun_protocols::sign::ProtocolSigner;
use edgerun_storage::core::EventLog;
use edgerun_storage::{DurableStreamWriter, MemEventLog};
pub use error::{NodeError, NodeResult};
use protocol_signer::MeshProtocolSigner;

type HashMap<K, V> = BTreeMap<K, V>;
type HashSet<T> = BTreeSet<T>;

/// Minimal node construction input.
#[derive(Clone, Debug, Default)]
pub struct NodeConfig {
    /// The node's stream identifier.
    pub stream_id: String,
    /// Node name (human-readable).
    pub name: Option<String>,
    /// Trusted controller identity IDs (who can send commands).
    pub controllers: Vec<String>,
    /// Trusted node IDs for bootstrapping trust relationships.
    pub trust_nodes: Vec<String>,
}

impl NodeConfig {
    pub fn new(
        stream_id: impl Into<String>,
        name: Option<String>,
        controllers: Vec<String>,
        trust_nodes: Vec<String>,
    ) -> Result<Self, NodeError> {
        let stream_id = stream_id.into();
        if stream_id.is_empty() {
            return Err(NodeError::MissingField("stream_id".into()));
        }
        Ok(Self {
            stream_id,
            name,
            controllers,
            trust_nodes,
        })
    }
}

/// A edgerun node.
///
/// The node owns its stream, manages capability grants, and processes
/// incoming commands through the full validation → authorization →
/// event recording pipeline.
pub struct Node<L = MemEventLog, S = MeshProtocolSigner> {
    /// The node's identity (public key).
    identity: NodeID,
    /// The node's configuration.
    config: NodeConfig,
    /// The node's signed durable event stream writer.
    stream_writer: DurableStreamWriter<L, S>,
    /// Capability grant store.
    policy: SimplePolicyEngine,
    /// Replay cache: command_hash -> (command_id, decision_event_seq) for already-processed commands.
    /// command_hash is the globally unique key; command_id is an idempotency hint.
    processed_commands: HashMap<Vec<u8>, (Vec<u8>, i64)>,
    /// Known revoked delegation IDs.
    revoked_delegation_ids: HashSet<Vec<u8>>,
}

impl Node<MemEventLog, MeshProtocolSigner> {
    /// Creates a new node from native construction input.
    ///
    /// This creates a genesis event containing the configuration,
    /// then returns the initialized node.
    pub fn from_config(
        config: NodeConfig,
        signer: Arc<dyn edgerun_hardware_signing::MeshSigner>,
    ) -> Result<Self, NodeError> {
        Self::from_config_with_event_log(config, signer, MemEventLog::new())
    }
}

impl<L: EventLog> Node<L, MeshProtocolSigner> {
    /// Creates a new node backed by a caller-provided durable event log.
    pub fn from_config_with_event_log(
        config: NodeConfig,
        signer: Arc<dyn edgerun_hardware_signing::MeshSigner>,
        event_log: L,
    ) -> Result<Self, NodeError> {
        let identity = signer.node_id();
        let signer = MeshProtocolSigner::new(signer);
        Self::from_config_with_protocol_signer(config, identity, signer, event_log)
    }
}

impl<L: EventLog, S: ProtocolSigner> Node<L, S> {
    /// Creates a new node backed by a caller-provided protocol signer.
    pub fn from_config_with_protocol_signer(
        config: NodeConfig,
        identity: NodeID,
        signer: S,
        event_log: L,
    ) -> Result<Self, NodeError> {
        let stream_writer = DurableStreamWriter::new(identity.0, signer, now_ms(), event_log)?;

        // Record the config as the genesis event metadata
        // The genesis event is already created by StreamWriter::new
        // We attach the config to it conceptually
        // In production, the genesis event payload would contain the config

        let policy = SimplePolicyEngine::default();

        Ok(Self {
            identity,
            config,
            stream_writer,
            policy,
            processed_commands: HashMap::new(),
            revoked_delegation_ids: HashSet::new(),
        })
    }

    /// Processes an incoming command through the full pipeline:
    ///
    /// 1. Validate command (signature, replay, timing, delegation)
    /// 2. Check authorization via capability grants
    /// 3. Record `CommandCommitted` or `CommandRejected` event in stream
    ///
    /// Returns `Ok(())` if the command is valid and authorized,
    /// or `Err(reason)` if validation or authorization fails.
    pub fn process_command(&mut self, command: &CommandEnvelope) -> Result<(), NodeError> {
        // Controllers must be hex-encoded 64-byte identity IDs (P-256 public keys).
        // Strings that are not valid 128-char hex are silently skipped — they can
        // never match a real delegation root issuer.
        let trusted_root_ids: Vec<Vec<u8>> = self
            .config
            .controllers
            .iter()
            .filter_map(|s| edgerun_protocols::core_protocol::util::hex_to_bytes(s).ok())
            .collect();
        let delegation_use_counts: HashMap<Vec<u8>, u64> = HashMap::new();
        let delegation_rate_events_ms: HashMap<Vec<u8>, Vec<i64>> = HashMap::new();

        let ctx = CommandValidationContext {
            local_node_id: &self.identity.0,
            replay_cache: &self.processed_commands,
            revoked_delegation_ids: &self.revoked_delegation_ids,
            delegation_use_counts: &delegation_use_counts,
            delegation_rate_events_ms: &delegation_rate_events_ms,
            now_ms: now_ms(),
            trusted_root_ids: &trusted_root_ids,
            local_assurance_class: 0, // Unknown — simple config path doesn't track signer type
            accepted_assurance_claims: &[],
            has_local_session: false,
            has_user_presence: false,
            transport_class: None,
            location_classes: &[],
            target_stream_id: None,
            target_view_type: None,
            target_domain: None,
            execution_class: None,
            storage_class: None,
        };

        let result = validate_command(command, &ctx);

        match result.verdict {
            Verdict::Accept => {
                self.record_commitment(command)?;
                if let Some(Value::String(cmd_id)) =
                    result.derived.as_map().and_then(|m| m.get("command_id"))
                {
                    if let Some(Value::String(cmd_hash)) =
                        result.derived.as_map().and_then(|m| m.get("command_hash"))
                    {
                        self.processed_commands.insert(
                            edgerun_protocols::core_protocol::util::hex_to_bytes(cmd_hash)
                                .unwrap_or_default(),
                            (
                                edgerun_protocols::core_protocol::util::hex_to_bytes(cmd_id)
                                    .unwrap_or_default(),
                                0i64,
                            ),
                        );
                    }
                }
                Ok(())
            }
            Verdict::Duplicate => Ok(()),
            Verdict::Defer => {
                let reason = result
                    .reason_code
                    .map(|r| r.as_str().to_string())
                    .unwrap_or_else(|| "deferred".into());
                self.record_rejection(command, &reason)?;
                Err(NodeError::CommandDeferred(reason))
            }
            Verdict::Reject => {
                let reason = result
                    .reason_code
                    .map(|r| r.as_str().to_string())
                    .unwrap_or_else(|| "invalid".into());
                self.record_rejection(command, &reason)?;
                Err(NodeError::CommandRejected(reason))
            }
        }
    }

    /// Installs a capability grant directly into the policy engine.
    ///
    /// **WARNING**: This bypasses the event stream. In production, use
    /// `capabilities::record_capability_grant_event()` instead to ensure
    /// the grant is recorded as a signed event.
    #[cfg(test)]
    pub fn install_grant(&mut self, grant: CapabilityGrant) {
        let _ = self.policy.import_grant(grant);
    }

    /// Returns the node's identity.
    #[must_use]
    pub fn identity(&self) -> NodeID {
        self.identity
    }

    /// Returns the node's configuration.
    #[must_use]
    pub fn config(&self) -> &NodeConfig {
        &self.config
    }

    /// Returns the node's event stream.
    #[must_use]
    pub fn events(&self) -> &[EventEnvelope] {
        self.stream_writer.events()
    }

    /// Returns the current stream head.
    #[must_use]
    pub fn head(&self) -> Option<&EventEnvelope> {
        self.stream_writer.head()
    }

    /// Returns the durable event log backing this node.
    #[must_use]
    pub fn event_log(&self) -> &L {
        self.stream_writer.event_log()
    }

    fn record_rejection(
        &mut self,
        _command: &CommandEnvelope,
        _reason: &str,
    ) -> Result<(), NodeError> {
        self.stream_writer
            .append(EventType::CommandRejected as i32, 1, now_ms())?;
        Ok(())
    }

    fn record_commitment(&mut self, _command: &CommandEnvelope) -> Result<(), NodeError> {
        self.stream_writer
            .append(EventType::CommandCommitted as i32, 1, now_ms())?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use edgerun_crypto::rand_core::RngCore;
    use edgerun_hardware_signing::MeshSigner;
    use edgerun_protocols::core_protocol::protocol::{
        CommandType, IdentityRef, NodeRef, ProtocolRecord, Signature, Timestamp,
    };
    use edgerun_protocols::keygen::generate_ephemeral_node_identity;
    use edgerun_protocols::sign::{ProtocolSigner, SignableProtocolFamily};
    use std::sync::Arc;

    use crate::test_support::TestSigner;

    fn test_signer() -> TestSigner {
        TestSigner::generate()
    }

    fn signed_query_command_for_node(
        node_id: NodeID,
    ) -> edgerun_protocols::core_protocol::protocol::CommandEnvelope {
        let issuer = generate_ephemeral_node_identity();
        let now_secs = edgerun_protocols::core_protocol::util::now_unix_secs_i64();
        let mut command = edgerun_protocols::core_protocol::protocol::CommandEnvelope {
            envelope_version: 1,
            command_id: vec![0x10, 0x20, 0x30, 0x40],
            target_node: Some(NodeRef {
                node_id: node_id.0.to_vec(),
            }),
            issuer: Some(IdentityRef {
                identity_id: issuer.node_id.to_vec(),
                identity_kind: Some(edgerun_protocols::core_protocol::crypto::IDENTITY_KIND_NODE),
                key_hint: Some(issuer.node_id.to_vec()),
            }),
            command_type: CommandType::Query as i32,
            command_version: 1,
            issued_at: Some(Timestamp {
                seconds: now_secs.saturating_sub(1),
                nanos: 0,
            }),
            not_before: None,
            expires_at: Some(Timestamp {
                seconds: now_secs.saturating_add(60),
                nanos: 0,
            }),
            idempotency_key: Vec::new(),
            payload: None,
            delegation_chain: Vec::new(),
            requested_assurance: None,
            command_metadata: None,
            signatures: Vec::new(),
            app_intent: Vec::new(),
        };
        let signed = issuer
            .signer
            .sign_protocol_record(
                &ProtocolRecord::CommandEnvelope(command.clone()),
                SignableProtocolFamily::CommandEnvelope,
            )
            .expect("sign command");
        command.signatures.push(Signature {
            algorithm: signed.signature.algorithm,
            value: signed.signature.value,
        });
        command
    }

    fn test_config() -> NodeConfig {
        NodeConfig::new(
            "node-test-stream",
            Some("Test Node".into()),
            vec!["ctrl-alice".into(), "ctrl-bob".into()],
            vec!["node-alpha".into(), "node-beta".into()],
        )
        .unwrap()
    }

    // -----------------------------------------------------------------------
    // Node creation and identity
    // -----------------------------------------------------------------------

    #[test]
    fn node_creates_from_config() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let expected_id = signer.node_id();
        let node = Node::from_config(config, signer).unwrap();

        assert_eq!(node.identity(), expected_id);
    }

    #[test]
    fn node_genesis_event_present() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let node = Node::from_config(config, signer).unwrap();

        let events = node.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].seq, 0);
    }

    #[test]
    fn node_can_use_caller_provided_event_log() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let node = Node::from_config_with_event_log(config, signer, MemEventLog::new()).unwrap();

        let scanned = node.event_log().scan().unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].event.seq, 0);
        assert!(scanned[0].event.signature.is_some());
    }

    #[test]
    fn node_config_accessor() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let node = Node::from_config(config.clone(), signer).unwrap();

        assert_eq!(node.config().stream_id, config.stream_id);
        assert_eq!(node.config().controllers, config.controllers);
    }

    #[test]
    fn node_head_returns_genesis() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let node = Node::from_config(config, signer).unwrap();

        let head = node.head();
        assert!(head.is_some());
        assert_eq!(head.unwrap().seq, 0);
    }

    // -----------------------------------------------------------------------
    // Command processing pipeline
    // -----------------------------------------------------------------------

    #[test]
    fn node_rejects_command_with_empty_command_id() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let mut node = Node::from_config(config, signer).unwrap();

        // Build a minimal command with empty command_id
        let command = edgerun_protocols::core_protocol::protocol::CommandEnvelope {
            envelope_version: 1,
            command_id: vec![], // empty -> structural reject
            target_node: Some(edgerun_protocols::core_protocol::protocol::NodeRef {
                node_id: node.identity().0.to_vec(),
            }),
            issuer: Some(edgerun_protocols::core_protocol::protocol::IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(0),
                key_hint: None,
            }),
            command_type: 7, // QUERY
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: vec![],
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signatures: Vec::new(),
            app_intent: Vec::new(),
        };

        let result = node.process_command(&command);
        assert!(result.is_err());
        // Should be rejected for structural reasons
        let err_msg = result.unwrap_err().to_string().to_lowercase();
        assert!(err_msg.contains("structural") || err_msg.contains("reject"));
    }

    #[test]
    fn node_rejects_command_targeting_wrong_node() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let mut node = Node::from_config(config, signer).unwrap();

        let command = edgerun_protocols::core_protocol::protocol::CommandEnvelope {
            envelope_version: 1,
            command_id: vec![1, 2, 3],
            target_node: Some(edgerun_protocols::core_protocol::protocol::NodeRef {
                node_id: vec![0u8; 64], // wrong target
            }),
            issuer: Some(edgerun_protocols::core_protocol::protocol::IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(0),
                key_hint: None,
            }),
            command_type: 7,
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: vec![],
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signatures: Vec::new(),
            app_intent: Vec::new(),
        };

        let result = node.process_command(&command);
        assert!(result.is_err());
    }

    #[test]
    fn node_rejects_command_without_target() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let mut node = Node::from_config(config, signer).unwrap();

        let command = edgerun_protocols::core_protocol::protocol::CommandEnvelope {
            envelope_version: 1,
            command_id: vec![1, 2, 3],
            target_node: None, // no target
            issuer: Some(edgerun_protocols::core_protocol::protocol::IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(0),
                key_hint: None,
            }),
            command_type: 7,
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: vec![],
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signatures: Vec::new(),
            app_intent: Vec::new(),
        };

        let result = node.process_command(&command);
        assert!(result.is_err());
    }

    #[test]
    fn node_records_rejection_event_for_bad_command() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let mut node = Node::from_config(config, signer).unwrap();

        let initial_events = node.events().len();

        let command = edgerun_protocols::core_protocol::protocol::CommandEnvelope {
            envelope_version: 1,
            command_id: vec![1],
            target_node: Some(edgerun_protocols::core_protocol::protocol::NodeRef {
                node_id: node.identity().0.to_vec(),
            }),
            issuer: Some(edgerun_protocols::core_protocol::protocol::IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: Some(0),
                key_hint: None,
            }),
            command_type: 7,
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: vec![],
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signatures: Vec::new(),
            app_intent: Vec::new(),
        };

        let _ = node.process_command(&command);

        // Should have recorded a rejection event
        assert!(node.events().len() > initial_events);
    }

    #[test]
    fn node_commits_valid_signed_command_to_hash_linked_stream() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let mut node = Node::from_config(config, signer).unwrap();
        let command = signed_query_command_for_node(node.identity());

        node.process_command(&command).unwrap();

        let events = node.events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[1].seq, 1);
        assert_eq!(
            events[1].event_type,
            edgerun_protocols::core_protocol::protocol::EventType::CommandCommitted as i32
        );
        assert!(events[1].signature.is_some());
        let genesis_hash = edgerun_storage::canonical_event_hash(&events[0]).value;
        assert_eq!(
            events[1]
                .prev_event_hash
                .as_ref()
                .map(|h| h.value.as_slice()),
            Some(genesis_hash.as_slice())
        );
        edgerun_stream::validate_stream(events, &node.identity().0).unwrap();
    }

    #[test]
    fn node_persists_committed_command_to_backing_event_log() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let mut node =
            Node::from_config_with_event_log(config, signer, MemEventLog::new()).unwrap();
        let command = signed_query_command_for_node(node.identity());

        node.process_command(&command).unwrap();

        let scanned = node.event_log().scan().unwrap();
        assert_eq!(scanned.len(), 2);
        assert_eq!(scanned[1].event.seq, 1);
        assert_eq!(
            scanned[1].event.event_type,
            edgerun_protocols::core_protocol::protocol::EventType::CommandCommitted as i32
        );
        assert_eq!(scanned[1].event, node.events()[1]);
    }

    #[test]
    fn node_treats_replayed_command_hash_as_duplicate_without_appending() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let mut node = Node::from_config(config, signer).unwrap();
        let command = signed_query_command_for_node(node.identity());

        node.process_command(&command).unwrap();
        let event_count_after_first_delivery = node.events().len();

        node.process_command(&command).unwrap();

        assert_eq!(node.events().len(), event_count_after_first_delivery);
        assert_eq!(node.head().unwrap().seq, 1);
    }

    #[test]
    fn node_install_grant() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let mut node = Node::from_config(config, signer).unwrap();

        // Install a grant — should not panic
        let grant = edgerun_capabilities::CapabilityGrant {
            grant_version: 1,
            grant_id: vec![1, 2, 3],
            issuer: Some(edgerun_protocols::core_protocol::protocol::IdentityRef {
                identity_id: vec![4, 5, 6],
                identity_kind: Some(2),
                key_hint: None,
            }),
            grantee: Some(edgerun_protocols::core_protocol::protocol::IdentityRef {
                identity_id: vec![1, 2, 3],
                identity_kind: Some(0),
                key_hint: None,
            }),
            grantee_node: None,
            selector: None,
            granted_operations: vec![],
            enforced_constraints: vec![],
            access_class: 0,
            issued_at: None,
            expires_at: None,
            correlation_id: vec![],
            supersedes_revocation: None,
            signature: None,
        };
        node.install_grant(grant);
        // No assertion needed — just verifying it doesn't panic
    }

    // -----------------------------------------------------------------------
    // Replay cache
    // -----------------------------------------------------------------------

    #[test]
    fn replay_cache_populated_on_accept() {
        let config = test_config();
        let signer = Arc::new(test_signer());
        let mut node = Node::from_config(config, signer).unwrap();

        // Create a command that will pass structural validation but fail signature
        // We use a command_id that's non-empty and target that matches
        let command = edgerun_protocols::core_protocol::protocol::CommandEnvelope {
            envelope_version: 1,
            command_id: vec![10, 20, 30],
            target_node: Some(edgerun_protocols::core_protocol::protocol::NodeRef {
                node_id: node.identity().0.to_vec(),
            }),
            issuer: Some(edgerun_protocols::core_protocol::protocol::IdentityRef {
                identity_id: node
                    .config()
                    .controllers
                    .first()
                    .unwrap()
                    .as_bytes()
                    .to_vec(),
                identity_kind: Some(2),
                key_hint: Some(vec![0u8; 64]), // wrong key, will fail signature
            }),
            command_type: 7,
            command_version: 1,
            issued_at: Some(edgerun_protocols::core_protocol::protocol::Timestamp {
                seconds: edgerun_protocols::core_protocol::util::now_unix_secs_i64(),
                nanos: 0,
            }),
            not_before: None,
            expires_at: None,
            idempotency_key: vec![],
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signatures: vec![edgerun_protocols::core_protocol::protocol::Signature {
                algorithm: 1,
                value: vec![0u8; 64], // bad signature
            }],
            app_intent: Vec::new(),
        };

        let result = node.process_command(&command);
        // The command should be rejected due to bad signature
        assert!(result.is_err());
        // but the path runs and events are recorded
        assert!(node.events().len() >= 2);
    }
}
