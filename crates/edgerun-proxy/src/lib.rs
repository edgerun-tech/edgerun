#![allow(missing_docs)]
#![no_std]
//! HTTP proxy server with CONNECT tunnel support (RFC 9110 / RFC 9111).
//!
//! # Architecture
//! - **HTTP CONNECT** — establish TCP tunnels to arbitrary hosts
//! - **HTTP proxy** — forward HTTP/HTTPS requests through upstream proxies
//! - **Upstream chaining** — configurable upstream proxy URL
//!
//! # Flow
//! ```text
//! Client                      Proxy                   Upstream
//!   |                          |                        |
//!   |-- CONNECT target:443 -->|                        |
//!   |<-- 200 Connection OK ---|                        |
//!   |                        |-- CONNECT target:443 -->|
//!   |                        |<-- 200 Connection OK -|
//!   |                        |<-- 200 OK ------------|
//!   |<-- 200 OK -------------|                        |
//!   |=========================| (tunnel established) |
//!   |<~~~~ encrypted data ~~~~>|~~~~ encrypted data ~~~~>|
//! ```

extern crate alloc;
#[cfg(target_os = "none")]
extern crate self as std;
#[cfg(not(target_os = "none"))]
extern crate std;

pub mod prelude {
    pub use alloc::format;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec;
    pub use alloc::vec::Vec;
    pub use core::convert::Into;
    pub use core::module_path;
    pub use core::option::Option::{self, None, Some};
    pub use core::prelude::rust_2024::*;
    pub use core::result::Result::{self, Err, Ok};
}

#[cfg(target_os = "none")]
pub mod io {
    use alloc::string::{String, ToString};
    use core::fmt;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum ErrorKind {
        InvalidData,
        PermissionDenied,
        ConnectionRefused,
        TimedOut,
        Other,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    pub struct Error {
        kind: ErrorKind,
        message: String,
    }

    impl Error {
        pub fn new(kind: ErrorKind, message: impl ToString) -> Self {
            Self {
                kind,
                message: message.to_string(),
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

    impl From<edgerun_rt::io::IoError> for Error {
        fn from(error: edgerun_rt::io::IoError) -> Self {
            match error {
                edgerun_rt::io::IoError::UnexpectedEof => {
                    Self::new(ErrorKind::InvalidData, "unexpected EOF")
                }
                edgerun_rt::io::IoError::WriteZero => Self::new(ErrorKind::Other, "write zero"),
                edgerun_rt::io::IoError::Other(message) => Self::new(ErrorKind::Other, message),
            }
        }
    }

    pub type Result<T> = core::result::Result<T, Error>;
}

#[cfg(target_os = "none")]
pub mod net {
    pub use core::net::*;
}

#[cfg(target_os = "none")]
pub mod sync {
    pub use alloc::sync::Arc;

    pub mod atomic {
        pub use core::sync::atomic::*;
    }
}

#[cfg(target_os = "none")]
pub mod time {
    pub use edgerun_rt::Duration;
}

pub mod server;

pub use server::{ProxyConfig, ProxyServer};
