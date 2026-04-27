//! Minimal std-shaped compatibility surface backed by core, alloc, and edgerun-bare-rt.

pub use core::{fmt, result, str};

pub mod collections {
    pub type HashMap<K, V> = alloc::collections::BTreeMap<K, V>;
}

pub mod io {
    use alloc::format;
    use alloc::string::String;
    use core::fmt;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum ErrorKind {
        InvalidInput,
        InvalidData,
        WouldBlock,
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

    impl core::error::Error for Error {}

    pub type Result<T> = core::result::Result<T, Error>;
}

pub mod net {
    pub use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4};
}

pub mod sync {
    pub use alloc::sync::Arc;
    pub use edgerun_bare_rt::{AsyncMutex as Mutex, MutexGuard};
}

pub mod time {
    pub use edgerun_bare_rt::Duration;
}
