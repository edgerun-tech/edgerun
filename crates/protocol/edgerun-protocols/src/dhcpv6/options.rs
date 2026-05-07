//! DHCPv6 options — RFC 8415 §21.

use super::io;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::net::Ipv6Addr;
use edgerun_encoding::byteorder::{push_u16_be, push_u32_be, read_u16_be, read_u32_be};

// ---------------------------------------------------------------------------
// Option codes
// ---------------------------------------------------------------------------

pub const OPT_CLIENTID: u16 = 1;
pub const OPT_SERVERID: u16 = 2;
pub const OPT_IA_NA: u16 = 3;
pub const OPT_IA_TA: u16 = 4;
pub const OPT_IAADDR: u16 = 5;
pub const OPT_ORO: u16 = 6;
pub const OPT_PREFERENCE: u16 = 7;
pub const OPT_ELAPSED_TIME: u16 = 8;
pub const OPT_RELAY_MSG: u16 = 9;
pub const OPT_AUTH: u16 = 11;
pub const OPT_UNICAST: u16 = 12;
pub const OPT_STATUS_CODE: u16 = 13;
pub const OPT_RAPID_COMMIT: u16 = 14;
pub const OPT_USER_CLASS: u16 = 15;
pub const OPT_VENDOR_CLASS: u16 = 16;
pub const OPT_VENDOR_OPTS: u16 = 17;
pub const OPT_INTERFACE_ID: u16 = 18;
pub const OPT_RECONF_MSG: u16 = 19;
pub const OPT_RECONF_ACCEPT: u16 = 20;
pub const OPT_DNS_SERVERS: u16 = 23;
pub const OPT_DOMAIN_LIST: u16 = 24;
pub const OPT_IA_PD: u16 = 25;
pub const OPT_IAPREFIX: u16 = 26;
pub const OPT_NIS_SERVERS: u16 = 27;
pub const OPT_NTP_SERVER: u16 = 31;
pub const OPT_FQDN: u16 = 39;
pub const OPT_SNTP_SERVERS: u16 = 31; // Same as NTP in practice

// ---------------------------------------------------------------------------
// DHCPv6 Option
// ---------------------------------------------------------------------------

/// A single DHCPv6 option.
/// Wire format: option-code (2 bytes) + option-len (2 bytes) + option-data.
#[derive(Debug, Clone)]
pub struct Dhcpv6Option {
    pub code: u16,
    pub data: Vec<u8>,
}

impl Dhcpv6Option {
    /// Create an option from raw code and data.
    pub fn from_raw(code: u16, data: Vec<u8>) -> Self {
        Self { code, data }
    }

    /// Create an IA_NA option with sub-options.
    /// IAID(4) + T1(4) + T2(4) + [sub-options]
    pub fn ia_na(iaid: u32, t1: u32, t2: u32, sub_options: Vec<Dhcpv6Option>) -> Self {
        let mut data = Vec::new();
        push_u32_be(&mut data, iaid);
        push_u32_be(&mut data, t1);
        push_u32_be(&mut data, t2);
        for sub in sub_options {
            sub.to_wire(&mut data);
        }
        Self {
            code: OPT_IA_NA,
            data,
        }
    }

    /// Create an IA Address sub-option.
    /// IPv6 address(16) + preferred-lifetime(4) + valid-lifetime(4) + [sub-options]
    pub fn iaaddr(
        addr: Ipv6Addr,
        preferred_lifetime: u32,
        valid_lifetime: u32,
        sub_options: Vec<Dhcpv6Option>,
    ) -> Self {
        let mut data = Vec::new();
        data.extend_from_slice(&addr.octets());
        push_u32_be(&mut data, preferred_lifetime);
        push_u32_be(&mut data, valid_lifetime);
        for sub in sub_options {
            sub.to_wire(&mut data);
        }
        Self {
            code: OPT_IAADDR,
            data,
        }
    }

