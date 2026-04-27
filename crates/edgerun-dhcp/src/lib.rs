#![no_std]
#![allow(missing_docs)]
//! Dependency-free DHCPv4 server and client using only `std`.
//!
//! # Architecture
//! - **DHCP message parser/serializer** — RFC 2131 wire format
//! - **DHCP client** — discovers, requests, renews, releases leases
//! - **DHCP server** — offers, acks, tracks leases, configurable pools
//! - **Raw UDP sockets** — ports 67 (server) / 68 (client)
//!
//! # Wire Format (RFC 2131)
//! ```text
//! 0                   1                   2                   3
//! 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |     op (1)    |    htype (1)  |    hlen (1)   |    hops (1)   |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                            xid (4)                            |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |           secs (2)            |           flags (2)           |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                          ciaddr  (4)                          |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                          yiaddr  (4)                          |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                          siaddr  (4)                          |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                          giaddr  (4)                          |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                          chaddr (16)                          |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                          sname  (64)                          |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                          file   (128)                         |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                          options (variable)                   |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```

extern crate alloc;

pub mod client;
pub mod lease;
pub mod message;
pub mod options;
pub mod server;

pub use client::DhcpClient;
pub use lease::Lease;
pub use message::{
    DhcpError, DhcpMessage, DhcpMessageType, DhcpOp, DhcpOptions, Ipv4Addr, NetworkConfig,
    PxeClientArch,
};
pub use message::{
    OPT_BOOTFILE_NAME, OPT_CLIENT_ARCH, OPT_CLIENT_MACHINE_ID, OPT_CLIENT_NDI, OPT_HOST_NAME,
    OPT_TFTP_SERVER_NAME, OPT_VENDOR_ENCAP,
};
pub use server::DhcpServer;
