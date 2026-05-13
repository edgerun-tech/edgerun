use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::capability_packet::{
    CAPABILITY_CONTENT_OBJECT, CAPABILITY_OPERATION_OBJECT_GET, CAPABILITY_PACKET_INVOKE,
    CapabilityEnvelope, capability_envelope, verify_capability_message_payload,
};
use crate::identity::node_identity_from_key;
use crate::message_seal::unseal_message_from_recipient_payload;
use crate::preimage::HashBuilder;
use crate::protocol::*;
use crate::roles::{
    ROLE_STATUS_ACCEPTED, ROLE_STATUS_REJECTED, RoleContext, RoleInput, RoleOutput, WorkRole,
    network_message_for_role,
};

const TRUST_CONTAINER_CAPABILITY_ID_DOMAIN: &[u8] =
    b"edgerun:v1:work:trust-container:capability-id";
pub const TRUST_CONTAINER_OPERATION_MESSAGE_DECRYPT: u16 = CAPABILITY_OPERATION_OBJECT_GET;

pub fn trust_container_capability_id() -> Hash {
    HashBuilder::domain(TRUST_CONTAINER_CAPABILITY_ID_DOMAIN).finish()
}

pub fn trust_container_message_decrypt_request(
    session_id: Hash,
    invocation_id: Hash,
    source_node_id: NodeId,
    trust_container_node_id: NodeId,
    sequence: u64,
    timestamp_unix_ms: u64,
    sealed_message_payload: Vec<u8>,
) -> CapabilityEnvelope {
    capability_envelope(
        session_id,
        invocation_id,
        trust_container_capability_id(),
        source_node_id,
        trust_container_node_id,
        CAPABILITY_PACKET_INVOKE,
        TRUST_CONTAINER_OPERATION_MESSAGE_DECRYPT,
        CAPABILITY_CONTENT_OBJECT,
        sequence,
        timestamp_unix_ms,
        sealed_message_payload,
    )
}

pub fn is_trust_container_message_decrypt_request(envelope: &CapabilityEnvelope) -> bool {
    envelope.capability_id == trust_container_capability_id()
        && envelope.kind == CAPABILITY_PACKET_INVOKE
        && envelope.operation == TRUST_CONTAINER_OPERATION_MESSAGE_DECRYPT
        && envelope.content_type == CAPABILITY_CONTENT_OBJECT
}

#[derive(Clone, Debug)]
pub struct TrustContainerRole {
    owner_key: Ed25519SigningKey,
    decrypted_count: usize,
}

impl TrustContainerRole {
    pub fn new(owner_key: Ed25519SigningKey) -> Self {
        Self {
            owner_key,
            decrypted_count: 0,
        }
    }

    pub fn identity(&self) -> NodeIdentity {
        node_identity_from_key(&self.owner_key, NODE_ROLE_CAPABILITY)
    }

    pub fn decrypted_count(&self) -> usize {
        self.decrypted_count
    }

    fn open_message(&mut self, context: &RoleContext, envelope: CapabilityEnvelope) -> RoleOutput {
        if !is_trust_container_message_decrypt_request(&envelope)
            || envelope.target_node_id != context.local_node.node_id
        {
            return RoleOutput::rejected(b"unsupported trust container capability".to_vec());
        }

        let recipient = self.identity();
        if recipient.node_id != context.local_node.node_id {
            return RoleOutput::rejected(b"trust container key mismatch".to_vec());
        }
        let Ok(plaintext) =
            unseal_message_from_recipient_payload(&self.owner_key, &recipient, &envelope.payload)
        else {
            return RoleOutput::rejected(b"not decryptable by this trust container".to_vec());
        };
        self.decrypted_count += 1;
        RoleOutput {
            status: ROLE_STATUS_ACCEPTED,
            packet: None,
            bytes: plaintext,
        }
    }
}

impl WorkRole for TrustContainerRole {
    fn role_id(&self) -> u16 {
        NODE_ROLE_CAPABILITY
    }

    fn accepts_department(&self, department: u16) -> bool {
        department == DEPARTMENT_CAPABILITY
    }

    fn accepts_work_type(&self, work_type: u16) -> bool {
        work_type == WORK_TYPE_CAPABILITY_INVOKE
    }

    fn handle(&mut self, context: &RoleContext, input: RoleInput) -> RoleOutput {
        let Some(message) = network_message_for_role(self, context, input) else {
            return RoleOutput::ignored();
        };
        let Ok(envelope) = verify_capability_message_payload(&message) else {
            return RoleOutput {
                status: ROLE_STATUS_REJECTED,
                packet: None,
                bytes: b"invalid capability envelope".to_vec(),
            };
        };
        self.open_message(context, envelope)
    }
}
