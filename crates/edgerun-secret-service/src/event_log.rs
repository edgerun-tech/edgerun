//! Append-only event log for secret operations.
//!
//! **DEPRECATED**: This module writes to a separate event log file.
//! The canonical approach is to record secret operations as signed events
//! in the node's main event stream via `SecretPutPayload` / `SecretDeletePayload`.
//! See `Backend::new()` which accepts a `SecretEventRecorder` callback.
//!
//! This module is kept for backward compatibility with existing deployments.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use edgerun_proto::edgerun::v0::stream::{
    SecretPutPayload, SecretDeletePayload,
    CollectionCreatedPayload, CollectionDeletedPayload,
};
use prost::Message;

// ===========================================================================
// Stream identity
// ===========================================================================

/// The stream ID used for all secret-service events.
pub const SECRET_STREAM_ID: &[u8] = b"secret-service";

// ===========================================================================
// Event types
// ===========================================================================

/// The type of a secret event, used for decoding.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SecretEventType {
    SecretPut,
    SecretDelete,
    CollectionCreated,
    CollectionDeleted,
}

impl SecretEventType {
    fn as_byte(&self) -> u8 {
        match self {
            Self::SecretPut => 0x01,
            Self::SecretDelete => 0x02,
            Self::CollectionCreated => 0x03,
            Self::CollectionDeleted => 0x04,
        }
    }

    fn from_byte(b: u8) -> Option<Self> {
        match b {
            0x01 => Some(Self::SecretPut),
            0x02 => Some(Self::SecretDelete),
            0x03 => Some(Self::CollectionCreated),
            0x04 => Some(Self::CollectionDeleted),
            _ => None,
        }
    }
}

/// A single event record in the log.
pub enum SecretEvent {
    Put(SecretPutPayload),
    Delete(SecretDeletePayload),
    CollectionCreated(CollectionCreatedPayload),
    CollectionDeleted(CollectionDeletedPayload),
}

impl SecretEvent {
    fn event_type(&self) -> SecretEventType {
        match self {
            Self::Put(_) => SecretEventType::SecretPut,
            Self::Delete(_) => SecretEventType::SecretDelete,
            Self::CollectionCreated(_) => SecretEventType::CollectionCreated,
            Self::CollectionDeleted(_) => SecretEventType::CollectionDeleted,
        }
    }

    fn payload_bytes(&self) -> Vec<u8> {
        match self {
            Self::Put(p) => p.encode_to_vec(),
            Self::Delete(p) => p.encode_to_vec(),
            Self::CollectionCreated(p) => p.encode_to_vec(),
            Self::CollectionDeleted(p) => p.encode_to_vec(),
        }
    }
}

// ===========================================================================
// Event log
// ===========================================================================

/// Append-only event log for secret operations.
///
/// Writes to `{data_root}/events/{stream_id_hex}.log` in the same wire format
/// as NodeStore: `[varint total_length][1-byte event_type][protobuf payload]`.
///
/// The event log is immutable. "Deletion" is recorded by appending a new
/// `SecretDelete` event — the history is never altered.
pub struct EventLog {
    file: File,
    path: PathBuf,
}

impl EventLog {
    /// Opens or creates the event log.
    pub fn open(data_root: &Path) -> io::Result<Self> {
        let stream_hex = edgerun_core::util::bytes_to_hex(SECRET_STREAM_ID);
        let events_dir = data_root.join("events");
        fs::create_dir_all(&events_dir)?;
        let path = events_dir.join(format!("{}.log", stream_hex));

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;

        Ok(Self { file, path })
    }

    /// Record a secret put (create or rotate).
    pub fn record_put(
        &mut self,
        namespace: &str,
        key: &str,
        label: &str,
        attributes: &[(String, String)],
        blob_id: &str,
    ) -> io::Result<()> {
        let payload = SecretPutPayload {
            payload_version: 1,
            namespace: namespace.into(),
            key: key.into(),
            label: label.into(),
            attributes: attributes.iter().cloned().collect(),
            secret_blob_id: blob_id.into(),
        };
        self.append(SecretEvent::Put(payload))
    }

    /// Record a secret deletion.
    pub fn record_delete(
        &mut self,
        namespace: &str,
        key: &str,
        label: &str,
        reason: Option<&str>,
    ) -> io::Result<()> {
        let payload = SecretDeletePayload {
            payload_version: 1,
            namespace: namespace.into(),
            key: key.into(),
            label: label.into(),
            reason: reason.unwrap_or("").into(),
        };
        self.append(SecretEvent::Delete(payload))
    }

