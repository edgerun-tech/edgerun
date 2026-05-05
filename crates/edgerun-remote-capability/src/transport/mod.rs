//! Transport implementations: framed (Unix/TCP) and in-memory.

#[cfg(unix)]
mod framed;
mod memory;

#[cfg(unix)]
pub use framed::FramedRemoteTransport;
#[cfg(unix)]
pub use framed::{accept_tcp, accept_unix};
pub use memory::MemoryRemoteTransport;
