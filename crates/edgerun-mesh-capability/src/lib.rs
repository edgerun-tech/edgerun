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

mod inbox;
mod transport;
mod dispatcher;
mod server;
mod client;

use edgerun_hardware_signing::NodeID;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use edgerun_proto::edgerun::v0::capability_runtime::{
    capability_remote_envelope, CapabilityRemoteEnvelope,
};
use edgerun_capabilities::CapabilityError;
use edgerun_mesh_link::MeshLink;
use edgerun_remote_capability::{RemoteCapabilityProvider, RemoteCapabilityTransport};
use prost::Message;

pub use inbox::EnvelopeInbox;
pub use transport::MeshCapabilityTransport;
pub use dispatcher::MeshEnvelopeDispatcher;
pub use server::MeshCapabilityServer;
pub use client::MeshCapabilityClient;

/// Shared outbound queue: (destination NodeID, serialized protobuf payload).
/// Used by `MeshCapabilityTransport::send()` to push frames that the daemon
/// will drain, encrypt, and send.
///
/// Thread-safe via `Arc<Mutex<>>` to allow future multi-threaded mesh daemons.
pub type OutboundQueue = Arc<Mutex<VecDeque<(NodeID, Vec<u8>)>>>;

#[cfg(test)]
mod tests;
