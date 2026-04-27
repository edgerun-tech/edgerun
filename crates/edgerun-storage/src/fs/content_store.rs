//! Filesystem content-addressed object backend.

use crate::prelude::v1::*;
use edgerun_proto::edgerun::v0::common::ObjectRef;
use std::sync::Arc;

use crate::blobs::BlobStore;
use crate::core::{cas::raw_object_ids, ContentStore, ObjectBytes};
use crate::error::StorageError;
use crate::file_index::FileIndex;

/// Hosted object store backed by encrypted blob files and `FileIndex`.
#[derive(Clone)]
pub struct FsContentStore {
    blobs: Arc<BlobStore>,
    index: Arc<FileIndex>,
}

impl FsContentStore {
    pub fn new(blobs: Arc<BlobStore>, index: Arc<FileIndex>) -> Self {
        Self { blobs, index }
    }
}

impl ContentStore for FsContentStore {
    fn put_object(
        &self,
        content: &[u8],
        object_kind: i32,
        recipients: &[Vec<u8>],
    ) -> Result<ObjectRef, StorageError> {
        let ids = raw_object_ids(content);

        // Create the descriptor now to keep this backend responsible for the
        // logical object model. Persistence of descriptors can be added without
        // changing `NodeStore` callers.
        let _descriptor = edgerun_proto::edgerun::v0::object::LogicalObjectDescriptor {
            descriptor_version: 1,
            object_id: ids.object_id.clone(),
            object_kind,
            object_schema_version: 1,
            canonicalization_id: "raw-bytes-v0".into(),
            canonical_digest: Some(edgerun_proto::edgerun::v0::common::Digest {
                algorithm: 1,
                value: edgerun_core::crypto::sha256(content).to_vec(),
            }),
            canonical_size: content.len() as u64,
            created_at: Some(now_timestamp()),
            producer: None,
            describes_object: None,
            object_metadata: None,
        };

        let blob_id = self.blobs.store(content, recipients)?;
        self.index
            .mark_object_present(&ids.object_id_hex, &ids.representation_id_hex, &blob_id)?;

        Ok(ObjectRef {
            object_id: ids.object_id,
            object_kind: Some(object_kind),
        })
    }

    fn get_object(&self, object_ref: &ObjectRef) -> Result<Option<ObjectBytes>, StorageError> {
        let object_id_hex = edgerun_core::util::bytes_to_hex(&object_ref.object_id);

        if !self.index.is_object_present(&object_id_hex)? {
            return Ok(None);
        }

        let blob_id = self
            .index
            .lookup_objects(std::slice::from_ref(&object_id_hex))?
            .into_iter()
            .find(|(id, _, _, status)| id == &object_id_hex && status == "present")
            .and_then(|(_, _, blob, _)| blob)
            .ok_or_else(|| {
                StorageError::InvalidBlob(format!(
                    "object {object_id_hex} is marked present without a blob id"
                ))
            })?;

        let Some(entry) = self.blobs.load(&blob_id)? else {
            return Ok(None);
        };
        let content = self.blobs.decrypt(&entry.nonce, &entry.ciphertext)?;

        Ok(Some(ObjectBytes {
            object_id: object_ref.object_id.clone(),
            object_kind: object_ref.object_kind.unwrap_or(0),
            content,
        }))
    }
}

fn now_timestamp() -> prost_types::Timestamp {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    prost_types::Timestamp {
        seconds: now.as_secs() as i64,
        nanos: now.subsec_nanos() as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blobs::{BlobKeySource, BlobStoreConfig};
    use std::path::PathBuf;

    fn tmp_data_root() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "fs_content_store_test_{}_{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn make_store(data_root: &std::path::Path) -> FsContentStore {
        let blobs = Arc::new(
            BlobStore::open(
                &BlobStoreConfig {
                    blob_dir: data_root.join("blobs"),
                },
                BlobKeySource::Software {
                    private_key_bytes: vec![0x42; 32],
                },
            )
            .unwrap(),
        );
        let index = Arc::new(FileIndex::open(data_root).unwrap());
        FsContentStore::new(blobs, index)
    }

    #[test]
    fn stores_and_loads_by_logical_object_ref() {
        let data_root = tmp_data_root();
        let store = make_store(&data_root);

        let object_ref = store
            .put_object(b"content-store payload", 1, &[vec![0x99; 32]])
            .unwrap();
        let loaded = store.get_object(&object_ref).unwrap().unwrap();

        assert_eq!(loaded.object_id, object_ref.object_id);
        assert_eq!(loaded.object_kind, 1);
        assert_eq!(loaded.content, b"content-store payload");

        let _ = std::fs::remove_dir_all(data_root);
    }
}
