//! Dependency-free DNS server and client using only `std`.
//!
//! # Architecture
//! - **DNS message parser/serializer** — RFC 1035 wire format
//! - **DNS client** — sends queries, parses responses
//! - **DNS server** — handles queries, supports A, AAAA, CNAME, MX, NS, TXT, PTR, SOA
//! - **Zone file** — in-memory DNS zone with record management
//! - **UDP + TCP** — port 53, with TCP fallback for truncated responses
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
pub use server::DnsServer;
pub use zone::DnsZone;
pub use record::{DnsRecordType, DnsRecordData};
