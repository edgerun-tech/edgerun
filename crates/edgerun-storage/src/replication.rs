// SPDX-License-Identifier: GPL-2.0-only
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Mutex, OnceLock, RwLock};
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

use crate::event::{ActorId, HlcTimestamp};

#[derive(Error, Debug)]
pub enum ReplicationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Node not found")]
    NodeNotFound,
    #[error("Segment not found")]
    SegmentNotFound,
    #[error("Verification failed")]
    VerificationFailed,
    #[error("Network error: {0}")]
    Network(String),
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    #[error("Invalid quorum requirement: required={required}, members={members}")]
    InvalidQuorumRequirement { required: u8, members: usize },
    #[error("Quorum not met: required={required}, observed={observed}")]
    QuorumNotMet { required: u8, observed: u8 },
    #[error(
        "Quorum wait timed out after {timeout_ms} ms (required={required}, observed={observed})"
    )]
    QuorumTimeout {
        timeout_ms: u64,
        required: u8,
        observed: u8,
    },
}

#[derive(Debug, Clone)]
pub struct NodeInfo {
    pub actor_id: ActorId,
    pub address: String,
    pub last_seen: HlcTimestamp,
    pub store_uuid: [u8; 16],
    pub auth_key: Option<[u8; 32]>,
}

impl NodeInfo {
    pub fn new(actor_id: ActorId, address: String, store_uuid: [u8; 16]) -> Self {
        Self {
            actor_id,
            address,
            last_seen: HlcTimestamp::now(),
            store_uuid,
            auth_key: None,
        }
    }

    pub fn with_auth_key(mut self, auth_key: [u8; 32]) -> Self {
        self.auth_key = Some(auth_key);
        self
    }

    pub fn with_auth_token(mut self, token: &[u8]) -> Self {
        self.auth_key = Some(derive_auth_key(token));
        self
    }
}

#[derive(Debug, Clone)]
pub struct ManifestSummary {
    pub store_uuid: [u8; 16],
    pub epoch: u64,
    pub segment_ids: Vec<[u8; 32]>,
    pub frontier: FrontierSummary,
}

#[derive(Debug, Clone, Default)]
pub struct FrontierSummary {
    pub per_actor: HashMap<[u8; 16], u64>,
}

#[derive(Debug, Clone)]
pub struct MerkleNode {
    pub hash: [u8; 32],
    pub children: Option<[Option<[u8; 32]>; 2]>,
}

pub struct MerkleTree {
    root: Option<MerkleNode>,
}

impl MerkleTree {
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn from_segments(segments: &[[u8; 32]]) -> Self {
        if segments.is_empty() {
            return Self { root: None };
        }

        let leaves: Vec<[u8; 32]> = segments.to_vec();
        let root = Self::build_tree(&leaves);

        Self { root: Some(root) }
    }

    fn build_tree(leaves: &[[u8; 32]]) -> MerkleNode {
        if leaves.len() == 1 {
            return MerkleNode {
                hash: Self::hash_node(&leaves[0], &leaves[0]),
                children: None,
            };
        }

        let mid = leaves.len().div_ceil(2);
        let left = Self::build_tree(&leaves[..mid]);
        let right = Self::build_tree(&leaves[mid..]);

        let combined = [left.hash, right.hash];
        MerkleNode {
            hash: Self::hash_node(&combined[0], &combined[1]),
            children: Some([Some(left.hash), Some(right.hash)]),
        }
    }

    fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(left);
        hasher.update(right);
        *hasher.finalize().as_bytes()
    }

    pub fn root_hash(&self) -> Option<[u8; 32]> {
        self.root.as_ref().map(|r| r.hash)
    }

    pub fn diff(&self, other: &MerkleTree) -> Vec<[u8; 32]> {
        let mut missing = Vec::new();

        let self_hash = self.root_hash();
        let other_hash = other.root_hash();

        if self_hash != other_hash {
            if let (Some(s), Some(o)) = (&self.root, &other.root) {
                Self::find_missing(s, o, &mut missing);
            }
        }

        missing
    }

    fn find_missing(this: &MerkleNode, other: &MerkleNode, missing: &mut Vec<[u8; 32]>) {
        if this.hash == other.hash {
            return;
        }

        match (&this.children, &other.children) {
            (None, None) => {
                missing.push(this.hash);
            }
            (Some([Some(l1), Some(r1)]), Some([Some(l2), Some(r2)])) => {
                if l1 != l2 {
                    missing.push(*l1);
                }
                if r1 != r2 {
                    missing.push(*r1);
                }
            }
            _ => {
                missing.push(this.hash);
            }
        }
    }
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Replicator {
    #[allow(dead_code)]
    local_node_id: ActorId,
    local_store_uuid: [u8; 16],
    nodes: RwLock<HashMap<ActorId, NodeInfo>>,
    segment_set: RwLock<HashSet<[u8; 32]>>,
    frontier: RwLock<super::index::VersionVector>,
}

impl Replicator {
    pub fn new(actor_id: ActorId, store_uuid: [u8; 16]) -> Self {
        Self {
            local_node_id: actor_id,
            local_store_uuid: store_uuid,
            nodes: RwLock::new(HashMap::new()),
            segment_set: RwLock::new(HashSet::new()),
            frontier: RwLock::new(super::index::VersionVector::new()),
        }
    }

    pub fn add_node(&self, node: NodeInfo) {
        let mut nodes = self.nodes.write().unwrap();
        nodes.insert(node.actor_id.clone(), node);
    }

    pub fn remove_node(&self, actor_id: &ActorId) {
        let mut nodes = self.nodes.write().unwrap();
        nodes.remove(actor_id);
    }

    pub fn get_nodes(&self) -> Vec<NodeInfo> {
        let nodes = self.nodes.read().unwrap();
        nodes.values().cloned().collect()
    }

    pub fn add_segment(&self, segment_id: [u8; 32]) {
        let mut segments = self.segment_set.write().unwrap();
        segments.insert(segment_id);
    }

    pub fn has_segment(&self, segment_id: &[u8; 32]) -> bool {
        let segments = self.segment_set.read().unwrap();
        segments.contains(segment_id)
    }

    pub fn get_segments(&self) -> Vec<[u8; 32]> {
        let segments = self.segment_set.read().unwrap();
        segments.iter().cloned().collect()
    }

    pub fn update_frontier(&self, actor_id: [u8; 16], _counter: u64) {
        let mut frontier = self.frontier.write().unwrap();
        frontier.increment(actor_id);
    }

    pub fn get_frontier(&self) -> super::index::VersionVector {
        self.frontier.read().unwrap().clone()
    }

    pub fn get_local_manifest_summary(&self) -> ManifestSummary {
        let segments = self.segment_set.read().unwrap();
        let frontier = self.frontier.read().unwrap();

        ManifestSummary {
            store_uuid: self.local_store_uuid,
            epoch: 0,
            segment_ids: segments.iter().cloned().collect(),
            frontier: FrontierSummary {
                per_actor: frontier.to_hashmap(),
            },
        }
    }

