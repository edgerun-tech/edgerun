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

use std::collections::HashMap;

// ---------------------------------------------------------------------------
// K8s-compatible resource envelope
// ---------------------------------------------------------------------------

/// The API version for all edgerun config resources.
pub const API_VERSION: &str = "edgerun.io/v1alpha1";

/// Standard K8s-style metadata.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceMetadata {
    /// Resource name (unique within namespace).
    pub name: String,
    /// Namespace for grouping resources.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Key-value labels for selection and filtering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<HashMap<String, String>>,
    /// Arbitrary annotations (for converters, notes, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<HashMap<String, String>>,
}

/// A complete K8s-style config resource envelope.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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

impl<T: serde::Serialize + serde::de::DeserializeOwned> Resource<T> {
    /// Serialize to YAML string.
    pub fn to_yaml(&self) -> Result<String, edgerun_json::yaml::YamlError> {
        let json = edgerun_json::to_value(self).map_err(|_| {
            edgerun_json::yaml::YamlError::IoError("serialization error".to_string())
        })?;

        edgerun_json::yaml::to_yaml_string(&edgerun_json::yaml::json_to_yaml(json))
    }

    /// Parse from YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, edgerun_json::yaml::YamlError> {
        let yaml_value = edgerun_json::yaml::from_yaml_str(yaml)?;
        let json = edgerun_json::yaml::yaml_to_json(yaml_value);
        edgerun_json::from_value(json).map_err(|_| {
            edgerun_json::yaml::YamlError::IoError("deserialization error".to_string())
        })
    }
}

/// K8s-style config resource — wraps raw spec types with the envelope.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "PascalCase")]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DnsServerSpec {
    /// UDP/TCP bind address (default: "0.0.0.0:53").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_address: Option<String>,
    /// IPv6 bind address (e.g. "[::]:53").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_address_ipv6: Option<String>,
    /// Default TTL for records (seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_ttl: Option<u32>,
    /// Rate limit queries per second per IP (0 = unlimited).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_qps: Option<u32>,
    /// List of zone names this server serves.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zones: Option<Vec<String>>,
    /// Upstream resolver for forwarding.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forward_to: Option<String>,
    /// Enable recursive resolution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recursive: Option<bool>,
}

// ---------------------------------------------------------------------------
// DNS Zone spec
// ---------------------------------------------------------------------------

/// DNS zone configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DnsZoneSpec {
    /// Zone origin (e.g. "example.com").
    pub origin: String,
    /// SOA record.
    pub soa: SoaRecord,
    /// DNS records in the zone.
    #[serde(default)]
    pub records: Vec<ZoneRecord>,
    /// DNSSEC signing config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dnssec: Option<DnssecConfig>,
    /// Wildcard records.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wildcards: Option<Vec<ZoneRecord>>,
}

/// SOA record configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SoaRecord {
    /// Primary nameserver.
    pub mname: String,
    /// Responsible admin email (with `@` replaced by `.`).
    pub rname: String,
    /// Zone serial number.
    #[serde(default = "default_serial")]
    pub serial: u32,
    /// Refresh interval (seconds).
    #[serde(default = "default_3600")]
    pub refresh: u32,
    /// Retry interval (seconds).
    #[serde(default = "default_900")]
    pub retry: u32,
    /// Expiry time (seconds).
    #[serde(default = "default_604800")]
    pub expire: u32,
    /// Minimum TTL / negative cache TTL (seconds).
    #[serde(default = "default_86400")]
    pub minimum: u32,
}

/// A single DNS record in a zone.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ZoneRecord {
    /// Record name (relative to zone origin, or FQDN).
    pub name: String,
    /// Record type (A, AAAA, CNAME, MX, TXT, SRV, etc.).
    #[serde(rename = "type")]
    pub record_type: String,
    /// TTL in seconds (defaults to zone default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u32>,
    /// Record value — type-specific.
    pub value: edgerun_json::JsonValue,
}

/// DNSSEC configuration for a zone.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DnssecConfig {
    /// Algorithm: "ed25519", "ecdsap256".
    pub algorithm: String,
    /// Key flags: 256 (ZSK), 257 (KSK).
    #[serde(default = "default_ksk")]
    pub key_flags: u16,
    /// Key TTL (seconds).
    #[serde(default = "default_86400")]
    pub key_ttl: u32,
    /// Signature validity period (seconds from now).
    #[serde(default = "default_30d")]
    pub signature_validity: u32,
    /// Enable NSEC3.
    #[serde(default)]
    pub nsec3: bool,
    /// NSEC3 salt (hex string).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nsec3_salt: Option<String>,
    /// NSEC3 iterations.
    #[serde(default)]
    pub nsec3_iterations: u16,
}

