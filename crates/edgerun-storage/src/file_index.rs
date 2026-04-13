//! THE EVENT LOG IS THE STATE.
//!
#![allow(dead_code)]
//! File-based index — materialized view rebuilt from the event log.
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
use std::sync::atomic::{AtomicI64, Ordering};
use edgerun_rt::sync::RwLock;

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
        Some(s) => { w.write_all(&[1])?; write_str(w, s)?; }
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

/// Credential index entry — maps namespace/name → blob_id.
#[derive(Clone)]
pub struct CredentialRecord {
    /// The blob_id where the encrypted credential is stored.
    pub blob_id: String,
    /// Human-readable description (optional, stored for auditing).
    pub description: Option<String>,
    /// When the credential was stored.
    pub stored_at: i64,
}

#[derive(Clone)]
pub struct ControllerChange {
    pub controller_hex: String,
    pub change_type: String,
    pub event_seq: i64,
}

/// Persistent work accounting record.
/// Stores the serialized WorkAccounting bytes along with query indexes.
#[derive(Clone)]
pub struct WorkAccountingRecord {
    /// Serialized WorkAccounting (from edgerun_core::accounting)
    pub data: Vec<u8>,
    /// SHA-256 hash of the record (unique identifier)
    pub record_hash: String,
    /// Requester identity (for indexing)
    pub requester_hex: String,
    /// Provider identity (for indexing)
    pub provider_hex: String,
    /// Workload class string
    pub workload_class: String,
    /// Status string
    pub status: String,
    /// Start timestamp in µs
    pub started_at_us: u64,
    /// Billable compute RC-µs
    pub billable_rc_us: u64,
}

// ===========================================================================
// FileIndex
// ===========================================================================

pub struct FileIndex {
    stream_heads: RwLock<HashMap<String, StreamHead>>,
    events: RwLock<HashMap<(String, i64), EventRecord>>,
    replay_cache: RwLock<HashMap<(String, String), ReplayEntry>>,
    fetch_queue: RwLock<Vec<FetchEntry>>,
    next_fetch_id: AtomicI64,
    object_presence: RwLock<HashMap<String, (String, String, String)>>,
    peers: RwLock<HashMap<String, PeerRecord>>,
    snapshots: RwLock<HashMap<String, SnapshotRecord>>,
    controller_changes: RwLock<Vec<ControllerChange>>,
    delegations: RwLock<HashMap<String, DelegationRecord>>,
    revocations: RwLock<HashMap<String, RevocationRecord>>,
    credentials: RwLock<HashMap<String, CredentialRecord>>,
    work_accounting: RwLock<Vec<WorkAccountingRecord>>,
    data_root: PathBuf,
}

impl FileIndex {
    // Note: all methods take &self because interior mutability is via RwLock.