    pub fn compute_missing_segments(&self, other: &ManifestSummary) -> Vec<[u8; 32]> {
        let local_segments = self.segment_set.read().unwrap();

        let local_vec: Vec<[u8; 32]> = local_segments.iter().cloned().collect();

        other
            .segment_ids
            .iter()
            .filter(|id| !local_vec.contains(id))
            .cloned()
            .collect()
    }

    pub fn create_merkle_tree(&self) -> MerkleTree {
        let segments = self.segment_set.read().unwrap();
        let segment_array: Vec<[u8; 32]> = segments.iter().cloned().collect();
        MerkleTree::from_segments(&segment_array)
    }

    pub fn verify_segment(
        &self,
        segment_id: [u8; 32],
        data: &[u8],
        expected_hash: &[u8; 32],
    ) -> bool {
        let computed = blake3::hash(data);

        if computed.as_bytes() != expected_hash {
            return false;
        }

        let mut segments = self.segment_set.write().unwrap();
        segments.insert(segment_id);

        true
    }
}

pub struct AntiEntropy {
    replicator: Replicator,
}

/// Tracks quorum ACKs for a single replicated append operation.
#[derive(Debug)]
pub struct QuorumTracker {
    required: u8,
    started: Instant,
    timeout: Duration,
    known_remotes: HashSet<ActorId>,
    local_durable: bool,
    remote_acks: HashSet<ActorId>,
}

impl QuorumTracker {
    pub fn new(
        required: u8,
        timeout: Duration,
        replica_nodes: &[NodeInfo],
    ) -> Result<Self, ReplicationError> {
        let known_remotes: HashSet<ActorId> =
            replica_nodes.iter().map(|n| n.actor_id.clone()).collect();
        let members = 1 + known_remotes.len(); // include local node
        if required == 0 || required as usize > members {
            return Err(ReplicationError::InvalidQuorumRequirement { required, members });
        }

        Ok(Self {
            required,
            started: Instant::now(),
            timeout,
            known_remotes,
            local_durable: false,
            remote_acks: HashSet::new(),
        })
    }

    pub fn ack_local_durable(&mut self) {
        self.local_durable = true;
    }

    pub fn ack_remote(&mut self, actor_id: &ActorId) -> Result<(), ReplicationError> {
        if !self.known_remotes.contains(actor_id) {
            return Err(ReplicationError::NodeNotFound);
        }
        self.remote_acks.insert(actor_id.clone());
        Ok(())
    }

    pub fn observed_acks(&self) -> u8 {
        (self.local_durable as u8).saturating_add(self.remote_acks.len() as u8)
    }

    pub fn is_satisfied(&self) -> bool {
        self.observed_acks() >= self.required
    }

    pub fn finalize(&self) -> Result<(), ReplicationError> {
        let observed = self.observed_acks();
        if observed >= self.required {
            return Ok(());
        }
        if self.started.elapsed() >= self.timeout {
            return Err(ReplicationError::QuorumTimeout {
                timeout_ms: self.timeout.as_millis() as u64,
                required: self.required,
                observed,
            });
        }
        Err(ReplicationError::QuorumNotMet {
            required: self.required,
            observed,
        })
    }
}

const ACK_REQUEST_MAGIC: &[u8; 4] = b"ACK?";
const ACK_RESPONSE_OK: &[u8; 4] = b"ACK\n";
const ACK_BATCH_REQUEST_MAGIC: &[u8; 4] = b"AKB?";
const ACK_BATCH_RESPONSE_MAGIC: &[u8; 4] = b"AKB!";
const ACKV2_REQUEST_MAGIC: &[u8; 4] = b"AK2?";
const ACKV2_RESPONSE_MAGIC: &[u8; 4] = b"AK2!";
const ACKV2_BATCH_REQUEST_MAGIC: &[u8; 4] = b"AB2?";
const ACKV2_BATCH_RESPONSE_MAGIC: &[u8; 4] = b"AB2!";
const ACKV2_FRAME_LEN: usize = 4 + 16 + 32 + 32;
const ACK_BATCH_HEADER_LEN: usize = 4 + 2;
const ACKV2_BATCH_HEADER_LEN: usize = 4 + 16 + 2;
const ACK_BATCH_MAX_OPS: usize = u16::MAX as usize;
static ACK_CONNECTION_POOL: OnceLock<Mutex<HashMap<String, TcpStream>>> = OnceLock::new();

pub fn derive_auth_key(token: &[u8]) -> [u8; 32] {
    *blake3::hash(token).as_bytes()
}

fn authenticated_mac(
    key: [u8; 32],
    magic: &[u8; 4],
    store_uuid: [u8; 16],
    op_id: [u8; 32],
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_keyed(&key);
    hasher.update(magic);
    hasher.update(&store_uuid);
    hasher.update(&op_id);
    *hasher.finalize().as_bytes()
}

pub fn build_authenticated_ack_request(
    store_uuid: [u8; 16],
    op_id: [u8; 32],
    auth_key: [u8; 32],
) -> [u8; ACKV2_FRAME_LEN] {
    let mut frame = [0u8; ACKV2_FRAME_LEN];
    frame[0..4].copy_from_slice(ACKV2_REQUEST_MAGIC);
    frame[4..20].copy_from_slice(&store_uuid);
    frame[20..52].copy_from_slice(&op_id);
    let mac = authenticated_mac(auth_key, ACKV2_REQUEST_MAGIC, store_uuid, op_id);
    frame[52..84].copy_from_slice(&mac);
    frame
}

pub fn verify_authenticated_ack_request(
    frame: &[u8],
    expected_store_uuid: [u8; 16],
    auth_key: [u8; 32],
) -> Result<[u8; 32], ReplicationError> {
    if frame.len() != ACKV2_FRAME_LEN {
        return Err(ReplicationError::AuthenticationFailed(
            "invalid request frame length".to_string(),
        ));
    }
    if &frame[0..4] != ACKV2_REQUEST_MAGIC {
        return Err(ReplicationError::AuthenticationFailed(
            "invalid request magic".to_string(),
        ));
    }
    let mut store_uuid = [0u8; 16];
    store_uuid.copy_from_slice(&frame[4..20]);
    if store_uuid != expected_store_uuid {
        return Err(ReplicationError::AuthenticationFailed(
            "store uuid mismatch".to_string(),
        ));
    }
    let mut op_id = [0u8; 32];
    op_id.copy_from_slice(&frame[20..52]);
    let mut mac = [0u8; 32];
    mac.copy_from_slice(&frame[52..84]);
    let expected_mac = authenticated_mac(auth_key, ACKV2_REQUEST_MAGIC, store_uuid, op_id);
    if mac != expected_mac {
        return Err(ReplicationError::AuthenticationFailed(
            "request mac mismatch".to_string(),
        ));
    }
    Ok(op_id)
}

