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
pub mod command_dispatch;
pub mod command_dispatch_event;
pub mod command_dispatch_result;
pub mod error;
#[cfg(feature = "std")]
pub mod hardware;
#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "http")]
pub mod http_client;
pub mod logging;
pub mod mesh_node;
pub mod network;
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
pub mod storage;
pub mod stream_append;
#[cfg(feature = "tls")]
pub mod tls;

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
    /// Command dispatch is storage-backed. Callers must append/dispatch through
    /// `command_dispatch::dispatch_command` with a configured `NodeStore` so
    /// every decision starts from durable event-log state.
    ///
    /// This in-memory `Node` API is intentionally inert: accepting commands here
    /// would let callers mutate authoritative state before storage is ready.
    pub fn process_command(&mut self, _command: &CommandEnvelope) -> Result<(), NodeError> {
        Err(NodeError::Storage(
            "command dispatch requires configured NodeStore".into(),
        ))
    }

    #[cfg(test)]
    fn process_command_in_memory_for_tests(
        &mut self,
        command: &CommandEnvelope,
    ) -> Result<(), NodeError> {
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
mod tests;
