//! DHCP scopes (subnets) — multiple pool support.
//!
//! A **scope** is a subnet-specific address pool with its own configuration
//! (mask, router, DNS, etc.). The server selects the correct scope based on:
//! - `giaddr` (relay agent IP) — tells us which subnet the client is on
//! - The `ciaddr` field for renewals
//! - Subnet mask matching

use crate::libc;
use crate::std::io;
use crate::std::net::{Ipv4Addr, SocketAddr};
use alloc::collections::BTreeMap as HashMap;
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use super::lease::LeasePool;
use super::message::{
    DhcpMessage, DhcpMessageType, DhcpOp, NetworkConfig, DHCP_CLIENT_PORT, DHCP_SERVER_PORT,
};

/// A DHCP scope (subnet-specific pool + config).
#[derive(Debug)]
pub struct DhcpScope {
    /// Human-readable name for this scope.
    pub name: String,
    /// Subnet mask for this scope.
    pub subnet_mask: Ipv4Addr,
    /// Router/gateway for this scope.
    pub router: Ipv4Addr,
    /// DNS servers for this scope.
    pub dns_servers: Vec<Ipv4Addr>,
    /// Lease time for this scope.
    pub lease_time: u32,
    /// TFTP server for PXE.
    pub tftp_server: Option<Ipv4Addr>,
    /// Bootfile name for PXE.
    pub bootfile: Option<String>,
    /// The lease pool for this scope.
    pub pool: LeasePool,
    /// The network address of this scope (computed from mask).
    pub network: Ipv4Addr,
}

impl DhcpScope {
    /// Create a new DHCP scope.
    pub fn new(
        name: &str,
        pool_start: Ipv4Addr,
        pool_end: Ipv4Addr,
        subnet_mask: Ipv4Addr,
        router: Ipv4Addr,
        dns_servers: Vec<Ipv4Addr>,
        lease_time: u32,
    ) -> Self {
        let network = apply_mask(&pool_start, &subnet_mask);
        let mut pool = LeasePool::new(pool_start, pool_end);

        // Reserve server IP (network addr), gateway, broadcast, and DNS
        pool.reserve(network);
        pool.reserve(router);
        pool.reserve(network_broadcast(&network, &subnet_mask));
        for dns in &dns_servers {
            pool.reserve(*dns);
        }

        Self {
            name: name.to_string(),
            subnet_mask,
            router,
            dns_servers,
            lease_time,
            tftp_server: None,
            bootfile: None,
            pool,
            network,
        }
    }

    /// Set PXE boot parameters for this scope.
    pub fn with_pxe(mut self, tftp_server: Ipv4Addr, bootfile: String) -> Self {
        self.tftp_server = Some(tftp_server);
        self.bootfile = Some(bootfile);
        self
    }

    /// Get the broadcast address for this scope.
    pub fn broadcast(&self) -> Ipv4Addr {
        network_broadcast(&self.network, &self.subnet_mask)
    }

    /// Check if an IP is in this scope's network.
    pub fn contains_ip(&self, ip: Ipv4Addr) -> bool {
        apply_mask(&ip, &self.subnet_mask) == self.network
    }

    /// Statistics for this scope.
    pub fn stats(&self) -> String {
        format!(
            "{}: {} leased / {} available (mask: {}, router: {})",
            self.name,
            self.pool.active_count(),
            self.pool.available_count(),
            self.subnet_mask,
            self.router,
        )
    }
}

/// Helper: apply subnet mask to IP.
fn apply_mask(ip: &Ipv4Addr, mask: &Ipv4Addr) -> Ipv4Addr {
    edgerun_encoding::ip::network_address(ip, mask)
}

/// Helper: broadcast address for network + mask.
fn network_broadcast(network: &Ipv4Addr, mask: &Ipv4Addr) -> Ipv4Addr {
    edgerun_encoding::ip::broadcast_address(network, mask)
}

/// A DHCP server with multiple scopes (subnets).
pub struct DhcpMultiServer {
    socket: crate::std::net::UdpSocket,
    /// Scopes indexed by name.
    scopes: HashMap<String, DhcpScope>,
    /// Interface name for binding.
    interface: Option<String>,
    /// Server's main IP address.
    server_ip: Ipv4Addr,
}

