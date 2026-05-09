//! DHCPv4 message codec and server state machine.

pub mod lease;
pub mod message;
pub mod options;
pub mod server_core;

pub use lease::Lease;
pub use message::{
    DHCP_CLIENT_PORT, DHCP_SERVER_PORT, OPT_BOOTFILE_NAME, OPT_CLIENT_ARCH, OPT_CLIENT_MACHINE_ID,
    OPT_CLIENT_NDI, OPT_HOST_NAME, OPT_TFTP_SERVER_NAME, OPT_VENDOR_ENCAP,
};
pub use message::{
    DhcpError, DhcpMessage, DhcpMessageType, DhcpOp, DhcpOptions, Ipv4Addr, NetworkConfig,
    PxeClientArch,
};
pub use server_core::{DhcpDatagram, DhcpServerConfig, DhcpServerCore};