pub fn build_authenticated_ack_response(
    store_uuid: [u8; 16],
    op_id: [u8; 32],
    auth_key: [u8; 32],
) -> [u8; ACKV2_FRAME_LEN] {
    let mut frame = [0u8; ACKV2_FRAME_LEN];
    frame[0..4].copy_from_slice(ACKV2_RESPONSE_MAGIC);
    frame[4..20].copy_from_slice(&store_uuid);
    frame[20..52].copy_from_slice(&op_id);
    let mac = authenticated_mac(auth_key, ACKV2_RESPONSE_MAGIC, store_uuid, op_id);
    frame[52..84].copy_from_slice(&mac);
    frame
}

pub fn verify_authenticated_ack_response(
    frame: &[u8],
    expected_store_uuid: [u8; 16],
    expected_op_id: [u8; 32],
    auth_key: [u8; 32],
) -> Result<(), ReplicationError> {
    if frame.len() != ACKV2_FRAME_LEN {
        return Err(ReplicationError::AuthenticationFailed(
            "invalid response frame length".to_string(),
        ));
    }
    if &frame[0..4] != ACKV2_RESPONSE_MAGIC {
        return Err(ReplicationError::AuthenticationFailed(
            "invalid response magic".to_string(),
        ));
    }
    let mut store_uuid = [0u8; 16];
    store_uuid.copy_from_slice(&frame[4..20]);
    if store_uuid != expected_store_uuid {
        return Err(ReplicationError::AuthenticationFailed(
            "response store uuid mismatch".to_string(),
        ));
    }
    let mut op_id = [0u8; 32];
    op_id.copy_from_slice(&frame[20..52]);
    if op_id != expected_op_id {
        return Err(ReplicationError::AuthenticationFailed(
            "response op id mismatch".to_string(),
        ));
    }
    let mut mac = [0u8; 32];
    mac.copy_from_slice(&frame[52..84]);
    let expected_mac = authenticated_mac(auth_key, ACKV2_RESPONSE_MAGIC, store_uuid, op_id);
    if mac != expected_mac {
        return Err(ReplicationError::AuthenticationFailed(
            "response mac mismatch".to_string(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    pub max_retries: u8,
    pub backoff_base: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            backoff_base: Duration::from_millis(10),
        }
    }
}

#[derive(Debug)]
pub struct IdempotencyWindow {
    max_entries: usize,
    seen: std::collections::VecDeque<[u8; 32]>,
    set: HashSet<[u8; 32]>,
}

impl IdempotencyWindow {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries: max_entries.max(1),
            seen: std::collections::VecDeque::new(),
            set: HashSet::new(),
        }
    }

    /// Returns true if this op id was newly inserted, false if it was already seen.
    pub fn observe(&mut self, op_id: [u8; 32]) -> bool {
        if self.set.contains(&op_id) {
            return false;
        }
        self.seen.push_back(op_id);
        self.set.insert(op_id);
        while self.seen.len() > self.max_entries {
            if let Some(oldest) = self.seen.pop_front() {
                self.set.remove(&oldest);
            }
        }
        true
    }
}

fn connect_stream(node: &NodeInfo, timeout: Duration) -> Result<TcpStream, ReplicationError> {
    let mut addrs = node
        .address
        .to_socket_addrs()
        .map_err(|e| ReplicationError::Network(format!("resolve {}: {}", node.address, e)))?;
    let addr = addrs.next().ok_or_else(|| {
        ReplicationError::Network(format!("no address resolved for {}", node.address))
    })?;
    let stream = TcpStream::connect_timeout(&addr, timeout)
        .map_err(|e| ReplicationError::Network(format!("connect {}: {}", node.address, e)))?;
    Ok(stream)
}

fn pool_key(node: &NodeInfo) -> String {
    format!("{}#{:x?}", node.address, node.store_uuid)
}

fn ack_connection_pool() -> &'static Mutex<HashMap<String, TcpStream>> {
    ACK_CONNECTION_POOL.get_or_init(|| Mutex::new(HashMap::new()))
}

fn try_take_pooled_stream(node: &NodeInfo) -> Option<TcpStream> {
    let key = pool_key(node);
    let mut guard = ack_connection_pool().lock().unwrap();
    guard.remove(&key)
}

fn return_pooled_stream(node: &NodeInfo, stream: TcpStream) {
    let key = pool_key(node);
    let mut guard = ack_connection_pool().lock().unwrap();
    guard.insert(key, stream);
}

/// Close and clear pooled ACK transport connections.
///
/// Returns number of pooled streams that were dropped.
pub fn close_pooled_ack_connections() -> usize {
    let mut guard = ack_connection_pool().lock().unwrap();
    let count = guard.len();
    guard.clear();
    count
}

fn configure_stream_timeouts(
    stream: &TcpStream,
    timeout: Duration,
) -> Result<(), ReplicationError> {
    stream
        .set_read_timeout(Some(timeout))
        .map_err(|e| ReplicationError::Network(format!("set read timeout: {e}")))?;
    stream
        .set_write_timeout(Some(timeout))
        .map_err(|e| ReplicationError::Network(format!("set write timeout: {e}")))?;
    Ok(())
}

fn request_single_ack_over_stream(
    stream: &mut TcpStream,
    node: &NodeInfo,
    op_id: [u8; 32],
) -> Result<(), ReplicationError> {
    if let Some(auth_key) = node.auth_key {
        let request = build_authenticated_ack_request(node.store_uuid, op_id, auth_key);
        stream
            .write_all(&request)
            .map_err(|e| ReplicationError::Network(format!("write request: {e}")))?;
        let mut response = [0u8; ACKV2_FRAME_LEN];
        stream
            .read_exact(&mut response)
            .map_err(|e| ReplicationError::Network(format!("read response: {e}")))?;
        verify_authenticated_ack_response(&response, node.store_uuid, op_id, auth_key)?;
        return Ok(());
    }

    let mut request = [0u8; 36];
    request[0..4].copy_from_slice(ACK_REQUEST_MAGIC);
    request[4..36].copy_from_slice(&op_id);
    stream
        .write_all(&request)
        .map_err(|e| ReplicationError::Network(format!("write request: {e}")))?;

    let mut response = [0u8; 4];
    stream
        .read_exact(&mut response)
        .map_err(|e| ReplicationError::Network(format!("read response: {e}")))?;
    if response != *ACK_RESPONSE_OK {
        return Err(ReplicationError::Network(format!(
            "invalid ACK response from {}",
            node.address
        )));
    }
    Ok(())
}

fn build_ack_batch_request(op_ids: &[[u8; 32]]) -> Result<Vec<u8>, ReplicationError> {
    if op_ids.len() > ACK_BATCH_MAX_OPS {
        return Err(ReplicationError::Network(
            "too many ops in batch".to_string(),
        ));
    }
    let mut frame = Vec::with_capacity(ACK_BATCH_HEADER_LEN + (op_ids.len() * 32));
    frame.extend_from_slice(ACK_BATCH_REQUEST_MAGIC);
    frame.extend_from_slice(&(op_ids.len() as u16).to_be_bytes());
    for op_id in op_ids {
        frame.extend_from_slice(op_id);
    }
    Ok(frame)
}

