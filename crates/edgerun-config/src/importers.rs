//! Import converters — transform dnsmasq.conf and CoreDNS Corefile into
//! Kubernetes-compatible YAML config resources.

use crate::types::*;
use std::collections::HashMap;
use edgerun_json::JsonValue;

// ---------------------------------------------------------------------------
// dnsmasq.conf → ConfigResource
// ---------------------------------------------------------------------------

/// Parse a dnsmasq.conf file and produce equivalent K8s-style config resources.
pub fn import_dnsmasq(conf: &str) -> Result<Vec<ConfigResource>, ImportError> {
    let mut resources: Vec<ConfigResource> = Vec::new();
    let mut interface: Option<String> = None;
    let mut dhcp_ranges: Vec<DhcpRange> = Vec::new();
    let mut dhcp_hosts: Vec<DhcpHost> = Vec::new();
    let mut dhcp_options: HashMap<u8, String> = HashMap::new();
    let mut upstreams: Vec<String> = Vec::new();
    let mut domain_name: Option<String> = None;
    let mut router: Option<String> = None;
    let mut dns_servers: Vec<String> = Vec::new();
    let mut ntp_servers: Vec<String> = Vec::new();
    let mut tftp_root: Option<String> = None;
    let mut tftp_enabled = false;

    for line in conf.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let Some((key, value)) = line.split_once('=') else { continue; };

        match key {
            "interface" => interface = Some(value.to_string()),
            "dhcp-range" => dhcp_ranges.push(parse_dhcp_range(value)),
            "dhcp-host" => dhcp_hosts.push(parse_dhcp_host(value)),
            "dhcp-option" => {
                if let Some((opt, val)) = value.split_once(',') {
                    if let Ok(num) = opt.parse::<u8>() {
                        dhcp_options.insert(num, val.to_string());
                    }
                }
            }
            "dhcp-leaseTime" => {} // handled via spec default
            "server" => upstreams.push(value.to_string()),
            "domain" if domain_name.is_none() => { domain_name = Some(value.to_string()); }
            "tftp-root" => { tftp_root = Some(value.to_string()); tftp_enabled = true; }
            "enable-tftp" => tftp_enabled = true,
            _ => {}
        }
    }

    for (opt, val) in &dhcp_options {
        match opt {
            3 => router = Some(val.clone()),
            6 => dns_servers = val.split(',').map(|s| s.trim().to_string()).collect(),
            15 => domain_name = Some(val.clone()),
            42 => ntp_servers = val.split(',').map(|s| s.trim().to_string()).collect(),
            _ => {}
        }
    }

    if !dhcp_ranges.is_empty() || !dhcp_hosts.is_empty() {
        let pools: Vec<String> = dhcp_ranges.iter().enumerate()
            .map(|(i, _)| format!("pool-{}", i)).collect();
        let reservations: Vec<DhcpReservation> = dhcp_hosts.iter()
            .filter_map(|h| {
                if let (Some(mac), Some(ip)) = (&h.mac, &h.ip) {
                    Some(DhcpReservation { mac: mac.clone(), ip: ip.clone(), hostname: h.hostname.clone() })
                } else { None }
            }).collect();

        resources.push(ConfigResource::DhcpServer(DhcpServerSpec {
            interface: interface.clone().unwrap_or_else(|| "eth0".to_string()),
            pools,
            default_lease_time: 86400,
            max_lease_time: None,
            dns_servers: if dns_servers.is_empty() { None } else { Some(dns_servers) },
            router,
            ntp_servers: if ntp_servers.is_empty() { None } else { Some(ntp_servers) },
            domain_name,
            bootfile: None,
            tftp_server: None,
            reservations: if reservations.is_empty() { None } else { Some(reservations) },
        }));

        for (i, range) in dhcp_ranges.iter().enumerate() {
            resources.push(ConfigResource::DhcpPool(DhcpPoolSpec {
                name: format!("pool-{}", i),
                range_start: range.start.clone().unwrap_or_else(|| "192.168.1.100".to_string()),
                range_end: range.end.clone().unwrap_or_else(|| "192.168.1.200".to_string()),
                subnet_mask: range.mask.clone().unwrap_or_else(|| "255.255.255.0".to_string()),
                exclude: range.exclude.clone(),
            }));
        }
    }

    if !upstreams.is_empty() {
        resources.push(ConfigResource::DnsForwarder(DnsForwarderSpec {
            bind_address: None,
            upstreams,
            timeout: None,
            cache: true,
            cache_ttl: None,
            cache_max_entries: None,
        }));
    }

    if tftp_enabled {
        resources.push(ConfigResource::TftpServer(TftpServerSpec {
            bind_address: None,
            root_dir: tftp_root.unwrap_or_else(|| "/srv/tftp".to_string()),
            block_size: None,
            timeout: None,
            allow_writes: false,
        }));
    }

    if resources.is_empty() { Err(ImportError::EmptyConfig) } else { Ok(resources) }
}

