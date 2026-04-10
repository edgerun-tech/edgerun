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

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod message;
pub mod client;
pub mod server;
pub mod options;
pub mod lease;

pub use message::{DhcpMessage, DhcpOp, DhcpMessageType, DhcpOptions, PxeClientArch};
pub use message::{
    OPT_TFTP_SERVER_NAME, OPT_BOOTFILE_NAME, OPT_CLIENT_ARCH, OPT_CLIENT_NDI,
    OPT_CLIENT_MACHINE_ID, OPT_VENDOR_ENCAP, OPT_HOST_NAME,
};
pub use client::DhcpClient;
pub use server::DhcpServer;
pub use lease::Lease;
