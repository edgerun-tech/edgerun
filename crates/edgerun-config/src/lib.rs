//! edgerun-config — Kubernetes-compatible configuration for DNS/DHCP services.
//!
//! # Design
//!
//! Configuration is expressed as K8s-style YAML resources:
//! - `apiVersion: edgerun.io/v1alpha1`
//! - `kind: DnsServer | DnsZone | DhcpServer | DhcpPool | ForwardingRule | TlsConfig`
//! - `metadata: { name, namespace, labels, annotations }`
//! - `spec: { ... typed spec ... }`
//!
//! # Event-sourced state
//!
//! Config changes are recorded as events in the append-only event store
//! (edgerun-stream). The current config state is projected by replaying
//! these events. This means:
//! - Full audit trail of every config change
//! - Config can be rolled back to any point in time
//! - Multiple nodes can have divergent views that eventually converge
//! - No single point of failure for config storage
//!
//! # Import converters
//!
//! - `dnsmasq.conf` → `DhcpServer` + `DnsForwarder` YAML
//! - CoreDNS `Corefile` → `DnsServer` + `ForwardingRule` YAML
//! - BIND zone files → `DnsZone` YAML

pub mod types;
pub mod parser;
pub mod projector;
pub mod importers;

pub use types::*;
pub use parser::*;
pub use projector::*;
pub use importers::*;
