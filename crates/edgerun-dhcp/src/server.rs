//! DHCPv4 server — no_std DISCOVER/REQUEST handling.

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::net::Ipv4Addr;
use core::time::Duration;

use edgerun_bare_rt::{sleep, CancellationToken, Mutex, SocketAddr, UdpSocket};

use super::lease::LeasePool;
use super::message::io;
use super::message::{
    DhcpMessage, DhcpMessageType, NetworkConfig, DHCP_CLIENT_PORT, DHCP_SERVER_PORT,
};

pub struct DhcpServerConfig {
    pub server_ip: Ipv4Addr,
    pub subnet_mask: Ipv4Addr,
    pub router: Ipv4Addr,
    pub dns_servers: Vec<Ipv4Addr>,
    pub lease_time: u32,
    pub tftp_server: Option<Ipv4Addr>,
    pub default_bootfile: Option<String>,
    pub bootfile_by_arch: BTreeMap<String, String>,
}

pub struct DhcpServer {
    socket: Arc<UdpSocket>,
    config: DhcpServerConfig,
    pool: Mutex<LeasePool>,
    interface: Option<String>,
}

impl DhcpServer {
    pub fn new(
        config: DhcpServerConfig,
        pool_start: Ipv4Addr,
        pool_end: Ipv4Addr,
    ) -> Result<Self, io::Error> {
        let mut socket = UdpSocket::new();
        socket
            .bind(SocketAddr::new(0, DHCP_SERVER_PORT))
            .map_err(map_udp_error)?;
        socket.set_nonblocking(true);

        let mut pool = LeasePool::new(pool_start, pool_end);
        pool.reserve(config.server_ip);
        pool.reserve(config.router);
        pool.reserve(network_broadcast(config.server_ip, config.subnet_mask));
        for dns in &config.dns_servers {
            pool.reserve(*dns);
        }

        Ok(Self {
            socket: Arc::new(socket),
            config,
            pool: Mutex::new(pool),
            interface: None,
        })
    }

    pub fn set_interface(&mut self, iface: String) {
        self.interface = Some(iface);
    }

    pub async fn run(&self, shutdown: CancellationToken) {
        while !shutdown.is_cancelled() {
            if self.tick().await.is_err() {
                sleep(Duration::from_millis(100)).await;
            }
        }
    }

    pub async fn tick(&self) -> Result<(), io::Error> {
        let mut buf = [0u8; 1500];
        let (n, src) = self.socket.recv_from(&mut buf).map_err(map_udp_error)?;
        let msg = match DhcpMessage::from_wire(&buf[..n]) {
            Ok(m) => m,
            Err(_) => return Ok(()),
        };
        self.handle_message(&msg, src).await
    }

    async fn handle_message(&self, msg: &DhcpMessage, src: SocketAddr) -> Result<(), io::Error> {
        let _ = src;
        match msg.options.message_type {
            Some(DhcpMessageType::Discover) => self.handle_discover(msg).await,
            Some(DhcpMessageType::Request) => self.handle_request(msg).await,
            Some(DhcpMessageType::Release) => self.handle_release(msg).await,
            Some(DhcpMessageType::Decline) => self.handle_decline(msg).await,
            Some(DhcpMessageType::Inform) => self.handle_inform(msg).await,
            Some(DhcpMessageType::Offer | DhcpMessageType::Ack | DhcpMessageType::Nak) | None => {
                Ok(())
            }
        }
    }

    async fn handle_discover(&self, msg: &DhcpMessage) -> Result<(), io::Error> {
        let mac = msg.client_mac();
        let ip = {
            let mut pool = self.pool.lock();
            pool.allocate(mac, self.config.lease_time, msg.xid)
        };
        let ip = match ip {
            Some(ip) => ip,
            None => return self.send_nak(msg.xid).await,
        };

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

        if !msg.ciaddr.is_unspecified() && requested_ip.is_none() {
            let pool = self.pool.lock();
            if let Some(lease) = pool.find_lease_by_ip(msg.ciaddr) {
                if lease.mac == mac && !lease.is_expired() {
                    drop(pool);
                    return self.send_ack(msg, msg.ciaddr).await;
                }
            }
            drop(pool);
            return self.send_nak(msg.xid).await;
        }

        if let Some(sid) = msg.options.server_id {
            if sid != self.config.server_ip {
                return Ok(());
            }
        }

        let ip = match requested_ip {
            Some(ip) => ip,
            None => return self.send_nak(msg.xid).await,
        };

        {
            let mut pool = self.pool.lock();
            if let Some(existing) = pool.find_lease_by_ip(ip) {
                if existing.mac != mac || existing.is_expired() {
                    let existing_mac = existing.mac;
                    pool.release(existing_mac);
                    match pool.allocate(mac, self.config.lease_time, msg.xid) {
                        Some(new_ip) if new_ip == ip => {}
                        _ => {
                            drop(pool);
                            return self.send_nak(msg.xid).await;
                        }
                    }
                }
            } else {
                drop(pool);
                return self.send_nak(msg.xid).await;
            }
        }

        self.send_ack(msg, ip).await
    }

    async fn handle_release(&self, msg: &DhcpMessage) -> Result<(), io::Error> {
        self.pool.lock().release(msg.client_mac());
        Ok(())
    }

    async fn handle_decline(&self, msg: &DhcpMessage) -> Result<(), io::Error> {
        let mac = msg.client_mac();
        if let Some(ip) = msg.options.requested_ip {
            let mut pool = self.pool.lock();
            pool.release(mac);
            pool.reserve(ip);
        }
        Ok(())
    }

    async fn handle_inform(&self, msg: &DhcpMessage) -> Result<(), io::Error> {
        let ack = DhcpMessage::ack(
            msg.xid,
            msg.client_mac(),
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

    async fn send_ack(&self, msg: &DhcpMessage, ip: Ipv4Addr) -> Result<(), io::Error> {
        let (tftp, bootfile) = self.pxe_boot_params(msg);
        let ack = DhcpMessage::ack(
            msg.xid,
            msg.client_mac(),
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
        self.send_to(&nak, Ipv4Addr::BROADCAST, DHCP_CLIENT_PORT)
    }

    async fn send_reply(&self, msg: &DhcpMessage, original: &DhcpMessage) -> Result<(), io::Error> {
        let dest = if !original.ciaddr.is_unspecified() && !original.broadcast {
            original.ciaddr
        } else {
            Ipv4Addr::BROADCAST
        };
        self.send_to(msg, dest, DHCP_CLIENT_PORT)
    }

    fn send_to(&self, msg: &DhcpMessage, dest: Ipv4Addr, port: u16) -> Result<(), io::Error> {
        let wire = msg.to_wire();
        let addr = SocketAddr::new(u32::from_be_bytes(dest.octets()), port);
        self.socket.send_to(&wire, addr).map(|_| ()).map_err(map_udp_error)
    }

    pub async fn stats(&self) -> String {
        let pool = self.pool.lock();
        format!(
            "active={}, available={}, pool_size={}",
            pool.active_count(),
            pool.available_count(),
            pool.pool_size()
        )
    }
}

fn network_broadcast(ip: Ipv4Addr, mask: Ipv4Addr) -> Ipv4Addr {
    let i = ip.octets();
    let m = mask.octets();
    Ipv4Addr::new(i[0] | !m[0], i[1] | !m[1], i[2] | !m[2], i[3] | !m[3])
}

fn map_udp_error(_: edgerun_bare_rt::UdpError) -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "UDP operation not ready")
}
