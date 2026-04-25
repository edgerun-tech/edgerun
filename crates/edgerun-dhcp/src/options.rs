//! DHCP options convenience helpers.

use super::message::DhcpOptions;
use super::message::{
    OPT_DNS_SERVER, OPT_LEASE_TIME, OPT_REBIND_TIME, OPT_RENEWAL_TIME, OPT_ROUTER, OPT_SUBNET_MASK,
};
use std::net::Ipv4Addr;

/// Builder for constructing DHCP option sets.
pub struct OptionsBuilder {
    opts: DhcpOptions,
}

impl OptionsBuilder {
    pub fn new() -> Self {
        Self {
            opts: DhcpOptions::default(),
        }
    }

    pub fn message_type(mut self, mt: super::DhcpMessageType) -> Self {
        self.opts.message_type = Some(mt);
        self
    }

    pub fn subnet_mask(mut self, mask: Ipv4Addr) -> Self {
        self.opts.subnet_mask = Some(mask);
        self
    }

    pub fn router(mut self, gw: Ipv4Addr) -> Self {
        self.opts.router = Some(gw);
        self
    }

    pub fn dns_servers(mut self, servers: Vec<Ipv4Addr>) -> Self {
        self.opts.dns_servers = servers;
        self
    }

    pub fn server_id(mut self, id: Ipv4Addr) -> Self {
        self.opts.server_id = Some(id);
        self
    }

    pub fn requested_ip(mut self, ip: Ipv4Addr) -> Self {
        self.opts.requested_ip = Some(ip);
        self
    }

    pub fn lease_time(mut self, secs: u32) -> Self {
        self.opts.lease_time = Some(secs);
        self.opts.renewal_time = Some(secs / 2);
        self.opts.rebind_time = Some(secs * 7 / 8);
        self
    }

    pub fn param_request(mut self, codes: Vec<u8>) -> Self {
        self.opts.param_request_list = codes;
        self
    }

    pub fn client_id(mut self, id: Vec<u8>) -> Self {
        self.opts.client_id = Some(id);
        self
    }

    pub fn build(self) -> DhcpOptions {
        self.opts
    }
}

impl Default for OptionsBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Common parameter request list for clients.
pub fn standard_param_request() -> Vec<u8> {
    vec![
        OPT_SUBNET_MASK,
        OPT_ROUTER,
        OPT_DNS_SERVER,
        OPT_LEASE_TIME,
        OPT_RENEWAL_TIME,
        OPT_REBIND_TIME,
    ]
}
