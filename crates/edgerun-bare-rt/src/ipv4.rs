//! IPv4 packet handling



pub const IP_VERSION: u8 = 4;
pub const IP_DEFAULT_TTL: u8 = 64;

pub const ICMP: u8 = 1;
pub const TCP_PROTO: u8 = 6;
pub const UDP_PROTO: u8 = 17;

#[derive(Debug, Clone, Copy, Default)]
pub struct Ipv4Addr(pub u32);

impl Ipv4Addr {
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self(((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | (d as u32))
    }

    pub const LOCALHOST: Self = Self(0x7F000001);
    pub const UNSPECIFIED: Self = Self(0);
    pub const BROADCAST: Self = Self(0xFFFFFFFF);

    pub fn from_bytes(&self) -> [u8; 4] {
        [(self.0 >> 24) as u8, (self.0 >> 16) as u8, (self.0 >> 8) as u8, self.0 as u8]
    }

    pub fn octets(&self) -> [u8; 4] {
        self.from_bytes()
    }

    pub fn is_loopback(&self) -> bool {
        (self.0 >> 24) == 0x7F
    }

    pub fn is_unspecified(&self) -> bool {
        self.0 == 0
    }

    pub fn is_broadcast(&self) -> bool {
        self.0 == 0xFFFFFFFF
    }
}

impl core::fmt::Display for Ipv4Addr {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let b = self.from_bytes();
        write!(f, "{}.{}.{}.{}", b[0], b[1], b[2], b[3])
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Ipv4Header {
    pub ver_ihl: u8,
    pub tos: u8,
    pub len: u16,
    pub id: u16,
    pub flags_offset: u16,
    pub ttl: u8,
    pub proto: u8,
    pub checksum: u16,
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,
}

impl Ipv4Header {
    pub fn new(src: Ipv4Addr, dst: Ipv4Addr, proto: u8) -> Self {
        Self {
            ver_ihl: (IP_VERSION << 4) | 5,
            tos: 0,
            len: 20,
            id: 0,
            flags_offset: 0,
            ttl: IP_DEFAULT_TTL,
            proto,
            checksum: 0,
            src,
            dst,
        }
    }

    pub fn checksum(&mut self) -> u16 {
        let mut sum: u32 = 0;
        let data = self as *const _ as *const u8;
        unsafe {
            for i in 0..20 {
                let byte = data.add(i).read_volatile() as u32;
                if i % 2 == 0 {
                    sum += byte << 8;
                } else {
                    sum += byte;
                }
            }
        }
        while sum >> 16 != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        !(sum as u16)
    }
}