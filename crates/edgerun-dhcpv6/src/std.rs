//! Minimal std-shaped compatibility surface backed by core, alloc, and edgerun-bare-rt.

pub use core::{cmp, convert, fmt, mem, option, result, str};

pub mod prelude {
    pub mod v1 {
        pub use alloc::borrow::ToOwned;
        pub use alloc::format;
        pub use alloc::string::{String, ToString};
        pub use alloc::vec;
        pub use alloc::vec::Vec;
        pub use core::prelude::rust_2021::*;
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
}

pub mod net {
    use super::io;
    use alloc::string::{String, ToString};
    pub use core::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

    pub struct UdpSocket(edgerun_bare_rt::UdpSocket);

    impl UdpSocket {
        pub fn bind<A: IntoSocketAddr>(addr: A) -> Result<Self, io::Error> {
            let mut socket = edgerun_bare_rt::UdpSocket::new();
            socket
                .bind(crate::compat::to_bare_addr(addr.into_socket_addr()?))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
            Ok(Self(socket))
        }

        pub fn set_read_timeout(
            &self,
            _timeout: Option<crate::std::time::Duration>,
        ) -> Result<(), io::Error> {
            Ok(())
        }

        pub fn set_broadcast(&self, _broadcast: bool) -> Result<(), io::Error> {
            Ok(())
        }

        pub fn send_to<A: IntoSocketAddr>(&self, buf: &[u8], addr: A) -> Result<usize, io::Error> {
            self.0
                .send_to(buf, crate::compat::to_bare_addr(addr.into_socket_addr()?))
                .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
        }

        pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), io::Error> {
            self.0
                .recv_from(buf)
                .map(|(n, addr)| (n, crate::compat::from_bare_addr(addr)))
                .map_err(|e| io::Error::new(io::ErrorKind::WouldBlock, e.to_string()))
        }
    }

    pub trait IntoSocketAddr {
        fn into_socket_addr(self) -> Result<SocketAddr, io::Error>;
    }

    impl IntoSocketAddr for SocketAddr {
        fn into_socket_addr(self) -> Result<SocketAddr, io::Error> {
            Ok(self)
        }
    }

    impl IntoSocketAddr for &str {
        fn into_socket_addr(self) -> Result<SocketAddr, io::Error> {
            self.parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
        }
    }

    impl IntoSocketAddr for &String {
        fn into_socket_addr(self) -> Result<SocketAddr, io::Error> {
            self.as_str().into_socket_addr()
        }
    }

    impl IntoSocketAddr for (&str, u16) {
        fn into_socket_addr(self) -> Result<SocketAddr, io::Error> {
            let ip: IpAddr = self
                .0
                .parse()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
            Ok(SocketAddr::new(ip, self.1))
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