// ---------------------------------------------------------------------------
// DNS Forwarder spec
// ---------------------------------------------------------------------------

/// DNS forwarder configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DnsForwarderSpec {
    /// Bind address for the forwarder.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_address: Option<String>,
    /// List of upstream resolvers.
    pub upstreams: Vec<String>,
    /// Query timeout (seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,
    /// Enable caching.
    #[serde(default)]
    pub cache: bool,
    /// Cache TTL (seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_ttl: Option<u32>,
    /// Max cache entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_max_entries: Option<usize>,
}

// ---------------------------------------------------------------------------
// Forwarding Rule spec
// ---------------------------------------------------------------------------

/// Conditional forwarding rule (like CoreDNS's `forward` plugin).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ForwardingRuleSpec {
    /// Domain zone to match (e.g. "cluster.local").
    pub zone: String,
    /// Upstream servers to forward to.
    pub upstreams: Vec<String>,
    /// Optional policy: "sequential", "random", "round_robin".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy: Option<String>,
    /// Health check interval (seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub health_check: Option<u64>,
}

// ---------------------------------------------------------------------------
// TLS Config spec
// ---------------------------------------------------------------------------

/// TLS configuration for DoT/DoH.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TlsConfigSpec {
    /// Enable DNS-over-TLS on port 853.
    #[serde(default)]
    pub dot_enabled: bool,
    /// Enable DNS-over-HTTPS.
    #[serde(default)]
    pub doh_enabled: bool,
    /// DoH bind address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doh_bind_address: Option<String>,
    /// DoH URL path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doh_path: Option<String>,
    /// TLS certificate path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cert_path: Option<String>,
    /// TLS key path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,
}

// ---------------------------------------------------------------------------
// Rate Limit spec
// ---------------------------------------------------------------------------

/// Rate limiting configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RateLimitSpec {
    /// Queries per second per client IP.
    #[serde(default = "default_100")]
    pub qps: u32,
    /// Burst size.
    #[serde(default = "default_200")]
    pub burst: u32,
    /// Block duration after exceeding limit (seconds).
    #[serde(default = "default_60")]
    pub block_duration: u32,
}

// ---------------------------------------------------------------------------
// DHCP Server spec
// ---------------------------------------------------------------------------

/// DHCP server configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DhcpServerSpec {
    /// Network interface to bind to.
    pub interface: String,
    /// Pool name references.
    pub pools: Vec<String>,
    /// Default lease time (seconds).
    #[serde(default = "default_86400")]
    pub default_lease_time: u32,
    /// Max lease time (seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_lease_time: Option<u32>,
    /// DNS servers to hand out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_servers: Option<Vec<String>>,
    /// Router/gateway to hand out.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub router: Option<String>,
    /// NTP servers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ntp_servers: Option<Vec<String>>,
    /// Domain name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_name: Option<String>,
    /// PXE bootfile.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootfile: Option<String>,
    /// PXE TFTP server.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tftp_server: Option<String>,
    /// Static reservations.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reservations: Option<Vec<DhcpReservation>>,
}

/// DHCP static reservation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DhcpReservation {
    /// MAC address.
    pub mac: String,
    /// Reserved IP address.
    pub ip: String,
    /// Optional hostname.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
}

// ---------------------------------------------------------------------------
// DHCPv6 Server spec
// ---------------------------------------------------------------------------

/// DHCPv6 server configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Dhcpv6ServerSpec {
    /// Network interface to bind to.
    pub interface: String,
    /// Pool name references.
    pub pools: Vec<String>,
    /// Default preferred lifetime (seconds).
    #[serde(default = "default_3600")]
    pub default_preferred_lifetime: u32,
    /// Default valid lifetime (seconds).
    #[serde(default = "default_7200")]
    pub default_valid_lifetime: u32,
    /// DNS servers to hand out (IPv6 addresses).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_servers: Option<Vec<String>>,
    /// Domain name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain_name: Option<String>,
    /// Static reservations (IPv6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reservations: Option<Vec<Dhcpv6Reservation>>,
}

