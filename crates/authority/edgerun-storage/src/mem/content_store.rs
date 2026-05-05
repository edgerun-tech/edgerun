//! Memory-only content-addressed object store for tests and simple backends.

use crate::prelude::v1::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use edgerun_core::protocol::ObjectRef;

use crate::core::{cas::raw_object_ids, ContentStore, ObjectBytes};
use crate::error::StorageError;

#[derive(Clone, Debug)]
struct MemObject {
    object_kind: i32,
    content: Vec<u8>,
}

/// Thread-safe in-memory implementation of logical object storage.
#[derive(Clone, Default)]
pub struct MemContentStore {
    objects: Arc<Mutex<HashMap<String, MemObject>>>,
}

impl MemContentStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ContentStore for MemContentStore {
    fn put_object(
        &self,
        content: &[u8],
        object_kind: i32,
        _recipients: &[Vec<u8>],
    ) -> Result<ObjectRef, StorageError> {
        let ids = raw_object_ids(content);
        let mut objects = self
            .objects
            .lock()
            .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?;

        objects.insert(
            ids.object_id_hex.clone(),
            MemObject {
                object_kind,
                content: content.to_vec(),
            },
        );

        Ok(ObjectRef {
            object_id: ids.object_id,
            object_kind: Some(object_kind),
        })
    }

    fn get_object(&self, object_ref: &ObjectRef) -> Result<Option<ObjectBytes>, StorageError> {
        let object_id_hex = edgerun_core::util::bytes_to_hex(&object_ref.object_id);
        let objects = self
            .objects
            .lock()
            .map_err(|e| StorageError::Io(std::io::Error::other(e.to_string())))?;

        let Some(object) = objects.get(&object_id_hex) else {
            return Ok(None);
        };
        let ids = raw_object_ids(&object.content);
        if ids.object_id != object_ref.object_id {
            return Err(StorageError::InvalidBlob(format!(
                "stored bytes for object {object_id_hex} have object id {}",
                ids.object_id_hex
            )));
        }

        Ok(Some(ObjectBytes {
            object_id: object_ref.object_id.clone(),
            object_kind: object_ref.object_kind.unwrap_or(object.object_kind),
            content: object.content.clone(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_and_loads_object_by_logical_reference() {
        let store = MemContentStore::new();
        let object_ref = store
            .put_object(b"mem payload", 7, &[vec![0x44; 32]])
            .unwrap();
        let loaded = store.get_object(&object_ref).unwrap().unwrap();

        assert_eq!(loaded.object_id, object_ref.object_id);
        assert_eq!(loaded.object_kind, 7);
        assert_eq!(loaded.content, b"mem payload");
    }

    #[test]
    fn missing_object_returns_none() {
        let store = MemContentStore::new();
        let missing = ObjectRef {
            object_id: vec![0x01; 32],
            object_kind: None,
        };
        assert!(store.get_object(&missing).unwrap().is_none());
    }

    #[test]
    fn rejects_object_ref_that_does_not_match_stored_bytes() {
        let store = MemContentStore::new();
        let mut object_ref = store.put_object(b"mem payload", 7, &[]).unwrap();
        object_ref.object_id[0] ^= 0xff;
        let bad_hex = edgerun_core::util::bytes_to_hex(&object_ref.object_id);
        store.objects.lock().unwrap().insert(
            bad_hex,
            MemObject {
                object_kind: 7,
                content: b"mem payload".to_vec(),
            },
        );

        let result = store.get_object(&object_ref);
        assert!(matches!(result, Err(StorageError::InvalidBlob(_))));
    }
}
