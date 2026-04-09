//! Credential backend — stores encrypted blobs via edgerun-storage,
//! tracks metadata via edgerun-json.

use std::collections::HashMap;
use std::io;
use std::path::PathBuf;

use edgerun_storage::{BlobStore, BlobKeySource, BlobStoreConfig, FileIndex};
use edgerun_json::{from_str, to_string, JsonValue};

// ===========================================================================
// Metadata
// ===========================================================================

/// Metadata stored alongside each credential (encrypted in the blob sidecar).
#[derive(Clone, Debug)]
pub struct CredentialMeta {
    pub label: String,
    pub attributes: HashMap<String, String>,
    pub created_us: u64,
}

impl CredentialMeta {
    pub fn to_json(&self) -> String {
        let mut attrs = JsonValue::Object(edgerun_json::Map::new());
        for (k, v) in &self.attributes {
            attrs.insert(k.clone(), JsonValue::String(v.clone()));
        }
        let root = json!({
            "label": self.label.clone(),
            "attributes": attrs,
            "created_us": self.created_us,
        });
        to_string(&root).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Option<Self> {
        let v: JsonValue = from_str(s).ok()?;
        let label = v.get("label").and_then(JsonValue::as_str).unwrap_or("").to_string();
        let created_us = v.get("created_us").and_then(JsonValue::as_u64).unwrap_or(0);
        let mut attributes = HashMap::new();
        if let Some(attrs) = v.get("attributes").and_then(JsonValue::as_object) {
            for (k, v) in attrs.iter() {
                if let Some(vs) = v.as_str() {
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

pub struct Backend {
    blobs: BlobStore,
    index: FileIndex,
}

impl Backend {
    pub fn new(data_root: PathBuf) -> io::Result<Self> {
        let pk = [0xBBu8; 32];
        let blob_cfg = BlobStoreConfig { blob_dir: data_root.join("blobs") };
        let _ = std::fs::create_dir_all(&data_root.join("blobs"));
        let blobs = BlobStore::open(&blob_cfg, BlobKeySource::Software { private_key_bytes: pk.to_vec() })
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        let index = FileIndex::open(&data_root)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        Ok(Self { blobs, index })
    }

    /// Map a collection D-Bus path to a credential namespace.
    pub fn coll_to_ns(coll_path: &str) -> String {
        coll_path.trim_start_matches("/org/freedesktop/secrets/collections/").into()
    }

    /// Compute a stable item key from label + attributes.
    pub fn item_key(label: &str, attrs: &[(String, String)]) -> String {
        use edgerun_json::json;
        let arr: Vec<JsonValue> = attrs.iter()
            .map(|(k, v)| json!([k.as_str(), v.as_str()]))
            .collect();
        let meta = json!({"l": label, "a": arr});
        let json = edgerun_json::to_string(&meta).unwrap_or_default();
        edgerun_core::util::bytes_to_hex(&edgerun_core::crypto::sha256(json.as_bytes()))
    }

    // -- CRUD --

    pub fn put(&self, coll: &str, key: &str, secret: &[u8], label: &str, attrs: &[(String, String)]) -> io::Result<()> {
        let ns = Self::coll_to_ns(coll);
        let meta = CredentialMeta {
            label: label.into(),
            attributes: attrs.iter().cloned().collect(),
            created_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_micros() as u64,
        };
        // Store secret + metadata as one encrypted blob
        let payload = meta.to_json().into_bytes();
        let blob_id = self.blobs.store(&payload, &[])
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        // Store the actual secret separately — blob_id = SHA256 of meta JSON, not secret
        // Actually we need the secret encrypted too. Store the raw secret as the blob.
        // Let me reconsider: blob is content-addressed by SHA256 of payload.
        // We need TWO blobs: one for the secret, one for metadata indexed by name.
        // Simpler: store {meta: {...}, secret_bytes: ay} as one blob.
        // But then the blob_id changes on every secret rotation.
        // Best: store the secret in the blob (content-addressed), metadata in the FileIndex description.
        // The FileIndex description field holds the JSON metadata.

        // Re-do: store the raw secret as the blob
        let secret_blob_id = self.blobs.store(secret, &[])
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
        // Index with JSON metadata in description
        self.index.put_credential(&ns, key, &secret_blob_id, Some(&meta.to_json()))
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
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

    pub fn delete(&self, coll: &str, key: &str) -> io::Result<bool> {
        let ns = Self::coll_to_ns(coll);
        self.index.delete_credential(&ns, key)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
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
        let be = Backend::new(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(coll, "test-key", b"super-secret", "Test Label", &[]).unwrap();
        let (secret, meta) = be.get(coll, "test-key").unwrap().unwrap();
        assert_eq!(secret, b"super-secret");
        assert_eq!(meta.label, "Test Label");
    }

    #[test]
    fn backend_delete() {
        let root = tmp_root();
        let be = Backend::new(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(coll, "del-key", b"secret", "Del Label", &[]).unwrap();
        assert!(be.delete(coll, "del-key").unwrap());
        assert!(!be.delete(coll, "del-key").unwrap());
        assert!(be.get(coll, "del-key").unwrap().is_none());
    }

    #[test]
    fn backend_list() {
        let root = tmp_root();
        let be = Backend::new(root).unwrap();
        let coll = "/org/freedesktop/secrets/collections/default";
        be.put(coll, "k1", b"v1", "Label 1", &[]).unwrap();
        be.put(coll, "k2", b"v2", "Label 2", &[]).unwrap();
        let items = be.list(coll).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn backend_search_by_attr() {
        let root = tmp_root();
        let be = Backend::new(root).unwrap();
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
    fn item_key_deterministic() {
        let a = Backend::item_key("label", &[("a".into(), "1".into())]);
        let b = Backend::item_key("label", &[("a".into(), "1".into())]);
        assert_eq!(a, b);
    }

    #[test]
    fn item_key_differs_on_label() {
        let a = Backend::item_key("label-a", &[]);
        let b = Backend::item_key("label-b", &[]);
        assert_ne!(a, b);
    }
}
