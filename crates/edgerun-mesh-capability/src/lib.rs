//! Bridges `edgerun-remote-capability` onto the edgerun mesh.
//!
//! Provides:
//! - **`MeshCapabilityTransport`** — wraps the mesh to send/receive
//!   `CapabilityRemoteEnvelope` protobuf messages routed by `NodeID`.
//! - **`MeshCapabilityServer`** — hosts local `RemoteCapabilityProvider`
//!   instances and serves inbound requests routed to this node.
//! - **`MeshCapabilityClient`** — initiates sessions with remote nodes
//!   by NodeID, sending invocations and receiving results.
//!
//! The mesh handles delivery — this layer only deals in capability
//! protocol messages wrapped in mesh frames.

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

#[cfg(target_os = "none")]
extern crate self as std;

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

mod client;
mod dispatcher;
mod inbox;
mod server;
mod transport;

use crate::prelude::v1::*;
use edgerun_capabilities::CapabilityError;
use edgerun_hardware_signing::NodeID;
use edgerun_mesh_link::MeshLink;
use edgerun_proto::edgerun::v0::capability_runtime::{
    capability_remote_envelope, CapabilityRemoteEnvelope,
};
use edgerun_remote_capability::{RemoteCapabilityProvider, RemoteCapabilityTransport};
use prost::Message;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

pub use client::MeshCapabilityClient;
pub use dispatcher::MeshEnvelopeDispatcher;
pub use inbox::EnvelopeInbox;
pub use server::MeshCapabilityServer;
pub use transport::MeshCapabilityTransport;

/// Shared outbound queue: (destination NodeID, serialized protobuf payload).
/// Used by `MeshCapabilityTransport::send()` to push frames that the daemon
/// will drain, encrypt, and send.
///
/// Thread-safe via `Arc<Mutex<>>` to allow future multi-threaded mesh daemons.
pub type OutboundQueue = Arc<Mutex<VecDeque<(NodeID, Vec<u8>)>>>;

#[cfg(test)]
mod tests;
