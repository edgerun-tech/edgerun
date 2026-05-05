#![no_std]

extern crate alloc;

use alloc::vec::Vec;

use edgerun_core::protocol::{EventEnvelope, EventType, ProtocolRecord};
use edgerun_keygen::{generate_node_signing_key, GeneratedKeyStore, KeygenError, NodeId};
use edgerun_sign_p256::P256ProtocolSigner;
use edgerun_verify::ProtocolFamily;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootstrapError {
    KeyStoreFailed,
    SignFailed,
}

impl From<KeygenError> for BootstrapError {
    fn from(_: KeygenError) -> Self {
        Self::KeyStoreFailed
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerBinding {
    None,
    PublicKey(NodeId),
}

impl Default for ControllerBinding {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BootstrapConfig {
    pub controller: ControllerBinding,
    pub node_label: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapResult<Stored> {
    /// Node identity. This is the node public key as raw P-256 x || y.
    pub node_id: NodeId,
    /// Optional controller binding requested during bootstrap.
    pub controller: ControllerBinding,
    /// Store-specific result returned after the private key was handed to storage.
    pub stored_key: Stored,
    /// The signed seq-0 node genesis event.
    pub genesis_event: EventEnvelope,
}

pub fn unsigned_node_genesis_event(node_id: &NodeId) -> EventEnvelope {
    EventEnvelope {
        envelope_version: 1,
        stream_id: node_id.to_vec(),
        seq: 0,
        prev_event_hash: None,
        event_type: EventType::NodeGenesis as i32,
        event_version: 1,
        recorded_at: None,
        effective_at: None,
        payload_object: None,
        related_events: Vec::new(),
        related_commands: Vec::new(),
        related_objects: Vec::new(),
        related_delegations: Vec::new(),
        related_revocations: Vec::new(),
        event_metadata: None,
        signature: None,
    }
}

pub fn bootstrap_new_node<S: GeneratedKeyStore>(
    store: &mut S,
    config: BootstrapConfig,
) -> Result<BootstrapResult<S::Stored>, BootstrapError> {
    let (signing_key, identity) = generate_node_signing_key();
    let signer = P256ProtocolSigner::new(signing_key.clone());

    let mut genesis_event = unsigned_node_genesis_event(&identity.node_id);
    let signed = signer
        .sign_record(
            &ProtocolRecord::EventEnvelope(genesis_event.clone()),
            ProtocolFamily::EventEnvelope,
        )
        .map_err(|_| BootstrapError::SignFailed)?;
    genesis_event.signature = Some(signed.signature);

    let stored_key = store.store_generated_node_key(&identity.node_id, signing_key)?;

    Ok(BootstrapResult {
        node_id: identity.node_id,
        controller: config.controller,
        stored_key,
        genesis_event,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_keygen::MemoryKeyStore;
    use edgerun_verify::{verify_event_envelope, ProtocolSignerRef};

    #[test]
    fn bootstrap_generates_stores_and_signs_genesis() {
        let mut store = MemoryKeyStore::new();
        let result = bootstrap_new_node(&mut store, BootstrapConfig::default()).unwrap();

        assert_eq!(store.len(), 1);
        assert_eq!(result.node_id, result.genesis_event.stream_id.as_slice());
        assert_eq!(result.genesis_event.seq, 0);
        assert_eq!(result.genesis_event.event_type, EventType::NodeGenesis as i32);
        assert!(result.genesis_event.signature.is_some());

        let verified = verify_event_envelope(
            &result.genesis_event,
            ProtocolSignerRef::P256Raw64(&result.node_id),
        )
        .unwrap();
        assert_eq!(verified.family, ProtocolFamily::EventEnvelope);
    }

    #[test]
    fn bootstrap_can_carry_controller_public_key() {
        let mut node_store = MemoryKeyStore::new();
        let controller = edgerun_keygen::generate_ephemeral_node_identity();
        let result = bootstrap_new_node(
            &mut node_store,
            BootstrapConfig {
                controller: ControllerBinding::PublicKey(controller.node_id),
                node_label: Some(b"test-node".to_vec()),
            },
        )
        .unwrap();

        assert_eq!(
            result.controller,
            ControllerBinding::PublicKey(controller.node_id)
        );
    }
}