fn default_3600() -> u32 {
    3600
}
fn default_7200() -> u32 {
    7200
}

/// DHCPv6 static reservation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Dhcpv6Reservation {
    /// DUID (DHCP Unique Identifier) or MAC address.
    pub duid: String,
    /// Reserved IPv6 address.
    pub ip: String,
    /// Optional hostname.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
}

// ---------------------------------------------------------------------------
// DHCPv6 Pool spec
// ---------------------------------------------------------------------------

/// DHCPv6 pool configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// DHCP Pool spec
// ---------------------------------------------------------------------------

/// DHCP pool configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exclude: Option<Vec<String>>,
}

// ---------------------------------------------------------------------------
// TFTP Server spec
// ---------------------------------------------------------------------------

/// TFTP server configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TftpServerSpec {
    /// Bind address (default: "0.0.0.0:69").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_address: Option<String>,
    /// Root directory for served files.
    pub root_dir: String,
    /// Block size (default: 512, max: 65464).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_size: Option<u16>,
    /// Timeout (seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u32>,
    /// Allow writes.
    #[serde(default)]
    pub allow_writes: bool,
}

// ---------------------------------------------------------------------------
// SMTP Server spec
// ---------------------------------------------------------------------------

/// SMTP server configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SmtpServerSpec {
    /// Server hostname (for EHLO/HELO).
    pub hostname: String,
    /// TCP bind address (default: "0.0.0.0:25").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_address: Option<String>,
    /// Enable SMTPS on port 465.
    #[serde(default)]
    pub smtps: bool,
    /// Enable STARTTLS on port 587.
    #[serde(default)]
    pub starttls: bool,
    /// Maximum message size in bytes (default: 35MB).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_message_size: Option<usize>,
    /// Local domains for mail delivery.
    #[serde(default)]
    pub local_domains: Vec<String>,
    /// Maildir root for storing messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maildir_root: Option<String>,
    /// Enable outbound relay/queue.
    #[serde(default)]
    pub relay_enabled: bool,
    /// Queue data directory for outbound mail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queue_dir: Option<String>,
    /// DNS server for MX lookups (default: "8.8.8.8:53").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dns_server: Option<String>,
    /// DKIM signing domain.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dkim_domain: Option<String>,
    /// DKIM selector.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dkim_selector: Option<String>,
    /// Path to DKIM private key file.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dkim_key_path: Option<String>,
    /// TLS certificate (PEM format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_cert: Option<String>,
    /// TLS private key (PEM format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_key: Option<String>,
}

// ---------------------------------------------------------------------------
// IMAP Server spec
// ---------------------------------------------------------------------------

/// IMAP server configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ImapServerSpec {
    /// Server hostname (for CAPABILITY).
    pub hostname: String,
    /// TCP bind address (default: "0.0.0.0:143").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bind_address: Option<String>,
    /// Enable IMAPS on port 993.
    #[serde(default)]
    pub imaps: bool,
    /// Maildir root for storing messages (should match SMTP).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maildir_root: Option<String>,
    /// TLS certificate (PEM format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_cert: Option<String>,
    /// TLS private key (PEM format).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls_key: Option<String>,
}

// ---------------------------------------------------------------------------
// Node spec (K8s Node-compatible)
// ---------------------------------------------------------------------------

/// Node configuration - defines controllers who can control this node.
/// Node ID is the public key (derived from genesis), not set in config.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeSpec {
    /// Controller identity IDs (node IDs = public keys).
    /// These identities can send commands to this node.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controllers: Vec<String>,
    /// Node roles (e.g. "control-plane", "worker").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    /// Public IPs for this node (for reachability).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub public_ips: Vec<String>,
    /// Taints applied to pods that can't schedule on this node.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub taints: Vec<NodeTaint>,
    /// Unschedulable marks node as unschedulable.
    #[serde(default)]
    pub unschedulable: bool,
}

/// Node taint - marks pods that can't schedule on this node.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeTaint {
    /// Taint key.
    pub key: String,
    /// Taint value.
    pub value: Option<String>,
    /// Taint effect: "NoSchedule", "PreferNoSchedule", "NoExecute".
    pub effect: String,
    /// Time when taint expires.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_added: Option<String>,
}

// ---------------------------------------------------------------------------
// Peer spec (mesh peer)
// ---------------------------------------------------------------------------

