//! Credential backend — stores secrets via BlobStore (encrypted) + FileIndex (metadata).
//! Secret operations are recorded as signed events in the NODE'S main event stream
//! (NOT a separate log). This ensures all secret mutations are part of the
//! cryptographically linked, prev_hash-chained event log.

use crate::prelude::v1::*;
use alloc::collections::BTreeMap as HashMap;
use std::io;
use std::path::{Path, PathBuf};

use edgerun_storage::{BlobKeySource, BlobStore, BlobStoreConfig, FileIndex};

use edgerun_protocols::core_protocol::protocol::{
    CollectionCreatedPayload, CollectionDeletedPayload, SecretDeletePayload, SecretPutPayload,
};

// ===========================================================================
// Event recorder
// ===========================================================================

/// Callback type for recording a secret event into the node's main stream.
/// The caller provides the event type discriminator and the payload.
pub type SecretEventRecorder = Box<dyn Fn(&str, Vec<u8>) -> io::Result<()> + Send + Sync>;

fn no_op_recorder() -> SecretEventRecorder {
    Box::new(|_event_type: &str, _payload: Vec<u8>| Ok(()))
}

/// Create a no-op event recorder for use when event-stream integration
/// is not needed (e.g. simple token storage without audit trail).
pub fn no_op_event_recorder() -> SecretEventRecorder {
    no_op_recorder()
}

// ===========================================================================
// Metadata
// ===========================================================================

/// Metadata stored alongside each credential.
#[derive(Clone, Debug)]
pub struct CredentialMeta {
    pub label: String,
    pub attributes: HashMap<String, String>,
    pub created_us: u64,
}

impl CredentialMeta {
    pub fn to_json(&self) -> String {
        let mut obj = edgerun_json::Map::new();
        let mut attrs = edgerun_json::Map::new();
        for (k, v) in &self.attributes {
            attrs.insert(k.clone(), edgerun_json::JsonValue::String(v.clone()));
        }
        obj.insert(
            "label".into(),
            edgerun_json::JsonValue::String(self.label.clone()),
        );
        obj.insert("attributes".into(), edgerun_json::JsonValue::Object(attrs));
        obj.insert(
            "created_us".into(),
            edgerun_json::JsonValue::from(self.created_us),
        );
        edgerun_json::to_string(&edgerun_json::JsonValue::Object(obj)).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Option<Self> {
        let tape = edgerun_json::parse_json_tape(s).ok()?;
        let root = tape.root(s)?;
        let label = root
            .get("label")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let created_us = root
            .get("created_us")
            .and_then(|value| value.as_u64())
            .unwrap_or(0);
        let mut attributes = HashMap::new();
        if let Some(attrs) = root.get_object_fields("attributes") {
            for (k, val) in attrs {
                if let Some(vs) = val.as_str() {
                    attributes.insert(k.to_string(), vs.to_string());
                }
            }
        }
        Some(Self {
            label,
            attributes,
            created_us,
        })
    }
}

// ===========================================================================
// Backend
// ===========================================================================

/// Secret service backend.
///
/// - `BlobStore` encrypts/decrypts secret payloads (AES-256-GCM, content-addressed)
/// - `FileIndex` maps (namespace, name) → blob_id with JSON metadata in description
/// - Secret operations are recorded via `record_event` into the node's main
///   signed event stream (NOT a separate log).
pub struct Backend {
    blobs: BlobStore,
    index: FileIndex,
    record_event: SecretEventRecorder,
}

// Static node_id — initialized once at startup.
#[cfg(not(target_os = "none"))]
static NODE_ID: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
#[cfg(target_os = "none")]
static NODE_ID: edgerun_node::rt::Mutex<Option<Vec<u8>>> = edgerun_node::rt::Mutex::new(None);

/// Initialize the static node_id. Must be called exactly once before using Backend.
/// The node_id is public information derived from the node's identity key.
#[cfg(not(target_os = "none"))]
pub fn init_node_id(id: Vec<u8>) -> Result<(), Vec<u8>> {
    NODE_ID.set(id)
}

/// Initialize the static node_id. Must be called exactly once before using Backend.
/// The node_id is public information derived from the node's identity key.
#[cfg(target_os = "none")]
pub fn init_node_id(id: Vec<u8>) -> Result<(), Vec<u8>> {
    let mut guard = NODE_ID.lock();
    if guard.is_some() {
        Err(id)
    } else {
        *guard = Some(id);
        Ok(())
    }
}

/// Get the static node_id. Panics if not initialized.
#[cfg(not(target_os = "none"))]
fn get_node_id() -> &'static [u8] {
    NODE_ID
        .get()
        .expect("node_id not initialized — call init_node_id first")
        .as_slice()
}

