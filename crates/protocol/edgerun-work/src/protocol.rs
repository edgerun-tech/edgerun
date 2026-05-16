use alloc::string::String;
use alloc::vec::Vec;

use crate::channel::ChannelEndpoint;
pub use crate::node_control::{NodeAvailable, NodeHeartbeat, RelayAssignment, RelayEndpoint};

pub const WORK_WIRE_ABI_VERSION: u16 = 1;
pub const DEFAULT_HEARTBEAT_SECS: u64 = 10;
pub const MAX_WORK_FRAME_LEN: usize = 1024 * 1024;

pub const NODE_ROLE_RELAY: u16 = 1;
pub const NODE_ROLE_STORAGE: u16 = 2;
pub const NODE_ROLE_COMPUTE: u16 = 3;
pub const NODE_ROLE_ADMISSION: u16 = 4;
pub const NODE_ROLE_MESSAGE: u16 = 5;
pub const NODE_ROLE_CAPABILITY: u16 = 6;
pub const NODE_ROLE_NOTARY: u16 = 7;
pub const NODE_ROLE_VERIFIER: u16 = 8;

pub const WORK_TYPE_MESSAGE_DELIVER: u16 = 1;
pub const WORK_TYPE_OBJECT_STORE: u16 = 2;
pub const WORK_TYPE_OBJECT_RETRIEVE: u16 = 3;
pub const WORK_TYPE_OBJECT_PIN: u16 = 4;
pub const WORK_TYPE_COMPUTE_RUN: u16 = 5;
pub const WORK_TYPE_PROGRAM_OPEN: u16 = 6;
pub const WORK_TYPE_PROGRAM_STDIN: u16 = 7;
pub const WORK_TYPE_PROGRAM_CLOSE: u16 = 8;
pub const WORK_TYPE_PROGRAM_POLL: u16 = 9;
pub const WORK_TYPE_PROGRAM_EVENT: u16 = 10;
pub const WORK_TYPE_CAPABILITY_REQUEST: u16 = 11;
pub const WORK_TYPE_CAPABILITY_INVOKE: u16 = 12;
pub const WORK_TYPE_CAPABILITY_EVENT: u16 = 13;
pub const WORK_TYPE_CAPABILITY_CLOSE: u16 = 14;
pub const WORK_TYPE_NOTARY_SEAL: u16 = 15;
pub const WORK_TYPE_NOTARY_UNSEAL: u16 = 16;
pub const WORK_TYPE_VERIFY_CLAIM: u16 = 17;

pub const DEPARTMENT_ADMISSION: u16 = 1;
pub const DEPARTMENT_RELAY: u16 = 2;
pub const DEPARTMENT_MESSAGE: u16 = 3;
pub const DEPARTMENT_STORAGE: u16 = 4;
pub const DEPARTMENT_RETRIEVAL: u16 = 5;
pub const DEPARTMENT_COMPUTE: u16 = 6;
pub const DEPARTMENT_CAPABILITY: u16 = 7;
pub const DEPARTMENT_NOTARY: u16 = 8;
pub const DEPARTMENT_VERIFICATION: u16 = 9;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkSignature {
    pub algorithm: u16,
    pub public_key: Vec<u8>,
    pub signature: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeIdentity {
    pub node_id: NodeId,
    pub role: u16,
    pub public_key: PublicKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkAdmission {
    pub abi_version: u16,
    pub admission_id: Hash,
    pub dao_id: PublicKey,
    pub user: PublicKey,
    pub admission_node: NodeIdentity,
    pub request_hash: Hash,
    pub assigned_route_commitment: Hash,
    pub assigned_channel: ChannelEndpoint,
    pub assigned_relay_path: Vec<NodeId>,
    pub admitted_budget: u64,
    pub policy_hash: Hash,
    pub sequence: u64,
    pub valid_until_unix_ms: u64,
    pub signature: WorkSignature,
}

#[derive(Clone, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorkPacket {
    NodeAvailable(NodeAvailable),
    NodeHeartbeat(NodeHeartbeat),
    RelayAssignment(RelayAssignment),
    NetworkMessage(NetworkMessage),
    WorkRequest(WorkRequest),
    WorkAdmission(WorkAdmission),
    WorkReceipt(WorkReceipt),
    Ack(WorkAck),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkAck {
    pub ok: bool,
    pub code: u16,
    pub text: String,
}