    /// Record a collection creation.
    pub fn record_collection_created(
        &mut self,
        collection_name: &str,
        label: &str,
    ) -> io::Result<()> {
        let payload = CollectionCreatedPayload {
            payload_version: 1,
            collection_name: collection_name.into(),
            label: label.into(),
        };
        self.append(SecretEvent::CollectionCreated(payload))
    }

    /// Record a collection deletion.
    pub fn record_collection_deleted(
        &mut self,
        collection_name: &str,
        items_removed: u32,
    ) -> io::Result<()> {
        let payload = CollectionDeletedPayload {
            payload_version: 1,
            collection_name: collection_name.into(),
            items_removed,
        };
        self.append(SecretEvent::CollectionDeleted(payload))
    }

    // -----------------------------------------------------------------------
    // Internal
    // -----------------------------------------------------------------------

    fn append(&mut self, event: SecretEvent) -> io::Result<()> {
        let payload_bytes = event.payload_bytes();
        let total_len = 1 + payload_bytes.len(); // 1 byte type + payload
        let len_prefix = encode_varint(total_len as u64);

        self.file.write_all(&len_prefix)?;
        self.file.write_all(&[event.event_type().as_byte()])?;
        self.file.write_all(&payload_bytes)?;
        self.file.sync_all()?;
        Ok(())
    }

    /// Replays all events from the log.
    pub fn replay(&self) -> io::Result<Vec<SecretEvent>> {
        let mut file = File::open(&self.path)?;
        let mut events = Vec::new();
        while let Ok(Some(total_len)) = decode_varint_stream(&mut file) {
            let mut buf = vec![0u8; total_len as usize];
            let _ = file.read_exact(&mut buf);
            let event_type = SecretEventType::from_byte(buf[0]);
            let payload = &buf[1..];
            if let Some(et) = event_type {
                match et {
                    SecretEventType::SecretPut => {
                        if let Ok(p) = SecretPutPayload::decode(payload) {
                            events.push(SecretEvent::Put(p));
                        }
                    }
                    SecretEventType::SecretDelete => {
                        if let Ok(p) = SecretDeletePayload::decode(payload) {
                            events.push(SecretEvent::Delete(p));
                        }
                    }
                    SecretEventType::CollectionCreated => {
                        if let Ok(p) = CollectionCreatedPayload::decode(payload) {
                            events.push(SecretEvent::CollectionCreated(p));
                        }
                    }
                    SecretEventType::CollectionDeleted => {
                        if let Ok(p) = CollectionDeletedPayload::decode(payload) {
                            events.push(SecretEvent::CollectionDeleted(p));
                        }
                    }
                }
            }
        }
        Ok(events)
    }
}

// ===========================================================================
// ===========================================================================
// Helpers
// ===========================================================================

use edgerun_core::varint::{encode_varint, decode_varint_from_read as decode_varint_stream};

mod tests {
    use super::*;

    fn tmp_root() -> PathBuf {
        static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("evlog_test_{}_{}", std::process::id(), n));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn record_put_and_replay() {
        let root = tmp_root();
        let mut log = EventLog::open(&root).unwrap();
        log.record_put("default", "k1", "Key 1", &[], "blob123").unwrap();
        log.record_put("default", "k2", "Key 2", &[("server".into(), "github.com".into())], "blob456").unwrap();
        log.record_delete("default", "k1", "Key 1", Some("no longer needed")).unwrap();

        let events = log.replay().unwrap();
        assert_eq!(events.len(), 3);
        assert!(matches!(&events[0], SecretEvent::Put(_)));
        assert!(matches!(&events[1], SecretEvent::Put(_)));
        assert!(matches!(&events[2], SecretEvent::Delete(_)));

        // Decode payload
        if let SecretEvent::Put(p) = &events[0] {
            assert_eq!(p.namespace, "default");
            assert_eq!(p.key, "k1");
            assert_eq!(p.label, "Key 1");
            assert_eq!(p.secret_blob_id, "blob123");
        } else { panic!("expected Put"); }
    }

    #[test]
    fn seq_resumes_after_reopen() {
        let root = tmp_root();
        {
            let mut log = EventLog::open(&root).unwrap();
            log.record_put("default", "k1", "K1", &[], "b1").unwrap();
        }
        {
            let mut log = EventLog::open(&root).unwrap();
            log.record_put("default", "k2", "K2", &[], "b2").unwrap();
        }
        let log = EventLog::open(&root).unwrap();
        let events = log.replay().unwrap();
        assert_eq!(events.len(), 2);
    }

