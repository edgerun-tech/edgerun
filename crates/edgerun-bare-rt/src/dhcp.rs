//! DHCP client for bare metal

#![allow(dead_code)]

use crate::ip::IpAddr;

pub use crate::ip::{DHCP_SERVER_PORT, DHCP_CLIENT_PORT};

pub const DHCP_MSG_DISCOVER: u8 = 1;
pub const DHCP_MSG_OFFER: u8 = 2;
pub const DHCP_MSG_REQUEST: u8 = 3;
pub const DHCP_MSG_ACK: u8 = 5;
pub const DHCP_MSG_NACK: u8 = 6;

pub const DHCP_OPT_SUBNET_MASK: u8 = 1;
pub const DHCP_OPT_ROUTER: u8 = 3;
pub const DHCP_OPT_DNS: u8 = 6;
pub const DHCP_OPT_REQUESTED_IP: u8 = 50;
pub const DHCP_OPT_LEASE_TIME: u8 = 51;
pub const DHCP_OPT_MSG_TYPE: u8 = 53;
pub const DHCP_OPT_SERVER_ID: u8 = 54;
pub const DHCP_OPT_END: u8 = 255;

pub const DHCP_COOKIE: u32 = 0x63825363;

pub const DHCP_HWTYPE_ETHERNET: u8 = 1;

pub struct DhcpOption {
    pub code: u8,
    pub len: u8,
    pub data: [u8; 254],
}

pub struct DhcpMessage {
    pub op: u8,
    pub htype: u8,
    pub hlen: u8,
    pub hops: u8,
    pub xid: u32,
    pub secs: u16,
    pub flags: u16,
    pub ciaddr: [u8; 4],
    pub yiaddr: [u8; 4],
    pub siaddr: [u8; 4],
    pub giaddr: [u8; 4],
    pub chaddr: [u8; 16],
}

impl DhcpMessage {
    pub fn new(xid: u32, mac: [u8; 6]) -> Self {
        let mut chaddr = [0u8; 16];
        chaddr[..6].copy_from_slice(&mac);
        
        Self {
            op: 1,
            htype: DHCP_HWTYPE_ETHERNET,
            hlen: 6,
            hops: 0,
            xid,
            secs: 0,
            flags: 0x8000,
            ciaddr: [0; 4],
            yiaddr: [0; 4],
            siaddr: [0; 4],
            giaddr: [0; 4],
            chaddr,
        }
    }

    pub fn to_bytes(&self, buf: &mut [u8]) {
        buf[0] = self.op;
        buf[1] = self.htype;
        buf[2] = self.hlen;
        buf[3] = self.hops;
        buf[4..8].copy_from_slice(&self.xid.to_be_bytes());
        buf[8..10].copy_from_slice(&self.secs.to_be_bytes());
        buf[10..12].copy_from_slice(&self.flags.to_be_bytes());
        buf[12..16].copy_from_slice(&self.ciaddr);
        buf[16..20].copy_from_slice(&self.yiaddr);
        buf[20..24].copy_from_slice(&self.siaddr);
        buf[24..28].copy_from_slice(&self.giaddr);
        buf[28..44].copy_from_slice(&self.chaddr);
    }

    pub fn yiaddr_ip(&self) -> IpAddr {
        IpAddr::from_slice(&self.yiaddr)
    }
}

pub struct DhcpClient {
    xid: u32,
    mac: [u8; 6],
    timeout: usize,
    state: DhcpState,
}

#[derive(Clone, Copy, PartialEq)]
pub enum DhcpState {
    Init,
    Selecting,
    Requesting,
    Bound,
    Renewing,
}

impl DhcpClient {
    pub fn new(mac: [u8; 6]) -> Self {
        Self {
            xid: 0,
            mac,
            timeout: 0,
            state: DhcpState::Init,
        }
    }

    pub fn init(&mut self) {
        self.xid = 0x12345678;
        self.timeout = 0;
        self.state = DhcpState::Init;
    }

