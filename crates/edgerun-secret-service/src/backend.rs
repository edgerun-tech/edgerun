//! Credential backend — stores secrets via BlobStore (encrypted) + FileIndex (metadata),
//! and records operations in the append-only event log using protobuf EventEnvelopes.

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use edgerun_storage::{BlobStore, BlobKeySource, BlobStoreConfig, FileIndex};

use crate::event_log::{EventLog, SecretEvent};

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
        obj.insert("label".into(), edgerun_json::JsonValue::String(self.label.clone()));
        obj.insert("attributes".into(), edgerun_json::JsonValue::Object(attrs));
        obj.insert("created_us".into(), edgerun_json::JsonValue::from(self.created_us));
        edgerun_json::to_string(&edgerun_json::JsonValue::Object(obj)).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Option<Self> {
        let v: edgerun_json::JsonValue = edgerun_json::from_str(s).ok()?;
        let label = v.get("label").and_then(edgerun_json::JsonValue::as_str).unwrap_or("").to_string();
        let created_us = v.get("created_us").and_then(edgerun_json::JsonValue::as_u64).unwrap_or(0);
        let mut attributes = HashMap::new();
        if let Some(attrs) = v.get("attributes").and_then(edgerun_json::JsonValue::as_object) {
            for (k, val) in attrs.iter() {
                if let Some(vs) = val.as_str() {
                    attributes.insert(k.clone(), vs.to_string());
                }
            }
        }
        Some(Self { label, attributes, created_us })
    }
}

// ===========================================================================
// Backend
// ===========================================================================

/// Secret service backend.
///
/// - `BlobStore` encrypts/decrypts secret payloads (AES-256-GCM, content-addressed)
/// - `FileIndex` maps (namespace, name) → blob_id with JSON metadata in description
/// - `EventLog` appends a protobuf EventEnvelope for every mutation
pub struct Backend {
    blobs: BlobStore,
    index: FileIndex,
    event_log: EventLog,
}

impl Backend {
    pub fn new(data_root: PathBuf) -> io::Result<Self> {
        let pk = derive_key_from_path(&data_root);

        let blob_cfg = BlobStoreConfig { blob_dir: data_root.join("blobs") };
        let _ = std::fs::create_dir_all(&blob_cfg.blob_dir);
        let blobs = BlobStore::open(&blob_cfg, BlobKeySource::Software { private_key_bytes: pk.to_vec() })
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let index = FileIndex::open(&data_root)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let event_log = EventLog::open(&data_root)?;

        Ok(Self { blobs, index, event_log })
    }

    /// Map a collection D-Bus path to a credential namespace.
    pub fn coll_to_ns(coll_path: &str) -> String {
        coll_path.trim_start_matches("/org/freedesktop/secrets/collections/").into()
    }

    /// Map a collection path + item name to an item D-Bus path.
    pub fn item_path(coll: &str, key: &str) -> String {
        format!("{coll}/{key}")
    }

    /// Compute a stable item key from label + attributes (SHA-256 based).
    pub fn item_key(label: &str, attrs: &[(String, String)]) -> String {
        let mut obj = edgerun_json::Map::new();
        obj.insert("l".into(), edgerun_json::JsonValue::String(label.to_string()));
        let mut arr = Vec::new();
        for (k, v) in attrs {
            let mut pair = edgerun_json::Map::new();
            pair.insert("k".into(), edgerun_json::JsonValue::String(k.clone()));
            pair.insert("v".into(), edgerun_json::JsonValue::String(v.clone()));
            arr.push(edgerun_json::JsonValue::Object(pair));
        }
        obj.insert("a".into(), edgerun_json::JsonValue::Array(arr));
        let json = edgerun_json::to_string(&edgerun_json::JsonValue::Object(obj)).unwrap_or_default();
        edgerun_core::util::bytes_to_hex(&edgerun_core::crypto::sha256(json.as_bytes()))
    }

    // -- CRUD --

    pub fn put(&mut self, coll: &str, key: &str, secret: &[u8], label: &str, attrs: &[(String, String)]) -> io::Result<()> {
        let ns = Self::coll_to_ns(coll);
        let meta = CredentialMeta {
            label: label.into(),
            attributes: attrs.iter().cloned().collect(),
            created_us: now_us(),
        };
        let description = meta.to_json();

        // Store encrypted blob
        let blob_id = self.blobs.store(secret, &[])
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        // Index with metadata in description
        self.index.put_credential(&ns, key, &blob_id, Some(&description))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        // Event log — protobuf EventEnvelope with SecretPutPayload
        self.event_log.record_put(&ns, key, label, attrs, &blob_id)?;

        Ok(())
    }

