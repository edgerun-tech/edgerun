use edgerun_crypto::Ed25519SigningKey;

use crate::protocol::{NodeId, NodeIdentity, PublicKey};

pub fn derive_node_id(public_key: &PublicKey, _role: u16) -> NodeId {
    *public_key
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{NODE_ROLE_COMPUTE, NODE_ROLE_STORAGE};

    #[test]
    fn node_id_is_public_key_and_roles_are_capabilities() {
        let key = Ed25519SigningKey::from_bytes(&[42u8; 32]);
        let storage = node_identity_from_key(&key, NODE_ROLE_STORAGE);
        let compute = node_identity_from_key(&key, NODE_ROLE_COMPUTE);

        assert_eq!(storage.node_id, storage.public_key);
        assert_eq!(compute.node_id, compute.public_key);
        assert_eq!(storage.node_id, compute.node_id);
        assert_ne!(storage.role, compute.role);
        assert!(verify_node_identity(&storage));
        assert!(verify_node_identity(&compute));
    }
}
