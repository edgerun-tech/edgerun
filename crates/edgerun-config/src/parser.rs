//! Config file parser — reads YAML and produces typed config resources.

use crate::types::ConfigResource;
use serde::Deserialize;

/// A raw K8s-style YAML document (before being mapped to ConfigResource).
#[derive(Debug, Clone, serde::Deserialize)]
struct RawDoc {
    kind: String,
    spec: serde_yaml::Value,
}

/// Parse a single YAML file into a list of config resources.
/// Supports multi-document YAML (separated by `---`).
pub fn parse_config_file(yaml: &str) -> Result<Vec<ConfigResource>, ConfigError> {
    let raw_docs: Vec<RawDoc> = serde_yaml::Deserializer::from_str(yaml)
        .map(|de| serde_yaml::Value::deserialize(de))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| ConfigError::ParseError(e.to_string()))?
        .into_iter()
        .filter(|d| !matches!(d, serde_yaml::Value::Null))
        .filter_map(|d| serde_yaml::from_value(d).ok())
        .collect();

    let mut resources = Vec::new();
    for raw in raw_docs {
        let spec = raw.spec.clone();
        let res = match raw.kind.as_str() {
            "DnsServer" => serde_yaml::from_value(spec).map(ConfigResource::DnsServer),
            "DnsZone" => serde_yaml::from_value(spec).map(ConfigResource::DnsZone),
            "DnsForwarder" => serde_yaml::from_value(spec).map(ConfigResource::DnsForwarder),
            "ForwardingRule" => serde_yaml::from_value(spec).map(ConfigResource::ForwardingRule),
            "TlsConfig" => serde_yaml::from_value(spec).map(ConfigResource::TlsConfig),
            "RateLimit" => serde_yaml::from_value(spec).map(ConfigResource::RateLimit),
            "DhcpServer" => serde_yaml::from_value(spec).map(ConfigResource::DhcpServer),
            "DhcpPool" => serde_yaml::from_value(spec).map(ConfigResource::DhcpPool),
            "TftpServer" => serde_yaml::from_value(spec).map(ConfigResource::TftpServer),
            _ => continue,
        };
        if let Ok(r) = res { resources.push(r); }
    }

    if resources.is_empty() {
        return Err(ConfigError::EmptyFile);
    }

    Ok(resources)
}

/// Serialize resources as multi-document K8s-style YAML.
pub fn to_yaml_all(resources: &[ConfigResource]) -> Result<String, serde_yaml::Error> {
    let mut out = String::new();
    for (i, res) in resources.iter().enumerate() {
        if i > 0 { out.push_str("---\n"); }
        let spec_val = serde_yaml::to_value(res)?;
        // Build K8s envelope manually
        let mut doc = serde_yaml::Mapping::new();
        doc.insert(serde_yaml::Value::String("apiVersion".to_string()),
            serde_yaml::Value::String("edgerun.io/v1alpha1".to_string()));
        doc.insert(serde_yaml::Value::String("kind".to_string()),
            serde_yaml::Value::String(res.kind().to_string()));
        let mut meta = serde_yaml::Mapping::new();
        meta.insert(serde_yaml::Value::String("name".to_string()),
            serde_yaml::Value::String(res.name().to_string()));
        doc.insert(serde_yaml::Value::String("metadata".to_string()),
            serde_yaml::Value::Mapping(meta));
        doc.insert(serde_yaml::Value::String("spec".to_string()), spec_val);
        out.push_str(&serde_yaml::to_string(&serde_yaml::Value::Mapping(doc))?);
    }
    Ok(out)
}

/// Parse and validate a config file, returning both raw resources and
/// a validated `ConfigState` projection.
pub fn parse_and_validate(yaml: &str) -> Result<ConfigState, ConfigError> {
    let resources = parse_config_file(yaml)?;
    let state = ConfigState::from_resources(&resources)?;
    Ok(state)
}

/// Projected configuration state built from a set of resources.
#[derive(Debug, Clone, Default)]
pub struct ConfigState {
    /// DNS server configs.
    pub dns_servers: Vec<crate::types::DnsServerSpec>,
    /// DNS zone configs.
    pub dns_zones: Vec<crate::types::DnsZoneSpec>,
    /// DNS forwarder configs.
    pub dns_forwarders: Vec<crate::types::DnsForwarderSpec>,
    /// Forwarding rules.
    pub forwarding_rules: Vec<crate::types::ForwardingRuleSpec>,
    /// TLS configs.
    pub tls_configs: Vec<crate::types::TlsConfigSpec>,
    /// Rate limit configs.
    pub rate_limits: Vec<crate::types::RateLimitSpec>,
    /// DHCP server configs.
    pub dhcp_servers: Vec<crate::types::DhcpServerSpec>,
    /// DHCP pool configs.
    pub dhcp_pools: Vec<crate::types::DhcpPoolSpec>,
    /// DHCPv6 server configs.
    pub dhcpv6_servers: Vec<crate::types::Dhcpv6ServerSpec>,
    /// DHCPv6 pool configs.
    pub dhcpv6_pools: Vec<crate::types::Dhcpv6PoolSpec>,
    /// TFTP server configs.
    pub tftp_servers: Vec<crate::types::TftpServerSpec>,
}

