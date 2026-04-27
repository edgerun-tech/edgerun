//! DHCPv4 client — no_std DORA state machine.

use alloc::string::{String, ToString};
use core::net::Ipv4Addr;
use core::time::Duration;

use edgerun_bare_rt::{Instant, SocketAddr, UdpSocket};

use super::lease::Lease;
use super::message::io;
use super::message::{DhcpMessage, DhcpMessageType, DHCP_CLIENT_PORT, DHCP_SERVER_PORT};

pub struct DhcpClient {
    socket: UdpSocket,
    interface: String,
    mac: [u8; 6],
    xid: u32,
    pub current_lease: Option<Lease>,
    pub server_id: Option<Ipv4Addr>,
    pub offered_ip: Option<Ipv4Addr>,
}

impl DhcpClient {
    /// Create a client for an interface. Bare builds cannot read sysfs, so
    /// callers should prefer `new_with_mac` when a hardware MAC is available.
    pub fn new(interface: &str) -> Result<Self, io::Error> {
        Self::new_with_mac(interface, [0; 6])
    }

    pub fn new_with_mac(interface: &str, mac: [u8; 6]) -> Result<Self, io::Error> {
        let mut socket = UdpSocket::new();
        socket
            .bind(SocketAddr::new(0, DHCP_CLIENT_PORT))
            .map_err(map_udp_error)?;
        socket.set_nonblocking(true);

        Ok(Self {
            socket,
            interface: interface.to_string(),
            mac,
            xid: random_xid(),
            current_lease: None,
            server_id: None,
            offered_ip: None,
        })
    }

    pub fn acquire_lease(&mut self, timeout: Duration) -> Result<Lease, io::Error> {
        let deadline = Instant::now() + timeout;
        self.send_discover()?;
        let (offered_ip, server_id) = self.wait_for_offer(&deadline)?;
        self.send_request(offered_ip, server_id)?;
        let lease = self.wait_for_ack(&deadline)?;
        self.current_lease = Some(lease.clone());
        Ok(lease)
    }

    pub fn renew_lease(&mut self) -> Result<Lease, io::Error> {
        let lease = self
            .current_lease
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No active lease to renew"))?;
        let server_id = self.server_id.ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "No server ID known for renewal")
        })?;

        let msg = DhcpMessage::request(self.xid, self.mac, lease.ip, server_id);
        self.send_message(&msg, server_id)?;

        let deadline = Instant::now() + Duration::from_secs(10);
        let new_lease = self.wait_for_ack(&deadline)?;
        self.current_lease = Some(new_lease.clone());
        Ok(new_lease)
    }

    pub fn release_lease(&mut self) -> Result<(), io::Error> {
        let lease = self
            .current_lease
            .take()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No active lease to release"))?;
        let server_id = self
            .server_id
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No server ID known"))?;
        let msg = DhcpMessage::release(self.xid, self.mac, lease.ip, server_id);
        self.send_message(&msg, server_id)
    }

    fn send_discover(&self) -> Result<(), io::Error> {
        let msg = DhcpMessage::discover(self.xid, self.mac);
        self.send_to(&msg, Ipv4Addr::BROADCAST, DHCP_SERVER_PORT)
    }

    fn send_request(&self, ip: Ipv4Addr, server_id: Ipv4Addr) -> Result<(), io::Error> {
        let msg = DhcpMessage::request(self.xid, self.mac, ip, server_id);
        self.send_to(&msg, Ipv4Addr::BROADCAST, DHCP_SERVER_PORT)
    }

    fn send_message(&self, msg: &DhcpMessage, dest: Ipv4Addr) -> Result<(), io::Error> {
        self.send_to(msg, dest, DHCP_SERVER_PORT)
    }

    fn send_to(&self, msg: &DhcpMessage, dest: Ipv4Addr, port: u16) -> Result<(), io::Error> {
        let wire = msg.to_wire();
        let addr = SocketAddr::new(u32::from_be_bytes(dest.octets()), port);
        self.socket
            .send_to(&wire, addr)
            .map(|_| ())
            .map_err(map_udp_error)
    }

    fn wait_for_offer(&mut self, deadline: &Instant) -> Result<(Ipv4Addr, Ipv4Addr), io::Error> {
        while Instant::now() < *deadline {
            match self.recv_message() {
                Ok((msg, _src)) if msg.xid == self.xid => {
                    if msg.options.message_type == Some(DhcpMessageType::Offer) {
                        let yiaddr = msg.yiaddr;
                        let server_id = msg.options.server_id.ok_or_else(|| {
                            io::Error::new(io::ErrorKind::InvalidData, "OFFER missing server ID")
                        })?;
                        self.offered_ip = Some(yiaddr);
                        self.server_id = Some(server_id);
                        return Ok((yiaddr, server_id));
                    }
                }
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(_) => {}
            }
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "No DHCP OFFER received",
        ))
    }

    fn wait_for_ack(&mut self, deadline: &Instant) -> Result<Lease, io::Error> {
        while Instant::now() < *deadline {
            match self.recv_message() {
                Ok((msg, _src)) if msg.xid == self.xid => match msg.options.message_type {
                    Some(DhcpMessageType::Ack) => {
                        let lease = Lease::new(
                            self.mac,
                            msg.yiaddr,
                            msg.options.lease_time.unwrap_or(3600),
                            self.xid,
                        );
                        self.server_id = msg.options.server_id;
                        return Ok(lease);
                    }
                    Some(DhcpMessageType::Nak) => {
                        return Err(io::Error::new(
                            io::ErrorKind::ConnectionRefused,
                            "DHCP server sent NAK",
                        ));
                    }
                    _ => {}
                },
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {}
                Err(_) => {}
            }
        }
        Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "No DHCP ACK/NAK received",
        ))
    }

    fn recv_message(&self) -> Result<(DhcpMessage, Ipv4Addr), io::Error> {
        let mut buf = [0u8; 1500];
        let (n, src) = self.socket.recv_from(&mut buf).map_err(map_udp_error)?;
        let msg = DhcpMessage::from_wire(&buf[..n])?;
        Ok((msg, Ipv4Addr::from(src.ip_bytes())))
    }

    pub fn mac(&self) -> [u8; 6] {
        self.mac
    }

    pub fn xid(&self) -> u32 {
        self.xid
    }

    pub fn interface(&self) -> &str {
        &self.interface
    }
}

fn map_udp_error(_: edgerun_bare_rt::UdpError) -> io::Error {
    io::Error::new(io::ErrorKind::WouldBlock, "UDP operation not ready")
}

fn random_xid() -> u32 {
    edgerun_bare_rt::now() as u32 ^ 0x9e37_79b9
}
