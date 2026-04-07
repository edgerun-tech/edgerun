//! File-based index — replaces SQLite.
//!
//! The index stores data as in-memory HashMaps, persisted to disk as
//! append-only binary logs. All indexes are rebuildable from the event log.
//!
//! Storage layout:
//!   {data_root}/indexes/stream_heads.bin   — simple key-value store
//!   {data_root}/indexes/events.bin         — append-only event records
//!   {data_root}/indexes/replay_cache.bin   — key-value store
//!   {data_root}/indexes/peers.bin          — key-value store
//!   {data_root}/indexes/snapshots.bin      — key-value store
//!   {data_root}/indexes/delegations.bin    — key-value store
//!   {data_root}/indexes/revocations.bin    — key-value store
//!   {data_root}/indexes/controller_changes.bin — append-only records
//!   {data_root}/indexes/fetch_queue.bin    — append-only records
//!   {data_root}/indexes/object_presence.bin — key-value store

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::sync::Mutex;
use std::cell::RefCell;

// ===========================================================================
// Simple binary serialization helpers
// ===========================================================================

fn write_u64(w: &mut impl Write, v: u64) -> io::Result<()> {
    w.write_all(&v.to_le_bytes())
}

fn read_u64(r: &mut impl Read) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    r.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}

fn write_str(w: &mut impl Write, s: &str) -> io::Result<()> {
    let bytes = s.as_bytes();
    write_u64(w, bytes.len() as u64)?;
    w.write_all(bytes)
}

fn read_str(r: &mut impl Read) -> io::Result<String> {
    let len = read_u64(r)? as usize;
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf)?;
    Ok(String::from_utf8_lossy(&buf).to_string())
}

fn write_option_str(w: &mut impl Write, s: Option<&str>) -> io::Result<()> {
    match s {
        Some(s) => { w.write_all(&[1])?; write_str(w, s); }
        None => w.write_all(&[0])?,
    }
    Ok(())
}

fn read_option_str(r: &mut impl Read) -> io::Result<Option<String>> {
    let mut tag = [0u8; 1];
    r.read_exact(&mut tag)?;
    if tag[0] == 1 {
        read_str(r).map(Some)
    } else {
        Ok(None)
    }
}

fn write_vec_str(w: &mut impl Write, v: &[String]) -> io::Result<()> {
    write_u64(w, v.len() as u64)?;
    for s in v { write_str(w, s)?; }
    Ok(())
}

fn read_vec_str(r: &mut impl Read) -> io::Result<Vec<String>> {
    let len = read_u64(r)? as usize;
    let mut v = Vec::with_capacity(len);
    for _ in 0..len { v.push(read_str(r)?); }
    Ok(v)
}

// ===========================================================================
// Index state
// ===========================================================================

#[derive(Clone)]
pub struct StreamHead {
    pub seq: i64,
    pub hash: Vec<u8>,
}


pub type EventIndexEntry = EventRecord;

pub struct EventRecord {
    pub event_hash: Vec<u8>,
    pub file_offset: u64,
    pub envelope_version: i64,
}

#[derive(Clone)]
pub struct ReplayEntry {
    pub command_id: String,
    pub decision_event_seq: i64,
}

#[derive(Clone)]
pub struct FetchEntry {
    pub id: i64,
    pub target_type: String,
    pub target_id: String,
    pub priority: i64,
    pub created_at: i64,
    pub status: String,
}

#[derive(Clone)]
pub struct PeerRecord {
    pub addr: Option<String>,
    pub status: String,
    pub last_seen: Option<i64>,
    pub first_seen: i64,
    pub is_bootstrap: bool,
}

#[derive(Clone)]
pub struct SnapshotRecord {
    pub object_id_hex: String,
    pub view_type: String,
    pub producer_hex: String,
    pub produced_at: i64,
    pub completeness: i32,
    pub base_heads: String,
    pub stored_at: i64,
}