fn parse_ack_batch_response(frame: &[u8]) -> Result<Vec<[u8; 32]>, ReplicationError> {
    if frame.len() < ACK_BATCH_HEADER_LEN {
        return Err(ReplicationError::Network(
            "batch response frame too short".to_string(),
        ));
    }
    if &frame[0..4] != ACK_BATCH_RESPONSE_MAGIC {
        return Err(ReplicationError::Network(
            "invalid batch response magic".to_string(),
        ));
    }
    let mut count_bytes = [0u8; 2];
    count_bytes.copy_from_slice(&frame[4..6]);
    let count = u16::from_be_bytes(count_bytes) as usize;
    let expected = ACK_BATCH_HEADER_LEN + count * 32;
    if frame.len() != expected {
        return Err(ReplicationError::Network(
            "invalid batch response frame length".to_string(),
        ));
    }
    let mut acked = Vec::with_capacity(count);
    for chunk in frame[ACK_BATCH_HEADER_LEN..].chunks_exact(32) {
        let mut op_id = [0u8; 32];
        op_id.copy_from_slice(chunk);
        acked.push(op_id);
    }
    Ok(acked)
}

fn authenticated_batch_mac(
    key: [u8; 32],
    magic: &[u8; 4],
    store_uuid: [u8; 16],
    op_ids: &[[u8; 32]],
) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_keyed(&key);
    hasher.update(magic);
    hasher.update(&store_uuid);
    hasher.update(&(op_ids.len() as u16).to_be_bytes());
    for op_id in op_ids {
        hasher.update(op_id);
    }
    *hasher.finalize().as_bytes()
}

pub fn build_authenticated_ack_batch_request(
    store_uuid: [u8; 16],
    op_ids: &[[u8; 32]],
    auth_key: [u8; 32],
) -> Result<Vec<u8>, ReplicationError> {
    if op_ids.len() > ACK_BATCH_MAX_OPS {
        return Err(ReplicationError::AuthenticationFailed(
            "too many ops in batch".to_string(),
        ));
    }
    let mut frame = Vec::with_capacity(ACKV2_BATCH_HEADER_LEN + (op_ids.len() * 32) + 32);
    frame.extend_from_slice(ACKV2_BATCH_REQUEST_MAGIC);
    frame.extend_from_slice(&store_uuid);
    frame.extend_from_slice(&(op_ids.len() as u16).to_be_bytes());
    for op_id in op_ids {
        frame.extend_from_slice(op_id);
    }
    let mac = authenticated_batch_mac(auth_key, ACKV2_BATCH_REQUEST_MAGIC, store_uuid, op_ids);
    frame.extend_from_slice(&mac);
    Ok(frame)
}

pub fn verify_authenticated_ack_batch_request(
    frame: &[u8],
    expected_store_uuid: [u8; 16],
    auth_key: [u8; 32],
) -> Result<Vec<[u8; 32]>, ReplicationError> {
    if frame.len() < ACKV2_BATCH_HEADER_LEN + 32 {
        return Err(ReplicationError::AuthenticationFailed(
            "invalid batch request frame length".to_string(),
        ));
    }
    if &frame[0..4] != ACKV2_BATCH_REQUEST_MAGIC {
        return Err(ReplicationError::AuthenticationFailed(
            "invalid batch request magic".to_string(),
        ));
    }
    let mut store_uuid = [0u8; 16];
    store_uuid.copy_from_slice(&frame[4..20]);
    if store_uuid != expected_store_uuid {
        return Err(ReplicationError::AuthenticationFailed(
            "batch request store uuid mismatch".to_string(),
        ));
    }
    let mut count_bytes = [0u8; 2];
    count_bytes.copy_from_slice(&frame[20..22]);
    let count = u16::from_be_bytes(count_bytes) as usize;
    let ops_end = ACKV2_BATCH_HEADER_LEN + count * 32;
    let expected_len = ops_end + 32;
    if frame.len() != expected_len {
        return Err(ReplicationError::AuthenticationFailed(
            "invalid batch request ops length".to_string(),
        ));
    }
    let mut op_ids = Vec::with_capacity(count);
    for chunk in frame[ACKV2_BATCH_HEADER_LEN..ops_end].chunks_exact(32) {
        let mut op_id = [0u8; 32];
        op_id.copy_from_slice(chunk);
        op_ids.push(op_id);
    }
    let mut mac = [0u8; 32];
    mac.copy_from_slice(&frame[ops_end..expected_len]);
    let expected_mac =
        authenticated_batch_mac(auth_key, ACKV2_BATCH_REQUEST_MAGIC, store_uuid, &op_ids);
    if mac != expected_mac {
        return Err(ReplicationError::AuthenticationFailed(
            "batch request mac mismatch".to_string(),
        ));
    }
    Ok(op_ids)
}

pub fn build_authenticated_ack_batch_response(
    store_uuid: [u8; 16],
    acked_op_ids: &[[u8; 32]],
    auth_key: [u8; 32],
) -> Result<Vec<u8>, ReplicationError> {
    if acked_op_ids.len() > ACK_BATCH_MAX_OPS {
        return Err(ReplicationError::AuthenticationFailed(
            "too many ops in batch response".to_string(),
        ));
    }
    let mut frame = Vec::with_capacity(ACKV2_BATCH_HEADER_LEN + (acked_op_ids.len() * 32) + 32);
    frame.extend_from_slice(ACKV2_BATCH_RESPONSE_MAGIC);
    frame.extend_from_slice(&store_uuid);
    frame.extend_from_slice(&(acked_op_ids.len() as u16).to_be_bytes());
    for op_id in acked_op_ids {
        frame.extend_from_slice(op_id);
    }
    let mac = authenticated_batch_mac(
        auth_key,
        ACKV2_BATCH_RESPONSE_MAGIC,
        store_uuid,
        acked_op_ids,
    );
    frame.extend_from_slice(&mac);
    Ok(frame)
}