impl ConfigState {
    /// Build a ConfigState from a list of resources.
    pub fn from_resources(resources: &[ConfigResource]) -> Result<Self, ConfigError> {
        let mut state = ConfigState::default();

        for res in resources {
            match res {
                ConfigResource::DnsServer(spec) => state.dns_servers.push(spec.clone()),
                ConfigResource::DnsZone(spec) => state.dns_zones.push(spec.clone()),
                ConfigResource::DnsForwarder(spec) => state.dns_forwarders.push(spec.clone()),
                ConfigResource::ForwardingRule(spec) => state.forwarding_rules.push(spec.clone()),
                ConfigResource::TlsConfig(spec) => state.tls_configs.push(spec.clone()),
                ConfigResource::RateLimit(spec) => state.rate_limits.push(spec.clone()),
                ConfigResource::DhcpServer(spec) => state.dhcp_servers.push(spec.clone()),
                ConfigResource::DhcpPool(spec) => state.dhcp_pools.push(spec.clone()),
                ConfigResource::Dhcpv6Server(spec) => state.dhcpv6_servers.push(spec.clone()),
                ConfigResource::Dhcpv6Pool(spec) => state.dhcpv6_pools.push(spec.clone()),
                ConfigResource::TftpServer(spec) => state.tftp_servers.push(spec.clone()),
            }
        }

        // Validate cross-resource references
        state.validate()?;

        Ok(state)
    }

    /// Validate cross-resource consistency.
    pub fn validate(&self) -> Result<(), ConfigError> {
        // Check that all zone references in DnsServer specs exist
        for (i, dns) in self.dns_servers.iter().enumerate() {
            if let Some(zones) = &dns.zones {
                for zone_name in zones {
                    if !self.dns_zones.iter().any(|z| z.origin == *zone_name) {
                        return Err(ConfigError::ValidationError(format!(
                            "dns_servers[{}]: references zone '{}' which does not exist",
                            i, zone_name
                        )));
                    }
                }
            }
        }

        // Check that all pool references in DhcpServer specs exist
        for (i, dhcp) in self.dhcp_servers.iter().enumerate() {
            for pool_name in &dhcp.pools {
                if !self.dhcp_pools.iter().any(|p| p.name == *pool_name) {
                    return Err(ConfigError::ValidationError(format!(
                        "dhcp_servers[{}]: references pool '{}' which does not exist",
                        i, pool_name
                    )));
                }
            }
        }

        // Validate zone records have required types (NS at minimum; SOA is in soa field)
        for (i, zone) in self.dns_zones.iter().enumerate() {
            if !zone.records.iter().any(|r| r.record_type == "NS") {
                return Err(ConfigError::ValidationError(format!(
                    "dns_zones[{}]: zone '{}' has no NS record",
                    i, zone.origin
                )));
            }
        }

        // Validate DHCP pool ranges
        for (i, pool) in self.dhcp_pools.iter().enumerate() {
            let start = parse_ipv4(&pool.range_start)?;
            let end = parse_ipv4(&pool.range_end)?;
            if start > end {
                return Err(ConfigError::ValidationError(format!(
                    "dhcp_pools[{}]: range_start {} > range_end {}",
                    i, pool.range_start, pool.range_end
                )));
            }
        }

        Ok(())
    }

