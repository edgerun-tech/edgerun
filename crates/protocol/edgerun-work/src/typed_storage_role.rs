use alloc::vec::Vec;

use crate::protocol::*;
use crate::roles::*;
use crate::storage_adapter::{InMemoryObjectStorage, ObjectStorageAdapter, StorageAdapterError};
use crate::storage_payload::*;

pub struct TypedObjectStoreRole<S = InMemoryObjectStorage> {
    storage: S,
}

impl Default for TypedObjectStoreRole<InMemoryObjectStorage> {
    fn default() -> Self {
        Self::memory_unlimited()
    }
}

impl TypedObjectStoreRole<InMemoryObjectStorage> {
    pub fn memory_unlimited() -> Self {
        Self::with_storage(InMemoryObjectStorage::unlimited())
    }

    pub fn memory_with_capacity(capacity_bytes: u64) -> Self {
        Self::with_storage(InMemoryObjectStorage::new(capacity_bytes))
    }
}

#[cfg(feature = "std")]
impl TypedObjectStoreRole<crate::storage_adapter::FileObjectStorage> {
    pub fn file_backed(
        root: impl Into<std::path::PathBuf>,
        capacity_bytes: u64,
    ) -> Result<Self, std::io::Error> {
        Ok(Self::with_storage(
            crate::storage_adapter::FileObjectStorage::open(root, capacity_bytes)?,
        ))
    }
}

#[cfg(feature = "virtual-disk")]
impl
    TypedObjectStoreRole<
        crate::storage_adapter::VirtualDiskObjectStorage<edgerun_virtual_disk::MemoryBlockBackend>,
    >
{
    pub fn memory_virtual_disk(
        capacity_bytes: u64,
        block_size: u32,
        slot_size: u64,
    ) -> Result<Self, StorageAdapterError> {
        Ok(Self::with_storage(
            crate::storage_adapter::VirtualDiskObjectStorage::memory(
                capacity_bytes,
                block_size,
                slot_size,
            )?,
        ))
    }
}

#[cfg(feature = "virtual-disk")]
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
impl
    TypedObjectStoreRole<
        crate::storage_adapter::VirtualDiskObjectStorage<edgerun_virtual_disk::FileBlockBackend>,
    >
{
    pub fn file_virtual_disk(
        path: impl AsRef<std::path::Path>,
        capacity_bytes: u64,
        block_size: u32,
        slot_size: u64,
    ) -> Result<Self, StorageAdapterError> {
        Ok(Self::with_storage(
            crate::storage_adapter::VirtualDiskObjectStorage::open_file_image(
                path,
                capacity_bytes,
                block_size,
                slot_size,
            )?,
        ))
    }
}

impl<S: ObjectStorageAdapter> TypedObjectStoreRole<S> {
    pub const fn with_storage(storage: S) -> Self {
        Self { storage }
    }

    pub fn storage(&self) -> &S {
        &self.storage
    }

    pub fn storage_mut(&mut self) -> &mut S {
        &mut self.storage
    }

    pub fn object_count(&self) -> usize {
        self.storage.object_count()
    }

    pub fn capacity_bytes(&self) -> u64 {
        self.storage.capacity_bytes()
    }

    pub fn used_bytes(&self) -> u64 {
        self.storage.used_bytes()
    }

    pub fn has_shard(&self, shard_hash: &Hash) -> bool {
        self.storage.has_shard(shard_hash)
    }

    pub fn list_shards(&self) -> Vec<Hash> {
        self.storage.list_shards()
    }

    pub fn retrieve(&self, request: &ObjectRetrieveRequest) -> Option<ObjectRetrieveResponse> {
        self.storage.retrieve(request).ok()
    }

    pub fn delete_shard(&mut self, shard_hash: &Hash) -> Result<bool, StorageAdapterError> {
        self.storage.delete_shard(shard_hash)
    }
}

