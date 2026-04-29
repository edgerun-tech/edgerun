//! DHCP compatibility re-exports.
//!
//! DHCPv4 is implemented by the dedicated `edgerun-dhcp` crate. This module
//! keeps the historical `edgerun_dns::dhcp` path available without carrying a
//! second DHCP implementation.

pub use edgerun_dhcp::{
    client, lease, message, options, server, DhcpClient, DhcpError, DhcpMessage, DhcpMessageType,
    DhcpOp, DhcpOptions, DhcpServer, Ipv4Addr, Lease, NetworkConfig, PxeClientArch,
    OPT_BOOTFILE_NAME, OPT_CLIENT_ARCH, OPT_CLIENT_MACHINE_ID, OPT_CLIENT_NDI, OPT_HOST_NAME,
    OPT_TFTP_SERVER_NAME, OPT_VENDOR_ENCAP,
};
