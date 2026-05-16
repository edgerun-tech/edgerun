use alloc::string::String;
use alloc::vec::Vec;

pub const WORK_WIRE_ABI_VERSION: u16 = 1;

pub const NODE_ROLE_ADMISSION: u16 = 4;

pub const WORK_TYPE_CAPABILITY_INVOKE: u16 = 12;

pub const DEPARTMENT_CAPABILITY: u16 = 7;

pub const SIGNATURE_ALGORITHM_SOLANA_ED25519: u16 = 101;

pub type Hash = [u8; 32];
pub type NodeId = [u8; 32];
pub type PublicKey = [u8; 32];
pub type ChannelId = [u8; 32];

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
pub struct ChannelEndpoint {
    pub abi_version: u16,
    pub channel_id: ChannelId,
    pub kind: u16,
    pub address: Vec<u8>,
    pub label: String,
}

impl ChannelEndpoint {
    pub fn new(channel_id: ChannelId, kind: u16, address: Vec<u8>, label: String) -> Self {
        Self {
            abi_version: WORK_WIRE_ABI_VERSION,
            channel_id,
            kind,
            address,
            label,
        }
    }
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
