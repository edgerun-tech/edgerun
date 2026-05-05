//! DHCPv6 client — RFC 8415.

use crate::std::io;
use crate::std::net::{IpAddr, Ipv6Addr, SocketAddr, UdpSocket};
use crate::std::prelude::v1::*;
use crate::std::time::{Duration, Instant};

use super::duid::Duid;
use super::lease::Dhcpv6Lease;
use super::message::{
    Dhcpv6Message, Dhcpv6MsgType, TransactionId, ALL_DHCP_RELAY_AND_SERVERS, DHCPV6_CLIENT_PORT,
    DHCPV6_SERVER_PORT,
};
use super::options::{
    Dhcpv6Option, StatusCode, OPT_DNS_SERVERS, OPT_DOMAIN_LIST, OPT_IAADDR, OPT_IA_NA,
    OPT_RAPID_COMMIT, OPT_SERVERID,
};

/// DHCPv6 client state machine.
pub struct Dhcpv6Client {
    socket: UdpSocket,
    client_duid: Vec<u8>,
    iaid: u32,
    /// Current lease if we have one.
    pub lease: Option<Dhcpv6Lease>,
    /// Server DUID from the last reply.
    pub server_duid: Option<Vec<u8>>,
    /// DNS servers received.
    pub dns_servers: Vec<Ipv6Addr>,
    /// Domain search received.
    pub domain_search: Option<Vec<u8>>,
    /// Transaction timeout.
    timeout: Duration,
}

impl Dhcpv6Client {
    /// Create a new DHCPv6 client with the given DUID and IAID.
    pub fn new(client_duid: Vec<u8>, iaid: u32) -> Result<Self, io::Error> {
        let socket = UdpSocket::bind(("::", DHCPV6_CLIENT_PORT))?;
        socket.set_read_timeout(Some(Duration::from_secs(2)))?;

        Ok(Self {
            socket,
            client_duid,
            iaid,
            lease: None,
            server_duid: None,
            dns_servers: Vec::new(),
            domain_search: None,
            timeout: Duration::from_secs(10),
        })
    }

    /// Create a client with an auto-generated DUID-LLT from a MAC address.
    pub fn with_mac(mac: [u8; 6], iaid: u32) -> Result<Self, io::Error> {
        let duid = Duid::generate_llt(mac).to_wire();
        Self::new(duid, iaid)
    }

    /// Set the transaction timeout.
    pub fn set_timeout(&mut self, t: Duration) {
        self.timeout = t;
    }

