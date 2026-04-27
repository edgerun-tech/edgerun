//! Minimal std-shaped compatibility surface backed by core, alloc, and edgerun-bare-rt.

pub use core::{cmp, convert, fmt, future, hash, mem, option, pin, result, str, task};

pub mod ffi {
    pub use core::ffi::*;
}

pub mod os {
    pub mod raw {
        pub type c_int = i32;
    }

    pub mod unix {
        pub mod io {
            pub trait AsRawFd {
                fn as_raw_fd(&self) -> i32;
            }
        }
    }
}

pub mod collections {
    pub use alloc::collections::{BTreeMap, BTreeSet, VecDeque};

    pub type HashMap<K, V> = alloc::collections::BTreeMap<K, V>;
    pub type HashSet<T> = alloc::collections::BTreeSet<T>;

    pub mod hash_map {
        pub use alloc::collections::btree_map::Entry;
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
        ConnectionRefused,
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

    impl From<fmt::Error> for Error {
        fn from(_: fmt::Error) -> Self {
            Self::new(ErrorKind::Other, "format write failed")
        }
    }

    pub type Result<T> = core::result::Result<T, Error>;

    pub trait Write {
        fn write_all(&mut self, buf: &[u8]) -> Result<()>;
    }
}

pub mod net {
    use super::io;
    use alloc::string::{String, ToString};
    pub use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4};

    pub struct UdpSocket(edgerun_bare_rt::UdpSocket);

    impl UdpSocket {
        pub fn bind<A: IntoSocketAddr>(addr: A) -> io::Result<Self> {
            let mut socket = edgerun_bare_rt::UdpSocket::new();
            socket
                .bind(crate::compat::to_bare_addr(addr.into_socket_addr()?))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            Ok(Self(socket))
        }

        pub fn set_broadcast(&self, _broadcast: bool) -> io::Result<()> {
            Ok(())
        }

        pub fn set_read_timeout(
            &self,
            _timeout: Option<crate::std::time::Duration>,
        ) -> io::Result<()> {
            Ok(())
        }

        pub fn set_write_timeout(
            &self,
            _timeout: Option<crate::std::time::Duration>,
        ) -> io::Result<()> {
            Ok(())
        }

        pub fn set_nonblocking(&self, _nonblocking: bool) -> io::Result<()> {
            Ok(())
        }

        pub fn send_to<A: IntoSocketAddr>(&self, buf: &[u8], addr: A) -> io::Result<usize> {
            self.0
                .send_to(buf, crate::compat::to_bare_addr(addr.into_socket_addr()?))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
        }

        pub fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
            self.0
                .recv_from(buf)
                .map(|(n, addr)| (n, crate::compat::from_bare_addr(addr)))
                .map_err(|e| io::Error::new(io::ErrorKind::WouldBlock, e.to_string()))
        }

        pub fn local_addr(&self) -> io::Result<SocketAddr> {
            self.0
                .local_addr()
                .map(crate::compat::from_bare_addr)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "socket is not bound"))
        }

        pub fn as_raw_fd(&self) -> i32 {
            -1
        }
    }

    impl crate::std::os::unix::io::AsRawFd for UdpSocket {
        fn as_raw_fd(&self) -> i32 {
            self.as_raw_fd()
        }
    }

    pub trait IntoSocketAddr {
        fn into_socket_addr(self) -> io::Result<SocketAddr>;
    }

    impl IntoSocketAddr for SocketAddr {
        fn into_socket_addr(self) -> io::Result<SocketAddr> {
            Ok(self)
        }
    }

    impl IntoSocketAddr for &str {
        fn into_socket_addr(self) -> io::Result<SocketAddr> {
            self.parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
        }
    }

    impl IntoSocketAddr for &String {
        fn into_socket_addr(self) -> io::Result<SocketAddr> {
            self.as_str().into_socket_addr()
        }
    }

    impl IntoSocketAddr for (&str, u16) {
        fn into_socket_addr(self) -> io::Result<SocketAddr> {
            let ip = self
                .0
                .parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            Ok(SocketAddr::new(IpAddr::V4(ip), self.1))
        }
    }
}

pub mod sync {
    pub use alloc::sync::Arc;

    pub struct Mutex<T>(edgerun_bare_rt::Mutex<T>);

    impl<T> Mutex<T> {
        pub fn new(value: T) -> Self {
            Self(edgerun_bare_rt::Mutex::new(value))
        }

        pub fn lock(&self) -> LockResult<'_, T> {
            LockResult(Some(self.0.lock()))
        }
    }

    pub struct LockResult<'a, T>(Option<edgerun_bare_rt::MutexGuard<'a, T>>);

    impl<'a, T> LockResult<'a, T> {
        pub fn unwrap(mut self) -> edgerun_bare_rt::MutexGuard<'a, T> {
            self.0.take().expect("lock result consumed")
        }
    }
}

pub mod time {
    use core::ops::Add;
    pub use edgerun_bare_rt::Duration;

    #[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
    pub struct Instant(edgerun_bare_rt::Instant);

    impl Instant {
        pub fn now() -> Self {
            Self(edgerun_bare_rt::Instant::now())
        }

        pub fn elapsed(&self) -> Duration {
            self.0.elapsed()
        }

        pub fn duration_since(&self, earlier: Instant) -> Duration {
            self.0 - earlier.0
        }

        pub fn saturating_duration_since(&self, earlier: Instant) -> Duration {
            if *self >= earlier {
                self.duration_since(earlier)
            } else {
                Duration::from_micros(0)
            }
        }
    }

    impl Add<Duration> for Instant {
        type Output = Instant;

        fn add(self, rhs: Duration) -> Self::Output {
            Instant(self.0 + rhs)
        }
    }

    pub struct SystemTime;
    pub struct UnixEpoch;
    pub const UNIX_EPOCH: UnixEpoch = UnixEpoch;

    impl SystemTime {
        pub const UNIX_EPOCH: UnixEpoch = UnixEpoch;

        pub fn now() -> Self {
            Self
        }

        pub fn duration_since(&self, _epoch: UnixEpoch) -> Result<Duration, ()> {
            Ok(Duration::from_micros(edgerun_bare_rt::timer::ticks_to_us(
                edgerun_bare_rt::timer::now(),
            )))
        }
    }
}

pub mod fs {
    use super::io;
    use alloc::string::String;
    use core::fmt;

    pub struct File;

    impl File {
        pub fn create(_path: &str) -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "filesystem is not available in no_std DNS",
            ))
        }
    }

    impl io::Write for File {
        fn write_all(&mut self, _buf: &[u8]) -> io::Result<()> {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "filesystem is not available in no_std DNS",
            ))
        }
    }

    impl fmt::Write for File {
        fn write_str(&mut self, _s: &str) -> fmt::Result {
            Err(fmt::Error)
        }
    }

    pub fn read_to_string(_path: &str) -> io::Result<String> {
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "filesystem is not available in no_std DNS",
        ))
    }
}

pub mod thread {
    pub fn sleep(_duration: crate::std::time::Duration) {}
}
