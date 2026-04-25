//! Core types for the edgerun identity-routed mesh network.
//!
//! Every node's identity IS its address: the raw ECDSA P-256 public key
//! (64 bytes, uncompressed x||y).  All mesh frames are signed by the
//! sender's secure hardware and verified against the sender's NodeID
//! embedded in the frame header.

pub mod benchmark;
pub mod discovery;
pub mod mesh_payload;
pub mod router;
pub mod router_benchmark;

mod frame;
mod frame_types;
mod node;
mod routing;

#[cfg(test)]
mod router_tests;

pub use edgerun_hardware_signing::NodeID;
pub use frame::{MeshFrame, MeshFrameHeader};
pub use frame_types::FrameType;
pub use node::{sign_frame, DiscoveryPayload, LocalNode, MeshPeer};
pub use routing::{MeshRoute, MeshRoutingTable};

#[cfg(test)]
mod tests;

// ---------------------------------------------------------------------------
// Re-exports from the router module for backward compatibility with
// code that previously depended on the `edgerun-mesh-router` crate.
// ---------------------------------------------------------------------------

pub use discovery::DiscoveryPacket;
pub use mesh_payload::{
    DeploymentMetrics, MetricsReportPayload, MigrationCompletePayload, MigrationOrderPayload,
};
pub use router::MeshRouter;