    /// Create an IA_PD option.
    /// IAID(4) + T1(4) + T2(4) + [IAPREFIX sub-options]
    pub fn ia_pd(iaid: u32, t1: u32, t2: u32, sub_options: Vec<Dhcpv6Option>) -> Self {
        let mut data = Vec::new();
        push_u32_be(&mut data, iaid);
        push_u32_be(&mut data, t1);
        push_u32_be(&mut data, t2);
        for sub in sub_options {
            sub.to_wire(&mut data);
        }
        Self {
            code: OPT_IA_PD,
            data,
        }
    }

    /// Create an IA Prefix sub-option.
    /// preferred-lifetime(4) + valid-lifetime(4) + prefix-len(1) + prefix(16) + [sub-options]
    pub fn iaprefix(
        preferred_lifetime: u32,
        valid_lifetime: u32,
        prefix_len: u8,
        prefix: Ipv6Addr,
        sub_options: Vec<Dhcpv6Option>,
    ) -> Self {
        let mut data = Vec::new();
        push_u32_be(&mut data, preferred_lifetime);
        push_u32_be(&mut data, valid_lifetime);
        data.push(prefix_len);
        data.extend_from_slice(&prefix.octets());
        for sub in sub_options {
            sub.to_wire(&mut data);
        }
        Self {
            code: OPT_IAPREFIX,
            data,
        }
    }

    /// Create a Status Code option.
    pub fn status_code(status: StatusCode, message: &str) -> Self {
        let mut data = Vec::new();
        push_u16_be(&mut data, status as u16);
        data.extend_from_slice(message.as_bytes());
        Self {
            code: OPT_STATUS_CODE,
            data,
        }
    }

    /// Create a DNS Servers option.
    pub fn dns_servers(servers: &[Ipv6Addr]) -> Self {
        let mut data = Vec::new();
        for s in servers {
            data.extend_from_slice(&s.octets());
        }
        Self {
            code: OPT_DNS_SERVERS,
            data,
        }
    }

    /// Create a Domain List option (DNS wire format encoding).
    pub fn domain_list(domains: &[&str]) -> Self {
        let mut data = Vec::new();
        for domain in domains {
            for label in domain.split('.') {
                if !label.is_empty() {
                    data.push(label.len() as u8);
                    data.extend_from_slice(label.as_bytes());
                }
            }
            data.push(0); // root
        }
        Self {
            code: OPT_DOMAIN_LIST,
            data,
        }
    }

    /// Create an Elapsed Time option (in centiseconds, per RFC 8415).
    /// Note: DHCPv6 uses centiseconds (1/100s), not milliseconds.
    pub fn elapsed_time(cs: u16) -> Self {
        let mut data = Vec::with_capacity(2);
        push_u16_be(&mut data, cs);
        Self {
            code: OPT_ELAPSED_TIME,
            data,
        }
    }

