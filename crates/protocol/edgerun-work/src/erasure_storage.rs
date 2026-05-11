use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;

use crate::codec::blake3_hash;
use crate::preimage::HashBuilder;
use crate::protocol::{Hash, NodeId};

pub const ERASURE_SCHEME_XOR_2_1: u16 = 1;
const ERASURE_JOB_ID_DOMAIN: &[u8] = b"edgerun:v1:work:erasure-job";
const ERASURE_SHARD_HASH_DOMAIN: &[u8] = b"edgerun:v1:work:erasure-shard";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ErasureStorageError {
    InvalidDataShardCount,
    TooManyMissingDataShards,
    MissingParityShard,
    MissingShard,
    HashMismatch,
    ManifestMismatch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErasureShard {
    pub job_id: Hash,
    pub index: u16,
    pub is_parity: bool,
    pub original_len: u64,
    pub bytes: Vec<u8>,
    pub hash: Hash,
    pub assigned_node: NodeId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErasureManifest {
    pub job_id: Hash,
    pub scheme: u16,
    pub original_hash: Hash,
    pub original_len: u64,
    pub data_shards: u16,
    pub parity_shards: u16,
    pub shard_len: u64,
    pub shard_hashes: Vec<Hash>,
    pub assigned_nodes: Vec<NodeId>,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryShardStore {
    shards: BTreeMap<Hash, ErasureShard>,
}

impl MemoryShardStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn put(&mut self, shard: ErasureShard) -> Hash {
        let hash = shard.hash;
        self.shards.insert(hash, shard);
        hash
    }

    pub fn get(&self, hash: &Hash) -> Option<&ErasureShard> {
        self.shards.get(hash)
    }

    pub fn remove(&mut self, hash: &Hash) -> Option<ErasureShard> {
        self.shards.remove(hash)
    }

    pub fn len(&self) -> usize {
        self.shards.len()
    }

    pub fn is_empty(&self) -> bool {
        self.shards.is_empty()
    }
}

pub fn encode_xor_2_1(
    input: &[u8],
    assigned_nodes: [NodeId; 3],
) -> Result<(ErasureManifest, Vec<ErasureShard>), ErasureStorageError> {
    let original_hash = blake3_hash(input);
    let original_len = input.len() as u64;
    let shard_len = input.len().div_ceil(2).max(1);
    let job_id = erasure_job_id(original_hash, original_len, ERASURE_SCHEME_XOR_2_1);

    let mut data0 = vec![0u8; shard_len];
    let mut data1 = vec![0u8; shard_len];
    let split = input.len().min(shard_len);
    data0[..split].copy_from_slice(&input[..split]);
    if input.len() > split {
        data1[..input.len() - split].copy_from_slice(&input[split..]);
    }

    let parity = xor_bytes(&data0, &data1);
    let shard_bytes = [data0, data1, parity];
    let mut shards = Vec::with_capacity(3);
    let mut shard_hashes = Vec::with_capacity(3);
    for (index, bytes) in shard_bytes.into_iter().enumerate() {
        let hash = erasure_shard_hash(job_id, index as u16, index == 2, &bytes);
        shard_hashes.push(hash);
        shards.push(ErasureShard {
            job_id,
            index: index as u16,
            is_parity: index == 2,
            original_len,
            bytes,
            hash,
            assigned_node: assigned_nodes[index],
        });
    }

    let manifest = ErasureManifest {
        job_id,
        scheme: ERASURE_SCHEME_XOR_2_1,
        original_hash,
        original_len,
        data_shards: 2,
        parity_shards: 1,
        shard_len: shard_len as u64,
        shard_hashes,
        assigned_nodes: assigned_nodes.into_iter().collect(),
    };
    Ok((manifest, shards))
}

pub fn verify_manifest(
    manifest: &ErasureManifest,
    shards: &[ErasureShard],
) -> Result<(), ErasureStorageError> {
    if manifest.scheme != ERASURE_SCHEME_XOR_2_1
        || manifest.data_shards != 2
        || manifest.parity_shards != 1
    {
        return Err(ErasureStorageError::InvalidDataShardCount);
    }
    for shard in shards {
        let expected_hash = erasure_shard_hash(shard.job_id, shard.index, shard.is_parity, &shard.bytes);
        if shard.hash != expected_hash {
            return Err(ErasureStorageError::HashMismatch);
        }
        let index = shard.index as usize;
        if shard.job_id != manifest.job_id
            || index >= manifest.shard_hashes.len()
            || manifest.shard_hashes[index] != shard.hash
            || manifest.assigned_nodes[index] != shard.assigned_node
        {
            return Err(ErasureStorageError::ManifestMismatch);
        }
    }
    Ok(())
}

pub fn reconstruct_xor_2_1(
    manifest: &ErasureManifest,
    shards: &[ErasureShard],
) -> Result<Vec<u8>, ErasureStorageError> {
    verify_manifest(manifest, shards)?;
    let mut data: [Option<Vec<u8>>; 2] = [None, None];
    let mut parity: Option<Vec<u8>> = None;

    for shard in shards {
        match (shard.index, shard.is_parity) {
            (0, false) => data[0] = Some(shard.bytes.clone()),
            (1, false) => data[1] = Some(shard.bytes.clone()),
            (2, true) => parity = Some(shard.bytes.clone()),
            _ => return Err(ErasureStorageError::ManifestMismatch),
        }
    }

    if data[0].is_none() && data[1].is_none() {
        return Err(ErasureStorageError::TooManyMissingDataShards);
    }
    if data[0].is_none() || data[1].is_none() {
        let parity = parity.as_ref().ok_or(ErasureStorageError::MissingParityShard)?;
        if data[0].is_none() {
            let data1 = data[1].as_ref().ok_or(ErasureStorageError::MissingShard)?;
            data[0] = Some(xor_bytes(data1, parity));
        }
        if data[1].is_none() {
            let data0 = data[0].as_ref().ok_or(ErasureStorageError::MissingShard)?;
            data[1] = Some(xor_bytes(data0, parity));
        }
    }

    let mut output = Vec::with_capacity((manifest.shard_len * 2) as usize);
    output.extend_from_slice(data[0].as_ref().ok_or(ErasureStorageError::MissingShard)?);
    output.extend_from_slice(data[1].as_ref().ok_or(ErasureStorageError::MissingShard)?);
    output.truncate(manifest.original_len as usize);
    if blake3_hash(&output) != manifest.original_hash {
        return Err(ErasureStorageError::HashMismatch);
    }
    Ok(output)
}

pub fn retrieve_shards(
    store: &MemoryShardStore,
    manifest: &ErasureManifest,
    available_indexes: &[u16],
) -> Vec<ErasureShard> {
    let mut out = Vec::new();
    for index in available_indexes {
        if let Some(hash) = manifest.shard_hashes.get(*index as usize) {
            if let Some(shard) = store.get(hash) {
                out.push(shard.clone());
            }
        }
    }
    out
}

fn xor_bytes(a: &[u8], b: &[u8]) -> Vec<u8> {
    let len = a.len().max(b.len());
    let mut out = vec![0u8; len];
    for i in 0..len {
        out[i] = a.get(i).copied().unwrap_or(0) ^ b.get(i).copied().unwrap_or(0);
    }
    out
}

pub fn erasure_job_id(original_hash: Hash, original_len: u64, scheme: u16) -> Hash {
    HashBuilder::domain(ERASURE_JOB_ID_DOMAIN)
        .hash(&original_hash)
        .u64(original_len)
        .u16(scheme)
        .finish()
}

pub fn erasure_shard_hash(job_id: Hash, index: u16, is_parity: bool, bytes: &[u8]) -> Hash {
    HashBuilder::domain(ERASURE_SHARD_HASH_DOMAIN)
        .hash(&job_id)
        .u16(index)
        .raw(&[is_parity as u8])
        .bytes(bytes)
        .finish()
}
