//! DHCPv4 server — handles DISCOVER/REQUEST and responds with OFFER/ACK/NAK.

use std::io;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::sync::Arc;
use std::time::Duration;

use edgerun_rt::{AsyncUdpSocket, Mutex};

use super::lease::LeasePool;
use super::message::{
    DhcpMessage, DhcpMessageType, NetworkConfig, DHCP_CLIENT_PORT, DHCP_SERVER_PORT,
};

/// DHCPv4 server configuration.
pub struct DhcpServerConfig {
    /// Server IP address (the IP we bind to).
    pub server_ip: Ipv4Addr,
    /// Subnet mask for the pool.
    pub subnet_mask: Ipv4Addr,
    /// Default gateway / router.
    pub router: Ipv4Addr,
    /// DNS servers to hand out.
    pub dns_servers: Vec<Ipv4Addr>,
    /// Default lease time in seconds.
    pub lease_time: u32,
    // --- PXE Boot ---
    /// TFTP server address for PXE booting.
    pub tftp_server: Option<Ipv4Addr>,
    /// Default bootfile name (used if client doesn't specify arch).
    pub default_bootfile: Option<String>,
    /// Bootfile map: architecture name → bootfile (auto-selects based on client arch).
    pub bootfile_by_arch: std::collections::HashMap<String, String>,
}

/// DHCPv4 server — listens for client requests and hands out leases.
///
/// Now fully async using `edgerun_rt::AsyncUdpSocket`.
/// The pool is protected by an async `Mutex` so it can be
/// shared across async tasks.
pub struct DhcpServer {
    socket: Arc<AsyncUdpSocket>,
    config: DhcpServerConfig,
    pool: Mutex<LeasePool>,
    /// Interface name for binding.
    interface: Option<String>,
}

impl DhcpServer {
    /// Create a new DHCP server.
    ///
    /// `pool_start` and `pool_end` define the range of IPs to hand out.
    pub fn new(
        config: DhcpServerConfig,
        pool_start: Ipv4Addr,
        pool_end: Ipv4Addr,
    ) -> Result<Self, io::Error> {
        let std_socket = UdpSocket::bind(("0.0.0.0", DHCP_SERVER_PORT))?;
        std_socket.set_broadcast(true)?;
        let socket = Arc::new(AsyncUdpSocket::from_std(std_socket)?);

        let mut pool = LeasePool::new(pool_start, pool_end);

        // Reserve the server IP, gateway, and broadcast
        pool.reserve(config.server_ip);
        pool.reserve(config.router);
        let broadcast = network_broadcast(config.server_ip, config.subnet_mask);
        pool.reserve(broadcast);

        // Reserve DNS servers if they're in the pool range
        for dns in &config.dns_servers {
            pool.reserve(*dns);
        }

        Ok(Self {
            socket,
            config,
            pool: Mutex::new(pool),
            interface: None,
        })
    }

    /// Set the interface name (for logging / diagnostics).
    pub fn set_interface(&mut self, iface: String) {
        self.interface = Some(iface);
    }

