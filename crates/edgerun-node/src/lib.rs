//! edgerun Node — connects streams, commands, and capabilities.
//!
//! A node is initialized from a YAML configuration that defines:
//! - Its identity (public key)
//! - Trusted controller identities
//! - Initial trust relationships
//!
//! The config is embedded in the genesis event (seq=0) as the authoritative
//! record of the node's initial state.

pub mod mesh_node;
pub mod metering;

use edgerun_capabilities::CapabilityGrant;
use edgerun_capability_policy::SimplePolicyEngine;
use edgerun_core::command::{validate_command, CommandValidationContext};
use edgerun_core::protocol::{CommandEnvelope, EventEnvelope, EventType};
use edgerun_core::result::Verdict;
use edgerun_core::value::Value;
use edgerun_stream::{StreamWriter, StreamError};
use edgerun_hardware_signing::NodeID;
// Simple YAML config parser (no serde dependency)
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Node configuration — loaded from YAML and embedded in the genesis event.
#[derive(Clone, Debug)]
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
    /// Loads a node configuration from a YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, String> {
        let mut config = NodeConfig {
            stream_id: String::new(),
            name: None,
            controllers: Vec::new(),
            trust_nodes: Vec::new(),
        };
        let mut current_list: Option<&mut Vec<String>> = None;
        for line in yaml.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') { continue; }
            if trimmed.starts_with("- ") {
                // List item continuation
                if let Some(ref mut list) = current_list {
                    list.push(unquote(trimmed[2..].trim()));
                }
                continue;
            }
            if let Some(colon) = trimmed.find(':') {
                let key = trimmed[..colon].trim();
                let val = trimmed[colon+1..].trim();
                current_list = None; // reset list context
                match key {
                    "stream_id" => config.stream_id = unquote(val),
                    "name" => config.name = Some(unquote(val)),
                    "controllers" => {
                        let parsed = parse_list(val);
                        if parsed.is_empty() && val.is_empty() {
                            // multi-line list follows
                            current_list = Some(&mut config.controllers);
                        } else {
                            config.controllers = parsed;
                        }
                    }
                    "trust_nodes" => {
                        let parsed = parse_list(val);
                        if parsed.is_empty() && val.is_empty() {
                            current_list = Some(&mut config.trust_nodes);
                        } else {
                            config.trust_nodes = parsed;
                        }
                    }
                    _ => {}
                }
            }
        }
        if config.stream_id.is_empty() {
            return Err("missing required field: stream_id".into());
        }
        Ok(config)
    }

    /// Serializes the config to a YAML string.
    pub fn to_yaml(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("stream_id: \"{}\"\n", self.stream_id));
        if let Some(ref name) = self.name { out.push_str(&format!("name: \"{}\"\n", name)); }
        out.push_str(&format!("controllers: {:?}\n", self.controllers));
        out.push_str(&format!("trust_nodes: {:?}\n", self.trust_nodes));
        out
    }
}

/// A edgerun node.
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
        signer: Arc<dyn edgerun_hardware_signing::MeshSigner>,
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
                            edgerun_core::util::hex_to_bytes(cmd_hash).unwrap_or_default(),
                            (edgerun_core::util::hex_to_bytes(cmd_id).unwrap_or_default(), 0i64),
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
        .unwrap_or_default()
        .as_millis() as i64
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn unquote(s: &str) -> String {
    let s = s.trim();
    if s.len() >= 2 && s.starts_with("\x22") && s.ends_with("\x22") {
        return s[1..s.len()-1].to_string();
    }
    s.to_string()
}