pub fn verify_authenticated_ack_batch_response(
    frame: &[u8],
    expected_store_uuid: [u8; 16],
    auth_key: [u8; 32],
) -> Result<Vec<[u8; 32]>, ReplicationError> {
    if frame.len() < ACKV2_BATCH_HEADER_LEN + 32 {
        return Err(ReplicationError::AuthenticationFailed(
            "invalid batch response frame length".to_string(),
        ));
    }
    if &frame[0..4] != ACKV2_BATCH_RESPONSE_MAGIC {
        return Err(ReplicationError::AuthenticationFailed(
            "invalid batch response magic".to_string(),
        ));
    }
    let mut store_uuid = [0u8; 16];
    store_uuid.copy_from_slice(&frame[4..20]);
    if store_uuid != expected_store_uuid {
        return Err(ReplicationError::AuthenticationFailed(
            "batch response store uuid mismatch".to_string(),
        ));
    }
    let mut count_bytes = [0u8; 2];
    count_bytes.copy_from_slice(&frame[20..22]);
    let count = u16::from_be_bytes(count_bytes) as usize;
    let ops_end = ACKV2_BATCH_HEADER_LEN + count * 32;
    let expected_len = ops_end + 32;
    if frame.len() != expected_len {
        return Err(ReplicationError::AuthenticationFailed(
            "invalid batch response ops length".to_string(),
        ));
    }
    let mut op_ids = Vec::with_capacity(count);
    for chunk in frame[ACKV2_BATCH_HEADER_LEN..ops_end].chunks_exact(32) {
        let mut op_id = [0u8; 32];
        op_id.copy_from_slice(chunk);
        op_ids.push(op_id);
    }
    let mut mac = [0u8; 32];
    mac.copy_from_slice(&frame[ops_end..expected_len]);
    let expected_mac =
        authenticated_batch_mac(auth_key, ACKV2_BATCH_RESPONSE_MAGIC, store_uuid, &op_ids);
    if mac != expected_mac {
        return Err(ReplicationError::AuthenticationFailed(
            "batch response mac mismatch".to_string(),
        ));
    }
    Ok(op_ids)
}

fn request_batch_ack_over_stream(
    stream: &mut TcpStream,
    node: &NodeInfo,
    op_ids: &[[u8; 32]],
) -> Result<Vec<[u8; 32]>, ReplicationError> {
    if op_ids.is_empty() {
        return Ok(Vec::new());
    }

    if let Some(auth_key) = node.auth_key {
        let request = build_authenticated_ack_batch_request(node.store_uuid, op_ids, auth_key)?;
        stream
            .write_all(&request)
            .map_err(|e| ReplicationError::Network(format!("write batch request: {e}")))?;

        let mut header = [0u8; ACKV2_BATCH_HEADER_LEN];
        stream
            .read_exact(&mut header)
            .map_err(|e| ReplicationError::Network(format!("read batch response header: {e}")))?;
        if &header[0..4] != ACKV2_BATCH_RESPONSE_MAGIC {
            return Err(ReplicationError::AuthenticationFailed(
                "invalid batch response magic".to_string(),
            ));
        }
        let mut count_bytes = [0u8; 2];
        count_bytes.copy_from_slice(&header[20..22]);
        let count = u16::from_be_bytes(count_bytes) as usize;
        let body_len = (count * 32) + 32;
        let mut body = vec![0u8; body_len];
        stream
            .read_exact(&mut body)
            .map_err(|e| ReplicationError::Network(format!("read batch response body: {e}")))?;
        let mut frame = Vec::with_capacity(ACKV2_BATCH_HEADER_LEN + body_len);
        frame.extend_from_slice(&header);
        frame.extend_from_slice(&body);
        return verify_authenticated_ack_batch_response(&frame, node.store_uuid, auth_key);
    }

    let request = build_ack_batch_request(op_ids)?;
    stream
        .write_all(&request)
        .map_err(|e| ReplicationError::Network(format!("write batch request: {e}")))?;

    let mut header = [0u8; ACK_BATCH_HEADER_LEN];
    stream
        .read_exact(&mut header)
        .map_err(|e| ReplicationError::Network(format!("read batch response header: {e}")))?;
    if &header[0..4] != ACK_BATCH_RESPONSE_MAGIC {
        return Err(ReplicationError::Network(
            "invalid batch response magic".to_string(),
        ));
    }
    let mut count_bytes = [0u8; 2];
    count_bytes.copy_from_slice(&header[4..6]);
    let count = u16::from_be_bytes(count_bytes) as usize;
    let body_len = count * 32;
    let mut body = vec![0u8; body_len];
    stream
        .read_exact(&mut body)
        .map_err(|e| ReplicationError::Network(format!("read batch response body: {e}")))?;

    let mut frame = Vec::with_capacity(ACK_BATCH_HEADER_LEN + body_len);
    frame.extend_from_slice(&header);
    frame.extend_from_slice(&body);
    parse_ack_batch_response(&frame)
}

/// Request an ACK from a remote node using a tiny TCP protocol.
///
/// Protocol:
/// - client -> server: `ACK?` + 32-byte op id
/// - server -> client: `ACK\n` on success
pub fn request_network_ack(
    node: &NodeInfo,
    op_id: [u8; 32],
    timeout: Duration,
) -> Result<(), ReplicationError> {
    let mut stream = if let Some(s) = try_take_pooled_stream(node) {
        s
    } else {
        connect_stream(node, timeout)?
    };
    configure_stream_timeouts(&stream, timeout)?;
    let result = request_single_ack_over_stream(&mut stream, node, op_id);
    if result.is_ok() {
        return_pooled_stream(node, stream);
    }
    result
}

/// Stream-oriented ACK collection for one node with retries/backoff and idempotency window.
///
/// Reuses a single TCP connection when possible and retries failed operations with reconnects.
pub fn request_network_acks_stream(
    node: &NodeInfo,
    op_ids: &[[u8; 32]],
    timeout: Duration,
    retry_policy: RetryPolicy,
) -> Vec<[u8; 32]> {
    let started = Instant::now();
    let mut acked = Vec::new();
    let mut stream: Option<TcpStream> = try_take_pooled_stream(node);
    let mut dedupe = IdempotencyWindow::new(op_ids.len().max(1) * 2);
    let mut pending = Vec::new();
    for &op_id in op_ids {
        if dedupe.observe(op_id) {
            pending.push(op_id);
        }
    }

    if pending.len() > 1 {
        let mut attempt = 0u8;
        while attempt <= retry_policy.max_retries {
            let elapsed = started.elapsed();
            if elapsed >= timeout {
                break;
            }
            let remaining = timeout.saturating_sub(elapsed);
            if stream.is_none() {
                match connect_stream(node, remaining) {
                    Ok(s) => {
                        if configure_stream_timeouts(&s, remaining).is_ok() {
                            stream = Some(s);
                        } else {
                            stream = None;
                        }
                    }
                    Err(_) => {
                        stream = None;
                    }
                }
            }
            let Some(s) = stream.as_mut() else {
                if attempt == retry_policy.max_retries {
                    break;
                }
                let backoff = retry_policy.backoff_base.saturating_mul(1u32 << attempt);
                thread::sleep(backoff.min(Duration::from_millis(100)));
                attempt = attempt.saturating_add(1);
                continue;
            };
            match request_batch_ack_over_stream(s, node, &pending) {
                Ok(mut batch_acked) => {
                    acked.append(&mut batch_acked);
                    break;
                }
                Err(_) => {
                    stream = None;
                    if attempt == retry_policy.max_retries {
                        break;
                    }
                    let backoff = retry_policy.backoff_base.saturating_mul(1u32 << attempt);
                    thread::sleep(backoff.min(Duration::from_millis(100)));
                    attempt = attempt.saturating_add(1);
                }
            }
        }
        if let Some(s) = stream {
            return_pooled_stream(node, s);
        }
        return acked;
    }

    for &op_id in &pending {
        let mut attempt = 0u8;
        let mut op_acked = false;
        while attempt <= retry_policy.max_retries {
            let elapsed = started.elapsed();
            if elapsed >= timeout {
                break;
            }
            let remaining = timeout.saturating_sub(elapsed);

            if stream.is_none() {
                match connect_stream(node, remaining) {
                    Ok(s) => {
                        if configure_stream_timeouts(&s, remaining).is_ok() {
                            stream = Some(s);
                        } else {
                            stream = None;
                        }
                    }
                    Err(_) => {
                        stream = None;
                    }
                }
            }

            let Some(s) = stream.as_mut() else {
                if attempt == retry_policy.max_retries {
                    break;
                }
                let backoff = retry_policy.backoff_base.saturating_mul(1u32 << attempt);
                thread::sleep(backoff.min(Duration::from_millis(100)));
                attempt = attempt.saturating_add(1);
                continue;
            };

            match request_single_ack_over_stream(s, node, op_id) {
                Ok(()) => {
                    acked.push(op_id);
                    op_acked = true;
                    break;
                }
                Err(_) => {
                    // Connection likely stale/broken. Reconnect on next attempt.
                    stream = None;
                    if attempt == retry_policy.max_retries {
                        break;
                    }
                    let backoff = retry_policy.backoff_base.saturating_mul(1u32 << attempt);
                    thread::sleep(backoff.min(Duration::from_millis(100)));
                    attempt = attempt.saturating_add(1);
                }
            }
        }
        if !op_acked {
            // continue to next op id; best-effort collection
        }
    }
    if let Some(s) = stream {
        return_pooled_stream(node, s);
    }
    acked
}

