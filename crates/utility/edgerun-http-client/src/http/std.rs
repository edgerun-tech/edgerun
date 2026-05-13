//! Minimal host-like compatibility surface for bare HTTP builds.

pub mod collections {
    pub use alloc::collections::{BTreeMap, BTreeSet, VecDeque};

    pub type HashMap<K, V> = alloc::collections::BTreeMap<K, V>;
    pub type HashSet<T> = alloc::collections::BTreeSet<T>;

    pub mod hash_map {
        use core::hash::Hasher;

        pub use alloc::collections::btree_map::Entry;

        #[derive(Default)]
        pub struct DefaultHasher(u64);

        impl DefaultHasher {
            pub fn new() -> Self {
                Self(0xcbf29ce484222325)
            }
        }

        impl Hasher for DefaultHasher {
            fn finish(&self) -> u64 {
                self.0
            }

            fn write(&mut self, bytes: &[u8]) {
                for byte in bytes {
                    self.0 ^= u64::from(*byte);
                    self.0 = self.0.wrapping_mul(0x100000001b3);
                }
            }
        }
    }
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

    #[cfg(feature = "runtime")]
    impl From<crate::rt::IoError> for Error {
        fn from(value: crate::rt::IoError) -> Self {
            match value {
                crate::rt::IoError::UnexpectedEof => Self::new(ErrorKind::UnexpectedEof, value),
                crate::rt::IoError::WriteZero => Self::new(ErrorKind::WriteZero, value),
                crate::rt::IoError::Other(_) => Self::new(ErrorKind::Other, value),
            }
        }
    }

    pub type Result<T> = core::result::Result<T, Error>;

    pub trait Read {
        fn read(&mut self, buf: &mut [u8]) -> Result<usize>;

        fn read_exact(&mut self, mut buf: &mut [u8]) -> Result<()> {
            while !buf.is_empty() {
                match self.read(buf)? {
                    0 => return Err(Error::new(ErrorKind::UnexpectedEof, "unexpected EOF")),
                    n => {
                        let tmp = buf;
                        buf = &mut tmp[n..];
                    }
                }
            }
            Ok(())
        }
    }

    pub trait Write {
        fn write(&mut self, buf: &[u8]) -> Result<usize>;

        fn flush(&mut self) -> Result<()> {
            Ok(())
        }

        fn write_all(&mut self, mut buf: &[u8]) -> Result<()> {
            while !buf.is_empty() {
                match self.write(buf)? {
                    0 => return Err(Error::new(ErrorKind::WriteZero, "failed to write buffer")),
                    n => buf = &buf[n..],
                }
            }
            Ok(())
        }
    }
}

pub mod net {
    use super::io;
    use alloc::string::{String, ToString};
    use alloc::vec::Vec;
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
        type Iter = alloc::vec::IntoIter<SocketAddr>;

        fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
            #[cfg(not(target_os = "none"))]
            {
                return std::net::ToSocketAddrs::to_socket_addrs(self)
                    .map(|addrs| addrs.collect::<Vec<_>>().into_iter())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e));
            }

            #[cfg(target_os = "none")]
            self.parse::<SocketAddr>()
                .map(|addr| alloc::vec![addr].into_iter())
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
        }
    }

    impl ToSocketAddrs for String {
        type Iter = alloc::vec::IntoIter<SocketAddr>;

        fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
            self.as_str().to_socket_addrs()
        }
    }

    impl ToSocketAddrs for (&str, u16) {
        type Iter = alloc::vec::IntoIter<SocketAddr>;

        fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
            #[cfg(not(target_os = "none"))]
            {
                return std::net::ToSocketAddrs::to_socket_addrs(self)
                    .map(|addrs| addrs.collect::<Vec<_>>().into_iter())
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e));
            }

            #[cfg(target_os = "none")]
            {
                let ip = self
                    .0
                    .parse()
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
                Ok(alloc::vec![SocketAddr::new(IpAddr::V4(ip), self.1)].into_iter())
            }
        }
    }

    #[cfg(all(feature = "runtime", feature = "node-core"))]
    pub struct UdpSocket(crate::rt::UdpSocket);

    #[cfg(all(feature = "runtime", feature = "node-core"))]
    impl UdpSocket {
        pub fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
            let addr = addr
                .to_socket_addrs()?
                .next()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no socket address"))?;
            let mut socket = crate::rt::UdpSocket::new();
            socket
                .bind(to_bare_addr(addr))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
            Ok(Self(socket))
        }
    }

    #[cfg(all(feature = "runtime", feature = "node-core"))]
    fn to_bare_addr(addr: SocketAddr) -> crate::rt::SocketAddr {
        match addr {
            SocketAddr::V4(v4) => {
                crate::rt::SocketAddr(u32::from_be_bytes(v4.ip().octets()), v4.port())
            }
            SocketAddr::V6(_) => crate::rt::SocketAddr(0, addr.port()),
        }
    }
}

