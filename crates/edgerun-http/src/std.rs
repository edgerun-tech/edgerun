//! Minimal std-shaped compatibility surface for bare HTTP builds.

pub mod prelude {
    pub mod v1 {
        pub use alloc::boxed::Box;
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2021::*;
    }
}

pub use core::{any, borrow, cmp, convert, fmt, future, hash, mem, option, pin, result, str, task};

pub mod collections {
    pub use alloc::collections::{BTreeMap, BTreeSet, VecDeque};

    pub type HashMap<K, V> = alloc::collections::BTreeMap<K, V>;
    pub type HashSet<T> = alloc::collections::BTreeSet<T>;

    pub mod hash_map {
        pub use alloc::collections::btree_map::Entry;
    }
}

pub mod error {
    pub use core::error::Error;
}

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
        WriteZero,
        BrokenPipe,
        ConnectionRefused,
        ConnectionReset,
        NotConnected,
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

        pub fn other(message: impl fmt::Display) -> Self {
            Self::new(ErrorKind::Other, message)
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

    impl From<edgerun_bare_rt::IoError> for Error {
        fn from(value: edgerun_bare_rt::IoError) -> Self {
            match value {
                edgerun_bare_rt::IoError::UnexpectedEof => {
                    Self::new(ErrorKind::UnexpectedEof, value)
                }
                edgerun_bare_rt::IoError::WriteZero => Self::new(ErrorKind::WriteZero, value),
                edgerun_bare_rt::IoError::Other(_) => Self::new(ErrorKind::Other, value),
            }
        }
    }

    pub type Result<T> = core::result::Result<T, Error>;
}

pub mod net {
    use super::io;
    use alloc::string::{String, ToString};
    pub use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4};

    pub trait ToSocketAddrs {
        type Iter: Iterator<Item = SocketAddr>;
        fn to_socket_addrs(&self) -> io::Result<Self::Iter>;
    }

    impl ToSocketAddrs for SocketAddr {
        type Iter = core::option::IntoIter<SocketAddr>;

        fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
            Ok(Some(*self).into_iter())
        }
    }

    impl ToSocketAddrs for &str {
        type Iter = core::option::IntoIter<SocketAddr>;

        fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
            self.parse::<SocketAddr>()
                .map(Some)
                .map(Option::into_iter)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
        }
    }

    impl ToSocketAddrs for String {
        type Iter = core::option::IntoIter<SocketAddr>;

        fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
            self.as_str().to_socket_addrs()
        }
    }

    impl ToSocketAddrs for (&str, u16) {
        type Iter = core::option::IntoIter<SocketAddr>;

        fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
            let ip = self
                .0
                .parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            Ok(Some(SocketAddr::new(IpAddr::V4(ip), self.1)).into_iter())
        }
    }

    pub struct UdpSocket(edgerun_bare_rt::UdpSocket);

    impl UdpSocket {
        pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
            let addr = addr
                .to_socket_addrs()?
                .next()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no socket address"))?;
            let mut socket = edgerun_bare_rt::UdpSocket::new();
            socket
                .bind(to_bare_addr(addr))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            Ok(Self(socket))
        }
    }

    fn to_bare_addr(addr: SocketAddr) -> edgerun_bare_rt::SocketAddr {
        match addr {
            SocketAddr::V4(v4) => {
                edgerun_bare_rt::SocketAddr(u32::from_be_bytes(v4.ip().octets()), v4.port())
            }
            SocketAddr::V6(_) => edgerun_bare_rt::SocketAddr(0, addr.port()),
        }
    }
}

pub mod path {
    pub struct Path;
    pub struct PathBuf;
}

pub mod fs {
    use super::io;
    use alloc::vec::Vec;

    pub fn read<T>(_path: T) -> io::Result<Vec<u8>> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "filesystem is unavailable in bare runtime",
        ))
    }
}

pub mod sync {
    pub use alloc::sync::Arc;
    pub use edgerun_bare_rt::{Mutex, MutexGuard};
}

pub mod time {
    pub use edgerun_bare_rt::{Duration, Instant};
}