/// Copy the static node_id for runtimes where we cannot return a static slice
/// from the lock-protected storage.
#[cfg(target_os = "none")]
fn get_node_id_vec() -> Vec<u8> {
    NODE_ID
        .lock()
        .as_ref()
        .expect("node_id not initialized — call init_node_id first")
        .clone()
}

#[cfg(not(target_os = "none"))]
fn get_node_id_vec() -> Vec<u8> {
    get_node_id().to_vec()
}

impl Backend {
    pub fn new(data_root: PathBuf, record_event: SecretEventRecorder) -> io::Result<Self> {
        let pk = derive_key_from_path(&data_root);

        let blob_cfg = BlobStoreConfig {
            blob_dir: data_root.join("blobs"),
        };
        let _ = std::fs::create_dir_all(&blob_cfg.blob_dir);
        let blobs = BlobStore::open(
            &blob_cfg,
            BlobKeySource::Software {
                private_key_bytes: pk.to_vec(),
            },
        )
        .map_err(|e| io::Error::other(e.to_string()))?;
        let index = FileIndex::open(&data_root).map_err(|e| io::Error::other(e.to_string()))?;

        Ok(Self {
            blobs,
            index,
            record_event,
        })
    }

    /// Creates a backend with a no-op event recorder (for tests).
    pub fn new_noop(data_root: PathBuf) -> io::Result<Self> {
        Self::new(data_root, no_op_recorder())
    }

    /// Map a collection D-Bus path to a credential namespace.
    pub fn coll_to_ns(coll_path: &str) -> String {
        coll_path
            .trim_start_matches("/org/freedesktop/secrets/collections/")
            .into()
    }

    /// Map a collection path + item name to an item D-Bus path.
    pub fn item_path(coll: &str, key: &str) -> String {
        format!("{coll}/{key}")
    }

    /// Compute a stable item key from label + attributes (SHA-256 based).
    pub fn item_key(label: &str, attrs: &[(String, String)]) -> String {
        let mut obj = edgerun_json::Map::new();
        obj.insert(
            "l".into(),
            edgerun_json::JsonValue::String(label.to_string()),
        );
        let mut arr = Vec::new();
        for (k, v) in attrs {
            let mut pair = edgerun_json::Map::new();
            pair.insert("k".into(), edgerun_json::JsonValue::String(k.clone()));
            pair.insert("v".into(), edgerun_json::JsonValue::String(v.clone()));
            arr.push(edgerun_json::JsonValue::Object(pair));
        }
        obj.insert("a".into(), edgerun_json::JsonValue::Array(arr));
        let json =
            edgerun_json::to_string(&edgerun_json::JsonValue::Object(obj)).unwrap_or_default();
        edgerun_protocols::core_protocol::util::bytes_to_hex(
            &edgerun_protocols::core_protocol::crypto::sha256(json.as_bytes()),
        )
    }

    // -- CRUD --

    pub fn put(
        &mut self,
        coll: &str,
        key: &str,
        secret: &[u8],
        label: &str,
        attrs: &[(String, String)],
    ) -> io::Result<()> {
        let ns = Self::coll_to_ns(coll);
        let meta = CredentialMeta {
            label: label.into(),
            attributes: attrs.iter().cloned().collect(),
            created_us: now_us(),
        };
        let description = meta.to_json();

        // Store encrypted blob
        let blob_id = self
            .blobs
            .store(secret, &[get_node_id_vec()])
            .map_err(|e| io::Error::other(e.to_string()))?;

        // Index with metadata in description
        self.index
            .put_credential(&ns, key, &blob_id, Some(&description))
            .map_err(|e| io::Error::other(e.to_string()))?;

        // Record event in node's main stream
        let payload = SecretPutPayload {
            payload_version: 1,
            namespace: ns.clone(),
            key: key.into(),
            label: label.into(),
            attributes: attrs.iter().cloned().collect(),
            secret_blob_id: blob_id,
        };
        (self.record_event)("secret_put", encode_secret_put_payload(&payload))?;

        Ok(())
    }

