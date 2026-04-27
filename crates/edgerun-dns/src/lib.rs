//! no_std DNS server and client using edgerun-rt.
//!
//! # Architecture
//! - **DNS message parser/serializer** — RFC 1035 wire format
//! - **Async DNS client** — non-blocking queries via edgerun-rt primitives
//! - **Async DNS server** — concurrent query handling via `edgerun_rt::spawn`
//! - **Zone file** — in-memory DNS zone with record management
//! - **UDP** — socket I/O via `edgerun_rt::UdpSocket`
//!
//! # Wire Format (RFC 1035)
//! ```text
//! +---------------------+
//! |        Header       |
//! +---------------------+
//! |       Question      | the question for the name server
//! +---------------------+
//! |        Answer       | RRs answering the question
//! +---------------------+
//! |      Authority      | RRs pointing toward an authority
//! +---------------------+
//! |      Additional     | RRs holding additional information
//! +---------------------+
//! ```
//!
//! # Usage
//! ```no_run
//! use edgerun_dns::server::{DnsServer, DnsServerConfig};
//! use edgerun_dns::zone::DnsZone;
//! use edgerun_rt::Runtime;
//!
//! let rt = Runtime::new_multi_thread().build().unwrap();
//! rt.block_on(async {
//!     let mut zone = DnsZone::new("example.com");
//!     zone.add_a("@", std::net::Ipv4Addr::new(192, 168, 1, 1), 3600);
//!
//!     let config = DnsServerConfig::default();
//!     let server = DnsServer::new(config).unwrap();
//!     server.add_zone(zone).await;
//!     // server.run().await; // runs forever
//! });
//! ```

#![no_std]

#[macro_use]
extern crate alloc;

pub mod libc {
    #![allow(non_camel_case_types)]

    pub type c_int = i32;
    pub type c_void = core::ffi::c_void;
    pub type socklen_t = u32;

    pub const SOL_SOCKET: c_int = 1;
    pub const SO_REUSEADDR: c_int = 2;
    pub const SO_BROADCAST: c_int = 6;

    pub unsafe fn setsockopt(
        _socket: c_int,
        _level: c_int,
        _optname: c_int,
        _optval: *const c_void,
        _optlen: socklen_t,
    ) -> c_int {
        0
    }
}

pub mod compat;
pub mod std;

pub mod axfr;
pub mod cache;
pub mod client;
#[cfg(feature = "dhcp")]
pub mod dhcp;
#[cfg(feature = "dnssec")]
pub mod dnssec;
pub mod doh;
pub mod dot;
pub mod message;
pub mod name;
pub mod record;
pub mod resolv_conf;
pub mod resolver;
pub mod server;
pub mod tftp;
#[cfg(feature = "tsig")]
pub mod tsig;
pub mod zone;
pub mod zone_file;

pub use axfr::{handle_axfr, handle_notify, handle_update};
pub use cache::DnsCache;
pub use doh::{DohServer, DohServerConfig};
pub use dot::{DotServer, DotServerConfig};
pub use resolver::{default_root_hints, RecursiveResolver, RootHint};
pub use server::RateLimiter;
#[cfg(feature = "tsig")]
pub use tsig::{TsigAlgorithm, TsigError, TsigKey, TsigSigner, TsigVerifier};

// DHCP re-exports
#[cfg(feature = "dhcp-client")]
pub use dhcp::DhcpClient;
#[cfg(feature = "dhcp")]
pub use dhcp::{
    DhcpMessage, DhcpMessageType, DhcpOp, DhcpOptions, DhcpServer, Lease, NetworkConfig,
    PxeClientArch,
};
#[cfg(feature = "dhcp")]
pub use dhcp::{
    OPT_BOOTFILE_NAME, OPT_CLIENT_ARCH, OPT_CLIENT_MACHINE_ID, OPT_CLIENT_NDI, OPT_HOST_NAME,
    OPT_TFTP_SERVER_NAME, OPT_VENDOR_ENCAP,
};

// TFTP re-exports
pub use tftp::{BlobTftpProvider, TftpError, TftpMessage, TftpOpcode, TftpOptions, TftpServer};

// resolv.conf re-exports
pub use resolv_conf::{Nameserver, ResolvConf};

pub use client::DnsClient;
#[cfg(feature = "dnssec")]
pub use dnssec::{
    compute_key_tag, validate_response, verify_chain_of_trust, verify_rrsig, DnssecResult,
};
#[cfg(feature = "dnssec")]
pub use dnssec::{
    find_nsec3_covering, nsec3_base32hex, nsec3_hash_owner, nsec3_type_bitmap,
    synthesize_nsec3_chain,
};
#[cfg(feature = "dnssec")]
pub use dnssec::{
    generate_dnskey_ecdsap256, generate_dnskey_ed25519, sign_rrset_ecdsap256, sign_rrsig_ed25519,
};
#[cfg(feature = "dnssec")]
pub use dnssec::{sign_zone_ecdsap256, sign_zone_ed25519};
pub use message::{DnsHeader, DnsMessage, DnsOpcode, DnsResponseCode};
pub use message::{DnsQuestion, DnsRecord};
pub use name::{normalize_name, validate_name, NameError};
pub use record::{DnsRecordData, DnsRecordType};
pub use server::query::{handle_query, ServerState, MAX_UDP_RESPONSE};
pub use server::{DnsServer, DnsServerConfig};
pub use zone::DnsZone;
