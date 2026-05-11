use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::protocol::Hash;
use crate::storage_payload::{
    ObjectRetrieveRequest, ObjectRetrieveResponse, ObjectStoreRequest, StoragePayload,
    retrieve_response_from_store_request, storage_payload_bytes, storage_payload_from_bytes,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StorageAdapterError {
    CapacityExceeded,
    NotFound,
    HashMismatch,
    InvalidStoredObject,
    Io,
}

impl StorageAdapterError {
    pub const fn message(&self) -> &'static [u8] {
        match self {
            Self::CapacityExceeded => b"storage capacity exceeded",
            Self::NotFound => b"shard not found",
            Self::HashMismatch => b"stored shard binding mismatch",
            Self::InvalidStoredObject => b"invalid stored object",
            Self::Io => b"storage io error",
        }
    }
}

pub trait ObjectStorageAdapter {
    fn object_count(&self) -> usize;
    fn capacity_bytes(&self) -> u64;
    fn used_bytes(&self) -> u64;
    fn has_shard(&self, shard_hash: &Hash) -> bool;
    fn list_shards(&self) -> Vec<Hash>;
    fn store(&mut self, request: ObjectStoreRequest) -> Result<(), StorageAdapterError>;
    fn retrieve(
        &self,
        request: &ObjectRetrieveRequest,
    ) -> Result<ObjectRetrieveResponse, StorageAdapterError>;
    fn delete_shard(&mut self, shard_hash: &Hash) -> Result<bool, StorageAdapterError>;
}

#[derive(Clone, Debug)]
pub struct InMemoryObjectStorage {
    objects: BTreeMap<Hash, ObjectStoreRequest>,
    capacity_bytes: u64,
    used_bytes: u64,
}

impl Default for InMemoryObjectStorage {
    fn default() -> Self {
        Self::unlimited()
    }
}

impl InMemoryObjectStorage {
    pub fn new(capacity_bytes: u64) -> Self {
        Self {
            objects: BTreeMap::new(),
            capacity_bytes,
            used_bytes: 0,
        }
    }

    pub fn unlimited() -> Self {
        Self::new(u64::MAX)
    }
}

impl ObjectStorageAdapter for InMemoryObjectStorage {
    fn object_count(&self) -> usize {
        self.objects.len()
    }

    fn capacity_bytes(&self) -> u64 {
        self.capacity_bytes
    }

    fn used_bytes(&self) -> u64 {
        self.used_bytes
    }

    fn has_shard(&self, shard_hash: &Hash) -> bool {
        self.objects.contains_key(shard_hash)
    }

    fn list_shards(&self) -> Vec<Hash> {
        self.objects.keys().copied().collect()
    }

    fn store(&mut self, request: ObjectStoreRequest) -> Result<(), StorageAdapterError> {
        let new_len = request.bytes.len() as u64;
        let old_len = self
            .objects
            .get(&request.shard_hash)
            .map(|stored| stored.bytes.len() as u64)
            .unwrap_or(0);
        let used_without_old = self.used_bytes.saturating_sub(old_len);
        let next_used = used_without_old
            .checked_add(new_len)
            .ok_or(StorageAdapterError::CapacityExceeded)?;
        if next_used > self.capacity_bytes {
            return Err(StorageAdapterError::CapacityExceeded);
        }
        self.objects.insert(request.shard_hash, request);
        self.used_bytes = next_used;
        Ok(())
    }

    fn retrieve(
        &self,
        request: &ObjectRetrieveRequest,
    ) -> Result<ObjectRetrieveResponse, StorageAdapterError> {
        let stored = self
            .objects
            .get(&request.shard_hash)
            .ok_or(StorageAdapterError::NotFound)?;
        if stored.manifest_hash != request.manifest_hash
            || stored.job_id != request.job_id
            || stored.shard_index != request.shard_index
        {
            return Err(StorageAdapterError::HashMismatch);
        }
        Ok(retrieve_response_from_store_request(stored))
    }