impl DhcpMultiServer {
    /// Create a new multi-scope DHCP server on the given port.
    /// Use a port > 1024 to avoid requiring root (e.g. 1067 for testing).
    pub fn with_port(
        server_ip: Ipv4Addr,
        port: u16,
        scopes: Vec<DhcpScope>,
    ) -> Result<Self, io::Error> {
        use crate::std::net::UdpSocket;
        use crate::std::time::Duration;

        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        socket.set_broadcast(true)?;
        socket.set_read_timeout(Some(Duration::from_millis(200)))?;
        #[cfg(unix)]
        {
            use crate::std::os::unix::io::AsRawFd;
            let fd = socket.as_raw_fd();
            let opt: libc::c_int = 1;
            unsafe {
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_REUSEADDR,
                    &opt as *const _ as *const libc::c_void,
                    core::mem::size_of::<libc::c_int>() as libc::socklen_t,
                );
            }
        }

        let mut scope_map = HashMap::new();
        for scope in scopes {
            scope_map.insert(scope.name.clone(), scope);
        }

        Ok(Self {
            socket,
            scopes: scope_map,
            interface: None,
            server_ip,
        })
    }

    /// Create a new multi-scope DHCP server (default port 67, requires root).
    pub fn new(server_ip: Ipv4Addr, scopes: Vec<DhcpScope>) -> Result<Self, io::Error> {
        Self::with_port(server_ip, DHCP_SERVER_PORT, scopes)
    }

    /// Add a scope to this server.
    pub fn add_scope(&mut self, scope: DhcpScope) {
        self.scopes.insert(scope.name.clone(), scope);
    }

    /// Set the interface name.
    pub fn set_interface(&mut self, iface: String) {
        self.interface = Some(iface);
    }

    /// Select the correct scope for a client message.
    /// Uses giaddr (relay agent) first, then falls back to subnet matching.
    pub fn select_scope(&self, msg: &DhcpMessage) -> Option<&DhcpScope> {
        // If relay agent present, match by giaddr subnet
        if !msg.giaddr.is_unspecified() {
            return self.scopes.values().find(|s| s.contains_ip(msg.giaddr));
        }

        // For renewals, match by ciaddr subnet
        if !msg.ciaddr.is_unspecified() {
            if let Some(scope) = self.scopes.values().find(|s| s.contains_ip(msg.ciaddr)) {
                return Some(scope);
            }
        }

        // For direct requests, try to match by subnet mask
        if let Some(ref mask) = msg.options.subnet_mask {
            return self.scopes.values().find(|s| &s.subnet_mask == mask);
        }

        // Fallback: return first scope
        self.scopes.values().next()
    }

    /// Run the server event loop.
    pub fn run(&mut self) -> Result<(), io::Error> {
        edgerun_log::warn!(
            "edgerun-dhcp: multi-scope server listening on 0.0.0.0:{}",
            DHCP_SERVER_PORT
        );
        for (name, scope) in &self.scopes {
            edgerun_log::warn!("  scope {}: {}", name, scope.stats());
        }

        loop {
            match self.tick() {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) if e.kind() == io::ErrorKind::TimedOut => continue,
                Err(e) => {
                    edgerun_log::warn!("edgerun-dhcp: server error: {}", e);
                    return Err(e);
                }
            }
        }
    }

    /// Process one incoming packet.
    pub fn tick(&mut self) -> Result<(), io::Error> {
        let mut buf = [0u8; 1500];
        let (n, src) = self.socket.recv_from(&mut buf)?;

        let msg = match DhcpMessage::from_wire(&buf[..n]) {
            Ok(m) => m,
            Err(e) => {
                edgerun_log::warn!("edgerun-dhcp: failed to parse from {}: {}", src, e);
                return Ok(());
            }
        };

        let scope_name = self.select_scope(&msg).map(|s| s.name.clone());

        if let Some(name) = scope_name {
            let server_ip = self.server_ip;
            let socket = &self.socket;
            if let Some(scope) = self.scopes.get_mut(&name) {
                handle_scope_message(scope, &msg, src, server_ip, socket)?;
            }
        }

        Ok(())
    }

    /// Get statistics for all scopes.
    pub fn stats(&self) -> HashMap<&str, String> {
        self.scopes
            .iter()
            .map(|(k, v)| (k.as_str(), v.stats()))
            .collect()
    }

    /// Get the number of scopes.
    pub fn scope_count(&self) -> usize {
        self.scopes.len()
    }
}

