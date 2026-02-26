// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use edgerun_runtime_proto::wire::{
    LabelPairV1, SnapshotKindV1, SnapshotStateEntryV1, SnapshotStateMapV1,
};
use edgerun_runtime_proto::SnapshotMaterializedEvent;
use prost::Message;
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

const DEFAULT_SNAPSHOT_MOUNT_ROOT: &str = "/run/edgerun/snapshots";

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

    fn mount_root_base(&self) -> PathBuf {
        self.state_file
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("snapshots")
    }

    fn persist_locked(&self, map: &BTreeMap<String, Snapshot>) -> Result<(), SnapshotterError> {
        save_state_map(&self.state_file, map)
    }
}

fn snapshot_mount_root(base: &Path, key: &str) -> String {
    base.join(key).to_string_lossy().into_owned()
}

fn run_prepare(
    map: &mut BTreeMap<String, Snapshot>,
    key: &str,
    parent: Option<&str>,
    labels: BTreeMap<String, String>,
    mount_root_base: &Path,
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
        mount_root: snapshot_mount_root(mount_root_base, key),
    };
    map.insert(key.to_string(), snapshot.clone());
    Ok(snapshot)
}

fn run_view(
    map: &mut BTreeMap<String, Snapshot>,
    key: &str,
    parent: &str,
    labels: BTreeMap<String, String>,
    mount_root_base: &Path,
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
        mount_root: snapshot_mount_root(mount_root_base, key),
    };
    map.insert(key.to_string(), snapshot.clone());
    Ok(snapshot)
}

fn run_commit(
    map: &mut BTreeMap<String, Snapshot>,
    name: &str,
    key: &str,
    mount_root_base: &Path,
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
        mount_root: snapshot_mount_root(mount_root_base, name),
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
    let decoded = SnapshotStateMapV1::decode(raw.as_slice())
        .map_err(|e| SnapshotterError::Serde(format!("decode snapshot state: {e}")))?;
    if decoded.schema_version != 1 {
        return Err(SnapshotterError::Serde(format!(
            "unsupported snapshot state schema_version: {}",
            decoded.schema_version
        )));
    }

    let mut out = BTreeMap::new();
    for item in decoded.items {
        let kind = from_wire_snapshot_kind(item.kind).ok_or_else(|| {
            SnapshotterError::Serde(format!("invalid snapshot kind: {}", item.kind))
        })?;
        let labels = item
            .labels
            .into_iter()
            .map(|pair| (pair.key, pair.value))
            .collect();
        out.insert(
            item.key.clone(),
            Snapshot {
                key: item.key,
                parent: if item.parent.is_empty() {
                    None
                } else {
                    Some(item.parent)
                },
                kind,
                labels,
                size_bytes: item.size_bytes,
                inode_count: item.inode_count,
                mount_root: item.mount_root,
            },
        );
    }
    Ok(out)
}

fn save_state_map(path: &Path, map: &BTreeMap<String, Snapshot>) -> Result<(), SnapshotterError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| SnapshotterError::Io(e.to_string()))?;
    }
    let mut state = SnapshotStateMapV1 {
        schema_version: 1,
        items: Vec::with_capacity(map.len()),
    };
    for snapshot in map.values() {
        let labels = snapshot
            .labels
            .iter()
            .map(|(key, value)| LabelPairV1 {
                key: key.clone(),
                value: value.clone(),
            })
            .collect();
        state.items.push(SnapshotStateEntryV1 {
            key: snapshot.key.clone(),
            parent: snapshot.parent.clone().unwrap_or_default(),
            kind: to_wire_snapshot_kind(&snapshot.kind),
            labels,
            size_bytes: snapshot.size_bytes,
            inode_count: snapshot.inode_count,
            mount_root: snapshot.mount_root.clone(),
        });
    }
    let bytes = state.encode_to_vec();
    fs::write(path, bytes).map_err(|e| SnapshotterError::Io(e.to_string()))
}

fn to_wire_snapshot_kind(kind: &SnapshotKind) -> i32 {
    match kind {
        SnapshotKind::Active => SnapshotKindV1::Active as i32,
        SnapshotKind::View => SnapshotKindV1::View as i32,
        SnapshotKind::Committed => SnapshotKindV1::Committed as i32,
    }
}

