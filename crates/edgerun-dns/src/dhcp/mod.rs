//! DHCPv4 protocol compatibility re-exports.

pub use edgerun_protocols::dhcp::{
    lease, message, options, server_core, DhcpError, DhcpMessage, DhcpMessageType, DhcpOp,
    DhcpOptions, Ipv4Addr, Lease, NetworkConfig, PxeClientArch, OPT_BOOTFILE_NAME, OPT_CLIENT_ARCH,
    OPT_CLIENT_MACHINE_ID, OPT_CLIENT_NDI, OPT_HOST_NAME, OPT_TFTP_SERVER_NAME, OPT_VENDOR_ENCAP,
};
