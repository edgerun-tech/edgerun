use alloc::vec::Vec;

use crate::protocol::*;

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

impl RoleOutput {
    pub fn accepted(packet: WorkPacket) -> Self {
        Self { status: ROLE_STATUS_ACCEPTED, packet: Some(packet), bytes: Vec::new() }
    }

    pub fn ignored() -> Self {
        Self { status: ROLE_STATUS_IGNORED, packet: None, bytes: Vec::new() }
    }

    pub fn rejected(reason: Vec<u8>) -> Self {
        Self { status: ROLE_STATUS_REJECTED, packet: None, bytes: reason }
    }
}

pub trait WorkRole {
    fn role_id(&self) -> u16;
    fn accepts_department(&self, department: u16) -> bool;
    fn accepts_work_type(&self, work_type: u16) -> bool;
    fn handle(&mut self, context: &RoleContext, input: RoleInput) -> RoleOutput;
}

pub fn department_for_work_type(work_type: u16) -> Option<u16> {
    match work_type {
        WORK_TYPE_MESSAGE_DELIVER => Some(DEPARTMENT_MESSAGE),
        WORK_TYPE_OBJECT_STORE | WORK_TYPE_OBJECT_PIN => Some(DEPARTMENT_STORAGE),
        WORK_TYPE_OBJECT_RETRIEVE => Some(DEPARTMENT_RETRIEVAL),
        WORK_TYPE_COMPUTE_RUN => Some(DEPARTMENT_COMPUTE),
        _ => None,
    }
}

pub fn role_for_department(department: u16) -> Option<u16> {
    match department {
        DEPARTMENT_RELAY => Some(NODE_ROLE_RELAY),
        DEPARTMENT_MESSAGE => Some(NODE_ROLE_MESSAGE),
        DEPARTMENT_STORAGE => Some(NODE_ROLE_STORAGE),
        DEPARTMENT_RETRIEVAL => Some(NODE_ROLE_STORAGE),
        DEPARTMENT_COMPUTE => Some(NODE_ROLE_COMPUTE),
        DEPARTMENT_ADMISSION => Some(NODE_ROLE_ADMISSION),
        _ => None,
    }
}
