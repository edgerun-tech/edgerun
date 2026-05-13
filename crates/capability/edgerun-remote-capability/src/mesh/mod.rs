//! Capability envelope queues for the edgerun mesh.
//!
//! Mesh capability code queues and drains shared `CapabilityRemoteEnvelope`
//! values across the rkyv-normalized capability boundary.

pub mod prelude {
    pub mod v1 {
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2024::*;
    }
}

pub mod collections {
    pub use alloc::collections::{BTreeMap as HashMap, BTreeSet as HashSet, VecDeque};
}

pub mod sync {
    pub use alloc::sync::Arc;
    use core::cell::{RefCell, RefMut};

    #[derive(Debug)]
    pub struct Mutex<T> {
        inner: RefCell<T>,
    }

    unsafe impl<T: Send> Send for Mutex<T> {}
    unsafe impl<T: Send> Sync for Mutex<T> {}

    impl<T> Mutex<T> {
        #[must_use]
        pub const fn new(value: T) -> Self {
            Self {
                inner: RefCell::new(value),
            }
        }

        pub fn lock(&self) -> core::result::Result<RefMut<'_, T>, ()> {
            Ok(self.inner.borrow_mut())
        }
    }
}

pub mod vec {
    pub use alloc::vec::*;
}

pub mod string {
    pub use alloc::string::*;
}

pub mod option {
    pub use core::option::*;
}

pub mod result {
    pub use core::result::*;
}

mod inbox;

use self::collections::VecDeque;
use self::sync::{Arc, Mutex};
use alloc::vec::Vec;
use edgerun_hardware_signing::NodeID;
use edgerun_protocols::core_protocol::protocol::capability_runtime::CapabilityRemoteEnvelope;

pub use inbox::EnvelopeInbox;

/// Shared outbound queue: (destination NodeID, serialized native payload).
/// Used to push frames that the daemon
/// will drain, encrypt, and send.
///
/// Thread-safe via `Arc<Mutex<>>` to allow future multi-threaded mesh daemons.
pub type OutboundQueue = Arc<Mutex<VecDeque<(NodeID, Vec<u8>)>>>;