    fn delete_shard(&mut self, shard_hash: &Hash) -> Result<bool, StorageAdapterError> {
        let Some(stored) = self.objects.remove(shard_hash) else {
            return Ok(false);
        };
        self.used_bytes = self.used_bytes.saturating_sub(stored.bytes.len() as u64);
        Ok(true)
    }
}

#[cfg(feature = "std")]
#[derive(Clone, Debug)]
pub struct FileObjectStorage {
    root: std::path::PathBuf,
    capacity_bytes: u64,
    used_bytes: u64,
    object_count: usize,
}

#[cfg(feature = "std")]
impl FileObjectStorage {
    pub fn open(
        root: impl Into<std::path::PathBuf>,
        capacity_bytes: u64,
    ) -> Result<Self, std::io::Error> {
        let root = root.into();
        std::fs::create_dir_all(&root)?;
        let (used_bytes, object_count) = scan_store_dir(&root)?;
        Ok(Self {
            root,
            capacity_bytes,
            used_bytes,
            object_count,
        })
    }

    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    fn shard_path(&self, shard_hash: &Hash) -> std::path::PathBuf {
        self.root.join(hash_hex(shard_hash)).with_extension("estore")
    }
}

#[cfg(feature = "std")]
impl ObjectStorageAdapter for FileObjectStorage {
    fn object_count(&self) -> usize {
        self.object_count
    }

    fn capacity_bytes(&self) -> u64 {
        self.capacity_bytes
    }

    fn used_bytes(&self) -> u64 {
        self.used_bytes
    }

    fn has_shard(&self, shard_hash: &Hash) -> bool {
        self.shard_path(shard_hash).is_file()
    }

    fn list_shards(&self) -> Vec<Hash> {
        let Ok(entries) = std::fs::read_dir(&self.root) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("estore") {
                    return None;
                }
                let stem = path.file_stem()?.to_str()?;
                parse_hash_hex(stem)
            })
            .collect()
    }

    fn store(&mut self, request: ObjectStoreRequest) -> Result<(), StorageAdapterError> {
        let bytes = storage_payload_bytes(&StoragePayload::StoreRequest(request.clone()))
            .map_err(|_| StorageAdapterError::InvalidStoredObject)?;
        let path = self.shard_path(&request.shard_hash);
        let old_len = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        let used_without_old = self.used_bytes.saturating_sub(old_len);
        let next_used = used_without_old
            .checked_add(bytes.len() as u64)
            .ok_or(StorageAdapterError::CapacityExceeded)?;
        if next_used > self.capacity_bytes {
            return Err(StorageAdapterError::CapacityExceeded);
        }

        let tmp = path.with_extension("estore.tmp");
        std::fs::write(&tmp, &bytes).map_err(|_| StorageAdapterError::Io)?;
        std::fs::rename(&tmp, &path).map_err(|_| StorageAdapterError::Io)?;
        self.used_bytes = next_used;
        if old_len == 0 {
            self.object_count = self.object_count.saturating_add(1);
        }
        Ok(())
    }

    fn retrieve(
        &self,
        request: &ObjectRetrieveRequest,
    ) -> Result<ObjectRetrieveResponse, StorageAdapterError> {
        let bytes = std::fs::read(self.shard_path(&request.shard_hash))
            .map_err(|_| StorageAdapterError::NotFound)?;
        let payload = storage_payload_from_bytes(&bytes)
            .map_err(|_| StorageAdapterError::InvalidStoredObject)?;
        let StoragePayload::StoreRequest(stored) = payload else {
            return Err(StorageAdapterError::InvalidStoredObject);
        };
        if stored.manifest_hash != request.manifest_hash
            || stored.job_id != request.job_id
            || stored.shard_index != request.shard_index
            || stored.shard_hash != request.shard_hash
        {
            return Err(StorageAdapterError::HashMismatch);
        }
        Ok(retrieve_response_from_store_request(&stored))
    }

    fn delete_shard(&mut self, shard_hash: &Hash) -> Result<bool, StorageAdapterError> {
        let path = self.shard_path(shard_hash);
        let Ok(metadata) = std::fs::metadata(&path) else {
            return Ok(false);
        };
        std::fs::remove_file(path).map_err(|_| StorageAdapterError::Io)?;
        self.used_bytes = self.used_bytes.saturating_sub(metadata.len());
        self.object_count = self.object_count.saturating_sub(1);
        Ok(true)
    }
}

