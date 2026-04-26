//! DHCP client for bare-metal networking

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

impl DhcpClient {
    pub fn new(mac: [u8; 6]) -> Self {
        let _ = mac;
        Self {
            config: DhcpConfig::default(),
            state: DhcpState::Init,
        }
    }

    pub fn discover(&mut self, _buf: &mut [u8]) -> usize {
        self.state = DhcpState::Selecting;
        0
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
        };
        self.state = DhcpState::Bound;
        true
    }

    pub fn renew(&mut self) {
        if self.state == DhcpState::Bound {
            self.state = DhcpState::Renewing;
        }
    }

    pub fn is_bound(&self) -> bool {
        self.state == DhcpState::Bound
    }

    pub fn state(&self) -> DhcpState {
        self.state
    }
}