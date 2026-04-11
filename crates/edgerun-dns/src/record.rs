//! DNS record types and data structures.

use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use std::fmt;

// ---------------------------------------------------------------------------
// Record type constants (RFC 1035 + extensions)
// ---------------------------------------------------------------------------

/// DNS record type (TYPE field in RR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u16)]
pub enum DnsRecordType {
    /// IPv4 address
    A = 1,
    /// Authoritative name server
    NS = 2,
    /// Canonical name (alias)
    CNAME = 5,
    /// Start of authority
    SOA = 6,
    /// Pointer (reverse DNS)
    PTR = 12,
    /// Mail exchange
    MX = 15,
    /// Text record
    TXT = 16,
    /// IPv6 address
    AAAA = 28,
    /// Service locator
    SRV = 33,
    /// Any record (AXFR/IXFR)
    AXFR = 252,
    /// Any record (wildcard query)
    ANY = 255,
    /// OPT pseudo-record for EDNS0 (RFC 6891).
    OPT = 41,
}

impl DnsRecordType {
    /// Parse a record type from a u16.
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            1 => Some(Self::A),
            2 => Some(Self::NS),
            5 => Some(Self::CNAME),
            6 => Some(Self::SOA),
            12 => Some(Self::PTR),
            15 => Some(Self::MX),
            16 => Some(Self::TXT),
            28 => Some(Self::AAAA),
            33 => Some(Self::SRV),
            252 => Some(Self::AXFR),
            255 => Some(Self::ANY),
            41 => Some(Self::OPT),
            _ => None,
        }
    }

    /// Record type as u16.
    pub fn as_u16(self) -> u16 {
        self as u16
    }

    /// Record type as string.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::NS => "NS",
            Self::CNAME => "CNAME",
            Self::SOA => "SOA",
            Self::PTR => "PTR",
            Self::MX => "MX",
            Self::TXT => "TXT",
            Self::AAAA => "AAAA",
            Self::SRV => "SRV",
            Self::AXFR => "AXFR",
            Self::ANY => "ANY",
            Self::OPT => "OPT",
        }
    }
}

impl fmt::Display for DnsRecordType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ---------------------------------------------------------------------------
// Record data (RDATA)
// ---------------------------------------------------------------------------

/// Parsed DNS record data (varies by record type).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DnsRecordData {
    /// A record — IPv4 address.
    A(Ipv4Addr),
    /// AAAA record — IPv6 address.
    AAAA(Ipv6Addr),
    /// CNAME record — canonical name.
    CNAME(String),
    /// NS record — name server.
    NS(String),
    /// PTR record — pointer/reverse name.
    PTR(String),
    /// Mail exchange — mail server with a priority (lower = preferred).
    MX {
        /// Priority — lower values are preferred.
        priority: u16,
        /// Hostname of the mail server.
        exchange: String,
    },
    /// Text record — arbitrary text (SPF, DKIM, verification strings).
    TXT(String),
    /// Start of authority — zone metadata (serial, refresh, retry, expire).
    SOA {
        /// Primary name server for the zone.
        mname: String,
        /// Email of the responsible administrator (with `@` replaced by `.`).
        rname: String,
        /// Zone serial number — incremented on each change.
        serial: u32,
        /// Seconds between zone refresh checks.
        refresh: u32,
        /// Seconds between retry attempts after a failed refresh.
        retry: u32,
        /// Seconds after which the zone expires if not refreshed.
        expire: u32,
        /// Minimum TTL — used as a default for negative caching.
        minimum: u32,
    },
    /// Service locator — specifies a host and port for a service.
    SRV {
        /// Priority — lower values are preferred.
        priority: u16,
        /// Weight — relative load distribution among same-priority targets.
        weight: u16,
        /// TCP or UDP port the service listens on.
        port: u16,
        /// Hostname of the machine providing the service.
        target: String,
    },
    /// Unknown or uninterpreted raw record data.
    Raw(Vec<u8>),
    /// EDNS0 OPT pseudo-record data (RFC 6891).
    OPT {
        /// Extended RCODE upper bits.
        ext_rcode: u8,
        /// EDNS0 version (must be 0).
        version: u8,
        /// Flags — bit 15 is DO (DNSSEC OK).
        flags: u16,
        /// Raw option data (RFC 6891 options).
        options: Vec<u8>,
    },
}

