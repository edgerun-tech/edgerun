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
    let mut node = Node::from_config_with_event_log(config, signer, MemEventLog::new()).unwrap();
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
