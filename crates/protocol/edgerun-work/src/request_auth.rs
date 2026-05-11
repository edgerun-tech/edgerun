use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::protocol::*;
use crate::signing::{sign_ed25519, verify_solana_ed25519};

const WORK_REQUEST_DOMAIN: &[u8] = b"edgerun:v1:work:request";

pub fn work_request_preimage(value: &WorkRequest) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(WORK_REQUEST_DOMAIN);
    out.push(0);
    out.extend_from_slice(&value.request_id);
    out.extend_from_slice(&value.user);
    out.extend_from_slice(&value.user_sequence.to_be_bytes());
    out.extend_from_slice(&value.recipient);
    out.extend_from_slice(&value.work_type.to_be_bytes());
    out.extend_from_slice(&value.department.to_be_bytes());
    out.extend_from_slice(&value.payload_hash);
    out.extend_from_slice(&value.input_root);
    out.extend_from_slice(&value.max_total_cost.to_be_bytes());
    out.extend_from_slice(&value.valid_until_unix_ms.to_be_bytes());
    out
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