#[cfg(feature = "virtual-disk")]
#[derive(Debug)]
pub struct VirtualDiskObjectStorage<B> {
    backend: B,
    slot_count: u64,
    slot_size: u64,
    used_slots: u64,
    index: BTreeMap<Hash, VirtualDiskObjectIndexEntry>,
}

#[cfg(feature = "virtual-disk")]
#[derive(Clone, Debug, PartialEq, Eq)]
struct VirtualDiskObjectIndexEntry {
    slot: u64,
    len: u64,
}

#[cfg(feature = "virtual-disk")]
impl<B: edgerun_protocols::block::BlockBackend> VirtualDiskObjectStorage<B> {
    pub fn new(backend: B, slot_size: u64) -> Result<Self, StorageAdapterError> {
        let info = backend.info();
        let block_size = u64::from(info.block_size);
        let capacity = info
            .block_count
            .checked_mul(block_size)
            .ok_or(StorageAdapterError::CapacityExceeded)?;
        if block_size == 0 || slot_size == 0 || slot_size % block_size != 0 {
            return Err(StorageAdapterError::InvalidStoredObject);
        }
        let slot_count = capacity / slot_size;
        if slot_count == 0 {
            return Err(StorageAdapterError::CapacityExceeded);
        }
        let mut storage = Self {
            backend,
            slot_count,
            slot_size,
            used_slots: 0,
            index: BTreeMap::new(),
        };
        storage.scan_index()?;
        Ok(storage)
    }

    pub fn backend(&self) -> &B {
        &self.backend
    }

    pub fn slot_size(&self) -> u64 {
        self.slot_size
    }

    pub fn slot_count(&self) -> u64 {
        self.slot_count
    }

    fn scan_index(&mut self) -> Result<(), StorageAdapterError> {
        self.index.clear();
        self.used_slots = 0;
        for slot in 0..self.slot_count {
            if let Some(header) = self.read_header(slot)? {
                self.index.insert(
                    header.shard_hash,
                    VirtualDiskObjectIndexEntry {
                        slot,
                        len: header.payload_len,
                    },
                );
                self.used_slots = self.used_slots.saturating_add(1);
            }
        }
        Ok(())
    }

    fn read_slot(&self, slot: u64) -> Result<Vec<u8>, StorageAdapterError> {
        let info = self.backend.info();
        let block_size = u64::from(info.block_size);
        let blocks = blocks_per_slot(self.slot_size, block_size)?;
        let lba = slot
            .checked_mul(u64::from(blocks))
            .ok_or(StorageAdapterError::CapacityExceeded)?;
        let mut out = alloc::vec![0u8; self.slot_size as usize];
        self.backend
            .read_blocks(lba, blocks, &mut out)
            .map_err(|_| StorageAdapterError::Io)?;
        Ok(out)
    }

    fn write_slot(&self, slot: u64, bytes: &[u8]) -> Result<(), StorageAdapterError> {
        if bytes.len() as u64 > self.slot_size {
            return Err(StorageAdapterError::CapacityExceeded);
        }
        let info = self.backend.info();
        let block_size = u64::from(info.block_size);
        let blocks = blocks_per_slot(self.slot_size, block_size)?;
        let lba = slot
            .checked_mul(u64::from(blocks))
            .ok_or(StorageAdapterError::CapacityExceeded)?;
        let mut padded = alloc::vec![0u8; self.slot_size as usize];
        padded[..bytes.len()].copy_from_slice(bytes);
        self.backend
            .write_blocks(lba, blocks, &padded)
            .map_err(|_| StorageAdapterError::Io)?;
        Ok(())
    }

    fn clear_slot(&self, slot: u64) -> Result<(), StorageAdapterError> {
        let info = self.backend.info();
        let block_size = u64::from(info.block_size);
        let blocks = blocks_per_slot(self.slot_size, block_size)?;
        let lba = slot
            .checked_mul(u64::from(blocks))
            .ok_or(StorageAdapterError::CapacityExceeded)?;
        self.backend
            .write_zeroes(lba, blocks)
            .map_err(|_| StorageAdapterError::Io)
    }

