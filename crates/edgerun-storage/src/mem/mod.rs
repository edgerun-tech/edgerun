//! In-memory storage backends for local testing and early bare-metal bring-up.

pub mod content_store;
pub mod event_log;

pub use content_store::MemContentStore;
pub use event_log::MemEventLog;