    /// Build DhcpScope instances for a given DHCP server config.
    /// Returns scopes indexed by pool name, ready for DhcpMultiServer.
    pub fn build_dhcp_scopes(
        &self,
        server_index: usize,
    ) -> Result<std::collections::HashMap<String, edgerun_dns::dhcp::DhcpScope>, ConfigError> {
        let server = self.dhcp_servers.get(server_index).ok_or_else(|| {
            ConfigError::ValidationError(format!("dhcp_servers[{}]: out of range", server_index))
        })?;

        let dns_servers: Vec<std::net::Ipv4Addr> = server.dns_servers.iter()
            .flatten()
            .filter_map(|s| s.parse().ok())
            .collect();

        let router: std::net::Ipv4Addr = server.router.as_deref()
            .unwrap_or("0.0.0.0")
            .parse()
            .map_err(|e| ConfigError::ValidationError(format!("invalid router: {}", e)))?;

        let mut scopes = std::collections::HashMap::new();

        for pool_name in &server.pools {
            let pool = self.dhcp_pools.iter()
                .find(|p| &p.name == pool_name)
                .ok_or_else(|| ConfigError::ValidationError(
                    format!("pool '{}' not found", pool_name)
                ))?;

            let range_start: std::net::Ipv4Addr = pool.range_start.parse()
                .map_err(|e| ConfigError::ValidationError(format!("invalid range_start: {}", e)))?;
            let range_end: std::net::Ipv4Addr = pool.range_end.parse()
                .map_err(|e| ConfigError::ValidationError(format!("invalid range_end: {}", e)))?;
            let subnet_mask: std::net::Ipv4Addr = pool.subnet_mask.parse()
                .map_err(|e| ConfigError::ValidationError(format!("invalid subnet_mask: {}", e)))?;

            let mut scope = edgerun_dns::dhcp::DhcpScope::new(
                &pool.name,
                range_start,
                range_end,
                subnet_mask,
                router,
                dns_servers.clone(),
                server.default_lease_time,
            );

            // Apply PXE config
            if let (Some(tftp), Some(bootfile)) = (&server.tftp_server, &server.bootfile) {
                if let Ok(tftp_ip) = tftp.parse::<std::net::Ipv4Addr>() {
                    scope = scope.with_pxe(tftp_ip, bootfile.clone());
                }
            }

            // Apply static reservations
            if let Some(ref reservations) = server.reservations {
                for res in reservations {
                    if let (Ok(mac), Ok(ip)) = (parse_mac(&res.mac), res.ip.parse::<std::net::Ipv4Addr>()) {
                        scope.pool.reserve(ip);
                        // Reserve in the pool's lease map too
                        scope.pool.leases.insert(
                            edgerun_dns::dhcp::lease::ip_to_u32(ip),
                            edgerun_dns::dhcp::lease::Lease::with_client_id(
                                mac, res.mac.as_bytes().to_vec(), ip,
                                server.default_lease_time, 0,
                            ),
                        );
                    }
                }
            }

            scopes.insert(pool.name.clone(), scope);
        }

        Ok(scopes)
    }

    /// Build DHCPv6 scope instances for a given DHCPv6 server config.
    /// Returns scopes indexed by pool name.
    pub fn build_dhcpv6_scopes(
        &self,
        server_index: usize,
    ) -> Result<std::collections::HashMap<String, edgerun_dhcpv6::lease::LeasePool>, ConfigError> {
        let server = self.dhcpv6_servers.get(server_index).ok_or_else(|| {
            ConfigError::ValidationError(format!("dhcpv6_servers[{}]: out of range", server_index))
        })?;

        let dns_servers: Vec<std::net::Ipv6Addr> = server.dns_servers.iter()
            .flatten()
            .filter_map(|s| s.parse().ok())
            .collect();

        let mut scopes = std::collections::HashMap::new();

        for pool_name in &server.pools {
            let pool = self.dhcpv6_pools.iter()
                .find(|p| &p.name == pool_name)
                .ok_or_else(|| ConfigError::ValidationError(
                    format!("DHCPv6 pool '{}' not found", pool_name)
                ))?;

            let range_start: std::net::Ipv6Addr = pool.range_start.parse()
                .map_err(|e| ConfigError::ValidationError(format!("invalid DHCPv6 range_start: {}", e)))?;
            let range_end: std::net::Ipv6Addr = pool.range_end.parse()
                .map_err(|e| ConfigError::ValidationError(format!("invalid DHCPv6 range_end: {}", e)))?;

            // Create a DHCPv6 lease pool
            let mut lease_pool = edgerun_dhcpv6::lease::LeasePool::new();

            // For DHCPv6, we typically delegate prefixes rather than individual addresses
            // Add the range as available prefixes with the specified prefix length
            lease_pool.add_prefixes([(range_start, pool.prefix_length)]);

            // Note: DNS servers are configured in Dhcpv6ServerConfig, not in LeasePool
            // The pool mainly manages address/prefix allocation

            // Apply static reservations by pre-allocating addresses
            if let Some(ref reservations) = server.reservations {
                for res in reservations {
                    if let Ok(ip) = res.ip.parse::<std::net::Ipv6Addr>() {
                        // Parse DUID or MAC
                        if let Ok(_duid_bytes) = parse_duid_or_mac(&res.duid) {
                            // Reservation parsed successfully - actual reservation would require
                            // modifying the pool's allocation logic
                        }
                    }
                }
            }

            scopes.insert(pool.name.clone(), lease_pool);
        }

        Ok(scopes)
    }
}

