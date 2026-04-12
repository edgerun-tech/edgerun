//! Core types for the edgerun identity-routed mesh network.
//!
//! Every node's identity IS its address: the raw ECDSA P-256 public key
//! (64 bytes, uncompressed x||y).  All mesh frames are signed by the
//! sender's secure hardware and verified against the sender's NodeID
//! embedded in the frame header.

pub mod benchmark;
pub mod discovery;
pub mod router;
pub mod router_tests;
pub mod router_benchmark;

mod frame_types;
mod frame;
mod routing;
mod node;

pub use edgerun_hardware_signing::NodeID;
pub use frame_types::FrameType;
pub use frame::{MeshFrameHeader, MeshFrame};
pub use routing::{MeshRoute, MeshRoutingTable};
pub use node::{MeshPeer, DiscoveryPayload, LocalNode, sign_frame};

#[cfg(test)]
mod tests;


// ---------------------------------------------------------------------------
// Re-exports from the router module for backward compatibility with
// code that previously depended on the `edgerun-mesh-router` crate.
// ---------------------------------------------------------------------------

pub use discovery::DiscoveryPacket;
pub use router::MeshRouter;