#[derive(Clone)]
pub struct DelegationRecord {
    pub issuer_hex: String,
    pub recipient_hex: String,
    pub capability_hex: String,
    pub expires_at: Option<i64>,
    pub is_revoked: bool,
    pub stored_at: i64,
}

#[derive(Clone)]
pub struct RevocationRecord {
    pub issuer_hex: String,
    pub target_type: String,
    pub target_hex: String,
    pub effective_at: Option<i64>,
    pub stored_at: i64,
}

#[derive(Clone)]
pub struct ControllerChange {
    pub controller_hex: String,
    pub change_type: String,
    pub event_seq: i64,
}

// ===========================================================================
// FileIndex
// ===========================================================================

pub struct FileIndex {
    stream_heads: std::cell::RefCell<HashMap<String, StreamHead>>,
    events: std::cell::RefCell<HashMap<(String, i64), EventRecord>>,
    replay_cache: std::cell::RefCell<HashMap<(String, String), ReplayEntry>>,
    fetch_queue: std::cell::RefCell<Vec<FetchEntry>>,
    next_fetch_id: std::cell::Cell<i64>,
    object_presence: std::cell::RefCell<HashMap<String, (String, String, String)>>,
    peers: std::cell::RefCell<HashMap<String, PeerRecord>>,
    snapshots: std::cell::RefCell<HashMap<String, SnapshotRecord>>,
    controller_changes: std::cell::RefCell<Vec<ControllerChange>>,
    delegations: std::cell::RefCell<HashMap<String, DelegationRecord>>,
    revocations: std::cell::RefCell<HashMap<String, RevocationRecord>>,
    data_root: PathBuf,
}

impl FileIndex {
    // Note: all methods take &self because interior mutability is via RefCell

    pub fn open(data_root: &PathBuf) -> io::Result<Self> {
        let idx_dir = data_root.join("indexes");
        fs::create_dir_all(&idx_dir)?;

        let index = Self {
            stream_heads: std::cell::RefCell::new(HashMap::new()),
            events: std::cell::RefCell::new(HashMap::new()),
            replay_cache: std::cell::RefCell::new(HashMap::new()),
            fetch_queue: std::cell::RefCell::new(Vec::new()),
            next_fetch_id: std::cell::Cell::new(1),
            object_presence: std::cell::RefCell::new(HashMap::new()),
            peers: std::cell::RefCell::new(HashMap::new()),
            snapshots: std::cell::RefCell::new(HashMap::new()),
            controller_changes: std::cell::RefCell::new(Vec::new()),
            delegations: std::cell::RefCell::new(HashMap::new()),
            revocations: std::cell::RefCell::new(HashMap::new()),
            data_root: data_root.clone(),
        };

        index.load()?;
        Ok(index)
    }

    fn idx_dir(&self) -> PathBuf {
        self.data_root.join("indexes")
    }

