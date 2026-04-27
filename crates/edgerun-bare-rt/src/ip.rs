//! IP Stack - IPv4, ARP, ICMP, UDP, TCP

#![allow(dead_code)]

pub const ETH_TYPE_IPV4: u16 = 0x0800;
pub const ETH_TYPE_ARP: u16 = 0x0806;

pub const IP_PROTO_ICMP: u8 = 1;
pub const IP_PROTO_TCP: u8 = 6;
pub const IP_PROTO_UDP: u8 = 17;

pub const ARP_OP_REQUEST: u16 = 1;
pub const ARP_OP_REPLY: u16 = 2;

pub const ICMP_ECHO_REQUEST: u8 = 8;
pub const ICMP_ECHO_REPLY: u8 = 0;

#[derive(Clone, Copy)]
pub struct EthHeader {
    pub dst: [u8; 6],
    pub src: [u8; 6],
    pub ethertype: u16,
}

impl EthHeader {
    pub fn from_slice(data: &[u8]) -> Self {
        let mut dst = [0u8; 6];
        let mut src = [0u8; 6];
        dst.copy_from_slice(&data[0..6]);
        src.copy_from_slice(&data[6..12]);
        Self { dst, src, ethertype: 0 }
    }
}

#[derive(Clone, Copy)]
pub struct IpHeader {
    pub ver_ihl: u8,
    pub tos: u8,
    pub len: u16,
    pub ttl: u8,
    pub proto: u8,
    pub checksum: u16,
    pub src: [u8; 4],
    pub dst: [u8; 4],
}

impl IpHeader {
    pub fn from_slice(data: &[u8]) -> Self {
        let mut src = [0u8; 4];
        let mut dst = [0u8; 4];
        src.copy_from_slice(&data[12..16]);
        dst.copy_from_slice(&data[16..20]);
        let len = u16::from_be_bytes([data[2], data[3]]);
        Self {
            ver_ihl: data[0],
            tos: data[1],
            len,
            ttl: data[8],
            proto: data[9],
            checksum: u16::from_be_bytes([data[10], data[11]]),
            src,
            dst,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ArpHeader {
    pub oper: u16,
    pub sha: [u8; 6],
    pub spa: [u8; 4],
    pub tha: [u8; 6],
    pub tpa: [u8; 4],
}

impl ArpHeader {
    pub fn from_slice(data: &[u8]) -> Self {
        let mut sha = [0u8; 6];
        let mut spa = [0u8; 4];
        let mut tha = [0u8; 6];
        let mut tpa = [0u8; 4];
        sha.copy_from_slice(&data[8..14]);
        spa.copy_from_slice(&data[14..18]);
        tha.copy_from_slice(&data[18..24]);
        tpa.copy_from_slice(&data[24..28]);
        Self {
            oper: u16::from_be_bytes([data[6], data[7]]),
            sha, spa, tha, tpa,
        }
    }
}

#[derive(Clone, Copy)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub len: u16,
    pub checksum: u16,
}

impl UdpHeader {
    pub fn from_slice(data: &[u8]) -> Self {
        Self {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            len: u16::from_be_bytes([data[4], data[5]]),
            checksum: u16::from_be_bytes([data[6], data[7]]),
        }
    }
}

#[derive(Clone, Copy)]
pub struct IpAddr([u8; 4]);

impl IpAddr {
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }

    pub fn from_slice(data: &[u8]) -> Self {
        Self([data[0], data[1], data[2], data[3]])
    }

    pub const fn zero() -> Self {
        Self([0, 0, 0, 0])
    }

    pub const fn loopback() -> Self {
        Self([127, 0, 0, 1])
    }

    pub fn as_u32(&self) -> u32 {
        (self.0[0] as u32) << 24 | (self.0[1] as u32) << 16 | (self.0[2] as u32) << 8 | (self.0[3] as u32)
    }

    pub fn is_private(&self) -> bool {
        self.0[0] == 10 || (self.0[0] == 172 && self.0[1] >= 16 && self.0[1] < 32) || (self.0[0] == 192 && self.0[1] == 168)
    }
}

pub struct IpStack {
    pub ip: IpAddr,
    pub netmask: IpAddr,
    pub gateway: IpAddr,
    pub mac: [u8; 6],
}

impl IpStack {
    pub const fn new() -> Self {
        Self { ip: IpAddr::zero(), netmask: IpAddr::new(255, 255, 255, 0), gateway: IpAddr::zero(), mac: [0; 6] }
    }

    pub fn configure(&mut self, ip: IpAddr, netmask: IpAddr, gateway: IpAddr, mac: [u8; 6]) {
        self.ip = ip;
        self.netmask = netmask;
        self.gateway = gateway;
        self.mac = mac;
    }

    pub fn route(&self, dst: IpAddr) -> IpAddr {
        if dst.0[0] == self.ip.0[0] {
            dst
        } else {
            self.gateway
        }
    }
}