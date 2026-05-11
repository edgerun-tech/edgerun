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
}