// ---------------------------------------------------------------------------
// CoreDNS Corefile → ConfigResource
// ---------------------------------------------------------------------------

/// Parse a CoreDNS Corefile and produce equivalent K8s-style config resources.
pub fn import_corefile(corefile: &str) -> Result<Vec<ConfigResource>, ImportError> {
    let mut resources: Vec<ConfigResource> = Vec::new();
    let blocks = parse_corefile_blocks(corefile);

    for block in blocks {
        let zone = block.zone.trim_matches('"').trim_matches('/');
        let mut forward_upstreams: Vec<String> = Vec::new();
        let mut cache_enabled = false;
        let mut cache_ttl: Option<u32> = None;
        let mut host_entries: Vec<(String, String)> = Vec::new();
        let mut tls_cert: Option<String> = None;
        let mut tls_key: Option<String> = None;
        let mut bind_addr: Option<String> = None;

        for plugin in &block.plugins {
            match plugin.name.as_str() {
                "forward" => {
                    for arg in &plugin.args {
                        if arg.starts_with('/') || matches!(arg.as_str(),
                            "sequential"|"random"|"round_robin"|"force_tcp"|"prefer_udp"
                            |"expire"|"max_concurrent") { continue; }
                        forward_upstreams.push(arg.clone());
                    }
                }
                "cache" => {
                    cache_enabled = true;
                    if let Some(ttl) = plugin.args.first().and_then(|t| t.parse().ok()) {
                        cache_ttl = Some(ttl);
                    }
                }
                "hosts" => {
                    let mut i = 0;
                    while i + 1 < plugin.args.len() {
                        let ip = &plugin.args[i];
                        let name = &plugin.args[i + 1];
                        if ip.contains('.') && !name.starts_with('/') {
                            host_entries.push((name.clone(), ip.clone()));
                            i += 2;
                        } else { i += 1; }
                    }
                }
                "tls"
                    if plugin.args.len() >= 2 => {
                        tls_cert = Some(plugin.args[0].clone());
                        tls_key = Some(plugin.args[1].clone());
                    }
                "bind" => { if let Some(a) = plugin.args.first() { bind_addr = Some(a.clone()); } }
                _ => {}
            }
        }

        if !forward_upstreams.is_empty() {
            resources.push(ConfigResource::ForwardingRule(ForwardingRuleSpec {
                zone: zone.to_string(),
                upstreams: forward_upstreams.clone(),
                policy: None,
                health_check: None,
            }));
        }

        if !host_entries.is_empty() {
            let mut records: Vec<ZoneRecord> = host_entries.iter()
                .map(|(name, ip)| ZoneRecord {
                    name: name.clone(), record_type: "A".to_string(), ttl: None,
                    value: edgerun_json::JsonValue::String(ip.clone()),
                }).collect();
            records.push(ZoneRecord {
                name: "@".to_string(), record_type: "NS".to_string(), ttl: None,
                value: edgerun_json::JsonValue::String(format!("ns1.{}", zone)),
            });
            resources.push(ConfigResource::DnsZone(DnsZoneSpec {
                origin: zone.to_string(),
                soa: SoaRecord {
                    mname: format!("ns1.{}", zone), rname: format!("admin.{}", zone),
                    serial: 1, refresh: 3600, retry: 900, expire: 604800, minimum: 86400,
                },
                records,
                dnssec: None,
                wildcards: None,
            }));
        }

        if cache_enabled && !forward_upstreams.is_empty() {
            resources.push(ConfigResource::DnsForwarder(DnsForwarderSpec {
                bind_address: bind_addr,
                upstreams: forward_upstreams.clone(),
                timeout: None,
                cache: true,
                cache_ttl,
                cache_max_entries: None,
            }));
        }

        if tls_cert.is_some() || tls_key.is_some() {
            resources.push(ConfigResource::TlsConfig(TlsConfigSpec {
                dot_enabled: true,
                doh_enabled: false,
                doh_bind_address: None,
                doh_path: None,
                cert_path: tls_cert,
                key_path: tls_key,
            }));
        }
    }

    if resources.is_empty() { Err(ImportError::EmptyConfig) } else { Ok(resources) }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct DhcpRange { start: Option<String>, end: Option<String>, mask: Option<String>, exclude: Option<Vec<String>> }
#[derive(Debug)]
struct DhcpHost { mac: Option<String>, ip: Option<String>, hostname: Option<String> }

#[derive(Debug)]
struct CoreBlock { zone: String, plugins: Vec<CorePlugin> }
#[derive(Debug)]
struct CorePlugin { name: String, args: Vec<String> }

fn parse_dhcp_range(value: &str) -> DhcpRange {
    let mut r = DhcpRange { start: None, end: None, mask: None, exclude: None };
    for part in value.split(',') {
        if let Some((k, v)) = part.split_once('=') {
            match k {
                "start" => r.start = Some(v.to_string()),
                "end" => r.end = Some(v.to_string()),
                "mask"|"netmask" => r.mask = Some(v.to_string()),
                _ => {}
            }
        } else if part.contains('-') {
            let p: Vec<&str> = part.split('-').collect();
            if p.len() == 2 { r.start = Some(p[0].trim().to_string()); r.end = Some(p[1].trim().to_string()); }
        }
    }
    r
}

fn parse_dhcp_host(value: &str) -> DhcpHost {
    let mut h = DhcpHost { mac: None, ip: None, hostname: None };
    for part in value.split(',') {
        let part = part.trim();
        if part.contains(':') && part.len() == 17 { h.mac = Some(part.to_string()); }
        else if part.parse::<std::net::Ipv4Addr>().is_ok() { h.ip = Some(part.to_string()); }
        else if !part.is_empty() { h.hostname = Some(part.to_string()); }
    }
    h
}

fn parse_corefile_blocks(corefile: &str) -> Vec<CoreBlock> {
    let mut blocks = Vec::new();
    let mut current_zone: Option<String> = None;
    let mut current_plugins: Vec<CorePlugin> = Vec::new();
    let mut depth = 0;

    for line in corefile.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }

        if depth == 0 {
            if let Some(stripped) = line.strip_suffix('{') {
                let zone = stripped.trim().trim_matches('"').trim_matches('/');
                if !zone.is_empty() {
                    if let Some(z) = current_zone.take() {
                        blocks.push(CoreBlock { zone: z, plugins: std::mem::take(&mut current_plugins) });
                    }
                    current_zone = Some(zone.to_string());
                    depth = 1;
                }
            } else if !line.is_empty() {
                // Single-line block
                let zone = line.trim_matches('"').trim_matches('/');
                if let Some(z) = current_zone.take() {
                    blocks.push(CoreBlock { zone: z, plugins: std::mem::take(&mut current_plugins) });
                }
                blocks.push(CoreBlock { zone: zone.to_string(), plugins: vec![] });
            }
        } else {
            if line == "}" {
                depth -= 1;
                if depth == 0 {
                    if let Some(z) = current_zone.take() {
                        blocks.push(CoreBlock { zone: z, plugins: std::mem::take(&mut current_plugins) });
                    }
                }
            } else if let Some(stripped) = line.strip_suffix('{') {
                // Nested block (e.g. hosts { ... })
                depth += 1;
                // Parse as plugin with args from the part before {
                let plugin_line = stripped.trim();
                let parts: Vec<&str> = plugin_line.split_whitespace().collect();
                if !parts.is_empty() {
                    current_plugins.push(CorePlugin {
                        name: parts[0].to_string(),
                        args: parts[1..].iter().map(|s| s.to_string()).collect(),
                    });
                }
            } else {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if !parts.is_empty() {
                    // If we're in a nested block (depth > 1), these are args to the last plugin
                    if depth > 1 {
                        if let Some(last) = current_plugins.last_mut() {
                            last.args.extend(parts.iter().map(|s| s.to_string()));
                        }
                    } else {
                        current_plugins.push(CorePlugin {
                            name: parts[0].to_string(),
                            args: parts[1..].iter().map(|s| s.to_string()).collect(),
                        });
                    }
                }
            }
        }
    }
    if let Some(z) = current_zone.take() {
        blocks.push(CoreBlock { zone: z, plugins: std::mem::take(&mut current_plugins) });
    }
    blocks
}