    fn read_header(&self, slot: u64) -> Result<Option<VirtualDiskSlotHeader>, StorageAdapterError> {
        let slot_bytes = self.read_slot(slot)?;
        VirtualDiskSlotHeader::decode(&slot_bytes)
    }

    fn first_free_slot(&self) -> Option<u64> {
        (0..self.slot_count)
            .find(|slot| !self.index.values().any(|entry| entry.slot == *slot))
    }
}

#[cfg(feature = "virtual-disk")]
impl<B: edgerun_protocols::block::BlockBackend> ObjectStorageAdapter for VirtualDiskObjectStorage<B> {
    fn object_count(&self) -> usize {
        self.index.len()
    }

    fn capacity_bytes(&self) -> u64 {
        self.slot_count.saturating_mul(self.slot_size)
    }

    fn used_bytes(&self) -> u64 {
        self.used_slots.saturating_mul(self.slot_size)
    }

    fn has_shard(&self, shard_hash: &Hash) -> bool {
        self.index.contains_key(shard_hash)
    }

    fn list_shards(&self) -> Vec<Hash> {
        self.index.keys().copied().collect()
    }

    fn store(&mut self, request: ObjectStoreRequest) -> Result<(), StorageAdapterError> {
        let payload = storage_payload_bytes(&StoragePayload::StoreRequest(request.clone()))
            .map_err(|_| StorageAdapterError::InvalidStoredObject)?;
        let header = VirtualDiskSlotHeader {
            shard_hash: request.shard_hash,
            payload_len: payload.len() as u64,
        };
        let mut slot_bytes = header.encode();
        slot_bytes.extend_from_slice(&payload);
        if slot_bytes.len() as u64 > self.slot_size {
            return Err(StorageAdapterError::CapacityExceeded);
        }
        let slot = if let Some(existing) = self.index.get(&request.shard_hash) {
            existing.slot
        } else {
            self.first_free_slot()
                .ok_or(StorageAdapterError::CapacityExceeded)?
        };
        self.write_slot(slot, &slot_bytes)?;
        self.index.insert(
            request.shard_hash,
            VirtualDiskObjectIndexEntry {
                slot,
                len: payload.len() as u64,
            },
        );
        self.used_slots = self.index.len() as u64;
        self.backend.flush().map_err(|_| StorageAdapterError::Io)?;
        Ok(())
    }

    fn retrieve(
        &self,
        request: &ObjectRetrieveRequest,
    ) -> Result<ObjectRetrieveResponse, StorageAdapterError> {
        let entry = self
            .index
            .get(&request.shard_hash)
            .ok_or(StorageAdapterError::NotFound)?;
        let slot_bytes = self.read_slot(entry.slot)?;
        let header = VirtualDiskSlotHeader::decode(&slot_bytes)?
            .ok_or(StorageAdapterError::NotFound)?;
        if header.shard_hash != request.shard_hash || header.payload_len != entry.len {
            return Err(StorageAdapterError::HashMismatch);
        }
        let start = VIRTUAL_DISK_SLOT_HEADER_LEN;
        let end = start
            .checked_add(header.payload_len as usize)
            .ok_or(StorageAdapterError::InvalidStoredObject)?;
        if end > slot_bytes.len() {
            return Err(StorageAdapterError::InvalidStoredObject);
        }
        let payload = storage_payload_from_bytes(&slot_bytes[start..end])
            .map_err(|_| StorageAdapterError::InvalidStoredObject)?;
        let StoragePayload::StoreRequest(stored) = payload else {
            return Err(StorageAdapterError::InvalidStoredObject);
        };
        if stored.manifest_hash != request.manifest_hash
            || stored.job_id != request.job_id
            || stored.shard_index != request.shard_index
            || stored.shard_hash != request.shard_hash
        {
            return Err(StorageAdapterError::HashMismatch);
        }
        Ok(retrieve_response_from_store_request(&stored))
    }

