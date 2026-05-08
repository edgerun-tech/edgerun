//! Native node bootstrap payloads.
//!
//! Bootstrap authority is established by typed records that are archived through
//! the rkyv wire boundary and then referenced by signed stream events.

use alloc::string::String;
use alloc::vec::Vec;

use edgerun_protocols::core_protocol::protocol::{
    CompiledBootstrapPolicyPayload, IdentityRef, NodeGenesisPayload,
};

pub fn node_identity_ref(identity_id: Vec<u8>) -> IdentityRef {
    IdentityRef {
        identity_id: identity_id.clone(),
        identity_kind: Some(2),
        key_hint: Some(identity_id),
    }
}

pub fn controller_identity_ref(identity_id: Vec<u8>) -> IdentityRef {
    IdentityRef {
        identity_id: identity_id.clone(),
        identity_kind: None,
        key_hint: Some(identity_id),
    }
}

pub fn node_genesis_payload(
    node_id: Vec<u8>,
    initial_controllers: Vec<Vec<u8>>,
) -> NodeGenesisPayload {
    NodeGenesisPayload {
        payload_version: 1,
        node_id: node_id.clone(),
        primary_node_identity: Some(node_identity_ref(node_id)),
        initial_controllers: initial_controllers
            .into_iter()
            .map(controller_identity_ref)
            .collect(),
        initial_policy_object: None,
        bootstrap_records: Vec::new(),
        assurance_claims: Vec::new(),
        node_roles: Vec::new(),
        genesis_metadata: None,
    }
}

pub fn compiled_bootstrap_policy_payload(
    node_label: String,
    stream_id: Vec<u8>,
    controller_id: Vec<u8>,
    bootstrap_relays: Vec<String>,
) -> CompiledBootstrapPolicyPayload {
    CompiledBootstrapPolicyPayload {
        payload_version: 1,
        node_label,
        stream_id,
        controller_id,
        bootstrap_relays,
    }
}

pub fn archive_node_genesis_payload(payload: &NodeGenesisPayload) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(payload)
        .expect("node genesis payload must serialize through rkyv")
        .into_vec()
}

pub fn archive_bootstrap_policy_payload(payload: &CompiledBootstrapPolicyPayload) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(payload)
        .expect("bootstrap policy payload must serialize through rkyv")
        .into_vec()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    #[test]
    fn node_genesis_payload_archives_through_rkyv() {
        let payload = node_genesis_payload(vec![1; 64], vec![vec![2; 64]]);
        let bytes = archive_node_genesis_payload(&payload);
        let decoded = edgerun_protocols::wire::from_bytes::<
            NodeGenesisPayload,
            edgerun_protocols::wire::WireError,
        >(&bytes)
        .unwrap();

        assert_eq!(decoded.node_id, vec![1; 64]);
        assert_eq!(decoded.initial_controllers.len(), 1);
        assert_eq!(decoded.initial_controllers[0].identity_id, vec![2; 64]);
    }

    #[test]
    fn bootstrap_policy_payload_archives_through_rkyv() {
        let payload = compiled_bootstrap_policy_payload(
            "node-a".into(),
            vec![1; 64],
            vec![2; 64],
            vec!["mesh://relay".into()],
        );
        let bytes = archive_bootstrap_policy_payload(&payload);
        let decoded = edgerun_protocols::wire::from_bytes::<
            CompiledBootstrapPolicyPayload,
            edgerun_protocols::wire::WireError,
        >(&bytes)
        .unwrap();

        assert_eq!(decoded.node_label, "node-a");
        assert_eq!(decoded.controller_id, vec![2; 64]);
        assert_eq!(decoded.bootstrap_relays, vec![String::from("mesh://relay")]);
    }
}
