//! DHCPv4 server — handles DISCOVER/REQUEST and responds with OFFER/ACK/NAK.

use std::io;
use std::net::{Ipv4Addr, SocketAddr, UdpSocket};
use std::time::Duration;

use super::message::{DhcpMessage, DhcpMessageType, DHCP_CLIENT_PORT, DHCP_SERVER_PORT};
use super::lease::LeasePool;

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
/// # Example
/// ```ignore
/// use edgerun_dns::dhcp::server::{DhcpServer, DhcpServerConfig};
/// use std::net::Ipv4Addr;
/// use std::time::Duration;
///
/// let config = DhcpServerConfig {
///     server_ip: Ipv4Addr::new(192, 168, 1, 1),
///     subnet_mask: Ipv4Addr::new(255, 255, 255, 0),
///     router: Ipv4Addr::new(192, 168, 1, 1),
///     dns_servers: vec![Ipv4Addr::new(8, 8, 8, 8)],
///     lease_time: 86400,
///     tftp_server: Some(Ipv4Addr::new(192, 168, 1, 1)),
///     default_bootfile: Some("pxelinux.0".to_string()),
///     bootfile_by_arch: std::collections::HashMap::new(),
/// };
///
/// let mut server = DhcpServer::new(config,
///     Ipv4Addr::new(192, 168, 1, 100),
///     Ipv4Addr::new(192, 168, 1, 200),
/// ).unwrap();
///
/// // Run the server event loop
/// server.run();
/// ```
pub struct DhcpServer {
    socket: UdpSocket,
    config: DhcpServerConfig,
    pool: LeasePool,
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
        let socket = UdpSocket::bind(("0.0.0.0", DHCP_SERVER_PORT))?;
        socket.set_broadcast(true)?;
        socket.set_read_timeout(Some(Duration::from_millis(200)))?;
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            let fd = socket.as_raw_fd();
            let opt: libc::c_int = 1;
            unsafe {
                libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_REUSEADDR,
                    &opt as *const _ as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t);
            }
        }

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
            pool,
            interface: None,
        })
    }

    /// Set the interface name (for logging / diagnostics).
    pub fn set_interface(&mut self, iface: String) {
        self.interface = Some(iface);
    }

    /// Run the server event loop (blocking). Runs until the socket errors.
    pub fn run(&mut self) -> Result<(), io::Error> {
        eprintln!(
            "edgerun-dhcp: server listening on 0.0.0.0:{}",
            DHCP_SERVER_PORT
        );
        eprintln!(
            "  pool: {} - {}",
            self.pool.pool_start, self.pool.pool_end
        );
        eprintln!(
            "  server: {}, router: {}, dns: {:?}",
            self.config.server_ip, self.config.router, self.config.dns_servers
        );

        loop {
            match self.tick() {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) if e.kind() == io::ErrorKind::TimedOut => continue,
                Err(e) => {
                    eprintln!("edgerun-dhcp: server error: {}", e);
                    return Err(e);
                }
            }
        }
    }

    /// Process one incoming packet. Call this from your own event loop.
    pub fn tick(&mut self) -> Result<(), io::Error> {
        let mut buf = [0u8; 1500];
        let (n, src) = self.socket.recv_from(&mut buf)?;

        let msg = match DhcpMessage::from_wire(&buf[..n]) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("edgerun-dhcp: failed to parse message from {}: {}", src, e);
                return Ok(());
            }
        };

        self.handle_message(&msg, src)?;
        Ok(())
    }

    fn handle_message(&mut self, msg: &DhcpMessage, src: SocketAddr) -> Result<(), io::Error> {
        let mt = match msg.options.message_type {
            Some(mt) => mt,
            None => {
                eprintln!("edgerun-dhcp: message from {} has no message type", src);
                return Ok(());
            }
        };

        match mt {
            DhcpMessageType::Discover => self.handle_discover(msg),
            DhcpMessageType::Request => self.handle_request(msg),
            DhcpMessageType::Release => self.handle_release(msg),
            DhcpMessageType::Decline => self.handle_decline(msg),
            DhcpMessageType::Inform => self.handle_inform(msg),
            DhcpMessageType::Offer | DhcpMessageType::Ack | DhcpMessageType::Nak => {
                // Server doesn't process these
                Ok(())
            }
        }
    }

    fn handle_discover(&mut self, msg: &DhcpMessage) -> Result<(), io::Error> {
        let mac = msg.client_mac();
        eprintln!(
            "edgerun-dhcp: DISCOVER from {} (xid=0x{:08x}) bootp={} rapid={}",
            format_mac(mac),
            msg.xid,
            msg.is_bootp,
            msg.options.param_request_list.contains(&super::message::OPT_RAPID_COMMIT)
        );

        // Check for rapid commit (RFC 4039) — 2-message DORA
        let rapid_commit = msg.options.param_request_list.contains(&super::message::OPT_RAPID_COMMIT);

        // Try to allocate an IP
        let ip = match self.pool.allocate(
            mac,
            msg.options.client_id.clone(),
            self.config.lease_time,
            msg.xid,
        ) {
            Some(ip) => ip,
            None => {
                eprintln!("edgerun-dhcp: pool exhausted, sending NAK");
                return self.send_nak(msg.xid, mac);
            }
        };

        let (tftp, bootfile) = self.pxe_boot_params(msg);

        let mut offer = DhcpMessage::offer(
            msg.xid,
            mac,
            ip,
            self.config.server_ip,
            self.config.subnet_mask,
            self.config.router,
            self.config.dns_servers.clone(),
            self.config.lease_time,
            tftp.clone(),
            bootfile.clone(),
        );

        // BOOTP compatibility: populate sname/file/siaddr fields (RFC 951 §4)
        if msg.is_bootp {
            offer.is_bootp = true;
            // siaddr = TFTP server IP
            if let Some(tftp_ip) = self.config.tftp_server {
                offer.siaddr = tftp_ip;
            }
            // sname = server hostname
            if let Some(ref bootfile) = bootfile {
                // Put bootfile name in the file field (RFC 951)
                let file_bytes = bootfile.as_bytes();
                offer.file[..file_bytes.len().min(128)].copy_from_slice(&file_bytes[..file_bytes.len().min(128)]);
            }
        }

        // Rapid commit: respond with ACK instead of OFFER (RFC 4039)
        if rapid_commit {
            offer.options.param_request_list.push(super::message::OPT_RAPID_COMMIT);
            edgerun_log::info!("edgerun-dhcp: sending RAPID-COMMIT ACK {} to {}",
                ip, format_mac(mac));
        } else {
            eprintln!(
                "edgerun-dhcp: sending OFFER {} to {}",
                ip,
                format_mac(mac)
            );
        }

        self.send_reply(&offer, msg)
    }

    fn handle_request(&mut self, msg: &DhcpMessage) -> Result<(), io::Error> {
        let mac = msg.client_mac();
        let requested_ip = msg.options.requested_ip;
        let server_id = msg.options.server_id;

        eprintln!(
            "edgerun-dhcp: REQUEST from {} (xid=0x{:08x}) req={:?} server={:?}",
            format_mac(mac),
            msg.xid,
            requested_ip,
            server_id
        );

        // If client is renewing (ciaddr is set, no requested_ip)
        if !msg.ciaddr.is_unspecified() && requested_ip.is_none() {
            // Renewal — check lease
            if let Some(lease) = self.pool.find_lease_by_ip(msg.ciaddr) {
                if lease.mac == mac && !lease.is_expired() {
                    // Renew it
                    eprintln!(
                        "edgerun-dhcp: renewing {} for {}",
                        msg.ciaddr,
                        format_mac(mac)
                    );
                    return self.send_ack(msg, msg.ciaddr);
                }
            }
            eprintln!("edgerun-dhcp: renewal denied for {}", msg.ciaddr);
            return self.send_nak(msg.xid, mac);
        }

        // New request — check server_id matches us
        if let Some(sid) = server_id {
            if sid != self.config.server_ip {
                // Client is requesting another server — ignore
                eprintln!(
                    "edgerun-dhcp: REQUEST for server {} (we are {}) — ignoring",
                    sid, self.config.server_ip
                );
                return Ok(());
            }
        }

        // Check the requested IP
        let ip = match requested_ip {
            Some(ip) => ip,
            None => {
                eprintln!("edgerun-dhcp: REQUEST missing requested_ip");
                return self.send_nak(msg.xid, mac);
            }
        };

        // Check if this IP is actually available for this client
        if let Some(existing) = self.pool.find_lease_by_ip(ip) {
            if existing.mac != mac || existing.is_expired() {
                // IP is assigned to someone else — release old, allocate new
                self.pool.release(existing.mac);
                // Try re-allocate
                match self.pool.allocate(mac, msg.options.client_id.clone(), self.config.lease_time, msg.xid) {
                    Some(new_ip) if new_ip == ip => {
                        // Good, re-allocated same IP
                    }
                    Some(_) => {
                        // Got a different IP — shouldn't happen after release
                        return self.send_nak(msg.xid, mac);
                    }
                    None => {
                        return self.send_nak(msg.xid, mac);
                    }
                }
            }
            // Same MAC, same IP — just ACK
        } else {
            // Not in pool at all — maybe client is requesting an IP we never offered
            eprintln!(
                "edgerun-dhcp: REQUEST for {} not in our pool — NAK",
                ip
            );
            return self.send_nak(msg.xid, mac);
        }

        eprintln!(
            "edgerun-dhcp: sending ACK {} to {}",
            ip,
            format_mac(mac)
        );
        self.send_ack(msg, ip)
    }

    fn handle_release(&mut self, msg: &DhcpMessage) -> Result<(), io::Error> {
        let mac = msg.client_mac();
        eprintln!(
            "edgerun-dhcp: RELEASE from {} (xid=0x{:08x})",
            format_mac(mac),
            msg.xid
        );
        self.pool.release(mac);
        Ok(())
    }

    fn handle_decline(&mut self, msg: &DhcpMessage) -> Result<(), io::Error> {
        let mac = msg.client_mac();
        let requested = msg.options.requested_ip;
        eprintln!(
            "edgerun-dhcp: DECLINE from {} for {:?}",
            format_mac(mac),
            requested
        );
        if let Some(ip) = requested {
            // Record conflict — this IP is in use by another host (RFC 2131 §2.2)
            self.pool.record_conflict(ip, mac);
        }
        Ok(())
    }

    fn handle_inform(&mut self, msg: &DhcpMessage) -> Result<(), io::Error> {
        // INFORM: client has IP already, wants config info
        // Per RFC 2131 §4.3.5, server MUST NOT include lease_time in INFORM response
        let mac = msg.client_mac();
        eprintln!(
            "edgerun-dhcp: INFORM from {} (xid=0x{:08x})",
            format_mac(mac),
            msg.xid
        );

        let mut ack = DhcpMessage::ack(
            msg.xid,
            mac,
            msg.ciaddr,
            self.config.server_ip,
            self.config.subnet_mask,
            self.config.router,
            self.config.dns_servers.clone(),
            0, // Placeholder — will be omitted
            None, // No PXE for INFORM
            None,
        );
        // Remove lease_time from INFORM response per RFC 2131 §4.3.5
        ack.options.lease_time = None;
        ack.options.renewal_time = None;
        ack.options.rebind_time = None;

        self.send_reply(&ack, msg)
    }

    // --- Reply helpers ---

    fn send_ack(&mut self, msg: &DhcpMessage, ip: Ipv4Addr) -> Result<(), io::Error> {
        let mac = msg.client_mac();
        let (tftp, bootfile) = self.pxe_boot_params(msg);
        let mut ack = DhcpMessage::ack(
            msg.xid,
            mac,
            ip,
            self.config.server_ip,
            self.config.subnet_mask,
            self.config.router,
            self.config.dns_servers.clone(),
            self.config.lease_time,
            tftp.clone(),
            bootfile.clone(),
        );

        // BOOTP compatibility
        if msg.is_bootp {
            ack.is_bootp = true;
            if let Some(tftp_ip) = self.config.tftp_server {
                ack.siaddr = tftp_ip;
            }
            if let Some(ref bf) = bootfile {
                let file_bytes = bf.as_bytes();
                ack.file[..file_bytes.len().min(128)].copy_from_slice(&file_bytes[..file_bytes.len().min(128)]);
            }
        }

        self.send_reply(&ack, msg)
    }

    /// Determine PXE TFTP server and bootfile for this client.
    fn pxe_boot_params(&self, msg: &DhcpMessage) -> (Option<String>, Option<String>) {
        let tftp = self.config.tftp_server.map(|ip| ip.to_string());
        let bootfile = if let Some(arch) = msg.options.client_arch {
            // Try arch-specific bootfile, then default
            let arch_key = arch.as_str().replace(' ', "-").to_lowercase();
            self.config
                .bootfile_by_arch
                .get(&arch_key)
                .cloned()
                .or_else(|| self.config.default_bootfile.clone())
                .or_else(|| {
                    // Fallback: use the architecture's default
                    Some(arch.default_bootfile().to_string())
                })
        } else {
            self.config.default_bootfile.clone()
        };
        (tftp, bootfile)
    }

    fn send_nak(&mut self, xid: u32, client_mac: [u8; 6]) -> Result<(), io::Error> {
        let nak = DhcpMessage::nak(xid, self.config.server_ip, client_mac);
        let wire = nak.to_wire();
        let broadcast = SocketAddr::new(
            std::net::IpAddr::V4(Ipv4Addr::BROADCAST),
            DHCP_CLIENT_PORT,
        );
        let _ = self.socket.send_to(&wire, broadcast);
        Ok(())
    }

    fn send_reply(&mut self, msg: &DhcpMessage, original: &DhcpMessage) -> Result<(), io::Error> {
        let wire = msg.to_wire();

        // RFC 2131 §4.1: If relay agent (giaddr) is set, unicast to it
        if !original.giaddr.is_unspecified() {
            let addr = SocketAddr::new(
                std::net::IpAddr::V4(original.giaddr),
                DHCP_SERVER_PORT,
            );
            self.socket.send_to(&wire, addr)?;
        } else if !original.ciaddr.is_unspecified() && !original.broadcast {
            let addr = SocketAddr::new(
                std::net::IpAddr::V4(original.ciaddr),
                DHCP_CLIENT_PORT,
            );
            self.socket.send_to(&wire, addr)?;
        } else {
            // Broadcast to client
            let broadcast = SocketAddr::new(
                std::net::IpAddr::V4(Ipv4Addr::BROADCAST),
                DHCP_CLIENT_PORT,
            );
            self.socket.send_to(&wire, broadcast)?;
        }
        Ok(())
    }

    /// Get current pool stats.
    pub fn stats(&self) -> String {
        format!(
            "active={}, available={}, pool_size={}",
            self.pool.active_count(),
            self.pool.available_count(),
            self.pool.pool_size()
        )
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn network_broadcast(ip: Ipv4Addr, mask: Ipv4Addr) -> Ipv4Addr {
    let i = ip.octets();
    let m = mask.octets();
    Ipv4Addr::new(
        i[0] | !m[0],
        i[1] | !m[1],
        i[2] | !m[2],
        i[3] | !m[3],
    )
}

fn format_mac(mac: [u8; 6]) -> String {
    edgerun_encoding::hex::format_mac(&mac)
}