    fn delete_shard(&mut self, shard_hash: &Hash) -> Result<bool, StorageAdapterError> {
        let Some(entry) = self.index.remove(shard_hash) else {
            return Ok(false);
        };
        self.clear_slot(entry.slot)?;
        self.used_slots = self.index.len() as u64;
        self.backend.flush().map_err(|_| StorageAdapterError::Io)?;
        Ok(true)
    }
}

#[cfg(feature = "virtual-disk")]
impl VirtualDiskObjectStorage<edgerun_virtual_disk::MemoryBlockBackend> {
    pub fn memory(
        capacity_bytes: u64,
        block_size: u32,
        slot_size: u64,
    ) -> Result<Self, StorageAdapterError> {
        use edgerun_protocols::block::BlockDeviceInfo;
        let block_count = capacity_bytes
            .checked_div(u64::from(block_size))
            .ok_or(StorageAdapterError::InvalidStoredObject)?;
        let backend = edgerun_virtual_disk::MemoryBlockBackend::new(BlockDeviceInfo {
            block_size,
            block_count,
            readonly: false,
            supports_flush: true,
            supports_discard: true,
            supports_write_zeroes: true,
            model: "edgerun-work-memory-virtual-disk".into(),
            serial: "edgerun-work-memory".into(),
        })
        .map_err(|_| StorageAdapterError::Io)?;
        Self::new(backend, slot_size)
    }
}

#[cfg(feature = "virtual-disk")]
#[cfg(not(any(target_os = "none", target_arch = "wasm32")))]
impl VirtualDiskObjectStorage<edgerun_virtual_disk::FileBlockBackend> {
    pub fn open_file_image(
        path: impl AsRef<std::path::Path>,
        capacity_bytes: u64,
        block_size: u32,
        slot_size: u64,
    ) -> Result<Self, StorageAdapterError> {
        let path = path.as_ref();
        if !path.exists() {
            edgerun_virtual_disk::create(&edgerun_virtual_disk::VirtualDiskSpec {
                path: path.to_path_buf(),
                size_bytes: capacity_bytes,
                format: edgerun_virtual_disk::VirtualDiskFormat::Raw,
                sparse: true,
            })
            .map_err(|_| StorageAdapterError::Io)?;
        }
        let backend = edgerun_virtual_disk::FileBlockBackend::open(path, block_size, false)
            .map_err(|_| StorageAdapterError::Io)?;
        Self::new(backend, slot_size)
    }
}

#[cfg(feature = "virtual-disk")]
const VIRTUAL_DISK_SLOT_MAGIC: &[u8; 8] = b"EDGSTOR1";
#[cfg(feature = "virtual-disk")]
const VIRTUAL_DISK_SLOT_HEADER_LEN: usize = 48;

#[cfg(feature = "virtual-disk")]
#[derive(Clone, Debug, PartialEq, Eq)]
struct VirtualDiskSlotHeader {
    shard_hash: Hash,
    payload_len: u64,
}

#[cfg(feature = "virtual-disk")]
impl VirtualDiskSlotHeader {
    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(VIRTUAL_DISK_SLOT_HEADER_LEN);
        out.extend_from_slice(VIRTUAL_DISK_SLOT_MAGIC);
        out.extend_from_slice(&self.payload_len.to_le_bytes());
        out.extend_from_slice(&self.shard_hash);
        out
    }

    fn decode(bytes: &[u8]) -> Result<Option<Self>, StorageAdapterError> {
        if bytes.len() < VIRTUAL_DISK_SLOT_HEADER_LEN {
            return Err(StorageAdapterError::InvalidStoredObject);
        }
        if bytes[..8].iter().all(|byte| *byte == 0) {
            return Ok(None);
        }
        if &bytes[..8] != VIRTUAL_DISK_SLOT_MAGIC {
            return Err(StorageAdapterError::InvalidStoredObject);
        }
        let mut len = [0u8; 8];
        len.copy_from_slice(&bytes[8..16]);
        let payload_len = u64::from_le_bytes(len);
        let mut shard_hash = [0u8; 32];
        shard_hash.copy_from_slice(&bytes[16..48]);
        Ok(Some(Self {
            shard_hash,
            payload_len,
        }))
    }
}