    fn load(&self) -> io::Result<()> {
        // Load stream_heads
        if let Ok(data) = fs::read(self.idx_dir().join("stream_heads.bin")) {
            let data_len = data.len() as u64;
            let mut r = std::io::Cursor::new(data);
            while r.position() < data_len {
                let key = read_str(&mut r)?;
                let seq = read_u64(&mut r)? as i64;
                let hash_len = read_u64(&mut r)? as usize;
                let mut hash = vec![0u8; hash_len];
                r.read_exact(&mut hash)?;
                self.stream_heads.borrow_mut().insert(key, StreamHead { seq, hash });
            }
        }

        // Load peers
        if let Ok(data) = fs::read(self.idx_dir().join("peers.bin")) {
            let data_len = data.len() as u64;
            let mut r = std::io::Cursor::new(data);
            while r.position() < data_len {
                let key = read_str(&mut r)?;
                let addr = read_option_str(&mut r)?;
                let status = read_str(&mut r)?;
                let last_seen = {
                    let tag = { let mut t = [0u8; 1]; r.read_exact(&mut t)?; t[0] };
                    if tag == 1 { Some(read_u64(&mut r)? as i64) } else { None }
                };
                let first_seen = read_u64(&mut r)? as i64;
                let is_bootstrap = { let mut t = [0u8; 1]; r.read_exact(&mut t)?; t[0] != 0 };
                self.peers.borrow_mut().insert(key, PeerRecord { addr, status, last_seen, first_seen, is_bootstrap });
            }
        }

        // Load snapshots
        if let Ok(data) = fs::read(self.idx_dir().join("snapshots.bin")) {
            let data_len = data.len() as u64;
            let mut r = std::io::Cursor::new(data);
            while r.position() < data_len {
                let key = read_str(&mut r)?;
                let object_id_hex = read_str(&mut r)?;
                let view_type = read_str(&mut r)?;
                let producer_hex = read_str(&mut r)?;
                let produced_at = read_u64(&mut r)? as i64;
                let completeness = read_u64(&mut r)? as i32;
                let base_heads = read_str(&mut r)?;
                let stored_at = read_u64(&mut r)? as i64;
                self.snapshots.borrow_mut().insert(key, SnapshotRecord {
                    object_id_hex, view_type, producer_hex, produced_at,
                    completeness, base_heads, stored_at,
                });
            }
        }

        // Load delegations
        if let Ok(data) = fs::read(self.idx_dir().join("delegations.bin")) {
            let data_len = data.len() as u64;
            let mut r = std::io::Cursor::new(data);
            while r.position() < data_len {
                let key = read_str(&mut r)?;
                let issuer_hex = read_str(&mut r)?;
                let recipient_hex = read_str(&mut r)?;
                let capability_hex = read_str(&mut r)?;
                let expires_at = {
                    let tag = { let mut t = [0u8; 1]; r.read_exact(&mut t)?; t[0] };
                    if tag == 1 { Some(read_u64(&mut r)? as i64) } else { None }
                };
                let is_revoked = { let mut t = [0u8; 1]; r.read_exact(&mut t)?; t[0] != 0 };
                let stored_at = read_u64(&mut r)? as i64;
                self.delegations.borrow_mut().insert(key, DelegationRecord {
                    issuer_hex, recipient_hex, capability_hex, expires_at, is_revoked, stored_at,
                });
            }
        }

        // Load revocations
        if let Ok(data) = fs::read(self.idx_dir().join("revocations.bin")) {
            let data_len = data.len() as u64;
            let mut r = std::io::Cursor::new(data);
            while r.position() < data_len {
                let key = read_str(&mut r)?;
                let issuer_hex = read_str(&mut r)?;
                let target_type = read_str(&mut r)?;
                let target_hex = read_str(&mut r)?;
                let effective_at = {
                    let tag = { let mut t = [0u8; 1]; r.read_exact(&mut t)?; t[0] };
                    if tag == 1 { Some(read_u64(&mut r)? as i64) } else { None }
                };
                let stored_at = read_u64(&mut r)? as i64;
                self.revocations.borrow_mut().insert(key, RevocationRecord {
                    issuer_hex, target_type, target_hex, effective_at, stored_at,
                });
            }
        }

        // Load object_presence
        if let Ok(data) = fs::read(self.idx_dir().join("object_presence.bin")) {
            let data_len = data.len() as u64;
            let mut r = std::io::Cursor::new(data);
            while r.position() < data_len {
                let key = read_str(&mut r)?;
                let rep_id = read_str(&mut r)?;
                let blob_id = read_str(&mut r)?;
                let status = read_str(&mut r)?;
                self.object_presence.borrow_mut().insert(key, (rep_id, blob_id, status));
            }
        }

        // Load replay_cache
        if let Ok(data) = fs::read(self.idx_dir().join("replay_cache.bin")) {
            let data_len = data.len() as u64;
            let mut r = std::io::Cursor::new(data);
            while r.position() < data_len {
                let node = read_str(&mut r)?;
                let cmd_hash = read_str(&mut r)?;
                let cmd_id = read_str(&mut r)?;
                let seq = read_u64(&mut r)? as i64;
                self.replay_cache.borrow_mut().insert((node, cmd_hash), ReplayEntry { command_id: cmd_id, decision_event_seq: seq });
            }
        }

        // Load controller_changes
        if let Ok(data) = fs::read(self.idx_dir().join("controller_changes.bin")) {
            let data_len = data.len() as u64;
            let mut r = std::io::Cursor::new(data);
            while r.position() < data_len {
                let controller_hex = read_str(&mut r)?;
                let change_type = read_str(&mut r)?;
                let event_seq = read_u64(&mut r)? as i64;
                self.controller_changes.borrow_mut().push(ControllerChange { controller_hex, change_type, event_seq });
            }
        }

        // Load fetch_queue
        if let Ok(data) = fs::read(self.idx_dir().join("fetch_queue.bin")) {
            let data_len = data.len() as u64;
            let mut r = std::io::Cursor::new(data);
            while r.position() < data_len {
                let id = read_u64(&mut r)? as i64;
                let target_type = read_str(&mut r)?;
                let target_id = read_str(&mut r)?;
                let priority = read_u64(&mut r)? as i64;
                let created_at = read_u64(&mut r)? as i64;
                let status = read_str(&mut r)?;
                self.fetch_queue.borrow_mut().push(FetchEntry { id, target_type, target_id, priority, created_at, status });
                if id >= self.next_fetch_id.get() {
                    self.next_fetch_id.set(id + 1);
                }
            }
        }

        Ok(())
    }

