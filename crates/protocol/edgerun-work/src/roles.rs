use alloc::vec::Vec;

use crate::protocol::*;
use crate::types::{department_for_work_type_typed, role_for_department_typed, Department, NodeRole, WorkType};

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
    WorkType::from_u16(work_type)
        .and_then(department_for_work_type_typed)
        .map(Department::as_u16)
}

pub fn role_for_department(department: u16) -> Option<u16> {
    Department::from_u16(department)
        .and_then(role_for_department_typed)
        .map(NodeRole::as_u16)
}
