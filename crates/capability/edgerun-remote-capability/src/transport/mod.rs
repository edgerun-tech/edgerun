//! Transport implementations over already-owned streams.

#[cfg(unix)]
mod framed;
mod memory;

#[cfg(unix)]
pub use framed::FramedRemoteTransport;
pub use memory::MemoryRemoteTransport;
