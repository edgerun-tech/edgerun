use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::preimage::{HashBuilder, PreimageBuilder};
use crate::protocol::{Hash, NodeIdentity, WorkSignature, WORK_WIRE_ABI_VERSION};
use crate::signing::{empty_signature, sign_ed25519, verify_signature};

const STORAGE_AVAILABILITY_DOMAIN: &[u8] = b"edgerun:v1:work:storage-availability";
const STORAGE_RETRIEVAL_DOMAIN: &[u8] = b"edgerun:v1:work:storage-retrieval";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageAvailabilityProof {
    pub abi_version: u16,
    pub provider: NodeIdentity,
    pub admission_hash: Hash,
    pub route_commitment: Hash,
    pub channel_proof_hash: Hash,
    pub object_hash: Hash,
    pub object_len: u64,
    pub observed_at_unix_ms: u64,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StorageRetrievalProof {
    pub abi_version: u16,
    pub provider: NodeIdentity,
    pub availability_proof_hash: Hash,
    pub request_hash: Hash,
    pub response_payload_hash: Hash,
    pub response_len: u64,
    pub observed_at_unix_ms: u64,
    pub signature: WorkSignature,
}

pub fn storage_availability_preimage(value: &StorageAvailabilityProof) -> Vec<u8> {
    PreimageBuilder::domain(STORAGE_AVAILABILITY_DOMAIN)
        .node(&value.provider)
        .hash(&value.admission_hash)
        .hash(&value.route_commitment)
        .hash(&value.channel_proof_hash)
        .hash(&value.object_hash)
        .u64(value.object_len)
        .u64(value.observed_at_unix_ms)
        .finish()
}

pub fn storage_availability_proof_hash(value: &StorageAvailabilityProof) -> Hash {
    HashBuilder::domain(STORAGE_AVAILABILITY_DOMAIN)
        .bytes(&storage_availability_preimage(value))
        .finish()
}

pub fn sign_storage_availability_proof(
    key: &Ed25519SigningKey,
    mut value: StorageAvailabilityProof,
) -> StorageAvailabilityProof {
    value.signature = sign_ed25519(key, &storage_availability_preimage(&value));
    value
}

pub fn verify_storage_availability_proof(value: &StorageAvailabilityProof) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.object_len > 0
        && value.admission_hash != [0; 32]
        && value.route_commitment != [0; 32]
        && value.channel_proof_hash != [0; 32]
        && verify_signature(
            &value.provider,
            &value.signature,
            &storage_availability_preimage(value),
        )
}

pub fn storage_retrieval_preimage(value: &StorageRetrievalProof) -> Vec<u8> {
    PreimageBuilder::domain(STORAGE_RETRIEVAL_DOMAIN)
        .node(&value.provider)
        .hash(&value.availability_proof_hash)
        .hash(&value.request_hash)
        .hash(&value.response_payload_hash)
        .u64(value.response_len)
        .u64(value.observed_at_unix_ms)
        .finish()
}

pub fn storage_retrieval_proof_hash(value: &StorageRetrievalProof) -> Hash {
    HashBuilder::domain(STORAGE_RETRIEVAL_DOMAIN)
        .bytes(&storage_retrieval_preimage(value))
        .finish()
}

pub fn sign_storage_retrieval_proof(
    key: &Ed25519SigningKey,
    mut value: StorageRetrievalProof,
) -> StorageRetrievalProof {
    value.signature = sign_ed25519(key, &storage_retrieval_preimage(&value));
    value
}

pub fn verify_storage_retrieval_proof(
    value: &StorageRetrievalProof,
    availability: &StorageAvailabilityProof,
) -> bool {
    value.abi_version == WORK_WIRE_ABI_VERSION
        && value.provider == availability.provider
        && value.availability_proof_hash == storage_availability_proof_hash(availability)
        && value.response_len > 0
        && value.request_hash != [0; 32]
        && value.response_payload_hash == availability.object_hash
        && verify_signature(
            &value.provider,
            &value.signature,
            &storage_retrieval_preimage(value),
        )
}

pub fn unsigned_storage_availability_proof(
    provider: NodeIdentity,
    admission_hash: Hash,
    route_commitment: Hash,
    channel_proof_hash: Hash,
    object_hash: Hash,
    object_len: u64,
    observed_at_unix_ms: u64,
) -> StorageAvailabilityProof {
    StorageAvailabilityProof {
        abi_version: WORK_WIRE_ABI_VERSION,
        provider,
        admission_hash,
        route_commitment,
        channel_proof_hash,
        object_hash,
        object_len,
        observed_at_unix_ms,
        signature: empty_signature(),
    }
}

pub fn unsigned_storage_retrieval_proof(
    provider: NodeIdentity,
    availability_proof_hash: Hash,
    request_hash: Hash,
    response_payload_hash: Hash,
    response_len: u64,
    observed_at_unix_ms: u64,
) -> StorageRetrievalProof {
    StorageRetrievalProof {
        abi_version: WORK_WIRE_ABI_VERSION,
        provider,
        availability_proof_hash,
        request_hash,
        response_payload_hash,
        response_len,
        observed_at_unix_ms,
        signature: empty_signature(),
    }
}
