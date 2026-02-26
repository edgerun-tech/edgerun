// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use edgerun_runtime_proto::SnapshotMaterializedEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mount {
    pub r#type: String,
    pub source: String,
    pub options: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SnapshotKind {
    Active,
    View,
    Committed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub key: String,
    pub parent: Option<String>,
    pub kind: SnapshotKind,
    pub labels: BTreeMap<String, String>,
    pub size_bytes: u64,
    pub inode_count: u64,
    pub mount_root: String,
}

impl Snapshot {
    pub fn mounts(&self) -> Vec<Mount> {
        let mut options = vec![
            "rbind".to_string(),
            "nosuid".to_string(),
            "nodev".to_string(),
        ];
        if matches!(self.kind, SnapshotKind::View) {
            options.push("ro".to_string());
        } else {
            options.push("rw".to_string());
        }
        vec![Mount {
            r#type: "bind".to_string(),
            source: self.mount_root.clone(),
            options,
        }]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotUsage {
    pub size_bytes: u64,
    pub inode_count: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotterError {
    #[error("snapshot not found: {0}")]
    NotFound(String),
    #[error("snapshot already exists: {0}")]
    AlreadyExists(String),
    #[error("invalid snapshot transition: {0}")]
    InvalidTransition(String),
    #[error("internal lock poisoned")]
    LockPoisoned,
    #[error("i/o error: {0}")]
    Io(String),
    #[error("serialization error: {0}")]
    Serde(String),
}

pub trait Snapshotter {
    fn prepare(
        &self,
        key: &str,
        parent: Option<&str>,
        labels: BTreeMap<String, String>,
    ) -> Result<Snapshot, SnapshotterError>;
    fn view(
        &self,
        key: &str,
        parent: &str,
        labels: BTreeMap<String, String>,
    ) -> Result<Snapshot, SnapshotterError>;
    fn commit(&self, name: &str, key: &str) -> Result<Snapshot, SnapshotterError>;
    fn mounts(&self, key: &str) -> Result<Vec<Mount>, SnapshotterError>;
    fn remove(&self, key: &str) -> Result<(), SnapshotterError>;
    fn stat(&self, key: &str) -> Result<Snapshot, SnapshotterError>;
    fn update_labels(
        &self,
        key: &str,
        labels: BTreeMap<String, String>,
    ) -> Result<Snapshot, SnapshotterError>;
    fn usage(&self, key: &str) -> Result<SnapshotUsage, SnapshotterError>;
    fn walk(&self) -> Result<Vec<Snapshot>, SnapshotterError>;
    fn cleanup(&self) -> Result<u64, SnapshotterError>;
}

#[derive(Debug, Clone)]
pub struct InMemorySnapshotter {
    workspace_id: String,
    snapshots: Arc<Mutex<BTreeMap<String, Snapshot>>>,
}

impl InMemorySnapshotter {
    pub fn new(workspace_id: impl Into<String>) -> Self {
        Self {
            workspace_id: workspace_id.into(),
            snapshots: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn build_materialized_event(
        &self,
        lane: &str,
        snapshot_key: &str,
        cursor_offset: u64,
        compacted_events: u64,
        ts_unix_ms: u64,
    ) -> SnapshotMaterializedEvent {
        let root_hash_blake3_hex = blake3::hash(snapshot_key.as_bytes()).to_hex().to_string();
        SnapshotMaterializedEvent {
            schema_version: 1,
            workspace_id: self.workspace_id.clone(),
            lane: lane.to_string(),
            snapshot_key: snapshot_key.to_string(),
            cursor_offset,
            root_hash_blake3_hex,
            events_compacted: compacted_events,
            ts_unix_ms,
        }
    }

    fn map(&self) -> &Arc<Mutex<BTreeMap<String, Snapshot>>> {
        &self.snapshots
    }
}

#[derive(Debug, Clone)]
pub struct PersistentSnapshotter {
    workspace_id: String,
    state_file: PathBuf,
    snapshots: Arc<Mutex<BTreeMap<String, Snapshot>>>,
}

impl PersistentSnapshotter {
    pub fn open(
        workspace_id: impl Into<String>,
        state_file: impl Into<PathBuf>,
    ) -> Result<Self, SnapshotterError> {
        let state_file = state_file.into();
        let snapshots = load_state_map(&state_file)?;
        Ok(Self {
            workspace_id: workspace_id.into(),
            state_file,
            snapshots: Arc::new(Mutex::new(snapshots)),
        })
    }

    pub fn workspace_id(&self) -> &str {
        &self.workspace_id
    }

    pub fn state_file(&self) -> &Path {
        &self.state_file
    }

    pub fn build_materialized_event(
        &self,
        lane: &str,
        snapshot_key: &str,
        cursor_offset: u64,
        compacted_events: u64,
        ts_unix_ms: u64,
    ) -> SnapshotMaterializedEvent {
        let root_hash_blake3_hex = blake3::hash(snapshot_key.as_bytes()).to_hex().to_string();
        SnapshotMaterializedEvent {
            schema_version: 1,
            workspace_id: self.workspace_id.clone(),
            lane: lane.to_string(),
            snapshot_key: snapshot_key.to_string(),
            cursor_offset,
            root_hash_blake3_hex,
            events_compacted: compacted_events,
            ts_unix_ms,
        }
    }

    fn map(&self) -> &Arc<Mutex<BTreeMap<String, Snapshot>>> {
        &self.snapshots
    }

    fn persist_locked(&self, map: &BTreeMap<String, Snapshot>) -> Result<(), SnapshotterError> {
        save_state_map(&self.state_file, map)
    }
}

fn run_prepare(
    map: &mut BTreeMap<String, Snapshot>,
    key: &str,
    parent: Option<&str>,
    labels: BTreeMap<String, String>,
) -> Result<Snapshot, SnapshotterError> {
    if map.contains_key(key) {
        return Err(SnapshotterError::AlreadyExists(key.to_string()));
    }
    if let Some(parent_key) = parent {
        if !map.contains_key(parent_key) {
            return Err(SnapshotterError::NotFound(parent_key.to_string()));
        }
    }
    let snapshot = Snapshot {
        key: key.to_string(),
        parent: parent.map(std::string::ToString::to_string),
        kind: SnapshotKind::Active,
        labels,
        size_bytes: 0,
        inode_count: 0,
        mount_root: format!("/run/edgerun/snapshots/{key}"),
    };
    map.insert(key.to_string(), snapshot.clone());
    Ok(snapshot)
}

fn run_view(
    map: &mut BTreeMap<String, Snapshot>,
    key: &str,
    parent: &str,
    labels: BTreeMap<String, String>,
) -> Result<Snapshot, SnapshotterError> {
    if map.contains_key(key) {
        return Err(SnapshotterError::AlreadyExists(key.to_string()));
    }
    if !map.contains_key(parent) {
        return Err(SnapshotterError::NotFound(parent.to_string()));
    }
    let snapshot = Snapshot {
        key: key.to_string(),
        parent: Some(parent.to_string()),
        kind: SnapshotKind::View,
        labels,
        size_bytes: 0,
        inode_count: 0,
        mount_root: format!("/run/edgerun/snapshots/{key}"),
    };
    map.insert(key.to_string(), snapshot.clone());
    Ok(snapshot)
}

fn run_commit(
    map: &mut BTreeMap<String, Snapshot>,
    name: &str,
    key: &str,
) -> Result<Snapshot, SnapshotterError> {
    if map.contains_key(name) {
        return Err(SnapshotterError::AlreadyExists(name.to_string()));
    }
    let current = map
        .get(key)
        .cloned()
        .ok_or_else(|| SnapshotterError::NotFound(key.to_string()))?;
    if !matches!(current.kind, SnapshotKind::Active) {
        return Err(SnapshotterError::InvalidTransition(format!(
            "only active snapshots can be committed: {key}"
        )));
    }
    let committed = Snapshot {
        key: name.to_string(),
        parent: current.parent.clone(),
        kind: SnapshotKind::Committed,
        labels: current.labels.clone(),
        size_bytes: current.size_bytes,
        inode_count: current.inode_count,
        mount_root: current.mount_root,
    };
    map.remove(key);
    map.insert(name.to_string(), committed.clone());
    Ok(committed)
}

fn load_state_map(path: &Path) -> Result<BTreeMap<String, Snapshot>, SnapshotterError> {
    if !path.exists() {
        return Ok(BTreeMap::new());
    }
    let raw = fs::read(path).map_err(|e| SnapshotterError::Io(e.to_string()))?;
    if raw.is_empty() {
        return Ok(BTreeMap::new());
    }
    serde_json::from_slice(&raw).map_err(|e| SnapshotterError::Serde(e.to_string()))
}

fn save_state_map(path: &Path, map: &BTreeMap<String, Snapshot>) -> Result<(), SnapshotterError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| SnapshotterError::Io(e.to_string()))?;
    }
    let bytes =
        serde_json::to_vec_pretty(map).map_err(|e| SnapshotterError::Serde(e.to_string()))?;
    fs::write(path, bytes).map_err(|e| SnapshotterError::Io(e.to_string()))
}

fn snapshotter_impl_prepare(
    map_ref: &Arc<Mutex<BTreeMap<String, Snapshot>>>,
    key: &str,
    parent: Option<&str>,
    labels: BTreeMap<String, String>,
) -> Result<Snapshot, SnapshotterError> {
    let mut map = map_ref.lock().map_err(|_| SnapshotterError::LockPoisoned)?;
    run_prepare(&mut map, key, parent, labels)
}

fn snapshotter_impl_view(
    map_ref: &Arc<Mutex<BTreeMap<String, Snapshot>>>,
    key: &str,
    parent: &str,
    labels: BTreeMap<String, String>,
) -> Result<Snapshot, SnapshotterError> {
    let mut map = map_ref.lock().map_err(|_| SnapshotterError::LockPoisoned)?;
    run_view(&mut map, key, parent, labels)
}

fn snapshotter_impl_commit(
    map_ref: &Arc<Mutex<BTreeMap<String, Snapshot>>>,
    name: &str,
    key: &str,
) -> Result<Snapshot, SnapshotterError> {
    let mut map = map_ref.lock().map_err(|_| SnapshotterError::LockPoisoned)?;
    run_commit(&mut map, name, key)
}

impl Snapshotter for InMemorySnapshotter {
    fn prepare(
        &self,
        key: &str,
        parent: Option<&str>,
        labels: BTreeMap<String, String>,
    ) -> Result<Snapshot, SnapshotterError> {
        snapshotter_impl_prepare(self.map(), key, parent, labels)
    }

    fn view(
        &self,
        key: &str,
        parent: &str,
        labels: BTreeMap<String, String>,
    ) -> Result<Snapshot, SnapshotterError> {
        snapshotter_impl_view(self.map(), key, parent, labels)
    }

    fn commit(&self, name: &str, key: &str) -> Result<Snapshot, SnapshotterError> {
        snapshotter_impl_commit(self.map(), name, key)
    }

    fn mounts(&self, key: &str) -> Result<Vec<Mount>, SnapshotterError> {
        let guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        let snapshot = guard
            .get(key)
            .ok_or_else(|| SnapshotterError::NotFound(key.to_string()))?;
        Ok(snapshot.mounts())
    }

    fn remove(&self, key: &str) -> Result<(), SnapshotterError> {
        let mut guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        if guard.remove(key).is_none() {
            return Err(SnapshotterError::NotFound(key.to_string()));
        }
        Ok(())
    }

    fn stat(&self, key: &str) -> Result<Snapshot, SnapshotterError> {
        let guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        guard
            .get(key)
            .cloned()
            .ok_or_else(|| SnapshotterError::NotFound(key.to_string()))
    }

    fn update_labels(
        &self,
        key: &str,
        labels: BTreeMap<String, String>,
    ) -> Result<Snapshot, SnapshotterError> {
        let mut guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        let entry = guard
            .get_mut(key)
            .ok_or_else(|| SnapshotterError::NotFound(key.to_string()))?;
        entry.labels = labels;
        Ok(entry.clone())
    }

    fn usage(&self, key: &str) -> Result<SnapshotUsage, SnapshotterError> {
        let guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        let entry = guard
            .get(key)
            .ok_or_else(|| SnapshotterError::NotFound(key.to_string()))?;
        Ok(SnapshotUsage {
            size_bytes: entry.size_bytes,
            inode_count: entry.inode_count,
        })
    }

    fn walk(&self) -> Result<Vec<Snapshot>, SnapshotterError> {
        let guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        Ok(guard.values().cloned().collect())
    }

    fn cleanup(&self) -> Result<u64, SnapshotterError> {
        let mut guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        let keys_to_drop: Vec<String> = guard
            .iter()
            .filter_map(|(key, snapshot)| {
                if matches!(snapshot.kind, SnapshotKind::View) {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();
        for key in &keys_to_drop {
            guard.remove(key);
        }
        Ok(keys_to_drop.len() as u64)
    }
}

impl Snapshotter for PersistentSnapshotter {
    fn prepare(
        &self,
        key: &str,
        parent: Option<&str>,
        labels: BTreeMap<String, String>,
    ) -> Result<Snapshot, SnapshotterError> {
        let mut map = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        let snapshot = run_prepare(&mut map, key, parent, labels)?;
        self.persist_locked(&map)?;
        Ok(snapshot)
    }

    fn view(
        &self,
        key: &str,
        parent: &str,
        labels: BTreeMap<String, String>,
    ) -> Result<Snapshot, SnapshotterError> {
        let mut map = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        let snapshot = run_view(&mut map, key, parent, labels)?;
        self.persist_locked(&map)?;
        Ok(snapshot)
    }

    fn commit(&self, name: &str, key: &str) -> Result<Snapshot, SnapshotterError> {
        let mut map = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        let snapshot = run_commit(&mut map, name, key)?;
        self.persist_locked(&map)?;
        Ok(snapshot)
    }

    fn mounts(&self, key: &str) -> Result<Vec<Mount>, SnapshotterError> {
        let guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        let snapshot = guard
            .get(key)
            .ok_or_else(|| SnapshotterError::NotFound(key.to_string()))?;
        Ok(snapshot.mounts())
    }

    fn remove(&self, key: &str) -> Result<(), SnapshotterError> {
        let mut guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        if guard.remove(key).is_none() {
            return Err(SnapshotterError::NotFound(key.to_string()));
        }
        self.persist_locked(&guard)
    }

    fn stat(&self, key: &str) -> Result<Snapshot, SnapshotterError> {
        let guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        guard
            .get(key)
            .cloned()
            .ok_or_else(|| SnapshotterError::NotFound(key.to_string()))
    }

    fn update_labels(
        &self,
        key: &str,
        labels: BTreeMap<String, String>,
    ) -> Result<Snapshot, SnapshotterError> {
        let mut guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        let entry = guard
            .get_mut(key)
            .ok_or_else(|| SnapshotterError::NotFound(key.to_string()))?;
        entry.labels = labels;
        let updated = entry.clone();
        self.persist_locked(&guard)?;
        Ok(updated)
    }

    fn usage(&self, key: &str) -> Result<SnapshotUsage, SnapshotterError> {
        let guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        let entry = guard
            .get(key)
            .ok_or_else(|| SnapshotterError::NotFound(key.to_string()))?;
        Ok(SnapshotUsage {
            size_bytes: entry.size_bytes,
            inode_count: entry.inode_count,
        })
    }

    fn walk(&self) -> Result<Vec<Snapshot>, SnapshotterError> {
        let guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        Ok(guard.values().cloned().collect())
    }

    fn cleanup(&self) -> Result<u64, SnapshotterError> {
        let mut guard = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        let keys_to_drop: Vec<String> = guard
            .iter()
            .filter_map(|(key, snapshot)| {
                if matches!(snapshot.kind, SnapshotKind::View) {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect();
        for key in &keys_to_drop {
            guard.remove(key);
        }
        self.persist_locked(&guard)?;
        Ok(keys_to_drop.len() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepare_commit_walk() {
        let s = InMemorySnapshotter::new("workspace-a");
        s.prepare("active-1", None, BTreeMap::new())
            .expect("prepare active");
        s.commit("committed-1", "active-1").expect("commit");
        let snapshots = s.walk().expect("walk");
        assert_eq!(snapshots.len(), 1);
        assert_eq!(snapshots[0].key, "committed-1");
        assert!(matches!(snapshots[0].kind, SnapshotKind::Committed));
    }

    #[test]
    fn view_and_cleanup() {
        let s = InMemorySnapshotter::new("workspace-a");
        s.prepare("base", None, BTreeMap::new())
            .expect("prepare base");
        s.commit("base-c", "base").expect("commit base");
        s.view("view-1", "base-c", BTreeMap::new())
            .expect("create view");
        let dropped = s.cleanup().expect("cleanup");
        assert_eq!(dropped, 1);
    }

    #[test]
    fn persistent_state_roundtrip() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = tmp.path().join("snapshot-state.json");

        let s = PersistentSnapshotter::open("workspace-p", state.clone()).expect("open persistent");
        s.prepare("a", None, BTreeMap::new()).expect("prepare");
        s.commit("a-c", "a").expect("commit");
        drop(s);

        let reopened = PersistentSnapshotter::open("workspace-p", state).expect("reopen");
        let all = reopened.walk().expect("walk");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].key, "a-c");
    }
}
