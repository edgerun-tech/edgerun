//! DHCPv4 server and client (RFC 2131).
//!
//! - **DHCP message parser/serializer** — RFC 2131 wire format
//! - **DHCP client** — discovers, requests, renews, releases leases
//! - **DHCP server** — offers, acks, tracks leases, configurable pools
//! - **Raw UDP sockets** — ports 67 (server) / 68 (client)
//! - **PXE boot support** — option 66/67/93/94/97 for network boot
//! - **Multiple scopes/subnets** — serve multiple subnets from one server

use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
#[cfg(feature = "dhcp-client")]
pub mod client;
pub mod lease;
pub mod message;
pub mod options;
pub mod scope;
pub mod server;

#[cfg(feature = "dhcp-client")]
pub use client::DhcpClient;
pub use lease::Lease;
pub use message::{
    DhcpMessage, DhcpMessageType, DhcpOp, DhcpOptions, NetworkConfig, PxeClientArch,
};
pub use message::{
    OPT_BOOTFILE_NAME, OPT_CLIENT_ARCH, OPT_CLIENT_MACHINE_ID, OPT_CLIENT_NDI, OPT_HOST_NAME,
    OPT_TFTP_SERVER_NAME, OPT_VENDOR_ENCAP,
};
pub use scope::{DhcpMultiServer, DhcpScope};
pub use server::DhcpServer;
