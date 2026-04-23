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

mod types;
mod parser;
mod projector;
mod importers;

pub use edgerun_json;

// Explicit public API — no glob re-exports
pub use types::{
    API_VERSION, ResourceMetadata, Resource, ConfigResource,
    DnsServerSpec, DnsZoneSpec, SoaRecord, ZoneRecord, DnssecConfig,
    DnsForwarderSpec, ForwardingRuleSpec, TlsConfigSpec, RateLimitSpec,
    DhcpServerSpec, DhcpReservation, DhcpPoolSpec, TftpServerSpec,
    NodeSpec, NodeTaint,
    ContainerSpec, ContainerRestartPolicy, Container, PodTemplateSpec, PodSpec,
    ContainerPort, VolumeMount, EnvVar, ResourceRequirements,
    SecretSpec, SecretType,
    PeerSpec, PeerEndpoint,
    GatewaySpec, GatewayListener, GatewayTlsConfig,
    ServiceSpec, ServicePort, ServiceAffinity,
    HttpRouteSpec, HttpRouteRule, HttpRouteMatch, HttpRouteHeaderMatch, HttpRouteBackend,
    TcpRouteSpec, TcpRouteBackend,
    TlsRouteSpec, TlsRouteBackend,
};
pub use parser::{parse_config_file, parse_and_validate, to_yaml_all, ConfigState, ConfigError};
pub use projector::{ConfigEvent, ConfigOp, ConfigProjector};
pub use importers::{
    import_dnsmasq, import_corefile, ImportError,
};