/// Handle a DHCP message for a specific scope.
fn handle_scope_message(
    scope: &mut DhcpScope,
    msg: &DhcpMessage,
    src: SocketAddr,
    server_ip: Ipv4Addr,
    socket: &crate::std::net::UdpSocket,
) -> Result<(), io::Error> {
    let mt = match msg.options.message_type {
        Some(m) => m,
        None => return Ok(()),
    };

    match mt {
        DhcpMessageType::Discover => handle_discover(scope, msg, server_ip, socket),
        DhcpMessageType::Request => handle_request(scope, msg, server_ip, socket),
        DhcpMessageType::Release => handle_release(scope, msg),
        DhcpMessageType::Decline => handle_decline(scope, msg),
        DhcpMessageType::Inform => handle_inform(scope, msg, server_ip, socket),
        _ => Ok(()),
    }
}

fn handle_discover(
    scope: &mut DhcpScope,
    msg: &DhcpMessage,
    server_ip: Ipv4Addr,
    socket: &crate::std::net::UdpSocket,
) -> Result<(), io::Error> {
    let mac = msg.client_mac();
    let ip = match scope.pool.allocate(
        mac,
        msg.options.client_id.clone(),
        scope.lease_time,
        msg.xid,
    ) {
        Some(ip) => ip,
        None => return send_nak(server_ip, msg.xid, mac, msg, socket),
    };

    let offer = DhcpMessage::offer(
        msg.xid,
        mac,
        ip,
        NetworkConfig {
            server_id: server_ip,
            subnet_mask: scope.subnet_mask,
            router: scope.router,
            dns_servers: scope.dns_servers.clone(),
            lease_time: scope.lease_time,
            tftp_server: scope.tftp_server.map(|i| i.to_string()),
            bootfile: scope.bootfile.clone(),
        },
    );

    send_reply(&offer, msg, server_ip, socket)
}

fn handle_request(
    scope: &mut DhcpScope,
    msg: &DhcpMessage,
    server_ip: Ipv4Addr,
    socket: &crate::std::net::UdpSocket,
) -> Result<(), io::Error> {
    let mac = msg.client_mac();
    let ip = msg.options.requested_ip.unwrap_or(msg.ciaddr);

    let ack = DhcpMessage::ack(
        msg.xid,
        mac,
        ip,
        NetworkConfig {
            server_id: server_ip,
            subnet_mask: scope.subnet_mask,
            router: scope.router,
            dns_servers: scope.dns_servers.clone(),
            lease_time: scope.lease_time,
            tftp_server: scope.tftp_server.map(|i| i.to_string()),
            bootfile: scope.bootfile.clone(),
        },
    );

    send_reply(&ack, msg, server_ip, socket)
}

fn handle_release(_scope: &DhcpScope, msg: &DhcpMessage) -> Result<(), io::Error> {
    edgerun_log::warn!("edgerun-dhcp: RELEASE for {:?}", msg.options.requested_ip);
    Ok(())
}

fn handle_decline(scope: &mut DhcpScope, msg: &DhcpMessage) -> Result<(), io::Error> {
    let mac = msg.client_mac();
    if let Some(ip) = msg.options.requested_ip {
        scope.pool.record_conflict(ip, mac);
    }
    Ok(())
}

fn handle_inform(
    scope: &mut DhcpScope,
    msg: &DhcpMessage,
    server_ip: Ipv4Addr,
    socket: &crate::std::net::UdpSocket,
) -> Result<(), io::Error> {
    let mac = msg.client_mac();
    let mut ack = DhcpMessage::ack(
        msg.xid,
        mac,
        msg.ciaddr,
        NetworkConfig {
            server_id: server_ip,
            subnet_mask: scope.subnet_mask,
            router: scope.router,
            dns_servers: scope.dns_servers.clone(),
            lease_time: 0,
            tftp_server: None,
            bootfile: None,
        },
    );
    ack.options.lease_time = None;
    ack.options.renewal_time = None;
    ack.options.rebind_time = None;
    send_reply(&ack, msg, server_ip, socket)
}

