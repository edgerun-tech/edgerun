//! edgerun-dhcpv6 — DHCPv6 server and client (RFC 8415).
//!
//! # Architecture
//! - **DHCPv6 message parser/serializer** — RFC 8415 wire format
//! - **DHCPv6 server** — handles Solicit→Advertise, Request→Reply, Renew→Reply, etc.
//! - **DHCPv6 client** — full Solicit/Request state machine with IA_NA/IA_TA
//! - **DUID management** — DUID-LLT, DUID-EN, DUID-LL, DUID-UUID
//! - **Prefix delegation** — IA_PD support
//! - **Stateful + stateless** — both address assignment and information-request
//!
//! # Ports
//! - Server: UDP 547
//! - Client: UDP 546
//! - Multicast: ff02::1:2 (All_DHCP_Relay_Agents_and_Servers)
//! - Multicast: ff05::1:3 (All_DHCP_Servers, site-local)

pub mod client;
pub mod duid;
pub mod lease;
pub mod message;
pub mod options;
pub mod server;

pub use client::Dhcpv6Client;
pub use duid::{Duid, DuidType};
pub use lease::{Dhcpv6Lease, LeaseState, PrefixLease};
pub use message::{Dhcpv6Message, Dhcpv6MsgType, TransactionId};
pub use options::{Dhcpv6Option, IaNaOption, IaPdOption, IaTaOption};
pub use server::Dhcpv6Server;
