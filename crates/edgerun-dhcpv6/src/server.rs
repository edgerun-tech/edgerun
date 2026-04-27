//! DHCPv6 server — RFC 8415.

use crate::std::io;
use crate::std::net::{Ipv6Addr, SocketAddr, UdpSocket};
use crate::std::prelude::v1::*;
use crate::std::time::Duration;

use super::duid::{default_server_duid, Duid};
use super::lease::{LeasePool, LeaseState};
use super::message::{
    Dhcpv6Message, Dhcpv6MsgType, TransactionId, DHCPV6_CLIENT_PORT, DHCPV6_SERVER_PORT,
};
use super::options::{
    Dhcpv6Option, StatusCode, OPT_CLIENTID, OPT_DNS_SERVERS, OPT_DOMAIN_LIST, OPT_ELAPSED_TIME,
    OPT_IAADDR, OPT_IAPREFIX, OPT_IA_NA, OPT_IA_PD, OPT_ORO, OPT_RAPID_COMMIT, OPT_SERVERID,
    OPT_STATUS_CODE,
};

/// DHCPv6 server configuration.
pub struct Dhcpv6ServerConfig {
    /// The server's DUID.
    pub server_duid: Vec<u8>,
    /// DNS servers to advertise.
    pub dns_servers: Vec<Ipv6Addr>,
    /// DNS domain names to advertise.
    pub domain_list: Vec<String>,
    /// Default preferred lifetime for addresses (seconds).
    pub preferred_lifetime: u32,
    /// Default valid lifetime for addresses (seconds).
    pub valid_lifetime: u32,
    /// Default preferred lifetime for prefixes.
    pub pd_preferred_lifetime: u32,
    /// Default valid lifetime for prefixes.
    pub pd_valid_lifetime: u32,
    /// Default T1 (renewal time) — 0 = server chooses.
    pub t1: u32,
    /// Default T2 (rebinding time) — 0 = server chooses.
    pub t2: u32,
}

impl Default for Dhcpv6ServerConfig {
    fn default() -> Self {
        Self {
            server_duid: default_server_duid().to_wire(),
            dns_servers: vec![
                Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888),
                Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8844),
            ],
            domain_list: vec!["edgerun.tech".to_string()],
            preferred_lifetime: 3600,
            valid_lifetime: 7200,
            pd_preferred_lifetime: 3600,
            pd_valid_lifetime: 86400,
            t1: 1800,
            t2: 2700,
        }
    }
}

/// DHCPv6 server — handles address and prefix assignment.
pub struct Dhcpv6Server {
    socket: UdpSocket,
    config: Dhcpv6ServerConfig,
    pool: LeasePool,
}

impl Dhcpv6Server {
    /// Create a new DHCPv6 server.
    pub fn new(config: Dhcpv6ServerConfig, mut pool: LeasePool) -> Result<Self, io::Error> {
        // Bind to UDP 547 on all interfaces
        let socket = UdpSocket::bind(("::", DHCPV6_SERVER_PORT))?;
        socket.set_broadcast(false)?; // DHCPv6 uses multicast, not broadcast
        socket.set_read_timeout(Some(Duration::from_millis(200)))?;

        Ok(Self {
            socket,
            config,
            pool,
        })
    }

