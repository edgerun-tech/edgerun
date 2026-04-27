//! Shared storage primitives.
//!
//! This module holds protocol-level storage semantics that should be shared by
//! filesystem and bare-metal backends.

pub mod cas;
pub mod event_log;

pub use cas::{
    derive_logical_object_id, derive_representation_id, ContentStore, ObjectBytes, ObjectIds,
    ObjectPresence,
};
pub use event_log::{
    canonical_event_hash, encode_event_frame, AppendReceipt, EventLocation, EventLog, ScannedEvent,
    StreamHead,
};
