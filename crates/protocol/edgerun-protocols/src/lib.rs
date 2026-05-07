#![allow(missing_docs)]
#![no_std]
//! Transport-independent Edgerun protocol implementations.
//!
//! This crate owns protocol bytes and protocol state only. It must not bind
//! ports, open sockets, spawn tasks, sleep, touch filesystems, or decide host
//! transport policy. Runtime hosts such as `edgerun-node` provide those
//! capabilities and call into these modules.

extern crate alloc;

pub mod prelude {
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
    pub use core::option::Option::{self, None, Some};
    pub use core::prelude::rust_2024::*;
    pub use core::result::Result::{self, Err, Ok};
}

#[cfg(feature = "dhcp")]
pub mod dhcp;
#[cfg(feature = "dhcpv6")]
pub mod dhcpv6;
#[cfg(feature = "dns")]
pub mod dns;
#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "lmtp")]
pub mod lmtp;
#[cfg(feature = "proxy")]
pub mod proxy;
#[cfg(feature = "smtp")]
pub mod smtp;
#[cfg(feature = "tftp")]
pub mod tftp;
