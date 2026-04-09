//! Credential backend — stores secrets via BlobStore (encrypted) + FileIndex (metadata),
//! and records operations in an append-only event log.

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use edgerun_storage::{BlobStore, BlobKeySource, BlobStoreConfig, FileIndex};

use crate::event_log::{EventLog, SecretEvent, SecretOp};

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
/// - `BlobStore` encrypts/decrypts secret payloads
/// - `FileIndex` maps (namespace, name) → blob_id with JSON metadata in description
/// - `EventLog` appends an audit record for every mutation
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

        // Event log
        self.event_log.append(&SecretEvent {
            ts_us: now_us(),
            ns: ns.clone(),
            key: key.to_string(),
            op: SecretOp::Put { label: label.into(), attributes: attrs.iter().cloned().collect() },
        })?;

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
        let existed = self.index.delete_credential(&ns, key)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;

        if existed {
            self.event_log.append(&SecretEvent {
                ts_us: now_us(),
                ns: ns.clone(),
                key: key.to_string(),
                op: SecretOp::Delete,
            })?;
        }

        Ok(existed)
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
}
