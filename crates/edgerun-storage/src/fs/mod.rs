//! Filesystem-backed storage backend.

pub mod content_store;
pub mod event_log;

pub use content_store::FsContentStore;
pub use event_log::{
    open_stream_file, read_event_at, scan_event_logs, write_event_to_file, FsEventLog,
};