    pub fn get(&self, coll: &str, key: &str) -> io::Result<Option<(Vec<u8>, CredentialMeta)>> {
        let ns = Self::coll_to_ns(coll);

        let rec = self
            .index
            .get_credential(&ns, key)
            .map_err(|e| io::Error::other(e.to_string()))?;
        let Some(rec) = rec else {
            return Ok(None);
        };

        let entry = self
            .blobs
            .load(&rec.blob_id)
            .map_err(|e| io::Error::other(e.to_string()))?;
        let Some(entry) = entry else {
            return Ok(None);
        };

        let secret = self
            .blobs
            .decrypt(&entry.nonce, &entry.ciphertext)
            .map_err(|e| io::Error::other(e.to_string()))?;

        let meta = rec
            .description
            .as_deref()
            .and_then(CredentialMeta::from_json)
            .unwrap_or_else(|| CredentialMeta {
                label: key.to_string(),
                attributes: HashMap::new(),
                created_us: 0,
            });

        Ok(Some((secret, meta)))
    }

    pub fn delete(&mut self, coll: &str, key: &str) -> io::Result<bool> {
        let ns = Self::coll_to_ns(coll);

        // Get current state for the event (before index mutation)
        let (label, existed) = {
            let rec = self
                .index
                .get_credential(&ns, key)
                .map_err(|e| io::Error::other(e.to_string()))?;
            let label = rec
                .as_ref()
                .and_then(|r| r.description.as_deref())
                .and_then(CredentialMeta::from_json)
                .map(|m| m.label)
                .unwrap_or_else(|| key.to_string());
            (label, rec.is_some())
        };

        // Record the deletion event in the node's main stream
        if existed {
            let payload = SecretDeletePayload {
                payload_version: 1,
                namespace: ns.clone(),
                key: key.into(),
                label: label.clone(),
                reason: String::new(),
            };
            (self.record_event)("secret_delete", encode_secret_delete_payload(&payload))?;
        }

        // Then update the mutable index (rebuildable from events)
        let removed = self
            .index
            .delete_credential(&ns, key)
            .map_err(|e| io::Error::other(e.to_string()))?;

        Ok(removed)
    }

    pub fn list(&self, coll: &str) -> io::Result<Vec<(String, CredentialMeta)>> {
        let ns = Self::coll_to_ns(coll);
        let creds = self
            .index
            .list_credentials(&ns)
            .map_err(|e| io::Error::other(e.to_string()))?;

        Ok(creds
            .into_iter()
            .map(|(key, desc, _ts)| {
                let meta = desc
                    .as_deref()
                    .and_then(CredentialMeta::from_json)
                    .unwrap_or_else(|| CredentialMeta {
                        label: key.clone(),
                        attributes: HashMap::new(),
                        created_us: 0,
                    });
                (key, meta)
            })
            .collect())
    }

    pub fn search(
        &self,
        coll: &str,
        attrs: &[(String, String)],
    ) -> io::Result<Vec<(String, CredentialMeta)>> {
        let items = self.list(coll)?;
        if attrs.is_empty() {
            return Ok(items);
        }
        Ok(items
            .into_iter()
            .filter(|(_, meta)| attrs.iter().all(|(k, v)| meta.attributes.get(k) == Some(v)))
            .collect())
    }

    pub fn list_collections(&self) -> io::Result<Vec<String>> {
        let namespaces = self
            .index
            .list_credential_namespaces()
            .map_err(|e| io::Error::other(e.to_string()))?;
        Ok(namespaces
            .into_iter()
            .map(|ns| format!("/org/freedesktop/secrets/collections/{}", ns))
            .collect())
    }

    pub fn collection_exists(&self, coll: &str) -> bool {
        let ns = Self::coll_to_ns(coll);
        self.index
            .list_credential_namespaces()
            .is_ok_and(|ns_list| ns_list.contains(&ns))
    }