/// Peer configuration - defines a peer node in the mesh.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerSpec {
    /// Node ID (public key) of the peer.
    pub node_id: String,
    /// Reachability hints - how to contact this peer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub endpoints: Vec<PeerEndpoint>,
    /// Last seen timestamp (RFC3339).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen: Option<String>,
    /// Peer is currently reachable.
    #[serde(default)]
    pub reachable: bool,
    /// Roles advertised by peer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
}

/// Peer endpoint - transport-specific reachability.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeerEndpoint {
    /// Transport: "tcp", "quic", "ble", "http3".
    pub transport: String,
    /// Address (host:port) or URL.
    pub address: String,
    /// Is encrypted.
    #[serde(default)]
    pub encrypted: bool,
    /// Cost metric (lower is better).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cost: Option<i32>,
}

// ---------------------------------------------------------------------------
// Gateway (proxy server)
// ---------------------------------------------------------------------------

/// Gateway specification - configures the proxy server.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GatewaySpec {
    /// Listeners - ports and protocols to listen on.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub listeners: Vec<GatewayListener>,
    /// Default TLS config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls: Option<GatewayTlsConfig>,
    /// Route selector - which routes this gateway handles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub route_selector: Option<std::collections::HashMap<String, String>>,
}

/// Gateway listener - a port/protocol combination.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GatewayListener {
    /// Listener name.
    pub name: String,
    /// Port to listen on.
    pub port: i32,
    /// Protocol: HTTP, HTTPS, TCP, TLS.
    pub protocol: String,
    /// TLS config (for HTTPS/TLS listeners).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls: Option<GatewayTlsConfig>,
}

/// Gateway TLS configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GatewayTlsConfig {
    /// Secret reference for TLS cert.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_ref: Option<String>,
    /// Min TLS version.
    #[serde(default)]
    pub min_version: String,
    /// Cipher suites.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ciphers: Vec<String>,
}

// ---------------------------------------------------------------------------
// Service (backend discovery)
// ---------------------------------------------------------------------------

/// Service specification - selector-based container discovery.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServiceSpec {
    /// Selector - matches containers with these labels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selector: Option<std::collections::HashMap<String, String>>,
    /// Ports to expose.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<ServicePort>,
    /// Session affinity.
    #[serde(default)]
    pub affinity: ServiceAffinity,
}

/// Service port definition.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServicePort {
    /// Port number.
    pub port: i32,
    /// Target port on container.
    pub target_port: i32,
    /// Protocol: TCP, UDP.
    #[serde(default)]
    pub protocol: String,
    /// Port name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HttpRouteSpec {
    /// Parent gateway reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_ref: Option<String>,
    /// Hostnames to match.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub hostnames: Vec<String>,
    /// Routing rules.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<HttpRouteRule>,
}

/// HTTP route rule.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HttpRouteRule {
    /// Path matches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub matches: Vec<HttpRouteMatch>,
    /// Backend reference.
    pub backend: HttpRouteBackend,
}

/// HTTP route match.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HttpRouteMatch {
    /// Path value to match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Path type: "Exact", "Prefix", "Regex".
    #[serde(default)]
    pub path_type: String,
    /// Header matches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<HttpRouteHeaderMatch>,
}

/// HTTP route header match.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HttpRouteHeaderMatch {
    pub name: String,
    pub value: String,
    pub type_: String,
}

/// HTTP route backend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TcpRouteSpec {
    /// Parent gateway reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_ref: Option<String>,
    /// Port to match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<i32>,
    /// Backend reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<TcpRouteBackend>,
}

/// TCP route backend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TcpRouteBackend {
    pub service: String,
    pub port: i32,
}

// ---------------------------------------------------------------------------
// TlsRoute (TLS passthrough)
// ---------------------------------------------------------------------------

/// TlsRoute specification - TLS passthrough routing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TlsRouteSpec {
    /// Parent gateway reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_ref: Option<String>,
    /// SNI hostnames to match.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sni_hostnames: Vec<String>,
    /// Backend reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend: Option<TlsRouteBackend>,
}

/// TLS route backend.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TlsRouteBackend {
    pub service: String,
    pub port: i32,
}

// ---------------------------------------------------------------------------
// Container spec (K8s Pod-compatible OCI container)
// ---------------------------------------------------------------------------

