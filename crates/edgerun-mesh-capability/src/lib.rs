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

mod client;
mod dispatcher;
mod inbox;
mod server;
mod transport;

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
