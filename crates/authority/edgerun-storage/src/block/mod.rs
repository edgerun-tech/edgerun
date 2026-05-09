//! Block-backed storage backend interfaces and implementations.

use crate::prelude::v1::*;

pub mod event_log;
pub mod fat;
pub mod filesystem;
pub mod partition;
pub mod store;
pub mod virtual_disk;

pub use event_log::{BlockEventLog, BlockStorage, InMemoryBlockDevice};
pub use fat::{FatDirectoryEntry, FatError, FatReadOnly};
pub use filesystem::{
    ExFatInfo, ExtInfo, FatInfo, FileSystemDetails, FileSystemKind, FileSystemProbe,
    FileSystemProbeError, Iso9660Info, probe_filesystem,
};
pub use partition::{
    PartitionBlockDevice, PartitionEntry, PartitionError, PartitionKind, PartitionTable,
    PartitionTableKind, detect_partitions,
};
pub use store::BlockStreamStore;
pub use virtual_disk::VirtualDiskBlockStorage;
