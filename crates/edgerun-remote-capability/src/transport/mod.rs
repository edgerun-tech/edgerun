//! Transport implementations: framed (Unix/TCP) and in-memory.

mod framed;
mod memory;

pub use framed::FramedRemoteTransport;
pub use framed::{accept_unix, accept_tcp};
pub use memory::MemoryRemoteTransport;
