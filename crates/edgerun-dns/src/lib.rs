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

#![warn(missing_docs)]
#![warn(rustdoc::missing_crate_level_docs)]

pub mod message;
pub mod client;
pub mod server;
pub mod zone;
pub mod record;

pub use message::{DnsMessage, DnsHeader, DnsOpcode, DnsResponseCode};
pub use message::{DnsQuestion, DnsRecord};
pub use client::DnsClient;
pub use server::{DnsServer, DnsServerConfig};
pub use zone::DnsZone;
pub use record::{DnsRecordType, DnsRecordData};
