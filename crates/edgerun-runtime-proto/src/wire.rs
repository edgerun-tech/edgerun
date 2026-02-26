// SPDX-License-Identifier: Apache-2.0

use prost::Message;

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum LifecycleCommandV1 {
    Unspecified = 0,
    Create = 1,
    Start = 2,
    Exit = 3,
    Kill = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum TaskServiceOpV1 {
    Unspecified = 0,
    Create = 1,
    Start = 2,
    State = 3,
    Kill = 4,
    Delete = 5,
    Wait = 6,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum SnapshotOpV1 {
    Unspecified = 0,
    Health = 1,
    Prepare = 2,
    View = 3,
    Commit = 4,
    Remove = 5,
    Walk = 6,
    Cleanup = 7,
    Stat = 8,
    Mounts = 9,
    Usage = 10,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
#[repr(i32)]
pub enum SnapshotKindV1 {
    Unspecified = 0,
    Active = 1,
    View = 2,
    Committed = 3,
}

#[derive(Clone, PartialEq, Message)]
pub struct ShimRequestV1 {
    #[prost(uint32, tag = "1")]
    pub schema_version: u32,
    #[prost(oneof = "shim_request_v1::Op", tags = "2, 3, 4, 5, 6")]
    pub op: Option<shim_request_v1::Op>,
}

pub mod shim_request_v1 {
    use prost::Oneof;

    use super::{
        EmitEventRequestV1, HealthRequestV1, PrintSubjectRequestV1, TaskServiceRequestV1,
        TaskStateRequestV1,
    };

    #[derive(Clone, PartialEq, Oneof)]
    pub enum Op {
        #[prost(message, tag = "2")]
        Health(HealthRequestV1),
        #[prost(message, tag = "3")]
        PrintSubject(PrintSubjectRequestV1),
        #[prost(message, tag = "4")]
        EmitEvent(EmitEventRequestV1),
        #[prost(message, tag = "5")]
        TaskState(TaskStateRequestV1),
        #[prost(message, tag = "6")]
        TaskApi(TaskServiceRequestV1),
    }
}

#[derive(Clone, PartialEq, Message)]
pub struct HealthRequestV1 {}

#[derive(Clone, PartialEq, Message)]
pub struct PrintSubjectRequestV1 {
    #[prost(string, tag = "1")]
    pub namespace: String,
    #[prost(string, tag = "2")]
    pub task_id: String,
    #[prost(string, tag = "3")]
    pub lane: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct EmitEventRequestV1 {
    #[prost(string, tag = "1")]
    pub namespace: String,
    #[prost(string, tag = "2")]
    pub task_id: String,
    #[prost(string, tag = "3")]
    pub event_id: String,
    #[prost(enumeration = "LifecycleCommandV1", tag = "4")]
    pub command: i32,
    #[prost(uint32, optional, tag = "5")]
    pub pid: Option<u32>,
    #[prost(uint32, optional, tag = "6")]
    pub exit_code: Option<u32>,
    #[prost(string, tag = "7")]
    pub detail: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct TaskStateRequestV1 {
    #[prost(string, tag = "1")]
    pub namespace: String,
    #[prost(string, tag = "2")]
    pub task_id: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct ShimResponseV1 {
    #[prost(uint32, tag = "1")]
    pub schema_version: u32,
    #[prost(bool, tag = "2")]
    pub ok: bool,
    #[prost(string, tag = "3")]
    pub message: String,
    #[prost(uint64, optional, tag = "4")]
    pub offset: Option<u64>,
    #[prost(string, tag = "5")]
    pub subject: String,
    #[prost(string, tag = "6")]
    pub state: String,
    #[prost(bool, tag = "7")]
    pub pending: bool,
    #[prost(uint32, optional, tag = "8")]
    pub pid: Option<u32>,
    #[prost(uint32, optional, tag = "9")]
    pub exit_code: Option<u32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct TaskServiceRequestV1 {
    #[prost(string, tag = "1")]
    pub namespace: String,
    #[prost(string, tag = "2")]
    pub task_id: String,
    #[prost(enumeration = "TaskServiceOpV1", tag = "3")]
    pub op: i32,
    #[prost(uint32, optional, tag = "4")]
    pub signal: Option<u32>,
    #[prost(string, tag = "5")]
    pub runtime_name: String,
    #[prost(string, tag = "6")]
    pub runtime_selector_source: String,
    #[prost(string, tag = "7")]
    pub bundle_path: String,
    #[prost(string, tag = "8")]
    pub stdin_path: String,
    #[prost(string, tag = "9")]
    pub stdout_path: String,
    #[prost(string, tag = "10")]
    pub stderr_path: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct RuntimeTaskEventV1 {
    #[prost(uint32, tag = "1")]
    pub schema_version: u32,
    #[prost(string, tag = "2")]
    pub namespace: String,
    #[prost(string, tag = "3")]
    pub task_id: String,
    #[prost(string, tag = "4")]
    pub event_id: String,
    #[prost(string, tag = "5")]
    pub kind: String,
    #[prost(uint64, tag = "6")]
    pub ts_unix_ms: u64,
    #[prost(uint32, optional, tag = "7")]
    pub pid: Option<u32>,
    #[prost(uint32, optional, tag = "8")]
    pub exit_code: Option<u32>,
    #[prost(string, tag = "9")]
    pub detail: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct SnapshotMaterializedEventV1 {
    #[prost(uint32, tag = "1")]
    pub schema_version: u32,
    #[prost(string, tag = "2")]
    pub workspace_id: String,
    #[prost(string, tag = "3")]
    pub lane: String,
    #[prost(string, tag = "4")]
    pub snapshot_key: String,
    #[prost(uint64, tag = "5")]
    pub cursor_offset: u64,
    #[prost(string, tag = "6")]
    pub root_hash_blake3_hex: String,
    #[prost(uint64, tag = "7")]
    pub events_compacted: u64,
    #[prost(uint64, tag = "8")]
    pub ts_unix_ms: u64,
}

#[derive(Clone, PartialEq, Message)]
pub struct SnapshotRequestV1 {
    #[prost(uint32, tag = "1")]
    pub schema_version: u32,
    #[prost(enumeration = "SnapshotOpV1", tag = "2")]
    pub op: i32,
    #[prost(string, tag = "3")]
    pub key: String,
    #[prost(string, tag = "4")]
    pub parent: String,
    #[prost(string, tag = "5")]
    pub name: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct LabelPairV1 {
    #[prost(string, tag = "1")]
    pub key: String,
    #[prost(string, tag = "2")]
    pub value: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct MountV1 {
    #[prost(string, tag = "1")]
    pub r#type: String,
    #[prost(string, tag = "2")]
    pub source: String,
    #[prost(string, repeated, tag = "3")]
    pub options: Vec<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct SnapshotItemV1 {
    #[prost(string, tag = "1")]
    pub key: String,
    #[prost(string, tag = "2")]
    pub parent: String,
    #[prost(enumeration = "SnapshotKindV1", tag = "3")]
    pub kind: i32,
    #[prost(message, repeated, tag = "4")]
    pub labels: Vec<LabelPairV1>,
    #[prost(uint64, tag = "5")]
    pub size_bytes: u64,
    #[prost(uint64, tag = "6")]
    pub inode_count: u64,
    #[prost(string, tag = "7")]
    pub mount_root: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct SnapshotListV1 {
    #[prost(message, repeated, tag = "1")]
    pub items: Vec<SnapshotItemV1>,
}

#[derive(Clone, PartialEq, Message)]
pub struct MountListV1 {
    #[prost(message, repeated, tag = "1")]
    pub items: Vec<MountV1>,
}

#[derive(Clone, PartialEq, Message)]
pub struct SnapshotUsageV1 {
    #[prost(uint64, tag = "1")]
    pub size_bytes: u64,
    #[prost(uint64, tag = "2")]
    pub inode_count: u64,
}

#[derive(Clone, PartialEq, Message)]
pub struct RemoveResultV1 {
    #[prost(string, tag = "1")]
    pub key: String,
}

#[derive(Clone, PartialEq, Message)]
pub struct CleanupResultV1 {
    #[prost(uint64, tag = "1")]
    pub dropped: u64,
}

#[derive(Clone, PartialEq, Message)]
pub struct SnapshotResponseV1 {
    #[prost(uint32, tag = "1")]
    pub schema_version: u32,
    #[prost(bool, tag = "2")]
    pub ok: bool,
    #[prost(string, tag = "3")]
    pub message: String,
    #[prost(oneof = "snapshot_response_v1::Body", tags = "4, 5, 6, 7, 8, 9")]
    pub body: Option<snapshot_response_v1::Body>,
}

pub mod snapshot_response_v1 {
    use prost::Oneof;

    use super::{
        CleanupResultV1, MountListV1, RemoveResultV1, SnapshotItemV1, SnapshotListV1,
        SnapshotUsageV1,
    };

    #[derive(Clone, PartialEq, Oneof)]
    pub enum Body {
        #[prost(message, tag = "4")]
        Snapshot(SnapshotItemV1),
        #[prost(message, tag = "5")]
        Snapshots(SnapshotListV1),
        #[prost(message, tag = "6")]
        Mounts(MountListV1),
        #[prost(message, tag = "7")]
        Usage(SnapshotUsageV1),
        #[prost(message, tag = "8")]
        Removed(RemoveResultV1),
        #[prost(message, tag = "9")]
        Cleanup(CleanupResultV1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shim_wire_roundtrip() {
        let req = ShimRequestV1 {
            schema_version: 1,
            op: Some(shim_request_v1::Op::Health(HealthRequestV1 {})),
        };
        let mut encoded = Vec::new();
        req.encode(&mut encoded).expect("encode");
        let decoded = ShimRequestV1::decode(encoded.as_slice()).expect("decode");
        assert_eq!(decoded.schema_version, 1);
        assert!(matches!(decoded.op, Some(shim_request_v1::Op::Health(_))));
    }

    #[test]
    fn snapshot_response_roundtrip() {
        let resp = SnapshotResponseV1 {
            schema_version: 1,
            ok: true,
            message: "ok".to_string(),
            body: Some(snapshot_response_v1::Body::Cleanup(CleanupResultV1 {
                dropped: 2,
            })),
        };
        let mut encoded = Vec::new();
        resp.encode(&mut encoded).expect("encode");
        let decoded = SnapshotResponseV1::decode(encoded.as_slice()).expect("decode");
        assert!(decoded.ok);
        assert!(matches!(
            decoded.body,
            Some(snapshot_response_v1::Body::Cleanup(CleanupResultV1 {
                dropped: 2
            }))
        ));
    }
}