    pub fn discover(&self, buf: &mut [u8]) -> usize {
        let msg = DhcpMessage::new(self.xid, self.mac);
        let pos = 44;
        msg.to_bytes(buf);
        
        buf[pos] = 0x63;
        buf[pos + 1] = 0x82;
        buf[pos + 2] = 0x53;
        buf[pos + 3] = 0x63;
        
        let opts = pos + 4;
        buf[opts] = DHCP_OPT_MSG_TYPE;
        buf[opts + 1] = 1;
        buf[opts + 2] = DHCP_MSG_DISCOVER;
        
        buf[opts + 3] = DHCP_OPT_REQUESTED_IP;
        buf[opts + 4] = 4;
        buf[opts + 5] = 0;
        buf[opts + 6] = 0;
        buf[opts + 7] = 0;
        buf[opts + 8] = 0;
        
        buf[opts + 9] = DHCP_OPT_END;
        buf[opts + 10] = 0;
        
        240
    }

    pub fn request(&self, server_ip: IpAddr, requested: IpAddr, buf: &mut [u8]) -> usize {
        let msg = DhcpMessage::new(self.xid, self.mac);
        let pos = 44;
        msg.to_bytes(buf);
        
        buf[pos] = 0x63;
        buf[pos + 1] = 0x82;
        buf[pos + 2] = 0x53;
        buf[pos + 3] = 0x63;
        
        let opts = pos + 4;
        buf[opts] = DHCP_OPT_MSG_TYPE;
        buf[opts + 1] = 1;
        buf[opts + 2] = DHCP_MSG_REQUEST;
        
        buf[opts + 3] = DHCP_OPT_REQUESTED_IP;
        buf[opts + 4] = 4;
        let req_bytes = requested.as_bytes();
        buf[opts + 5] = req_bytes[0];
        buf[opts + 6] = req_bytes[1];
        buf[opts + 7] = req_bytes[2];
        buf[opts + 8] = req_bytes[3];
        
        buf[opts + 9] = DHCP_OPT_SERVER_ID;
        buf[opts + 10] = 4;
        let srv_bytes = server_ip.as_bytes();
        buf[opts + 11] = srv_bytes[0];
        buf[opts + 12] = srv_bytes[1];
        buf[opts + 13] = srv_bytes[2];
        buf[opts + 14] = srv_bytes[3];
        
        buf[opts + 15] = DHCP_OPT_END;
        
        240
    }

    pub fn parse_offer(&mut self, buf: &[u8]) -> Option<(IpAddr, IpAddr, IpAddr)> {
        if buf.len() < 240 {
            return None;
        }
        
        let cookie = u32::from_be_bytes([buf[44], buf[45], buf[46], buf[47]]);
        if cookie != DHCP_COOKIE {
            return None;
        }
        
        let mut msg_type = 0;
        let mut subnet = IpAddr::new(255, 255, 255, 0);
        let mut gateway = IpAddr::zero();
        
        let opt_off = 240;
        let mut i = opt_off;
        while i < buf.len().min(548) - 2 {
            let code = buf[i];
            if code == DHCP_OPT_END {
                break;
            }
            if code == DHCP_OPT_MSG_TYPE {
                msg_type = buf[i + 2];
            } else if code == DHCP_OPT_SUBNET_MASK {
                subnet = IpAddr::from_slice(&buf[i + 2..i + 6]);
            } else if code == DHCP_OPT_ROUTER {
                gateway = IpAddr::from_slice(&buf[i + 2..i + 6]);
            }
            i += 2 + buf[i + 1] as usize;
        }
        
        if msg_type == DHCP_MSG_OFFER || msg_type == DHCP_MSG_ACK {
            let yiaddr = IpAddr::from_slice(&buf[16..20]);
            Some((yiaddr, subnet, gateway))
        } else {
            None
        }
    }

    pub fn is_bound(&self) -> bool {
        self.state == DhcpState::Bound
    }

    pub fn tick(&mut self) {
        if self.timeout > 0 {
            self.timeout -= 1;
        }
        if self.timeout == 0 && self.state != DhcpState::Bound {
            self.state = DhcpState::Init;
        }
    }
}

impl DhcpMessage {
    pub fn apply(&self, stack: &mut super::ip::IpStack) {
        stack.ip = self.yiaddr_ip();
    }
}

pub struct DhcpVlan {
    pub inner: DhcpClient,
    pub vlan_id: u16,
}