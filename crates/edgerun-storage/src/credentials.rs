//! Named credential store — stores and retrieves secrets by namespace/name.
//!
//! Builds on top of the encrypted `BlobStore` to provide a high-level API
//! for managing credentials (API keys, passwords, tokens, private keys, etc.).
//!
//! ## Architecture
//!
//! - **Encryption**: Credentials are encrypted via `BlobStore` using AES-256-GCM
//! - **Naming**: Credentials are addressed by `(namespace, name)` pairs
//! - **Index**: A `FileIndex` maps `{namespace}/{name}` → `blob_id`
//! - **Key management**: Inherits blob key derivation (software or hardware-sealed)
//!
//! ## Example
//!
//! ```ignore
//! let store = CredentialStore::new(blobs, index);
//!
//! // Store a credential
//! store.put("wifi", "home-network", b"my-wifi-password", Some("WPA2 passphrase"))?;
//!
//! // Retrieve it
//! let secret = store.get("wifi", "home-network")?;
//! assert_eq!(secret, Some(b"my-wifi-password".to_vec()));
//!
//! // List credentials in a namespace
//! let names = store.list("wifi")?;
//!
//! // Delete
//! store.delete("wifi", "home-network")?;
//! ```

use crate::blobs::BlobStore;
use crate::file_index::FileIndex;
use crate::error::StorageError;

/// High-level credential store backed by encrypted blobs.
pub struct CredentialStore {
    blobs: BlobStore,
    index: FileIndex,
}

impl CredentialStore {
    /// Creates a new credential store.
    ///
    /// The `blobs` handle provides encryption; the `index` tracks
    /// the mapping from `(namespace, name)` to encrypted blob IDs.
    pub fn new(blobs: BlobStore, index: FileIndex) -> Self {
        Self { blobs, index }
    }

    // -----------------------------------------------------------------------
    // Core API
    // -----------------------------------------------------------------------

    /// Store a secret under the given namespace and name.
    ///
    /// If a credential with the same `(namespace, name)` already exists,
    /// it is overwritten (a new blob is created; the old blob remains on
    /// disk as orphaned content-addressed data).
    ///
    /// The `description` is optional metadata stored for auditing purposes.
    pub fn put(
        &self,
        namespace: &str,
        name: &str,
        secret: &[u8],
        description: Option<&str>,
    ) -> Result<(), StorageError> {
        // Encrypt and store as a blob (no recipients — node decrypts with its own key)
        let blob_id = self.blobs.store(secret, &[])?;

        // Index the mapping
        self.index.put_credential(namespace, name, &blob_id, description)?;

        Ok(())
    }

    /// Retrieve a secret by namespace and name.
    ///
    /// Returns `Ok(None)` if the credential does not exist.
    /// Returns `Err(StorageError::Decryption)` if the blob is corrupt.
    pub fn get(&self, namespace: &str, name: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let Some(record) = self.index.get_credential(namespace, name)? else {
            return Ok(None);
        };

        let Some(entry) = self.blobs.load(&record.blob_id)? else {
            return Ok(None);
        };

        let plaintext = self.blobs.decrypt(&entry.nonce, &entry.ciphertext)?;
        Ok(Some(plaintext))
    }

    /// Delete a credential by namespace and name.
    ///
    /// Removes the index entry only — the encrypted blob on disk is
    /// content-addressed and left in place (can be cleaned up by a
    /// future garbage collection pass).
    ///
    /// Returns `true` if the credential existed, `false` if it was already gone.
    pub fn delete(&self, namespace: &str, name: &str) -> Result<bool, StorageError> {
        self.index.delete_credential(namespace, name).map_err(StorageError::Io)
    }

    /// List all credential names in a namespace.
    ///
    /// Returns `(name, description, stored_at)` tuples sorted by name.
    pub fn list(&self, namespace: &str) -> Result<Vec<(String, Option<String>, i64)>, StorageError> {
        Ok(self.index.list_credentials(namespace)?)
    }

    /// List all namespaces that have stored credentials.
    pub fn list_namespaces(&self) -> Result<Vec<String>, StorageError> {
        Ok(self.index.list_credential_namespaces()?)
    }