fn send_reply(
    msg: &DhcpMessage,
    req: &DhcpMessage,
    server_ip: Ipv4Addr,
    socket: &crate::std::net::UdpSocket,
) -> Result<(), io::Error> {
    let wire = msg.to_wire();
    let dest = if !req.giaddr.is_unspecified() {
        SocketAddr::new(crate::std::net::IpAddr::V4(req.giaddr), DHCP_SERVER_PORT)
    } else if !req.broadcast {
        SocketAddr::new(crate::std::net::IpAddr::V4(req.ciaddr), DHCP_CLIENT_PORT)
    } else {
        SocketAddr::new(
            crate::std::net::IpAddr::V4(Ipv4Addr::BROADCAST),
            DHCP_CLIENT_PORT,
        )
    };
    socket.send_to(&wire, dest)?;
    Ok(())
}

fn send_nak(
    server_ip: Ipv4Addr,
    xid: u32,
    mac: [u8; 6],
    req: &DhcpMessage,
    socket: &crate::std::net::UdpSocket,
) -> Result<(), io::Error> {
    let nak = DhcpMessage::nak(xid, server_ip, mac);
    let wire = nak.to_wire();
    let dest = SocketAddr::new(
        crate::std::net::IpAddr::V4(Ipv4Addr::BROADCAST),
        DHCP_CLIENT_PORT,
    );
    socket.send_to(&wire, dest)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_creation() {
        let scope = DhcpScope::new(
            "test-scope",
            Ipv4Addr::new(10, 0, 0, 100),
            Ipv4Addr::new(10, 0, 0, 200),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(10, 0, 0, 1),
            vec![Ipv4Addr::new(8, 8, 8, 8)],
            3600,
        );
        assert_eq!(scope.name, "test-scope");
        assert_eq!(scope.network, Ipv4Addr::new(10, 0, 0, 0));
        assert_eq!(scope.router, Ipv4Addr::new(10, 0, 0, 1));
        assert!(scope.contains_ip(Ipv4Addr::new(10, 0, 0, 50)));
        assert!(!scope.contains_ip(Ipv4Addr::new(192, 168, 1, 1)));
    }

    #[test]
    fn test_scope_contains_ip() {
        let scope = DhcpScope::new(
            "class-c",
            Ipv4Addr::new(192, 168, 1, 100),
            Ipv4Addr::new(192, 168, 1, 200),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(192, 168, 1, 1),
            vec![],
            3600,
        );
        assert!(scope.contains_ip(Ipv4Addr::new(192, 168, 1, 1)));
        assert!(scope.contains_ip(Ipv4Addr::new(192, 168, 1, 254)));
        assert!(!scope.contains_ip(Ipv4Addr::new(192, 168, 2, 1)));
        assert!(!scope.contains_ip(Ipv4Addr::new(10, 0, 0, 1)));
    }

    #[test]
    fn test_multi_server_scope_selection() {
        let scope1 = DhcpScope::new(
            "net-10",
            Ipv4Addr::new(10, 0, 0, 100),
            Ipv4Addr::new(10, 0, 0, 200),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(10, 0, 0, 1),
            vec![],
            3600,
        );
        let scope2 = DhcpScope::new(
            "net-192",
            Ipv4Addr::new(192, 168, 1, 100),
            Ipv4Addr::new(192, 168, 1, 200),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(192, 168, 1, 1),
            vec![],
            3600,
        );

        let server =
            DhcpMultiServer::with_port(Ipv4Addr::new(10, 0, 0, 1), 1067, vec![scope1, scope2])
                .unwrap();

        assert_eq!(server.scope_count(), 2);

        // Can't test giaddr selection without constructing messages, but
        // we can verify scopes are stored
        let stats = server.stats();
        assert!(stats.contains_key("net-10"));
        assert!(stats.contains_key("net-192"));
    }

    #[test]
    fn test_scope_stats() {
        let scope = DhcpScope::new(
            "stats-test",
            Ipv4Addr::new(10, 0, 0, 100),
            Ipv4Addr::new(10, 0, 0, 200),
            Ipv4Addr::new(255, 255, 255, 0),
            Ipv4Addr::new(10, 0, 0, 1),
            vec![],
            3600,
        );
        let stats = scope.stats();
        assert!(stats.contains("stats-test"));
        assert!(stats.contains("available"));
    }
}
