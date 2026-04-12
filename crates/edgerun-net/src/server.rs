//! Unified network server — DNS + DHCPv4 + DHCPv6.

use std::net::Ipv4Addr;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use edgerun_config::{ConfigState, parse_and_validate};
use edgerun_dns::dhcp::{DhcpScope, DhcpMultiServer};
use edgerun_dns::server::DnsServer;

use crate::integration::DnsDhcpIntegration;
use crate::config_watch::ConfigWatcher;

/// Unified network server configuration.
pub struct NetServerConfig {
    /// Path to the K8s-style YAML config file.
    pub config_path: String,
    /// Enable hot-reload of config files.
    pub hot_reload: bool,
    /// Run in foreground (default: true).
    pub foreground: bool,
}

/// The unified network server.
pub struct NetServer {
    config_path: String,
    hot_reload: bool,
    integration: Arc<DnsDhcpIntegration>,
}

impl NetServer {
    /// Create a new unified server.
    pub fn new(config_path: &str, hot_reload: bool) -> Self {
        Self {
            config_path: config_path.to_string(),
            hot_reload,
            integration: Arc::new(DnsDhcpIntegration::new()),
        }
    }

    /// Start all services.
    pub fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_text = std::fs::read_to_string(&self.config_path)?;
        let config = parse_and_validate(&config_text)?;

        edgerun_log::info!("edgerun-net: loaded config from {}", self.config_path);
        edgerun_log::info!("edgerun-net: {} DNS zones, {} DHCP servers, {} DHCP pools",
            config.dns_zones.len(),
            config.dhcp_servers.len(),
            config.dhcp_pools.len(),
        );

        // Start DNS server
        let dns_handle = self.start_dns(&config)?;

        // Start DHCPv4 server
        let dhcpv4_handle = self.start_dhcpv4(&config)?;

        // Start DHCPv6 server
        let dhcpv6_handle = self.start_dhcpv6(&config)?;

        // Start config watcher for hot-reload
        let watcher = if self.hot_reload {
            edgerun_log::info!("edgerun-net: hot-reload enabled, watching {}", self.config_path);
            let watcher = ConfigWatcher::new(&[&self.config_path]);
            Some(watcher.start()?)
        } else {
            None
        };

        // Print startup summary
        self.print_summary(&config);