    /// Check if a credential exists.
    pub fn exists(&self, namespace: &str, name: &str) -> Result<bool, StorageError> {
        Ok(self.index.get_credential(namespace, name)?.is_some())
    }

    /// Rotate a credential — store a new secret under the same name.
    ///
    /// This is equivalent to `put()` but makes intent clearer in calling code.
    /// The old encrypted blob becomes orphaned content.
    pub fn rotate(
        &self,
        namespace: &str,
        name: &str,
        new_secret: &[u8],
        description: Option<&str>,
    ) -> Result<(), StorageError> {
        self.put(namespace, name, new_secret, description)
    }
}

#[cfg(test)]
mod tests {
    use super::CredentialStore;
    use crate::blobs::{BlobStore, BlobKeySource, BlobStoreConfig};
    use crate::file_index::FileIndex;
    use std::path::PathBuf;

    fn tmp_data_root() -> PathBuf {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("cred_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn make_store(data_root: PathBuf) -> CredentialStore {
        let private_key = [0xBBu8; 32];
        let blob_config = BlobStoreConfig {
            blob_dir: data_root.join("blobs"),
        };
        let blobs = BlobStore::open(&blob_config, BlobKeySource::Software {
            private_key_bytes: private_key.to_vec(),
        }).unwrap();
        let index = FileIndex::open(&data_root).unwrap();
        CredentialStore::new(blobs, index)
    }

    // -- Basic CRUD --

    #[test]
    fn put_and_get_secret() {
        let root = tmp_data_root();
        let store = make_store(root);

        store.put("api", "github", b"ghp_xxx123", Some("GitHub PAT")).unwrap();
        let secret = store.get("api", "github").unwrap();
        assert_eq!(secret, Some(b"ghp_xxx123".to_vec()));
    }

    #[test]
    fn get_missing_returns_none() {
        let root = tmp_data_root();
        let store = make_store(root);

        assert!(store.get("api", "nonexistent").unwrap().is_none());
    }

    #[test]
    fn delete_secret() {
        let root = tmp_data_root();
        let store = make_store(root);

        store.put("db", "postgres", b"password123", None).unwrap();
        assert!(store.exists("db", "postgres").unwrap());

        let deleted = store.delete("db", "postgres").unwrap();
        assert!(deleted);
        assert!(!store.exists("db", "postgres").unwrap());
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let root = tmp_data_root();
        let store = make_store(root);

        assert!(!store.delete("db", "ghost").unwrap());
    }

    #[test]
    fn overwrite_existing_secret() {
        let root = tmp_data_root();
        let store = make_store(root);

        store.put("api", "stripe", b"sk_old", None).unwrap();
        store.put("api", "stripe", b"sk_new", Some("rotated")).unwrap();

        let secret = store.get("api", "stripe").unwrap();
        assert_eq!(secret, Some(b"sk_new".to_vec()));
    }

    // -- Listing --

    #[test]
    fn list_credentials_in_namespace() {
        let root = tmp_data_root();
        let store = make_store(root);

        store.put("wifi", "home", b"home-pass", None).unwrap();
        store.put("wifi", "office", b"office-pass", Some("WPA2")).unwrap();
        store.put("api", "github", b"ghp_xxx", None).unwrap();

        let creds = store.list("wifi").unwrap();
        assert_eq!(creds.len(), 2);
        assert_eq!(creds[0].0, "home");
        assert_eq!(creds[1].0, "office");
        assert_eq!(creds[1].1, Some("WPA2".to_string()));
    }

    #[test]
    fn list_empty_namespace() {
        let root = tmp_data_root();
        let store = make_store(root);

        let creds = store.list("empty").unwrap();
        assert!(creds.is_empty());
    }

    #[test]
    fn list_namespaces() {
        let root = tmp_data_root();
        let store = make_store(root);

        store.put("ns-a", "key1", b"v1", None).unwrap();
        store.put("ns-b", "key2", b"v2", None).unwrap();
        store.put("ns-a", "key3", b"v3", None).unwrap();

        let namespaces = store.list_namespaces().unwrap();
        assert_eq!(namespaces, vec!["ns-a", "ns-b"]);
    }

    // -- Rotation --

    #[test]
    fn rotate_credential() {
        let root = tmp_data_root();
        let store = make_store(root);

        store.put("api", "aws", b"old-key", None).unwrap();
        store.rotate("api", "aws", b"new-key-2025", Some("rotated Jan 2025")).unwrap();

        let secret = store.get("api", "aws").unwrap();
        assert_eq!(secret, Some(b"new-key-2025".to_vec()));
    }

    // -- Persistence across of restart --

    #[test]
    fn secret_survives_store_restart() {
        let root = tmp_data_root();

        // First session: store
        {
            let store = make_store(root.clone());
            store.put("persistent", "key", b"persistent-value", None).unwrap();
        }

        // Second session: retrieve
        {
            let store = make_store(root.clone());
            let secret = store.get("persistent", "key").unwrap();
            assert_eq!(secret, Some(b"persistent-value".to_vec()));
        }
    }

    // -- Edge cases --

    #[test]
    fn empty_secret() {
        let root = tmp_data_root();
        let store = make_store(root);

        store.put("test", "empty", b"", None).unwrap();
        let secret = store.get("test", "empty").unwrap();
        assert_eq!(secret, Some(b"".to_vec()));
    }

    #[test]
    fn large_secret() {
        let root = tmp_data_root();
        let store = make_store(root);

        let large = vec![0xCCu8; 50_000];
        store.put("test", "large", &large, None).unwrap();
        let secret = store.get("test", "large").unwrap();
        assert_eq!(secret, Some(large));
    }

    #[test]
    fn special_characters_in_name() {
        let root = tmp_data_root();
        let store = make_store(root);

        store.put("test", "key-with-dashes", b"v1", None).unwrap();
        store.put("test", "key_with_underscores", b"v2", None).unwrap();
        store.put("test", "key.with.dots", b"v3", None).unwrap();

        assert_eq!(store.get("test", "key-with-dashes").unwrap(), Some(b"v1".to_vec()));
        assert_eq!(store.get("test", "key_with_underscores").unwrap(), Some(b"v2".to_vec()));
        assert_eq!(store.get("test", "key.with.dots").unwrap(), Some(b"v3".to_vec()));
    }

    #[test]
    fn special_characters_in_namespace() {
        let root = tmp_data_root();
        let store = make_store(root);

        store.put("com.example", "api-key", b"secret", None).unwrap();
        assert_eq!(store.get("com.example", "api-key").unwrap(), Some(b"secret".to_vec()));
    }

    #[test]
    fn utf8_secret() {
        let root = tmp_data_root();
        let store = make_store(root);

        let utf8_secret = "🔐 secret Unicode key 🔑".as_bytes();
        store.put("test", "utf8", utf8_secret, Some("Unicode test")).unwrap();
        let retrieved = store.get("test", "utf8").unwrap();
        assert_eq!(retrieved, Some(utf8_secret.to_vec()));
    }

    #[test]
    fn list_shows_description() {
        let root = tmp_data_root();
        let store = make_store(root);

        store.put("api", "key1", b"v1", Some("Production API key")).unwrap();
        store.put("api", "key2", b"v2", None).unwrap();

        let creds = store.list("api").unwrap();
        assert_eq!(creds[0].1, Some("Production API key".to_string()));
        assert!(creds[1].1.is_none());
    }

    #[test]
    fn ciphertext_differs_from_plaintext() {
        let root = tmp_data_root();
        let store = make_store(root.clone());

        store.put("verify", "test", b"plaintext", None).unwrap();

        // Verify the blob file on disk doesn't contain the plaintext
        let blob_dir = root.join("blobs");
        for entry in std::fs::read_dir(blob_dir).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().map_or(false, |e| e == "blob") {
                let content = std::fs::read(entry.path()).unwrap();
                assert!(!content.windows(9).any(|w| w == b"plaintext"));
            }
        }
    }
}