    /// Run the server event loop.
    pub fn run(&mut self) -> Result<(), io::Error> {
        edgerun_log::info!(
            "edgerun-dhcpv6: server listening on UDP {}",
            DHCPV6_SERVER_PORT
        );

        loop {
            match self.tick() {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) if e.kind() == io::ErrorKind::TimedOut => continue,
                Err(e) => {
                    edgerun_log::warn!("edgerun-dhcpv6: server error: {}", e);
                    return Err(e);
                }
            }
        }
    }

    /// Process one incoming packet.
    pub fn tick(&mut self) -> Result<(), io::Error> {
        let mut buf = [0u8; 1500];
        let (n, src) = self.socket.recv_from(&mut buf)?;

        let msg = match Dhcpv6Message::from_wire(&buf[..n]) {
            Ok(m) => m,
            Err(e) => {
                edgerun_log::warn!("edgerun-dhcpv6: failed to parse from {}: {}", src, e);
                return Ok(());
            }
        };

        edgerun_log::debug!(
            "edgerun-dhcpv6: {} from {} (xid={})",
            msg.msg_type,
            src,
            msg.transaction_id
        );

        let response = self.handle_message(&msg, src)?;

        if let Some(resp) = response {
            let wire = resp.to_wire();
            let dest = SocketAddr::new(src.ip(), DHCPV6_CLIENT_PORT);
            if let Err(e) = self.socket.send_to(&wire, dest) {
                edgerun_log::warn!("edgerun-dhcpv6: failed to send to {}: {}", dest, e);
            }
        }

        Ok(())
    }

    /// Handle a DHCPv6 message and produce a response.
    fn handle_message(
        &mut self,
        msg: &Dhcpv6Message,
        src: SocketAddr,
    ) -> Result<Option<Dhcpv6Message>, io::Error> {
        match msg.msg_type {
            Dhcpv6MsgType::Solicit => self.handle_solicit(msg, src),
            Dhcpv6MsgType::Request => self.handle_request(msg, src),
            Dhcpv6MsgType::Renew => self.handle_renew(msg, src),
            Dhcpv6MsgType::Rebind => self.handle_rebind(msg, src),
            Dhcpv6MsgType::Release => self.handle_release(msg, src),
            Dhcpv6MsgType::Decline => self.handle_decline(msg, src),
            Dhcpv6MsgType::InformationRequest => self.handle_information_request(msg, src),
            Dhcpv6MsgType::Confirm => self.handle_confirm(msg, src),
            Dhcpv6MsgType::Advertise
            | Dhcpv6MsgType::Reply
            | Dhcpv6MsgType::Reconfigure
            | Dhcpv6MsgType::RelayForw
            | Dhcpv6MsgType::RelayRepl => {
                Ok(None) // Server doesn't process these
            }
        }
    }

    // --- Message handlers ---

    fn handle_solicit(
        &mut self,
        msg: &Dhcpv6Message,
        _src: SocketAddr,
    ) -> Result<Option<Dhcpv6Message>, io::Error> {
        let client_duid = match msg.client_duid() {
            Some(d) => d,
            None => return Ok(None),
        };

        let iaid = match msg.ia_id() {
            Some(id) => id,
            None => return Ok(None),
        };

        // Check for rapid commit — if present, respond with Reply instead of Advertise
        let rapid_commit = msg.options.iter().any(|o| o.code == OPT_RAPID_COMMIT);

        // Allocate an address
        let addr = match self.pool.allocate_address(
            client_duid.clone(),
            iaid,
            self.config.preferred_lifetime,
            self.config.valid_lifetime,
        ) {
            Some(a) => a,
            None => {
                // Pool exhausted — send Reply with NoAddrsAvail
                return Ok(Some(self.make_error_reply(
                    msg,
                    StatusCode::NoAddrsAvail,
                    "No addresses available",
                )));
            }
        };

        // Build IA_NA response with IAADDR
        let t1 = if self.config.t1 > 0 {
            self.config.t1
        } else {
            self.config.preferred_lifetime / 2
        };
        let t2 = if self.config.t2 > 0 {
            self.config.t2
        } else {
            (self.config.valid_lifetime * 7) / 8
        };

        let iaaddr = Dhcpv6Option::iaaddr(
            addr,
            self.config.preferred_lifetime,
            self.config.valid_lifetime,
            vec![],
        );
        let ia_na = Dhcpv6Option::ia_na(iaid, t1, t2, vec![iaaddr]);

        let mut resp_options = vec![
            Dhcpv6Option::from_raw(OPT_SERVERID, self.config.server_duid.clone()),
            ia_na,
            Dhcpv6Option::dns_servers(&self.config.dns_servers),
            Dhcpv6Option::domain_list(
                &self
                    .config
                    .domain_list
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>(),
            ),
        ];

        if rapid_commit {
            resp_options.push(Dhcpv6Option::from_raw(OPT_RAPID_COMMIT, vec![]));
        }

        let resp_type = if rapid_commit {
            Dhcpv6MsgType::Reply
        } else {
            Dhcpv6MsgType::Advertise
        };

        edgerun_log::info!(
            "edgerun-dhcpv6: {} {} to duid={}",
            resp_type,
            addr,
            Duid::from_wire(&client_duid)
                .map(|d| format!("{}", d))
                .unwrap_or_default()
        );

        Ok(Some(Dhcpv6Message {
            msg_type: resp_type,
            transaction_id: msg.transaction_id,
            options: resp_options,
        }))
    }

    fn handle_request(
        &mut self,
        msg: &Dhcpv6Message,
        _src: SocketAddr,
    ) -> Result<Option<Dhcpv6Message>, io::Error> {
        let client_duid = match msg.client_duid() {
            Some(d) => d,
            None => return Ok(None),
        };

        // Verify server ID matches us
        if let Some(server_duid) = msg.server_duid() {
            if server_duid != self.config.server_duid {
                return Ok(None); // Not for us
            }
        }

        // Find existing lease or allocate new
        let iaid = match msg.ia_id() {
            Some(id) => id,
            None => {
                return Ok(Some(self.make_error_reply(
                    msg,
                    StatusCode::MalformedQuery,
                    "Missing IAID",
                )))
            }
        };

        // Check if already leased
        let existing = self
            .pool
            .address_leases
            .iter()
            .find(|l| l.client_duid == client_duid && l.iaid == iaid);

        let (addr, preferred, valid) = if let Some(lease) = existing {
            (
                lease.address,
                lease.preferred_lifetime,
                lease.valid_lifetime,
            )
        } else {
            match self.pool.allocate_address(
                client_duid.clone(),
                iaid,
                self.config.preferred_lifetime,
                self.config.valid_lifetime,
            ) {
                Some(a) => (
                    a,
                    self.config.preferred_lifetime,
                    self.config.valid_lifetime,
                ),
                None => {
                    return Ok(Some(self.make_error_reply(
                        msg,
                        StatusCode::NoAddrsAvail,
                        "No addresses available",
                    )))
                }
            }
        };

        let t1 = self.config.t1;
        let t2 = self.config.t2;
        let iaaddr = Dhcpv6Option::iaaddr(addr, preferred, valid, vec![]);
        let ia_na = Dhcpv6Option::ia_na(iaid, t1, t2, vec![iaaddr]);

        let resp_options = vec![
            Dhcpv6Option::from_raw(OPT_SERVERID, self.config.server_duid.clone()),
            ia_na,
            Dhcpv6Option::dns_servers(&self.config.dns_servers),
        ];

        edgerun_log::info!(
            "edgerun-dhcpv6: REPLY {} to duid={}",
            addr,
            Duid::from_wire(&client_duid)
                .map(|d| format!("{}", d))
                .unwrap_or_default()
        );

        Ok(Some(Dhcpv6Message {
            msg_type: Dhcpv6MsgType::Reply,
            transaction_id: msg.transaction_id,
            options: resp_options,
        }))
    }

    fn handle_renew(
        &mut self,
        msg: &Dhcpv6Message,
        _src: SocketAddr,
    ) -> Result<Option<Dhcpv6Message>, io::Error> {
        self.handle_lease_update(msg, Dhcpv6MsgType::Renew)
    }

    fn handle_rebind(
        &mut self,
        msg: &Dhcpv6Message,
        _src: SocketAddr,
    ) -> Result<Option<Dhcpv6Message>, io::Error> {
        self.handle_lease_update(msg, Dhcpv6MsgType::Rebind)
    }

    fn handle_lease_update(
        &mut self,
        msg: &Dhcpv6Message,
        msg_type: Dhcpv6MsgType,
    ) -> Result<Option<Dhcpv6Message>, io::Error> {
        let client_duid = match msg.client_duid() {
            Some(d) => d,
            None => return Ok(None),
        };

        let iaid = match msg.ia_id() {
            Some(id) => id,
            None => return Ok(None),
        };

        // Find existing lease
        let lease = self
            .pool
            .address_leases
            .iter_mut()
            .find(|l| l.client_duid == client_duid && l.iaid == iaid && !l.is_expired());

        if let Some(lease) = lease {
            let t1 = self.config.t1;
            let t2 = self.config.t2;
            let iaaddr = Dhcpv6Option::iaaddr(
                lease.address,
                lease.preferred_lifetime,
                lease.valid_lifetime,
                vec![],
            );
            let ia_na = Dhcpv6Option::ia_na(iaid, t1, t2, vec![iaaddr]);
            lease.state = if msg_type == Dhcpv6MsgType::Renew {
                LeaseState::Renewing
            } else {
                LeaseState::Rebinding
            };

            let resp_options = vec![
                Dhcpv6Option::from_raw(OPT_SERVERID, self.config.server_duid.clone()),
                ia_na,
            ];

            Ok(Some(Dhcpv6Message {
                msg_type: Dhcpv6MsgType::Reply,
                transaction_id: msg.transaction_id,
                options: resp_options,
            }))
        } else {
            Ok(Some(self.make_error_reply(
                msg,
                StatusCode::NoBinding,
                "No binding for IAID",
            )))
        }
    }

    fn handle_release(
        &mut self,
        msg: &Dhcpv6Message,
        _src: SocketAddr,
    ) -> Result<Option<Dhcpv6Message>, io::Error> {
        let client_duid = match msg.client_duid() {
            Some(d) => d,
            None => return Ok(None),
        };

        let iaid = match msg.ia_id() {
            Some(id) => id,
            None => return Ok(None),
        };

        // Find and release lease
        if let Some(lease) = self
            .pool
            .address_leases
            .iter()
            .find(|l| l.client_duid == client_duid && l.iaid == iaid)
        {
            let addr = lease.address;
            self.pool.release_address(&client_duid, iaid, addr);
            edgerun_log::info!(
                "edgerun-dhcpv6: RELEASE {} from duid={}",
                addr,
                Duid::from_wire(&client_duid)
                    .map(|d| format!("{}", d))
                    .unwrap_or_default()
            );
        }

        // Release is typically not acknowledged per RFC 8415
        Ok(None)
    }

    fn handle_decline(
        &mut self,
        msg: &Dhcpv6Message,
        _src: SocketAddr,
    ) -> Result<Option<Dhcpv6Message>, io::Error> {
        let client_duid = match msg.client_duid() {
            Some(d) => d,
            None => return Ok(None),
        };

        let iaid = match msg.ia_id() {
            Some(id) => id,
            None => return Ok(None),
        };

        // Find and mark as declined
        if let Some(lease) = self
            .pool
            .address_leases
            .iter_mut()
            .find(|l| l.client_duid == client_duid && l.iaid == iaid)
        {
            lease.state = LeaseState::Declined;
            edgerun_log::warn!(
                "edgerun-dhcpv6: DECLINE {} from duid={}",
                lease.address,
                Duid::from_wire(&client_duid)
                    .map(|d| format!("{}", d))
                    .unwrap_or_default()
            );
        }

        Ok(None)
    }

    fn handle_information_request(
        &mut self,
        msg: &Dhcpv6Message,
        _src: SocketAddr,
    ) -> Result<Option<Dhcpv6Message>, io::Error> {
        // Stateless: return options only, no addresses
        let mut options = vec![
            Dhcpv6Option::from_raw(OPT_SERVERID, self.config.server_duid.clone()),
            Dhcpv6Option::dns_servers(&self.config.dns_servers),
        ];

        // Add domain list if requested
        let oro = msg.option_request_list();
        if oro.contains(&OPT_DOMAIN_LIST) {
            options.push(Dhcpv6Option::domain_list(
                &self
                    .config
                    .domain_list
                    .iter()
                    .map(|s| s.as_str())
                    .collect::<Vec<_>>(),
            ));
        }

        Ok(Some(Dhcpv6Message {
            msg_type: Dhcpv6MsgType::Reply,
            transaction_id: msg.transaction_id,
            options,
        }))
    }

    fn handle_confirm(
        &mut self,
        msg: &Dhcpv6Message,
        _src: SocketAddr,
    ) -> Result<Option<Dhcpv6Message>, io::Error> {
        // Confirm that addresses are still appropriate for the new link
        // For now, just acknowledge
        let options = vec![
            Dhcpv6Option::from_raw(OPT_SERVERID, self.config.server_duid.clone()),
            Dhcpv6Option::status_code(StatusCode::Success, ""),
        ];

        Ok(Some(Dhcpv6Message {
            msg_type: Dhcpv6MsgType::Reply,
            transaction_id: msg.transaction_id,
            options,
        }))
    }

    // --- Helpers ---

    fn make_error_reply(
        &self,
        msg: &Dhcpv6Message,
        status: StatusCode,
        message: &str,
    ) -> Dhcpv6Message {
        Dhcpv6Message {
            msg_type: Dhcpv6MsgType::Reply,
            transaction_id: msg.transaction_id,
            options: vec![
                Dhcpv6Option::from_raw(OPT_SERVERID, self.config.server_duid.clone()),
                Dhcpv6Option::status_code(status, message),
            ],
        }
    }

    /// Get pool statistics.
    pub fn stats(&self) -> String {
        format!(
            "addresses: {} available, {} leased | prefixes: {} available, {} leased",
            self.pool.available_addresses.len(),
            self.pool.active_address_count(),
            self.pool.available_prefixes.len(),
            self.pool.active_prefix_count(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::std::net::Ipv6Addr;

    #[test]
    fn test_server_config_default() {
        let config = Dhcpv6ServerConfig::default();
        assert!(!config.server_duid.is_empty());
        assert!(!config.dns_servers.is_empty());
    }

    #[test]
    fn test_server_with_pool() {
        let mut pool = LeasePool::new();
        pool.add_addresses([Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)]);

        let config = Dhcpv6ServerConfig::default();
        // Can't actually bind to port 547 without root, so just test config
        assert!(!config.server_duid.is_empty());
        assert_eq!(pool.available_addresses.len(), 1);
    }
}