impl<S: ObjectStorageAdapter> WorkRole for TypedObjectStoreRole<S> {
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
        let Some(message) = network_message_for_role(self, context, input) else {
            return RoleOutput::ignored();
        };
        let Ok(payload) = storage_payload_from_bytes(&message.payload) else {
            return RoleOutput::rejected(b"invalid storage payload".to_vec());
        };
        match payload {
            StoragePayload::StoreRequest(request) => {
                if message.work_type != WORK_TYPE_OBJECT_STORE || !verify_store_request(&request) {
                    return RoleOutput::rejected(b"invalid store request".to_vec());
                }
                match self.storage.store(request) {
                    Ok(()) => RoleOutput::accepted_ack(200, "typed object stored"),
                    Err(err) => RoleOutput::rejected(err.message().to_vec()),
                }
            }
            StoragePayload::RetrieveRequest(request) => {
                if message.work_type != WORK_TYPE_OBJECT_RETRIEVE {
                    return RoleOutput::rejected(b"invalid retrieve request".to_vec());
                }
                let response = match self.storage.retrieve(&request) {
                    Ok(response) => response,
                    Err(err) => return RoleOutput::rejected(err.message().to_vec()),
                };
                let Ok(bytes) = storage_payload_bytes(&StoragePayload::RetrieveResponse(response))
                else {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage_payload::{
        storage_payload_from_bytes, typed_shard_hash, verify_retrieve_response,
    };

    fn store_request(bytes: &[u8]) -> ObjectStoreRequest {
        let job_id = [9u8; 32];
        let shard_index = 0;
        ObjectStoreRequest {
            manifest_hash: [4u8; 32],
            job_id,
            shard_index,
            shard_hash: typed_shard_hash(job_id, shard_index, false, bytes),
            original_len: bytes.len() as u64,
            bytes: bytes.to_vec(),
        }
    }

    fn retrieve_request(object: &ObjectStoreRequest) -> ObjectRetrieveRequest {
        ObjectRetrieveRequest {
            manifest_hash: object.manifest_hash,
            job_id: object.job_id,
            shard_index: object.shard_index,
            shard_hash: object.shard_hash,
        }
    }

    fn storage_message(local: NodeId, work_type: u16, payload: StoragePayload) -> WorkPacket {
        let payload = storage_payload_bytes(&payload).expect("payload bytes");
        WorkPacket::NetworkMessage(NetworkMessage {
            abi_version: WORK_WIRE_ABI_VERSION,
            message_id: [1u8; 32],
            prev_hash: [0u8; 32],
            from: [2u8; 32],
            to: local,
            via_relay: [3u8; 32],
            department: if work_type == WORK_TYPE_OBJECT_RETRIEVE {
                DEPARTMENT_RETRIEVAL
            } else {
                DEPARTMENT_STORAGE
            },
            work_type,
            sequence: 1,
            payload_hash: [0u8; 32],
            payload,
            signature: crate::signing::empty_signature(),
        })
    }

    fn assert_role_roundtrip<S: ObjectStorageAdapter>(mut role: TypedObjectStoreRole<S>) {
        let local = [8u8; 32];
        let object = store_request(b"role adapter data");
        let retrieve = retrieve_request(&object);
        let context = RoleContext {
            now_unix_ms: 1,
            local_node: NodeIdentity {
                node_id: local,
                role: NODE_ROLE_STORAGE,
                public_key: [0u8; 32],
            },
            policy_hash: [0u8; 32],
        };

        let stored = role.handle(
            &context,
            RoleInput {
                packet: storage_message(
                    local,
                    WORK_TYPE_OBJECT_STORE,
                    StoragePayload::StoreRequest(object),
                ),
                previous_hash: [0u8; 32],
                channel_hash: [0u8; 32],
            },
        );
        assert_eq!(stored.status, ROLE_STATUS_ACCEPTED);
        assert_eq!(role.object_count(), 1);

        let retrieved = role.handle(
            &context,
            RoleInput {
                packet: storage_message(
                    local,
                    WORK_TYPE_OBJECT_RETRIEVE,
                    StoragePayload::RetrieveRequest(retrieve),
                ),
                previous_hash: [0u8; 32],
                channel_hash: [0u8; 32],
            },
        );
        assert_eq!(retrieved.status, ROLE_STATUS_ACCEPTED);
        let StoragePayload::RetrieveResponse(response) =
            storage_payload_from_bytes(&retrieved.bytes).expect("retrieve response")
        else {
            panic!("expected retrieve response");
        };
        assert_eq!(response.bytes, b"role adapter data");
        assert!(verify_retrieve_response(&response));
    }

    #[test]
    fn role_stores_and_retrieves_with_memory_adapter() {
        assert_role_roundtrip(TypedObjectStoreRole::memory_with_capacity(4096));
    }

    #[cfg(feature = "virtual-disk")]
    #[test]
    fn role_stores_and_retrieves_with_memory_virtual_disk_adapter() {
        let role = TypedObjectStoreRole::memory_virtual_disk(64 * 1024, 512, 4096)
            .expect("memory virtual disk role");
        assert_role_roundtrip(role);
    }
}
