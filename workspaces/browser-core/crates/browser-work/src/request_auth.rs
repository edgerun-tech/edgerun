use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::codec::blake3_hash;
use crate::preimage::PreimageBuilder;
use crate::protocol::{
    Hash, WorkRequest, SIGNATURE_ALGORITHM_SOLANA_ED25519, WORK_WIRE_ABI_VERSION,
};
use crate::signing::{sign_ed25519, verify_solana_ed25519};

const WORK_REQUEST_DOMAIN: &[u8] = b"edgerun:v1:work:request";

pub fn work_request_preimage(value: &WorkRequest) -> Vec<u8> {
    PreimageBuilder::domain(WORK_REQUEST_DOMAIN)
        .hash(&value.request_id)
        .hash(&value.user)
        .u64(value.user_sequence)
        .node_id(&value.recipient)
        .u16(value.work_type)
        .u16(value.department)
        .hash(&value.payload_hash)
        .hash(&value.input_root)
        .u64(value.max_total_cost)
        .u64(value.valid_until_unix_ms)
        .finish()
}

pub fn work_request_hash(value: &WorkRequest) -> Hash {
    blake3_hash(&work_request_preimage(value))
}

pub fn sign_work_request(key: &Ed25519SigningKey, mut value: WorkRequest) -> WorkRequest {
    value.signature = sign_ed25519(key, &work_request_preimage(&value));
    value
}

pub fn verify_work_request(value: &WorkRequest) -> bool {
    if value.abi_version != WORK_WIRE_ABI_VERSION {
        return false;
    }
    if value.signature.algorithm != SIGNATURE_ALGORITHM_SOLANA_ED25519 {
        return false;
    }
    if value.signature.public_key.as_slice() != &value.user[..] {
        return false;
    }
    verify_solana_ed25519(
        &value.user,
        &work_request_preimage(value),
        &value.signature.signature,
    )
}
