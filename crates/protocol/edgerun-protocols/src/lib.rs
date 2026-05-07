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

#[cfg(feature = "core")]
pub mod core_protocol {
    pub use edgerun_core::*;
}

#[cfg(feature = "dhcp")]
pub mod dhcp;
#[cfg(feature = "dhcpv6")]
pub mod dhcpv6;
#[cfg(feature = "dns")]
pub mod dns;
#[cfg(feature = "email-auth")]
pub mod email_auth;
#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "keygen")]
pub mod keygen;
#[cfg(feature = "lmtp")]
pub mod lmtp;
#[cfg(feature = "node-bootstrap")]
pub mod node_bootstrap;
#[cfg(feature = "oci")]
pub mod oci;
#[cfg(feature = "proxy")]
pub mod proxy;
#[cfg(feature = "pxe")]
pub mod pxe;
#[cfg(feature = "quic")]
pub mod quic;
#[cfg(feature = "seal")]
pub mod seal;
#[cfg(feature = "sign")]
pub mod sign;
#[cfg(feature = "sign-p256")]
pub mod sign_p256;
#[cfg(feature = "smtp")]
pub mod smtp;
#[cfg(feature = "tftp")]
pub mod tftp;
#[cfg(feature = "tls")]
pub mod tls;
#[cfg(feature = "verify")]
pub mod verify;
#[cfg(feature = "wire")]
pub mod wire {
    pub use edgerun_wire::*;
}
