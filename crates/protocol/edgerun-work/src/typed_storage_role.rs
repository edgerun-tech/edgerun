use alloc::collections::BTreeMap;

use crate::protocol::*;
use crate::roles::*;
use crate::storage_payload::*;

#[derive(Default)]
pub struct TypedObjectStoreRole {
    objects: BTreeMap<Hash, ObjectStoreRequest>,
}

impl TypedObjectStoreRole {
    pub fn object_count(&self) -> usize {
        self.objects.len()
    }

    pub fn has_shard(&self, shard_hash: &Hash) -> bool {
        self.objects.contains_key(shard_hash)
    }

    pub fn retrieve(&self, request: &ObjectRetrieveRequest) -> Option<ObjectRetrieveResponse> {
        let stored = self.objects.get(&request.shard_hash)?;
        if stored.manifest_hash != request.manifest_hash
            || stored.job_id != request.job_id
            || stored.shard_index != request.shard_index
        {
            return None;
        }
        Some(retrieve_response_from_store_request(stored))
    }
}

impl WorkRole for TypedObjectStoreRole {
    fn role_id(&self) -> u16 {
        NODE_ROLE_STORAGE
    }

    fn accepts_department(&self, department: u16) -> bool {
        department == DEPARTMENT_STORAGE || department == DEPARTMENT_RETRIEVAL
    }

    fn accepts_work_type(&self, work_type: u16) -> bool {
        work_type == WORK_TYPE_OBJECT_STORE || work_type == WORK_TYPE_OBJECT_RETRIEVE
    }

    fn handle(&mut self, context: &RoleContext, input: RoleInput) -> RoleOutput {
        let WorkPacket::NetworkMessage(message) = input.packet else {
            return RoleOutput::ignored();
        };
        if message.to != context.local_node.node_id
            || !self.accepts_department(message.department)
            || !self.accepts_work_type(message.work_type)
        {
            return RoleOutput::ignored();
        }
        let Ok(payload) = storage_payload_from_bytes(&message.payload) else {
            return RoleOutput::rejected(b"invalid storage payload".to_vec());
        };
        match payload {
            StoragePayload::StoreRequest(request) => {
                if message.work_type != WORK_TYPE_OBJECT_STORE || !verify_store_request(&request) {
                    return RoleOutput::rejected(b"invalid store request".to_vec());
                }
                self.objects.insert(request.shard_hash, request);
                RoleOutput::accepted(WorkPacket::Ack(WorkAck {
                    ok: true,
                    code: 200,
                    text: "typed object stored".into(),
                }))
            }
            StoragePayload::RetrieveRequest(request) => {
                if message.work_type != WORK_TYPE_OBJECT_RETRIEVE {
                    return RoleOutput::rejected(b"invalid retrieve request".to_vec());
                }
                let Some(response) = self.retrieve(&request) else {
                    return RoleOutput::rejected(b"shard not found".to_vec());
                };
                let Ok(bytes) = storage_payload_bytes(&StoragePayload::RetrieveResponse(response)) else {
                    return RoleOutput::rejected(b"response encode failed".to_vec());
                };
                RoleOutput {
                    status: ROLE_STATUS_ACCEPTED,
                    packet: None,
                    bytes,
                }
            }
            StoragePayload::RetrieveResponse(_) => RoleOutput::ignored(),
        }
    }
}