    pub fn get(&self, coll: &str, key: &str) -> io::Result<Option<(Vec<u8>, CredentialMeta)>> {
        let ns = Self::coll_to_ns(coll);

        let rec = self.index.get_credential(&ns, key)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let Some(rec) = rec else { return Ok(None); };

        let entry = self.blobs.load(&rec.blob_id)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let Some(entry) = entry else { return Ok(None); };

        let secret = self.blobs.decrypt(&entry.nonce, &entry.ciphertext)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        let meta = rec.description.as_deref()
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

        // Get current state for the event log (before index mutation)
        let (label, existed) = {
            let rec = self.index.get_credential(&ns, key)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            let label = rec.as_ref()
                .and_then(|r| r.description.as_deref())
                .and_then(CredentialMeta::from_json)
                .map(|m| m.label)
                .unwrap_or_else(|| key.to_string());
            (label, rec.is_some())
        };

        // 1. Record the deletion in the immutable event log FIRST
        if existed {
            self.event_log.record_delete(&ns, key, &label, None)?;
        }

        // 2. Then update the mutable index (rebuildable from event log)
        let removed = self.index.delete_credential(&ns, key)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        Ok(removed)
    }

    pub fn list(&self, coll: &str) -> io::Result<Vec<(String, CredentialMeta)>> {
        let ns = Self::coll_to_ns(coll);
        let creds = self.index.list_credentials(&ns)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        Ok(creds.into_iter().filter_map(|(key, desc, _ts)| {
            let meta = desc.as_deref()
                .and_then(CredentialMeta::from_json)
                .unwrap_or_else(|| CredentialMeta {
                    label: key.clone(),
                    attributes: HashMap::new(),
                    created_us: 0,
                });
            Some((key, meta))
        }).collect())
    }

    pub fn search(&self, coll: &str, attrs: &[(String, String)]) -> io::Result<Vec<(String, CredentialMeta)>> {
        let items = self.list(coll)?;
        if attrs.is_empty() { return Ok(items); }
        Ok(items.into_iter().filter(|(_, meta)| {
            attrs.iter().all(|(k, v)| meta.attributes.get(k) == Some(v))
        }).collect())
    }

    pub fn list_collections(&self) -> io::Result<Vec<String>> {
        let namespaces = self.index.list_credential_namespaces()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(namespaces.into_iter()
            .map(|ns| format!("/org/freedesktop/secrets/collections/{}", ns))
            .collect())
    }

    pub fn collection_exists(&self, coll: &str) -> bool {
        let ns = Self::coll_to_ns(coll);
        self.index.list_credential_namespaces().map_or(false, |ns_list| ns_list.contains(&ns))
    }

    /// Rebuilds the credential index by replaying the event log.
    ///
    /// The event log is the authoritative history; the FileIndex is a
    /// mutable cache that can be dropped and rebuilt at any time.
    ///
    /// Process:
    /// 1. Clear the current index
    /// 2. Replay all events in order
    /// 3. For each `SecretPut` → restore index entry
    /// 4. For each `SecretDelete` → remove index entry
    /// 5. For each `CollectionCreated` → no-op (namespace created on first put)
    /// 6. For each `CollectionDeleted` → remove all items in that namespace
    pub fn rebuild_index(&mut self) -> io::Result<usize> {
        // Clear current credential index entries
        let namespaces = self.index.list_credential_namespaces()
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        for ns in &namespaces {
            let items = self.index.list_credentials(ns)
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            for (key, _, _) in items {
                let _ = self.index.delete_credential(ns, &key);
            }
        }

        let events = self.event_log.replay()?;
        let mut applied = 0;

        for event in events {
            match event {
                SecretEvent::Put(payload) => {
                    let mut attrs: Vec<(String, String)> = payload.attributes.iter()
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
                SecretEvent::Delete(payload) => {
                    self.index.delete_credential(&payload.namespace, &payload.key)?;
                    applied += 1;
                }
                SecretEvent::CollectionDeleted(payload) => {
                    let items = self.index.list_credentials(&payload.collection_name)?;
                    for (key, _, _) in items {
                        self.index.delete_credential(&payload.collection_name, &key)?;
                    }
                    applied += 1;
                }
                SecretEvent::CollectionCreated(_) => {
                    applied += 1;
                }
            }
        }

        Ok(applied)
    }
}

fn now_us() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_micros() as u64
}