/// Import errors.
#[derive(Debug, Clone, edgerun_error::Error)]
pub enum ImportError {
    EmptyConfig,
    InvalidValue(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_dnsmasq_basic() {
        let conf = r#"
interface=eth0
dhcp-range=192.168.1.100,192.168.1.200,255.255.255.0
dhcp-host=aa:bb:cc:dd:ee:ff,192.168.1.50,printer
dhcp-option=3,192.168.1.1
dhcp-option=6,8.8.8.8,1.1.1.1
dhcp-option=15,home.local
server=8.8.8.8
server=1.1.1.1
"#;
        let resources = import_dnsmasq(conf).unwrap();
        assert!(resources.iter().any(|r| matches!(r, ConfigResource::DhcpServer(_))));
        assert!(resources.iter().any(|r| matches!(r, ConfigResource::DhcpPool(_))));
        assert!(resources.iter().any(|r| matches!(r, ConfigResource::DnsForwarder(_))));
    }

    #[test]
    fn test_import_corefile_basic() {
        let corefile = r#"
. {
    forward . 8.8.8.8 1.1.1.1
    cache 30
}

cluster.local {
    forward . 10.96.0.10
    cache 5
}
"#;
        let resources = import_corefile(corefile).unwrap();
        assert!(resources.iter().any(|r| matches!(r, ConfigResource::ForwardingRule(_))));
    }

    #[test]
    fn test_import_corefile_with_hosts() {
        let corefile = r#"
. {
    hosts {
        10.0.0.1 myapp.local
        10.0.0.2 api.local
        fallthrough
    }
    forward . 8.8.8.8
}
"#;
        let resources = import_corefile(corefile).unwrap();
        assert!(resources.iter().any(|r| matches!(r, ConfigResource::DnsZone(_))));
    }
}
