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
        Ok(Self::with_storage(crate::storage_adapter::FileObjectStorage::open(
            root,
            capacity_bytes,
        )?))
    }
}

#[cfg(feature = "virtual-disk")]
impl TypedObjectStoreRole<crate::storage_adapter::VirtualDiskObjectStorage<edgerun_virtual_disk::MemoryBlockBackend>> {
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
impl TypedObjectStoreRole<crate::storage_adapter::VirtualDiskObjectStorage<edgerun_virtual_disk::FileBlockBackend>> {
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
                match self.storage.store(request) {
                    Ok(()) => RoleOutput::accepted(WorkPacket::Ack(WorkAck {
                        ok: true,
                        code: 200,
                        text: "typed object stored".into(),
                    })),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::blake3_hash;
    use crate::identity::node_identity_from_key;
    use crate::preimage::HashBuilder;
    use crate::relay_role::RelayRole;
    use crate::route_builder::{tcp_endpoint, RouteAdvertisementBuilder};
    use crate::signing::{empty_signature, sign_network_message};
    use crate::std_runtime::{unix_ms, TcpNodeRuntime};
    use crate::storage_payload::{storage_payload_from_bytes, typed_shard_hash, verify_retrieve_response};
    use crate::work_channel::WorkChannel;
    use crate::ChannelOrderBook;
    use edgerun_crypto::Ed25519SigningKey;

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

    fn signed_storage_message(
        key: &Ed25519SigningKey,
        from: NodeId,
        to: NodeId,
        via_relay: NodeId,
        sequence: u64,
        payload: Vec<u8>,
    ) -> NetworkMessage {
        let payload_hash = blake3_hash(&payload);
        let message_id = HashBuilder::domain(b"edgerun:test:storage-message")
            .node_id(&from)
            .node_id(&to)
            .node_id(&via_relay)
            .u64(sequence)
            .hash(&payload_hash)
            .finish();
        sign_network_message(
            key,
            NetworkMessage {
                abi_version: WORK_WIRE_ABI_VERSION,
                message_id,
                prev_hash: [0u8; 32],
                from,
                to,
                via_relay,
                department: DEPARTMENT_STORAGE,
                work_type: WORK_TYPE_OBJECT_STORE,
                sequence,
                payload_hash,
                payload,
                signature: empty_signature(),
            },
        )
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
                packet: storage_message(local, WORK_TYPE_OBJECT_STORE, StoragePayload::StoreRequest(object)),
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

    #[test]
    fn tcp_relay_forwards_store_request_to_storage_role() {
        let client_key = Ed25519SigningKey::from_bytes(&[41u8; 32]);
        let relay_key = Ed25519SigningKey::from_bytes(&[42u8; 32]);
        let storage_key = Ed25519SigningKey::from_bytes(&[43u8; 32]);
        let client = node_identity_from_key(&client_key, NODE_ROLE_MESSAGE);
        let relay = node_identity_from_key(&relay_key, NODE_ROLE_RELAY);
        let storage = node_identity_from_key(&storage_key, NODE_ROLE_STORAGE);

        let mut client_runtime = TcpNodeRuntime::bind(client.node_id, "127.0.0.1:0").expect("client runtime");
        let mut relay_runtime = TcpNodeRuntime::bind(relay.node_id, "127.0.0.1:0").expect("relay runtime");
        let storage_runtime = TcpNodeRuntime::bind(storage.node_id, "127.0.0.1:0").expect("storage runtime");

        let relay_route = RouteAdvertisementBuilder::new(
            &relay_key,
            NODE_ROLE_RELAY,
            tcp_endpoint("relay", relay_runtime.listen_addr().to_string()),
        )
        .departments(vec![DEPARTMENT_RELAY])
        .valid_until_unix_ms(unix_ms().saturating_add(60_000))
        .build(&relay_key);
        let storage_route = RouteAdvertisementBuilder::new(
            &storage_key,
            NODE_ROLE_STORAGE,
            tcp_endpoint("storage", storage_runtime.listen_addr().to_string()),
        )
        .relay_node_id(relay.node_id)
        .departments(vec![DEPARTMENT_STORAGE, DEPARTMENT_RETRIEVAL])
        .valid_until_unix_ms(unix_ms().saturating_add(60_000))
        .build(&storage_key);

        let relay_route_hash = client_runtime.add_route(relay_route.clone()).expect("client relay route");
        relay_runtime.add_route(storage_route).expect("relay storage route");

        let object = store_request(b"stored through relay");
        let retrieve = retrieve_request(&object);
        let payload = storage_payload_bytes(&StoragePayload::StoreRequest(object)).expect("payload bytes");
        let message = signed_storage_message(
            &client_key,
            client.node_id,
            storage.node_id,
            relay.node_id,
            1,
            payload,
        );
        client_runtime
            .send_unordered(client.node_id, relay.node_id, WorkPacket::NetworkMessage(message))
            .expect("send to relay");

        std::thread::sleep(std::time::Duration::from_millis(50));
        let mut order = ChannelOrderBook::new();
        let ordered = relay_runtime.drain_ordered(
            client.node_id,
            &mut order,
            relay_route_hash,
            relay_route.endpoint.channel_id,
        );
        assert_eq!(ordered.len(), 1);

        let mut relay_role = RelayRole::from_key(relay_key, 0);
        relay_role
            .forward_ordered_on(&mut relay_runtime, &ordered[0], [0u8; 32], [0u8; 32])
            .expect("relay forward");

        std::thread::sleep(std::time::Duration::from_millis(50));
        let packets = storage_runtime.drain_packets();
        assert_eq!(packets.len(), 1);

        let mut storage_role = TypedObjectStoreRole::memory_with_capacity(4096);
        let output = storage_role.handle(
            &RoleContext {
                now_unix_ms: unix_ms(),
                local_node: storage,
                policy_hash: [0u8; 32],
            },
            RoleInput {
                packet: packets.into_iter().next().expect("storage packet"),
                previous_hash: [0u8; 32],
                channel_hash: [0u8; 32],
            },
        );
        assert_eq!(output.status, ROLE_STATUS_ACCEPTED);
        assert_eq!(storage_role.object_count(), 1);
        assert_eq!(
            storage_role.retrieve(&retrieve).expect("stored object").bytes,
            b"stored through relay"
        );

        client_runtime.shutdown().expect("client shutdown");
        relay_runtime.shutdown().expect("relay shutdown");
        storage_runtime.shutdown().expect("storage shutdown");
    }
}