pub mod path {
    use alloc::string::String;

    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    pub struct PathBuf(String);

    pub type Path = PathBuf;

    pub struct OsStr;

    impl OsStr {
        pub fn to_str(&self) -> Option<&str> {
            None
        }
    }

    impl PathBuf {
        pub fn join(&self, _path: impl AsRef<str>) -> Self {
            self.clone()
        }

        pub fn is_file(&self) -> bool {
            false
        }

        pub fn is_dir(&self) -> bool {
            false
        }

        pub fn extension(&self) -> Option<&OsStr> {
            None
        }
    }

    impl From<&str> for PathBuf {
        fn from(value: &str) -> Self {
            Self(String::from(value))
        }
    }

    impl From<String> for PathBuf {
        fn from(value: String) -> Self {
            Self(value)
        }
    }
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

    #[cfg(feature = "runtime")]
    pub use crate::rt::MutexGuard;

    #[cfg(feature = "runtime")]
    pub struct Mutex<T>(crate::rt::Mutex<T>);

    #[cfg(feature = "runtime")]
    impl<T> Mutex<T> {
        pub fn new(value: T) -> Self {
            Self(crate::rt::Mutex::new(value))
        }

        pub fn lock(&self) -> core::result::Result<MutexGuard<'_, T>, ()> {
            Ok(self.0.lock())
        }
    }
}

pub mod time {
    use core::ops::{Add, Sub};

    #[cfg(feature = "runtime")]
    pub use crate::rt::Duration;
    #[cfg(not(feature = "runtime"))]
    pub use core::time::Duration;

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
    #[cfg(feature = "runtime")]
    pub struct Instant(crate::rt::Instant);
    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
    #[cfg(not(feature = "runtime"))]
    pub struct Instant(Duration);

    impl Instant {
        pub fn now() -> Self {
            #[cfg(feature = "runtime")]
            {
                Self(crate::rt::Instant::now())
            }
            #[cfg(not(feature = "runtime"))]
            {
                Self(Duration::from_secs(0))
            }
        }

        pub fn duration_since(self, earlier: Instant) -> Duration {
            #[cfg(feature = "runtime")]
            {
                self.0 - earlier.0
            }
            #[cfg(not(feature = "runtime"))]
            {
                self.0.checked_sub(earlier.0).unwrap_or_default()
            }
        }

        pub fn elapsed(self) -> Duration {
            #[cfg(feature = "runtime")]
            {
                self.0.elapsed()
            }
            #[cfg(not(feature = "runtime"))]
            {
                Duration::from_secs(0)
            }
        }
    }

    impl Add<Duration> for Instant {
        type Output = Instant;

        fn add(self, rhs: Duration) -> Self::Output {
            Instant(self.0 + rhs)
        }
    }

    impl Sub<Duration> for Instant {
        type Output = Instant;

        fn sub(self, rhs: Duration) -> Self::Output {
            Instant(self.0 - rhs)
        }
    }

    impl Sub<Instant> for Instant {
        type Output = Duration;

        fn sub(self, rhs: Instant) -> Self::Output {
            self.0 - rhs.0
        }
    }

    #[derive(Clone, Copy, Debug)]
    pub struct SystemTime(Duration);

    pub const UNIX_EPOCH: SystemTime = SystemTime(Duration::from_secs(0));

    impl SystemTime {
        pub fn now() -> Self {
            Self(Duration::from_secs(0))
        }

        pub fn duration_since(self, earlier: SystemTime) -> Result<Duration, ()> {
            self.0.checked_sub(earlier.0).ok_or(())
        }
    }
}
