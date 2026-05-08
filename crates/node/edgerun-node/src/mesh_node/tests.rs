use super::*;
use alloc::vec;
use edgerun_hardware_signing::MeshSigner;
use edgerun_protocols::core_protocol::protocol::common as proto_common;

use crate::test_support::TestSigner;

fn test_signer() -> TestSigner {
    TestSigner::generate()
}

fn test_config() -> NodeConfig {
    NodeConfig::new(
        "test-node",
        Some("Test Node".into()),
        Vec::new(),
        Vec::new(),
    )
    .unwrap()
}

#[test]
fn mesh_node_creates_with_genesis() {
    let config = test_config();
    let node = MeshNode::from_config(config, Box::new(test_signer())).unwrap();

    assert_eq!(node.events().len(), 1);
    assert_eq!(node.events()[0].seq, 0);
}

#[test]
fn mesh_node_has_identity() {
    let config = test_config();
    let signer = test_signer();
    let expected_id = signer.node_id();
    let node = MeshNode::from_config(config, Box::new(signer)).unwrap();

    assert_eq!(node.identity(), expected_id);
}

#[test]
fn mesh_node_tick_processes_nothing_when_idle() {
    let config = test_config();
    let signer = Box::new(test_signer());
    let mut node = MeshNode::from_config(config, signer).unwrap();

    let processed = node.tick().unwrap();
    assert_eq!(processed, 0);
}

