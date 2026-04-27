//! Transport implementations: framed (Unix/TCP) and in-memory.

#[cfg(not(target_os = "none"))]
mod framed;
mod memory;

#[cfg(not(target_os = "none"))]
pub use framed::FramedRemoteTransport;
#[cfg(not(target_os = "none"))]
pub use framed::{accept_tcp, accept_unix};
pub use memory::MemoryRemoteTransport;