#[cfg(feature = "virtual-disk")]
fn blocks_per_slot(slot_size: u64, block_size: u64) -> Result<u32, StorageAdapterError> {
    if block_size == 0 || slot_size == 0 || slot_size % block_size != 0 {
        return Err(StorageAdapterError::InvalidStoredObject);
    }
    let blocks = slot_size / block_size;
    u32::try_from(blocks).map_err(|_| StorageAdapterError::CapacityExceeded)
}

#[cfg(feature = "std")]
fn scan_store_dir(root: &std::path::Path) -> Result<(u64, usize), std::io::Error> {
    let mut used = 0u64;
    let mut count = 0usize;
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("estore") {
            continue;
        }
        used = used.saturating_add(entry.metadata()?.len());
        count = count.saturating_add(1);
    }
    Ok((used, count))
}

#[cfg(feature = "std")]
fn hash_hex(hash: &Hash) -> alloc::string::String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = alloc::string::String::with_capacity(64);
    for byte in hash {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(feature = "std")]
fn parse_hash_hex(value: &str) -> Option<Hash> {
    if value.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    let bytes = value.as_bytes();
    let mut index = 0;
    while index < 32 {
        let high = hex_value(bytes[index * 2])?;
        let low = hex_value(bytes[index * 2 + 1])?;
        out[index] = (high << 4) | low;
        index += 1;
    }
    Some(out)
}

#[cfg(feature = "std")]
fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::storage_payload::{typed_shard_hash, verify_retrieve_response};

    fn request(bytes: &[u8]) -> ObjectStoreRequest {
        let job_id = [7u8; 32];
        let shard_index = 0;
        ObjectStoreRequest {
            manifest_hash: [3u8; 32],
            job_id,
            shard_index,
            shard_hash: typed_shard_hash(job_id, shard_index, false, bytes),
            original_len: bytes.len() as u64,
            bytes: bytes.to_vec(),
        }
    }

    #[test]
    fn memory_store_roundtrips_request() {
        let mut store = InMemoryObjectStorage::new(1024);
        let object = request(b"hello");
        let retrieve = ObjectRetrieveRequest {
            manifest_hash: object.manifest_hash,
            job_id: object.job_id,
            shard_index: object.shard_index,
            shard_hash: object.shard_hash,
        };
        store.store(object).expect("store");
        let response = store.retrieve(&retrieve).expect("retrieve");
        assert_eq!(response.bytes, b"hello");
        assert!(verify_retrieve_response(&response));
    }

    #[test]
    fn file_store_roundtrips_request() {
        let dir = std::env::temp_dir().join(format!(
            "edgerun-work-file-store-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        let mut store = FileObjectStorage::open(&dir, 4096).expect("open");
        let object = request(b"stored on disk");
        let retrieve = ObjectRetrieveRequest {
            manifest_hash: object.manifest_hash,
            job_id: object.job_id,
            shard_index: object.shard_index,
            shard_hash: object.shard_hash,
        };
        store.store(object).expect("store");
        assert_eq!(store.object_count(), 1);
        let response = store.retrieve(&retrieve).expect("retrieve");
        assert_eq!(response.bytes, b"stored on disk");
        assert!(verify_retrieve_response(&response));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(feature = "virtual-disk")]
    #[test]
    fn virtual_disk_memory_store_roundtrips_request() {
        let mut store = VirtualDiskObjectStorage::memory(64 * 1024, 512, 4096).expect("open");
        let object = request(b"stored in block slots");
        let retrieve = ObjectRetrieveRequest {
            manifest_hash: object.manifest_hash,
            job_id: object.job_id,
            shard_index: object.shard_index,
            shard_hash: object.shard_hash,
        };
        store.store(object).expect("store");
        assert_eq!(store.object_count(), 1);
        let response = store.retrieve(&retrieve).expect("retrieve");
        assert_eq!(response.bytes, b"stored in block slots");
        assert!(verify_retrieve_response(&response));
    }
}
