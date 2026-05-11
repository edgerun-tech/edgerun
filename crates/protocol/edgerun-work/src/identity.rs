use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::codec::blake3_hash;
use crate::protocol::{NodeId, NodeIdentity, PublicKey};

pub fn derive_node_id(public_key: &PublicKey, role: u16) -> NodeId {
    let mut input = Vec::with_capacity(34);
    input.extend_from_slice(public_key);
    input.extend_from_slice(&role.to_be_bytes());
    blake3_hash(&input)
}

pub fn verify_node_identity(identity: &NodeIdentity) -> bool {
    identity.role != 0 && identity.node_id == derive_node_id(&identity.public_key, identity.role)
}

pub fn node_identity_from_key(key: &Ed25519SigningKey, role: u16) -> NodeIdentity {
    let mut public_key = [0u8; 32];
    public_key.copy_from_slice(key.verifying_key().as_bytes());
    NodeIdentity {
        node_id: derive_node_id(&public_key, role),
        role,
        public_key,
    }
}