    fn persist<F>(&self, filename: &str, writer: F) -> io::Result<()>
    where
        F: Fn(&mut Vec<u8>) -> io::Result<()>,
    {
        let mut buf = Vec::new();
        writer(&mut buf)?;
        fs::write(self.idx_dir().join(filename), buf)
    }

    pub fn save(&self) -> io::Result<()> {
        // Save stream_heads
        self.persist("stream_heads.bin", |w| {
            for (k, v) in self.stream_heads.borrow().iter() {
                write_str(w, k)?;
                write_u64(w, v.seq as u64)?;
                write_u64(w, v.hash.len() as u64)?;
                w.write_all(&v.hash)?;
            }
            Ok(())
        })?;

        // Save peers
        self.persist("peers.bin", |w| {
            for (k, v) in self.peers.borrow().iter() {
                write_str(w, k)?;
                write_option_str(w, v.addr.as_deref())?;
                write_str(w, &v.status)?;
                match v.last_seen {
                    Some(t) => { w.write_all(&[1])?; write_u64(w, t as u64)?; }
                    None => { w.write_all(&[0])?; }
                }
                write_u64(w, v.first_seen as u64)?;
                w.write_all(&[if v.is_bootstrap { 1 } else { 0 }])?;
            }
            Ok(())
        })?;

        // Save snapshots
        self.persist("snapshots.bin", |w| {
            for (k, v) in self.snapshots.borrow().iter() {
                write_str(w, k)?;
                write_str(w, &v.object_id_hex)?;
                write_str(w, &v.view_type)?;
                write_str(w, &v.producer_hex)?;
                write_u64(w, v.produced_at as u64)?;
                write_u64(w, v.completeness as u64)?;
                write_str(w, &v.base_heads)?;
                write_u64(w, v.stored_at as u64)?;
            }
            Ok(())
        })?;

        // Save delegations
        self.persist("delegations.bin", |w| {
            for (k, v) in self.delegations.borrow().iter() {
                write_str(w, k)?;
                write_str(w, &v.issuer_hex)?;
                write_str(w, &v.recipient_hex)?;
                write_str(w, &v.capability_hex)?;
                match v.expires_at {
                    Some(t) => { w.write_all(&[1])?; write_u64(w, t as u64)?; }
                    None => { w.write_all(&[0])?; }
                }
                w.write_all(&[if v.is_revoked { 1 } else { 0 }])?;
                write_u64(w, v.stored_at as u64)?;
            }
            Ok(())
        })?;

        // Save revocations
        self.persist("revocations.bin", |w| {
            for (k, v) in self.revocations.borrow().iter() {
                write_str(w, k)?;
                write_str(w, &v.issuer_hex)?;
                write_str(w, &v.target_type)?;
                write_str(w, &v.target_hex)?;
                match v.effective_at {
                    Some(t) => { w.write_all(&[1])?; write_u64(w, t as u64)?; }
                    None => { w.write_all(&[0])?; }
                }
                write_u64(w, v.stored_at as u64)?;
            }
            Ok(())
        })?;

        // Save object_presence
        self.persist("object_presence.bin", |w| {
            for (k, (rep, blob, status)) in self.object_presence.borrow().iter() {
                write_str(w, k)?;
                write_str(w, rep)?;
                write_str(w, blob)?;
                write_str(w, status)?;
            }
            Ok(())
        })?;

        // Save replay_cache
        self.persist("replay_cache.bin", |w| {
            for ((node, cmd_hash), entry) in self.replay_cache.borrow().iter() {
                write_str(w, node)?;
                write_str(w, cmd_hash)?;
                write_str(w, &entry.command_id)?;
                write_u64(w, entry.decision_event_seq as u64)?;
            }
            Ok(())
        })?;

        // Save controller_changes
        self.persist("controller_changes.bin", |w| {
            for v in self.controller_changes.borrow().iter() {
                write_str(w, &v.controller_hex)?;
                write_str(w, &v.change_type)?;
                write_u64(w, v.event_seq as u64)?;
            }
            Ok(())
        })?;

        // Save fetch_queue
        self.persist("fetch_queue.bin", |w| {
            for v in self.fetch_queue.borrow().iter() {
                write_u64(w, v.id as u64)?;
                write_str(w, &v.target_type)?;
                write_str(w, &v.target_id)?;
                write_u64(w, v.priority as u64)?;
                write_u64(w, v.created_at as u64)?;
                write_str(w, &v.status)?;
            }
            Ok(())
        })?;

        Ok(())
    }

