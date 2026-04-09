//! Append-only event log for credential operations.
//!
//! Records every secret creation, update, and deletion as a length-prefixed
//! JSON record. The log is authoritative — the FileIndex can be rebuilt
//! from it.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use edgerun_json::{from_str, to_string, JsonValue};

// ===========================================================================
// Event record
// ===========================================================================

#[derive(Clone, Debug)]
pub enum SecretOp {
    Put { label: String, attributes: HashMap<String, String> },
    Delete,
    CreateCollection,
    DeleteCollection,
}

#[derive(Clone, Debug)]
pub struct SecretEvent {
    pub ts_us: u64,
    pub ns: String,
    pub key: String,
    pub op: SecretOp,
}

impl SecretEvent {
    pub fn to_json(&self) -> String {
        let (op, label, attrs) = match &self.op {
            SecretOp::Put { label, attributes } => ("put", label, attributes),
            SecretOp::Delete => ("delete", &String::new(), &HashMap::new()),
            SecretOp::CreateCollection => ("create_collection", &String::new(), &HashMap::new()),
            SecretOp::DeleteCollection => ("delete_collection", &String::new(), &HashMap::new()),
        };

        let mut attrs_json = JsonValue::Object(edgerun_json::Map::new());
        for (k, v) in attrs {
            attrs_json.insert(k.clone(), JsonValue::String(v.clone()));
        }

        let root = edgerun_json::json!({
            "op": op,
            "ns": self.ns.clone(),
            "key": self.key.clone(),
            "ts_us": self.ts_us,
            "label": label.clone(),
            "attributes": attrs_json,
        });
        to_string(&root).unwrap_or_default()
    }

    pub fn from_json(s: &str) -> Option<Self> {
        let v: JsonValue = from_str(s).ok()?;
        let op_str = v.get("op").and_then(JsonValue::as_str).unwrap_or("");
        let ns = v.get("ns").and_then(JsonValue::as_str).unwrap_or("").to_string();
        let key = v.get("key").and_then(JsonValue::as_str).unwrap_or("").to_string();
        let ts_us = v.get("ts_us").and_then(JsonValue::as_u64).unwrap_or(0);
        let label = v.get("label").and_then(JsonValue::as_str).unwrap_or("").to_string();

        let mut attrs = HashMap::new();
        if let Some(attrs_obj) = v.get("attributes").and_then(JsonValue::as_object) {
            for (k, val) in attrs_obj.iter() {
                if let Some(vs) = val.as_str() {
                    attrs.insert(k.clone(), vs.to_string());
                }
            }
        }

        let op = match op_str {
            "put" => SecretOp::Put { label, attributes: attrs },
            "delete" => SecretOp::Delete,
            "create_collection" => SecretOp::CreateCollection,
            "delete_collection" => SecretOp::DeleteCollection,
            _ => return None,
        };

        Some(Self { ts_us, ns, key, op })
    }
}

// ===========================================================================
// Append-only log
// ===========================================================================

/// Append-only event log for secret operations.
///
/// Format: `[varint length][JSON payload]` per record.
pub struct EventLog {
    file: File,
}

impl EventLog {
    /// Opens or creates the event log file.
    pub fn open(data_root: &Path) -> io::Result<Self> {
        let path = data_root.join("secret_events.log");
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;
        Ok(Self { file })
    }

    /// Appends an event to the log.
    pub fn append(&mut self, event: &SecretEvent) -> io::Result<()> {
        let json = event.to_json();
        let bytes = json.as_bytes();
        let len_prefix = encode_varint(bytes.len() as u64);
        self.file.write_all(&len_prefix)?;
        self.file.write_all(bytes)?;
        self.file.sync_all()?;
        Ok(())
    }

    /// Replays all events from the log.
    pub fn replay(&self) -> io::Result<Vec<SecretEvent>> {
        let mut file = File::open(self.file.try_clone().unwrap().try_clone().unwrap())?;
        let mut events = Vec::new();
        loop {
            match decode_varint_stream(&mut file) {
                Ok(Some(len)) => {
                    let mut buf = vec![0u8; len as usize];
                    match file.read_exact(&mut buf) {
                        Ok(()) => {
                            let json = String::from_utf8_lossy(&buf);
                            if let Some(event) = SecretEvent::from_json(&json) {
                                events.push(event);
                            }
                        }
                        Err(_) => break,
                    }
                }
                Ok(None) | Err(_) => break,
            }
        }
        Ok(events)
    }
}

// ===========================================================================
// Varint encoding (protobuf style)
// ===========================================================================

fn encode_varint(mut v: u64) -> Vec<u8> {
    let mut out = Vec::with_capacity(8);
    loop {
        let mut byte = (v & 0x7F) as u8;
        v >>= 7;
        if v != 0 {
            byte |= 0x80;
        }
        out.push(byte);
        if v == 0 {
            break;
        }
    }
    out
}

fn decode_varint_stream<R: Read>(r: &mut R) -> io::Result<Option<u64>> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        let mut byte = [0u8; 1];
        match r.read_exact(&mut byte) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => {
                if shift == 0 { return Ok(None); }
                return Err(e);
            }
            Err(e) => return Err(e),
        }
        result |= ((byte[0] & 0x7F) as u64) << shift;
        shift += 7;
        if byte[0] & 0x80 == 0 {
            break;
        }
        if shift >= 64 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "varint too long"));
        }
    }
    Ok(Some(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_log() -> (PathBuf, EventLog) {
        static C: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = C.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let p = std::env::temp_dir().join(format!("evlog_test_{}_{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        let log = EventLog::open(&p).unwrap();
        (p, log)
    }

    #[test]
    fn event_roundtrip() {
        let (_path, _log) = tmp_log();
        let event = SecretEvent {
            ts_us: 12345,
            ns: "default".into(),
            key: "test-key".into(),
            op: SecretOp::Put {
                label: "Test".into(),
                attributes: [("server".into(), "github.com".into())].into(),
            },
        };
        let json = event.to_json();
        let back = SecretEvent::from_json(&json).unwrap();
        assert_eq!(back.ns, "default");
        assert_eq!(back.key, "test-key");
        assert_eq!(back.ts_us, 12345);
        if let SecretOp::Put { label, attributes } = back.op {
            assert_eq!(label, "Test");
            assert_eq!(attributes["server"], "github.com");
        } else { panic!("wrong op"); }
    }

    #[test]
    fn append_and_replay() {
        let (path, mut log) = tmp_log();
        log.append(&SecretEvent {
            ts_us: 100, ns: "default".into(), key: "k1".into(),
            op: SecretOp::Put { label: "Key 1".into(), attributes: HashMap::new() },
        }).unwrap();
        log.append(&SecretEvent {
            ts_us: 200, ns: "default".into(), key: "k2".into(),
            op: SecretOp::Put { label: "Key 2".into(), attributes: HashMap::new() },
        }).unwrap();
        log.append(&SecretEvent {
            ts_us: 300, ns: "default".into(), key: "k1".into(),
            op: SecretOp::Delete,
        }).unwrap();

        // Reopen and replay
        let log2 = EventLog::open(&path).unwrap();
        let events = log2.replay().unwrap();
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].key, "k1");
        assert_eq!(events[1].key, "k2");
        assert_eq!(events[2].op, SecretOp::Delete);
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
}
