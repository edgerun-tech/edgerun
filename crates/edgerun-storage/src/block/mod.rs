//! Block-backed storage backend interfaces and implementations.

pub mod event_log;
pub mod store;

pub use event_log::{BlockEventLog, BlockStorage, InMemoryBlockDevice};
pub use store::BlockStreamStore;