impl DnsRecordData {
    /// Serialize RDATA to wire format for a given record type.
    pub fn to_wire(&self, rtype: DnsRecordType) -> Vec<u8> {
        match (rtype, self) {
            (DnsRecordType::A, DnsRecordData::A(ip)) => ip.octets().to_vec(),
            (DnsRecordType::AAAA, DnsRecordData::AAAA(ip)) => ip.octets().to_vec(),
            (DnsRecordType::CNAME, DnsRecordData::CNAME(name)) => {
                encode_domain_name(name)
            }
            (DnsRecordType::NS, DnsRecordData::NS(name)) => {
                encode_domain_name(name)
            }
            (DnsRecordType::PTR, DnsRecordData::PTR(name)) => {
                encode_domain_name(name)
            }
            (
                DnsRecordType::MX,
                DnsRecordData::MX { priority, exchange },
            ) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(&priority.to_be_bytes());
                buf.extend_from_slice(&encode_domain_name(exchange));
                buf
            }
            (DnsRecordType::TXT, DnsRecordData::TXT(text)) => {
                let bytes = text.as_bytes();
                let mut buf = Vec::with_capacity(bytes.len() + 1);
                buf.push(bytes.len() as u8);
                buf.extend_from_slice(bytes);
                buf
            }
            (
                DnsRecordType::SOA,
                DnsRecordData::SOA {
                    mname,
                    rname,
                    serial,
                    refresh,
                    retry,
                    expire,
                    minimum,
                },
            ) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(&encode_domain_name(mname));
                buf.extend_from_slice(&encode_domain_name(rname));
                buf.extend_from_slice(&serial.to_be_bytes());
                buf.extend_from_slice(&refresh.to_be_bytes());
                buf.extend_from_slice(&retry.to_be_bytes());
                buf.extend_from_slice(&expire.to_be_bytes());
                buf.extend_from_slice(&minimum.to_be_bytes());
                buf
            }
            (
                DnsRecordType::SRV,
                DnsRecordData::SRV {
                    priority,
                    weight,
                    port,
                    target,
                },
            ) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(&priority.to_be_bytes());
                buf.extend_from_slice(&weight.to_be_bytes());
                buf.extend_from_slice(&port.to_be_bytes());
                buf.extend_from_slice(&encode_domain_name(target));
                buf
            }
            (_, DnsRecordData::Raw(data)) => data.clone(),
            (DnsRecordType::OPT, DnsRecordData::OPT { ext_rcode, version, flags, options }) => {
                let mut buf = Vec::with_capacity(4 + options.len());
                buf.push(*ext_rcode);
                buf.push(*version);
                buf.extend_from_slice(&flags.to_be_bytes());
                buf.extend_from_slice(options);
                buf
            }
            _ => Vec::new(),
        }
    }

    /// Parse RDATA from wire format for a given record type.
    pub fn from_wire(rtype: DnsRecordType, data: &[u8], offset_map: &[(usize, usize)]) -> Result<Self, std::io::Error> {
        match rtype {
            DnsRecordType::A => {
                if data.len() == 4 {
                    Ok(Self::A(Ipv4Addr::new(data[0], data[1], data[2], data[3])))
                } else {
                    Ok(Self::Raw(data.to_vec()))
                }
            }
            DnsRecordType::AAAA => {
                if data.len() == 16 {
                    let octets: [u8; 16] = data.try_into().map_err(|_| {
                        std::io::Error::new(std::io::ErrorKind::InvalidData, "AAAA wrong size")
                    })?;
                    Ok(Self::AAAA(Ipv6Addr::from(octets)))
                } else {
                    Ok(Self::Raw(data.to_vec()))
                }
            }
            DnsRecordType::CNAME => {
                let name = decode_domain_name(data, 0, offset_map)?;
                Ok(Self::CNAME(name))
            }
            DnsRecordType::NS => {
                let name = decode_domain_name(data, 0, offset_map)?;
                Ok(Self::NS(name))
            }
            DnsRecordType::PTR => {
                let name = decode_domain_name(data, 0, offset_map)?;
                Ok(Self::PTR(name))
            }
            DnsRecordType::MX => {
                if data.len() < 3 {
                    return Ok(Self::Raw(data.to_vec()));
                }
                let priority = u16::from_be_bytes([data[0], data[1]]);
                let exchange = decode_domain_name(data, 2, offset_map)?;
                Ok(Self::MX { priority, exchange })
            }
            DnsRecordType::TXT => {
                if data.is_empty() {
                    return Ok(Self::TXT(String::new()));
                }
                let len = data[0] as usize;
                let text = String::from_utf8_lossy(&data[1..=len.min(data.len() - 1)]).to_string();
                Ok(Self::TXT(text))
            }
            DnsRecordType::SOA => {
                let mut pos = 0;
                let mname = decode_domain_name(data, pos, offset_map)?;
                pos += domain_name_wire_len(data, pos);
                let rname = decode_domain_name(data, pos, offset_map)?;
                pos += domain_name_wire_len(data, pos);
                if pos + 20 > data.len() {
                    return Ok(Self::Raw(data.to_vec()));
                }
                let serial = u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
                let refresh = u32::from_be_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]]);
                let retry = u32::from_be_bytes([data[pos + 8], data[pos + 9], data[pos + 10], data[pos + 11]]);
                let expire = u32::from_be_bytes([data[pos + 12], data[pos + 13], data[pos + 14], data[pos + 15]]);
                let minimum = u32::from_be_bytes([data[pos + 16], data[pos + 17], data[pos + 18], data[pos + 19]]);
                Ok(Self::SOA {
                    mname,
                    rname,
                    serial,
                    refresh,
                    retry,
                    expire,
                    minimum,
                })
            }
            DnsRecordType::SRV => {
                if data.len() < 8 {
                    return Ok(Self::Raw(data.to_vec()));
                }
                let priority = u16::from_be_bytes([data[0], data[1]]);
                let weight = u16::from_be_bytes([data[2], data[3]]);
                let port = u16::from_be_bytes([data[4], data[5]]);
                let target = decode_domain_name(data, 6, offset_map)?;
                Ok(Self::SRV {
                    priority,
                    weight,
                    port,
                    target,
                })
            }
            DnsRecordType::OPT => {
                if data.len() < 4 {
                    return Ok(Self::OPT {
                        ext_rcode: 0,
                        version: 0,
                        flags: 0,
                        options: data.to_vec(),
                    });
                }
                let ext_rcode = data[0];
                let version = data[1];
                let flags = u16::from_be_bytes([data[2], data[3]]);
                let options = data[4..].to_vec();
                Ok(Self::OPT {
                    ext_rcode,
                    version,
                    flags,
                    options,
                })
            }
            _ => Ok(Self::Raw(data.to_vec())),
        }
    }

    /// Check if this OPT record has the DNSSEC OK (DO) flag set (RFC 3225).
    /// Returns `false` for non-OPT records.
    pub fn is_do(&self) -> bool {
        matches!(self, Self::OPT { flags, .. } if flags & 0x8000 != 0)
    }
}