    /// Run the full Solicit/Request exchange and obtain an address.
    /// Returns the assigned IPv6 address and the server's DUID.
    pub fn acquire_address(&mut self) -> Result<(Ipv6Addr, Vec<u8>), io::Error> {
        let deadline = Instant::now() + self.timeout;

        // Step 1: SOLICIT
        let solicit = Dhcpv6Message::solicit(self.iaid, &self.client_duid, 0);
        let xid = solicit.transaction_id;
        self.send_multicast(&solicit)?;

        // Step 2: Wait for ADVERTISE
        let advertise = match self.wait_for(&[Dhcpv6MsgType::Advertise], xid, &deadline) {
            Some(msg) => msg,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "No Advertise received",
                ));
            }
        };

        // Extract server DUID
        self.server_duid = advertise.server_duid();

        // Check for rapid commit (server sent Reply instead of Advertise)
        let rapid = advertise.options.iter().any(|o| o.code == OPT_RAPID_COMMIT);
        if rapid && advertise.msg_type == Dhcpv6MsgType::Reply {
            // Rapid commit — process Reply directly
            return self.process_reply(&advertise);
        }

        // Step 3: REQUEST
        let server_duid = self.server_duid.clone().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "No server DUID in Advertise")
        })?;

        let request = Dhcpv6Message::request(self.iaid, &self.client_duid, &server_duid);
        let xid = request.transaction_id;
        self.send_multicast(&request)?;

        // Step 4: Wait for REPLY
        let reply = match self.wait_for(&[Dhcpv6MsgType::Reply], xid, &deadline) {
            Some(msg) => msg,
            None => return Err(io::Error::new(io::ErrorKind::TimedOut, "No Reply received")),
        };

        self.process_reply(&reply)
    }

    /// Renew the current lease.
    pub fn renew(&mut self) -> Result<Dhcpv6Lease, io::Error> {
        let lease = self
            .lease
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No active lease"))?;

        let server_duid = self
            .server_duid
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No server DUID"))?;

        let renew = Dhcpv6Message::renew(self.iaid, &self.client_duid, server_duid);
        let xid = renew.transaction_id;
        self.send_multicast(&renew)?;

        let deadline = Instant::now() + self.timeout;
        let reply = match self.wait_for(&[Dhcpv6MsgType::Reply], xid, &deadline) {
            Some(msg) => msg,
            None => return Err(io::Error::new(io::ErrorKind::TimedOut, "No Reply to Renew")),
        };

        let (_addr, _duid) = self.process_reply(&reply)?;
        self.lease
            .clone()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No lease after renew"))
    }

    /// Release the current lease.
    pub fn release(&mut self) -> Result<(), io::Error> {
        let lease = self
            .lease
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No active lease"))?;

        let server_duid = self
            .server_duid
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No server DUID"))?;

        let release = Dhcpv6Message::release(self.iaid, &self.client_duid, server_duid);
        self.send_multicast(&release)?;

        self.lease = None;
        Ok(())
    }

    /// Request options only (stateless — no address assignment).
    pub fn request_options(&mut self) -> Result<(), io::Error> {
        let info_req = Dhcpv6Message::information_request(&self.client_duid);
        let xid = info_req.transaction_id;
        self.send_multicast(&info_req)?;

        let deadline = Instant::now() + self.timeout;
        let reply = match self.wait_for(&[Dhcpv6MsgType::Reply], xid, &deadline) {
            Some(msg) => msg,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "No Reply to Information-Request",
                ));
            }
        };

        self.extract_options(&reply);
        self.server_duid = reply.server_duid();
        Ok(())
    }

    // --- Internal ---

    fn send_multicast(&self, msg: &Dhcpv6Message) -> Result<(), io::Error> {
        let wire = msg.to_wire();
        // Send to All_DHCP_Relay_Agents_and_Servers (ff02::1:2)
        let ip: Ipv6Addr = ALL_DHCP_RELAY_AND_SERVERS
            .parse()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let dest = SocketAddr::new(IpAddr::V6(ip), DHCPV6_SERVER_PORT);
        self.socket.send_to(&wire, dest)?;
        Ok(())
    }

    fn wait_for(
        &self,
        expected_types: &[Dhcpv6MsgType],
        xid: TransactionId,
        deadline: &Instant,
    ) -> Option<Dhcpv6Message> {
        while Instant::now() < *deadline {
            let mut buf = [0u8; 1500];
            if let Ok((n, _)) = self.socket.recv_from(&mut buf) {
                if let Ok(msg) = Dhcpv6Message::from_wire(&buf[..n]) {
                    if msg.transaction_id == xid && expected_types.contains(&msg.msg_type) {
                        return Some(msg);
                    }
                }
            }
        }
        None
    }

    fn process_reply(&mut self, reply: &Dhcpv6Message) -> Result<(Ipv6Addr, Vec<u8>), io::Error> {
        // Extract IAADDR from reply
        for ia_na in reply.ia_na_options() {
            let subs = ia_na
                .sub_options()
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
            for sub in subs {
                if sub.code == OPT_IAADDR {
                    if let Some(addr) = sub.address() {
                        let preferred = sub.preferred_lifetime().unwrap_or(3600);
                        let valid = sub.valid_lifetime().unwrap_or(7200);

                        self.lease = Some(Dhcpv6Lease::new(
                            self.client_duid.clone(),
                            self.iaid,
                            addr,
                            preferred,
                            valid,
                        ));
                        self.extract_options(reply);
                        self.server_duid = reply.server_duid();
                        return Ok((addr, self.server_duid.clone().unwrap_or_default()));
                    }
                }
            }
        }

        // Check for status code
        for opt in &reply.options {
            if let Some((status, msg)) = opt.status() {
                if status != StatusCode::Success {
                    return Err(io::Error::other(format!(
                        "DHCPv6 error: {} ({})",
                        status.as_str(),
                        msg
                    )));
                }
            }
        }

        Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "No address in Reply",
        ))
    }

    fn extract_options(&mut self, msg: &Dhcpv6Message) {
        self.dns_servers.clear();
        for opt in &msg.options {
            match opt.code {
                OPT_DNS_SERVERS => {
                    for chunk in opt.data.chunks(16) {
                        if chunk.len() == 16 {
                            let octets: [u8; 16] = chunk.try_into().unwrap();
                            self.dns_servers.push(Ipv6Addr::from(octets));
                        }
                    }
                }
                OPT_DOMAIN_LIST => {
                    self.domain_search = Some(opt.data.clone());
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::std::net::Ipv6Addr;

    #[test]
    fn test_client_creation() {
        let mac = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        let duid = Duid::generate_llt(mac).to_wire();
        // DUID-LLT: type(2) + hw_type(2) + time(4) + mac(6) = 14
        assert_eq!(duid.len(), 14);
        // DUID-LL: type(2) + hw_type(2) + mac(6) = 10
        assert_eq!(Duid::generate_ll(mac).to_wire().len(), 10);
    }

    #[test]
    fn test_client_with_custom_duid() {
        use crate::DuidType;
        let duid = vec![0, 3, 0, 1, 0xaa, 0xbb, 0xcc, 0xdd];
        let parsed = Duid::from_wire(&duid).unwrap();
        assert_eq!(parsed.duid_type, DuidType::Ll);
    }
}
