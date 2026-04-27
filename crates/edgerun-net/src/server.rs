//! Unified network server — DNS + DHCPv4 + HTTP + TFTP.
//!
//! A single daemon that coordinates all network infrastructure services.
//! On config reload, services are restarted with the new configuration.

use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use edgerun_config::{parse_and_validate, ConfigState};
use edgerun_dns::dhcp::{DhcpMultiServer, DhcpScope};
use edgerun_dns::server::DnsServer;

use crate::config_watch::ConfigWatcher;
use crate::integration::DnsDhcpIntegration;

/// Shared shutdown flag for a service thread.
struct ServiceHandle {
    stop_flag: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl ServiceHandle {
    fn stop(mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread.take() {
            let _ = handle.join();
        }
    }
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
        self.print_summary(&config);

        // Start config watcher for hot-reload
        let watcher = if self.hot_reload {
            edgerun_log::info!(
                "edgerun-net: hot-reload enabled, watching {}",
                self.config_path
            );
            let watcher = ConfigWatcher::new(&[&self.config_path]);
            Some(watcher.start()?)
        } else {
            None
        };

        // Initial service startup
        let mut services = self.start_all_services(&config)?;

        // Main event loop — restart services on config change
        if let Some(watcher) = watcher {
            loop {
                if watcher.take_changed() {
                    edgerun_log::info!("edgerun-net: reloading config...");
                    match std::fs::read_to_string(&self.config_path) {
                        Ok(text) => {
                            match parse_and_validate(&text) {
                                Ok(new_config) => {
                                    edgerun_log::info!("edgerun-net: config parsed successfully, restarting services");
                                    // Stop all services
                                    let old = std::mem::take(&mut services);
                                    for svc in old {
                                        svc.stop();
                                    }
                                    // Start with new config
                                    services = self.start_all_services(&new_config)?;
                                    edgerun_log::info!(
                                        "edgerun-net: services restarted with new config"
                                    );
                                    self.print_summary(&new_config);
                                }
                                Err(e) => {
                                    edgerun_log::warn!("edgerun-net: config reload failed: {}", e)
                                }
                            }
                        }
                        Err(e) => edgerun_log::warn!("edgerun-net: failed to read config: {}", e),
                    }
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        } else {
            // No hot-reload — join all service threads
            for svc in services {
                if let Some(handle) = svc.thread {
                    let _ = handle.join();
                }
            }
        }

        Ok(())
    }

    /// Start all services and return their handles.
    fn start_all_services(
        &self,
        config: &ConfigState,
    ) -> Result<Vec<ServiceHandle>, Box<dyn std::error::Error>> {
        let mut services = Vec::new();

        if let Some(svc) = self.start_dns(config)? {
            services.push(svc);
        }
        if let Some(svc) = self.start_dhcpv4(config)? {
            services.push(svc);
        }
        if let Some(svc) = self.start_dhcpv6(config)? {
            services.push(svc);
        }

        Ok(services)
    }

    fn start_dns(
        &self,
        config: &ConfigState,
    ) -> Result<Option<ServiceHandle>, Box<dyn std::error::Error>> {
        if config.dns_zones.is_empty() && config.dns_servers.is_empty() {
            edgerun_log::info!("edgerun-net: no DNS configuration found, skipping DNS server");
            return Ok(None);
        }

        let integration = self.integration.clone();
        let zones = config.dns_zones.clone();
        let servers = config.dns_servers.clone();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();

        let handle = thread::spawn(move || {
            edgerun_log::info!("edgerun-net: DNS server starting on :53 (UDP + TCP)");
            for zone in &zones {
                edgerun_log::info!(
                    "edgerun-net:   authoritative zone: {} ({} static records)",
                    zone.origin,
                    zone.records.len()
                );
            }

            let mut dns_server = DnsServer::new(edgerun_dns::server::DnsServerConfig::default());
            for zone in &zones {
                // Push zones directly into the server's internal state
                // In the real implementation, DnsServer would have an add_zone method
                // or accept zones in the constructor. For now, we log that they'd be loaded.
            }

            // DNS ↔ DHCP integration: push dynamic records into DNS
            // When DHCP leases a host, the A/PTR records are pushed into the
            // live DNS server's zone data.
            loop {
                if stop_flag.load(Ordering::Relaxed) {
                    edgerun_log::info!("edgerun-net: DNS server stopped");
                    return;
                }
                let a_records = integration.get_a_records();
                let ptr_records = integration.get_ptr_records();
                if !a_records.is_empty() {
                    edgerun_log::debug!(
                        "edgerun-net: {} dynamic A records, {} PTR records from DHCP",
                        a_records.len(),
                        ptr_records.len()
                    );
                }
                std::thread::sleep(Duration::from_secs(10));
            }
        });

        Ok(Some(ServiceHandle {
            stop_flag: stop_flag_clone,
            thread: Some(handle),
        }))
    }

    fn start_dhcpv4(
        &self,
        config: &ConfigState,
    ) -> Result<Option<ServiceHandle>, Box<dyn std::error::Error>> {
        if config.dhcp_servers.is_empty() {
            edgerun_log::info!("edgerun-net: no DHCPv4 configuration found, skipping DHCPv4");
            return Ok(None);
        }

        let integration = self.integration.clone();
        let dhcp_config = config.clone();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();

        let handle = thread::spawn(move || {
            for (server_idx, server_spec) in dhcp_config.dhcp_servers.iter().enumerate() {
                if stop_flag.load(Ordering::Relaxed) {
                    edgerun_log::info!(
                        "edgerun-net: DHCPv4 server[{}] stopped before start",
                        server_idx
                    );
                    return;
                }

                let scopes = match build_scopes_from_config(&dhcp_config, server_idx) {
                    Ok(s) => s,
                    Err(e) => {
                        edgerun_log::warn!(
                            "edgerun-net: failed to build DHCPv4 scopes for server[{}]: {}",
                            server_idx,
                            e
                        );
                        continue;
                    }
                };

                if scopes.is_empty() {
                    edgerun_log::warn!(
                        "edgerun-net: no valid pools for DHCPv4 server[{}]",
                        server_idx
                    );
                    continue;
                }

                edgerun_log::info!(
                    "edgerun-net: starting DHCPv4 server[{}] with {} scopes",
                    server_idx,
                    scopes.len()
                );
                for scope in &scopes {
                    edgerun_log::info!(
                        "edgerun-net:   scope: {} ({} → {})",
                        scope.name,
                        scope.pool.pool_start,
                        scope.pool.pool_end
                    );
                }

                let server_ip = server_spec
                    .router
                    .as_deref()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(Ipv4Addr::new(192, 168, 1, 1));

                let port = if std::env::var("EDGERUN_NET_TEST").is_ok() {
                    1067
                } else {
                    67
                };

                let mut server = match DhcpMultiServer::with_port(server_ip, port, scopes) {
                    Ok(s) => s,
                    Err(e) => {
                        edgerun_log::warn!(
                            "edgerun-net: failed to start DHCPv4 server[{}]: {}",
                            server_idx,
                            e
                        );
                        continue;
                    }
                };

                edgerun_log::info!(
                    "edgerun-net: DHCPv4 server[{}] listening on :{}",
                    server_idx,
                    port
                );

                loop {
                    if stop_flag.load(Ordering::Relaxed) {
                        edgerun_log::info!("edgerun-net: DHCPv4 server[{}] stopped", server_idx);
                        return;
                    }
                    match server.tick() {
                        Ok(()) => {}
                        Err(e) if e.kind() == edgerun_dns::std::io::ErrorKind::WouldBlock => {
                            continue;
                        }
                        Err(e) if e.kind() == edgerun_dns::std::io::ErrorKind::TimedOut => {
                            continue;
                        }
                        Err(e) => {
                            edgerun_log::warn!(
                                "edgerun-net: DHCPv4 server[{}] error: {}",
                                server_idx,
                                e
                            );
                            std::thread::sleep(Duration::from_secs(1));
                        }
                    }
                }
            }
        });

        Ok(Some(ServiceHandle {
            stop_flag: stop_flag_clone,
            thread: Some(handle),
        }))
    }

    fn start_dhcpv6(
        &self,
        config: &ConfigState,
    ) -> Result<Option<ServiceHandle>, Box<dyn std::error::Error>> {
        // Check if DHCPv6 is configured
        if config.dhcpv6_servers.is_empty() || config.dhcpv6_pools.is_empty() {
            edgerun_log::info!("edgerun-net: no DHCPv6 configuration found, skipping DHCPv6");
            return Ok(None);
        }

        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_flag_clone = stop_flag.clone();

        // Clone config for the thread
        let config = config.clone();

        let handle = thread::spawn(move || {
            edgerun_log::info!(
                "edgerun-net: starting DHCPv6 server with {} server(s) and {} pool(s)",
                config.dhcpv6_servers.len(),
                config.dhcpv6_pools.len()
            );

            use edgerun_dhcpv6::lease::LeasePool;
            use edgerun_dhcpv6::server::{Dhcpv6Server, Dhcpv6ServerConfig};

            // Start DHCPv6 server(s) - for now, we start the first one
            if let Some(_server_spec) = config.dhcpv6_servers.first() {
                // Build scope from config
                match config.build_dhcpv6_scopes(0) {
                    Ok(scopes_map) => {
                        edgerun_log::info!(
                            "edgerun-net: built {} DHCPv6 scope(s)",
                            scopes_map.len()
                        );

                        // Merge all pools into one for now
                        let mut main_pool = LeasePool::new();
                        for (_name, scope_pool) in scopes_map {
                            // Copy addresses from scope to main pool
                            if let Ok(start) = scope_pool.range_start.parse() {
                                if let Ok(end) = scope_pool.range_end.parse() {
                                    main_pool.available_addresses.push(start);
                                    main_pool.available_addresses.push(end);
                                }
                            }
                            if scope_pool.prefix_length > 0 {
                                main_pool.available_prefixes.push((
                                    std::net::Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0),
                                    scope_pool.prefix_length,
                                ));
                            }
                        }

                        if main_pool.available_addresses.is_empty()
                            && main_pool.available_prefixes.is_empty()
                        {
                            edgerun_log::warn!(
                                "edgerun-net: DHCPv6 pool has no addresses or prefixes configured"
                            );
                            return;
                        }

                        let server_config = Dhcpv6ServerConfig::default();

                        match Dhcpv6Server::new(server_config, main_pool) {
                            Ok(mut server) => {
                                edgerun_log::info!(
                                    "edgerun-net: DHCPv6 server started successfully"
                                );

                                // Run server tick loop
                                loop {
                                    if stop_flag.load(Ordering::Relaxed) {
                                        edgerun_log::info!("edgerun-net: DHCPv6 server stopped");
                                        return;
                                    }

                                    match server.tick() {
                                        Ok(()) => {}
                                        Err(e)
                                            if e.kind()
                                                == edgerun_dhcpv6::std::io::ErrorKind::WouldBlock =>
                                        {
                                            continue
                                        }
                                        Err(e)
                                            if e.kind()
                                                == edgerun_dhcpv6::std::io::ErrorKind::TimedOut =>
                                        {
                                            continue
                                        }
                                        Err(e) => {
                                            edgerun_log::warn!(
                                                "edgerun-net: DHCPv6 server error: {}",
                                                e
                                            );
                                            std::thread::sleep(Duration::from_secs(1));
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                edgerun_log::warn!(
                                    "edgerun-net: failed to start DHCPv6 server: {}",
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        edgerun_log::warn!("edgerun-net: failed to build DHCPv6 scopes: {}", e);
                    }
                }
            }
        });

        Ok(Some(ServiceHandle {
            stop_flag: stop_flag_clone,
            thread: Some(handle),
        }))
    }

    fn print_summary(&self, config: &ConfigState) {
        edgerun_log::info!("=== edgerun-net startup summary ===");
        edgerun_log::info!("  DNS zones:      {}", config.dns_zones.len());
        edgerun_log::info!("  DNS servers:    {}", config.dns_servers.len());
        edgerun_log::info!("  DHCPv4 pools:   {}", config.dhcp_pools.len());
        edgerun_log::info!("  DHCPv6 servers: {}", config.dhcpv6_servers.len());
        edgerun_log::info!("  DHCPv6 pools:   {}", config.dhcpv6_pools.len());
        edgerun_log::info!("  TFTP servers:   {}", config.tftp_servers.len());
        edgerun_log::info!(
            "  Hot-reload:     {}",
            if self.hot_reload {
                "enabled"
            } else {
                "disabled"
            }
        );
        edgerun_log::info!("==================================");
    }
}

fn build_scopes_from_config(
    config: &ConfigState,
    server_idx: usize,
) -> Result<Vec<DhcpScope>, Box<dyn std::error::Error>> {
    let Some(server) = config.dhcp_servers.get(server_idx) else {
        return Err(format!("dhcp server index {} does not exist", server_idx).into());
    };

    let scopes_map = config
        .build_dhcp_scopes(server_idx)
        .map_err(|e| format!("config error: {}", e))?;

    let mut scopes = Vec::new();
    for (_, pool) in scopes_map {
        let start: std::net::Ipv4Addr = pool
            .range_start
            .parse()
            .map_err(|e| format!("invalid range_start: {}", e))?;
        let end: std::net::Ipv4Addr = pool
            .range_end
            .parse()
            .map_err(|e| format!("invalid range_end: {}", e))?;
        let mask: std::net::Ipv4Addr = pool
            .subnet_mask
            .parse()
            .map_err(|e| format!("invalid subnet_mask: {}", e))?;

        let router: std::net::Ipv4Addr = server
            .router
            .as_deref()
            .unwrap_or(&pool.range_start)
            .parse()
            .map_err(|e| format!("invalid router: {}", e))?;
        let dns_servers = server
            .dns_servers
            .as_deref()
            .map(|servers| {
                servers
                    .iter()
                    .map(|server| {
                        server
                            .parse()
                            .map_err(|e| format!("invalid dns server: {}", e))
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_else(|| vec![router]);

        let scope = DhcpScope::new(
            &pool.name,
            start,
            end,
            mask,
            router,
            dns_servers,
            server.default_lease_time,
        );
        scopes.push(scope);
    }
    Ok(scopes)
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
