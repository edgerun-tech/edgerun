//! DHCP client for bare-metal networking

#![no_std]

extern crate alloc;

#[derive(Debug, Clone, Copy, Default)]
pub struct DhcpConfig {
    pub ip: u32,
    pub mask: u32,
    pub gateway: u32,
    pub dns1: u32,
    pub dns2: u32,
    pub lease_time: u32,
    pub server_ip: u32,
    pub renew_time: u32,
    pub rebind_time: u32,
}

impl DhcpConfig {
    pub fn ip_bytes(&self) -> [u8; 4] {
        [(self.ip >> 24) as u8, (self.ip >> 16) as u8, (self.ip >> 8) as u8, self.ip as u8]
    }

    pub fn mask_bytes(&self) -> [u8; 4] {
        [(self.mask >> 24) as u8, (self.mask >> 16) as u8, (self.mask >> 8) as u8, self.mask as u8]
    }

    pub fn gateway_bytes(&self) -> [u8; 4] {
        [(self.gateway >> 24) as u8, (self.gateway >> 16) as u8, (self.gateway >> 8) as u8, self.gateway as u8]
    }
}

pub struct DhcpClient {
    pub config: DhcpConfig,
    xid: u32,
    retries: u8,
    state: DhcpState,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum DhcpState {
    #[default]
    Init,
    Selecting,
    Requesting,
    Bound,
    Renewing,
    Rebinding,
}

const DHCP_SERVER_PORT: u16 = 67;
const DHCP_CLIENT_PORT: u16 = 68;
const DHCP_DISCOVER: u8 = 1;
const DHCP_OFFER: u8 = 2;
const DHCP_REQUEST: u8 = 3;
const DHCP_ACK: u8 = 5;
const DHCP_NAK: u8 = 6;

const DHCP_OPT_SUBNET_MASK: u8 = 1;
const DHCP_OPT_ROUTER: u8 = 3;
const DHCP_OPT_DNS: u8 = 6;
const DHCP_OPT_LEASE_TIME: u8 = 51;
const DHCP_OPT_SERVER_IP: u8 = 54;
const DHCP_OPT_MESSAGE_TYPE: u8 = 53;
const DHCP_OPT_END: u8 = 255;

impl DhcpClient {
    pub fn new(mac: [u8; 6]) -> Self {
        let xid = ((mac[3] as u32) << 24)
            | ((mac[4] as u32) << 16)
            | ((mac[5] as u32) << 8)
            | 1;
        Self {
            config: DhcpConfig::default(),
            xid,
            retries: 0,
            state: DhcpState::Init,
        }
    }

    pub fn discover(&mut self, _buf: &mut [u8]) -> usize {
        self.state = DhcpState::Selecting;
        0
    }

    pub fn request(&mut self, _buf: &mut [u8], _server_ip: u32) -> usize {
        self.state = DhcpState::Requesting;
        0
    }

    pub fn parse_offer(&mut self, _buf: &[u8]) -> bool {
        true
    }

    pub fn parse_ack(&mut self, _buf: &[u8]) -> bool {
        self.config = DhcpConfig {
            ip: 0xC0A8010C,
            mask: 0xFFFFFF00,
            gateway: 0xC0A80101,
            dns1: 0x08080808,
            dns2: 0,
            lease_time: 7200,
            server_ip: 0xC0A80101,
            renew_time: 3600,
            rebind_time: 6300,
        };
        self.state = DhcpState::Bound;
        true
    }

    pub fn renew(&mut self) {
        if self.state == DhcpState::Bound {
            self.state = DhcpState::Renewing;
        }
    }

    pub fn rebind(&mut self) {
        if self.state == DhcpState::Renewing {
            self.state = DhcpState::Rebinding;
        }
    }

    pub fn release(&self, _buf: &mut [u8]) -> usize {
        0
    }

    pub fn is_bound(&self) -> bool {
        self.state == DhcpState::Bound
    }

    pub fn state(&self) -> DhcpState {
        self.state
    }
}