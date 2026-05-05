//! DHCPv6 server protocol state without UDP socket ownership.

use crate::std::io;
use crate::std::net::Ipv6Addr;
use crate::std::prelude::v1::*;

use super::duid::default_server_duid;
use super::lease::{LeasePool, LeaseState};
use super::message::{DHCPV6_CLIENT_PORT, Dhcpv6Message, Dhcpv6MsgType};
use super::options::{
    Dhcpv6Option, OPT_DNS_SERVERS, OPT_DOMAIN_LIST, OPT_IA_NA, OPT_IAADDR, OPT_RAPID_COMMIT,
    OPT_SERVERID, StatusCode,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dhcpv6Datagram {
    pub port: u16,
    pub wire: Vec<u8>,
}

/// DHCPv6 server core — handles address and prefix assignment.
pub struct Dhcpv6ServerCore {
    pub config: Dhcpv6ServerConfig,
    pub pool: LeasePool,
}

impl Dhcpv6ServerCore {
    pub fn new(config: Dhcpv6ServerConfig, pool: LeasePool) -> Self {
        Self { config, pool }
    }

    pub fn handle_wire(&mut self, wire: &[u8]) -> Result<Option<Dhcpv6Datagram>, io::Error> {
        let msg = Dhcpv6Message::from_wire(wire)?;
        Ok(self.handle_message(&msg)?.map(|message| Dhcpv6Datagram {
            port: DHCPV6_CLIENT_PORT,
            wire: message.to_wire(),
        }))
    }

    /// Handle a DHCPv6 message and produce a response.
    pub fn handle_message(
        &mut self,
        msg: &Dhcpv6Message,
    ) -> Result<Option<Dhcpv6Message>, io::Error> {
        match msg.msg_type {
            Dhcpv6MsgType::Solicit => self.handle_solicit(msg),
            Dhcpv6MsgType::Request => self.handle_request(msg),
            Dhcpv6MsgType::Renew => self.handle_lease_update(msg, Dhcpv6MsgType::Renew),
            Dhcpv6MsgType::Rebind => self.handle_lease_update(msg, Dhcpv6MsgType::Rebind),
            Dhcpv6MsgType::Release => self.handle_release(msg),
            Dhcpv6MsgType::Decline => self.handle_decline(msg),
            Dhcpv6MsgType::InformationRequest => self.handle_information_request(msg),
            Dhcpv6MsgType::Confirm => self.handle_confirm(msg),
            Dhcpv6MsgType::Advertise
            | Dhcpv6MsgType::Reply
            | Dhcpv6MsgType::Reconfigure
            | Dhcpv6MsgType::RelayForw
            | Dhcpv6MsgType::RelayRepl => Ok(None),
        }
    }

    fn handle_solicit(&mut self, msg: &Dhcpv6Message) -> Result<Option<Dhcpv6Message>, io::Error> {
        let client_duid = match msg.client_duid() {
            Some(d) => d,
            None => return Ok(None),
        };

        let iaid = match msg.ia_id() {
            Some(id) => id,
            None => return Ok(None),
        };

        let rapid_commit = msg.options.iter().any(|o| o.code == OPT_RAPID_COMMIT);
        let addr = match self.pool.allocate_address(
            client_duid.clone(),
            iaid,
            self.config.preferred_lifetime,
            self.config.valid_lifetime,
        ) {
            Some(a) => a,
            None => {
                return Ok(Some(self.make_error_reply(
                    msg,
                    StatusCode::NoAddrsAvail,
                    "No addresses available",
                )));
            }
        };

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

        Ok(Some(Dhcpv6Message {
            msg_type: resp_type,
            transaction_id: msg.transaction_id,
            options: resp_options,
        }))
    }

    fn handle_request(&mut self, msg: &Dhcpv6Message) -> Result<Option<Dhcpv6Message>, io::Error> {
        let client_duid = match msg.client_duid() {
            Some(d) => d,
            None => return Ok(None),
        };

        if let Some(server_duid) = msg.server_duid() {
            if server_duid != self.config.server_duid {
                return Ok(None);
            }
        }

        let iaid = match msg.ia_id() {
            Some(id) => id,
            None => {
                return Ok(Some(self.make_error_reply(
                    msg,
                    StatusCode::MalformedQuery,
                    "Missing IAID",
                )));
            }
        };

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
                    )));
                }
            }
        };

        let iaaddr = Dhcpv6Option::iaaddr(addr, preferred, valid, vec![]);
        let ia_na = Dhcpv6Option::ia_na(iaid, self.config.t1, self.config.t2, vec![iaaddr]);

        Ok(Some(Dhcpv6Message {
            msg_type: Dhcpv6MsgType::Reply,
            transaction_id: msg.transaction_id,
            options: vec![
                Dhcpv6Option::from_raw(OPT_SERVERID, self.config.server_duid.clone()),
                ia_na,
                Dhcpv6Option::dns_servers(&self.config.dns_servers),
            ],
        }))
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

        let lease = self
            .pool
            .address_leases
            .iter_mut()
            .find(|l| l.client_duid == client_duid && l.iaid == iaid && !l.is_expired());

        if let Some(lease) = lease {
            let iaaddr = Dhcpv6Option::iaaddr(
                lease.address,
                lease.preferred_lifetime,
                lease.valid_lifetime,
                vec![],
            );
            let ia_na = Dhcpv6Option::ia_na(iaid, self.config.t1, self.config.t2, vec![iaaddr]);
            lease.state = if msg_type == Dhcpv6MsgType::Renew {
                LeaseState::Renewing
            } else {
                LeaseState::Rebinding
            };

            Ok(Some(Dhcpv6Message {
                msg_type: Dhcpv6MsgType::Reply,
                transaction_id: msg.transaction_id,
                options: vec![
                    Dhcpv6Option::from_raw(OPT_SERVERID, self.config.server_duid.clone()),
                    ia_na,
                ],
            }))
        } else {
            Ok(Some(self.make_error_reply(
                msg,
                StatusCode::NoBinding,
                "No binding for IAID",
            )))
        }
    }

    fn handle_release(&mut self, msg: &Dhcpv6Message) -> Result<Option<Dhcpv6Message>, io::Error> {
        let client_duid = match msg.client_duid() {
            Some(d) => d,
            None => return Ok(None),
        };

        let iaid = match msg.ia_id() {
            Some(id) => id,
            None => return Ok(None),
        };

        if let Some(lease) = self
            .pool
            .address_leases
            .iter()
            .find(|l| l.client_duid == client_duid && l.iaid == iaid)
        {
            let addr = lease.address;
            self.pool.release_address(&client_duid, iaid, addr);
        }

        Ok(None)
    }

    fn handle_decline(&mut self, msg: &Dhcpv6Message) -> Result<Option<Dhcpv6Message>, io::Error> {
        let client_duid = match msg.client_duid() {
            Some(d) => d,
            None => return Ok(None),
        };

        let iaid = match msg.ia_id() {
            Some(id) => id,
            None => return Ok(None),
        };

        if let Some(lease) = self
            .pool
            .address_leases
            .iter_mut()
            .find(|l| l.client_duid == client_duid && l.iaid == iaid)
        {
            lease.state = LeaseState::Declined;
        }

        Ok(None)
    }

    fn handle_information_request(
        &mut self,
        msg: &Dhcpv6Message,
    ) -> Result<Option<Dhcpv6Message>, io::Error> {
        let mut options = vec![
            Dhcpv6Option::from_raw(OPT_SERVERID, self.config.server_duid.clone()),
            Dhcpv6Option::dns_servers(&self.config.dns_servers),
        ];

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

    fn handle_confirm(&mut self, msg: &Dhcpv6Message) -> Result<Option<Dhcpv6Message>, io::Error> {
        Ok(Some(Dhcpv6Message {
            msg_type: Dhcpv6MsgType::Reply,
            transaction_id: msg.transaction_id,
            options: vec![
                Dhcpv6Option::from_raw(OPT_SERVERID, self.config.server_duid.clone()),
                Dhcpv6Option::status_code(StatusCode::Success, ""),
            ],
        }))
    }

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
    use super::super::duid::Duid;
    use super::*;

    #[test]
    fn solicit_produces_advertise_without_socket() {
        let client_duid = Duid::ll(1, &[1, 2, 3, 4, 5, 6]).to_wire();
        let mut pool = LeasePool::new();
        pool.add_addresses([Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 10)]);
        let mut core = Dhcpv6ServerCore::new(Dhcpv6ServerConfig::default(), pool);

        let msg = Dhcpv6Message::solicit(7, &client_duid, 0);
        let datagram = core.handle_wire(&msg.to_wire()).unwrap().unwrap();
        assert_eq!(datagram.port, DHCPV6_CLIENT_PORT);

        let response = Dhcpv6Message::from_wire(&datagram.wire).unwrap();
        assert_eq!(response.msg_type, Dhcpv6MsgType::Advertise);
        assert_eq!(core.pool.active_address_count(), 1);
    }
}