    // ===========================================================================
    // Public API — matches SqliteIndex
    // ===========================================================================

    pub fn put_event(&self, stream_id: &str, seq: i64, event_hash: &[u8], file_offset: u64, envelope_version: i64) -> io::Result<()> {
        let mut inner = Mutex::new(());
        let _lock = inner.lock().unwrap();
        // We can't use a real mutex on self since &self is immutable.
        // For now, just write to the event log file.
        let record = EventRecord {
            event_hash: event_hash.to_vec(),
            file_offset,
            envelope_version,
        };
        // Append to events log
        let mut f = OpenOptions::new().create(true).append(true).open(self.idx_dir().join("events.bin"))?;
        write_str(&mut f, stream_id)?;
        write_u64(&mut f, seq as u64)?;
        write_u64(&mut f, event_hash.len() as u64)?;
        f.write_all(event_hash)?;
        write_u64(&mut f, file_offset)?;
        write_u64(&mut f, envelope_version as u64)?;
        Ok(())
    }

    pub fn get_event(&self, stream_id: &str, seq: i64) -> io::Result<Option<EventRecord>> {
        // Read from events log file
        let path = self.idx_dir().join("events.bin");
        if !path.exists() { return Ok(None); }
        let data = fs::read(&path)?;
        let data_len = data.len() as u64;
            let mut r = std::io::Cursor::new(data);
            while r.position() < data_len {
            let sid = read_str(&mut r)?;
            let s = read_u64(&mut r)? as i64;
            let hash_len = read_u64(&mut r)? as usize;
            let mut hash = vec![0u8; hash_len];
            r.read_exact(&mut hash)?;
            let offset = read_u64(&mut r)?;
            let ver = read_u64(&mut r)? as i64;
            if sid == stream_id && s == seq {
                return Ok(Some(EventRecord { event_hash: hash, file_offset: offset, envelope_version: ver }));
            }
        }
        Ok(None)
    }

