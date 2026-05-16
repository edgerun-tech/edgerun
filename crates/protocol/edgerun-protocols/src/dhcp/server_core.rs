//! DHCPv4 server protocol state without socket ownership.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::net::Ipv4Addr;
use edgerun_encoding::ip::broadcast_address;

use super::lease::LeasePool;
use super::message::{DHCP_CLIENT_PORT, DhcpMessage, DhcpMessageType, NetworkConfig};

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DhcpDatagram {
    pub dest: Ipv4Addr,
    pub port: u16,
    pub wire: Vec<u8>,
}

pub struct DhcpServerCore {
    pub config: DhcpServerConfig,
    pub pool: LeasePool,
}

impl DhcpServerCore {
    pub fn new(config: DhcpServerConfig, pool_start: Ipv4Addr, pool_end: Ipv4Addr) -> Self {
        let mut pool = LeasePool::new(pool_start, pool_end);
        pool.reserve(config.server_ip);
        pool.reserve(config.router);
        pool.reserve(broadcast_address(&config.server_ip, &config.subnet_mask));
        for dns in &config.dns_servers {
            pool.reserve(*dns);
        }
        Self { config, pool }
    }

    pub fn handle_wire(
        &mut self,
        wire: &[u8],
    ) -> Result<Option<DhcpDatagram>, super::message::DhcpError> {
        let msg = DhcpMessage::from_wire(wire)?;
        Ok(self.handle_message(&msg))
    }

    pub fn handle_message(&mut self, msg: &DhcpMessage) -> Option<DhcpDatagram> {
        match msg.options.message_type {
            Some(DhcpMessageType::Discover) => self.handle_discover(msg),
            Some(DhcpMessageType::Request) => self.handle_request(msg),
            Some(DhcpMessageType::Inform) => Some(self.handle_inform(msg)),
            Some(DhcpMessageType::Release) => {
                self.pool.release(msg.client_mac());
                None
            }
            Some(DhcpMessageType::Decline) => {
                let mac = msg.client_mac();
                if let Some(ip) = msg.options.requested_ip {
                    self.pool.release(mac);
                    self.pool.reserve(ip);
                }
                None
            }
            Some(DhcpMessageType::Offer | DhcpMessageType::Ack | DhcpMessageType::Nak) | None => {
                None
            }
        }
    }

    fn handle_discover(&mut self, msg: &DhcpMessage) -> Option<DhcpDatagram> {
        let mac = msg.client_mac();
        let Some(ip) = self.pool.allocate(mac, self.config.lease_time, msg.xid) else {
            return Some(self.nak(msg.xid));
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
        Some(self.reply(offer, msg))
    }

    fn handle_request(&mut self, msg: &DhcpMessage) -> Option<DhcpDatagram> {
        let mac = msg.client_mac();
        let requested_ip = msg.options.requested_ip;

        if !msg.ciaddr.is_unspecified() && requested_ip.is_none() {
            if let Some(lease) = self.pool.find_lease_by_ip(msg.ciaddr) {
                if lease.mac == mac && !lease.is_expired() {
                    return Some(self.ack(msg, msg.ciaddr));
                }
            }
            return Some(self.nak(msg.xid));
        }

        if let Some(sid) = msg.options.server_id {
            if sid != self.config.server_ip {
                return None;
            }
        }

        let Some(ip) = requested_ip else {
            return Some(self.nak(msg.xid));
        };

        if let Some(existing) = self.pool.find_lease_by_ip(ip) {
            if existing.mac != mac || existing.is_expired() {
                let existing_mac = existing.mac;
                self.pool.release(existing_mac);
                match self.pool.allocate(mac, self.config.lease_time, msg.xid) {
                    Some(new_ip) if new_ip == ip => {}
                    _ => return Some(self.nak(msg.xid)),
                }
            }
        } else {
            return Some(self.nak(msg.xid));
        }

        Some(self.ack(msg, ip))
    }

    fn handle_inform(&self, msg: &DhcpMessage) -> DhcpDatagram {
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
        self.reply(ack, msg)
    }

    fn ack(&self, msg: &DhcpMessage, ip: Ipv4Addr) -> DhcpDatagram {
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
        self.reply(ack, msg)
    }

    fn nak(&self, xid: u32) -> DhcpDatagram {
        self.datagram(
            DhcpMessage::nak(xid, self.config.server_ip),
            Ipv4Addr::BROADCAST,
        )
    }

    fn reply(&self, msg: DhcpMessage, original: &DhcpMessage) -> DhcpDatagram {
        let dest = if !original.ciaddr.is_unspecified() && !original.broadcast {
            original.ciaddr
        } else {
            Ipv4Addr::BROADCAST
        };
        self.datagram(msg, dest)
    }

    fn datagram(&self, msg: DhcpMessage, dest: Ipv4Addr) -> DhcpDatagram {
        DhcpDatagram {
            dest,
            port: DHCP_CLIENT_PORT,
            wire: msg.to_wire(),
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

    fn config() -> DhcpServerConfig {
        DhcpServerConfig {
            server_ip: Ipv4Addr::new(192, 168, 1, 1),
            subnet_mask: Ipv4Addr::new(255, 255, 255, 0),
            router: Ipv4Addr::new(192, 168, 1, 1),
            dns_servers: vec![Ipv4Addr::new(1, 1, 1, 1)],
            lease_time: 3600,
            tftp_server: Some(Ipv4Addr::new(192, 168, 1, 2)),
            default_bootfile: Some("boot.ipxe".into()),
            bootfile_by_arch: BTreeMap::new(),
        }
    }

    #[test]
    fn discover_produces_offer_without_socket() {
        let mut core = DhcpServerCore::new(
            config(),
            Ipv4Addr::new(192, 168, 1, 100),
            Ipv4Addr::new(192, 168, 1, 100),
        );
        let request = DhcpMessage::discover(7, [1, 2, 3, 4, 5, 6]).to_wire();

        let datagram = core.handle_wire(&request).unwrap().unwrap();
        assert_eq!(datagram.dest, Ipv4Addr::BROADCAST);
        assert_eq!(datagram.port, DHCP_CLIENT_PORT);

        let response = DhcpMessage::from_wire(&datagram.wire).unwrap();
        assert_eq!(response.options.message_type, Some(DhcpMessageType::Offer));
        assert_eq!(response.yiaddr, Ipv4Addr::new(192, 168, 1, 100));
    }

    #[test]
    fn request_for_wrong_server_is_ignored() {
        let mut core = DhcpServerCore::new(
            config(),
            Ipv4Addr::new(192, 168, 1, 100),
            Ipv4Addr::new(192, 168, 1, 100),
        );
        let request = DhcpMessage::request(
            8,
            [1, 2, 3, 4, 5, 6],
            Ipv4Addr::new(192, 168, 1, 100),
            Ipv4Addr::new(10, 0, 0, 1),
        );

        assert!(core.handle_message(&request).is_none());
    }
}
