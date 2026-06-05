#![allow(missing_docs)]
#![no_std]
//! Transport-independent Edgerun protocol implementations.
//!
//! This crate owns protocol bytes and protocol state only. It must not bind
//! ports, open sockets, spawn tasks, sleep, touch filesystems, or decide host
//! transport policy. Runtime hosts such as `edgerun-node` provide those
//! capabilities and call into these modules.

extern crate alloc;

#[cfg(feature = "tls-tpm")]
extern crate std;

#[cfg(feature = "acme")]
pub mod acme;
#[cfg(feature = "block")]
pub mod block;
#[cfg(feature = "bluetooth-gatt")]
pub mod bluetooth_gatt;
#[cfg(feature = "bluetooth-mgmt")]
pub mod bluetooth_mgmt;
#[cfg(feature = "cec")]
pub mod cec;

pub mod prelude {
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
    pub use core::option::Option::{self, None, Some};
    pub use core::prelude::rust_2024::*;
    pub use core::result::Result::{self, Err, Ok};
}

#[cfg(feature = "core")]
pub mod core_protocol {
    //! Compatibility access to legacy `edgerun-core` records and helpers.
    //!
    //! New cross-node authority should be modeled in `edgerun-work`, and
    //! cross-runtime records should move through `edgerun-wire`. Keep this
    //! module as a migration bridge for older stream, command, and protocol
    //! records that still have real callers.

    pub use edgerun_core::*;
}

#[cfg(feature = "dbus")]
pub mod dbus;
#[cfg(feature = "dhcp")]
pub mod dhcp;
#[cfg(feature = "dhcpv6")]
pub mod dhcpv6;
#[cfg(feature = "dns")]
pub mod dns;
#[cfg(feature = "email-auth")]
pub mod email_auth;
#[cfg(feature = "emrtd")]
pub mod emrtd;
#[cfg(feature = "ethernet-ipv4")]
pub mod ethernet_ipv4;
#[cfg(feature = "goodix-fingerprint")]
pub mod goodix_fingerprint;
#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "wifi")]
pub mod ieee80211;
#[cfg(feature = "imap")]
pub mod imap;
#[cfg(feature = "keygen")]
pub mod keygen;
#[cfg(feature = "lmtp")]
pub mod lmtp;
#[cfg(feature = "matter")]
pub mod matter;
#[cfg(feature = "nbd")]
pub mod nbd;
#[cfg(feature = "ndef")]
pub mod ndef;
#[cfg(feature = "node-bootstrap")]
pub mod node_bootstrap;
#[cfg(feature = "oauth")]
pub mod oauth;
#[cfg(feature = "oci")]
pub mod oci;
#[cfg(feature = "proxy")]
pub mod proxy;
#[cfg(feature = "pxe")]
pub mod pxe;
#[cfg(feature = "quectel-ec200a")]
pub mod quectel_ec200a;
#[cfg(feature = "quic")]
pub mod quic;
#[cfg(feature = "seal")]
pub mod seal;
#[cfg(feature = "serial-mux")]
pub mod serial_mux;
#[cfg(feature = "sign")]
pub mod sign;
#[cfg(feature = "sign-p256")]
pub mod sign_p256;
#[cfg(feature = "smtp")]
pub mod smtp;
#[cfg(feature = "ssh")]
pub mod ssh;
#[cfg(feature = "tcl-ac")]
pub mod tcl_ac;
#[cfg(feature = "tftp")]
pub mod tftp;
#[cfg(any(feature = "tls", feature = "tls-cert"))]
pub mod tls;
#[cfg(feature = "tuya")]
pub mod tuya;
#[cfg(feature = "usb")]
pub mod usb;
#[cfg(feature = "verify")]
pub mod verify;
#[cfg(feature = "wayland")]
pub mod wayland;
#[cfg(feature = "websocket")]
pub mod websocket;
#[cfg(feature = "wire")]
pub mod wire {
    pub use edgerun_wire::*;
}
