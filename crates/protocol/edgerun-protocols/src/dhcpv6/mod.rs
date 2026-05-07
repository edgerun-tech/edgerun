//! DHCPv6 message codec and server state machine.

pub mod duid;
pub mod lease;
pub mod message;
pub mod options;
pub mod server_core;

pub mod io {
    use alloc::format;
    use alloc::string::String;
    use core::fmt;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ErrorKind {
        InvalidInput,
        InvalidData,
        UnexpectedEof,
        WouldBlock,
        TimedOut,
        NotFound,
        Other,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct Error {
        kind: ErrorKind,
        message: String,
    }

    impl Error {
        pub fn new(kind: ErrorKind, message: impl fmt::Display) -> Self {
            Self {
                kind,
                message: format!("{message}"),
            }
        }

        pub fn kind(&self) -> ErrorKind {
            self.kind
        }
    }

    impl fmt::Display for Error {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(&self.message)
        }
    }
}

pub use duid::{Duid, DuidType};
pub use lease::{Dhcpv6Lease, LeasePool, LeaseState, PrefixLease};
pub use message::{Dhcpv6Message, Dhcpv6MsgType, TransactionId};
pub use options::{Dhcpv6Option, IaNaOption, IaPdOption, IaTaOption};
pub use server_core::{Dhcpv6Datagram, Dhcpv6ServerConfig, Dhcpv6ServerCore};
