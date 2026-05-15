use alloc::vec::Vec;

use crate::codec::{wire_bytes, wire_from_bytes};
use crate::erasure_storage::{erasure_shard_hash, ErasureManifest, ErasureShard};
use crate::generated_wire::ArchivedStoragePayload;
use crate::preimage::HashBuilder;
use crate::protocol::{Hash, WorkProtocolError};

pub const STORAGE_PAYLOAD_KIND_STORE_REQUEST: u16 = 1;
pub const STORAGE_PAYLOAD_KIND_RETRIEVE_REQUEST: u16 = 2;
pub const STORAGE_PAYLOAD_KIND_RETRIEVE_RESPONSE: u16 = 3;

const ERASURE_MANIFEST_HASH_DOMAIN: &[u8] = b"edgerun:v1:work:erasure-manifest";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectStoreRequest {
    pub manifest_hash: Hash,
    pub job_id: Hash,
    pub shard_index: u16,
    pub shard_hash: Hash,
    pub original_len: u64,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectRetrieveRequest {
    pub manifest_hash: Hash,
    pub job_id: Hash,
    pub shard_index: u16,
    pub shard_hash: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectRetrieveResponse {
    pub manifest_hash: Hash,
    pub job_id: Hash,
    pub shard_index: u16,
    pub shard_hash: Hash,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoragePayload {
    StoreRequest(ObjectStoreRequest),
    RetrieveRequest(ObjectRetrieveRequest),
    RetrieveResponse(ObjectRetrieveResponse),
}

pub fn manifest_hash(manifest: &ErasureManifest) -> Hash {
    let mut builder = HashBuilder::domain(ERASURE_MANIFEST_HASH_DOMAIN)
        .hash(&manifest.job_id)
        .hash(&manifest.original_hash)
        .u64(manifest.original_len)
        .u16(manifest.data_shards)
        .u16(manifest.parity_shards)
        .u64(manifest.shard_len)
        .u64(manifest.shard_hashes.len() as u64);
    for hash in &manifest.shard_hashes {
        builder = builder.hash(hash);
    }
    builder = builder.u64(manifest.assigned_nodes.len() as u64);
    for node_id in &manifest.assigned_nodes {
        builder = builder.node_id(node_id);
    }
    builder.finish()
}

pub fn store_request_from_shard(
    manifest: &ErasureManifest,
    shard: &ErasureShard,
) -> ObjectStoreRequest {
    ObjectStoreRequest {
        manifest_hash: manifest_hash(manifest),
        job_id: manifest.job_id,
        shard_index: shard.index,
        shard_hash: shard.hash,
        original_len: shard.original_len,
        bytes: shard.bytes.clone(),
    }
}

pub fn retrieve_request_from_manifest(
    manifest: &ErasureManifest,
    shard_index: u16,
) -> Option<ObjectRetrieveRequest> {
    let shard_hash = *manifest.shard_hashes.get(shard_index as usize)?;
    Some(ObjectRetrieveRequest {
        manifest_hash: manifest_hash(manifest),
        job_id: manifest.job_id,
        shard_index,
        shard_hash,
    })
}

pub fn retrieve_response_from_store_request(
    request: &ObjectStoreRequest,
) -> ObjectRetrieveResponse {
    ObjectRetrieveResponse {
        manifest_hash: request.manifest_hash,
        job_id: request.job_id,
        shard_index: request.shard_index,
        shard_hash: request.shard_hash,
        bytes: request.bytes.clone(),
    }
}

pub fn verify_store_request(request: &ObjectStoreRequest) -> bool {
    request.shard_hash
        == typed_shard_hash(
            request.job_id,
            request.shard_index,
            request.shard_index == 2,
            &request.bytes,
        )
}

pub fn verify_retrieve_response(response: &ObjectRetrieveResponse) -> bool {
    response.shard_hash
        == typed_shard_hash(
            response.job_id,
            response.shard_index,
            response.shard_index == 2,
            &response.bytes,
        )
}

pub fn storage_payload_bytes(payload: &StoragePayload) -> Result<Vec<u8>, WorkProtocolError> {
    wire_bytes(payload)
}

pub fn storage_payload_from_bytes(bytes: &[u8]) -> Result<StoragePayload, WorkProtocolError> {
    wire_from_bytes::<StoragePayload, ArchivedStoragePayload>(bytes)
}

pub fn typed_shard_hash(job_id: Hash, index: u16, is_parity: bool, bytes: &[u8]) -> Hash {
    erasure_shard_hash(job_id, index, is_parity, bytes)
}
