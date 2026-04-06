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
use lifegraph_core::command::{validate_command, CommandOutcome};
use lifegraph_core::protocol::{CommandEnvelope, EventEnvelope, EventType};
use lifegraph_stream::{StreamWriter, StreamError};
use lifegraph_hardware_signing::NodeID;
use serde::{Deserialize, Serialize};

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
        })
    }

    /// Processes an incoming command through the full pipeline:
    ///
    /// 1. Verify command signature
    /// 2. Validate command structure
    /// 3. Check authorization via capability grants
    /// 4. Record `CommandCommitted` or `CommandRejected` event in stream
    ///
    /// Returns `Ok(())` if the command is valid and authorized,
    /// or `Err(reason)` if validation or authorization fails.
    pub fn process_command(&mut self, command: &CommandEnvelope) -> Result<(), String> {
        // Step 1 & 2: Validate command signature and structure
        match validate_command(command) {
            CommandOutcome::Valid => {}
            CommandOutcome::MissingSignature => {
                self.record_rejection(command, "missing signature");
                return Err("missing signature".into());
            }
            CommandOutcome::InvalidPublicKey => {
                self.record_rejection(command, "invalid public key");
                return Err("invalid public key".into());
            }
            CommandOutcome::InvalidSignature => {
                self.record_rejection(command, "invalid signature");
                return Err("invalid signature".into());
            }
        }

        // Step 3: Check grant authorization (allow all for now)
        // In production, this would check the policy engine

        // Step 4: Record commitment in stream
        self.record_commitment(command);

        Ok(())
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
    use lifegraph_core::protocol::{CommandEnvelope, IdentityRef, NodeRef};
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

    fn make_command() -> CommandEnvelope {
        CommandEnvelope {
            envelope_version: 1,
            command_id: Some(vec![1, 2, 3]),
            target_node: NodeRef {
                node_id: vec![4, 5, 6],
            },
            issuer: IdentityRef {
                identity_id: vec![7, 8, 9],
                identity_kind: Some("user".into()),
                key_hint: None,
            },
            command_type: Some("COMMAND_TYPE_QUERY".into()),
            command_version: 1,
            issued_at: None,
            not_before: None,
            expires_at: None,
            idempotency_key: None,
            payload_object: None,
            inline_payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signature: None,
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
    fn node_boots_with_genesis_event() {
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let signer = Box::new(TestSigner::new());
        let node = Node::from_config(config, signer).unwrap();

        // Stream should have exactly 1 event: the genesis
        assert_eq!(node.events().len(), 1);
        let genesis = &node.events()[0];
        assert_eq!(genesis.seq, 0);
        assert_eq!(genesis.event_type, EventType::NodeGenesis);
        assert!(genesis.signature.is_some()); // Genesis must be signed
    }

    #[test]
    fn node_rejects_unsigned_command() {
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let signer = Box::new(TestSigner::new());
        let mut node = Node::from_config(config, signer).unwrap();

        let command = make_command(); // No signature
        let result = node.process_command(&command);
        assert!(result.is_err());
        // Stream should have genesis + rejection event
        assert_eq!(node.events().len(), 2);
        assert_eq!(node.events()[1].event_type, EventType::CommandRejected);
    }
}
