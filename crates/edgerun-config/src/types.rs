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
    pub fn to_yaml(&self) -> Result<String, serde_yaml::Error> {
        serde_yaml::to_string(self)
    }

    /// Parse from YAML string.
    pub fn from_yaml(yaml: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(yaml)
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
    pub value: serde_yaml::Value,
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

fn default_3600() -> u32 { 3600 }
fn default_7200() -> u32 { 7200 }

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
// Default helpers
// ---------------------------------------------------------------------------

fn default_serial() -> u32 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    ((secs / 86400) as u32) * 100 + 1 // YYYYMMDDNN format
}

fn default_900() -> u32 { 900 }
fn default_604800() -> u32 { 604800 }
fn default_86400() -> u32 { 86400 }
fn default_30d() -> u32 { 30 * 86400 }
fn default_ksk() -> u16 { 257 }
fn default_100() -> u32 { 100 }
fn default_200() -> u32 { 200 }
fn default_60() -> u32 { 60 }