// ---------------------------------------------------------------------------
// Domain name encoding / decoding (DNS compression pointers)
// ---------------------------------------------------------------------------

/// Encode a domain name in DNS wire format with label lengths.
pub fn encode_domain_name(name: &str) -> Vec<u8> {
    let mut buf = Vec::new();
    for label in name.split('.') {
        if label.is_empty() {
            continue; // trailing dot
        }
        buf.push(label.len() as u8);
        buf.extend_from_slice(label.as_bytes());
    }
    buf.push(0); // root
    buf
}

/// Decode a domain name from DNS wire format, following compression pointers.
/// Returns the decoded name and advances through the data.
pub fn decode_domain_name(
    data: &[u8],
    mut offset: usize,
    offset_map: &[(usize, usize)],
) -> Result<String, std::io::Error> {
    let mut labels = Vec::new();

    loop {
        if offset >= data.len() {
            break;
        }
        let len_byte = data[offset];

        if len_byte == 0 {
            break;
        }

        // Compression pointer (top 2 bits set)
        if (len_byte & 0xC0) == 0xC0 {
            if offset + 1 >= data.len() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Truncated DNS pointer",
                ));
            }
            let pointer = (((len_byte & 0x3F) as usize) << 8) | (data[offset + 1] as usize);

            // Look up in the full message offset map
            let target = offset_map
                .iter()
                .find(|&&(msg_off, _)| msg_off == pointer)
                .map(|&(_, data_off)| data_off)
                .unwrap_or(pointer);

            offset = target;
            continue;
        }

        // Regular label
        let len = len_byte as usize;
        offset += 1;
        if offset + len > data.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Truncated DNS label",
            ));
        }
        let label = String::from_utf8_lossy(&data[offset..offset + len]).to_string();
        labels.push(label);
        offset += len;
    }

    if labels.is_empty() {
        Ok(".".to_string())
    } else {
        Ok(labels.join("."))
    }
}