fn from_wire_snapshot_kind(kind: i32) -> Option<SnapshotKind> {
    match SnapshotKindV1::try_from(kind).ok()? {
        SnapshotKindV1::Active => Some(SnapshotKind::Active),
        SnapshotKindV1::View => Some(SnapshotKind::View),
        SnapshotKindV1::Committed => Some(SnapshotKind::Committed),
        SnapshotKindV1::Unspecified => None,
    }
}

fn ensure_mount_root(path: &Path) -> Result<(), SnapshotterError> {
    fs::create_dir_all(path).map_err(|e| SnapshotterError::Io(e.to_string()))
}

fn copy_dir_tree(src: &Path, dst: &Path) -> Result<(), SnapshotterError> {
    if !src.exists() {
        return Ok(());
    }
    ensure_mount_root(dst)?;
    for entry in fs::read_dir(src).map_err(|e| SnapshotterError::Io(e.to_string()))? {
        let entry = entry.map_err(|e| SnapshotterError::Io(e.to_string()))?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|e| SnapshotterError::Io(e.to_string()))?;
        if file_type.is_dir() {
            copy_dir_tree(&src_path, &dst_path)?;
            continue;
        }
        if file_type.is_file() {
            if let Some(parent) = dst_path.parent() {
                ensure_mount_root(parent)?;
            }
            fs::copy(&src_path, &dst_path).map_err(|e| SnapshotterError::Io(e.to_string()))?;
        }
    }
    Ok(())
}

fn remove_dir_if_exists(path: &Path) -> Result<(), SnapshotterError> {
    if path.exists() {
        fs::remove_dir_all(path).map_err(|e| SnapshotterError::Io(e.to_string()))?;
    }
    Ok(())
}

fn move_dir_or_copy(src: &Path, dst: &Path) -> Result<(), SnapshotterError> {
    if src == dst {
        return Ok(());
    }
    if let Some(parent) = dst.parent() {
        ensure_mount_root(parent)?;
    }
    remove_dir_if_exists(dst)?;
    if src.exists() {
        match fs::rename(src, dst) {
            Ok(()) => return Ok(()),
            Err(_) => {
                copy_dir_tree(src, dst)?;
                remove_dir_if_exists(src)?;
                return Ok(());
            }
        }
    }
    ensure_mount_root(dst)
}

fn dir_usage(path: &Path) -> Result<SnapshotUsage, SnapshotterError> {
    if !path.exists() {
        return Ok(SnapshotUsage {
            size_bytes: 0,
            inode_count: 0,
        });
    }
    let mut usage = SnapshotUsage {
        size_bytes: 0,
        inode_count: 0,
    };
    let mut stack = vec![path.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in fs::read_dir(&current).map_err(|e| SnapshotterError::Io(e.to_string()))? {
            let entry = entry.map_err(|e| SnapshotterError::Io(e.to_string()))?;
            usage.inode_count += 1;
            let file_type = entry
                .file_type()
                .map_err(|e| SnapshotterError::Io(e.to_string()))?;
            if file_type.is_dir() {
                stack.push(entry.path());
                continue;
            }
            if file_type.is_file() {
                let meta = entry
                    .metadata()
                    .map_err(|e| SnapshotterError::Io(e.to_string()))?;
                usage.size_bytes = usage.size_bytes.saturating_add(meta.len());
            }
        }
    }
    Ok(usage)
}

fn snapshotter_impl_prepare(
    map_ref: &Arc<Mutex<BTreeMap<String, Snapshot>>>,
    key: &str,
    parent: Option<&str>,
    labels: BTreeMap<String, String>,
) -> Result<Snapshot, SnapshotterError> {
    let mut map = map_ref.lock().map_err(|_| SnapshotterError::LockPoisoned)?;
    run_prepare(
        &mut map,
        key,
        parent,
        labels,
        Path::new(DEFAULT_SNAPSHOT_MOUNT_ROOT),
    )
}

fn snapshotter_impl_view(
    map_ref: &Arc<Mutex<BTreeMap<String, Snapshot>>>,
    key: &str,
    parent: &str,
    labels: BTreeMap<String, String>,
) -> Result<Snapshot, SnapshotterError> {
    let mut map = map_ref.lock().map_err(|_| SnapshotterError::LockPoisoned)?;
    run_view(
        &mut map,
        key,
        parent,
        labels,
        Path::new(DEFAULT_SNAPSHOT_MOUNT_ROOT),
    )
}

