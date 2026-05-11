use alloc::vec::Vec;

use crate::protocol::{Hash, NodeIdentity, WorkPacket};

pub const ROLE_STATUS_ACCEPTED: u16 = 1;
pub const ROLE_STATUS_REJECTED: u16 = 2;
pub const ROLE_STATUS_IGNORED: u16 = 3;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleContext {
    pub now_unix_ms: u64,
    pub local_node: NodeIdentity,
    pub policy_hash: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleInput {
    pub packet: WorkPacket,
    pub previous_hash: Hash,
    pub channel_hash: Hash,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleOutput {
    pub status: u16,
    pub packet: Option<WorkPacket>,
    pub bytes: Vec<u8>,
}
