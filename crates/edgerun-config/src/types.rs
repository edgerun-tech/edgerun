//! Kubernetes-compatible configuration resource types.
//!
//! Every config resource follows the K8s pattern:
//! ```yaml
//! apiVersion: edgerun.io/v1alpha1
//! kind: DnsServer
//! metadata:
//!   name: primary-dns
//!   namespace: edgerun-dns
//!   labels:
//!     app: dns
//! spec:
//!   bind_address: "0.0.0.0:53"
//!   ...
//! ```

use crate::collections::HashMap;
use crate::prelude::v1::*;
use edgerun_json::{FromJson, ToJson};

// ---------------------------------------------------------------------------
// K8s-compatible resource envelope
// ---------------------------------------------------------------------------

/// The API version for all edgerun config resources.
pub const API_VERSION: &str = "edgerun.io/v1alpha1";

/// Standard K8s-style metadata.
#[derive(Debug, Clone)]
pub struct ResourceMetadata {
    /// Resource name (unique within namespace).
    pub name: String,
    /// Namespace for grouping resources.
    pub namespace: Option<String>,
    /// Key-value labels for selection and filtering.
    pub labels: Option<HashMap<String, String>>,
    /// Arbitrary annotations (for converters, notes, etc.).
    pub annotations: Option<HashMap<String, String>>,
}

/// A complete K8s-style config resource envelope.
#[derive(Debug, Clone)]
pub struct Resource<T> {
    /// API version (always "edgerun.io/v1alpha1").
    pub api_version: String,
    /// Resource kind (e.g. "DnsServer", "DnsZone", "DhcpServer").
    pub kind: String,
    /// Standard K8s metadata.
    pub metadata: ResourceMetadata,
    /// The typed spec.
    pub spec: T,
}

impl<T> Resource<T> {
    /// Create a new resource.
    pub fn new(kind: &str, name: &str, spec: T) -> Self {
        Self {
            api_version: API_VERSION.to_string(),
            kind: kind.to_string(),
            metadata: ResourceMetadata {
                name: name.to_string(),
                namespace: None,
                labels: None,
                annotations: None,
            },
            spec,
        }
    }

    /// Set labels on this resource.
    pub fn with_labels(mut self, labels: HashMap<String, String>) -> Self {
        self.metadata.labels = Some(labels);
        self
    }

    /// Set namespace on this resource.
    pub fn with_namespace(mut self, ns: &str) -> Self {
        self.metadata.namespace = Some(ns.to_string());
        self
    }

    /// Set annotations on this resource.
    pub fn with_annotations(mut self, annotations: HashMap<String, String>) -> Self {
        self.metadata.annotations = Some(annotations);
        self
    }
}

impl<T: edgerun_json::ToJson + edgerun_json::FromJson> Resource<T> {
    /// Serialize to YAML string.
    pub fn to_yaml(&self) -> Result<String, edgerun_json::yaml::YamlError> {
        edgerun_json::yaml::to_yaml_string(&edgerun_json::yaml::json_to_yaml(
            edgerun_json::ToJson::to_json(self),
        ))
    }

    /// Parse from YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, edgerun_json::yaml::YamlError> {
        let yaml_value = edgerun_json::yaml::from_yaml_str(yaml)?;
        let json = edgerun_json::yaml::yaml_to_json(yaml_value);
        edgerun_json::FromJson::from_json(json).map_err(|_| {
            edgerun_json::yaml::YamlError::IoError("deserialization error".to_string())
        })
    }
}

/// K8s-style config resource — wraps raw spec types with the envelope.
#[derive(Debug, Clone)]
pub enum ConfigResource {
    DnsServer(DnsServerSpec),
    DnsZone(DnsZoneSpec),
    DnsForwarder(DnsForwarderSpec),
    ForwardingRule(ForwardingRuleSpec),
    TlsConfig(TlsConfigSpec),
    RateLimit(RateLimitSpec),
    DhcpServer(DhcpServerSpec),
    DhcpPool(DhcpPoolSpec),
    Dhcpv6Server(Dhcpv6ServerSpec),
    Dhcpv6Pool(Dhcpv6PoolSpec),
    TftpServer(TftpServerSpec),
    SmtpServer(SmtpServerSpec),
    ImapServer(ImapServerSpec),
    Node(NodeSpec),
    Container(ContainerSpec),
    Deployment(DeploymentSpec),
    Secret(SecretSpec),
    Peer(PeerSpec),
    Gateway(GatewaySpec),
    Service(ServiceSpec),
    HttpRoute(HttpRouteSpec),
    TcpRoute(TcpRouteSpec),
    TlsRoute(TlsRouteSpec),
}

impl ConfigResource {
    /// Get the resource name.
    pub fn name(&self) -> &str {
        match self {
            Self::DnsServer(_) => "unnamed-dns-server",
            Self::DnsZone(r) => &r.origin,
            Self::DnsForwarder(_) => "unnamed-forwarder",
            Self::ForwardingRule(r) => &r.zone,
            Self::TlsConfig(_) => "unnamed-tls",
            Self::RateLimit(_) => "unnamed-ratelimit",
            Self::DhcpServer(_) => "unnamed-dhcp",
            Self::DhcpPool(r) => &r.name,
            Self::Dhcpv6Server(_) => "unnamed-dhcpv6",
            Self::Dhcpv6Pool(r) => &r.name,
            Self::TftpServer(_) => "unnamed-tftp",
            Self::SmtpServer(r) => &r.hostname,
            Self::ImapServer(r) => &r.hostname,
            Self::Node(_) => "unnamed-node",
            Self::Container(_) => "unnamed-container",
            Self::Deployment(r) => &r.name,
            Self::Secret(_) => "unnamed-secret",
            Self::Peer(_) => "unnamed-peer",
            Self::Gateway(_) => "unnamed-gateway",
            Self::Service(_) => "unnamed-service",
            Self::HttpRoute(_) => "unnamed-httproute",
            Self::TcpRoute(_) => "unnamed-tcproute",
            Self::TlsRoute(_) => "unnamed-tlsroute",
        }
    }

    /// Get the resource kind.
    pub fn kind(&self) -> &str {
        match self {
            Self::DnsServer(_) => "DnsServer",
            Self::DnsZone(_) => "DnsZone",
            Self::DnsForwarder(_) => "DnsForwarder",
            Self::ForwardingRule(_) => "ForwardingRule",
            Self::TlsConfig(_) => "TlsConfig",
            Self::RateLimit(_) => "RateLimit",
            Self::DhcpServer(_) => "DhcpServer",
            Self::DhcpPool(_) => "DhcpPool",
            Self::Dhcpv6Server(_) => "Dhcpv6Server",
            Self::Dhcpv6Pool(_) => "Dhcpv6Pool",
            Self::TftpServer(_) => "TftpServer",
            Self::SmtpServer(_) => "SmtpServer",
            Self::ImapServer(_) => "ImapServer",
            Self::Node(_) => "Node",
            Self::Container(_) => "Container",
            Self::Deployment(_) => "Deployment",
            Self::Secret(_) => "Secret",
            Self::Peer(_) => "Peer",
            Self::Gateway(_) => "Gateway",
            Self::Service(_) => "Service",
            Self::HttpRoute(_) => "HttpRoute",
            Self::TcpRoute(_) => "TcpRoute",
            Self::TlsRoute(_) => "TlsRoute",
        }
    }
}

// ---------------------------------------------------------------------------
// DNS Server spec
// ---------------------------------------------------------------------------

/// DNS server configuration.
#[derive(Debug, Clone)]
pub struct DnsServerSpec {
    /// UDP/TCP bind address (default: "0.0.0.0:53").
    pub bind_address: Option<String>,
    /// IPv6 bind address (e.g. "[::]:53").
    pub bind_address_ipv6: Option<String>,
    /// Default TTL for records (seconds).
    pub default_ttl: Option<u32>,
    /// Rate limit queries per second per IP (0 = unlimited).
    pub rate_limit_qps: Option<u32>,
    /// List of zone names this server serves.
    pub zones: Option<Vec<String>>,
    /// Upstream resolver for forwarding.
    pub forward_to: Option<String>,
    /// Enable recursive resolution.
    pub recursive: Option<bool>,
}