/// Derive a deterministic 32-byte key from the data_root path via SHA-256.
fn derive_key_from_path(path: &PathBuf) -> [u8; 32] {
    let bytes = path.as_os_str().as_encoded_bytes();
    let hash = edgerun_core::crypto::sha256(bytes);
    let mut key = [0u8; 32];
    key.copy_from_slice(&hash);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root() -> PathBuf {
        static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("ss_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn meta_roundtrip() {
        let mut attrs = HashMap::new();
        attrs.insert("xdg:schema".into(), "org.gnome.keyring.Note".into());
        let meta = CredentialMeta { label: "My Note".into(), attributes: attrs, created_us: 12345 };
        let json = meta.to_json();
        let back = CredentialMeta::from_json(&json).unwrap();
        assert_eq!(back.label, "My Note");
        assert_eq!(back.created_us, 12345);
        assert_eq!(back.attributes["xdg:schema"], "org.gnome.keyring.Note");
    }

    #[test]
    fn backend_put_get() {
        let root = tmp_root();
        let mut be = Backend::new(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(coll, "test-key", b"super-secret", "Test Label", &[]).unwrap();
        let (secret, meta) = be.get(coll, "test-key").unwrap().unwrap();
        assert_eq!(secret, b"super-secret");
        assert_eq!(meta.label, "Test Label");
    }

    #[test]
    fn backend_delete() {
        let root = tmp_root();
        let mut be = Backend::new(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(coll, "del-key", b"secret", "Del Label", &[]).unwrap();
        assert!(be.delete(coll, "del-key").unwrap());
        assert!(!be.delete(coll, "del-key").unwrap());
        assert!(be.get(coll, "del-key").unwrap().is_none());
    }

    #[test]
    fn backend_list() {
        let root = tmp_root();
        let mut be = Backend::new(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(coll, "k1", b"v1", "Label 1", &[]).unwrap();
        be.put(coll, "k2", b"v2", "Label 2", &[]).unwrap();
        let items = be.list(coll).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn backend_search_by_attr() {
        let root = tmp_root();
        let mut be = Backend::new(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(coll, "k1", b"v1", "GitHub", &[("server".into(), "github.com".into())]).unwrap();
        be.put(coll, "k2", b"v2", "GitLab", &[("server".into(), "gitlab.com".into())]).unwrap();
        let results = be.search(coll, &[("server".into(), "github.com".into())]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.label, "GitHub");
    }

    #[test]
    fn coll_to_ns() {
        assert_eq!(Backend::coll_to_ns("/org/freedesktop/secrets/collections/default"), "default");
        assert_eq!(Backend::coll_to_ns("/org/freedesktop/secrets/collections/login"), "login");
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
    fn rebuild_index_from_event_log() {
        let root = tmp_root();
        let mut be = Backend::new(root.clone()).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";

        // Put two items
        be.put(coll, "k1", b"secret1", "Key 1", &[("server".into(), "github.com".into())]).unwrap();
        be.put(coll, "k2", b"secret2", "Key 2", &[]).unwrap();
        // Delete one
        be.delete(coll, "k1").unwrap();

        // Verify current state: only k2 exists
        assert!(be.get(coll, "k1").unwrap().is_none());
        assert!(be.get(coll, "k2").unwrap().is_some());

        // Rebuild from event log
        let applied = be.rebuild_index().unwrap();
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
        let mut be = Backend::new(root.clone()).unwrap();
        let coll_default = "/org/freedesktop/secrets/collections/default";
        let coll_login = "/org/freedesktop/secrets/collections/login";

        be.put(coll_default, "d1", b"v1", "D1", &[]).unwrap();
        be.put(coll_login, "l1", b"v2", "L1", &[]).unwrap();
        be.put(coll_default, "d2", b"v3", "D2", &[]).unwrap();

        // Rebuild
        let applied = be.rebuild_index().unwrap();
        assert_eq!(applied, 3);

        // All items still retrievable
        assert_eq!(be.get(coll_default, "d1").unwrap().unwrap().0, b"v1");
        assert_eq!(be.get(coll_login, "l1").unwrap().unwrap().0, b"v2");
        assert_eq!(be.get(coll_default, "d2").unwrap().unwrap().0, b"v3");
    }

    #[test]
    fn delete_records_event_before_index_mutation() {
        let root = tmp_root();
        let mut be = Backend::new(root.clone()).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";

        be.put(coll, "k1", b"secret", "Test", &[]).unwrap();

        // Delete
        be.delete(coll, "k1").unwrap();

        // Event log should have both put and delete (immutable history)
        let events = be.event_log.replay().unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], SecretEvent::Put(_)));
        assert!(matches!(&events[1], SecretEvent::Delete(_)));

        // Index should reflect the delete
        assert!(be.get(coll, "k1").unwrap().is_none());
    }

    #[test]
    fn rebuild_preserves_search_attributes() {
        let root = tmp_root();
        let mut be = Backend::new(root.clone()).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";

        be.put(coll, "gh", b"tok1", "GitHub", &[("server".into(), "github.com".into()), ("type".into(), "token".into())]).unwrap();
        be.put(coll, "gl", b"tok2", "GitLab", &[("server".into(), "gitlab.com".into())]).unwrap();

        // Rebuild
        be.rebuild_index().unwrap();

        // Search still works
        let results = be.search(coll, &[("server".into(), "github.com".into())]).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.label, "GitHub");

        let results = be.search(coll, &[("type".into(), "token".into())]).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn rebuild_after_multiple_deletes() {
        let root = tmp_root();
        let mut be = Backend::new(root.clone()).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";

        be.put(coll, "k1", b"v1", "K1", &[]).unwrap();
        be.put(coll, "k2", b"v2", "K2", &[]).unwrap();
        be.put(coll, "k3", b"v3", "K3", &[]).unwrap();
        be.delete(coll, "k1").unwrap();
        be.delete(coll, "k3").unwrap();

        // Rebuild
        let applied = be.rebuild_index().unwrap();
        assert_eq!(applied, 5); // 3 puts + 2 deletes

        // Only k2 should remain
        assert!(be.get(coll, "k1").unwrap().is_none());
        assert!(be.get(coll, "k2").unwrap().is_some());
        assert!(be.get(coll, "k3").unwrap().is_none());
    }

    #[test]
    fn list_collections_after_operations() {
        let root = tmp_root();
        let mut be = Backend::new(root.clone()).unwrap();
        let coll_default = "/org/freedesktop/secrets/collections/default";
        let coll_wifi = "/org/freedesktop/secrets/collections/wifi";

        be.put(coll_default, "d1", b"v1", "D1", &[]).unwrap();
        be.put(coll_wifi, "w1", b"wifipass", "Home WiFi", &[]).unwrap();

        let collections = be.list_collections().unwrap();
        assert_eq!(collections.len(), 2);
        assert!(collections.contains(&"/org/freedesktop/secrets/collections/default".into()));
        assert!(collections.contains(&"/org/freedesktop/secrets/collections/wifi".into()));

        // Rebuild and verify collections still listed
        be.rebuild_index().unwrap();
        let collections_after = be.list_collections().unwrap();
        assert_eq!(collections_after.len(), 2);
    }

    #[test]
    fn collection_exists_after_rebuild() {
        let root = tmp_root();
        let mut be = Backend::new(root.clone()).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";

        be.put(coll, "k1", b"v1", "K1", &[]).unwrap();
        assert!(be.collection_exists(coll));

        be.rebuild_index().unwrap();
        assert!(be.collection_exists(coll));
    }

    #[test]
    fn item_key_deterministic_across_backends() {
        let attrs = vec![
            ("server".into(), "github.com".into()),
            ("type".into(), "password".into()),
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
        let mut be = Backend::new(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(coll, "empty", b"", "Empty Secret", &[]).unwrap();
        let (secret, meta) = be.get(coll, "empty").unwrap().unwrap();
        assert_eq!(secret, b"");
        assert_eq!(meta.label, "Empty Secret");
    }

    #[test]
    fn large_secret_roundtrip() {
        let root = tmp_root();
        let mut be = Backend::new(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        let large = vec![0xCCu8; 50_000];
        be.put(coll, "large", &large, "Large Secret", &[]).unwrap();
        let (secret, _) = be.get(coll, "large").unwrap().unwrap();
        assert_eq!(secret, large);
    }

    #[test]
    fn secret_rotation_overwrites() {
        let root = tmp_root();
        let mut be = Backend::new(root.clone()).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";

        be.put(coll, "api-key", b"old-secret", "API Key", &[]).unwrap();
        be.put(coll, "api-key", b"new-secret", "API Key", &[]).unwrap();

        let (secret, _) = be.get(coll, "api-key").unwrap().unwrap();
        assert_eq!(secret, b"new-secret");

        // Event log should show two put events
        let events = be.event_log.replay().unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], SecretEvent::Put(_)));
        assert!(matches!(&events[1], SecretEvent::Put(_)));

        // Rebuild should preserve latest state (last put wins)
        be.rebuild_index().unwrap();
        let (secret_after, _) = be.get(coll, "api-key").unwrap().unwrap();
        assert_eq!(secret_after, b"new-secret");
    }
}