    pub fn open(data_root: &PathBuf) -> io::Result<Self> {
        let idx_dir = data_root.join("indexes");
        fs::create_dir_all(&idx_dir)?;

        let index = Self {
            stream_heads: RwLock::new(HashMap::new()),
            events: RwLock::new(HashMap::new()),
            replay_cache: RwLock::new(HashMap::new()),
            fetch_queue: RwLock::new(Vec::new()),
            next_fetch_id: AtomicI64::new(1),
            object_presence: RwLock::new(HashMap::new()),
            peers: RwLock::new(HashMap::new()),
            snapshots: RwLock::new(HashMap::new()),
            controller_changes: RwLock::new(Vec::new()),
            delegations: RwLock::new(HashMap::new()),
            revocations: RwLock::new(HashMap::new()),
            credentials: RwLock::new(HashMap::new()),
            work_accounting: RwLock::new(Vec::new()),
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
                self.stream_heads.write().insert(key, StreamHead { seq, hash });
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
                self.peers.write().insert(key, PeerRecord { addr, status, last_seen, first_seen, is_bootstrap });
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
                self.snapshots.write().insert(key, SnapshotRecord {
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
                self.delegations.write().insert(key, DelegationRecord {
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
                self.revocations.write().insert(key, RevocationRecord {
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
                self.object_presence.write().insert(key, (rep_id, blob_id, status));
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
                self.replay_cache.write().insert((node, cmd_hash), ReplayEntry { command_id: cmd_id, decision_event_seq: seq });
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
                self.controller_changes.write().push(ControllerChange { controller_hex, change_type, event_seq });
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
                self.fetch_queue.write().push(FetchEntry { id, target_type, target_id, priority, created_at, status });
                if id >= self.next_fetch_id.load(Ordering::Relaxed) {
                    self.next_fetch_id.store(id + 1, Ordering::Relaxed);
                }
            }
        }

        // Load work_accounting
        if let Ok(data) = fs::read(self.idx_dir().join("work_accounting.bin")) {
            let data_len = data.len() as u64;
            let mut r = std::io::Cursor::new(data);
            while r.position() < data_len {
                let data_len_val = read_u64(&mut r)? as usize;
                let mut data = vec![0u8; data_len_val];
                r.read_exact(&mut data)?;
                let record_hash = read_str(&mut r)?;
                let requester_hex = read_str(&mut r)?;
                let provider_hex = read_str(&mut r)?;
                let workload_class = read_str(&mut r)?;
                let status = read_str(&mut r)?;
                let started_at_us = read_u64(&mut r)?;
                let billable_rc_us = read_u64(&mut r)?;
                self.work_accounting.write().push(WorkAccountingRecord {
                    data, record_hash, requester_hex, provider_hex,
                    workload_class, status, started_at_us, billable_rc_us,
                });
            }
        }

        // Load credentials
        if let Ok(data) = fs::read(self.idx_dir().join("credentials.bin")) {
            let data_len = data.len() as u64;
            let mut r = std::io::Cursor::new(data);
            while r.position() < data_len {
                let key = read_str(&mut r)?;
                let blob_id = read_str(&mut r)?;
                let description = read_option_str(&mut r)?;
                let stored_at = read_u64(&mut r)? as i64;
                self.credentials.write().insert(key, CredentialRecord {
                    blob_id, description, stored_at,
                });
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
            for (k, v) in self.stream_heads.read().iter() {
                write_str(w, k)?;
                write_u64(w, v.seq as u64)?;
                write_u64(w, v.hash.len() as u64)?;
                w.write_all(&v.hash)?;
            }
            Ok(())
        })?;

        // Save peers
        self.persist("peers.bin", |w| {
            for (k, v) in self.peers.read().iter() {
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
            for (k, v) in self.snapshots.read().iter() {
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
            for (k, v) in self.delegations.read().iter() {
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
            for (k, v) in self.revocations.read().iter() {
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
            for (k, (rep, blob, status)) in self.object_presence.read().iter() {
                write_str(w, k)?;
                write_str(w, rep)?;
                write_str(w, blob)?;
                write_str(w, status)?;
            }
            Ok(())
        })?;

        // Save replay_cache
        self.persist("replay_cache.bin", |w| {
            for ((node, cmd_hash), entry) in self.replay_cache.read().iter() {
                write_str(w, node)?;
                write_str(w, cmd_hash)?;
                write_str(w, &entry.command_id)?;
                write_u64(w, entry.decision_event_seq as u64)?;
            }
            Ok(())
        })?;

        // Save controller_changes
        self.persist("controller_changes.bin", |w| {
            for v in self.controller_changes.read().iter() {
                write_str(w, &v.controller_hex)?;
                write_str(w, &v.change_type)?;
                write_u64(w, v.event_seq as u64)?;
            }
            Ok(())
        })?;

        // Save fetch_queue
        self.persist("fetch_queue.bin", |w| {
            for v in self.fetch_queue.read().iter() {
                write_u64(w, v.id as u64)?;
                write_str(w, &v.target_type)?;
                write_str(w, &v.target_id)?;
                write_u64(w, v.priority as u64)?;
                write_u64(w, v.created_at as u64)?;
                write_str(w, &v.status)?;
            }
            Ok(())
        })?;

        // Save work_accounting
        self.persist("work_accounting.bin", |w| {
            for v in self.work_accounting.read().iter() {
                write_u64(w, v.data.len() as u64)?;
                w.write_all(&v.data)?;
                write_str(w, &v.record_hash)?;
                write_str(w, &v.requester_hex)?;
                write_str(w, &v.provider_hex)?;
                write_str(w, &v.workload_class)?;
                write_str(w, &v.status)?;
                write_u64(w, v.started_at_us)?;
                write_u64(w, v.billable_rc_us)?;
            }
            Ok(())
        })?;

        // Save credentials
        self.persist("credentials.bin", |w| {
            for (k, v) in self.credentials.read().iter() {
                write_str(w, k)?;
                write_str(w, &v.blob_id)?;
                write_option_str(w, v.description.as_deref())?;
                write_u64(w, v.stored_at as u64)?;
            }
            Ok(())
        })?;

        Ok(())
    }

    // ===========================================================================
    // Public API
    // ===========================================================================

    pub fn put_event(&self, stream_id: &str, seq: i64, event_hash: &[u8], file_offset: u64, envelope_version: i64) -> io::Result<()> {
        // FileIndex uses RefCell for interior mutability and is accessed
        // exclusively from the single store task thread. No cross-thread
        // synchronization is required.
        let _record = EventRecord {
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
        self.stream_heads.write().insert(stream_id.to_string(), StreamHead { seq, hash: hash.to_vec() });
        self.save()
    }

    pub fn get_head(&self, stream_id: &str) -> io::Result<Option<(i64, Vec<u8>)>> {
        Ok(self.stream_heads.read().get(stream_id).map(|h| (h.seq, h.hash.clone())))
    }

    pub fn put_replay_entry(&self, target_node: &str, command_hash: &str, command_id: &str, decision_event_seq: i64) -> io::Result<()> {
        self.replay_cache.write().insert(
            (target_node.to_string(), command_hash.to_string()),
            ReplayEntry { command_id: command_id.to_string(), decision_event_seq },
        );
        self.save()
    }

    pub fn get_replay_entry(&self, target_node: &str, command_hash: &str) -> io::Result<Option<ReplayEntry>> {
        Ok(self.replay_cache.read().get(&(target_node.to_string(), command_hash.to_string())).cloned())
    }

    pub fn enqueue_fetch(&self, target_type: &str, target_id: &str, priority: i64) -> io::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let id = self.next_fetch_id.load(Ordering::Relaxed);
        self.next_fetch_id.store(id + 1, Ordering::Relaxed);
        self.fetch_queue.write().push(FetchEntry {
            id, target_type: target_type.to_string(), target_id: target_id.to_string(),
            priority, created_at: now, status: "pending".to_string(),
        });
        self.save()
    }

    pub fn dequeue_fetch(&self) -> io::Result<Option<FetchEntry>> {
        // Find first pending entry
        for entry in self.fetch_queue.read().iter() {
            if entry.status == "pending" {
                let result = entry.clone();
                return Ok(Some(result));
            }
        }
        Ok(None)
    }

    pub fn mark_fetch_done(&self, fetch_id: i64) -> io::Result<()> {
        for entry in self.fetch_queue.write().iter_mut() {
            if entry.id == fetch_id { entry.status = "done".to_string(); }
        }
        self.save()
    }

    pub fn mark_fetch_failed(&self, fetch_id: i64) -> io::Result<()> {
        for entry in self.fetch_queue.write().iter_mut() {
            if entry.id == fetch_id { entry.status = "failed".to_string(); }
        }
        self.save()
    }

    pub fn mark_object_present(&self, object_id: &str, representation_id: &str, blob_id: &str) -> io::Result<()> {
        self.object_presence.write().insert(
            object_id.to_string(),
            (representation_id.to_string(), blob_id.to_string(), "present".to_string()),
        );
        self.save()
    }

    pub fn is_object_present(&self, object_id: &str) -> io::Result<bool> {
        Ok(self.object_presence.read().get(object_id).map(|(_, _, s)| s == "present").unwrap_or(false))
    }

    pub fn list_stream_heads(&self) -> io::Result<Vec<(String, i64, Vec<u8>)>> {
        Ok(self.stream_heads.read().iter().map(|(k, v)| (k.clone(), v.seq, v.hash.clone())).collect())
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
        let mut ids: Vec<String> = self.stream_heads.read().keys().cloned().collect();
        ids.sort();
        Ok(ids)
    }

    pub fn lookup_objects(&self, object_ids: &[String]) -> io::Result<Vec<(String, Option<String>, Option<String>, String)>> {
        let mut result = Vec::new();
        for oid in object_ids {
            if let Some((rep, blob, status)) = self.object_presence.read().get(oid) {
                result.push((oid.clone(), Some(rep.clone()), Some(blob.clone()), status.clone()));
            }
        }
        Ok(result)
    }

    pub fn upsert_peer(&self, node_id_hex: &str, addr: Option<&str>, status: &str, is_bootstrap: bool) -> io::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let exists = self.peers.read().contains_key(node_id_hex);
        if exists {
            if let Some(a) = addr {
                self.peers.write().get_mut(node_id_hex).unwrap().addr = Some(a.to_string());
            }
            self.peers.write().get_mut(node_id_hex).unwrap().status = status.to_string();
            self.peers.write().get_mut(node_id_hex).unwrap().last_seen = Some(now);
        } else {
            self.peers.write().insert(node_id_hex.to_string(), PeerRecord {
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
        if let Some(peer) = self.peers.write().get_mut(node_id_hex) {
            peer.status = status.to_string();
            peer.last_seen = Some(now);
        }
        self.save()
    }

    pub fn list_peers(&self) -> io::Result<Vec<(String, Option<String>, String, Option<i64>, bool)>> {
        let mut peers: Vec<_> = self.peers.read().iter().map(|(k, v)| {
            (k.clone(), v.addr.clone(), v.status.clone(), v.last_seen, v.is_bootstrap)
        }).collect();
        peers.sort_by(|a, b| b.3.cmp(&a.3));
        Ok(peers)
    }

    pub fn list_unreachable_peers_with_addr(&self) -> io::Result<Vec<(String, String)>> {
        Ok(self.peers.read().iter()
            .filter(|(_, v)| v.status == "unreachable" && v.addr.is_some())
            .map(|(k, v)| (k.clone(), v.addr.clone().unwrap()))
            .collect())
    }

    pub fn put_snapshot(&self, snapshot_id: &str, object_id_hex: &str, view_type: &str,
        producer_hex: &str, produced_at: i64, completeness: i32, base_heads: &str) -> io::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        self.snapshots.write().insert(snapshot_id.to_string(), SnapshotRecord {
            object_id_hex: object_id_hex.to_string(),
            view_type: view_type.to_string(),
            producer_hex: producer_hex.to_string(),
            produced_at, completeness, base_heads: base_heads.to_string(), stored_at: now,
        });
        self.save()
    }

    pub fn list_snapshots(&self) -> io::Result<Vec<(String, String, String, String, i64, i32, String)>> {
        let mut snaps: Vec<_> = self.snapshots.read().iter().map(|(k, v)| {
            (k.clone(), v.object_id_hex.clone(), v.view_type.clone(), v.producer_hex.clone(),
             v.produced_at, v.completeness, v.base_heads.clone())
        }).collect();
        snaps.sort_by(|a, b| b.4.cmp(&a.4));
        Ok(snaps)
    }

    pub fn get_snapshot(&self, snapshot_id: &str) -> io::Result<Option<(String, String, String, String, i64, i32, String)>> {
        Ok(self.snapshots.read().get(snapshot_id).map(|v| {
            (snapshot_id.to_string(), v.object_id_hex.clone(), v.view_type.clone(),
             v.producer_hex.clone(), v.produced_at, v.completeness, v.base_heads.clone())
        }))
    }

    pub fn record_controller_change(&self, controller_hex: &str, change_type: &str, event_seq: i64) -> io::Result<()> {
        self.controller_changes.write().push(ControllerChange {
            controller_hex: controller_hex.to_string(),
            change_type: change_type.to_string(),
            event_seq,
        });
        self.save()
    }

    pub fn list_controller_changes(&self, up_to_seq: i64) -> io::Result<Vec<(String, String)>> {
        Ok(self.controller_changes.read().iter()
            .filter(|c| c.event_seq <= up_to_seq)
            .map(|c| (c.controller_hex.clone(), c.change_type.clone()))
            .collect())
    }

    pub fn store_delegation(&self, delegation_id: &str, issuer_hex: &str,
        recipient_hex: &str, capability_hex: &str, expires_at: Option<i64>) -> io::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        self.delegations.write().insert(delegation_id.to_string(), DelegationRecord {
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
        if let Some(d) = self.delegations.write().get_mut(delegation_id) {
            d.is_revoked = true;
        }
        self.save()
    }

    pub fn list_active_delegations(&self, issuer_hex: &str) -> io::Result<Vec<String>> {
        let mut ids: Vec<_> = self.delegations.read().iter()
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
        self.revocations.write().insert(revocation_id.to_string(), RevocationRecord {
            issuer_hex: issuer_hex.to_string(),
            target_type: target_type.to_string(),
            target_hex: target_hex.to_string(),
            effective_at,
            stored_at: now,
        });
        self.save()
    }

    pub fn list_active_revocations(&self) -> io::Result<Vec<(String, String)>> {
        Ok(self.revocations.read().iter()
            .map(|(_, v)| (v.target_type.clone(), v.target_hex.clone()))
            .collect())
    }

    // ===========================================================================
    // Work accounting
    // ===========================================================================

    /// Record a completed work unit.
    pub fn record_work_accounting(&self, record: WorkAccountingRecord) -> io::Result<()> {
        self.work_accounting.write().push(record);
        self.save()
    }

    /// Get total billable RC-µs for a requester (buyer).
    pub fn total_billable_for_requester(&self, requester_hex: &str) -> io::Result<u64> {
        Ok(self.work_accounting.read().iter()
            .filter(|r| r.requester_hex == requester_hex && r.status == "completed")
            .map(|r| r.billable_rc_us)
            .sum())
    }

    /// Get total billable RC-µs for a provider (seller).
    pub fn total_billable_for_provider(&self, provider_hex: &str) -> io::Result<u64> {
        Ok(self.work_accounting.read().iter()
            .filter(|r| r.provider_hex == provider_hex && r.status == "completed")
            .map(|r| r.billable_rc_us)
            .sum())
    }

    /// Get all work records in a time window.
    pub fn work_in_time_range(&self, from_us: u64, to_us: u64) -> io::Result<Vec<WorkAccountingRecord>> {
        Ok(self.work_accounting.read().iter()
            .filter(|r| r.started_at_us >= from_us && r.started_at_us <= to_us)
            .cloned()
            .collect())
    }

    /// Get all work records for a specific workload class.
    pub fn work_by_class(&self, class: &str) -> io::Result<Vec<WorkAccountingRecord>> {
        Ok(self.work_accounting.read().iter()
            .filter(|r| r.workload_class == class)
            .cloned()
            .collect())
    }

    /// Get all work records with a specific status.
    pub fn work_by_status(&self, status: &str) -> io::Result<Vec<WorkAccountingRecord>> {
        Ok(self.work_accounting.read().iter()
            .filter(|r| r.status == status)
            .cloned()
            .collect())
    }

    /// Get all work records.
    pub fn list_all_work(&self) -> io::Result<Vec<WorkAccountingRecord>> {
        Ok(self.work_accounting.read().iter().cloned().collect())
    }

    // ===========================================================================
    // Credential index
    // ===========================================================================

    /// Store a credential mapping — maps `{namespace}/{name}` → `blob_id`.
    ///
    /// The actual encrypted secret is stored via the `BlobStore`; this index
    /// only tracks the mapping so credentials can be retrieved by name.
    pub fn put_credential(&self, namespace: &str, name: &str, blob_id: &str, description: Option<&str>) -> io::Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
        let key = format!("{}/{}", namespace, name);
        self.credentials.write().insert(key, CredentialRecord {
            blob_id: blob_id.to_string(),
            description: description.map(|s| s.to_string()),
            stored_at: now,
        });
        self.save()
    }

    /// Look up the blob_id for a named credential.
    pub fn get_credential(&self, namespace: &str, name: &str) -> io::Result<Option<CredentialRecord>> {
        let key = format!("{}/{}", namespace, name);
        Ok(self.credentials.read().get(&key).cloned())
    }

    /// Delete a credential from the index. Returns `true` if it existed.
    ///
    /// Note: this only removes the index entry — the encrypted blob on
    /// disk is left in place (content-addressed, can be garbage collected later).
    pub fn delete_credential(&self, namespace: &str, name: &str) -> io::Result<bool> {
        let key = format!("{}/{}", namespace, name);
        let existed = self.credentials.write().remove(&key).is_some();
        if existed {
            self.save()?;
        }
        Ok(existed)
    }

    /// List all credential names in a namespace.
    pub fn list_credentials(&self, namespace: &str) -> io::Result<Vec<(String, Option<String>, i64)>> {
        let prefix = format!("{}/", namespace);
        let mut results: Vec<_> = self.credentials.read().iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, v)| {
                let name = k.strip_prefix(&prefix).unwrap_or(k).to_string();
                (name, v.description.clone(), v.stored_at)
            })
            .collect();
        results.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(results)
    }

    /// List all namespaces that have stored credentials.
    pub fn list_credential_namespaces(&self) -> io::Result<Vec<String>> {
        let mut namespaces: Vec<_> = self.credentials.read().keys()
            .filter_map(|k| k.split_once('/').map(|(ns, _)| ns.to_string()))
            .collect();
        namespaces.sort();
        namespaces.dedup();
        Ok(namespaces)
    }

    pub fn clear(&self) -> io::Result<()> {
        self.stream_heads.write().clear();
        self.events.write().clear();
        self.replay_cache.write().clear();
        self.fetch_queue.write().clear();
        self.object_presence.write().clear();
        self.peers.write().clear();
        self.snapshots.write().clear();
        self.controller_changes.write().clear();
        self.delegations.write().clear();
        self.revocations.write().clear();
        self.credentials.write().clear();
        self.work_accounting.write().clear();
        // Clear files
        for file in &["stream_heads.bin", "events.bin", "replay_cache.bin", "fetch_queue.bin",
                      "object_presence.bin", "peers.bin", "snapshots.bin",
                      "controller_changes.bin", "delegations.bin", "revocations.bin",
                      "credentials.bin", "work_accounting.bin"] {
            let path = self.idx_dir().join(file);
            if path.exists() { fs::remove_file(path)?; }
        }
        self.save()
    }

    // -----------------------------------------------------------------------
    // Integrity and maintenance
    // -----------------------------------------------------------------------

    /// Verifies that all index files on disk can be read and parsed without
    /// errors. Returns `Ok(false)` if any file is corrupted.
    ///
    /// This reads each `.bin` file and validates record boundaries, string
    /// lengths, and u64 fields. It does NOT modify the in-memory state.
    pub fn integrity_check(&self) -> io::Result<bool> {
        let idx_dir = self.idx_dir();
        if !idx_dir.exists() {
            return Ok(true); // No indexes yet — trivially consistent
        }

        let bin_files = [
            "stream_heads.bin",
            "events.bin",
            "replay_cache.bin",
            "peers.bin",
            "snapshots.bin",
            "delegations.bin",
            "revocations.bin",
            "credentials.bin",
            "object_presence.bin",
            "controller_changes.bin",
            "fetch_queue.bin",
            "work_accounting.bin",
        ];

        for file in &bin_files {
            let path = idx_dir.join(file);
            if !path.exists() {
                continue; // Empty index — file hasn't been created yet
            }

            let data = fs::read(&path)?;
            if !validate_bin_file(&data, file) {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn wal_checkpoint(&self) -> io::Result<i64> {
        Ok(0) // No WAL to checkpoint — data is in memory + .bin files
    }

    pub fn wal_size_bytes(&self) -> io::Result<u64> {
        Ok(0) // No WAL
    }
}

// ---------------------------------------------------------------------------
// Binary format validation helpers
// ---------------------------------------------------------------------------

/// Validates that a binary index file can be fully parsed without errors.
/// Returns `true` if all records parse cleanly, `false` on any corruption.
pub fn validate_bin_file(data: &[u8], filename: &str) -> bool {
    if data.is_empty() {
        return true;
    }

    let data_len = data.len() as u64;
    let mut r = std::io::Cursor::new(data);

    match filename {
        "stream_heads.bin" => validate_stream_heads(&mut r, data_len),
        "events.bin" => validate_events_log(&mut r, data_len),
        "replay_cache.bin" => validate_replay_cache(&mut r, data_len),
        "peers.bin" => validate_peers(&mut r, data_len),
        "snapshots.bin" => validate_simple_kv(&mut r, data_len),
        "delegations.bin" => validate_delegations(&mut r, data_len),
        "revocations.bin" => validate_revocations(&mut r, data_len),
        "credentials.bin" => validate_credentials(&mut r, data_len),
        "object_presence.bin" => validate_object_presence(&mut r, data_len),
        "controller_changes.bin" => validate_controller_changes(&mut r, data_len),
        "fetch_queue.bin" => validate_fetch_queue(&mut r, data_len),
        "work_accounting.bin" => validate_work_accounting(&mut r, data_len),
        _ => false,
    }
}

fn validate_string_record(r: &mut std::io::Cursor<&[u8]>, _data_len: u64) -> bool {
    read_str(r).is_ok()
}

fn validate_stream_heads(r: &mut std::io::Cursor<&[u8]>, data_len: u64) -> bool {
    while r.position() < data_len {
        if read_str(r).is_err() { return false; }
        if read_u64(r).is_err() { return false; }
        let hash_len = match read_u64(r) {
            Ok(v) => v as usize,
            Err(_) => return false,
        };
        if hash_len > 1024 { return false; } // Sanity check
        let mut buf = vec![0u8; hash_len];
        if r.read_exact(&mut buf).is_err() { return false; }
    }
    true
}

fn validate_events_log(r: &mut std::io::Cursor<&[u8]>, data_len: u64) -> bool {
    while r.position() < data_len {
        if read_str(r).is_err() { return false; }  // stream_id
        if read_u64(r).is_err() { return false; }  // seq
        let hash_len = match read_u64(r) {
            Ok(v) => v as usize,
            Err(_) => return false,
        };
        if hash_len > 1024 { return false; }
        let mut buf = vec![0u8; hash_len];
        if r.read_exact(&mut buf).is_err() { return false; }
        if read_u64(r).is_err() { return false; }  // offset
        if read_u64(r).is_err() { return false; }  // envelope_version
    }
    true
}

fn validate_replay_cache(r: &mut std::io::Cursor<&[u8]>, data_len: u64) -> bool {
    while r.position() < data_len {
        if read_str(r).is_err() { return false; } // target_node
        if read_str(r).is_err() { return false; } // command_hash
        if read_str(r).is_err() { return false; } // command_id
        if read_u64(r).is_err() { return false; } // decision_event_seq
    }
    true
}

fn validate_peers(r: &mut std::io::Cursor<&[u8]>, data_len: u64) -> bool {
    while r.position() < data_len {
        if read_str(r).is_err() { return false; } // key
        if read_option_str(r).is_err() { return false; } // addr
        if read_str(r).is_err() { return false; } // status
        // last_seen (option u64)
        let mut tag = [0u8; 1];
        if r.read_exact(&mut tag).is_err() { return false; }
        if tag[0] == 1 && read_u64(r).is_err() { return false; }
        if read_u64(r).is_err() { return false; } // first_seen
        let mut t = [0u8; 1];
        if r.read_exact(&mut t).is_err() { return false; }
    }
    true
}

fn validate_simple_kv(r: &mut std::io::Cursor<&[u8]>, data_len: u64) -> bool {
    while r.position() < data_len {
        if read_str(r).is_err() { return false; } // key
        if read_str(r).is_err() { return false; } // value
    }
    true
}

fn validate_delegations(r: &mut std::io::Cursor<&[u8]>, data_len: u64) -> bool {
    while r.position() < data_len {
        if read_str(r).is_err() { return false; } // key (delegation_id)
        if read_str(r).is_err() { return false; } // issuer_hex
        if read_str(r).is_err() { return false; } // recipient_hex
        if read_str(r).is_err() { return false; } // capability_hex
        // expires_at: Option<i64>
        let mut tag = [0u8; 1];
        if r.read_exact(&mut tag).is_err() { return false; }
        if tag[0] == 1 && read_u64(r).is_err() { return false; }
        // is_revoked
        let mut t = [0u8; 1];
        if r.read_exact(&mut t).is_err() { return false; }
        // stored_at
        if read_u64(r).is_err() { return false; }
    }
    true
}

fn validate_revocations(r: &mut std::io::Cursor<&[u8]>, data_len: u64) -> bool {
    while r.position() < data_len {
        if read_str(r).is_err() { return false; } // key
        if read_str(r).is_err() { return false; } // issuer_hex
        if read_str(r).is_err() { return false; } // target_type
        if read_str(r).is_err() { return false; } // target_hex
        // effective_at: Option<i64>
        let mut tag = [0u8; 1];
        if r.read_exact(&mut tag).is_err() { return false; }
        if tag[0] == 1 && read_u64(r).is_err() { return false; }
        // revoked_at
        if read_u64(r).is_err() { return false; }
    }
    true
}

fn validate_credentials(r: &mut std::io::Cursor<&[u8]>, data_len: u64) -> bool {
    while r.position() < data_len {
        if read_str(r).is_err() { return false; } // key (namespace/name)
        if read_str(r).is_err() { return false; } // blob_id
        if read_option_str(r).is_err() { return false; } // description
        if read_u64(r).is_err() { return false; } // stored_at
    }
    true
}

fn validate_object_presence(r: &mut std::io::Cursor<&[u8]>, data_len: u64) -> bool {
    while r.position() < data_len {
        if read_str(r).is_err() { return false; } // object_id
        if read_str(r).is_err() { return false; } // rep_id
        if read_str(r).is_err() { return false; } // blob_id
        if read_str(r).is_err() { return false; } // status
    }
    true
}

fn validate_append_only(r: &mut std::io::Cursor<&[u8]>, data_len: u64) -> bool {
    while r.position() < data_len {
        if read_str(r).is_err() { return false; }
    }
    true
}

fn validate_controller_changes(r: &mut std::io::Cursor<&[u8]>, data_len: u64) -> bool {
    while r.position() < data_len {
        if read_str(r).is_err() { return false; }  // controller_hex
        if read_str(r).is_err() { return false; }  // change_type
        if read_u64(r).is_err() { return false; }  // event_seq
    }
    true
}

fn validate_fetch_queue(r: &mut std::io::Cursor<&[u8]>, data_len: u64) -> bool {
    while r.position() < data_len {
        if read_u64(r).is_err() { return false; }  // id
        if read_str(r).is_err() { return false; }   // target_type
        if read_str(r).is_err() { return false; }   // target_id
        if read_u64(r).is_err() { return false; }   // priority
        if read_u64(r).is_err() { return false; }   // created_at
        if read_str(r).is_err() { return false; }   // status
    }
    true
}

fn validate_work_accounting(r: &mut std::io::Cursor<&[u8]>, data_len: u64) -> bool {
    while r.position() < data_len {
        let data_len_val = match read_u64(r) {
            Ok(v) => v as usize,
            Err(_) => return false,
        };
        if data_len_val > data_len as usize { return false; }
        let mut buf = vec![0u8; data_len_val];
        if r.read_exact(&mut buf).is_err() { return false; }
        if read_str(r).is_err() { return false; }   // record_hash
        if read_str(r).is_err() { return false; }   // requester_hex
        if read_str(r).is_err() { return false; }   // provider_hex
        if read_str(r).is_err() { return false; }   // workload_class
        if read_str(r).is_err() { return false; }   // status
        if read_u64(r).is_err() { return false; }   // started_at_us
        if read_u64(r).is_err() { return false; }   // billable_rc_us
    }
    true
}