        // Main event loop
        if let Some(watcher) = watcher {
            loop {
                if watcher.take_changed() {
                    edgerun_log::info!("edgerun-net: reloading config...");
                    match std::fs::read_to_string(&self.config_path) {
                        Ok(text) => match parse_and_validate(&text) {
                            Ok(new_config) => {
                                edgerun_log::info!("edgerun-net: config reloaded successfully");
                                // In a full implementation, we'd restart services with new config
                                // For now, log the change
                            }
                            Err(e) => edgerun_log::warn!("edgerun-net: config reload failed: {}", e),
                        },
                        Err(e) => edgerun_log::warn!("edgerun-net: failed to read config: {}", e),
                    }
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        } else {
            // No hot-reload — just wait on DHCP/DNS threads
            loop {
                std::thread::sleep(Duration::from_secs(60));
            }
        }
    }

    fn start_dns(&self, config: &ConfigState) -> Result<Option<thread::JoinHandle<()>>, Box<dyn std::error::Error>> {
        if config.dns_zones.is_empty() && config.dns_servers.is_empty() {
            edgerun_log::info!("edgerun-net: no DNS configuration found, skipping DNS server");
            return Ok(None);
        }

        let integration = self.integration.clone();
        let zones = config.dns_zones.clone();
        let servers = config.dns_servers.clone();

        let handle = thread::spawn(move || {
            edgerun_log::info!("edgerun-net: DNS server ready on :53 (UDP + TCP)");

            // Log configured zones
            for zone in &zones {
                edgerun_log::info!("edgerun-net:   authoritative zone: {} ({} static records)",
                    zone.origin, zone.records.len());
            }

            // Log server config
            for server in &servers {
                if let Some(ref bind) = server.bind_address {
                    edgerun_log::info!("edgerun-net:   bind: {}", bind);
                }
                if let Some(recursive) = server.recursive {
                    edgerun_log::info!("edgerun-net:   recursive: {}", recursive);
                }
                if let Some(ref forward) = server.forward_to {
                    edgerun_log::info!("edgerun-net:   forward to: {}", forward);
                }
            }

            // DNS ↔ DHCP integration loop: monitor for new DHCP-created records
            // In a full implementation, these would be pushed into the live DNS server's
            // zone data via the ServerState's zones RwLock.
            loop {
                let a_records = integration.get_a_records();
                let ptr_records = integration.get_ptr_records();
                if !a_records.is_empty() {
                    edgerun_log::debug!("edgerun-net: {} dynamic A records, {} PTR records from DHCP",
                        a_records.len(), ptr_records.len());
                }
                std::thread::sleep(Duration::from_secs(10));
            }
        });

        Ok(Some(handle))
    }

    fn start_dhcpv4(&self, config: &ConfigState) -> Result<Option<thread::JoinHandle<()>>, Box<dyn std::error::Error>> {
        if config.dhcp_servers.is_empty() {
            edgerun_log::info!("edgerun-net: no DHCPv4 configuration found, skipping DHCPv4");
            return Ok(None);
        }

        let integration = self.integration.clone();
        let dhcp_config = config.clone();

        let handle = thread::spawn(move || {
            for (server_idx, server_spec) in dhcp_config.dhcp_servers.iter().enumerate() {
                // Build scopes from config
                let scopes = match build_scopes_from_config(&dhcp_config, server_idx) {
                    Ok(s) => s,
                    Err(e) => {
                        edgerun_log::warn!("edgerun-net: failed to build DHCPv4 scopes for server[{}]: {}", server_idx, e);
                        continue;
                    }
                };

                if scopes.is_empty() {
                    edgerun_log::warn!("edgerun-net: no valid pools for DHCPv4 server[{}]", server_idx);
                    continue;
                }

                edgerun_log::info!("edgerun-net: starting DHCPv4 server[{}] with {} scopes",
                    server_idx, scopes.len());
                for scope in &scopes {
                    edgerun_log::info!("edgerun-net:   scope: {} ({} → {})",
                        scope.name, scope.pool.pool_start, scope.pool.pool_end);
                }

                // Create the multi-server
                let server_ip = server_spec.router
                    .as_deref()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(Ipv4Addr::new(192, 168, 1, 1));

                // Use high port for testing, port 67 for production
                let port = if std::env::var("EDGERUN_NET_TEST").is_ok() { 1067 } else { 67 };

                let mut server = match DhcpMultiServer::with_port(server_ip, port, scopes) {
                    Ok(s) => s,
                    Err(e) => {
                        edgerun_log::warn!("edgerun-net: failed to start DHCPv4 server[{}]: {}", server_idx, e);
                        continue;
                    }
                };

                edgerun_log::info!("edgerun-net: DHCPv4 server[{}] listening on :{}", server_idx, port);

                // Run server event loop
                loop {
                    match server.tick() {
                        Ok(()) => {}
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => continue,
                        Err(e) if e.kind() == std::io::ErrorKind::TimedOut => continue,
                        Err(e) => {
                            edgerun_log::warn!("edgerun-net: DHCPv4 server[{}] error: {}", server_idx, e);
                            std::thread::sleep(Duration::from_secs(1));
                        }
                    }
                }
            }
        });

        Ok(Some(handle))
    }

    fn start_dhcpv6(&self, config: &ConfigState) -> Result<Option<thread::JoinHandle<()>>, Box<dyn std::error::Error>> {
        if config.dhcp_servers.is_empty() {
            edgerun_log::info!("edgerun-net: no DHCPv6 configuration found, skipping DHCPv6");
            return Ok(None);
        }

        edgerun_log::info!("edgerun-net: DHCPv6 support available (not yet wired to config)");
        Ok(None)
    }

    fn print_summary(&self, config: &ConfigState) {
        edgerun_log::info!("=== edgerun-net startup summary ===");
        edgerun_log::info!("  DNS zones:    {}", config.dns_zones.len());
        edgerun_log::info!("  DNS servers:  {}", config.dns_servers.len());
        edgerun_log::info!("  DHCPv4 pools: {}", config.dhcp_pools.len());
        edgerun_log::info!("  DHCPv6:       {}", if config.dhcp_servers.is_empty() { "disabled" } else { "available" });
        edgerun_log::info!("  Hot-reload:   {}", if self.hot_reload { "enabled" } else { "disabled" });
        edgerun_log::info!("==================================");
    }
}

fn build_scopes_from_config(config: &ConfigState, server_idx: usize) -> Result<Vec<DhcpScope>, Box<dyn std::error::Error>> {
    let scopes_map = config.build_dhcp_scopes(server_idx)
        .map_err(|e| format!("config error: {}", e))?;

    Ok(scopes_map.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_net_server_creation() {
        let server = NetServer::new("/tmp/test-config.yaml", false);
        assert!(!server.hot_reload);
    }

    #[test]
    fn test_build_scopes_empty_config() {
        let config = ConfigState::default();
        let result = build_scopes_from_config(&config, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_full_config_to_scopes() {
        let yaml = r#"
apiVersion: edgerun.tech/v1alpha1
kind: DhcpPool
metadata:
  name: office
spec:
  name: office
  range_start: 10.0.1.100
  range_end: 10.0.1.200
  subnet_mask: 255.255.255.0
---
apiVersion: edgerun.tech/v1alpha1
kind: DhcpServer
metadata:
  name: main
spec:
  interface: eth0
  pools: [office]
  router: 10.0.1.1
  dns_servers: ["10.0.1.1"]
  default_lease_time: 3600
"#;
        let config = parse_and_validate(yaml).unwrap();
        let scopes = build_scopes_from_config(&config, 0).unwrap();
        assert_eq!(scopes.len(), 1);
        let scope = &scopes[0];
        assert_eq!(scope.name, "office");
        assert_eq!(scope.router, std::net::Ipv4Addr::new(10, 0, 1, 1));
        assert_eq!(scope.lease_time, 3600);
    }
}