    #[test]
    fn collection_events() {
        let root = tmp_root();
        let mut log = EventLog::open(&root).unwrap();
        log.record_collection_created("default", "Default").unwrap();
        log.record_collection_deleted("default", 3).unwrap();

        let events = log.replay().unwrap();
        assert_eq!(events.len(), 2);
        assert!(matches!(&events[0], SecretEvent::CollectionCreated(_)));
        assert!(matches!(&events[1], SecretEvent::CollectionDeleted(_)));

        if let SecretEvent::CollectionDeleted(p) = &events[1] {
            assert_eq!(p.items_removed, 3);
        } else { panic!("expected CollectionDeleted"); }
    }

    #[test]
    fn event_log_is_append_only() {
        let root = tmp_root();
        let mut log = EventLog::open(&root).unwrap();
        log.record_put("default", "k1", "K1", &[], "b1").unwrap();
        log.record_delete("default", "k1", "K1", None).unwrap();

        // Replay always shows both events — deletion is an append, not a removal
        let events1 = log.replay().unwrap();
        assert_eq!(events1.len(), 2);

        // Reopen and replay again — same result
        let log2 = EventLog::open(&root).unwrap();
        let events2 = log2.replay().unwrap();
        assert_eq!(events2.len(), 2);
    }

    #[test]
    fn varint_roundtrip() {
        for &v in &[0u64, 1, 127, 128, 255, 256, 16383, 16384, 1_000_000, u64::MAX] {
            let encoded = encode_varint(v);
            let mut cursor = io::Cursor::new(encoded.clone());
            let decoded = decode_varint_stream(&mut cursor).unwrap().unwrap();
            assert_eq!(v, decoded);
        }
    }

    #[test]
    fn event_log_attributes_roundtrip() {
        let root = tmp_root();
        let mut log = EventLog::open(&root).unwrap();
        log.record_put("default", "k1", "Key 1", &[
            ("server".into(), "github.com".into()),
            ("type".into(), "password".into()),
        ], "blob123").unwrap();
        let events = log.replay().unwrap();
        assert_eq!(events.len(), 1);
        if let SecretEvent::Put(p) = &events[0] {
            assert_eq!(p.attributes.len(), 2);
            assert_eq!(p.attributes["server"], "github.com");
        } else { panic!("expected Put"); }
    }

    #[test]
    fn event_log_secret_type_byte_values() {
        assert_eq!(SecretEventType::SecretPut.as_byte(), 0x01);
        assert_eq!(SecretEventType::SecretDelete.as_byte(), 0x02);
        assert_eq!(SecretEventType::CollectionCreated.as_byte(), 0x03);
        assert_eq!(SecretEventType::CollectionDeleted.as_byte(), 0x04);
        assert!(SecretEventType::from_byte(0x00).is_none());
        assert!(SecretEventType::from_byte(0x05).is_none());
        assert!(SecretEventType::from_byte(0xFF).is_none());
    }

    #[test]
    fn event_log_record_delete_with_reason() {
        let root = tmp_root();
        let mut log = EventLog::open(&root).unwrap();
        log.record_delete("default", "k1", "Key 1", Some("user requested")).unwrap();
        let events = log.replay().unwrap();
        if let SecretEvent::Delete(p) = &events[0] {
            assert_eq!(p.reason, "user requested");
        } else { panic!("expected Delete"); }
    }

    #[test]
    fn event_log_record_delete_without_reason() {
        let root = tmp_root();
        let mut log = EventLog::open(&root).unwrap();
        log.record_delete("default", "k1", "Key 1", None).unwrap();
        let events = log.replay().unwrap();
        if let SecretEvent::Delete(p) = &events[0] {
            assert_eq!(p.reason, "");
        } else { panic!("expected Delete"); }
    }

    #[test]
    fn event_log_mixed_operations() {
        let root = tmp_root();
        let mut log = EventLog::open(&root).unwrap();
        log.record_collection_created("default", "Default").unwrap();
        log.record_put("default", "k1", "K1", &[("s".into(), "v".into())], "b1").unwrap();
        log.record_delete("default", "k1", "K1", None).unwrap();
        log.record_collection_deleted("default", 0).unwrap();
        let events = log.replay().unwrap();
        assert_eq!(events.len(), 4);
        assert!(matches!(&events[0], SecretEvent::CollectionCreated(_)));
        assert!(matches!(&events[1], SecretEvent::Put(_)));
        assert!(matches!(&events[2], SecretEvent::Delete(_)));
        assert!(matches!(&events[3], SecretEvent::CollectionDeleted(_)));
    }
}