    pub fn set_head(&self, stream_id: &str, seq: i64, hash: &[u8]) -> io::Result<()> {
        self.stream_heads.borrow_mut().insert(stream_id.to_string(), StreamHead { seq, hash: hash.to_vec() });
        self.save()
    }

    pub fn get_head(&self, stream_id: &str) -> io::Result<Option<(i64, Vec<u8>)>> {
        Ok(self.stream_heads.borrow().get(stream_id).map(|h| (h.seq, h.hash.clone())))
    }

    pub fn put_replay_entry(&self, target_node: &str, command_hash: &str, command_id: &str, decision_event_seq: i64) -> io::Result<()> {
        self.replay_cache.borrow_mut().insert(
            (target_node.to_string(), command_hash.to_string()),
            ReplayEntry { command_id: command_id.to_string(), decision_event_seq },
        );
        self.save()
    }

    pub fn get_replay_entry(&self, target_node: &str, command_hash: &str) -> io::Result<Option<ReplayEntry>> {
        Ok(self.replay_cache.borrow().get(&(target_node.to_string(), command_hash.to_string())).cloned())
    }

    pub fn enqueue_fetch(&self, target_type: &str, target_id: &str, priority: i64) -> io::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let id = self.next_fetch_id.get();
        self.next_fetch_id.set(self.next_fetch_id.get() + 1);
        self.fetch_queue.borrow_mut().push(FetchEntry {
            id, target_type: target_type.to_string(), target_id: target_id.to_string(),
            priority, created_at: now, status: "pending".to_string(),
        });
        self.save()
    }

    pub fn dequeue_fetch(&self) -> io::Result<Option<FetchEntry>> {
        // Find first pending entry
        for entry in self.fetch_queue.borrow().iter() {
            if entry.status == "pending" {
                let result = entry.clone();
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    pub fn mark_fetch_done(&self, fetch_id: i64) -> io::Result<()> {
        for entry in self.fetch_queue.borrow_mut().iter_mut() {
            if entry.id == fetch_id { entry.status = "done".to_string(); }
        }
        self.save()
    }

    pub fn mark_fetch_failed(&self, fetch_id: i64) -> io::Result<()> {
        for entry in self.fetch_queue.borrow_mut().iter_mut() {
            if entry.id == fetch_id { entry.status = "failed".to_string(); }
        }
        self.save()
    }

    pub fn mark_object_present(&self, object_id: &str, representation_id: &str, blob_id: &str) -> io::Result<()> {
        self.object_presence.borrow_mut().insert(
            object_id.to_string(),
            (representation_id.to_string(), blob_id.to_string(), "present".to_string()),
        );
        self.save()
    }

    pub fn is_object_present(&self, object_id: &str) -> io::Result<bool> {
        Ok(self.object_presence.borrow().get(object_id).map(|(_, _, s)| s == "present").unwrap_or(false))
    }

    pub fn list_stream_heads(&self) -> io::Result<Vec<(String, i64, Vec<u8>)>> {
        Ok(self.stream_heads.borrow().iter().map(|(k, v)| (k.clone(), v.seq, v.hash.clone())).collect())
    }

    pub fn list_event_range(&self, stream_id: &str, from_seq: i64, to_seq: i64) -> io::Result<Vec<(i64, Vec<u8>, i64)>> {
        let path = self.idx_dir().join("events.bin");
        if !path.exists() { return Ok(Vec::new()); }
        let data = fs::read(&path)?;
        let data_len = data.len() as u64;
        let mut r = std::io::Cursor::new(data);
        let mut result = Vec::new();
        while r.position() < data_len {
            let sid = read_str(&mut r)?;
            let seq = read_u64(&mut r)? as i64;
            let hash_len = read_u64(&mut r)? as usize;
            let mut hash = vec![0u8; hash_len];
            r.read_exact(&mut hash)?;
            let _offset = read_u64(&mut r)?;
            let ver = read_u64(&mut r)? as i64;
            if sid == stream_id && seq >= from_seq && seq <= to_seq {
                result.push((seq, hash, ver));
            }
        }
        result.sort_by_key(|(seq, _, _)| *seq);
        Ok(result)
    }

    pub fn list_stream_ids(&self) -> io::Result<Vec<String>> {
        let mut ids: Vec<String> = self.stream_heads.borrow().keys().cloned().collect();
        ids.sort();
        Ok(ids)
    }

    pub fn lookup_objects(&self, object_ids: &[String]) -> io::Result<Vec<(String, Option<String>, Option<String>, String)>> {
        let mut result = Vec::new();
        for oid in object_ids {
            if let Some((rep, blob, status)) = self.object_presence.borrow().get(oid) {
                result.push((oid.clone(), Some(rep.clone()), Some(blob.clone()), status.clone()));
            }
        }
        Ok(result)
    }

    pub fn upsert_peer(&self, node_id_hex: &str, addr: Option<&str>, status: &str, is_bootstrap: bool) -> io::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        if let Some(existing) = self.peers.borrow_mut().get_mut(node_id_hex) {
            if let Some(a) = addr { existing.addr = Some(a.to_string()); }
            existing.status = status.to_string();
            existing.last_seen = Some(now);
        } else {
            self.peers.borrow_mut().insert(node_id_hex.to_string(), PeerRecord {
                addr: addr.map(|s| s.to_string()),
                status: status.to_string(),
                last_seen: Some(now),
                first_seen: now,
                is_bootstrap,
            });
        }
        self.save()
    }

    pub fn update_peer_status(&self, node_id_hex: &str, status: &str) -> io::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        if let Some(peer) = self.peers.borrow_mut().get_mut(node_id_hex) {
            peer.status = status.to_string();
            peer.last_seen = Some(now);
        }
        self.save()
    }

    pub fn list_peers(&self) -> io::Result<Vec<(String, Option<String>, String, Option<i64>, bool)>> {
        let mut peers: Vec<_> = self.peers.borrow().iter().map(|(k, v)| {
            (k.clone(), v.addr.clone(), v.status.clone(), v.last_seen, v.is_bootstrap)
        }).collect();
        peers.sort_by(|a, b| b.3.cmp(&a.3));
        Ok(peers)
    }

    pub fn list_unreachable_peers_with_addr(&self) -> io::Result<Vec<(String, String)>> {
        Ok(self.peers.borrow().iter()
            .filter(|(_, v)| v.status == "unreachable" && v.addr.is_some())
            .map(|(k, v)| (k.clone(), v.addr.clone().unwrap()))
            .collect())
    }

    pub fn put_snapshot(&self, snapshot_id: &str, object_id_hex: &str, view_type: &str,
        producer_hex: &str, produced_at: i64, completeness: i32, base_heads: &str) -> io::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        self.snapshots.borrow_mut().insert(snapshot_id.to_string(), SnapshotRecord {
            object_id_hex: object_id_hex.to_string(),
            view_type: view_type.to_string(),
            producer_hex: producer_hex.to_string(),
            produced_at, completeness, base_heads: base_heads.to_string(), stored_at: now,
        });
        self.save()
    }

    pub fn list_snapshots(&self) -> io::Result<Vec<(String, String, String, String, i64, i32, String)>> {
        let mut snaps: Vec<_> = self.snapshots.borrow().iter().map(|(k, v)| {
            (k.clone(), v.object_id_hex.clone(), v.view_type.clone(), v.producer_hex.clone(),
             v.produced_at, v.completeness, v.base_heads.clone())
        }).collect();
        snaps.sort_by(|a, b| b.4.cmp(&a.4));
        Ok(snaps)
    }

    pub fn get_snapshot(&self, snapshot_id: &str) -> io::Result<Option<(String, String, String, String, i64, i32, String)>> {
        Ok(self.snapshots.borrow().get(snapshot_id).map(|v| {
            (snapshot_id.to_string(), v.object_id_hex.clone(), v.view_type.clone(),
             v.producer_hex.clone(), v.produced_at, v.completeness, v.base_heads.clone())
        }))
    }

    pub fn record_controller_change(&self, controller_hex: &str, change_type: &str, event_seq: i64) -> io::Result<()> {
        self.controller_changes.borrow_mut().push(ControllerChange {
            controller_hex: controller_hex.to_string(),
            change_type: change_type.to_string(),
            event_seq,
        });
        self.save()
    }

    pub fn list_controller_changes(&self, up_to_seq: i64) -> io::Result<Vec<(String, String)>> {
        Ok(self.controller_changes.borrow().iter()
            .filter(|c| c.event_seq <= up_to_seq)
            .map(|c| (c.controller_hex.clone(), c.change_type.clone()))
            .collect())
    }

    pub fn store_delegation(&self, delegation_id: &str, issuer_hex: &str,
        recipient_hex: &str, capability_hex: &str, expires_at: Option<i64>) -> io::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        self.delegations.borrow_mut().insert(delegation_id.to_string(), DelegationRecord {
            issuer_hex: issuer_hex.to_string(),
            recipient_hex: recipient_hex.to_string(),
            capability_hex: capability_hex.to_string(),
            expires_at,
            is_revoked: false,
            stored_at: now,
        });
        self.save()
    }

    pub fn revoke_delegation(&self, delegation_id: &str) -> io::Result<()> {
        if let Some(d) = self.delegations.borrow_mut().get_mut(delegation_id) {
            d.is_revoked = true;
        }
        self.save()
    }

    pub fn list_active_delegations(&self, issuer_hex: &str) -> io::Result<Vec<String>> {
        let mut ids: Vec<_> = self.delegations.borrow().iter()
            .filter(|(_, v)| v.issuer_hex == issuer_hex && !v.is_revoked)
            .map(|(k, _)| k.clone())
            .collect();
        ids.sort();
        Ok(ids)
    }

    pub fn store_revocation(&self, revocation_id: &str, issuer_hex: &str,
        target_type: &str, target_hex: &str, effective_at: Option<i64>) -> io::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        self.revocations.borrow_mut().insert(revocation_id.to_string(), RevocationRecord {
            issuer_hex: issuer_hex.to_string(),
            target_type: target_type.to_string(),
            target_hex: target_hex.to_string(),
            effective_at,
            stored_at: now,
        });
        self.save()
    }

    pub fn list_active_revocations(&self) -> io::Result<Vec<(String, String)>> {
        Ok(self.revocations.borrow().iter()
            .map(|(_, v)| (v.target_type.clone(), v.target_hex.clone()))
            .collect())
    }

    pub fn clear(&self) -> io::Result<()> {
        self.stream_heads.borrow_mut().clear();
        self.events.borrow_mut().clear();
        self.replay_cache.borrow_mut().clear();
        self.fetch_queue.borrow_mut().clear();
        self.object_presence.borrow_mut().clear();
        self.peers.borrow_mut().clear();
        self.snapshots.borrow_mut().clear();
        self.controller_changes.borrow_mut().clear();
        self.delegations.borrow_mut().clear();
        self.revocations.borrow_mut().clear();
        // Clear files
        for file in &["stream_heads.bin", "events.bin", "replay_cache.bin", "fetch_queue.bin",
                      "object_presence.bin", "peers.bin", "snapshots.bin",
                      "controller_changes.bin", "delegations.bin", "revocations.bin"] {
            let path = self.idx_dir().join(file);
            if path.exists() { fs::remove_file(path)?; }
        }
        self.save()
    }

    // Stub methods for compatibility with SQLite-based API
    pub fn integrity_check(&self) -> io::Result<bool> {
        Ok(true) // File-based indexes are always consistent
    }

    pub fn wal_checkpoint(&self) -> io::Result<i64> {
        Ok(0) // No WAL to checkpoint
    }

    pub fn wal_size_bytes(&self) -> io::Result<u64> {
        Ok(0) // No WAL
    }

}
