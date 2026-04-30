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

#![no_std]

extern crate alloc;
extern crate self as std;

#[path = "std.rs"]
mod std_compat;
pub use std_compat::*;

mod importers;
mod parser;
mod projector;
mod types;

pub use edgerun_json;

// Explicit public API — no glob re-exports
pub use importers::{import_corefile, import_dnsmasq, ImportError};
pub use parser::{parse_and_validate, parse_config_file, to_yaml_all, ConfigError, ConfigState};
pub use projector::{ConfigEvent, ConfigOp, ConfigProjector};
pub use types::{
    BrowserAppCapabilitySpec, BrowserAppModuleSpec, BrowserAppSpec, BrowserNodePolicySpec,
    ConfigResource, Container, ContainerPort, ContainerRestartPolicy, ContainerSpec, DhcpPoolSpec,
    DhcpReservation, DhcpServerSpec, DnsForwarderSpec, DnsServerSpec, DnsZoneSpec, DnssecConfig,
    EnvVar, ForwardingRuleSpec, GatewayListener, GatewaySpec, GatewayTlsConfig, HttpRouteBackend,
    HttpRouteHeaderMatch, HttpRouteMatch, HttpRouteRule, HttpRouteSpec, ImapServerSpec,
    MailUserSpec, NodeSpec, NodeTaint, PeerEndpoint, PeerSpec, PodSpec, PodTemplateSpec,
    RateLimitSpec, Resource, ResourceMetadata, ResourceRequirements, SecretSpec, SecretType,
    ServiceAffinity, ServicePort, ServiceSpec, SmtpServerSpec, SoaRecord, TcpRouteBackend,
    TcpRouteSpec, TftpServerSpec, TlsConfigSpec, TlsRouteBackend, TlsRouteSpec, VolumeMount,
    ZoneRecord, API_VERSION,
};
