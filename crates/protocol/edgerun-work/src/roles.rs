use alloc::string::String;
use alloc::vec::Vec;

use crate::identity::verify_node_identity;
use crate::protocol::*;
use crate::types::{
    Department, NodeRole, WorkType, department_for_work_type_typed, role_for_department_typed,
};

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

pub type WorkServiceResponse = RoleOutput;

impl RoleOutput {
    pub fn accepted(packet: WorkPacket) -> Self {
        Self {
            status: ROLE_STATUS_ACCEPTED,
            packet: Some(packet),
            bytes: Vec::new(),
        }
    }

    pub fn ignored() -> Self {
        Self {
            status: ROLE_STATUS_IGNORED,
            packet: None,
            bytes: Vec::new(),
        }
    }

    pub fn rejected(reason: Vec<u8>) -> Self {
        Self {
            status: ROLE_STATUS_REJECTED,
            packet: None,
            bytes: reason,
        }
    }

    pub fn accepted_ack(code: u16, text: impl Into<String>) -> Self {
        Self::accepted(WorkPacket::Ack(WorkAck {
            ok: true,
            code,
            text: text.into(),
        }))
    }

    pub fn accepted_bytes(bytes: Vec<u8>) -> Self {
        Self {
            status: ROLE_STATUS_ACCEPTED,
            packet: None,
            bytes,
        }
    }
}

pub trait WorkRole {
    fn role_id(&self) -> u16;
    fn accepts_department(&self, department: u16) -> bool;
    fn accepts_work_type(&self, work_type: u16) -> bool;
    fn handle(&mut self, context: &RoleContext, input: RoleInput) -> RoleOutput;
}

pub fn execute_role<R: WorkRole + ?Sized>(
    role: &mut R,
    local_node: &NodeIdentity,
    policy_hash: Hash,
    packet: WorkPacket,
    now_unix_ms: u64,
    previous_hash: Hash,
    channel_hash: Hash,
) -> RoleOutput {
    role.handle(
        &RoleContext {
            now_unix_ms,
            local_node: local_node.clone(),
            policy_hash,
        },
        RoleInput {
            packet,
            previous_hash,
            channel_hash,
        },
    )
}

pub fn execute_role_packet<R: WorkRole + ?Sized>(
    role: &mut R,
    local_node: &NodeIdentity,
    policy_hash: Hash,
    packet: WorkPacket,
    now_unix_ms: u64,
) -> RoleOutput {
    execute_role(
        role,
        local_node,
        policy_hash,
        packet,
        now_unix_ms,
        [0u8; 32],
        [0u8; 32],
    )
}

pub fn network_message_for_role<R: WorkRole + ?Sized>(
    role: &R,
    context: &RoleContext,
    input: RoleInput,
) -> Option<NetworkMessage> {
    let WorkPacket::NetworkMessage(message) = input.packet else {
        return None;
    };
    if !verify_node_identity(&context.local_node)
        || context.local_node.role != role.role_id()
        || message.to != context.local_node.node_id
        || !role.accepts_department(message.department)
        || !role.accepts_work_type(message.work_type)
    {
        return None;
    }
    Some(message)
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