    /// Rebuilds the credential index from secret operation events.
    ///
    /// Events are provided as an iterator of (event_type, payload_bytes) tuples.
    /// In production, these come from replaying the node's main signed event stream
    /// and filtering for secret_* event types.
    ///
    /// Process:
    /// 1. Clear the current index
    /// 2. Replay all events in order
    /// 3. For each `secret_put` → restore index entry
    /// 4. For each `secret_delete` → remove index entry
    pub fn rebuild_index(
        &mut self,
        events: impl Iterator<Item = (String, Vec<u8>)>,
    ) -> io::Result<usize> {
        // Clear current credential index entries
        let namespaces = self
            .index
            .list_credential_namespaces()
            .map_err(|e| io::Error::other(e.to_string()))?;
        for ns in &namespaces {
            let items = self
                .index
                .list_credentials(ns)
                .map_err(|e| io::Error::other(e.to_string()))?;
            for (key, _, _) in items {
                let _ = self.index.delete_credential(ns, &key);
            }
        }

        let mut applied = 0;
        for (event_type, payload) in events {
            match event_type.as_str() {
                "secret_put" => {
                    if let Ok(payload) = decode_secret_put_payload(payload.as_slice()) {
                        let attrs: Vec<(String, String)> = payload
                            .attributes
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect();
                        let meta = CredentialMeta {
                            label: payload.label.clone(),
                            attributes: attrs.into_iter().collect(),
                            created_us: 0,
                        };
                        self.index.put_credential(
                            &payload.namespace,
                            &payload.key,
                            &payload.secret_blob_id,
                            Some(&meta.to_json()),
                        )?;
                        applied += 1;
                    }
                }
                "secret_delete" => {
                    if let Ok(payload) = decode_secret_delete_payload(payload.as_slice()) {
                        self.index
                            .delete_credential(&payload.namespace, &payload.key)?;
                        applied += 1;
                    }
                }
                "collection_created" => {
                    applied += 1; // No-op — namespace created on first put
                }
                "collection_deleted" => {
                    if let Ok(payload) = decode_collection_deleted_payload(payload.as_slice()) {
                        let items = self.index.list_credentials(&payload.collection_name)?;
                        for (key, _, _) in items {
                            self.index
                                .delete_credential(&payload.collection_name, &key)?;
                        }
                        applied += 1;
                    }
                }
                _ => {}
            }
        }

        Ok(applied)
    }

    /// Creates a collection (records the event in the node's stream).
    pub fn create_collection(&mut self, collection_name: &str, label: &str) -> io::Result<()> {
        let payload = CollectionCreatedPayload {
            payload_version: 1,
            collection_name: collection_name.into(),
            label: label.into(),
        };
        (self.record_event)(
            "collection_created",
            encode_collection_created_payload(&payload),
        )
    }

    /// Deletes a collection (records the event and removes items from index).
    pub fn delete_collection(&mut self, collection_name: &str) -> io::Result<u32> {
        let items = self
            .index
            .list_credentials(collection_name)
            .map_err(|e| io::Error::other(e.to_string()))?;
        let count = items.len() as u32;

        for (key, _, _) in &items {
            self.index
                .delete_credential(collection_name, key)
                .map_err(|e| io::Error::other(e.to_string()))?;
        }

        let payload = CollectionDeletedPayload {
            payload_version: 1,
            collection_name: collection_name.into(),
            items_removed: count,
        };
        (self.record_event)(
            "collection_deleted",
            encode_collection_deleted_payload(&payload),
        )?;

        Ok(count)
    }
}

fn now_us() -> u64 {
    edgerun_time::now_unix_micros()
}