/// Container workload specification.
/// Deployment spec - multiple containers with networking.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DeploymentSpec {
    /// Deployment name.
    pub name: String,
    /// Namespace.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub namespace: String,
    /// Container specs to run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub containers: Vec<Container>,
    /// Replicas per container.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replicas: Option<i32>,
    /// Service networking between containers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<DeploymentService>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct DeploymentService {
    /// Ports exposed by container (container_name -> ports).
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub ports: std::collections::HashMap<String, Vec<u16>>,
}

/// Maps to K8s Pod spec with OCI container.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContainerSpec {
    /// Container image (OCI image reference).
    pub image: String,
    /// Image pull policy: "Always", "IfNotPresent", "Never".
    #[serde(default = "default_pull_always")]
    pub image_pull_policy: String,
    /// Container restart policy.
    #[serde(default)]
    pub restart_policy: ContainerRestartPolicy,
    /// Active deadline (seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_deadline_seconds: Option<u64>,
    /// Service account name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account_name: Option<String>,
    /// Number of desired pods (replicas).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replicas: Option<i32>,
    /// Selector for pods (label query).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<std::collections::HashMap<String, String>>,
    /// Pod template spec.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template: Option<PodTemplateSpec>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PodTemplateSpec {
    /// Standard object's metadata.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResourceMetadata>,
    /// Pod specification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec: Option<PodSpec>,
}

/// Pod specification (subset of K8s PodSpec).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PodSpec {
    /// Containers in this pod.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub containers: Vec<Container>,
    /// Init containers.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub init_containers: Vec<Container>,
    /// Restart policy.
    #[serde(default)]
    pub restart_policy: ContainerRestartPolicy,
    /// Termination grace period (seconds).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination_grace_period_seconds: Option<i64>,
    /// DNS policy.
    #[serde(default = "default_dns_policy")]
    pub dns_policy: String,
    /// Node selector.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_selector: Option<std::collections::HashMap<String, String>>,
    /// Node name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_name: Option<String>,
    /// Host network.
    #[serde(default)]
    pub host_network: bool,
    /// Host PID.
    #[serde(default)]
    pub host_pid: bool,
    /// Host IPC.
    #[serde(default)]
    pub host_ipc: bool,
    /// Share process namespace.
    #[serde(default)]
    pub share_process_namespace: bool,
    /// Security context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_context: Option<PodSecurityContext>,
    /// Image pull secrets.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub image_pull_secrets: Vec<LocalObjectReference>,
    /// Volumes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volumes: Vec<Volume>,
    /// Affinity.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affinity: Option<Affinity>,
    /// Tolerations.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tolerations: Vec<Toleration>,
}

