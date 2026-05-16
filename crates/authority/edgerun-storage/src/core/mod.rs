//! Shared storage primitives.
//!
//! This module holds protocol-level storage semantics that should be shared by
//! filesystem and bare-metal backends.

use crate::prelude::v1::*;

pub mod cas;
pub mod durable_stream;
pub mod event_log;

pub use cas::{
    ContentStore, ObjectBytes, ObjectIds, ObjectPresence, derive_logical_object_id,
    derive_representation_id,
};
pub use durable_stream::DurableStreamWriter;
pub use event_log::{
    AppendReceipt, EventLocation, EventLog, ScannedEvent, StreamHead, canonical_event_hash,
    encode_event_frame, validate_event_location,
};
