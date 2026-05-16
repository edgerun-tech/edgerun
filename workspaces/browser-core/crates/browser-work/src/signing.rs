use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::codec::blake3_hash;
use crate::preimage::PreimageBuilder;
use crate::protocol::{
    Hash, NodeIdentity, PublicKey, WorkAdmission, WorkSignature,
    SIGNATURE_ALGORITHM_SOLANA_ED25519, WORK_WIRE_ABI_VERSION,
};

const WORK_ADMISSION_DOMAIN: &[u8] = b"edgerun:v1:work:admission";

pub fn empty_signature() -> WorkSignature {
    WorkSignature {
        algorithm: SIGNATURE_ALGORITHM_SOLANA_ED25519,
        public_key: Vec::new(),
        signature: Vec::new(),
    }
}

pub fn sign_ed25519(key: &Ed25519SigningKey, preimage: &[u8]) -> WorkSignature {
    WorkSignature {
        algorithm: SIGNATURE_ALGORITHM_SOLANA_ED25519,
        public_key: key.verifying_key().as_bytes().to_vec(),
        signature: key.sign(preimage).to_bytes().to_vec(),
    }
}

pub fn verify_node_identity(identity: &NodeIdentity) -> bool {
    identity.node_id == identity.public_key
}

pub fn verify_signature(
    identity: &NodeIdentity,
    signature: &WorkSignature,
    preimage: &[u8],
) -> bool {
    if !verify_node_identity(identity) || signature.algorithm != SIGNATURE_ALGORITHM_SOLANA_ED25519
    {
        return false;
    }
    if signature.public_key.as_slice() != &identity.public_key[..] {
        return false;
    }
    verify_solana_ed25519(&identity.public_key, preimage, &signature.signature)
}

pub fn verify_solana_ed25519(public_key: &PublicKey, preimage: &[u8], signature: &[u8]) -> bool {
    edgerun_crypto::verification::ed25519_verify_strict(public_key, preimage, signature).is_ok()
}

pub fn work_admission_preimage(value: &WorkAdmission) -> Vec<u8> {
    PreimageBuilder::domain(WORK_ADMISSION_DOMAIN)
        .hash(&value.admission_id)
        .hash(&value.dao_id)
        .hash(&value.user)
        .node(&value.admission_node)
        .hash(&value.request_hash)
        .hash(&value.assigned_route_commitment)
        .channel(&value.assigned_channel)
        .hash_list(&value.assigned_relay_path)
        .u64(value.admitted_budget)
        .hash(&value.policy_hash)
        .u64(value.sequence)
        .u64(value.valid_until_unix_ms)
        .finish()
}

pub fn work_admission_hash(value: &WorkAdmission) -> Hash {
    blake3_hash(&work_admission_preimage(value))
}

pub fn sign_work_admission(key: &Ed25519SigningKey, mut value: WorkAdmission) -> WorkAdmission {
    value.signature = sign_ed25519(key, &work_admission_preimage(&value));
    value
}

pub fn verify_work_admission(value: &WorkAdmission) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.admitted_budget > 0
        && value.valid_until_unix_ms > 0
        && verify_signature(
            &value.admission_node,
            &value.signature,
            &work_admission_preimage(value),
        )
}
