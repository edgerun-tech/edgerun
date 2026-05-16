use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::chat_index::{CHAT_MESSAGE_KIND_TEXT, MessageObject};
use crate::codec::blake3_hash;
use crate::message_seal::sealed_message_object_from_network_message;
use crate::protocol::*;
use crate::roles::*;

#[derive(Default)]
pub struct MessageRole {
    delivered: Vec<MessageObject>,
}

impl MessageRole {
    pub fn delivered_len(&self) -> usize {
        self.delivered.len()
    }

    pub fn delivered(&self) -> &[MessageObject] {
        &self.delivered
    }
}

impl WorkRole for MessageRole {
    fn role_id(&self) -> u16 {
        NODE_ROLE_MESSAGE
    }

    fn accepts_department(&self, department: u16) -> bool {
        department == DEPARTMENT_MESSAGE
    }

    fn accepts_work_type(&self, work_type: u16) -> bool {
        work_type == WORK_TYPE_MESSAGE_DELIVER
    }

    fn handle(&mut self, context: &RoleContext, input: RoleInput) -> RoleOutput {
        let Some(message) = network_message_for_role(self, context, input) else {
            return RoleOutput::ignored();
        };
        let Ok(message_object) = sealed_message_object_from_network_message(
            &message,
            context.now_unix_ms,
            CHAT_MESSAGE_KIND_TEXT,
        ) else {
            return RoleOutput::rejected(b"invalid sealed message".to_vec());
        };
        self.delivered.push(message_object);
        RoleOutput::accepted_ack(200, "message delivered")
    }
}

#[derive(Default)]
pub struct ObjectStoreRole {
    objects: BTreeMap<Hash, Vec<u8>>,
}

impl ObjectStoreRole {
    pub fn get(&self, hash: &Hash) -> Option<&[u8]> {
        self.objects.get(hash).map(Vec::as_slice)
    }

    pub fn object_count(&self) -> usize {
        self.objects.len()
    }
}

impl WorkRole for ObjectStoreRole {
    fn role_id(&self) -> u16 {
        NODE_ROLE_STORAGE
    }

    fn accepts_department(&self, department: u16) -> bool {
        department == DEPARTMENT_STORAGE
    }

    fn accepts_work_type(&self, work_type: u16) -> bool {
        work_type == WORK_TYPE_OBJECT_STORE || work_type == WORK_TYPE_OBJECT_PIN
    }

    fn handle(&mut self, context: &RoleContext, input: RoleInput) -> RoleOutput {
        let Some(message) = network_message_for_role(self, context, input) else {
            return RoleOutput::ignored();
        };
        let hash = blake3_hash(&message.payload);
        self.objects.insert(hash, message.payload);
        RoleOutput::accepted_ack(200, "object stored")
    }
}
