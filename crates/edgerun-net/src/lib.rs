//! Unified DNS + DHCP server.
//!
//! One process replaces:
//! - dnsmasq (DNS forwarder + DHCP server)
//! - BIND/PowerDNS (authoritative DNS)
//! - CoreDNS (DNS forwarding + plugins)
//! - ISC DHCP/Kea (DHCPv4 + DHCPv6)
//! - TFTP servers (PXE boot)
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────┐
//! │                edgerun-net                       │
//! │                                                 │
//! │  ┌──────────┐  ┌──────────┐  ┌───────────────┐  │
//! │  │ DNS      │  │ DHCPv4   │  │ DHCPv6        │  │
//! │  │ Server   │  │ Server   │  │ Server        │  │
//! │  │ :53      │  │ :67      │  │ :547          │  │
//! │  └────┬─────┘  └────┬─────┘  └───────┬───────┘  │
//! │       │              │                │          │
//! │       └──────────────┼────────────────┘          │
//! │                      │                           │
//! │              ┌───────▼───────┐                   │
//! │              │  Integration  │                   │
//! │              │  Layer        │                   │
//! │              │  (A/PTR sync) │                   │
//! │              └───────┬───────┘                   │
//! │                      │                           │
//! │              ┌───────▼───────┐                   │
//! │              │  Config       │                   │
//! │              │  (K8s YAML)   │                   │
//! │              │  (hot-reload) │                   │
//! │              └───────────────┘                   │
//! └─────────────────────────────────────────────────┘
//! ```

pub mod server;
pub mod integration;
pub mod config_watch;

pub use server::NetServer;
pub use integration::DnsDhcpIntegration;
pub use config_watch::ConfigWatcher;