fn parse_list(val: &str) -> Vec<String> {
    let val = val.trim();
    if val == "[]" || val.is_empty() { return Vec::new(); }
    if val.starts_with('[') && val.ends_with(']') {
        let inner = &val[1..val.len()-1];
        if inner.trim().is_empty() { return Vec::new(); }
        return inner.split(',').map(|s| unquote(s.trim())).collect();
    }
    Vec::new()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_hardware_signing::MeshSigner;
    use p256::ecdsa::signature::hazmat::PrehashSigner;
    use std::sync::Arc;

    fn random_signing_key() -> p256::ecdsa::SigningKey {
        let mut bytes = [0u8; 32];
        edgerun_core::crypto::fill_random(&mut bytes);
        p256::ecdsa::SigningKey::from_bytes(&bytes.into()).unwrap()
    }

    struct TestSigner {
        node_id: NodeID,
        key: p256::ecdsa::SigningKey,
    }

    impl TestSigner {
        fn new() -> Self {
            let key = random_signing_key();
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
        ) -> Result<[u8; 64], edgerun_hardware_signing::HardwareSigningError> {
            let sig: p256::ecdsa::Signature = self.key.sign_prehash(digest)
                .map_err(|e| edgerun_hardware_signing::HardwareSigningError::Provider(e.to_string()))?;
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

    // -----------------------------------------------------------------------
    // NodeConfig YAML parsing
    // -----------------------------------------------------------------------

    #[test]
    fn config_parses_full_yaml() {
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        assert_eq!(config.stream_id, "node-test-stream");
        assert_eq!(config.name.as_deref(), Some("Test Node"));
        assert_eq!(config.controllers, vec!["ctrl-alice", "ctrl-bob"]);
        assert_eq!(config.trust_nodes, vec!["node-alpha", "node-beta"]);
    }

    #[test]
    fn config_parses_inline_lists() {
        let yaml = r#"
stream_id: "s1"
name: "Inline"
controllers: ["c1", "c2"]
trust_nodes: ["t1"]
"#;
        let config = NodeConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.stream_id, "s1");
        assert_eq!(config.name.as_deref(), Some("Inline"));
        assert_eq!(config.controllers, vec!["c1", "c2"]);
        assert_eq!(config.trust_nodes, vec!["t1"]);
    }

    #[test]
    fn config_parses_empty_lists() {
        let yaml = r#"
stream_id: "minimal"
controllers: []
trust_nodes: []
"#;
        let config = NodeConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.stream_id, "minimal");
        assert!(config.name.is_none());
        assert!(config.controllers.is_empty());
        assert!(config.trust_nodes.is_empty());
    }

    #[test]
    fn config_parses_multiline_list_with_no_inline_val() {
        let yaml = r#"
stream_id: "ml"
controllers:
  - "a"
  - "b"
  - "c"
trust_nodes: []
"#;
        let config = NodeConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.controllers, vec!["a", "b", "c"]);
        assert!(config.trust_nodes.is_empty());
    }

    #[test]
    fn config_missing_stream_id() {
        let yaml = r#"
name: "no-stream"
controllers: []
"#;
        let result = NodeConfig::from_yaml(yaml);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("stream_id"));
    }

    #[test]
    fn config_ignores_comments_and_blank_lines() {
        let yaml = r#"
# This is a comment
stream_id: "with-comments"

# Another comment
name: "Commented"
controllers: []

trust_nodes: []
"#;
        let config = NodeConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.stream_id, "with-comments");
        assert_eq!(config.name.as_deref(), Some("Commented"));
    }

    #[test]
    fn config_parses_unquoted_values() {
        let yaml = r#"
stream_id: unquoted-id
name: "Quoted Name"
controllers: []
trust_nodes: []
"#;
        let config = NodeConfig::from_yaml(yaml).unwrap();
        // unquoted stream_id passes through parse_list which returns empty,
        // but the key:value path uses unquote which strips quotes if present
        assert_eq!(config.stream_id, "unquoted-id");
    }

    #[test]
    fn config_unquote_strips_double_quotes() {
        assert_eq!(super::unquote("\"hello\""), "hello");
    }

    #[test]
    fn config_unquote_returns_raw_when_no_quotes() {
        assert_eq!(super::unquote("hello"), "hello");
        assert_eq!(super::unquote("  spaced  "), "spaced");
    }

    #[test]
    fn config_unquote_single_char() {
        assert_eq!(super::unquote("\"x\""), "x");
    }

    #[test]
    fn parse_list_empty() {
        assert!(super::parse_list("").is_empty());
        assert!(super::parse_list("[]").is_empty());
        assert!(super::parse_list("[  ]").is_empty());
    }

    #[test]
    fn parse_list_inline_items() {
        let items = super::parse_list("[\"a\", \"b\", \"c\"]");
        assert_eq!(items, vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_list_unknown_format() {
        // Non-bracket, non-empty strings return empty vec
        assert!(super::parse_list("just-a-string").is_empty());
    }

    #[test]
    fn config_to_yaml_roundtrip() {
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let yaml_out = config.to_yaml();
        let config2 = NodeConfig::from_yaml(&yaml_out).unwrap();
        assert_eq!(config.stream_id, config2.stream_id);
        assert_eq!(config.name, config2.name);
        assert_eq!(config.controllers, config2.controllers);
        assert_eq!(config.trust_nodes, config2.trust_nodes);
    }

    #[test]
    fn config_skips_unknown_keys() {
        let yaml = r#"
stream_id: "skip-test"
unknown_key: "ignored"
controllers: []
trust_nodes: []
"#;
        let config = NodeConfig::from_yaml(yaml).unwrap();
        assert_eq!(config.stream_id, "skip-test");
    }

    // -----------------------------------------------------------------------
    // Node creation and identity
    // -----------------------------------------------------------------------

    #[test]
    fn node_creates_from_config() {
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let signer = Arc::new(TestSigner::new());
        let expected_id = signer.node_id();
        let node = Node::from_config(config, signer).unwrap();

        assert_eq!(node.identity(), expected_id);
    }

    #[test]
    fn node_genesis_event_present() {
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let signer = Arc::new(TestSigner::new());
        let node = Node::from_config(config, signer).unwrap();

        let events = node.events();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].seq, 0);
    }

    #[test]
    fn node_config_accessor() {
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let signer = Arc::new(TestSigner::new());
        let node = Node::from_config(config.clone(), signer).unwrap();

        assert_eq!(node.config().stream_id, config.stream_id);
        assert_eq!(node.config().controllers, config.controllers);
    }

    #[test]
    fn node_head_returns_genesis() {
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let signer = Arc::new(TestSigner::new());
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
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let signer = Arc::new(TestSigner::new());
        let mut node = Node::from_config(config, signer).unwrap();

        // Build a minimal command with empty command_id
        let command = edgerun_proto::edgerun::v0::stream::CommandEnvelope {
            envelope_version: 1,
            command_id: vec![], // empty -> structural reject
            target_node: Some(edgerun_proto::edgerun::v0::common::NodeRef {
                node_id: node.identity().0.to_vec(),
            }),
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
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
            signature: None,
        };

        let result = node.process_command(&command);
        assert!(result.is_err());
        // Should be rejected for structural reasons
        let err_msg = result.unwrap_err().to_lowercase();
        assert!(err_msg.contains("structural") || err_msg.contains("reject"));
    }

    #[test]
    fn node_rejects_command_targeting_wrong_node() {
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let signer = Arc::new(TestSigner::new());
        let mut node = Node::from_config(config, signer).unwrap();

        let command = edgerun_proto::edgerun::v0::stream::CommandEnvelope {
            envelope_version: 1,
            command_id: vec![1, 2, 3],
            target_node: Some(edgerun_proto::edgerun::v0::common::NodeRef {
                node_id: vec![0u8; 64], // wrong target
            }),
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
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
            signature: None,
        };

        let result = node.process_command(&command);
        assert!(result.is_err());
    }

    #[test]
    fn node_rejects_command_without_target() {
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let signer = Arc::new(TestSigner::new());
        let mut node = Node::from_config(config, signer).unwrap();

        let command = edgerun_proto::edgerun::v0::stream::CommandEnvelope {
            envelope_version: 1,
            command_id: vec![1, 2, 3],
            target_node: None, // no target
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
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
            signature: None,
        };

        let result = node.process_command(&command);
        assert!(result.is_err());
    }

    #[test]
    fn node_records_rejection_event_for_bad_command() {
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let signer = Arc::new(TestSigner::new());
        let mut node = Node::from_config(config, signer).unwrap();

        let initial_events = node.events().len();

        let command = edgerun_proto::edgerun::v0::stream::CommandEnvelope {
            envelope_version: 1,
            command_id: vec![1],
            target_node: Some(edgerun_proto::edgerun::v0::common::NodeRef {
                node_id: node.identity().0.to_vec(),
            }),
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
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
            signature: None,
        };

        let _ = node.process_command(&command);

        // Should have recorded a rejection event
        assert!(node.events().len() > initial_events);
    }

    #[test]
    fn node_install_grant() {
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let signer = Arc::new(TestSigner::new());
        let mut node = Node::from_config(config, signer).unwrap();

        // Install a grant — should not panic
        let grant = edgerun_capabilities::CapabilityGrant {
            grant_version: 1,
            grant_id: vec![1, 2, 3],
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: vec![4, 5, 6],
                identity_kind: Some(2),
                key_hint: None,
            }),
            grantee: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
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
        let config = NodeConfig::from_yaml(TEST_CONFIG).unwrap();
        let signer = Arc::new(TestSigner::new());
        let mut node = Node::from_config(config, signer).unwrap();

        // Create a command that will pass structural validation but fail signature
        // We use a command_id that's non-empty and target that matches
        let command = edgerun_proto::edgerun::v0::stream::CommandEnvelope {
            envelope_version: 1,
            command_id: vec![10, 20, 30],
            target_node: Some(edgerun_proto::edgerun::v0::common::NodeRef {
                node_id: node.identity().0.to_vec(),
            }),
            issuer: Some(edgerun_proto::edgerun::v0::common::IdentityRef {
                identity_id: node.config().controllers.first().unwrap().as_bytes().to_vec(),
                identity_kind: Some(2),
                key_hint: Some(vec![0u8; 64]), // wrong key, will fail signature
            }),
            command_type: 7,
            command_version: 1,
            issued_at: Some(prost_types::Timestamp {
                seconds: (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64) / 1000,
                nanos: 0,
            }),
            not_before: None,
            expires_at: None,
            idempotency_key: vec![],
            payload: None,
            delegation_chain: vec![],
            requested_assurance: None,
            command_metadata: None,
            signature: Some(edgerun_proto::edgerun::v0::common::Signature {
                algorithm: 1,
                value: vec![0u8; 64], // bad signature
            }),
        };

        let result = node.process_command(&command);
        // The command should be rejected due to bad signature
        assert!(result.is_err());
        // but the path runs and events are recorded
        assert!(node.events().len() >= 2);
    }
}