// ---------------------------------------------------------------------------
// DNS Zone spec
// ---------------------------------------------------------------------------

/// DNS zone configuration.
#[derive(Debug, Clone)]
pub struct DnsZoneSpec {
    /// Zone origin (e.g. "example.com").
    pub origin: String,
    /// SOA record.
    pub soa: SoaRecord,
    /// DNS records in the zone.
    pub records: Vec<ZoneRecord>,
    /// DNSSEC signing config.
    pub dnssec: Option<DnssecConfig>,
    /// Wildcard records.
    pub wildcards: Option<Vec<ZoneRecord>>,
}

/// SOA record configuration.
#[derive(Debug, Clone)]
pub struct SoaRecord {
    /// Primary nameserver.
    pub mname: String,
    /// Responsible admin email (with `@` replaced by `.`).
    pub rname: String,
    /// Zone serial number.
    pub serial: u32,
    /// Refresh interval (seconds).
    pub refresh: u32,
    /// Retry interval (seconds).
    pub retry: u32,
    /// Expiry time (seconds).
    pub expire: u32,
    /// Minimum TTL / negative cache TTL (seconds).
    pub minimum: u32,
}

/// A single DNS record in a zone.
#[derive(Debug, Clone)]
pub struct ZoneRecord {
    /// Record name (relative to zone origin, or FQDN).
    pub name: String,
    /// Record type (A, AAAA, CNAME, MX, TXT, SRV, etc.).
    pub record_type: String,
    /// TTL in seconds (defaults to zone default).
    pub ttl: Option<u32>,
    /// Record value — type-specific.
    pub value: edgerun_json::JsonValue,
}

/// DNSSEC configuration for a zone.
#[derive(Debug, Clone)]
pub struct DnssecConfig {
    /// Algorithm: "ed25519", "ecdsap256".
    pub algorithm: String,
    /// Key flags: 256 (ZSK), 257 (KSK).
    pub key_flags: u16,
    /// Key TTL (seconds).
    pub key_ttl: u32,
    /// Signature validity period (seconds from now).
    pub signature_validity: u32,
    /// Enable NSEC3.
    pub nsec3: bool,
    /// NSEC3 salt (hex string).
    pub nsec3_salt: Option<String>,
    /// NSEC3 iterations.
    pub nsec3_iterations: u16,
}

// ---------------------------------------------------------------------------
// DNS Forwarder spec
// ---------------------------------------------------------------------------

/// DNS forwarder configuration.
#[derive(Debug, Clone)]
pub struct DnsForwarderSpec {
    /// Bind address for the forwarder.
    pub bind_address: Option<String>,
    /// List of upstream resolvers.
    pub upstreams: Vec<String>,
    /// Query timeout (seconds).
    pub timeout: Option<u64>,
    /// Enable caching.
    pub cache: bool,
    /// Cache TTL (seconds).
    pub cache_ttl: Option<u32>,
    /// Max cache entries.
    pub cache_max_entries: Option<usize>,
}

// ---------------------------------------------------------------------------
// Forwarding Rule spec
// ---------------------------------------------------------------------------

/// Conditional forwarding rule (like CoreDNS's `forward` plugin).
#[derive(Debug, Clone)]
pub struct ForwardingRuleSpec {
    /// Domain zone to match (e.g. "cluster.local").
    pub zone: String,
    /// Upstream servers to forward to.
    pub upstreams: Vec<String>,
    /// Optional policy: "sequential", "random", "round_robin".
    pub policy: Option<String>,
    /// Health check interval (seconds).
    pub health_check: Option<u64>,
}

// ---------------------------------------------------------------------------
// TLS Config spec
// ---------------------------------------------------------------------------

/// TLS configuration for DoT/DoH.
#[derive(Debug, Clone)]
pub struct TlsConfigSpec {
    /// Enable DNS-over-TLS on port 853.
    pub dot_enabled: bool,
    /// Enable DNS-over-HTTPS.
    pub doh_enabled: bool,
    /// DoH bind address.
    pub doh_bind_address: Option<String>,
    /// DoH URL path.
    pub doh_path: Option<String>,
    /// TLS certificate path.
    pub cert_path: Option<String>,
    /// TLS key path.
    pub key_path: Option<String>,
}

// ---------------------------------------------------------------------------
// Rate Limit spec
// ---------------------------------------------------------------------------

/// Rate limiting configuration.
#[derive(Debug, Clone)]
pub struct RateLimitSpec {
    /// Queries per second per client IP.
    pub qps: u32,
    /// Burst size.
    pub burst: u32,
    /// Block duration after exceeding limit (seconds).
    pub block_duration: u32,
}

// ---------------------------------------------------------------------------
// DHCP Server spec
// ---------------------------------------------------------------------------

/// DHCP server configuration.
#[derive(Debug, Clone)]
pub struct DhcpServerSpec {
    /// Network interface to bind to.
    pub interface: String,
    /// Pool name references.
    pub pools: Vec<String>,
    /// Default lease time (seconds).
    pub default_lease_time: u32,
    /// Max lease time (seconds).
    pub max_lease_time: Option<u32>,
    /// DNS servers to hand out.
    pub dns_servers: Option<Vec<String>>,
    /// Router/gateway to hand out.
    pub router: Option<String>,
    /// NTP servers.
    pub ntp_servers: Option<Vec<String>>,
    /// Domain name.
    pub domain_name: Option<String>,
    /// PXE bootfile.
    pub bootfile: Option<String>,
    /// PXE TFTP server.
    pub tftp_server: Option<String>,
    /// Static reservations.
    pub reservations: Option<Vec<DhcpReservation>>,
}

/// DHCP static reservation.
#[derive(Debug, Clone)]
pub struct DhcpReservation {
    /// MAC address.
    pub mac: String,
    /// Reserved IP address.
    pub ip: String,
    /// Optional hostname.
    pub hostname: Option<String>,
}

// ---------------------------------------------------------------------------
// DHCPv6 Server spec
// ---------------------------------------------------------------------------

/// DHCPv6 server configuration.
#[derive(Debug, Clone)]
pub struct Dhcpv6ServerSpec {
    /// Network interface to bind to.
    pub interface: String,
    /// Pool name references.
    pub pools: Vec<String>,
    /// Default preferred lifetime (seconds).
    pub default_preferred_lifetime: u32,
    /// Default valid lifetime (seconds).
    pub default_valid_lifetime: u32,
    /// DNS servers to hand out (IPv6 addresses).
    pub dns_servers: Option<Vec<String>>,
    /// Domain name.
    pub domain_name: Option<String>,
    /// Static reservations (IPv6).
    pub reservations: Option<Vec<Dhcpv6Reservation>>,
}

fn default_3600() -> u32 {
    3600
}
fn default_7200() -> u32 {
    7200
}

/// DHCPv6 static reservation.
#[derive(Debug, Clone)]
pub struct Dhcpv6Reservation {
    /// DUID (DHCP Unique Identifier) or MAC address.
    pub duid: String,
    /// Reserved IPv6 address.
    pub ip: String,
    /// Optional hostname.
    pub hostname: Option<String>,
}

// ---------------------------------------------------------------------------
// DHCPv6 Pool spec
// ---------------------------------------------------------------------------

