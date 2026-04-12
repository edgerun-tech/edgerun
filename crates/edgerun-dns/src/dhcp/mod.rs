//! DHCPv4 server and client (RFC 2131).
//!
//! - **DHCP message parser/serializer** — RFC 2131 wire format
//! - **DHCP client** — discovers, requests, renews, releases leases
//! - **DHCP server** — offers, acks, tracks leases, configurable pools
//! - **Raw UDP sockets** — ports 67 (server) / 68 (client)
//! - **PXE boot support** — option 66/67/93/94/97 for network boot

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
