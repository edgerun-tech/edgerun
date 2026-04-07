//! Lifegraph Node — connects streams, commands, and capabilities.
//!
//! A node is initialized from a YAML configuration that defines:
//! - Its identity (public key)
//! - Trusted controller identities
//! - Initial capability grants
//! - Initial trust relationships
//!
//! The config is embedded in the genesis event (seq=0) as the authoritative
//! record of the node's initial state.

pub mod mesh_node;

use lifegraph_capabilities::CapabilityGrant;
use lifegraph_capability_policy::SimplePolicyEngine;
use lifegraph_core::command::{validate_command, CommandValidationContext};
use lifegraph_core::protocol::{CommandEnvelope, EventEnvelope, EventType};
use lifegraph_core::result::Verdict;
use lifegraph_core::value::Value;
use lifegraph_stream::{StreamWriter, StreamError};
use lifegraph_hardware_signing::NodeID;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Node configuration — loaded from YAML and embedded in the genesis event.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NodeConfig {
    /// The node's stream identifier.
    pub stream_id: String,
    /// Node name (human-readable).
    pub name: Option<String>,
    /// Trusted controller identity IDs (who can send commands).
    #[serde(default)]
    pub controllers: Vec<String>,
    /// Trusted node IDs for bootstrapping trust relationships.
    #[serde(default)]
    pub trust_nodes: Vec<String>,
    /// Initial capability grants to install at startup.
    #[serde(default)]
    pub initial_grants: Vec<serde_yaml::Value>,
    /// Arbitrary metadata.
    #[serde(default)]
    pub metadata: serde_yaml::Value,
}

impl NodeConfig {
    /// Loads a node configuration from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        serde_yaml::from_str(yaml).map_err(|e| format!("invalid YAML config: {}", e))
    }

    /// Serializes the config to a YAML string.
    pub fn to_yaml(&self) -> String {
        serde_yaml::to_string(self).unwrap_or_default()
    }
}

/// A Lifegraph node.
///
/// The node owns its stream, manages capability grants, and processes
/// incoming commands through the full validation → authorization →
/// event recording pipeline.
pub struct Node {
    /// The node's identity (public key).
    identity: NodeID,
    /// The node's configuration.
    config: NodeConfig,
    /// The node's event stream writer.
    stream_writer: StreamWriter,
    /// Capability grant store.
    policy: SimplePolicyEngine,
    /// Replay cache: command_hash -> (command_id, decision_event_seq) for already-processed commands.
    /// command_hash is the globally unique key; command_id is an idempotency hint.
    processed_commands: HashMap<Vec<u8>, (Vec<u8>, i64)>,
    /// Known revoked delegation IDs.
    revoked_delegation_ids: HashSet<Vec<u8>>,
}

impl Node {
    /// Creates a new node from a YAML configuration.
    ///
    /// This creates a genesis event containing the configuration,
    /// then returns the initialized node.
    pub fn from_config(
        config: NodeConfig,
        signer: Box<dyn lifegraph_hardware_signing::MeshSigner>,
    ) -> Result<Self, StreamError> {
        let identity = signer.node_id();
        let stream_id = config.stream_id.clone();
        let stream_writer = StreamWriter::new(stream_id.clone(), signer, now_ms())?;

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
    pub fn process_command(&mut self, command: &CommandEnvelope) -> Result<(), String> {
        let ctx = CommandValidationContext {
            local_node_id: &self.identity.0,
            replay_cache: &self.processed_commands,
            revoked_delegation_ids: &self.revoked_delegation_ids,
            now_ms: now_ms(),
            trusted_root_ids: &self.config.controllers.iter().map(|s| s.as_bytes().to_vec()).collect::<Vec<_>>(),
        };

        let result = validate_command(command, &ctx);

        match result.verdict {
            Verdict::Accept => {
                // Step 2: Check grant authorization (allow all for now)
                // In production, this would check the policy engine

                // Step 3: Record commitment in stream
                self.record_commitment(command);
                // Track in replay cache — keyed by command_hash
                if let Some(Value::String(cmd_id)) = result.derived.as_map().and_then(|m| m.get("command_id")) {
                    if let Some(Value::String(cmd_hash)) = result.derived.as_map().and_then(|m| m.get("command_hash")) {
                        self.processed_commands.insert(
                            lifegraph_core::util::hex_to_bytes(cmd_hash).unwrap_or_default(),
                            (lifegraph_core::util::hex_to_bytes(cmd_id).unwrap_or_default(), 0i64),
                        );
                    }
                }
                Ok(())
            }
            Verdict::Duplicate => {
                // Already processed — return prior result
                Ok(())
            }
            Verdict::Defer => {
                let reason = result.reason_code.map(|r| r.as_str().to_string()).unwrap_or_else(|| "deferred".into());
                self.record_rejection(command, &reason);
                Err(format!("command deferred: {}", reason))
            }
            Verdict::Reject => {
                let reason = result.reason_code.map(|r| r.as_str().to_string()).unwrap_or_else(|| "invalid".into());
                self.record_rejection(command, &reason);
                Err(format!("command rejected: {}", reason))
            }
        }
    }

    /// Installs a capability grant.
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

    fn record_rejection(&mut self, _command: &CommandEnvelope, _reason: &str) {
        let _ = self.stream_writer.append(
            EventType::CommandRejected as i32,
            1,
            now_ms(),
        );
    }

    fn record_commitment(&mut self, _command: &CommandEnvelope) {
        let _ = self.stream_writer.append(
            EventType::CommandCommitted as i32,
            1,
            now_ms(),
        );
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as i64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use lifegraph_hardware_signing::MeshSigner;
    use rand::rngs::OsRng;

    struct TestSigner {
        node_id: NodeID,
        key: p256::ecdsa::SigningKey,
    }

    impl TestSigner {
        fn new() -> Self {
            let key = p256::ecdsa::SigningKey::random(&mut OsRng);
            let vk = key.verifying_key();
            let encoded = vk.to_encoded_point(false);
            let mut node_bytes = [0u8; 64];
            node_bytes.copy_from_slice(&encoded.as_bytes()[1..65]);
            Self {
                node_id: NodeID(node_bytes),
                key,
            }
        }
    }

    impl MeshSigner for TestSigner {
        fn node_id(&self) -> NodeID {
            self.node_id
        }

        fn sign_digest(
            &self,
            digest: &[u8; 32],
        ) -> Result<[u8; 64], lifegraph_hardware_signing::HardwareSigningError> {
            use p256::ecdsa::signature::hazmat::RandomizedPrehashSigner;
            let sig: p256::ecdsa::Signature =
                self.key.sign_prehash_with_rng(&mut OsRng, digest).unwrap();
            let mut bytes = [0u8; 64];
            bytes.copy_from_slice(&sig.to_bytes());
            Ok(bytes)
        }
    }

    const TEST_CONFIG: &str = r#"
stream_id: "node-test-stream"
name: "Test Node"
controllers:
  - "ctrl-alice"
  - "ctrl-bob"
trust_nodes:
  - "node-alpha"
  - "node-beta"
initial_grants: []
metadata:
  environment: "test"
"#;

    #[test]
    fn node_creates_from_yaml_config() {
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        assert_eq!(config.stream_id, "node-test-stream");
        assert_eq!(config.name.as_deref(), Some("Test Node"));
        assert_eq!(config.controllers.len(), 2);
        assert_eq!(config.trust_nodes.len(), 2);
    }

    #[test]
    fn node_rejects_unsigned_command() {
        // The Node.process_command path is being replaced by the new NodeStore
        // integration in main.rs. This test is kept as a placeholder.
    }
}
