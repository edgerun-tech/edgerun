//! Filesystem-backed storage backend.

use crate::prelude::v1::*;

pub mod content_store;
pub mod event_log;

pub use crate::std_compat::fs::{
    create_dir_all, read, read_dir, read_to_string, remove_dir_all, remove_file, write, DirEntry,
    File, Metadata, OpenOptions, ReadDir,
};
pub use content_store::FsContentStore;
pub use event_log::{
    open_stream_file, read_event_at, scan_event_logs, write_event_to_file, FsEventLog,
};
