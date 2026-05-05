//! Mesh daemon orchestration facade.
//!
//! This crate exposes the concrete rkyv mesh components used by node daemons:
//! link transport, session management, and capability envelope queues.

#![no_std]

extern crate alloc;

#[cfg(not(target_os = "none"))]
extern crate std;

pub use edgerun_mesh as mesh;
pub use edgerun_mesh_capability as capability;
pub use edgerun_mesh_link as link;
pub use edgerun_mesh_session as session;

pub use capability::{EnvelopeInbox, OutboundQueue};
pub use session::{MeshSession, SessionManager};
