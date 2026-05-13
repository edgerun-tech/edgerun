//! edgerun storage layer — durable event log, encrypted blobs, and rebuildable indexes.
//!
//! Implements the reference storage profile from the protocol spec (§6.1–§6.3, §19.9):
//!
//! - **Event log**: durable append/read/scan for already-signed stream events
//! - **File indexes**: binary append-only logs with in-memory HashMaps (rebuildable from event log)
//! - **Blob store**: AES-GCM encrypted ciphertext on filesystem, content-addressed
//!
//! ## Security invariants (§6.2)
//! - Every persisted blob is encrypted at rest
//! - Every persisted blob names at least one recipient
//! - No plaintext blob persistence path
//!
//! ## Rebuildability (§6.3)
//! File indexes can be deleted and rebuilt from the event log + encrypted blobs.
//! Loss of indexes does not invalidate already stored records.

#![no_std]

extern crate alloc;
#[cfg(target_os = "none")]
extern crate self as std;
#[cfg(not(target_os = "none"))]
extern crate std;

#[path = "std.rs"]
mod std_compat;
pub use std_compat::*;

pub mod blobs;
pub mod block;
pub mod core;
pub mod credentials;
pub mod derived_db;
pub mod edgefs;
pub mod error;
pub mod event_loop;
pub mod file_index;
pub mod fs;
pub mod materializer;
pub mod mem;
pub mod store;
pub mod stream;
#[cfg(test)]
mod test_support;

pub use blobs::{BlobEntry, BlobKeySource, BlobStore, BlobStoreConfig, blob_file_path};
pub use block::BlockStreamStore;
pub use block::{
    BlockEventLog, BlockStorage, ExFatInfo, ExtInfo, FatDirectoryEntry, FatError, FatInfo,
    FatReadOnly, FileSystemDetails, FileSystemKind, FileSystemProbe, FileSystemProbeError,
    InMemoryBlockDevice, Iso9660Info, PartitionBlockDevice, PartitionEntry, PartitionError,
    PartitionKind, PartitionTable, PartitionTableKind, detect_partitions, probe_filesystem,
};
pub use core::{
    AppendReceipt, DurableStreamWriter, EventLog, canonical_event_hash, derive_logical_object_id,
    derive_representation_id,
};
pub use credentials::CredentialStore;
pub use derived_db::*;
pub use edgefs::{
    DeviceId, DirEntry, EdgeFs, EdgeFsError, EdgeFsInfo, EntryKind, EntryRef, FileMeta,
    Result as EdgeFsResult,
};
pub use error::StorageError;
pub use event_loop::{
    CredentialDeleteHandler, CredentialHandler, DispatchContext, DurableEventAppender,
    EventHandler, EventLoopBuilder, FetchHandler, PeerDiscoveryHandler, PeerStatusHandler,
};
pub use file_index::{EventIndexEntry, FetchEntry, FileIndex, ReplayEntry};
pub use materializer::OpEventType;
pub use mem::{MemContentStore, MemEventLog};
pub use store::{CommandReplayResult, ControllerSet, NodeStore, NodeStoreConfig, ObjectResult};
pub use stream::{
    EventDraft, StreamError, StreamId, StreamWriter, build_signed_event, build_unsigned_event,
    compute_event_hash, event_signable_bytes, genesis_event, genesis_event_with_payload,
    sign_event, validate_stream, verify_event,
};
