use alloc::string::String;
use alloc::vec::Vec;

use rkyv::{Archive, Deserialize, Serialize};

use crate::channel::ChannelEndpoint;

pub const WORK_WIRE_ABI_VERSION: u16 = 1;
pub const DEFAULT_HEARTBEAT_SECS: u64 = 10;
pub const MAX_WORK_FRAME_LEN: usize = 1024 * 1024;

pub const NODE_ROLE_RELAY: u16 = 1;
pub const NODE_ROLE_STORAGE: u16 = 2;
pub const NODE_ROLE_COMPUTE: u16 = 3;
pub const NODE_ROLE_ADMISSION: u16 = 4;
pub const NODE_ROLE_MESSAGE: u16 = 5;

pub const WORK_TYPE_MESSAGE_DELIVER: u16 = 1;
pub const WORK_TYPE_OBJECT_STORE: u16 = 2;
pub const WORK_TYPE_OBJECT_RETRIEVE: u16 = 3;
pub const WORK_TYPE_OBJECT_PIN: u16 = 4;
pub const WORK_TYPE_COMPUTE_RUN: u16 = 5;

pub const DEPARTMENT_ADMISSION: u16 = 1;
pub const DEPARTMENT_RELAY: u16 = 2;
pub const DEPARTMENT_MESSAGE: u16 = 3;
pub const DEPARTMENT_STORAGE: u16 = 4;
pub const DEPARTMENT_RETRIEVAL: u16 = 5;
pub const DEPARTMENT_COMPUTE: u16 = 6;

pub const SIGNATURE_ALGORITHM_SOLANA_ED25519: u16 = 101;

pub type Hash = [u8; 32];
pub type NodeId = [u8; 32];
pub type PublicKey = [u8; 32];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkProtocolError {
    EmptyPacket,
    PacketTooLarge,
    InvalidPacket,
    InvalidShape,
    InvalidSignature,
    HashMismatch,
    WrongRelay,
    NoRelayAvailable,
    UnknownNode,
    Duplicate,
    Expired,
    Unsupported,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct WorkSignature {
    pub algorithm: u16,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct NodeIdentity {
    pub node_id: NodeId,
    pub role: u16,
    pub public_key: PublicKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelayEndpoint {
    pub relay_node_id: NodeId,
    pub host: String,
    pub port: u16,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct NodeAvailable {
    pub abi_version: u16,
    pub node: NodeIdentity,
    pub sequence: u64,
    pub unix_ms: u64,
    pub listen_host: String,
    pub listen_port: u16,
    pub heartbeat_secs: u64,
    pub log_head: Hash,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct NodeHeartbeat {
    pub abi_version: u16,
    pub node: NodeIdentity,
    pub sequence: u64,
    pub unix_ms: u64,
    pub connection_hash: Hash,
    pub log_head: Hash,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelayPeerList {
    pub abi_version: u16,
    pub assigned_to: NodeId,
    pub relays: Vec<RelayEndpoint>,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct RelayAssignment {
    pub abi_version: u16,
    pub node_id: NodeId,
    pub relay: RelayEndpoint,
    pub assigned_by: NodeIdentity,
    pub sequence: u64,
    pub valid_until_unix_ms: u64,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct NetworkMessage {
    pub abi_version: u16,
    pub message_id: Hash,
    pub prev_hash: Hash,
    pub from: NodeId,
    pub to: NodeId,
    pub via_relay: NodeId,
    pub department: u16,
    pub work_type: u16,
    pub sequence: u64,
    pub payload_hash: Hash,
    pub payload: Vec<u8>,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct WorkRequest {
    pub abi_version: u16,
    pub request_id: Hash,
    pub user: PublicKey,
    pub user_sequence: u64,
    pub recipient: NodeId,
    pub work_type: u16,
    pub department: u16,
    pub payload_hash: Hash,
    pub input_root: Hash,
    pub max_total_cost: u64,
    pub valid_until_unix_ms: u64,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct WorkAdmission {
    pub abi_version: u16,
    pub admission_id: Hash,
    pub dao_id: PublicKey,
    pub user: PublicKey,
    pub admission_node: NodeIdentity,
    pub request_hash: Hash,
    pub assigned_route_hash: Hash,
    pub assigned_channel: ChannelEndpoint,
    pub admitted_budget: u64,
    pub policy_hash: Hash,
    pub sequence: u64,
    pub valid_until_unix_ms: u64,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct WorkReceipt {
    pub abi_version: u16,
    pub receipt_id: Hash,
    pub request_hash: Hash,
    pub admission_hash: Hash,
    pub worker: NodeIdentity,
    pub relay_node_id: NodeId,
    pub input_hash: Hash,
    pub output_hash: Hash,
    pub units_used: u64,
    pub total_claim: u64,
    pub sequence: u64,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub enum WorkPacket {
    NodeAvailable(NodeAvailable),
    NodeHeartbeat(NodeHeartbeat),
    RelayPeerList(RelayPeerList),
    RelayAssignment(RelayAssignment),
    NetworkMessage(NetworkMessage),
    WorkRequest(WorkRequest),
    WorkAdmission(WorkAdmission),
    WorkReceipt(WorkReceipt),
    Ack(WorkAck),
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = rkyv)]
pub struct WorkAck {
    pub ok: bool,
    pub code: u16,
    pub text: String,
}