fn parse_ipv4(s: &str) -> Result<std::net::Ipv4Addr, ConfigError> {
    s.parse()
        .map_err(|e| ConfigError::ValidationError(format!("invalid IPv4 '{}': {}", s, e)))
}

fn parse_ipv6(s: &str) -> Result<std::net::Ipv6Addr, ConfigError> {
    s.parse()
        .map_err(|e| ConfigError::ValidationError(format!("invalid IPv6 '{}': {}", s, e)))
}

fn parse_mac(s: &str) -> Result<[u8; 6], ConfigError> {
    edgerun_encoding::hex::parse_mac(s)
        .ok_or_else(|| ConfigError::ValidationError(format!("invalid MAC '{}' (need 6 bytes)", s)))
}

/// Parse a DUID (DHCP Unique Identifier) or MAC address into bytes.
/// Returns variable-length bytes suitable for DHCPv6 client identification.
fn parse_duid_or_mac(s: &str) -> Result<Vec<u8>, ConfigError> {
    // Try parsing as colon-separated hex bytes (could be DUID or MAC)
    let result: Result<Vec<u8>, _> = s.split(':')
        .map(|p| u8::from_str_radix(p, 16))
        .collect();
    
    match result {
        Ok(bytes) if !bytes.is_empty() => Ok(bytes),
        _ => Err(ConfigError::ValidationError(
            format!("invalid DUID/MAC '{}' (expected colon-separated hex)", s)
        )),
    }
}

/// Config parsing/validation errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigError {
    #[error("config file is empty")]
    EmptyFile,
    #[error("parse error: {0}")]
    ParseError(String),
    #[error("validation error: {0}")]
    ValidationError(String),
    #[error("IO error: {0}")]
    IoError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dns_server() {
        let yaml = r#"
apiVersion: edgerun.io/v1alpha1
kind: DnsServer
metadata:
  name: primary
  namespace: edgerun-dns
spec:
  bind_address: "0.0.0.0:53"
  default_ttl: 3600
  recursive: false
"#;
        let state = parse_and_validate(yaml).unwrap();
        assert_eq!(state.dns_servers.len(), 1);
        let server = &state.dns_servers[0];
        assert_eq!(server.bind_address, Some("0.0.0.0:53".to_string()));
        assert_eq!(server.default_ttl, Some(3600));
        assert_eq!(server.recursive, Some(false));
    }

    #[test]
    fn test_parse_multi_document() {
        let yaml = r#"
apiVersion: edgerun.io/v1alpha1
kind: DnsServer
metadata:
  name: primary
spec:
  bind_address: "0.0.0.0:53"
---
apiVersion: edgerun.io/v1alpha1
kind: DnsForwarder
metadata:
  name: upstream
spec:
  upstreams:
    - "8.8.8.8:53"
    - "1.1.1.1:53"
  cache: true
  cache_max_entries: 10000
"#;
        let state = parse_and_validate(yaml).unwrap();
        assert_eq!(state.dns_servers.len(), 1);
        assert_eq!(state.dns_forwarders.len(), 1);
    }

    #[test]
    fn test_parse_dhcp_server() {
        let yaml = r#"
apiVersion: edgerun.io/v1alpha1
kind: DhcpServer
metadata:
  name: lan-dhcp
spec:
  interface: eth0
  pools:
    - lan-pool
  default_lease_time: 86400
  dns_servers:
    - "192.168.1.1"
  router: "192.168.1.1"
  domain_name: "home.local"
  reservations:
    - mac: "aa:bb:cc:dd:ee:ff"
      ip: "192.168.1.100"
      hostname: "printer"
"#;
        let state = parse_and_validate(yaml);
        // Validation will fail because pool doesn't exist, but parsing should work
        assert!(state.is_err());
        assert!(state.unwrap_err().to_string().contains("pool"));
    }

    #[test]
    fn test_parse_zone_with_records() {
        let yaml = r#"
apiVersion: edgerun.io/v1alpha1
kind: DnsZone
metadata:
  name: example-com
spec:
  origin: example.com
  soa:
    mname: ns1.example.com
    rname: admin.example.com
  records:
    - name: "@"
      type: NS
      value: "ns1.example.com"
    - name: "@"
      type: A
      value: "192.168.1.1"
    - name: www
      type: A
      value: "192.168.1.2"
    - name: "@"
      type: MX
      value:
        priority: 10
        exchange: "mail.example.com"
"#;
        let state = parse_and_validate(yaml).unwrap();
        assert_eq!(state.dns_zones.len(), 1);
        let zone = &state.dns_zones[0];
        assert_eq!(zone.origin, "example.com");
        assert_eq!(zone.records.len(), 4);
    }
}