    /// Parse options from wire format.
    pub fn parse_all(data: &[u8]) -> Result<Vec<Self>, io::Error> {
        let mut options = Vec::new();
        let mut pos = 0;

        while pos + 4 <= data.len() {
            let code = read_u16_be(data, pos);
            let len = read_u16_be(data, pos + 2) as usize;
            pos += 4;

            if pos + len > data.len() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Truncated DHCPv6 option",
                ));
            }

            options.push(Self {
                code,
                data: data[pos..pos + len].to_vec(),
            });
            pos += len;
        }

        Ok(options)
    }

    /// Serialize this option to wire format.
    pub fn to_wire(&self, buf: &mut Vec<u8>) {
        push_u16_be(buf, self.code);
        push_u16_be(buf, self.data.len() as u16);
        buf.extend_from_slice(&self.data);
    }

    /// Get IAID from an IA_NA or IA_PD option.
    pub fn iaid(&self) -> Option<u32> {
        if (self.code == OPT_IA_NA || self.code == OPT_IA_PD) && self.data.len() >= 12 {
            Some(read_u32_be(&self.data, 0))
        } else {
            None
        }
    }

    /// Get T1 from an IA_NA or IA_PD option.
    pub fn t1(&self) -> Option<u32> {
        if (self.code == OPT_IA_NA || self.code == OPT_IA_PD) && self.data.len() >= 12 {
            Some(read_u32_be(&self.data, 4))
        } else {
            None
        }
    }

    /// Get T2 from an IA_NA or IA_PD option.
    pub fn t2(&self) -> Option<u32> {
        if (self.code == OPT_IA_NA || self.code == OPT_IA_PD) && self.data.len() >= 12 {
            Some(read_u32_be(&self.data, 8))
        } else {
            None
        }
    }

    /// Parse sub-options from an IA_NA, IA_PD, or IAADDR option.
    pub fn sub_options(&self) -> Result<Vec<Self>, io::Error> {
        let offset = match self.code {
            OPT_IA_NA | OPT_IA_PD => 12,
            OPT_IAADDR => 24,
            OPT_IAPREFIX => 25,
            _ => 0,
        };
        if offset >= self.data.len() {
            return Ok(vec![]);
        }
        Self::parse_all(&self.data[offset..])
    }

    /// Get the IPv6 address from an IAADDR option.
    pub fn address(&self) -> Option<Ipv6Addr> {
        if self.code == OPT_IAADDR && self.data.len() >= 16 {
            let octets: [u8; 16] = self.data[0..16].try_into().ok()?;
            Some(Ipv6Addr::from(octets))
        } else {
            None
        }
    }

    /// Get preferred lifetime from IAADDR or IAPREFIX.
    pub fn preferred_lifetime(&self) -> Option<u32> {
        match self.code {
            OPT_IAADDR if self.data.len() >= 20 => Some(read_u32_be(&self.data, 16)),
            OPT_IAPREFIX if self.data.len() >= 8 => Some(read_u32_be(&self.data, 0)),
            _ => None,
        }
    }

    /// Get valid lifetime from IAADDR or IAPREFIX.
    pub fn valid_lifetime(&self) -> Option<u32> {
        match self.code {
            OPT_IAADDR if self.data.len() >= 24 => Some(read_u32_be(&self.data, 20)),
            OPT_IAPREFIX if self.data.len() >= 12 => Some(read_u32_be(&self.data, 4)),
            _ => None,
        }
    }

    /// Get the prefix from an IAPREFIX option.
    pub fn prefix(&self) -> Option<(u8, Ipv6Addr)> {
        if self.code == OPT_IAPREFIX && self.data.len() >= 21 {
            let prefix_len = self.data[8];
            let octets: [u8; 16] = self.data[9..25].try_into().ok()?;
            Some((prefix_len, Ipv6Addr::from(octets)))
        } else {
            None
        }
    }

    /// Get status code and message.
    pub fn status(&self) -> Option<(StatusCode, String)> {
        if self.code == OPT_STATUS_CODE && self.data.len() >= 2 {
            let code = read_u16_be(&self.data, 0);
            let msg = String::from_utf8_lossy(&self.data[2..]).to_string();
            Some((StatusCode::from_u16(code), msg))
        } else {
            None
        }
    }
}

/// DHCPv6 Status Codes — RFC 8415 §21.13.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum StatusCode {
    Success = 0,
    UnspecFail = 1,
    NoAddrsAvail = 2,
    NoBinding = 3,
    NotOnLink = 4,
    UseMulticast = 5,
    NoPrefixAvail = 6,
    UnknownQueryType = 7,
    MalformedQuery = 8,
    NotConfigured = 9,
    NotAllowed = 10,
}

