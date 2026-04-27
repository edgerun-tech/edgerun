//! THE EVENT LOG IS THE STATE.
//!
//! edgerun storage layer — durable event log, encrypted blobs, and rebuildable indexes.
//!
//! Implements the reference storage profile from the protocol spec (§6.1–§6.3, §19.9):
//!
//! - **Event log**: append-only protobuf records on the filesystem (authoritative truth)
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

pub mod blobs;
pub mod block;
pub mod core;
pub mod credentials;
pub mod error;
pub mod event_loop;
pub mod file_index;
pub mod fs;
pub mod mem;
pub mod store;

pub use blobs::{blob_file_path, BlobEntry, BlobKeySource, BlobStore, BlobStoreConfig};
pub use block::BlockStreamStore;
pub use block::{BlockEventLog, BlockStorage, InMemoryBlockDevice};
pub use core::{canonical_event_hash, derive_logical_object_id, derive_representation_id};
pub use credentials::CredentialStore;
pub use error::StorageError;
pub use event_loop::{
    CredentialDeleteHandler, CredentialHandler, DispatchContext, EventHandler, EventLoopBuilder,
    EventWriter, FetchHandler, OpEventType, PeerDiscoveryHandler, PeerStatusHandler,
};
pub use file_index::{EventIndexEntry, FetchEntry, FileIndex, ReplayEntry, WorkAccountingRecord};
pub use mem::{MemContentStore, MemEventLog};
pub use store::{CommandReplayResult, ControllerSet, NodeStore, NodeStoreConfig, ObjectResult};