/// Collect remote ACKs over the network until nodes are exhausted or timeout budget is reached.
pub fn collect_network_acks(
    nodes: &[NodeInfo],
    op_id: [u8; 32],
    timeout: Duration,
) -> Vec<ActorId> {
    collect_network_acks_for_ops(nodes, &[op_id], timeout)
        .remove(&op_id)
        .unwrap_or_default()
}

/// Collect remote ACKs for many op ids.
///
/// Returns a map of `op_id -> acking remote actor_ids`.
pub fn collect_network_acks_for_ops(
    nodes: &[NodeInfo],
    op_ids: &[[u8; 32]],
    timeout: Duration,
) -> HashMap<[u8; 32], Vec<ActorId>> {
    let mut per_op: HashMap<[u8; 32], Vec<ActorId>> =
        op_ids.iter().copied().map(|op| (op, Vec::new())).collect();
    if op_ids.is_empty() {
        return per_op;
    }

    let started = Instant::now();
    let retry_policy = RetryPolicy::default();
    for node in nodes {
        let elapsed = started.elapsed();
        if elapsed >= timeout {
            break;
        }
        let remaining = timeout.saturating_sub(elapsed);
        let acked_ops = request_network_acks_stream(node, op_ids, remaining, retry_policy);
        for acked_op in acked_ops {
            if let Some(peers) = per_op.get_mut(&acked_op) {
                peers.push(node.actor_id.clone());
            }
        }
    }
    per_op
}

impl AntiEntropy {
    pub fn new(replicator: Replicator) -> Self {
        Self { replicator }
    }

    pub fn exchange_manifests(&self, remote_summary: &ManifestSummary) -> Vec<[u8; 32]> {
        self.replicator.compute_missing_segments(remote_summary)
    }

    pub fn sync_segments(&self, missing: Vec<[u8; 32]>) -> Result<(), ReplicationError> {
        for segment_id in missing {
            if !self.replicator.has_segment(&segment_id) {
                return Err(ReplicationError::SegmentNotFound);
            }
        }
        Ok(())
    }

    pub fn verify_and_apply(&self, segment_id: [u8; 32], data: &[u8], hash: [u8; 32]) -> bool {
        self.replicator.verify_segment(segment_id, data, &hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn test_node_info() {
        let node = NodeInfo::new(ActorId::new(), "192.168.1.1:8080".to_string(), [1u8; 16]);

        assert!(!node.address.is_empty());
    }

    #[test]
    fn test_replicator_nodes() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);

        let node = NodeInfo::new(ActorId::new(), "localhost:8080".to_string(), [1u8; 16]);
        replicator.add_node(node);

        let nodes = replicator.get_nodes();
        assert_eq!(nodes.len(), 1);
    }