    /// Run the server event loop until shutdown is requested.
    pub async fn run(&self, shutdown: edgerun_rt::CancellationToken) {
        edgerun_log::info!(
            "edgerun-dhcp: server listening on 0.0.0.0:{}",
            DHCP_SERVER_PORT
        );
        let pool = self.pool.lock().await;
        edgerun_log::info!("  pool: {} - {}", pool.pool_start, pool.pool_end);
        edgerun_log::info!(
            "  server: {}, router: {}, dns: {:?}",
            self.config.server_ip,
            self.config.router,
            self.config.dns_servers
        );
        drop(pool);

        while !shutdown.is_cancelled() {
            match self.tick().await {
                Ok(()) => {}
                Err(e) => {
                    edgerun_log::warn!("edgerun-dhcp: server error: {}", e);
                    edgerun_rt::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        edgerun_log::info!("edgerun-dhcp: server shut down");
    }

    /// Process one incoming packet.
    pub async fn tick(&self) -> Result<(), io::Error> {
        let mut buf = [0u8; 1500];
        let (n, src) = match self.socket.recv_from(&mut buf).await {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        let msg = match DhcpMessage::from_wire(&buf[..n]) {
            Ok(m) => m,
            Err(e) => {
                edgerun_log::warn!("edgerun-dhcp: failed to parse message from {}: {}", src, e);
                return Ok(());
            }
        };

        self.handle_message(&msg, src).await?;
        Ok(())
    }

    async fn handle_message(&self, msg: &DhcpMessage, src: SocketAddr) -> Result<(), io::Error> {
        let mt = match msg.options.message_type {
            Some(mt) => mt,
            None => {
                edgerun_log::warn!("edgerun-dhcp: message from {} has no message type", src);
                return Ok(());
            }
        };

        match mt {
            DhcpMessageType::Discover => self.handle_discover(msg).await,
            DhcpMessageType::Request => self.handle_request(msg).await,
            DhcpMessageType::Release => self.handle_release(msg).await,
            DhcpMessageType::Decline => self.handle_decline(msg).await,
            DhcpMessageType::Inform => self.handle_inform(msg).await,
            DhcpMessageType::Offer | DhcpMessageType::Ack | DhcpMessageType::Nak => Ok(()),
        }
    }

    async fn handle_discover(&self, msg: &DhcpMessage) -> Result<(), io::Error> {
        let mac = msg.client_mac();
        edgerun_log::info!(
            "edgerun-dhcp: DISCOVER from {} (xid=0x{:08x})",
            format_mac(mac),
            msg.xid
        );

        let ip = {
            let mut pool = self.pool.lock().await;
            pool.allocate(mac, self.config.lease_time, msg.xid)
        };
        let ip = match ip {
            Some(ip) => ip,
            None => {
                edgerun_log::warn!("edgerun-dhcp: pool exhausted, sending NAK");
                return self.send_nak(msg.xid).await;
            }
        };

        edgerun_log::info!("edgerun-dhcp: sending OFFER {} to {}", ip, format_mac(mac));

        let (tftp, bootfile) = self.pxe_boot_params(msg);

        let offer = DhcpMessage::offer(
            msg.xid,
            mac,
            ip,
            NetworkConfig {
                server_id: self.config.server_ip,
                subnet_mask: self.config.subnet_mask,
                router: self.config.router,
                dns_servers: self.config.dns_servers.clone(),
                lease_time: self.config.lease_time,
                tftp_server: tftp,
                bootfile,
            },
        );

        self.send_reply(&offer, msg).await
    }

    async fn handle_request(&self, msg: &DhcpMessage) -> Result<(), io::Error> {
        let mac = msg.client_mac();
        let requested_ip = msg.options.requested_ip;
        let server_id = msg.options.server_id;

        edgerun_log::info!(
            "edgerun-dhcp: REQUEST from {} (xid=0x{:08x}) req={:?} server={:?}",
            format_mac(mac),
            msg.xid,
            requested_ip,
            server_id
        );

        // If client is renewing (ciaddr is set, no requested_ip)
        if !msg.ciaddr.is_unspecified() && requested_ip.is_none() {
            let mut pool = self.pool.lock().await;
            if let Some(lease) = pool.find_lease_by_ip(msg.ciaddr) {
                if lease.mac == mac && !lease.is_expired() {
                    edgerun_log::info!(
                        "edgerun-dhcp: renewing {} for {}",
                        msg.ciaddr,
                        format_mac(mac)
                    );
                    drop(pool);
                    return self.send_ack(msg, msg.ciaddr).await;
                }
            }
            edgerun_log::warn!("edgerun-dhcp: renewal denied for {}", msg.ciaddr);
            drop(pool);
            return self.send_nak(msg.xid).await;
        }

        // New request — check server_id matches us
        if let Some(sid) = server_id {
            if sid != self.config.server_ip {
                edgerun_log::info!(
                    "edgerun-dhcp: REQUEST for server {} (we are {}) — ignoring",
                    sid,
                    self.config.server_ip
                );
                return Ok(());
            }
        }

        // Check the requested IP
        let ip = match requested_ip {
            Some(ip) => ip,
            None => {
                edgerun_log::warn!("edgerun-dhcp: REQUEST missing requested_ip");
                return self.send_nak(msg.xid).await;
            }
        };

        // Check if this IP is actually available for this client
        {
            let mut pool = self.pool.lock().await;
            if let Some(existing) = pool.find_lease_by_ip(ip) {
                if existing.mac != mac || existing.is_expired() {
                    let existing_mac = existing.mac;
                    pool.release(existing_mac);
                    match pool.allocate(mac, self.config.lease_time, msg.xid) {
                        Some(new_ip) if new_ip == ip => {}
                        Some(_) => {
                            drop(pool);
                            return self.send_nak(msg.xid).await;
                        }
                        None => {
                            drop(pool);
                            return self.send_nak(msg.xid).await;
                        }
                    }
                }
            } else {
                edgerun_log::warn!("edgerun-dhcp: REQUEST for {} not in our pool — NAK", ip);
                drop(pool);
                return self.send_nak(msg.xid).await;
            }
        }

        edgerun_log::info!("edgerun-dhcp: sending ACK {} to {}", ip, format_mac(mac));
        self.send_ack(msg, ip).await
    }

    async fn handle_release(&self, msg: &DhcpMessage) -> Result<(), io::Error> {
        let mac = msg.client_mac();
        edgerun_log::info!(
            "edgerun-dhcp: RELEASE from {} (xid=0x{:08x})",
            format_mac(mac),
            msg.xid
        );
        self.pool.lock().await.release(mac);
        Ok(())
    }

    async fn handle_decline(&self, msg: &DhcpMessage) -> Result<(), io::Error> {
        let mac = msg.client_mac();
        let requested = msg.options.requested_ip;
        edgerun_log::info!(
            "edgerun-dhcp: DECLINE from {} for {:?}",
            format_mac(mac),
            requested
        );
        if let Some(ip) = requested {
            self.pool.lock().await.release(mac);
            self.pool.lock().await.reserve(ip);
        }
        Ok(())
    }

    async fn handle_inform(&self, msg: &DhcpMessage) -> Result<(), io::Error> {
        let mac = msg.client_mac();
        edgerun_log::info!(
            "edgerun-dhcp: INFORM from {} (xid=0x{:08x})",
            format_mac(mac),
            msg.xid
        );

        let ack = DhcpMessage::ack(
            msg.xid,
            mac,
            msg.ciaddr,
            NetworkConfig {
                server_id: self.config.server_ip,
                subnet_mask: self.config.subnet_mask,
                router: self.config.router,
                dns_servers: self.config.dns_servers.clone(),
                lease_time: 0,
                tftp_server: None,
                bootfile: None,
            },
        );

        self.send_reply(&ack, msg).await
    }

    // --- Reply helpers ---

    async fn send_ack(&self, msg: &DhcpMessage, ip: Ipv4Addr) -> Result<(), io::Error> {
        let mac = msg.client_mac();
        let (tftp, bootfile) = self.pxe_boot_params(msg);
        let ack = DhcpMessage::ack(
            msg.xid,
            mac,
            ip,
            NetworkConfig {
                server_id: self.config.server_ip,
                subnet_mask: self.config.subnet_mask,
                router: self.config.router,
                dns_servers: self.config.dns_servers.clone(),
                lease_time: self.config.lease_time,
                tftp_server: tftp,
                bootfile,
            },
        );
        self.send_reply(&ack, msg).await
    }

    /// Determine PXE TFTP server and bootfile for this client.
    fn pxe_boot_params(&self, msg: &DhcpMessage) -> (Option<String>, Option<String>) {
        let tftp = self.config.tftp_server.map(|ip| ip.to_string());
        let bootfile = if let Some(arch) = msg.options.client_arch {
            let arch_key = arch.as_str().replace(' ', "-").to_lowercase();
            self.config
                .bootfile_by_arch
                .get(&arch_key)
                .cloned()
                .or_else(|| self.config.default_bootfile.clone())
                .or_else(|| Some(arch.default_bootfile().to_string()))
        } else {
            self.config.default_bootfile.clone()
        };
        (tftp, bootfile)
    }

    async fn send_nak(&self, xid: u32) -> Result<(), io::Error> {
        let nak = DhcpMessage::nak(xid, self.config.server_ip);
        let wire = nak.to_wire();
        let broadcast =
            SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::BROADCAST), DHCP_CLIENT_PORT);
        let _ = self.socket.send_to(&wire, broadcast).await;
        Ok(())
    }

    async fn send_reply(&self, msg: &DhcpMessage, original: &DhcpMessage) -> Result<(), io::Error> {
        let wire = msg.to_wire();

        if !original.ciaddr.is_unspecified() && !original.broadcast {
            let addr = SocketAddr::new(std::net::IpAddr::V4(original.ciaddr), DHCP_CLIENT_PORT);
            self.socket.send_to(&wire, addr).await?;
        } else {
            let broadcast =
                SocketAddr::new(std::net::IpAddr::V4(Ipv4Addr::BROADCAST), DHCP_CLIENT_PORT);
            self.socket.send_to(&wire, broadcast).await?;
        }
        Ok(())
    }

    /// Get current pool stats.
    pub async fn stats(&self) -> String {
        let pool = self.pool.lock().await;
        format!(
            "active={}, available={}, pool_size={}",
            pool.active_count(),
            pool.available_count(),
            pool.pool_size()
        )
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn network_broadcast(ip: Ipv4Addr, mask: Ipv4Addr) -> Ipv4Addr {
    let i = ip.octets();
    let m = mask.octets();
    Ipv4Addr::new(i[0] | !m[0], i[1] | !m[1], i[2] | !m[2], i[3] | !m[3])
}

fn format_mac(mac: [u8; 6]) -> String {
    edgerun_encoding::hex::format_mac(&mac)
}