/// Container specification (from K8s Container).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Container {
    /// Container name.
    pub name: String,
    /// Container image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    /// Image pull policy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_pull_policy: Option<String>,
    /// Command (entrypoint).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command: Vec<String>,
    /// Args (command arguments).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Environment variables.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env: Vec<EnvVar>,
    /// Environment from sources.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub env_from: Vec<EnvFromSource>,
    /// Volume mounts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub volume_mounts: Vec<VolumeMount>,
    /// Ports.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ports: Vec<ContainerPort>,
    /// Resources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceRequirements>,
    /// Security context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_context: Option<ContainerSecurityContext>,
    /// Liveness probe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub liveness_probe: Option<Probe>,
    /// Readiness probe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readiness_probe: Option<Probe>,
    /// Startup probe.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub startup_probe: Option<Probe>,
    /// Lifecycle hook.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lifecycle: Option<Lifecycle>,
    /// Working directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_dir: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub enum ContainerRestartPolicy {
    Always,
    #[default]
    OnFailure,
    Never,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvVar {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_from: Option<EnvVarSource>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvVarSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field_ref: Option<ObjectFieldSelector>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_ref: Option<SecretEnvSource>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ObjectFieldSelector {
    pub field_path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecretEnvSource {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvFromSource {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_map_ref: Option<ConfigMapEnvSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret_ref: Option<SecretEnvSource>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigMapEnvSource {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VolumeMount {
    pub name: String,
    pub mount_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_only: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_path: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContainerPort {
    pub name: Option<String>,
    pub container_port: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_port: Option<i32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourceRequirements {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub limits: Option<std::collections::HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requests: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct ResourceLimits {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_cores: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub storage_bytes: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub network_mbps: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ContainerSecurityContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privileged: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_as_user: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_as_non_root: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Capabilities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seccomp_profile: Option<SeccompProfile>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Capabilities {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub add: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drop: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SeccompProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub localhost_profile: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PodSecurityContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_as_user: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_as_non_root: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_as_group: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fs_group: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supplemental_groups: Option<Vec<i64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seccomp_profile: Option<SeccompProfile>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sysctls: Option<Vec<Sysctl>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Sysctl {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LocalObjectReference {
    pub name: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Volume {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub empty_dir: Option<EmptyDirVolumeSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config_map: Option<ConfigMapVolumeSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secret: Option<SecretVolumeSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub persistent_volume_claim: Option<PersistentVolumeClaimVolumeSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host_path: Option<HostPathVolumeSource>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EmptyDirVolumeSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub medium: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_limit: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConfigMapVolumeSource {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<KeyToPath>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecretVolumeSource {
    pub secret_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<KeyToPath>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PersistentVolumeClaimVolumeSource {
    pub claim_name: String,
    #[serde(default)]
    pub read_only: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HostPathVolumeSource {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyToPath {
    pub key: String,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<i32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Affinity {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_affinity: Option<NodeAffinity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pod_affinity: Option<PodAffinity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pod_anti_affinity: Option<PodAntiAffinity>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeAffinity {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preferred_during_scheduling: Option<Vec<PreferredSchedulingTerm>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_during_scheduling: Option<NodeSelector>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PreferredSchedulingTerm {
    pub weight: i32,
    pub preference: NodeSelectorTerm,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeSelector {
    pub node_selector_terms: Vec<NodeSelectorTerm>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeSelectorTerm {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub match_expressions: Vec<NodeSelectorRequirement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub match_fields: Vec<NodeSelectorRequirement>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NodeSelectorRequirement {
    pub key: String,
    pub operator: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PodAffinity {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_during_scheduling: Vec<PodAffinityTerm>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preferred_during_scheduling: Vec<WeightedPodAffinityTerm>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PodAntiAffinity {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_during_scheduling: Vec<PodAffinityTerm>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preferred_during_scheduling: Vec<WeightedPodAffinityTerm>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PodAffinityTerm {
    pub label_selector: Option<ResourceMetadata>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub namespaces: Vec<String>,
    pub topology_key: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WeightedPodAffinityTerm {
    pub weight: i32,
    pub pod_affinity_term: PodAffinityTerm,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Toleration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub operator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toleration_seconds: Option<i64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Probe {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exec: Option<ExecAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_get: Option<HTTPGetAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tcp_socket: Option<TCPSocketAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grpc: Option<GRPCAction>,
    pub initial_delay_seconds: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub period_seconds: Option<i32>,
    pub success_threshold: Option<i32>,
    pub failure_threshold: Option<i32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecAction {
    pub command: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HTTPGetAction {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub port: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub http_headers: Vec<HTTPHeader>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HTTPHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TCPSocketAction {
    pub port: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GRPCAction {
    pub port: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Lifecycle {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_start: Option<LifecycleHandler>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_stop: Option<LifecycleHandler>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LifecycleHandler {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exec: Option<ExecAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http_get: Option<HTTPGetAction>,
}

// ---------------------------------------------------------------------------
// Secret spec (K8s Secret-compatible)
// ---------------------------------------------------------------------------

/// Secret specification - stores encrypted data reference in blob store.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SecretSpec {
    /// Secret data (base64-encoded when serialized).
    /// Stored encrypted in blob store, not in config.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<std::collections::HashMap<String, Vec<u8>>>,
    /// Secret string data (plaintext when serialized).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub string_data: Option<std::collections::HashMap<String, String>>,
    /// Secret type: "Opaque", "kubernetes.io/tls", "kubernetes.io/dockerconfigjson", etc.
    #[serde(default)]
    pub secret_type: SecretType,
    /// Reference to blob store for the actual encrypted data.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blob_ref: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub enum SecretType {
    #[default]
    Opaque,
    ServiceAccountToken,
    DockerConfigJson,
    TLS,
    BootstrapToken,
}

// ---------------------------------------------------------------------------
// Default helpers
// ---------------------------------------------------------------------------

fn default_serial() -> u32 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
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
