//! Core types for the edgerun identity-routed mesh network.
//!
//! Every node's identity IS its address: the raw ECDSA P-256 public key
//! (64 bytes, uncompressed x||y).  All mesh frames are signed by the
//! sender's secure hardware and verified against the sender's NodeID
//! embedded in the frame header.

#![no_std]

extern crate alloc;
#[cfg(target_os = "none")]
extern crate self as std;
#[cfg(not(target_os = "none"))]
extern crate std;

pub mod prelude {
    pub mod v1 {
        pub use alloc::borrow::ToOwned;
        pub use alloc::boxed::Box;
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2021::*;
    }
}

pub mod collections {
    pub type HashMap<K, V> = alloc::collections::BTreeMap<K, V>;
    pub type HashSet<T> = alloc::collections::BTreeSet<T>;
}

pub mod vec {
    pub use alloc::vec::Vec;
}

pub mod string {
    pub use alloc::string::{String, ToString};
}

pub mod option {
    pub use core::option::Option;
}

pub use alloc::format;
pub use core::{iter, result};

#[cfg(not(target_os = "none"))]
pub mod benchmark;
pub mod discovery;
pub mod link;
pub mod mesh_payload;
pub mod router;
#[cfg(not(target_os = "none"))]
pub mod router_benchmark;
pub mod session;

mod frame;
mod frame_types;
mod node;
mod routing;

#[cfg(test)]
mod router_tests;

pub use discovery::DiscoveryPacket;
pub use edgerun_hardware_signing::NodeID;
pub use frame::{
    MESH_MAX_PAYLOAD_LEN, MeshFrame, MeshFrameAdmission, MeshFrameAdmissionPolicy, MeshFrameHeader,
    MeshFrameReject, inspect_mesh_frame_wire, inspect_mesh_frame_wire_for,
};
pub use frame_types::FrameType;
pub use link::{IpTunnel, MeshLink, MulticastSocket, RawEthernetSocket, UdpBroadcastSocket};
pub use node::{DiscoveryPayload, LocalNode, MeshPeer, sign_frame};
pub use router::MeshRouter;
pub use routing::{MeshRoute, MeshRoutingTable};
pub use session::{
    EphemeralSecret, HandshakeAccept, HandshakeInit, MeshSession, SessionError, SessionManager,
};

#[cfg(test)]
mod tests;