fn snapshotter_impl_commit(
    map_ref: &Arc<Mutex<BTreeMap<String, Snapshot>>>,
    name: &str,
    key: &str,
) -> Result<Snapshot, SnapshotterError> {
    let mut map = map_ref.lock().map_err(|_| SnapshotterError::LockPoisoned)?;
    run_commit(&mut map, name, key, Path::new(DEFAULT_SNAPSHOT_MOUNT_ROOT))
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
        let parent_mount = parent.and_then(|p| map.get(p).map(|s| s.mount_root.clone()));
        let snapshot = run_prepare(&mut map, key, parent, labels, &self.mount_root_base())?;
        let snapshot_root = PathBuf::from(&snapshot.mount_root);
        if let Err(err) = (|| -> Result<(), SnapshotterError> {
            ensure_mount_root(&snapshot_root)?;
            if let Some(parent_root) = parent_mount {
                copy_dir_tree(Path::new(&parent_root), &snapshot_root)?;
            }
            Ok(())
        })() {
            map.remove(key);
            return Err(err);
        }
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
        let parent_mount = map
            .get(parent)
            .map(|s| s.mount_root.clone())
            .ok_or_else(|| SnapshotterError::NotFound(parent.to_string()))?;
        let snapshot = run_view(&mut map, key, parent, labels, &self.mount_root_base())?;
        let snapshot_root = PathBuf::from(&snapshot.mount_root);
        if let Err(err) = (|| -> Result<(), SnapshotterError> {
            ensure_mount_root(&snapshot_root)?;
            copy_dir_tree(Path::new(&parent_mount), &snapshot_root)?;
            Ok(())
        })() {
            map.remove(key);
            return Err(err);
        }
        self.persist_locked(&map)?;
        Ok(snapshot)
    }

    fn commit(&self, name: &str, key: &str) -> Result<Snapshot, SnapshotterError> {
        let mut map = self
            .map()
            .lock()
            .map_err(|_| SnapshotterError::LockPoisoned)?;
        let previous = map
            .get(key)
            .cloned()
            .ok_or_else(|| SnapshotterError::NotFound(key.to_string()))?;
        let snapshot = run_commit(&mut map, name, key, &self.mount_root_base())?;
        if let Err(err) = move_dir_or_copy(
            Path::new(&previous.mount_root),
            Path::new(&snapshot.mount_root),
        ) {
            map.remove(name);
            map.insert(key.to_string(), previous);
            return Err(err);
        }
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
        let removed = guard
            .remove(key)
            .ok_or_else(|| SnapshotterError::NotFound(key.to_string()))?;
        remove_dir_if_exists(Path::new(&removed.mount_root))?;
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
        let fs_usage = dir_usage(Path::new(&entry.mount_root))?;
        Ok(SnapshotUsage {
            size_bytes: fs_usage.size_bytes.max(entry.size_bytes),
            inode_count: fs_usage.inode_count.max(entry.inode_count),
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
            if let Some(removed) = guard.remove(key) {
                remove_dir_if_exists(Path::new(&removed.mount_root))?;
            }
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
        let state = tmp.path().join("snapshot-state.pb");

        let s = PersistentSnapshotter::open("workspace-p", state.clone()).expect("open persistent");
        s.prepare("a", None, BTreeMap::new()).expect("prepare");
        s.commit("a-c", "a").expect("commit");
        drop(s);

        let reopened = PersistentSnapshotter::open("workspace-p", state).expect("reopen");
        let all = reopened.walk().expect("walk");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].key, "a-c");
        assert!(all[0].mount_root.ends_with("/snapshots/a-c"));
    }

    #[test]
    fn persistent_mount_root_materialized_and_usage_visible() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let state = tmp.path().join("snapshot-state.pb");
        let mounts_base = tmp.path().join("snapshots");
        fs::create_dir_all(mounts_base.join("base")).expect("create base dir");
        fs::write(mounts_base.join("base/hello.txt"), b"hello").expect("write base file");

        let s = PersistentSnapshotter::open("workspace-p", state).expect("open persistent");
        s.prepare("base", None, BTreeMap::new())
            .expect("prepare base");
        s.commit("base-c", "base").expect("commit base");
        s.view("view-1", "base-c", BTreeMap::new())
            .expect("view snapshot");
        let usage = s.usage("view-1").expect("usage");
        assert!(usage.size_bytes >= 5);
        assert!(usage.inode_count >= 1);
    }
}
