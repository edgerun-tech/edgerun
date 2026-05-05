//! Shared content-addressed storage identifiers.

use crate::prelude::v1::*;
use edgerun_core::protocol::ObjectRef;

use crate::error::StorageError;

/// IDs derived for a logical object and its first stored representation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectIds {
    pub object_id: Vec<u8>,
    pub object_id_hex: String,
    pub representation_id: Vec<u8>,
    pub representation_id_hex: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectBytes {
    pub object_id: Vec<u8>,
    pub object_kind: i32,
    pub content: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectPresence {
    pub object_id_hex: String,
    pub representation_id_hex: String,
    pub blob_id: String,
    pub status: String,
}

/// Backend contract for logical objects and stored representations.
pub trait ContentStore {
    fn put_object(
        &self,
        content: &[u8],
        object_kind: i32,
        recipients: &[Vec<u8>],
    ) -> Result<ObjectRef, StorageError>;

    fn get_object(&self, object_ref: &ObjectRef) -> Result<Option<ObjectBytes>, StorageError>;
}

/// Derives the protocol logical object identity.
#[must_use]
pub fn derive_logical_object_id(canonicalization_id: &[u8], canonical_bytes: &[u8]) -> Vec<u8> {
    edgerun_core::crypto::derive_object_id(canonicalization_id, canonical_bytes)
}

/// Derives a stable stored-representation identity for the simple v0 profile.
///
/// This intentionally includes the logical object id and canonical plaintext,
/// not the randomized ciphertext, so object lookup remains stable while the
/// representation namespace stays distinct from logical object ids.
#[must_use]
pub fn derive_representation_id(
    object_id: &[u8],
    representation_kind: &[u8],
    bytes: &[u8],
) -> Vec<u8> {
    let mut input = Vec::with_capacity(
        b"edgerun:v0:representation".len()
            + 1
            + object_id.len()
            + 1
            + representation_kind.len()
            + 1
            + bytes.len(),
    );
    input.extend_from_slice(b"edgerun:v0:representation");
    input.push(0);
    input.extend_from_slice(object_id);
    input.push(0);
    input.extend_from_slice(representation_kind);
    input.push(0);
    input.extend_from_slice(bytes);
    edgerun_core::crypto::sha256(&input).to_vec()
}

/// Derives both logical and representation IDs for raw-byte objects.
#[must_use]
pub fn raw_object_ids(bytes: &[u8]) -> ObjectIds {
    let object_id = derive_logical_object_id(b"raw-bytes-v0", bytes);
    let representation_id = derive_representation_id(&object_id, b"encrypted-blob-v0", bytes);
    ObjectIds {
        object_id_hex: edgerun_core::util::bytes_to_hex(&object_id),
        representation_id_hex: edgerun_core::util::bytes_to_hex(&representation_id),
        object_id,
        representation_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_and_representation_ids_are_distinct() {
        let ids = raw_object_ids(b"payload");

        assert_eq!(ids.object_id.len(), 32);
        assert_eq!(ids.representation_id.len(), 32);
        assert_ne!(ids.object_id, ids.representation_id);
    }

    #[test]
    fn raw_object_ids_are_stable_for_same_bytes() {
        let a = raw_object_ids(b"same");
        let b = raw_object_ids(b"same");

        assert_eq!(a, b);
    }
}
