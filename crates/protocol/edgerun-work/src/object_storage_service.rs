use alloc::vec;
use alloc::vec::Vec;

use edgerun_crypto::Ed25519SigningKey;

use crate::channel::{ChannelEndpoint, RouteAdvertisement};
use crate::identity::node_identity_from_key;
use crate::protocol::{
    Hash, NodeId, NodeIdentity, WorkPacket, DEPARTMENT_RETRIEVAL, DEPARTMENT_STORAGE,
    NODE_ROLE_STORAGE,
};
use crate::roles::{RoleContext, RoleInput, WorkRole};
use crate::route_builder::RouteAdvertisementBuilder;
use crate::storage_adapter::{InMemoryObjectStorage, ObjectStorageAdapter};
use crate::typed_storage_role::TypedObjectStoreRole;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectStorageResponse {
    pub status: u16,
    pub packet: Option<WorkPacket>,
    pub bytes: Vec<u8>,
}

pub struct ObjectStorageService<S = InMemoryObjectStorage> {
    key: Ed25519SigningKey,
    identity: NodeIdentity,
    role: TypedObjectStoreRole<S>,
    policy_hash: Hash,
}

impl ObjectStorageService<InMemoryObjectStorage> {
    pub fn memory(key: Ed25519SigningKey, capacity_bytes: u64) -> Self {
        Self::with_storage(key, InMemoryObjectStorage::new(capacity_bytes))
    }

    pub fn memory_unlimited(key: Ed25519SigningKey) -> Self {
        Self::with_storage(key, InMemoryObjectStorage::unlimited())
    }
}

#[cfg(feature = "std")]
impl ObjectStorageService<crate::storage_adapter::FileObjectStorage> {
    pub fn object_store_dir(
        key: Ed25519SigningKey,
        root: impl Into<std::path::PathBuf>,
        capacity_bytes: u64,
    ) -> Result<Self, std::io::Error> {
        let storage = crate::storage_adapter::FileObjectStorage::open(root, capacity_bytes)?;
        Ok(Self::with_storage(key, storage))
    }
}

impl<S: ObjectStorageAdapter> ObjectStorageService<S> {
    pub fn with_storage(key: Ed25519SigningKey, storage: S) -> Self {
        Self::with_storage_and_policy(key, storage, [0u8; 32])
    }

    pub fn with_storage_and_policy(key: Ed25519SigningKey, storage: S, policy_hash: Hash) -> Self {
        let identity = node_identity_from_key(&key, NODE_ROLE_STORAGE);
        Self {
            key,
            identity,
            role: TypedObjectStoreRole::with_storage(storage),
            policy_hash,
        }
    }

    pub fn identity(&self) -> &NodeIdentity {
        &self.identity
    }

    pub fn node_id(&self) -> NodeId {
        self.identity.node_id
    }

    pub fn policy_hash(&self) -> Hash {
        self.policy_hash
    }

    pub fn storage(&self) -> &S {
        self.role.storage()
    }

    pub fn storage_mut(&mut self) -> &mut S {
        self.role.storage_mut()
    }

    pub fn object_count(&self) -> usize {
        self.role.object_count()
    }

    pub fn used_bytes(&self) -> u64 {
        self.role.used_bytes()
    }

    pub fn route_advertisement(
        &self,
        endpoint: ChannelEndpoint,
        relay_node_id: NodeId,
        valid_until_unix_ms: u64,
    ) -> RouteAdvertisement {
        RouteAdvertisementBuilder::new(&self.key, NODE_ROLE_STORAGE, endpoint)
            .relay_node_id(relay_node_id)
            .departments(vec![DEPARTMENT_STORAGE, DEPARTMENT_RETRIEVAL])
            .valid_until_unix_ms(valid_until_unix_ms)
            .build(&self.key)
    }

    pub fn handle_packet(&mut self, packet: WorkPacket, now_unix_ms: u64) -> ObjectStorageResponse {
        let output = self.role.handle(
            &RoleContext {
                now_unix_ms,
                local_node: self.identity.clone(),
                policy_hash: self.policy_hash,
            },
            RoleInput {
                packet,
                previous_hash: [0u8; 32],
                channel_hash: [0u8; 32],
            },
        );
        ObjectStorageResponse {
            status: output.status,
            packet: output.packet,
            bytes: output.bytes,
        }
    }
}
