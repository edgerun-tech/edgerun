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
pub mod credentials;
pub mod error;
pub mod event_loop;
pub mod file_index;
pub mod store;

pub use error::StorageError;
pub use blobs::{BlobStore, BlobKeySource, BlobEntry, BlobStoreConfig, blob_file_path};
pub use credentials::CredentialStore;
pub use event_loop::{EventWriter, EventLoopBuilder, DispatchContext, EventHandler, OpEventType, FetchHandler, PeerDiscoveryHandler, PeerStatusHandler, CredentialHandler, CredentialDeleteHandler};
pub use file_index::{FileIndex, EventIndexEntry, ReplayEntry, FetchEntry, WorkAccountingRecord};
pub use store::{NodeStore, NodeStoreConfig, CommandReplayResult, ObjectResult, ControllerSet};
