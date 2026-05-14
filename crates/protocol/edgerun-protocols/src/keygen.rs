extern crate alloc;

use alloc::vec::Vec;

use crate::sign_p256::P256ProtocolSigner;
use edgerun_core::crypto::{self, SigningKey};

pub type NodeId = [u8; crypto::ECDSA_P256_PUBLIC_KEY_LEN];
pub type NodeSigningKey = SigningKey;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeygenError {
    InvalidKey,
    StoreFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedNodeIdentity {
    /// Node identity. This is the raw P-256 public key as x || y.
    pub node_id: NodeId,
}

pub trait GeneratedKeyStore {
    type Stored;

    /// Store freshly generated node signing key material.
    ///
    /// The private key is handed to the store during provisioning. The keygen
    /// crate does not provide a public import/export API; persistence, sealing,
    /// encryption, or hardware migration belong to the concrete store.
    fn store_generated_node_key(
        &mut self,
        node_id: &NodeId,
        signing_key: SigningKey,
    ) -> Result<Self::Stored, KeygenError>;
}

pub fn generate_node_signing_key() -> (SigningKey, GeneratedNodeIdentity) {
    let signing_key = edgerun_crypto::signing::p256_key();
    let node_id = node_id_from_signing_key(&signing_key);
    (signing_key, GeneratedNodeIdentity { node_id })
}

pub fn node_id_from_signing_key(signing_key: &SigningKey) -> NodeId {
    crypto::verifying_key_to_node_id(&signing_key.verifying_key())
}

pub fn node_signing_key_from_bytes(bytes: [u8; 32]) -> Result<SigningKey, KeygenError> {
    SigningKey::from_bytes(&bytes).map_err(|_| KeygenError::InvalidKey)
}

pub fn generate_node_identity_into<S: GeneratedKeyStore>(
    store: &mut S,
) -> Result<(GeneratedNodeIdentity, S::Stored), KeygenError> {
    let (signing_key, identity) = generate_node_signing_key();
    let stored = store.store_generated_node_key(&identity.node_id, signing_key)?;
    Ok((identity, stored))
}

/// In-memory generated node identity for tests and ephemeral containers.
///
/// This is not an export format. It is just the smallest in-process packaged
/// result for code that wants to immediately compose signer + verifier.
pub struct EphemeralNodeIdentity {
    pub node_id: NodeId,
    pub signer: P256ProtocolSigner,
}

pub fn generate_ephemeral_node_identity() -> EphemeralNodeIdentity {
    let (signing_key, identity) = generate_node_signing_key();
    EphemeralNodeIdentity {
        node_id: identity.node_id,
        signer: P256ProtocolSigner::new(signing_key),
    }
}

pub struct MemoryKeyStore {
    stored: Vec<(NodeId, P256ProtocolSigner)>,
}

impl MemoryKeyStore {
    pub const fn new() -> Self {
        Self { stored: Vec::new() }
    }

    pub fn len(&self) -> usize {
        self.stored.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stored.is_empty()
    }

    pub fn get(&self, node_id: &NodeId) -> Option<&P256ProtocolSigner> {
        self.stored
            .iter()
            .find(|(candidate, _)| candidate == node_id)
            .map(|(_, signer)| signer)
    }
}

impl Default for MemoryKeyStore {
    fn default() -> Self {
        Self::new()
    }
}

impl GeneratedKeyStore for MemoryKeyStore {
    type Stored = NodeId;

    fn store_generated_node_key(
        &mut self,
        node_id: &NodeId,
        signing_key: SigningKey,
    ) -> Result<Self::Stored, KeygenError> {
        self.stored
            .push((*node_id, P256ProtocolSigner::new(signing_key)));
        Ok(*node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::verify::{ProtocolFamily, ProtocolSignerRef, verify_event_envelope};
    use edgerun_core::protocol::{EventEnvelope, ProtocolRecord};

    #[test]
    fn generated_node_identity_is_public_key() {
        let node = generate_ephemeral_node_identity();
        assert_eq!(node.node_id.len(), crypto::ECDSA_P256_PUBLIC_KEY_LEN);
        assert_eq!(node.node_id, node.signer.raw_public_key());
    }

    #[test]
    fn generated_node_can_sign_and_verify_event() {
        let node = generate_ephemeral_node_identity();
        let mut event = EventEnvelope {
            envelope_version: 1,
            stream_id: node.node_id.to_vec(),
            seq: 1,
            event_version: 1,
            ..EventEnvelope::default()
        };
        let signed = node
            .signer
            .sign_record(
                &ProtocolRecord::EventEnvelope(event.clone()),
                ProtocolFamily::EventEnvelope,
            )
            .unwrap();
        event.signature = Some(signed.signature);

        let verified =
            verify_event_envelope(&event, ProtocolSignerRef::P256Raw64(&node.node_id)).unwrap();
        assert_eq!(verified.family, ProtocolFamily::EventEnvelope);
    }

    #[test]
    fn generated_key_can_be_handed_to_store() {
        let mut store = MemoryKeyStore::new();
        let (identity, stored) = generate_node_identity_into(&mut store).unwrap();
        assert_eq!(identity.node_id, stored);
        assert_eq!(store.len(), 1);
        assert!(store.get(&identity.node_id).is_some());
    }
}
