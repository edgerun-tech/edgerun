pub mod config;
pub mod wire;
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const SUBJECT_PREFIX_RUNTIME_TASK: &str = "edgerun.runtime.task";
pub const SUBJECT_PREFIX_RUNTIME_IO: &str = "edgerun.runtime.io";
pub const SUBJECT_PREFIX_STORAGE_SNAPSHOT: &str = "edgerun.storage.snapshot";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTaskEventKind {
    Created,
    Started,
    Exited,
    Oom,
    Killed,
    ExecStarted,
    ExecExited,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeTaskEvent {
    pub schema_version: u32,
    pub namespace: String,
    pub task_id: String,
    pub event_id: String,
    pub kind: RuntimeTaskEventKind,
    pub ts_unix_ms: u64,
    pub pid: Option<u32>,
    pub exit_code: Option<u32>,
    pub detail: Option<String>,
}

impl RuntimeTaskEvent {
    pub fn validate(&self) -> Result<(), RuntimeProtoError> {
        if self.schema_version != 1 {
            return Err(RuntimeProtoError::InvalidField(
                "schema_version must be 1".to_string(),
            ));
        }
        if self.namespace.trim().is_empty() {
            return Err(RuntimeProtoError::InvalidField(
                "namespace must not be empty".to_string(),
            ));
        }
        if self.task_id.trim().is_empty() {
            return Err(RuntimeProtoError::InvalidField(
                "task_id must not be empty".to_string(),
            ));
        }
        if self.event_id.trim().is_empty() {
            return Err(RuntimeProtoError::InvalidField(
                "event_id must not be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StreamKind {
    Stdout,
    Stderr,
    Stdin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeIoEvent {
    pub schema_version: u32,
    pub namespace: String,
    pub task_id: String,
    pub exec_id: Option<String>,
    pub stream: StreamKind,
    pub chunk_b64: String,
    pub ts_unix_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FsIntentKind {
    Lookup,
    Open,
    CreateDir,
    Remove,
    Rename,
    Symlink,
    Read,
    Write,
    Truncate,
    Chmod,
    Chown,
    Stat,
    MountAttach,
    MountDetach,
    SetRoot,
    PathAccessCheck,
    FsConfigSet,
    FsConfigCreate,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsIntentEvent {
    pub schema_version: u32,
    pub workspace_id: String,
    pub lane: String,
    pub intent_id: String,
    pub kind: FsIntentKind,
    pub path: String,
    pub base_revision: String,
    pub actor_id: String,
    pub ts_unix_ms: u64,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SnapshotMaterializedEvent {
    pub schema_version: u32,
    pub workspace_id: String,
    pub lane: String,
    pub snapshot_key: String,
    pub cursor_offset: u64,
    pub root_hash_blake3_hex: String,
    pub events_compacted: u64,
    pub ts_unix_ms: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum RuntimeProtoError {
    #[error("invalid field: {0}")]
    InvalidField(String),
}

pub fn runtime_task_subject(namespace: &str, task_id: &str, lane: &str) -> String {
    format!(
        "{SUBJECT_PREFIX_RUNTIME_TASK}.{}.{}.{}",
        sanitize_subject_token(namespace),
        sanitize_subject_token(task_id),
        sanitize_subject_token(lane),
    )
}

pub fn runtime_io_subject(namespace: &str, task_id: &str, stream: StreamKind) -> String {
    let stream_token = match stream {
        StreamKind::Stdout => "stdout",
        StreamKind::Stderr => "stderr",
        StreamKind::Stdin => "stdin",
    };
    format!(
        "{SUBJECT_PREFIX_RUNTIME_IO}.{}.{}.{}",
        sanitize_subject_token(namespace),
        sanitize_subject_token(task_id),
        stream_token,
    )
}

pub fn snapshot_subject(snapshot_key: &str, lane: &str) -> String {
    format!(
        "{SUBJECT_PREFIX_STORAGE_SNAPSHOT}.{}.{}",
        sanitize_subject_token(snapshot_key),
        sanitize_subject_token(lane),
    )
}

fn sanitize_subject_token(token: &str) -> String {
    token
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_runtime_subject() {
        let subject = runtime_task_subject("k8s.io", "abc.123", "event");
        assert_eq!(subject, "edgerun.runtime.task.k8s_io.abc_123.event");
    }

    #[test]
    fn validates_runtime_event() {
        let evt = RuntimeTaskEvent {
            schema_version: 1,
            namespace: "default".to_string(),
            task_id: "task-1".to_string(),
            event_id: "evt-1".to_string(),
            kind: RuntimeTaskEventKind::Created,
            ts_unix_ms: 1,
            pid: None,
            exit_code: None,
            detail: None,
        };
        assert!(evt.validate().is_ok());
    }
}