/// DHCPv6 pool configuration.
#[derive(Debug, Clone)]
pub struct Dhcpv6PoolSpec {
    /// Pool name.
    pub name: String,
    /// Start IPv6 (inclusive).
    pub range_start: String,
    /// End IPv6 (inclusive).
    pub range_end: String,
    /// Prefix length (e.g., 64).
    pub prefix_length: u8,
    /// Excluded IPv6s.
    pub exclude: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// DHCP Pool spec
// ---------------------------------------------------------------------------

/// DHCP pool configuration.
#[derive(Debug, Clone)]
pub struct DhcpPoolSpec {
    /// Pool name.
    pub name: String,
    /// Start IP (inclusive).
    pub range_start: String,
    /// End IP (inclusive).
    pub range_end: String,
    /// Subnet mask.
    pub subnet_mask: String,
    /// Excluded IPs.
    pub exclude: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// TFTP Server spec
// ---------------------------------------------------------------------------

/// TFTP server configuration.
#[derive(Debug, Clone)]
pub struct TftpServerSpec {
    /// Bind address (default: "0.0.0.0:69").
    pub bind_address: Option<String>,
    /// Root directory for served files.
    pub root_dir: String,
    /// Block size (default: 512, max: 65464).
    pub block_size: Option<u16>,
    /// Timeout (seconds).
    pub timeout: Option<u32>,
    /// Allow writes.
    pub allow_writes: bool,
}

// ---------------------------------------------------------------------------
// SMTP Server spec
// ---------------------------------------------------------------------------

/// SMTP server configuration.
#[derive(Debug, Clone)]
pub struct SmtpServerSpec {
    /// Server hostname (for EHLO/HELO).
    pub hostname: String,
    /// TCP bind address (default: "0.0.0.0:25").
    pub bind_address: Option<String>,
    /// Enable SMTPS on port 465.
    pub smtps: bool,
    /// Enable STARTTLS on port 587.
    pub starttls: bool,
    /// Maximum message size in bytes (default: 35MB).
    pub max_message_size: Option<usize>,
    /// Local domains for mail delivery.
    pub local_domains: Vec<String>,
    /// Maildir root for storing messages.
    pub maildir_root: Option<String>,
    /// Enable outbound relay/queue.
    pub relay_enabled: bool,
    /// Queue data directory for outbound mail.
    pub queue_dir: Option<String>,
    /// DNS server for MX lookups (default: "8.8.8.8:53").
    pub dns_server: Option<String>,
    /// DKIM signing domain.
    pub dkim_domain: Option<String>,
    /// DKIM selector.
    pub dkim_selector: Option<String>,
    /// Path to DKIM private key file.
    pub dkim_key_path: Option<String>,
    /// TLS certificate (PEM format).
    pub tls_cert: Option<String>,
    /// TLS private key (PEM format).
    pub tls_key: Option<String>,
    /// Local mail users accepted by SMTP.
    pub users: Option<Vec<MailUserSpec>>,
    /// Optional mailbox username that receives otherwise unknown local recipients.
    pub catch_all_user: Option<String>,
    /// Enable ACME certificate issuance for this server.
    pub acme_enabled: bool,
    /// ACME directory URL or preset name.
    pub acme_directory: Option<String>,
    /// ACME account contact email.
    pub acme_contact_email: Option<String>,
    /// Domains to include in the ACME certificate.
    pub acme_domains: Option<Vec<String>>,
    /// ACME account key path.
    pub acme_account_key_path: Option<String>,
    /// ACME certificate directory containing fullchain.pem and privkey.pem.
    pub acme_cert_dir: Option<String>,
}

// ---------------------------------------------------------------------------
// IMAP Server spec
// ---------------------------------------------------------------------------

/// IMAP server configuration.
#[derive(Debug, Clone)]
pub struct ImapServerSpec {
    /// Server hostname (for CAPABILITY).
    pub hostname: String,
    /// TCP bind address (default: "0.0.0.0:143").
    pub bind_address: Option<String>,
    /// Enable IMAPS on port 993.
    pub imaps: bool,
    /// Maildir root for storing messages (should match SMTP).
    pub maildir_root: Option<String>,
    /// TLS certificate (PEM format).
    pub tls_cert: Option<String>,
    /// TLS private key (PEM format).
    pub tls_key: Option<String>,
    /// IMAP users and passwords.
    pub users: Option<Vec<MailUserSpec>>,
}

/// Local mailbox user configuration.
#[derive(Debug, Clone)]
pub struct MailUserSpec {
    /// Mailbox username, normally the local part before `@`.
    pub username: String,
    /// Optional password for IMAP authentication.
    pub password: Option<String>,
    /// Optional accepted domains for SMTP local delivery.
    pub domains: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// Node spec (K8s Node-compatible)
// ---------------------------------------------------------------------------

/// Node configuration - defines controllers who can control this node.
/// Node ID is the public key (derived from genesis), not set in config.
#[derive(Debug, Clone)]
pub struct NodeSpec {
    /// Controller identity IDs (node IDs = public keys).
    /// These identities can send commands to this node.
    pub controllers: Vec<String>,
    /// Node roles (e.g. "control-plane", "worker").
    pub roles: Vec<String>,
    /// Public IPs for this node (for reachability).
    pub public_ips: Vec<String>,
    /// Taints applied to pods that can't schedule on this node.
    pub taints: Vec<NodeTaint>,
    /// Unschedulable marks node as unschedulable.
    pub unschedulable: bool,
}

/// Node taint - marks pods that can't schedule on this node.
#[derive(Debug, Clone)]
pub struct NodeTaint {
    /// Taint key.
    pub key: String,
    /// Taint value.
    pub value: Option<String>,
    /// Taint effect: "NoSchedule", "PreferNoSchedule", "NoExecute".
    pub effect: String,
    /// Time when taint expires.
    pub time_added: Option<String>,
}

// ---------------------------------------------------------------------------
// Peer spec (mesh peer)
// ---------------------------------------------------------------------------

/// Peer configuration - defines a peer node in the mesh.
#[derive(Debug, Clone)]
pub struct PeerSpec {
    /// Node ID (public key) of the peer.
    pub node_id: String,
    /// Reachability hints - how to contact this peer.
    pub endpoints: Vec<PeerEndpoint>,
    /// Last seen timestamp (RFC3339).
    pub last_seen: Option<String>,
    /// Peer is currently reachable.
    pub reachable: bool,
    /// Roles advertised by peer.
    pub roles: Vec<String>,
}

/// Peer endpoint - transport-specific reachability.
#[derive(Debug, Clone)]
pub struct PeerEndpoint {
    /// Transport: "tcp", "quic", "ble", "http3".
    pub transport: String,
    /// Address (host:port) or URL.
    pub address: String,
    /// Is encrypted.
    pub encrypted: bool,
    /// Cost metric (lower is better).
    pub cost: Option<i32>,
}

// ---------------------------------------------------------------------------
// Gateway (proxy server)
// ---------------------------------------------------------------------------

/// Gateway specification - configures the proxy server.
#[derive(Debug, Clone)]
pub struct GatewaySpec {
    /// Listeners - ports and protocols to listen on.
    pub listeners: Vec<GatewayListener>,
    /// Default TLS config.
    pub tls: Option<GatewayTlsConfig>,
    /// Route selector - which routes this gateway handles.
    pub route_selector: Option<HashMap<String, String>>,
}

/// Gateway listener - a port/protocol combination.
#[derive(Debug, Clone)]
pub struct GatewayListener {
    /// Listener name.
    pub name: String,
    /// Port to listen on.
    pub port: i32,
    /// Protocol: HTTP, HTTPS, TCP, TLS.
    pub protocol: String,
    /// TLS config (for HTTPS/TLS listeners).
    pub tls: Option<GatewayTlsConfig>,
}

/// Gateway TLS configuration.
#[derive(Debug, Clone)]
pub struct GatewayTlsConfig {
    /// Secret reference for TLS cert.
    pub secret_ref: Option<String>,
    /// Min TLS version.
    pub min_version: String,
    /// Cipher suites.
    pub ciphers: Vec<String>,
}

// ---------------------------------------------------------------------------
// Service (backend discovery)
// ---------------------------------------------------------------------------

/// Service specification - selector-based container discovery.
#[derive(Debug, Clone)]
pub struct ServiceSpec {
    /// Selector - matches containers with these labels.
    pub selector: Option<HashMap<String, String>>,
    /// Ports to expose.
    pub ports: Vec<ServicePort>,
    /// Session affinity.
    pub affinity: ServiceAffinity,
}

/// Service port definition.
#[derive(Debug, Clone)]
pub struct ServicePort {
    /// Port number.
    pub port: i32,
    /// Target port on container.
    pub target_port: i32,
    /// Protocol: TCP, UDP.
    pub protocol: String,
    /// Port name.
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub enum ServiceAffinity {
    #[default]
    None,
    ClientIP,
    Cookie,
}

// ---------------------------------------------------------------------------
// HttpRoute (HTTP routing)
// ---------------------------------------------------------------------------

/// HttpRoute specification - HTTP routing rules.
#[derive(Debug, Clone)]
pub struct HttpRouteSpec {
    /// Parent gateway reference.
    pub parent_ref: Option<String>,
    /// Hostnames to match.
    pub hostnames: Vec<String>,
    /// Routing rules.
    pub rules: Vec<HttpRouteRule>,
}

/// HTTP route rule.
#[derive(Debug, Clone)]
pub struct HttpRouteRule {
    /// Path matches.
    pub matches: Vec<HttpRouteMatch>,
    /// Backend reference.
    pub backend: HttpRouteBackend,
}

/// HTTP route match.
#[derive(Debug, Clone)]
pub struct HttpRouteMatch {
    /// Path value to match.
    pub path: Option<String>,
    /// Path type: "Exact", "Prefix", "Regex".
    pub path_type: String,
    /// Header matches.
    pub headers: Vec<HttpRouteHeaderMatch>,
}

/// HTTP route header match.
#[derive(Debug, Clone)]
pub struct HttpRouteHeaderMatch {
    pub name: String,
    pub value: String,
    pub type_: String,
}

/// HTTP route backend.
#[derive(Debug, Clone)]
pub struct HttpRouteBackend {
    /// Service name.
    pub service: String,
    /// Service port.
    pub port: i32,
}

// ---------------------------------------------------------------------------
// TcpRoute (TCP routing)
// ---------------------------------------------------------------------------

/// TcpRoute specification - TCP routing rules.
#[derive(Debug, Clone)]
pub struct TcpRouteSpec {
    /// Parent gateway reference.
    pub parent_ref: Option<String>,
    /// Port to match.
    pub port: Option<i32>,
    /// Backend reference.
    pub backend: Option<TcpRouteBackend>,
}

/// TCP route backend.
#[derive(Debug, Clone)]
pub struct TcpRouteBackend {
    pub service: String,
    pub port: i32,
}

// ---------------------------------------------------------------------------
// TlsRoute (TLS passthrough)
// ---------------------------------------------------------------------------

/// TlsRoute specification - TLS passthrough routing.
#[derive(Debug, Clone)]
pub struct TlsRouteSpec {
    /// Parent gateway reference.
    pub parent_ref: Option<String>,
    /// SNI hostnames to match.
    pub sni_hostnames: Vec<String>,
    /// Backend reference.
    pub backend: Option<TlsRouteBackend>,
}

/// TLS route backend.
#[derive(Debug, Clone)]
pub struct TlsRouteBackend {
    pub service: String,
    pub port: i32,
}

// ---------------------------------------------------------------------------
// Container spec (K8s Pod-compatible OCI container)
// ---------------------------------------------------------------------------

/// Container workload specification.
/// Deployment spec - multiple containers with networking.
#[derive(Debug, Clone)]
pub struct DeploymentSpec {
    /// Deployment name.
    pub name: String,
    /// Namespace.
    pub namespace: String,
    /// Container specs to run.
    pub containers: Vec<Container>,
    /// Replicas per container.
    pub replicas: Option<i32>,
    /// Service networking between containers.
    pub service: Option<DeploymentService>,
}

#[derive(Debug, Clone, Default)]
pub struct DeploymentService {
    /// Ports exposed by container (container_name -> ports).
    pub ports: HashMap<String, Vec<u16>>,
}

/// Maps to K8s Pod spec with OCI container.
#[derive(Debug, Clone)]
pub struct ContainerSpec {
    /// Container image (OCI image reference).
    pub image: String,
    /// Image pull policy: "Always", "IfNotPresent", "Never".
    pub image_pull_policy: String,
    /// Container restart policy.
    pub restart_policy: ContainerRestartPolicy,
    /// Active deadline (seconds).
    pub active_deadline_seconds: Option<u64>,
    /// Service account name.
    pub service_account_name: Option<String>,
    /// Number of desired pods (replicas).
    pub replicas: Option<i32>,
    /// Selector for pods (label query).
    pub selector: Option<HashMap<String, String>>,
    /// Pod template spec.
    pub template: Option<PodTemplateSpec>,
}

#[derive(Debug, Clone)]
pub struct PodTemplateSpec {
    /// Standard object's metadata.
    pub metadata: Option<ResourceMetadata>,
    /// Pod specification.
    pub spec: Option<PodSpec>,
}

/// Pod specification (subset of K8s PodSpec).
#[derive(Debug, Clone)]
pub struct PodSpec {
    /// Containers in this pod.
    pub containers: Vec<Container>,
    /// Init containers.
    pub init_containers: Vec<Container>,
    /// Restart policy.
    pub restart_policy: ContainerRestartPolicy,
    /// Termination grace period (seconds).
    pub termination_grace_period_seconds: Option<i64>,
    /// DNS policy.
    pub dns_policy: String,
    /// Node selector.
    pub node_selector: Option<HashMap<String, String>>,
    /// Node name.
    pub node_name: Option<String>,
    /// Host network.
    pub host_network: bool,
    /// Host PID.
    pub host_pid: bool,
    /// Host IPC.
    pub host_ipc: bool,
    /// Share process namespace.
    pub share_process_namespace: bool,
    /// Security context.
    pub security_context: Option<PodSecurityContext>,
    /// Image pull secrets.
    pub image_pull_secrets: Vec<LocalObjectReference>,
    /// Volumes.
    pub volumes: Vec<Volume>,
    /// Affinity.
    pub affinity: Option<Affinity>,
    /// Tolerations.
    pub tolerations: Vec<Toleration>,
}

/// Container specification (from K8s Container).
#[derive(Debug, Clone)]
pub struct Container {
    /// Container name.
    pub name: String,
    /// Container image.
    pub image: Option<String>,
    /// Image pull policy.
    pub image_pull_policy: Option<String>,
    /// Command (entrypoint).
    pub command: Vec<String>,
    /// Args (command arguments).
    pub args: Vec<String>,
    /// Environment variables.
    pub env: Vec<EnvVar>,
    /// Environment from sources.
    pub env_from: Vec<EnvFromSource>,
    /// Volume mounts.
    pub volume_mounts: Vec<VolumeMount>,
    /// Ports.
    pub ports: Vec<ContainerPort>,
    /// Resources.
    pub resources: Option<ResourceRequirements>,
    /// Security context.
    pub security_context: Option<ContainerSecurityContext>,
    /// Liveness probe.
    pub liveness_probe: Option<Probe>,
    /// Readiness probe.
    pub readiness_probe: Option<Probe>,
    /// Startup probe.
    pub startup_probe: Option<Probe>,
    /// Lifecycle hook.
    pub lifecycle: Option<Lifecycle>,
    /// Working directory.
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub enum ContainerRestartPolicy {
    Always,
    #[default]
    OnFailure,
    Never,
}

#[derive(Debug, Clone)]
pub struct EnvVar {
    pub name: String,
    pub value: Option<String>,
    pub value_from: Option<EnvVarSource>,
}

#[derive(Debug, Clone)]
pub struct EnvVarSource {
    pub field_ref: Option<ObjectFieldSelector>,
    pub secret_ref: Option<SecretEnvSource>,
}

#[derive(Debug, Clone)]
pub struct ObjectFieldSelector {
    pub field_path: String,
}

#[derive(Debug, Clone)]
pub struct SecretEnvSource {
    pub name: String,
    pub optional: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct EnvFromSource {
    pub prefix: Option<String>,
    pub config_map_ref: Option<ConfigMapEnvSource>,
    pub secret_ref: Option<SecretEnvSource>,
}

#[derive(Debug, Clone)]
pub struct ConfigMapEnvSource {
    pub name: String,
    pub optional: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct VolumeMount {
    pub name: String,
    pub mount_path: String,
    pub read_only: Option<bool>,
    pub sub_path: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContainerPort {
    pub name: Option<String>,
    pub container_port: i32,
    pub protocol: Option<String>,
    pub host_port: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ResourceRequirements {
    pub limits: Option<HashMap<String, String>>,
    pub requests: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Default)]
pub struct ResourceLimits {
    pub cpu_cores: Option<u32>,
    pub memory_bytes: Option<u64>,
    pub storage_bytes: Option<u64>,
    pub network_mbps: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ContainerSecurityContext {
    pub privileged: Option<bool>,
    pub run_as_user: Option<i64>,
    pub run_as_non_root: Option<bool>,
    pub capabilities: Option<Capabilities>,
    pub seccomp_profile: Option<SeccompProfile>,
}

#[derive(Debug, Clone)]
pub struct Capabilities {
    pub add: Vec<String>,
    pub drop: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SeccompProfile {
    pub type_: Option<String>,
    pub localhost_profile: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PodSecurityContext {
    pub run_as_user: Option<i64>,
    pub run_as_non_root: Option<bool>,
    pub run_as_group: Option<i64>,
    pub fs_group: Option<i64>,
    pub supplemental_groups: Option<Vec<i64>>,
    pub seccomp_profile: Option<SeccompProfile>,
    pub sysctls: Option<Vec<Sysctl>>,
}

#[derive(Debug, Clone)]
pub struct Sysctl {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct LocalObjectReference {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct Volume {
    pub name: String,
    pub empty_dir: Option<EmptyDirVolumeSource>,
    pub config_map: Option<ConfigMapVolumeSource>,
    pub secret: Option<SecretVolumeSource>,
    pub persistent_volume_claim: Option<PersistentVolumeClaimVolumeSource>,
    pub host_path: Option<HostPathVolumeSource>,
}

#[derive(Debug, Clone)]
pub struct EmptyDirVolumeSource {
    pub medium: Option<String>,
    pub size_limit: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConfigMapVolumeSource {
    pub name: String,
    pub items: Option<Vec<KeyToPath>>,
    pub optional: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct SecretVolumeSource {
    pub secret_name: String,
    pub items: Option<Vec<KeyToPath>>,
    pub optional: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct PersistentVolumeClaimVolumeSource {
    pub claim_name: String,
    pub read_only: bool,
}

#[derive(Debug, Clone)]
pub struct HostPathVolumeSource {
    pub path: String,
    pub type_: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KeyToPath {
    pub key: String,
    pub path: String,
    pub mode: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct Affinity {
    pub node_affinity: Option<NodeAffinity>,
    pub pod_affinity: Option<PodAffinity>,
    pub pod_anti_affinity: Option<PodAntiAffinity>,
}

#[derive(Debug, Clone)]
pub struct NodeAffinity {
    pub preferred_during_scheduling: Option<Vec<PreferredSchedulingTerm>>,
    pub required_during_scheduling: Option<NodeSelector>,
}

#[derive(Debug, Clone)]
pub struct PreferredSchedulingTerm {
    pub weight: i32,
    pub preference: NodeSelectorTerm,
}

#[derive(Debug, Clone)]
pub struct NodeSelector {
    pub node_selector_terms: Vec<NodeSelectorTerm>,
}

#[derive(Debug, Clone)]
pub struct NodeSelectorTerm {
    pub match_expressions: Vec<NodeSelectorRequirement>,
    pub match_fields: Vec<NodeSelectorRequirement>,
}

#[derive(Debug, Clone)]
pub struct NodeSelectorRequirement {
    pub key: String,
    pub operator: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PodAffinity {
    pub required_during_scheduling: Vec<PodAffinityTerm>,
    pub preferred_during_scheduling: Vec<WeightedPodAffinityTerm>,
}

#[derive(Debug, Clone)]
pub struct PodAntiAffinity {
    pub required_during_scheduling: Vec<PodAffinityTerm>,
    pub preferred_during_scheduling: Vec<WeightedPodAffinityTerm>,
}

#[derive(Debug, Clone)]
pub struct PodAffinityTerm {
    pub label_selector: Option<ResourceMetadata>,
    pub namespaces: Vec<String>,
    pub topology_key: String,
}

#[derive(Debug, Clone)]
pub struct WeightedPodAffinityTerm {
    pub weight: i32,
    pub pod_affinity_term: PodAffinityTerm,
}

#[derive(Debug, Clone)]
pub struct Toleration {
    pub key: Option<String>,
    pub operator: Option<String>,
    pub value: Option<String>,
    pub effect: Option<String>,
    pub toleration_seconds: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct Probe {
    pub exec: Option<ExecAction>,
    pub http_get: Option<HTTPGetAction>,
    pub tcp_socket: Option<TCPSocketAction>,
    pub grpc: Option<GRPCAction>,
    pub initial_delay_seconds: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub period_seconds: Option<i32>,
    pub success_threshold: Option<i32>,
    pub failure_threshold: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ExecAction {
    pub command: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct HTTPGetAction {
    pub path: Option<String>,
    pub port: i32,
    pub host: Option<String>,
    pub scheme: Option<String>,
    pub http_headers: Vec<HTTPHeader>,
}

#[derive(Debug, Clone)]
pub struct HTTPHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct TCPSocketAction {
    pub port: i32,
    pub host: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GRPCAction {
    pub port: i32,
    pub service: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Lifecycle {
    pub post_start: Option<LifecycleHandler>,
    pub pre_stop: Option<LifecycleHandler>,
}

#[derive(Debug, Clone)]
pub struct LifecycleHandler {
    pub exec: Option<ExecAction>,
    pub http_get: Option<HTTPGetAction>,
}

// ---------------------------------------------------------------------------
// Secret spec (K8s Secret-compatible)
// ---------------------------------------------------------------------------

/// Secret specification - stores encrypted data reference in blob store.
#[derive(Debug, Clone)]
pub struct SecretSpec {
    /// Secret data (base64-encoded when serialized).
    /// Stored encrypted in blob store, not in config.
    pub data: Option<HashMap<String, Vec<u8>>>,
    /// Secret string data (plaintext when serialized).
    pub string_data: Option<HashMap<String, String>>,
    /// Secret type: "Opaque", "kubernetes.io/tls", "kubernetes.io/dockerconfigjson", etc.
    pub secret_type: SecretType,
    /// Reference to blob store for the actual encrypted data.
    pub blob_ref: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub enum SecretType {
    #[default]
    Opaque,
    ServiceAccountToken,
    DockerConfigJson,
    TLS,
    BootstrapToken,
}

macro_rules! impl_config_json_struct {
    (
        $ty:ty {
            required { $($req_field:ident : $req_key:expr => $req_ty:ty),* $(,)? }
            optional { $($opt_field:ident : $opt_key:expr => $opt_ty:ty),* $(,)? }
            default { $($def_field:ident : $def_key:expr => $def_ty:ty),* $(,)? }
            default_with { $($with_field:ident : $with_key:expr => $with_ty:ty = $with_fn:path),* $(,)? }
        }
    ) => {
        impl edgerun_json::ToJson for $ty {
            fn to_json(&self) -> edgerun_json::JsonValue {
                let mut object = edgerun_json::Map::new();
                $(object.push_field($req_key, edgerun_json::ToJson::to_json(&self.$req_field));)*
                $(if let Some(value) = &self.$opt_field {
                    object.push_field($opt_key, edgerun_json::ToJson::to_json(value));
                })*
                $(object.push_field($def_key, edgerun_json::ToJson::to_json(&self.$def_field));)*
                $(object.push_field($with_key, edgerun_json::ToJson::to_json(&self.$with_field));)*
                object.into()
            }
        }

        impl edgerun_json::FromJson for $ty {
            fn from_json(value: edgerun_json::JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
                let mut object = match value {
                    edgerun_json::JsonValue::Object(object) => object,
                    other => {
                        return Err(edgerun_json::JsonValueError::WrongType(format!(
                            "expected object for {}, found {:?}",
                            stringify!($ty),
                            other
                        )));
                    }
                };
                Ok(Self {
                    $($req_field: edgerun_json::FromJson::from_json(
                        object.remove($req_key).ok_or_else(|| {
                            edgerun_json::JsonValueError::WrongType(format!(
                                "missing field `{}` for {}",
                                $req_key,
                                stringify!($ty)
                            ))
                        })?
                    )?,)*
                    $($opt_field: object
                        .remove($opt_key)
                        .map(edgerun_json::FromJson::from_json)
                        .transpose()?,)*
                    $($def_field: object
                        .remove($def_key)
                        .map(edgerun_json::FromJson::from_json)
                        .transpose()?
                        .unwrap_or_default(),)*
                    $($with_field: object
                        .remove($with_key)
                        .map(edgerun_json::FromJson::from_json)
                        .transpose()?
                        .unwrap_or_else($with_fn),)*
                })
            }
        }
    };
}

impl_config_json_struct! {
    ResourceMetadata {
        required { name: "name" => String }
        optional {
            namespace: "namespace" => String,
            labels: "labels" => HashMap<String, String>,
            annotations: "annotations" => HashMap<String, String>,
        }
        default {}
        default_with {}
    }
}

impl<T: edgerun_json::ToJson> edgerun_json::ToJson for Resource<T> {
    fn to_json(&self) -> edgerun_json::JsonValue {
        let mut object = edgerun_json::Map::new();
        object.push_field("apiVersion", &self.api_version);
        object.push_field("kind", &self.kind);
        object.push_field("metadata", self.metadata.to_json());
        object.push_field("spec", self.spec.to_json());
        object.into()
    }
}

impl<T: edgerun_json::FromJson> edgerun_json::FromJson for Resource<T> {
    fn from_json(value: edgerun_json::JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let mut object = match value {
            edgerun_json::JsonValue::Object(object) => object,
            _ => {
                return Err(edgerun_json::JsonValueError::WrongType(
                    "expected resource object".to_string(),
                ));
            }
        };
        Ok(Self {
            api_version: object
                .remove("apiVersion")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_else(|| API_VERSION.to_string()),
            kind: edgerun_json::FromJson::from_json(object.remove("kind").ok_or_else(|| {
                edgerun_json::JsonValueError::WrongType("missing field `kind`".to_string())
            })?)?,
            metadata: object
                .remove("metadata")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_else(|| ResourceMetadata {
                    name: String::new(),
                    namespace: None,
                    labels: None,
                    annotations: None,
                }),
            spec: edgerun_json::FromJson::from_json(object.remove("spec").ok_or_else(|| {
                edgerun_json::JsonValueError::WrongType("missing field `spec`".to_string())
            })?)?,
        })
    }
}

impl_config_json_struct! {
    DnsServerSpec {
        required {}
        optional {
            bind_address: "bind_address" => String,
            bind_address_ipv6: "bind_address_ipv6" => String,
            default_ttl: "default_ttl" => u32,
            rate_limit_qps: "rate_limit_qps" => u32,
            zones: "zones" => Vec<String>,
            forward_to: "forward_to" => String,
            recursive: "recursive" => bool,
        }
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    DnsZoneSpec {
        required { origin: "origin" => String, soa: "soa" => SoaRecord }
        optional {
            dnssec: "dnssec" => DnssecConfig,
            wildcards: "wildcards" => Vec<ZoneRecord>,
        }
        default { records: "records" => Vec<ZoneRecord> }
        default_with {}
    }
}

impl_config_json_struct! {
    SoaRecord {
        required { mname: "mname" => String, rname: "rname" => String }
        optional {}
        default {}
        default_with {
            serial: "serial" => u32 = default_serial,
            refresh: "refresh" => u32 = default_3600,
            retry: "retry" => u32 = default_900,
            expire: "expire" => u32 = default_604800,
            minimum: "minimum" => u32 = default_86400,
        }
    }
}

impl_config_json_struct! {
    ZoneRecord {
        required {
            name: "name" => String,
            record_type: "type" => String,
            value: "value" => edgerun_json::JsonValue,
        }
        optional { ttl: "ttl" => u32 }
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    DnssecConfig {
        required { algorithm: "algorithm" => String }
        optional { nsec3_salt: "nsec3_salt" => String }
        default { nsec3: "nsec3" => bool, nsec3_iterations: "nsec3_iterations" => u16 }
        default_with {
            key_flags: "key_flags" => u16 = default_ksk,
            key_ttl: "key_ttl" => u32 = default_86400,
            signature_validity: "signature_validity" => u32 = default_30d,
        }
    }
}

impl_config_json_struct! {
    DnsForwarderSpec {
        required { upstreams: "upstreams" => Vec<String> }
        optional {
            bind_address: "bind_address" => String,
            timeout: "timeout" => u64,
            cache_ttl: "cache_ttl" => u32,
            cache_max_entries: "cache_max_entries" => usize,
        }
        default { cache: "cache" => bool }
        default_with {}
    }
}

impl_config_json_struct! {
    ForwardingRuleSpec {
        required { zone: "zone" => String, upstreams: "upstreams" => Vec<String> }
        optional { policy: "policy" => String, health_check: "health_check" => u64 }
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    TlsConfigSpec {
        required {}
        optional {
            doh_bind_address: "doh_bind_address" => String,
            doh_path: "doh_path" => String,
            cert_path: "cert_path" => String,
            key_path: "key_path" => String,
        }
        default { dot_enabled: "dot_enabled" => bool, doh_enabled: "doh_enabled" => bool }
        default_with {}
    }
}

impl_config_json_struct! {
    RateLimitSpec {
        required {}
        optional {}
        default {}
        default_with {
            qps: "qps" => u32 = default_100,
            burst: "burst" => u32 = default_200,
            block_duration: "block_duration" => u32 = default_60,
        }
    }
}

impl_config_json_struct! {
    DhcpServerSpec {
        required { interface: "interface" => String, pools: "pools" => Vec<String> }
        optional {
            max_lease_time: "max_lease_time" => u32,
            dns_servers: "dns_servers" => Vec<String>,
            router: "router" => String,
            ntp_servers: "ntp_servers" => Vec<String>,
            domain_name: "domain_name" => String,
            bootfile: "bootfile" => String,
            tftp_server: "tftp_server" => String,
            reservations: "reservations" => Vec<DhcpReservation>,
        }
        default {}
        default_with { default_lease_time: "default_lease_time" => u32 = default_86400 }
    }
}

impl_config_json_struct! {
    DhcpReservation {
        required { mac: "mac" => String, ip: "ip" => String }
        optional { hostname: "hostname" => String }
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    Dhcpv6ServerSpec {
        required { interface: "interface" => String, pools: "pools" => Vec<String> }
        optional {
            dns_servers: "dns_servers" => Vec<String>,
            domain_name: "domain_name" => String,
            reservations: "reservations" => Vec<Dhcpv6Reservation>,
        }
        default {}
        default_with {
            default_preferred_lifetime: "default_preferred_lifetime" => u32 = default_3600,
            default_valid_lifetime: "default_valid_lifetime" => u32 = default_7200,
        }
    }
}

impl_config_json_struct! {
    Dhcpv6Reservation {
        required { duid: "duid" => String, ip: "ip" => String }
        optional { hostname: "hostname" => String }
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    Dhcpv6PoolSpec {
        required {
            name: "name" => String,
            range_start: "range_start" => String,
            range_end: "range_end" => String,
            prefix_length: "prefix_length" => u8,
        }
        optional { exclude: "exclude" => Vec<String> }
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    DhcpPoolSpec {
        required {
            name: "name" => String,
            range_start: "range_start" => String,
            range_end: "range_end" => String,
            subnet_mask: "subnet_mask" => String,
        }
        optional { exclude: "exclude" => Vec<String> }
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    TftpServerSpec {
        required { root_dir: "root_dir" => String }
        optional {
            bind_address: "bind_address" => String,
            block_size: "block_size" => u16,
            timeout: "timeout" => u32,
        }
        default { allow_writes: "allow_writes" => bool }
        default_with {}
    }
}

impl_config_json_struct! {
    SmtpServerSpec {
        required { hostname: "hostname" => String }
        optional {
            bind_address: "bind_address" => String,
            max_message_size: "max_message_size" => usize,
            maildir_root: "maildir_root" => String,
            queue_dir: "queue_dir" => String,
            dns_server: "dns_server" => String,
            dkim_domain: "dkim_domain" => String,
            dkim_selector: "dkim_selector" => String,
            dkim_key_path: "dkim_key_path" => String,
            tls_cert: "tls_cert" => String,
            tls_key: "tls_key" => String,
            users: "users" => Vec<MailUserSpec>,
            catch_all_user: "catch_all_user" => String,
            acme_directory: "acme_directory" => String,
            acme_contact_email: "acme_contact_email" => String,
            acme_domains: "acme_domains" => Vec<String>,
            acme_account_key_path: "acme_account_key_path" => String,
            acme_cert_dir: "acme_cert_dir" => String,
        }
        default {
            smtps: "smtps" => bool,
            starttls: "starttls" => bool,
            local_domains: "local_domains" => Vec<String>,
            relay_enabled: "relay_enabled" => bool,
            acme_enabled: "acme_enabled" => bool,
        }
        default_with {}
    }
}

impl_config_json_struct! {
    ImapServerSpec {
        required { hostname: "hostname" => String }
        optional {
            bind_address: "bind_address" => String,
            maildir_root: "maildir_root" => String,
            tls_cert: "tls_cert" => String,
            tls_key: "tls_key" => String,
            users: "users" => Vec<MailUserSpec>,
        }
        default { imaps: "imaps" => bool }
        default_with {}
    }
}

impl_config_json_struct! {
    MailUserSpec {
        required { username: "username" => String }
        optional {
            password: "password" => String,
            domains: "domains" => Vec<String>,
        }
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    NodeSpec {
        required {}
        optional {}
        default {
            controllers: "controllers" => Vec<String>,
            roles: "roles" => Vec<String>,
            public_ips: "public_ips" => Vec<String>,
            taints: "taints" => Vec<NodeTaint>,
            unschedulable: "unschedulable" => bool,
        }
        default_with {}
    }
}

impl_config_json_struct! {
    NodeTaint {
        required { key: "key" => String, effect: "effect" => String }
        optional { value: "value" => String, time_added: "time_added" => String }
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    PeerSpec {
        required { node_id: "node_id" => String }
        optional { last_seen: "last_seen" => String }
        default {
            endpoints: "endpoints" => Vec<PeerEndpoint>,
            reachable: "reachable" => bool,
            roles: "roles" => Vec<String>,
        }
        default_with {}
    }
}

impl_config_json_struct! {
    PeerEndpoint {
        required { transport: "transport" => String, address: "address" => String }
        optional { cost: "cost" => i32 }
        default { encrypted: "encrypted" => bool }
        default_with {}
    }
}

impl_config_json_struct! {
    GatewaySpec {
        required {}
        optional {
            tls: "tls" => GatewayTlsConfig,
            route_selector: "route_selector" => HashMap<String, String>,
        }
        default { listeners: "listeners" => Vec<GatewayListener> }
        default_with {}
    }
}

impl_config_json_struct! {
    GatewayListener {
        required { name: "name" => String, port: "port" => i32, protocol: "protocol" => String }
        optional { tls: "tls" => GatewayTlsConfig }
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    GatewayTlsConfig {
        required {}
        optional { secret_ref: "secret_ref" => String }
        default { min_version: "min_version" => String, ciphers: "ciphers" => Vec<String> }
        default_with {}
    }
}

impl_config_json_struct! {
    ServiceSpec {
        required {}
        optional { selector: "selector" => HashMap<String, String> }
        default { ports: "ports" => Vec<ServicePort>, affinity: "affinity" => ServiceAffinity }
        default_with {}
    }
}

impl_config_json_struct! {
    ServicePort {
        required { port: "port" => i32, target_port: "target_port" => i32 }
        optional { name: "name" => String }
        default { protocol: "protocol" => String }
        default_with {}
    }
}

impl edgerun_json::ToJson for ServiceAffinity {
    fn to_json(&self) -> edgerun_json::JsonValue {
        match self {
            Self::None => "None",
            Self::ClientIP => "ClientIP",
            Self::Cookie => "Cookie",
        }
        .into()
    }
}

impl edgerun_json::FromJson for ServiceAffinity {
    fn from_json(value: edgerun_json::JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        match <String as edgerun_json::FromJson>::from_json(value)?.as_str() {
            "None" => Ok(Self::None),
            "ClientIP" => Ok(Self::ClientIP),
            "Cookie" => Ok(Self::Cookie),
            other => Err(edgerun_json::JsonValueError::WrongType(format!(
                "unknown service affinity `{other}`"
            ))),
        }
    }
}

impl_config_json_struct! {
    HttpRouteSpec {
        required {}
        optional { parent_ref: "parent_ref" => String }
        default { hostnames: "hostnames" => Vec<String>, rules: "rules" => Vec<HttpRouteRule> }
        default_with {}
    }
}

impl_config_json_struct! {
    HttpRouteRule {
        required { backend: "backend" => HttpRouteBackend }
        optional {}
        default { matches: "matches" => Vec<HttpRouteMatch> }
        default_with {}
    }
}

impl_config_json_struct! {
    HttpRouteMatch {
        required {}
        optional { path: "path" => String }
        default { path_type: "path_type" => String, headers: "headers" => Vec<HttpRouteHeaderMatch> }
        default_with {}
    }
}

impl_config_json_struct! {
    HttpRouteHeaderMatch {
        required { name: "name" => String, value: "value" => String, type_: "type" => String }
        optional {}
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    HttpRouteBackend {
        required { service: "service" => String, port: "port" => i32 }
        optional {}
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    TcpRouteSpec {
        required {}
        optional { parent_ref: "parent_ref" => String, port: "port" => i32, backend: "backend" => TcpRouteBackend }
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    TcpRouteBackend {
        required { service: "service" => String, port: "port" => i32 }
        optional {}
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    TlsRouteSpec {
        required {}
        optional { parent_ref: "parent_ref" => String, backend: "backend" => TlsRouteBackend }
        default { sni_hostnames: "sni_hostnames" => Vec<String> }
        default_with {}
    }
}

impl_config_json_struct! {
    TlsRouteBackend {
        required { service: "service" => String, port: "port" => i32 }
        optional {}
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    DeploymentSpec {
        required { name: "name" => String }
        optional { replicas: "replicas" => i32, service: "service" => DeploymentService }
        default { namespace: "namespace" => String, containers: "containers" => Vec<Container> }
        default_with {}
    }
}

impl_config_json_struct! {
    DeploymentService {
        required {}
        optional {}
        default { ports: "ports" => HashMap<String, Vec<u16>> }
        default_with {}
    }
}

impl_config_json_struct! {
    ContainerSpec {
        required { image: "image" => String }
        optional {
            active_deadline_seconds: "active_deadline_seconds" => u64,
            service_account_name: "service_account_name" => String,
            replicas: "replicas" => i32,
            selector: "selector" => HashMap<String, String>,
            template: "template" => PodTemplateSpec,
        }
        default { restart_policy: "restart_policy" => ContainerRestartPolicy }
        default_with { image_pull_policy: "image_pull_policy" => String = default_pull_always }
    }
}

impl_config_json_struct! {
    PodTemplateSpec {
        required {}
        optional { metadata: "metadata" => ResourceMetadata, spec: "spec" => PodSpec }
        default {}
        default_with {}
    }
}

impl edgerun_json::ToJson for PodSpec {
    fn to_json(&self) -> edgerun_json::JsonValue {
        let mut object = edgerun_json::Map::new();
        object.push_field("containers", self.containers.to_json());
        object.push_field("init_containers", self.init_containers.to_json());
        object.push_field("restart_policy", self.restart_policy.to_json());
        if let Some(value) = &self.termination_grace_period_seconds {
            object.push_field("termination_grace_period_seconds", *value);
        }
        object.push_field("dns_policy", &self.dns_policy);
        if let Some(value) = &self.node_selector {
            object.push_field("node_selector", value.to_json());
        }
        if let Some(value) = &self.node_name {
            object.push_field("node_name", value);
        }
        object.push_field("host_network", self.host_network);
        object.push_field("host_pid", self.host_pid);
        object.push_field("host_ipc", self.host_ipc);
        object.push_field("share_process_namespace", self.share_process_namespace);
        object.into()
    }
}

impl edgerun_json::FromJson for PodSpec {
    fn from_json(value: edgerun_json::JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let mut object = match value {
            edgerun_json::JsonValue::Object(object) => object,
            _ => {
                return Err(edgerun_json::JsonValueError::WrongType(
                    "expected PodSpec object".to_string(),
                ))
            }
        };
        Ok(Self {
            containers: object
                .remove("containers")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_default(),
            init_containers: object
                .remove("init_containers")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_default(),
            restart_policy: object
                .remove("restart_policy")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_default(),
            termination_grace_period_seconds: object
                .remove("termination_grace_period_seconds")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?,
            dns_policy: object
                .remove("dns_policy")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_else(default_dns_policy),
            node_selector: object
                .remove("node_selector")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?,
            node_name: object
                .remove("node_name")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?,
            host_network: object
                .remove("host_network")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_default(),
            host_pid: object
                .remove("host_pid")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_default(),
            host_ipc: object
                .remove("host_ipc")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_default(),
            share_process_namespace: object
                .remove("share_process_namespace")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_default(),
            security_context: None,
            image_pull_secrets: Vec::new(),
            volumes: Vec::new(),
            affinity: None,
            tolerations: Vec::new(),
        })
    }
}

impl edgerun_json::ToJson for Container {
    fn to_json(&self) -> edgerun_json::JsonValue {
        let mut object = edgerun_json::Map::new();
        object.push_field("name", &self.name);
        if let Some(value) = &self.image {
            object.push_field("image", value);
        }
        if let Some(value) = &self.image_pull_policy {
            object.push_field("image_pull_policy", value);
        }
        object.push_field("command", self.command.to_json());
        object.push_field("args", self.args.to_json());
        object.push_field("env", self.env.to_json());
        object.push_field("volume_mounts", self.volume_mounts.to_json());
        object.push_field("ports", self.ports.to_json());
        if let Some(value) = &self.resources {
            object.push_field("resources", value.to_json());
        }
        if let Some(value) = &self.working_dir {
            object.push_field("working_dir", value);
        }
        object.into()
    }
}

impl edgerun_json::FromJson for Container {
    fn from_json(value: edgerun_json::JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let mut object = match value {
            edgerun_json::JsonValue::Object(object) => object,
            _ => {
                return Err(edgerun_json::JsonValueError::WrongType(
                    "expected Container object".to_string(),
                ))
            }
        };
        Ok(Self {
            name: edgerun_json::FromJson::from_json(object.remove("name").ok_or_else(|| {
                edgerun_json::JsonValueError::WrongType("missing field `name`".to_string())
            })?)?,
            image: object
                .remove("image")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?,
            image_pull_policy: object
                .remove("image_pull_policy")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?,
            command: object
                .remove("command")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_default(),
            args: object
                .remove("args")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_default(),
            env: object
                .remove("env")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_default(),
            env_from: Vec::new(),
            volume_mounts: object
                .remove("volume_mounts")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_default(),
            ports: object
                .remove("ports")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?
                .unwrap_or_default(),
            resources: object
                .remove("resources")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?,
            security_context: None,
            liveness_probe: None,
            readiness_probe: None,
            startup_probe: None,
            lifecycle: None,
            working_dir: object
                .remove("working_dir")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?,
        })
    }
}

impl edgerun_json::ToJson for EnvVar {
    fn to_json(&self) -> edgerun_json::JsonValue {
        let mut object = edgerun_json::Map::new();
        object.push_field("name", &self.name);
        if let Some(value) = &self.value {
            object.push_field("value", value);
        }
        object.into()
    }
}

impl edgerun_json::FromJson for EnvVar {
    fn from_json(value: edgerun_json::JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        let mut object = match value {
            edgerun_json::JsonValue::Object(object) => object,
            _ => {
                return Err(edgerun_json::JsonValueError::WrongType(
                    "expected EnvVar object".to_string(),
                ))
            }
        };
        Ok(Self {
            name: edgerun_json::FromJson::from_json(object.remove("name").ok_or_else(|| {
                edgerun_json::JsonValueError::WrongType("missing field `name`".to_string())
            })?)?,
            value: object
                .remove("value")
                .map(edgerun_json::FromJson::from_json)
                .transpose()?,
            value_from: None,
        })
    }
}

impl_config_json_struct! {
    VolumeMount {
        required { name: "name" => String, mount_path: "mount_path" => String }
        optional { read_only: "read_only" => bool, sub_path: "sub_path" => String }
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    ContainerPort {
        required { container_port: "container_port" => i32 }
        optional {
            name: "name" => String,
            protocol: "protocol" => String,
            host_port: "host_port" => i32,
        }
        default {}
        default_with {}
    }
}

impl_config_json_struct! {
    ResourceRequirements {
        required {}
        optional {
            limits: "limits" => HashMap<String, String>,
            requests: "requests" => HashMap<String, String>,
        }
        default {}
        default_with {}
    }
}

impl edgerun_json::ToJson for ContainerRestartPolicy {
    fn to_json(&self) -> edgerun_json::JsonValue {
        match self {
            Self::Always => "Always",
            Self::OnFailure => "OnFailure",
            Self::Never => "Never",
        }
        .into()
    }
}

impl edgerun_json::FromJson for ContainerRestartPolicy {
    fn from_json(value: edgerun_json::JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        match <String as edgerun_json::FromJson>::from_json(value)?.as_str() {
            "Always" => Ok(Self::Always),
            "OnFailure" => Ok(Self::OnFailure),
            "Never" => Ok(Self::Never),
            other => Err(edgerun_json::JsonValueError::WrongType(format!(
                "unknown restart policy `{other}`"
            ))),
        }
    }
}

impl_config_json_struct! {
    SecretSpec {
        required {}
        optional {
            data: "data" => HashMap<String, Vec<u8>>,
            string_data: "string_data" => HashMap<String, String>,
            blob_ref: "blob_ref" => String,
        }
        default { secret_type: "secret_type" => SecretType }
        default_with {}
    }
}

impl edgerun_json::ToJson for SecretType {
    fn to_json(&self) -> edgerun_json::JsonValue {
        match self {
            Self::Opaque => "Opaque",
            Self::ServiceAccountToken => "ServiceAccountToken",
            Self::DockerConfigJson => "DockerConfigJson",
            Self::TLS => "TLS",
            Self::BootstrapToken => "BootstrapToken",
        }
        .into()
    }
}

impl edgerun_json::FromJson for SecretType {
    fn from_json(value: edgerun_json::JsonValue) -> Result<Self, edgerun_json::JsonValueError> {
        match <String as edgerun_json::FromJson>::from_json(value)?.as_str() {
            "Opaque" => Ok(Self::Opaque),
            "ServiceAccountToken" => Ok(Self::ServiceAccountToken),
            "DockerConfigJson" => Ok(Self::DockerConfigJson),
            "TLS" => Ok(Self::TLS),
            "BootstrapToken" => Ok(Self::BootstrapToken),
            other => Err(edgerun_json::JsonValueError::WrongType(format!(
                "unknown secret type `{other}`"
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// Default helpers
// ---------------------------------------------------------------------------

fn default_serial() -> u32 {
    let now = crate::time::SystemTime::now()
        .duration_since(crate::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    ((secs / 86400) as u32) * 100 + 1 // YYYYMMDDNN format
}

fn default_900() -> u32 {
    900
}
fn default_604800() -> u32 {
    604800
}
fn default_86400() -> u32 {
    86400
}
fn default_30d() -> u32 {
    30 * 86400
}
fn default_ksk() -> u16 {
    257
}
fn default_100() -> u32 {
    100
}
fn default_200() -> u32 {
    200
}
fn default_60() -> u32 {
    60
}
fn default_pull_always() -> String {
    "Always".to_string()
}
fn default_dns_policy() -> String {
    "ClusterFirst".to_string()
}