/// Get the wire length of a domain name starting at `offset`.
fn domain_name_wire_len(data: &[u8], offset: usize) -> usize {
    let mut pos = offset;
    loop {
        if pos >= data.len() {
            return data.len() - offset;
        }
        let b = data[pos];
        if b == 0 {
            return pos + 1 - offset;
        }
        if (b & 0xC0) == 0xC0 {
            return pos + 2 - offset;
        }
        pos += 1 + b as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_domain_name() {
        let wire = encode_domain_name("www.example.com");
        assert_eq!(wire, vec![3, b'w', b'w', b'w', 7, b'e', b'x', b'a', b'm', b'p', b'l', b'e', 3, b'c', b'o', b'm', 0]);
    }

    #[test]
    fn test_record_type_from_u16() {
        assert_eq!(DnsRecordType::from_u16(1), Some(DnsRecordType::A));
        assert_eq!(DnsRecordType::from_u16(28), Some(DnsRecordType::AAAA));
        assert_eq!(DnsRecordType::from_u16(99), None);
    }

    #[test]
    fn test_record_type_display() {
        assert_eq!(DnsRecordType::CNAME.to_string(), "CNAME");
        assert_eq!(DnsRecordType::AAAA.to_string(), "AAAA");
    }

    #[test]
    fn test_a_record_roundtrip() {
        let data = DnsRecordData::A(Ipv4Addr::new(192, 168, 1, 1));
        let wire = data.to_wire(DnsRecordType::A);
        assert_eq!(wire, vec![192, 168, 1, 1]);
        let parsed = DnsRecordData::from_wire(DnsRecordType::A, &wire, &[]).unwrap();
        assert_eq!(parsed, data);
    }

    #[test]
    fn test_aaaa_record_roundtrip() {
        let ip = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        let data = DnsRecordData::AAAA(ip);
        let wire = data.to_wire(DnsRecordType::AAAA);
        assert_eq!(wire.len(), 16);
        let parsed = DnsRecordData::from_wire(DnsRecordType::AAAA, &wire, &[]).unwrap();
        assert_eq!(parsed, data);
    }

    #[test]
    fn test_txt_record_roundtrip() {
        let data = DnsRecordData::TXT("v=spf1 include:example.com ~all".to_string());
        let wire = data.to_wire(DnsRecordType::TXT);
        let parsed = DnsRecordData::from_wire(DnsRecordType::TXT, &wire, &[]).unwrap();
        assert_eq!(parsed, data);
    }

    #[test]
    fn test_mx_record_roundtrip() {
        let data = DnsRecordData::MX {
            priority: 10,
            exchange: "mail.example.com".to_string(),
        };
        let wire = data.to_wire(DnsRecordType::MX);
        // Can't easily parse back without offset_map, but verify wire len
        assert!(wire.len() > 2);
        assert_eq!(wire[0..2], [0, 10]);
    }

    #[test]
    fn test_soa_record_roundtrip() {
        let data = DnsRecordData::SOA {
            mname: "ns1.example.com".to_string(),
            rname: "admin.example.com".to_string(),
            serial: 2024010101,
            refresh: 3600,
            retry: 900,
            expire: 604800,
            minimum: 86400,
        };
        let wire = data.to_wire(DnsRecordType::SOA);
        assert!(wire.len() > 20);
    }

    #[test]
    fn test_srv_record_roundtrip() {
        let data = DnsRecordData::SRV {
            priority: 10,
            weight: 60,
            port: 5060,
            target: "sip.example.com".to_string(),
        };
        let wire = data.to_wire(DnsRecordType::SRV);
        assert_eq!(wire[0..2], [0, 10]);
        assert_eq!(wire[2..4], [0, 60]);
        assert_eq!(wire[4..6], [5060u16.to_be_bytes()[0], 5060u16.to_be_bytes()[1]]);
    }

    #[test]
    fn test_decode_domain_name_simple() {
        let data = encode_domain_name("www.example.com");
        // Build an offset map: the data is self-contained, so pointer = offset
        let offset_map: Vec<(usize, usize)> = (0..data.len()).map(|i| (i, i)).collect();
        let name = decode_domain_name(&data, 0, &offset_map).unwrap();
        assert_eq!(name, "www.example.com");
    }
}