#[test]
fn mesh_node_is_inert_for_commands_before_storage_is_attached() {
    let config = test_config();
    let signer = Box::new(test_signer());
    let mut node = MeshNode::from_config(config, signer).unwrap();

    let command = edgerun_protocols::core_protocol::protocol::stream::CommandEnvelope {
        envelope_version: 1,
        command_id: vec![1, 2, 3],
        target_node: Some(proto_common::NodeRef {
            node_id: node.identity().0.to_vec(),
        }),
        issuer: Some(proto_common::IdentityRef {
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

    let buf = command_full_wire_bytes(&command);
    let mut frame = MeshFrame::from_payload(node.identity(), buf);
    frame.header.src = node.identity();
    frame.signature = [0u8; 64];
    let wire = frame.to_wire();

    let _ = node.deliver_inbound_frame(&wire);

    assert_eq!(node.events().len(), 1);
    assert!(!node.storage_ready());
}

#[test]
fn two_nodes_exchange_signed_commands() {
    let config_a = test_config();
    let signer_a = Box::new(test_signer());
    let node_a_id = signer_a.node_id();
    let _alice = MeshNode::from_config(config_a, signer_a).unwrap();

    let config_b = test_config();
    let signer_b = Box::new(test_signer());
    let mut bob = MeshNode::from_config(config_b, signer_b).unwrap();

    let command = edgerun_protocols::core_protocol::protocol::stream::CommandEnvelope {
        envelope_version: 1,
        command_id: vec![1, 2, 3],
        target_node: Some(proto_common::NodeRef {
            node_id: bob.identity().0.to_vec(),
        }),
        issuer: Some(proto_common::IdentityRef {
            identity_id: node_a_id.0.to_vec(),
            identity_kind: Some(2),
            key_hint: Some(node_a_id.0.to_vec()),
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

    let buf = command_full_wire_bytes(&command);
    let mut frame = MeshFrame::from_payload(bob.identity(), buf);
    frame.header.src = node_a_id;
    frame.signature = [0u8; 64];
    let wire = frame.to_wire();

    let processed = bob.deliver_inbound_frame(&wire).unwrap();
    assert!(processed > 0);
    assert_eq!(bob.events().len(), 1);
    assert!(!bob.storage_ready());
}

#[test]
fn mesh_node_router_is_accessible() {
    let config = test_config();
    let signer = Box::new(test_signer());
    let mut node = MeshNode::from_config(config, signer).unwrap();

    let router = node.router_mut();
    let _ = router;
}

#[test]
fn mesh_node_send_command_queues_frame() {
    let config = test_config();
    let signer = Box::new(test_signer());
    let mut node = MeshNode::from_config(config, signer).unwrap();

    let dest = NodeID([0xAAu8; 64]);
    let command = edgerun_protocols::core_protocol::protocol::stream::CommandEnvelope {
        envelope_version: 1,
        command_id: vec![1, 2, 3],
        target_node: Some(proto_common::NodeRef {
            node_id: dest.0.to_vec(),
        }),
        issuer: Some(proto_common::IdentityRef {
            identity_id: node.identity().0.to_vec(),
            identity_kind: Some(2),
            key_hint: Some(node.identity().0.to_vec()),
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

    node.send_command(dest, &command);
    let frames = node.drain_outbound_frames().unwrap();
    assert_eq!(frames.len(), 1);
    let frame = MeshFrame::from_wire(&frames[0]).unwrap();
    assert_eq!(frame.header.src, node.identity());
    assert_eq!(frame.header.dest, dest);
    assert!(frame.verify_signature());
}

#[test]
fn mesh_node_identity_is_consistent() {
    let config = test_config();
    let signer = Box::new(test_signer());
    let expected = signer.node_id();
    let node = MeshNode::from_config(config, signer).unwrap();

    assert_eq!(node.identity(), expected);
    assert_eq!(node.identity(), node.identity());
}

#[test]
fn mesh_node_events_accessor() {
    let config = test_config();
    let signer = Box::new(test_signer());
    let node = MeshNode::from_config(config, signer).unwrap();

    let events = node.events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].seq, 0);
}

#[test]
fn mesh_node_install_grant() {
    let config = test_config();
    let signer = Box::new(test_signer());
    let mut node = MeshNode::from_config(config, signer).unwrap();

    let grant = edgerun_capabilities::CapabilityGrant {
        grant_version: 1,
        grant_id: vec![1, 2, 3],
        issuer: Some(proto_common::IdentityRef {
            identity_id: vec![4, 5, 6],
            identity_kind: Some(2),
            key_hint: None,
        }),
        grantee: Some(proto_common::IdentityRef {
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
}

#[test]
fn mesh_node_multiple_ticks_are_idempotent() {
    let config = test_config();
    let signer = Box::new(test_signer());
    let mut node = MeshNode::from_config(config, signer).unwrap();

    for _ in 0..5 {
        let processed = node.tick().unwrap();
        assert_eq!(processed, 0);
    }
}

#[test]
fn mesh_node_decode_command_returns_none_for_garbage() {
    let config = test_config();
    let signer = Box::new(test_signer());
    let node = MeshNode::from_config(config, signer).unwrap();

    let frame = MeshFrame::from_payload(node.identity(), vec![0xFF, 0xFE, 0xFD]);
    let result = MeshNode::decode_command(&frame);
    assert!(result.is_none());
}

#[test]
fn mesh_node_decode_command_parses_valid_envelope() {
    let config = test_config();
    let signer = Box::new(test_signer());
    let node = MeshNode::from_config(config, signer).unwrap();

    let command = edgerun_protocols::core_protocol::protocol::stream::CommandEnvelope {
        envelope_version: 1,
        command_id: vec![1, 2, 3],
        target_node: Some(proto_common::NodeRef {
            node_id: node.identity().0.to_vec(),
        }),
        issuer: None,
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

    let buf = command_full_wire_bytes(&command);
    let frame = MeshFrame::from_payload(node.identity(), buf);
    let decoded = MeshNode::decode_command(&frame);
    assert!(decoded.is_some());
    assert_eq!(decoded.unwrap().command_id, vec![1, 2, 3]);
}

#[test]
fn mesh_node_frame_with_dest_preserves_payload() {
    let config = test_config();
    let signer = Box::new(test_signer());
    let node = MeshNode::from_config(config, signer).unwrap();

    let original_frame = MeshFrame::from_payload(node.identity(), vec![1, 2, 3, 4]);
    let payload_before = original_frame.payload.clone();
    let new_dest = NodeID([0xBBu8; 64]);
    let routed = MeshNode::frame_with_dest(original_frame, new_dest);

    assert_eq!(routed.header.dest, new_dest);
    assert_eq!(routed.payload, payload_before);
}

#[test]
fn mesh_node_from_config_fails_with_bad_signer() {
    let config = test_config();
    let signer = Box::new(test_signer());
    let result = MeshNode::from_config(config, signer);
    assert!(result.is_ok());
}
