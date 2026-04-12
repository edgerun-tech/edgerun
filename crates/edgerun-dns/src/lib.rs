//! Async DNS server and client using edgerun-rt.
//!
//! # Architecture
//! - **DNS message parser/serializer** — RFC 1035 wire format
//! - **Async DNS client** — non-blocking queries via edgerun-rt epoll reactor
//! - **Async DNS server** — concurrent query handling, one task per query via `edgerun_rt::spawn`
//! - **Zone file** — in-memory DNS zone with record management (protected by async RwLock)
//! - **UDP** — non-blocking socket I/O via `edgerun_rt::AsyncUdpSocket`
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
//! use std::net::Ipv4Addr;
//!
//! let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
//! rt.block_on(async {
//!     let mut zone = DnsZone::new("example.com");
//!     zone.add_a("@", Ipv4Addr::new(192, 168, 1, 1), 3600);
//!
//!     let config = DnsServerConfig::default();
//!     let server = DnsServer::new(config).unwrap();
//!     server.add_zone(zone).await;
//!     // server.run().await; // runs forever
//! });
//! ```

pub mod message;
pub mod client;
pub mod server;
pub mod zone;
pub mod record;
pub mod name;
pub mod dnssec;
pub mod zone_file;
pub mod cache;
pub mod resolver;

pub use cache::DnsCache;
pub use resolver::{RecursiveResolver, RootHint, default_root_hints};
pub use server::RateLimiter;

pub use message::{DnsMessage, DnsHeader, DnsOpcode, DnsResponseCode};
pub use message::{DnsQuestion, DnsRecord};
pub use client::DnsClient;
pub use server::{DnsServer, DnsServerConfig};
pub use server::query::{ServerState, handle_query, MAX_UDP_RESPONSE};
pub use zone::DnsZone;
pub use record::{DnsRecordType, DnsRecordData};
pub use name::{validate_name, normalize_name, NameError};
pub use dnssec::{DnssecResult, compute_key_tag, verify_rrsig, verify_chain_of_trust, validate_response};
pub use dnssec::{generate_dnskey_ed25519, generate_dnskey_ecdsap256, sign_rrsig_ed25519, sign_rrset_ecdsap256};
pub use dnssec::{sign_zone_ed25519, sign_zone_ecdsap256};
