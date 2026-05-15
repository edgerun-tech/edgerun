use alloc::vec::Vec;

use crate::capability_packet::{verify_capability_message_payload, CapabilityEnvelope};
use crate::protocol::*;
use crate::roles::{
    network_message_for_role, RoleContext, RoleInput, RoleOutput, WorkRole, ROLE_STATUS_ACCEPTED,
    ROLE_STATUS_REJECTED,
};

#[derive(Clone, Debug, Default)]
pub struct CapabilityRole {
    accepted_messages: Vec<NetworkMessage>,
    accepted_envelopes: Vec<CapabilityEnvelope>,
}

impl CapabilityRole {
    pub fn accepted_messages(&self) -> &[NetworkMessage] {
        &self.accepted_messages
    }

    pub fn accepted_envelopes(&self) -> &[CapabilityEnvelope] {
        &self.accepted_envelopes
    }

    pub fn accepted_len(&self) -> usize {
        self.accepted_envelopes.len()
    }
}

impl WorkRole for CapabilityRole {
    fn role_id(&self) -> u16 {
        NODE_ROLE_CAPABILITY
    }

    fn accepts_department(&self, department: u16) -> bool {
        department == DEPARTMENT_CAPABILITY
    }

    fn accepts_work_type(&self, work_type: u16) -> bool {
        matches!(
            work_type,
            WORK_TYPE_CAPABILITY_REQUEST
                | WORK_TYPE_CAPABILITY_INVOKE
                | WORK_TYPE_CAPABILITY_EVENT
                | WORK_TYPE_CAPABILITY_CLOSE
        )
    }

    fn handle(&mut self, context: &RoleContext, input: RoleInput) -> RoleOutput {
        let Some(message) = network_message_for_role(self, context, input) else {
            return RoleOutput {
                status: ROLE_STATUS_REJECTED,
                packet: None,
                bytes: Vec::new(),
            };
        };
        let Ok(envelope) = verify_capability_message_payload(&message) else {
            return RoleOutput::rejected(b"invalid capability envelope".to_vec());
        };
        self.accepted_messages.push(message);
        self.accepted_envelopes.push(envelope);
        RoleOutput {
            status: ROLE_STATUS_ACCEPTED,
            packet: None,
            bytes: Vec::new(),
        }
    }
}