    #[test]
    fn test_replicator_remove_node() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);

        let actor = ActorId::new();
        let node = NodeInfo::new(actor.clone(), "localhost:8080".to_string(), [1u8; 16]);
        replicator.add_node(node);

        replicator.remove_node(&actor);

        let nodes = replicator.get_nodes();
        assert!(nodes.is_empty());
    }

    #[test]
    fn test_replicator_segments() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);

        let segment_id: [u8; 32] = [1u8; 32];
        replicator.add_segment(segment_id);

        assert!(replicator.has_segment(&segment_id));

        let segments = replicator.get_segments();
        assert_eq!(segments.len(), 1);
    }

    #[test]
    fn test_replicator_frontier() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);

        let actor_id: [u8; 16] = [1u8; 16];
        replicator.update_frontier(actor_id, 10);

        let frontier = replicator.get_frontier();
        assert_eq!(frontier.get(&actor_id), 1);
    }

    #[test]
    fn test_manifest_summary() {
        let summary = ManifestSummary {
            store_uuid: [1u8; 16],
            epoch: 5,
            segment_ids: vec![[1u8; 32], [2u8; 32]],
            frontier: FrontierSummary::default(),
        };

        assert_eq!(summary.segment_ids.len(), 2);
    }

    #[test]
    fn test_merkle_tree_empty() {
        let tree = MerkleTree::new();

        assert!(tree.root_hash().is_none());
    }

    #[test]
    fn test_merkle_tree_single() {
        let tree = MerkleTree::from_segments(&[[1u8; 32]]);

        assert!(tree.root_hash().is_some());
    }

    #[test]
    fn test_merkle_tree_multiple() {
        let segments = [[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];

        let tree = MerkleTree::from_segments(&segments);

        assert!(tree.root_hash().is_some());
        assert!(!tree.root_hash().unwrap().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_merkle_tree_diff() {
        let tree1 = MerkleTree::from_segments(&[[1u8; 32], [2u8; 32]]);
        let tree2 = MerkleTree::from_segments(&[[1u8; 32], [3u8; 32]]);

        let diff = tree1.diff(&tree2);

        assert!(!diff.is_empty());
    }

    #[test]
    fn test_anti_entropy_exchange() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);
        let ae = AntiEntropy::new(replicator);

        let remote_summary = ManifestSummary {
            store_uuid: [1u8; 16],
            epoch: 1,
            segment_ids: vec![[1u8; 32], [2u8; 32]],
            frontier: FrontierSummary::default(),
        };

        let missing = ae.exchange_manifests(&remote_summary);

        assert_eq!(missing.len(), 2);
    }

    #[test]
    fn test_compute_missing_segments() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);

        replicator.add_segment([1u8; 32]);

        let summary = ManifestSummary {
            store_uuid: [1u8; 16],
            epoch: 1,
            segment_ids: vec![[1u8; 32], [2u8; 32]],
            frontier: FrontierSummary::default(),
        };

        let missing = replicator.compute_missing_segments(&summary);

        assert_eq!(missing.len(), 1);
    }

    #[test]
    fn test_verify_segment() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);

        let segment_id: [u8; 32] = [1u8; 32];
        let data = b"test segment data";

        let hash = *blake3::hash(data).as_bytes();

        let result = replicator.verify_segment(segment_id, data, &hash);

        assert!(result);
    }

    #[test]
    fn test_verify_segment_invalid() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);

        let segment_id: [u8; 32] = [1u8; 32];
        let data = b"test segment data";
        let wrong_hash: [u8; 32] = [0u8; 32];

        let result = replicator.verify_segment(segment_id, data, &wrong_hash);

        assert!(!result);
    }

    #[test]
    fn test_anti_entropy_sync_segments() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);
        let ae = AntiEntropy::new(replicator);

        let result = ae.sync_segments(vec![]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_anti_entropy_verify_and_apply() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);
        let ae = AntiEntropy::new(replicator);

        let segment_id: [u8; 32] = [1u8; 32];
        let data = b"test data";
        let hash = *blake3::hash(data).as_bytes();

        let result = ae.verify_and_apply(segment_id, data, hash);
        assert!(result);
    }

    #[test]
    fn test_merkle_tree_diff_different_structures() {
        // Tree with 1 leaf vs tree with 2 leaves - different structures
        let tree1 = MerkleTree::from_segments(&[[1u8; 32]]);
        let tree2 = MerkleTree::from_segments(&[[1u8; 32], [2u8; 32]]);

        let diff = tree1.diff(&tree2);
        // Should have some differences
        assert!(!diff.is_empty() || tree1.root_hash() == tree2.root_hash());
    }

    #[test]
    fn test_merkle_tree_from_segments_empty() {
        let tree = MerkleTree::from_segments(&[]);
        assert!(tree.root_hash().is_none());
    }

    #[test]
    fn test_create_merkle_tree() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);

        replicator.add_segment([1u8; 32]);
        replicator.add_segment([2u8; 32]);

        let tree = replicator.create_merkle_tree();
        assert!(tree.root_hash().is_some());
    }

    #[test]
    fn test_replicator_get_segments() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);

        replicator.add_segment([1u8; 32]);
        replicator.add_segment([2u8; 32]);

        let segments = replicator.get_segments();
        assert_eq!(segments.len(), 2);
    }

    #[test]
    fn test_frontier_summary_default() {
        let summary = FrontierSummary::default();
        assert!(summary.per_actor.is_empty());
    }

    #[test]
    fn test_anti_entropy_sync_segments_not_found() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);
        let ae = AntiEntropy::new(replicator);

        // Try to sync segments that don't exist
        let result = ae.sync_segments(vec![[1u8; 32]]);
        assert!(matches!(result, Err(ReplicationError::SegmentNotFound)));
    }

    #[test]
    fn test_manifest_summary_neq() {
        let s1 = ManifestSummary {
            store_uuid: [1u8; 16],
            epoch: 1,
            segment_ids: vec![[1u8; 32]],
            frontier: FrontierSummary::default(),
        };
        let s2 = ManifestSummary {
            store_uuid: [2u8; 16],
            epoch: 2,
            segment_ids: vec![[2u8; 32]],
            frontier: FrontierSummary::default(),
        };

        // These are different
        assert!(s1.store_uuid != s2.store_uuid);
    }

    #[test]
    fn test_local_manifest_summary() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);

        replicator.add_segment([1u8; 32]);

        let summary = replicator.get_local_manifest_summary();

        assert_eq!(summary.segment_ids.len(), 1);
    }

    #[test]
    fn test_has_segment() {
        let replicator = Replicator::new(ActorId::new(), [1u8; 16]);

        replicator.add_segment([1u8; 32]);

        assert!(replicator.has_segment(&[1u8; 32]));
        assert!(!replicator.has_segment(&[2u8; 32]));
    }

    #[test]
    fn test_quorum_tracker_satisfied() {
        let peer = NodeInfo::new(ActorId::new(), "n2:1".to_string(), [2u8; 16]);
        let mut tracker = QuorumTracker::new(2, Duration::from_secs(1), &[peer.clone()]).unwrap();
        tracker.ack_local_durable();
        tracker.ack_remote(&peer.actor_id).unwrap();
        assert!(tracker.is_satisfied());
        assert!(tracker.finalize().is_ok());
    }

    #[test]
    fn test_quorum_tracker_not_met() {
        let peer = NodeInfo::new(ActorId::new(), "n2:1".to_string(), [2u8; 16]);
        let mut tracker = QuorumTracker::new(2, Duration::from_secs(1), &[peer]).unwrap();
        tracker.ack_local_durable();
        let err = tracker.finalize().unwrap_err();
        assert!(matches!(err, ReplicationError::QuorumNotMet { .. }));
    }

    #[test]
    fn test_quorum_tracker_invalid_requirement() {
        let err = QuorumTracker::new(3, Duration::from_secs(1), &[]).unwrap_err();
        assert!(matches!(
            err,
            ReplicationError::InvalidQuorumRequirement { .. }
        ));
    }

    #[test]
    fn test_request_network_ack_success() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let node = NodeInfo::new(ActorId::new(), addr.to_string(), [9u8; 16]);
        let op_id = [7u8; 32];

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut req = [0u8; 36];
            stream.read_exact(&mut req).unwrap();
            assert_eq!(&req[0..4], ACK_REQUEST_MAGIC);
            stream.write_all(ACK_RESPONSE_OK).unwrap();
        });

        let result = request_network_ack(&node, op_id, Duration::from_secs(1));
        server.join().unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_collect_network_acks_mixed_results() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let good_addr = listener.local_addr().unwrap();
        let good = NodeInfo::new(ActorId::new(), good_addr.to_string(), [1u8; 16]);
        let bad = NodeInfo::new(ActorId::new(), "127.0.0.1:1".to_string(), [2u8; 16]);

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut req = [0u8; 36];
            stream.read_exact(&mut req).unwrap();
            stream.write_all(ACK_RESPONSE_OK).unwrap();
        });

        let acked =
            collect_network_acks(&[good.clone(), bad], [3u8; 32], Duration::from_millis(200));
        server.join().unwrap();

        assert_eq!(acked.len(), 1);
        assert_eq!(acked[0], good.actor_id);
    }

    #[test]
    fn test_request_network_ack_authenticated_success() {
        let auth_key = derive_auth_key(b"shared-secret");
        let store_uuid = [4u8; 16];
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let node =
            NodeInfo::new(ActorId::new(), addr.to_string(), store_uuid).with_auth_key(auth_key);
        let op_id = [5u8; 32];

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut req = [0u8; ACKV2_FRAME_LEN];
            stream.read_exact(&mut req).unwrap();
            let parsed = verify_authenticated_ack_request(&req, store_uuid, auth_key).unwrap();
            assert_eq!(parsed, op_id);
            let resp = build_authenticated_ack_response(store_uuid, op_id, auth_key);
            stream.write_all(&resp).unwrap();
        });

        let result = request_network_ack(&node, op_id, Duration::from_secs(1));
        server.join().unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn test_request_network_ack_authenticated_bad_mac() {
        let auth_key_client = derive_auth_key(b"client-secret");
        let auth_key_server = derive_auth_key(b"server-secret");
        let store_uuid = [6u8; 16];
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let node = NodeInfo::new(ActorId::new(), addr.to_string(), store_uuid)
            .with_auth_key(auth_key_client);
        let op_id = [7u8; 32];

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut req = [0u8; ACKV2_FRAME_LEN];
            stream.read_exact(&mut req).unwrap();
            let parsed =
                verify_authenticated_ack_request(&req, store_uuid, auth_key_client).unwrap();
            let resp = build_authenticated_ack_response(store_uuid, parsed, auth_key_server);
            stream.write_all(&resp).unwrap();
        });

        let err = request_network_ack(&node, op_id, Duration::from_secs(1)).unwrap_err();
        server.join().unwrap();
        assert!(matches!(err, ReplicationError::AuthenticationFailed(_)));
    }

    #[test]
    fn test_request_network_acks_stream_dedupes_operation_ids() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let node = NodeInfo::new(ActorId::new(), addr.to_string(), [9u8; 16]);
        let op_id = [8u8; 32];

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            for _ in 0..1 {
                let mut req = [0u8; 36];
                stream.read_exact(&mut req).unwrap();
                assert_eq!(&req[0..4], ACK_REQUEST_MAGIC);
                stream.write_all(ACK_RESPONSE_OK).unwrap();
            }
        });

        let acked = request_network_acks_stream(
            &node,
            &[op_id, op_id],
            Duration::from_secs(1),
            RetryPolicy::default(),
        );
        server.join().unwrap();
        assert_eq!(acked.len(), 1);
        assert_eq!(acked[0], op_id);
    }

    #[test]
    fn test_request_network_acks_stream_retries_after_disconnect() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let node = NodeInfo::new(ActorId::new(), addr.to_string(), [9u8; 16]);
        let op_id = [3u8; 32];

        let server = thread::spawn(move || {
            // First connection: drop without response.
            let (stream1, _) = listener.accept().unwrap();
            drop(stream1);

            // Second connection: normal ACK path.
            let (mut stream2, _) = listener.accept().unwrap();
            let mut req = [0u8; 36];
            stream2.read_exact(&mut req).unwrap();
            assert_eq!(&req[0..4], ACK_REQUEST_MAGIC);
            stream2.write_all(ACK_RESPONSE_OK).unwrap();
        });

        let acked = request_network_acks_stream(
            &node,
            &[op_id],
            Duration::from_secs(1),
            RetryPolicy {
                max_retries: 2,
                backoff_base: Duration::from_millis(5),
            },
        );
        server.join().unwrap();
        assert_eq!(acked, vec![op_id]);
    }

    #[test]
    fn test_request_network_acks_stream_batches_multiple_ops() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let node = NodeInfo::new(ActorId::new(), addr.to_string(), [3u8; 16]);
        let op_a = [0xAA; 32];
        let op_b = [0xBB; 32];
        let op_c = [0xCC; 32];

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut header = [0u8; ACK_BATCH_HEADER_LEN];
            stream.read_exact(&mut header).unwrap();
            assert_eq!(&header[0..4], ACK_BATCH_REQUEST_MAGIC);
            let mut count_bytes = [0u8; 2];
            count_bytes.copy_from_slice(&header[4..6]);
            let count = u16::from_be_bytes(count_bytes) as usize;
            assert_eq!(count, 3);
            let mut body = vec![0u8; count * 32];
            stream.read_exact(&mut body).unwrap();

            let mut response = Vec::with_capacity(ACK_BATCH_HEADER_LEN + body.len());
            response.extend_from_slice(ACK_BATCH_RESPONSE_MAGIC);
            response.extend_from_slice(&(count as u16).to_be_bytes());
            response.extend_from_slice(&body);
            stream.write_all(&response).unwrap();
        });

        let acked = request_network_acks_stream(
            &node,
            &[op_a, op_b, op_c],
            Duration::from_secs(1),
            RetryPolicy::default(),
        );
        server.join().unwrap();
        assert_eq!(acked, vec![op_a, op_b, op_c]);
    }

    #[test]
    fn test_request_network_acks_stream_batches_multiple_ops_authenticated() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let auth_key = derive_auth_key(b"batch-auth-key");
        let store_uuid = [0x11; 16];
        let node =
            NodeInfo::new(ActorId::new(), addr.to_string(), store_uuid).with_auth_key(auth_key);
        let op_a = [0x1A; 32];
        let op_b = [0x2B; 32];

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut header = [0u8; ACKV2_BATCH_HEADER_LEN];
            stream.read_exact(&mut header).unwrap();
            assert_eq!(&header[0..4], ACKV2_BATCH_REQUEST_MAGIC);
            let mut count_bytes = [0u8; 2];
            count_bytes.copy_from_slice(&header[20..22]);
            let count = u16::from_be_bytes(count_bytes) as usize;
            let body_len = (count * 32) + 32;
            let mut body = vec![0u8; body_len];
            stream.read_exact(&mut body).unwrap();

            let mut frame = Vec::with_capacity(header.len() + body.len());
            frame.extend_from_slice(&header);
            frame.extend_from_slice(&body);
            let op_ids =
                verify_authenticated_ack_batch_request(&frame, store_uuid, auth_key).unwrap();
            assert_eq!(op_ids, vec![op_a, op_b]);

            let resp =
                build_authenticated_ack_batch_response(store_uuid, &op_ids, auth_key).unwrap();
            stream.write_all(&resp).unwrap();
        });

        let acked = request_network_acks_stream(
            &node,
            &[op_a, op_b],
            Duration::from_secs(1),
            RetryPolicy::default(),
        );
        server.join().unwrap();
        assert_eq!(acked, vec![op_a, op_b]);
    }

    #[test]
    fn test_request_network_ack_reuses_pooled_connection() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let node = NodeInfo::new(ActorId::new(), addr.to_string(), [7u8; 16]);
        let op_a = [0x0A; 32];
        let op_b = [0x0B; 32];

        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            for _ in 0..2 {
                let mut req = [0u8; 36];
                stream.read_exact(&mut req).unwrap();
                assert_eq!(&req[0..4], ACK_REQUEST_MAGIC);
                stream.write_all(ACK_RESPONSE_OK).unwrap();
            }
        });

        request_network_ack(&node, op_a, Duration::from_secs(1)).unwrap();
        request_network_ack(&node, op_b, Duration::from_secs(1)).unwrap();

        server.join().unwrap();
    }
}