/// Derive a deterministic 32-byte key from the data_root path via SHA-256.
fn derive_key_from_path(path: &Path) -> [u8; 32] {
    #[cfg(target_os = "none")]
    let bytes = {
        use std::os::unix::ffi::OsStrExt;
        path.as_os_str().as_bytes()
    };
    #[cfg(not(target_os = "none"))]
    let bytes = path.as_os_str().as_encoded_bytes();
    let hash = edgerun_protocols::core_protocol::crypto::sha256(bytes);
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn init_test_node_id() {
        let _ = init_node_id(vec![0x55; 32]);
    }

    fn tmp_root() -> PathBuf {
        init_test_node_id();
        static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("ss_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    /// Creates a backend that captures all events for later replay.
    fn backend_with_capture(data_root: PathBuf) -> (Backend, Arc<Mutex<Vec<(String, Vec<u8>)>>>) {
        let events: Arc<Mutex<Vec<(String, Vec<u8>)>>> = Arc::new(Mutex::new(Vec::new()));
        let captured = events.clone();
        let recorder: SecretEventRecorder = Box::new(move |event_type, payload| {
            captured
                .lock()
                .unwrap()
                .push((event_type.to_string(), payload));
            Ok(())
        });
        let be = Backend::new(data_root, recorder).unwrap();
        (be, events)
    }

    #[test]
    fn meta_roundtrip() {
        let mut attrs = HashMap::new();
        attrs.insert("xdg:schema".into(), "org.gnome.keyring.Note".into());
        let meta = CredentialMeta {
            label: "My Note".into(),
            attributes: attrs,
            created_us: 12345,
        };
        let json = meta.to_json();
        let back = CredentialMeta::from_json(&json).unwrap();
        assert_eq!(back.label, "My Note");
        assert_eq!(back.created_us, 12345);
        assert_eq!(back.attributes["xdg:schema"], "org.gnome.keyring.Note");
    }

    #[test]
    fn backend_put_get() {
        let root = tmp_root();
        let mut be = Backend::new_noop(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(coll, "test-key", b"super-secret", "Test Label", &[])
            .unwrap();
        let (secret, meta) = be.get(coll, "test-key").unwrap().unwrap();
        assert_eq!(secret, b"super-secret");
        assert_eq!(meta.label, "Test Label");
    }

    #[test]
    fn backend_delete() {
        let root = tmp_root();
        let mut be = Backend::new_noop(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(coll, "del-key", b"secret", "Del Label", &[])
            .unwrap();
        assert!(be.delete(coll, "del-key").unwrap());
        assert!(!be.delete(coll, "del-key").unwrap());
        assert!(be.get(coll, "del-key").unwrap().is_none());
    }

    #[test]
    fn backend_list() {
        let root = tmp_root();
        let mut be = Backend::new_noop(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(coll, "k1", b"v1", "Label 1", &[]).unwrap();
        be.put(coll, "k2", b"v2", "Label 2", &[]).unwrap();
        let items = be.list(coll).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn backend_search_by_attr() {
        let root = tmp_root();
        let mut be = Backend::new_noop(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(
            coll,
            "k1",
            b"v1",
            "GitHub",
            &[("server".into(), "github.com".into())],
        )
        .unwrap();
        be.put(
            coll,
            "k2",
            b"v2",
            "GitLab",
            &[("server".into(), "gitlab.com".into())],
        )
        .unwrap();
        let results = be
            .search(coll, &[("server".into(), "github.com".into())])
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.label, "GitHub");
    }

    #[test]
    fn coll_to_ns() {
        assert_eq!(
            Backend::coll_to_ns("/org/freedesktop/secrets/collections/default"),
            "default"
        );
        assert_eq!(
            Backend::coll_to_ns("/org/freedesktop/secrets/collections/login"),
            "login"
        );
    }

    #[test]
    fn key_derivation_deterministic() {
        let path = PathBuf::from("/tmp/test_data");
        let k1 = derive_key_from_path(&path);
        let k2 = derive_key_from_path(&path);
        assert_eq!(k1, k2);
    }

    #[test]
    fn key_derivation_differs_per_path() {
        let k1 = derive_key_from_path(&PathBuf::from("/tmp/data_a"));
        let k2 = derive_key_from_path(&PathBuf::from("/tmp/data_b"));
        assert_ne!(k1, k2);
    }

    #[test]
    fn rebuild_index_from_events() {
        let root = tmp_root();
        let (mut be, events) = backend_with_capture(root.clone());
        let coll = "/org/freedesktop/secrets/collections/default";

        // Put two items
        be.put(
            coll,
            "k1",
            b"secret1",
            "Key 1",
            &[("server".into(), "github.com".into())],
        )
        .unwrap();
        be.put(coll, "k2", b"secret2", "Key 2", &[]).unwrap();
        // Delete one
        be.delete(coll, "k1").unwrap();

        // Verify current state: only k2 exists
        assert!(be.get(coll, "k1").unwrap().is_none());
        assert!(be.get(coll, "k2").unwrap().is_some());

        // Rebuild from captured events
        let applied = be
            .rebuild_index(events.lock().unwrap().clone().into_iter())
            .unwrap();
        assert_eq!(applied, 3); // 2 puts + 1 delete

        // State should be identical after rebuild
        assert!(be.get(coll, "k1").unwrap().is_none());
        let (secret, meta) = be.get(coll, "k2").unwrap().unwrap();
        assert_eq!(secret, b"secret2");
        assert_eq!(meta.label, "Key 2");
    }

    #[test]
    fn rebuild_index_multiple_collections() {
        let root = tmp_root();
        let (mut be, events) = backend_with_capture(root.clone());
        let coll_default = "/org/freedesktop/secrets/collections/default";
        let coll_login = "/org/freedesktop/secrets/collections/login";

        be.put(coll_default, "d1", b"v1", "D1", &[]).unwrap();
        be.put(coll_login, "l1", b"v2", "L1", &[]).unwrap();
        be.put(coll_default, "d2", b"v3", "D2", &[]).unwrap();

        // Rebuild
        let applied = be
            .rebuild_index(events.lock().unwrap().clone().into_iter())
            .unwrap();
        assert_eq!(applied, 3);

        // All items still retrievable
        assert_eq!(be.get(coll_default, "d1").unwrap().unwrap().0, b"v1");
        assert_eq!(be.get(coll_login, "l1").unwrap().unwrap().0, b"v2");
        assert_eq!(be.get(coll_default, "d2").unwrap().unwrap().0, b"v3");
    }

    #[test]
    fn delete_records_event_before_index_mutation() {
        let root = tmp_root();
        let (mut be, events) = backend_with_capture(root.clone());
        let coll = "/org/freedesktop/secrets/collections/default";

        be.put(coll, "k1", b"secret", "Test", &[]).unwrap();
        be.delete(coll, "k1").unwrap();

        // Captured events should have both put and delete (immutable history)
        let ev = events.lock().unwrap().clone();
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0].0, "secret_put");
        assert_eq!(ev[1].0, "secret_delete");

        // Index should reflect the delete
        assert!(be.get(coll, "k1").unwrap().is_none());
    }

    #[test]
    fn rebuild_preserves_search_attributes() {
        let root = tmp_root();
        let (mut be, events) = backend_with_capture(root.clone());
        let coll = "/org/freedesktop/secrets/collections/default";

        be.put(
            coll,
            "gh",
            b"tok1",
            "GitHub",
            &[
                ("server".into(), "github.com".into()),
                ("type".into(), "access".into()),
            ],
        )
        .unwrap();
        be.put(
            coll,
            "gl",
            b"tok2",
            "GitLab",
            &[("server".into(), "gitlab.com".into())],
        )
        .unwrap();

        // Rebuild
        be.rebuild_index(events.lock().unwrap().clone().into_iter())
            .unwrap();

        // Search still works
        let results = be
            .search(coll, &[("server".into(), "github.com".into())])
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.label, "GitHub");

        let results = be
            .search(coll, &[("type".into(), "access".into())])
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn rebuild_after_multiple_deletes() {
        let root = tmp_root();
        let (mut be, events) = backend_with_capture(root.clone());
        let coll = "/org/freedesktop/secrets/collections/default";

        be.put(coll, "k1", b"v1", "K1", &[]).unwrap();
        be.put(coll, "k2", b"v2", "K2", &[]).unwrap();
        be.put(coll, "k3", b"v3", "K3", &[]).unwrap();
        be.delete(coll, "k1").unwrap();
        be.delete(coll, "k3").unwrap();

        // Rebuild
        let applied = be
            .rebuild_index(events.lock().unwrap().clone().into_iter())
            .unwrap();
        assert_eq!(applied, 5); // 3 puts + 2 deletes

        // Only k2 should remain
        assert!(be.get(coll, "k1").unwrap().is_none());
        assert!(be.get(coll, "k2").unwrap().is_some());
        assert!(be.get(coll, "k3").unwrap().is_none());
    }

    #[test]
    fn list_collections_after_operations() {
        let root = tmp_root();
        let (mut be, events) = backend_with_capture(root.clone());
        let coll_default = "/org/freedesktop/secrets/collections/default";
        let coll_wifi = "/org/freedesktop/secrets/collections/wifi";

        be.put(coll_default, "d1", b"v1", "D1", &[]).unwrap();
        be.put(coll_wifi, "w1", b"wifipass", "Home WiFi", &[])
            .unwrap();

        let collections = be.list_collections().unwrap();
        assert_eq!(collections.len(), 2);
        assert!(collections.contains(&"/org/freedesktop/secrets/collections/default".into()));
        assert!(collections.contains(&"/org/freedesktop/secrets/collections/wifi".into()));

        // Rebuild and verify collections still listed
        be.rebuild_index(events.lock().unwrap().clone().into_iter())
            .unwrap();
        let collections_after = be.list_collections().unwrap();
        assert_eq!(collections_after.len(), 2);
    }

    #[test]
    fn collection_exists_after_rebuild() {
        let root = tmp_root();
        let (mut be, events) = backend_with_capture(root.clone());
        let coll = "/org/freedesktop/secrets/collections/default";

        be.put(coll, "k1", b"v1", "K1", &[]).unwrap();
        assert!(be.collection_exists(coll));

        be.rebuild_index(events.lock().unwrap().clone().into_iter())
            .unwrap();
        assert!(be.collection_exists(coll));
    }

    #[test]
    fn item_key_deterministic_across_backends() {
        let attrs = vec![
            ("server".into(), "github.com".into()),
            ("type".into(), "token".into()),
        ];
        let a = Backend::item_key("My Label", &attrs);
        let b = Backend::item_key("My Label", &attrs);
        assert_eq!(a, b);
    }

    #[test]
    fn item_key_differs_on_different_labels() {
        let a = Backend::item_key("Label A", &[]);
        let b = Backend::item_key("Label B", &[]);
        assert_ne!(a, b);
    }

    #[test]
    fn item_key_differs_on_different_attrs() {
        let a = Backend::item_key("Label", &[("k".into(), "v1".into())]);
        let b = Backend::item_key("Label", &[("k".into(), "v2".into())]);
        assert_ne!(a, b);
    }

    #[test]
    fn empty_secret_roundtrip() {
        let root = tmp_root();
        let mut be = Backend::new_noop(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(coll, "empty", b"", "Empty Secret", &[]).unwrap();
        let (secret, meta) = be.get(coll, "empty").unwrap().unwrap();
        assert_eq!(secret, b"");
        assert_eq!(meta.label, "Empty Secret");
    }

    #[test]
    fn large_secret_roundtrip() {
        let root = tmp_root();
        let mut be = Backend::new_noop(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        let large = vec![0xCCu8; 50_000];
        be.put(coll, "large", &large, "Large Secret", &[]).unwrap();
        let (secret, _) = be.get(coll, "large").unwrap().unwrap();
        assert_eq!(secret, large);
    }

    #[test]
    fn secret_rotation_overwrites() {
        let root = tmp_root();
        let (mut be, events) = backend_with_capture(root.clone());
        let coll = "/org/freedesktop/secrets/collections/default";

        be.put(coll, "api-key", b"old-secret", "API Key", &[])
            .unwrap();
        be.put(coll, "api-key", b"new-secret", "API Key", &[])
            .unwrap();

        let (secret, _) = be.get(coll, "api-key").unwrap().unwrap();
        assert_eq!(secret, b"new-secret");

        let ev = events.lock().unwrap().clone();
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0].0, "secret_put");
        assert_eq!(ev[1].0, "secret_put");

        be.rebuild_index(ev.clone().into_iter()).unwrap();
        let (secret_after, _) = be.get(coll, "api-key").unwrap().unwrap();
        assert_eq!(secret_after, b"new-secret");
    }

    #[test]
    fn backend_search_no_matches() {
        let root = tmp_root();
        let mut be = Backend::new_noop(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(
            coll,
            "k1",
            b"v1",
            "K1",
            &[("server".into(), "github.com".into())],
        )
        .unwrap();
        let results = be
            .search(coll, &[("server".into(), "bitbucket.org".into())])
            .unwrap();
        assert!(results.is_empty());
        let results = be
            .search(coll, &[("nonexistent".into(), "value".into())])
            .unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn backend_search_multiple_attributes() {
        let root = tmp_root();
        let mut be = Backend::new_noop(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(
            coll,
            "k1",
            b"v1",
            "GitHub",
            &[
                ("server".into(), "github.com".into()),
                ("type".into(), "access".into()),
            ],
        )
        .unwrap();
        be.put(
            coll,
            "k2",
            b"v2",
            "GitHub API",
            &[
                ("server".into(), "github.com".into()),
                ("type".into(), "token".into()),
            ],
        )
        .unwrap();
        be.put(
            coll,
            "k3",
            b"v3",
            "GitLab",
            &[
                ("server".into(), "gitlab.com".into()),
                ("type".into(), "token".into()),
            ],
        )
        .unwrap();
        let results = be
            .search(
                coll,
                &[
                    ("server".into(), "github.com".into()),
                    ("type".into(), "access".into()),
                ],
            )
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.label, "GitHub");
    }

    #[test]
    fn backend_get_nonexistent_collection() {
        let root = tmp_root();
        let be = Backend::new_noop(root).unwrap();
        assert!(
            be.get("/org/freedesktop/secrets/collections/nonexistent", "any")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn backend_list_nonexistent_collection() {
        let root = tmp_root();
        let be = Backend::new_noop(root).unwrap();
        assert!(
            be.list("/org/freedesktop/secrets/collections/nonexistent")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn backend_rebuild_after_put_delete_put() {
        let root = tmp_root();
        let (mut be, events) = backend_with_capture(root.clone());
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(coll, "k1", b"first", "First", &[]).unwrap();
        be.delete(coll, "k1").unwrap();
        be.put(coll, "k1", b"second", "Second", &[]).unwrap();
        let applied = be
            .rebuild_index(events.lock().unwrap().clone().into_iter())
            .unwrap();
        assert_eq!(applied, 3);
        let (secret, meta) = be.get(coll, "k1").unwrap().unwrap();
        assert_eq!(secret, b"second");
        assert_eq!(meta.label, "Second");
    }

    #[test]
    fn backend_list_collections_empty() {
        let root = tmp_root();
        let be = Backend::new_noop(root).unwrap();
        assert!(be.list_collections().unwrap().is_empty());
    }

    #[test]
    fn backend_meta_from_json_missing_fields() {
        let meta = CredentialMeta::from_json(r#"{"created_us": 999}"#).unwrap();
        assert_eq!(meta.label, "");
        assert_eq!(meta.created_us, 999);
        assert!(meta.attributes.is_empty());
        let meta = CredentialMeta::from_json(r#"{"label": "Test"}"#).unwrap();
        assert_eq!(meta.label, "Test");
        assert_eq!(meta.created_us, 0);
        assert!(CredentialMeta::from_json("not json").is_none());
        assert!(CredentialMeta::from_json("").is_none());
    }

    #[test]
    fn backend_item_key_edge_cases() {
        let k1 = Backend::item_key("", &[]);
        let k2 = Backend::item_key("", &[]);
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 64);
        let k3 = Backend::item_key("just-label", &[]);
        assert_ne!(k1, k3);
        let k4 = Backend::item_key("Unicode: 🔐", &[]);
        assert_eq!(k4.len(), 64);
        let attrs: Vec<(String, String)> = (0..100)
            .map(|i| (format!("k{}", i), format!("v{}", i)))
            .collect();
        let k5 = Backend::item_key("many-attrs", &attrs);
        assert_eq!(k5.len(), 64);
    }

    #[test]
    fn backend_delete_nonexistent() {
        let root = tmp_root();
        let (mut be, events) = backend_with_capture(root);
        let coll = "/org/freedesktop/secrets/collections/default";
        assert!(!be.delete(coll, "ghost").unwrap());
        // No events should be recorded for deleting a nonexistent key
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn backend_collection_exists_nonexistent() {
        let root = tmp_root();
        let be = Backend::new_noop(root).unwrap();
        assert!(!be.collection_exists("/org/freedesktop/secrets/collections/nonexistent"));
    }
}

fn encode_secret_put_payload(payload: &SecretPutPayload) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(payload)
        .expect("secret put payload must serialize through rkyv")
        .into_vec()
}

fn encode_secret_delete_payload(payload: &SecretDeletePayload) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(payload)
        .expect("secret delete payload must serialize through rkyv")
        .into_vec()
}

fn encode_collection_created_payload(payload: &CollectionCreatedPayload) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(payload)
        .expect("collection created payload must serialize through rkyv")
        .into_vec()
}

fn encode_collection_deleted_payload(payload: &CollectionDeletedPayload) -> Vec<u8> {
    edgerun_protocols::wire::to_bytes::<edgerun_protocols::wire::WireError>(payload)
        .expect("collection deleted payload must serialize through rkyv")
        .into_vec()
}

fn decode_secret_put_payload(bytes: &[u8]) -> Result<SecretPutPayload, &'static str> {
    edgerun_protocols::wire::from_bytes::<SecretPutPayload, edgerun_protocols::wire::WireError>(
        bytes,
    )
    .map_err(|_| "invalid rkyv secret put payload")
}

fn decode_secret_delete_payload(bytes: &[u8]) -> Result<SecretDeletePayload, &'static str> {
    edgerun_protocols::wire::from_bytes::<SecretDeletePayload, edgerun_protocols::wire::WireError>(
        bytes,
    )
    .map_err(|_| "invalid rkyv secret delete payload")
}

fn decode_collection_deleted_payload(
    bytes: &[u8],
) -> Result<CollectionDeletedPayload, &'static str> {
    edgerun_protocols::wire::from_bytes::<
        CollectionDeletedPayload,
        edgerun_protocols::wire::WireError,
    >(bytes)
    .map_err(|_| "invalid rkyv collection deleted payload")
}