impl StatusCode {
    pub fn from_u16(v: u16) -> Self {
        match v {
            0 => Self::Success,
            1 => Self::UnspecFail,
            2 => Self::NoAddrsAvail,
            3 => Self::NoBinding,
            4 => Self::NotOnLink,
            5 => Self::UseMulticast,
            6 => Self::NoPrefixAvail,
            7 => Self::UnknownQueryType,
            8 => Self::MalformedQuery,
            9 => Self::NotConfigured,
            10 => Self::NotAllowed,
            _ => Self::UnspecFail,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Success => "Success",
            Self::UnspecFail => "UnspecFail",
            Self::NoAddrsAvail => "NoAddrsAvail",
            Self::NoBinding => "NoBinding",
            Self::NotOnLink => "NotOnLink",
            Self::UseMulticast => "UseMulticast",
            Self::NoPrefixAvail => "NoPrefixAvail",
            Self::UnknownQueryType => "UnknownQueryType",
            Self::MalformedQuery => "MalformedQuery",
            Self::NotConfigured => "NotConfigured",
            Self::NotAllowed => "NotAllowed",
        }
    }
}

// Type aliases for readability
pub type IaNaOption = Dhcpv6Option;
pub type IaTaOption = Dhcpv6Option;
pub type IaPdOption = Dhcpv6Option;

#[cfg(test)]
mod tests {
    use super::*;
    use core::net::Ipv6Addr;

    #[test]
    fn test_ia_na_roundtrip() {
        let sub = Dhcpv6Option::iaaddr(
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
            3600,
            7200,
            vec![],
        );
        let opt = Dhcpv6Option::ia_na(0x12345678, 1800, 2700, vec![sub]);

        assert_eq!(opt.iaid(), Some(0x12345678));
        assert_eq!(opt.t1(), Some(1800));
        assert_eq!(opt.t2(), Some(2700));

        let subs = opt.sub_options().unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(
            subs[0].address(),
            Some(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1))
        );
        assert_eq!(subs[0].preferred_lifetime(), Some(3600));
    }

    #[test]
    fn test_dns_servers_roundtrip() {
        let servers = vec![
            Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888),
            Ipv6Addr::new(0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8844),
        ];
        let opt = Dhcpv6Option::dns_servers(&servers);

        let mut buf = Vec::new();
        opt.to_wire(&mut buf);
        let parsed = Dhcpv6Option::parse_all(&buf).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].code, OPT_DNS_SERVERS);
    }

    #[test]
    fn test_domain_list_encoding() {
        let opt = Dhcpv6Option::domain_list(&["example.com", "test.local"]);
        // "example.com" → 7example3com0
        assert!(opt.data.len() > 0);
    }

    #[test]
    fn test_status_code() {
        let opt = Dhcpv6Option::status_code(StatusCode::NoAddrsAvail, "Pool exhausted");
        let (code, msg) = opt.status().unwrap();
        assert_eq!(code, StatusCode::NoAddrsAvail);
        assert_eq!(msg, "Pool exhausted");
    }

    #[test]
    fn test_elapsed_time() {
        let opt = Dhcpv6Option::elapsed_time(500); // 5 seconds
        assert_eq!(opt.data.len(), 2);
        assert_eq!(opt.code, OPT_ELAPSED_TIME);
    }

    #[test]
    fn test_ia_pd_roundtrip() {
        let prefix_sub = Dhcpv6Option::iaprefix(
            3600,
            7200,
            64,
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
            vec![],
        );
        let opt = Dhcpv6Option::ia_pd(1, 1800, 2700, vec![prefix_sub]);

        assert_eq!(opt.iaid(), Some(1));
        let subs = opt.sub_options().unwrap();
        assert_eq!(subs.len(), 1);
        let (len, prefix) = subs[0].prefix().unwrap();
        assert_eq!(len, 64);
        assert_eq!(prefix, Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0));
    }

    #[test]
    fn test_parse_multiple_options() {
        let client_id = Dhcpv6Option::from_raw(OPT_CLIENTID, vec![0, 3, 0, 1, 0xaa, 0xbb]);
        let oro = Dhcpv6Option::from_raw(OPT_ORO, vec![23, 24]);

        let mut buf = Vec::new();
        client_id.to_wire(&mut buf);
        oro.to_wire(&mut buf);

        let parsed = Dhcpv6Option::parse_all(&buf).unwrap();
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].code, OPT_CLIENTID);
        assert_eq!(parsed[1].code, OPT_ORO);
    }
}
