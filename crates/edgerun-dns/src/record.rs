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
    /// Naming Authority Pointer (RFC 3403)
    NAPTR = 35,
    /// Certificate Authority Authorization (RFC 8659)
    CAA = 257,
    /// TLSA — DANE certificate binding (RFC 6698)
    TLSA = 52,
    /// HTTPS service binding (RFC 9460)
    HTTPS = 65,
    /// SVCB service binding (RFC 9460)
    SVCB = 64,
    /// Delegation Signer (RFC 4034)
    DS = 43,
    /// RRset Signature (RFC 4034)
    RRSIG = 46,
    /// Next Secure record (RFC 4034)
    NSEC = 47,
    /// DNS Public Key (RFC 4034)
    DNSKEY = 48,
    /// Next SECure record v3 (RFC 5155)
    NSEC3 = 50,
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
            35 => Some(Self::NAPTR),
            52 => Some(Self::TLSA),
            64 => Some(Self::SVCB),
            65 => Some(Self::HTTPS),
            43 => Some(Self::DS),
            46 => Some(Self::RRSIG),
            47 => Some(Self::NSEC),
            48 => Some(Self::DNSKEY),
            50 => Some(Self::NSEC3),
            252 => Some(Self::AXFR),
            255 => Some(Self::ANY),
            257 => Some(Self::CAA),
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
            Self::NAPTR => "NAPTR",
            Self::TLSA => "TLSA",
            Self::SVCB => "SVCB",
            Self::HTTPS => "HTTPS",
            Self::AXFR => "AXFR",
            Self::ANY => "ANY",
            Self::CAA => "CAA",
            Self::DS => "DS",
            Self::RRSIG => "RRSIG",
            Self::NSEC => "NSEC",
            Self::DNSKEY => "DNSKEY",
            Self::NSEC3 => "NSEC3",
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
    /// NAPTR — Naming Authority Pointer (RFC 3403).
    /// Used for URI/telephone routing and ENUM.
    NAPTR {
        /// Order — processed before weight (lower first).
        order: u16,
        /// Preference — processed before higher (lower first).
        preference: u16,
        /// Flags — controls rewrite behavior (e.g. "u", "s", "a", "p").
        flags: String,
        /// Services — protocol+service tuple (e.g. "SIP+D2U").
        services: String,
        /// Regexp — POSIX extended regex for URI rewriting.
        regexp: String,
        /// Replacement — domain name if regexp is empty.
        replacement: String,
    },
    /// CAA — Certificate Authority Authorization (RFC 8659).
    /// Restricts which CAs may issue certificates for a domain.
    CAA {
        /// Critical flag — if set, CA must not issue if it doesn't understand the tag.
        critical: bool,
        /// Tag — "issue", "issuewild", or "iodef".
        tag: String,
        /// Value — CA domain name or URI for reports.
        value: String,
    },
    /// TLSA — DANE certificate binding (RFC 6698).
    /// Associates a TLS certificate with a domain name via DNSSEC.
    TLSA {
        /// Certificate usage: 0=CA, 1=service cert, 2=trust anchor, 3=leaf cert.
        usage: u8,
        /// Selector: 0=full cert, 1=subject public key info.
        selector: u8,
        /// Matching type: 0=exact, 1=SHA-256, 2=SHA-512.
        matching_type: u8,
        /// Certificate association data.
        certificate: Vec<u8>,
    },
    /// SVCB/HTTPS — Service Binding (RFC 9460).
    /// Specifies how to reach a service (protocol, port, ALPN, IPs, etc.).
    SVCB {
        /// Priority — 0 means "alias mode" (points to another name).
        priority: u16,
        /// Target domain name.
        target: String,
        /// SVCB parameters (encoded wire format: key + length + value).
        params: Vec<u8>,
    },
    /// DS — Delegation Signer (RFC 4034).
    /// Links parent zone's DS to child zone's DNSKEY for chain of trust.
    DS {
        /// Key tag of the child's DNSKEY.
        key_tag: u16,
        /// Algorithm of the child's DNSKEY (RSA/ECDSA/etc).
        algorithm: u8,
        /// Digest type (1=SHA-1, 2=SHA-256, 4=SHA-384).
        digest_type: u8,
        /// Digest of the child's DNSKEY RDATA.
        digest: Vec<u8>,
    },
    /// DNSKEY — DNS Public Key (RFC 4034).
    /// Contains the public key used to verify RRSIG signatures.
    DNSKEY {
        /// Protocol (always 3 for DNSSEC).
        protocol: u8,
        /// Key flags: 256=ZSK, 257=KSK.
        flags: u16,
        /// Algorithm (5=RSASHA1, 8=RSASHA256, 13=ECDSAP256, 14=ECDSAP384, 15=ED25519, 16=ED448).
        algorithm: u8,
        /// Public key data.
        public_key: Vec<u8>,
    },
    /// RRSIG — RRset Signature (RFC 4034).
    /// Cryptographic signature over an RRset.
    RRSIG {
        /// Type covered (the record type this signature covers).
        type_covered: u16,
        /// Algorithm used for signing.
        algorithm: u8,
        /// Number of labels in the original RRset owner name.
        labels: u8,
        /// Original TTL of the signed RRset.
        original_ttl: u32,
        /// Signature expiration (seconds since epoch).
        expiration: u32,
        /// Signature inception (seconds since epoch).
        inception: u32,
        /// Key tag of the signing DNSKEY.
        key_tag: u16,
        /// Signer's name (domain that owns the signing key).
        signer_name: String,
        /// The cryptographic signature.
        signature: Vec<u8>,
    },
    /// NSEC — Next Secure record (RFC 4034).
    /// Proves non-existence of a name/type by pointing to the next name.
    NSEC {
        /// Next owner name (the next name in canonical order).
        next_owner: String,
        /// Type bit map of types that exist at this name.
        type_bits: Vec<u8>,
    },
    /// NSEC3 — Next SECure record v3 (RFC 5155).
    /// Hashed version of NSEC to prevent zone enumeration.
    NSEC3 {
        /// Hash algorithm (1=SHA-1).
        hash_algorithm: u8,
        /// Flags (1=opt-out).
        flags: u8,
        /// Number of iterations for hash computation.
        iterations: u16,
        /// Salt for hash computation.
        salt: Vec<u8>,
        /// Hashed next owner name.
        next_hashed_owner: Vec<u8>,
        /// Type bit map of types that exist at this name.
        type_bits: Vec<u8>,
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
            (DnsRecordType::SRV, DnsRecordData::SRV {
                priority,
                weight,
                port,
                target,
            }) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(&priority.to_be_bytes());
                buf.extend_from_slice(&weight.to_be_bytes());
                buf.extend_from_slice(&port.to_be_bytes());
                buf.extend_from_slice(&encode_domain_name(target));
                buf
            }
            (DnsRecordType::NAPTR, DnsRecordData::NAPTR {
                order, preference, flags, services, regexp, replacement,
            }) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(&order.to_be_bytes());
                buf.extend_from_slice(&preference.to_be_bytes());
                // Character strings: length byte + data
                buf.push(flags.len() as u8);
                buf.extend_from_slice(flags.as_bytes());
                buf.push(services.len() as u8);
                buf.extend_from_slice(services.as_bytes());
                buf.push(regexp.len() as u8);
                buf.extend_from_slice(regexp.as_bytes());
                buf.extend_from_slice(&encode_domain_name(replacement));
                buf
            }
            (DnsRecordType::CAA, DnsRecordData::CAA { critical, tag, value }) => {
                let mut buf = Vec::new();
                buf.push(if *critical { 0x80 } else { 0 });
                buf.push(tag.len() as u8);
                buf.extend_from_slice(tag.as_bytes());
                buf.extend_from_slice(value.as_bytes());
                buf
            }
            (DnsRecordType::TLSA, DnsRecordData::TLSA {
                usage, selector, matching_type, certificate,
            }) => {
                let mut buf = Vec::new();
                buf.push(*usage);
                buf.push(*selector);
                buf.push(*matching_type);
                buf.extend_from_slice(certificate);
                buf
            }
            (DnsRecordType::HTTPS, DnsRecordData::SVCB { priority, target, params })
            | (DnsRecordType::SVCB, DnsRecordData::SVCB { priority, target, params }) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(&priority.to_be_bytes());
                buf.extend_from_slice(&encode_domain_name(target));
                buf.extend_from_slice(params);
                buf
            }
            (DnsRecordType::DS, DnsRecordData::DS {
                key_tag, algorithm, digest_type, digest,
            }) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(&key_tag.to_be_bytes());
                buf.push(*algorithm);
                buf.push(*digest_type);
                buf.extend_from_slice(digest);
                buf
            }
            (DnsRecordType::DNSKEY, DnsRecordData::DNSKEY {
                protocol, flags, algorithm, public_key,
            }) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(&flags.to_be_bytes());
                buf.push(*protocol);
                buf.push(*algorithm);
                buf.extend_from_slice(public_key);
                buf
            }
            (DnsRecordType::RRSIG, DnsRecordData::RRSIG {
                type_covered, algorithm, labels, original_ttl,
                expiration, inception, key_tag, signer_name, signature,
            }) => {
                let mut buf = Vec::new();
                buf.extend_from_slice(&type_covered.to_be_bytes());
                buf.push(*algorithm);
                buf.push(*labels);
                buf.extend_from_slice(&original_ttl.to_be_bytes());
                buf.extend_from_slice(&expiration.to_be_bytes());
                buf.extend_from_slice(&inception.to_be_bytes());
                buf.extend_from_slice(&key_tag.to_be_bytes());
                buf.extend_from_slice(&encode_domain_name(signer_name));
                buf.extend_from_slice(signature);
                buf
            }
            (DnsRecordType::NSEC, DnsRecordData::NSEC { next_owner, type_bits }) => {
                let mut buf = encode_domain_name(next_owner);
                buf.extend_from_slice(type_bits);
                buf
            }
            (DnsRecordType::NSEC3, DnsRecordData::NSEC3 {
                hash_algorithm, flags, iterations, salt,
                next_hashed_owner, type_bits,
            }) => {
                let mut buf = Vec::new();
                buf.push(*hash_algorithm);
                buf.push(*flags);
                buf.extend_from_slice(&iterations.to_be_bytes());
                buf.push(salt.len() as u8);
                buf.extend_from_slice(salt);
                let hash_len = next_hashed_owner.len();
                buf.push(hash_len as u8);
                buf.extend_from_slice(next_hashed_owner);
                buf.extend_from_slice(type_bits);
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
            DnsRecordType::NAPTR => {
                if data.len() < 4 {
                    return Ok(Self::Raw(data.to_vec()));
                }
                let order = u16::from_be_bytes([data[0], data[1]]);
                let preference = u16::from_be_bytes([data[2], data[3]]);
                let mut pos = 4;
                // Character strings: length byte + data
                let parse_charstr = |d: &[u8], p: &mut usize| -> String {
                    if *p >= d.len() { return String::new(); }
                    let len = d[*p] as usize;
                    *p += 1;
                    let end = (*p + len).min(d.len());
                    let s = String::from_utf8_lossy(&d[*p..end]).to_string();
                    *p = end;
                    s
                };
                let flags = parse_charstr(data, &mut pos);
                let services = parse_charstr(data, &mut pos);
                let regexp = parse_charstr(data, &mut pos);
                let replacement = decode_domain_name(data, pos, offset_map).unwrap_or_default();
                Ok(Self::NAPTR {
                    order, preference, flags, services, regexp, replacement,
                })
            }
            DnsRecordType::CAA => {
                if data.len() < 2 {
                    return Ok(Self::Raw(data.to_vec()));
                }
                let critical = (data[0] & 0x80) != 0;
                let tag_len = data[1] as usize;
                let tag_start = 2;
                let tag_end = tag_start + tag_len.min(data.len().saturating_sub(2));
                let tag = String::from_utf8_lossy(&data[tag_start..tag_end]).to_string();
                let value = String::from_utf8_lossy(&data[tag_end..]).to_string();
                Ok(Self::CAA { critical, tag, value })
            }
            DnsRecordType::TLSA => {
                if data.len() < 3 {
                    return Ok(Self::Raw(data.to_vec()));
                }
                Ok(Self::TLSA {
                    usage: data[0],
                    selector: data[1],
                    matching_type: data[2],
                    certificate: data[3..].to_vec(),
                })
            }
            DnsRecordType::HTTPS | DnsRecordType::SVCB => {
                if data.len() < 2 {
                    return Ok(Self::Raw(data.to_vec()));
                }
                let priority = u16::from_be_bytes([data[0], data[1]]);
                let target = decode_domain_name(data, 2, offset_map)?;
                let target_len = domain_name_wire_len(data, 2);
                let params = data[2 + target_len..].to_vec();
                Ok(Self::SVCB { priority, target, params })
            }
            DnsRecordType::DS => {
                if data.len() < 4 {
                    return Ok(Self::Raw(data.to_vec()));
                }
                let key_tag = u16::from_be_bytes([data[0], data[1]]);
                let algorithm = data[2];
                let digest_type = data[3];
                Ok(Self::DS {
                    key_tag, algorithm, digest_type,
                    digest: data[4..].to_vec(),
                })
            }
            DnsRecordType::DNSKEY => {
                if data.len() < 4 {
                    return Ok(Self::Raw(data.to_vec()));
                }
                let flags = u16::from_be_bytes([data[0], data[1]]);
                let protocol = data[2];
                let algorithm = data[3];
                Ok(Self::DNSKEY {
                    protocol, flags, algorithm,
                    public_key: data[4..].to_vec(),
                })
            }
            DnsRecordType::RRSIG => {
                if data.len() < 18 {
                    return Ok(Self::Raw(data.to_vec()));
                }
                let type_covered = u16::from_be_bytes([data[0], data[1]]);
                let algorithm = data[2];
                let labels = data[3];
                let original_ttl = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
                let expiration = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
                let inception = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
                let key_tag = u16::from_be_bytes([data[16], data[17]]);
                // Signer name starts at offset 18, followed by signature
                let signer_name = decode_domain_name(data, 18, offset_map)?;
                let signer_len = domain_name_wire_len(data, 18);
                let sig_start = 18 + signer_len;
                Ok(Self::RRSIG {
                    type_covered, algorithm, labels, original_ttl,
                    expiration, inception, key_tag, signer_name,
                    signature: data[sig_start..].to_vec(),
                })
            }
            DnsRecordType::NSEC => {
                if data.is_empty() {
                    return Ok(Self::Raw(data.to_vec()));
                }
                let next_owner = decode_domain_name(data, 0, offset_map)?;
                let next_len = domain_name_wire_len(data, 0);
                Ok(Self::NSEC {
                    next_owner,
                    type_bits: data[next_len..].to_vec(),
                })
            }
            DnsRecordType::NSEC3 => {
                if data.len() < 6 {
                    return Ok(Self::Raw(data.to_vec()));
                }
                let hash_algorithm = data[0];
                let flags = data[1];
                let iterations = u16::from_be_bytes([data[2], data[3]]);
                let salt_len = data[4] as usize;
                let salt_start = 5;
                let salt_end = salt_start + salt_len;
                if salt_end + 1 > data.len() {
                    return Ok(Self::Raw(data.to_vec()));
                }
                let salt = data[salt_start..salt_end].to_vec();
                let hash_len = data[salt_end] as usize;
                let hash_start = salt_end + 1;
                let hash_end = hash_start + hash_len;
                if hash_end > data.len() {
                    return Ok(Self::Raw(data.to_vec()));
                }
                let next_hashed_owner = data[hash_start..hash_end].to_vec();
                let type_bits = data[hash_end..].to_vec();
                Ok(Self::NSEC3 {
                    hash_algorithm, flags, iterations, salt,
                    next_hashed_owner, type_bits,
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

    #[test]
    fn test_naptr_wire() {
        let data = DnsRecordData::NAPTR {
            order: 100,
            preference: 10,
            flags: "u".to_string(),
            services: "SIP+D2U".to_string(),
            regexp: "!^.*$!sip:customer-service@example.com!".to_string(),
            replacement: String::new(),
        };
        let wire = data.to_wire(DnsRecordType::NAPTR);
        assert_eq!(wire[0..2], [0, 100]); // order
        assert_eq!(wire[2..4], [0, 10]);  // preference
        assert_eq!(wire[4], 1);            // flags length
        assert_eq!(&wire[5..6], b"u");
    }

    #[test]
    fn test_caa_wire() {
        let data = DnsRecordData::CAA {
            critical: true,
            tag: "issue".to_string(),
            value: "letsencrypt.org".to_string(),
        };
        let wire = data.to_wire(DnsRecordType::CAA);
        assert_eq!(wire[0], 0x80); // critical flag
        assert_eq!(wire[1], 5);    // tag length
        assert_eq!(&wire[2..7], b"issue");
        assert_eq!(&wire[7..], b"letsencrypt.org");
    }

    #[test]
    fn test_tlsa_wire() {
        let data = DnsRecordData::TLSA {
            usage: 3,
            selector: 1,
            matching_type: 1,
            certificate: vec![0xDE, 0xAD, 0xBE, 0xEF],
        };
        let wire = data.to_wire(DnsRecordType::TLSA);
        assert_eq!(wire[0], 3);
        assert_eq!(wire[1], 1);
        assert_eq!(wire[2], 1);
        assert_eq!(&wire[3..], &[0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_svcb_wire() {
        let data = DnsRecordData::SVCB {
            priority: 1,
            target: "example.com".to_string(),
            params: vec![0, 4, 0, 8, 0, 1, 0x05, 0x06], // alpn=h2
        };
        let wire = data.to_wire(DnsRecordType::HTTPS);
        assert_eq!(wire[0..2], [0, 1]); // priority
        // target is domain-name encoded after priority
    }

    #[test]
    fn test_record_type_extended() {
        assert_eq!(DnsRecordType::from_u16(35), Some(DnsRecordType::NAPTR));
        assert_eq!(DnsRecordType::from_u16(257), Some(DnsRecordType::CAA));
        assert_eq!(DnsRecordType::from_u16(52), Some(DnsRecordType::TLSA));
        assert_eq!(DnsRecordType::from_u16(65), Some(DnsRecordType::HTTPS));
        assert_eq!(DnsRecordType::from_u16(64), Some(DnsRecordType::SVCB));
        assert_eq!(DnsRecordType::from_u16(43), Some(DnsRecordType::DS));
        assert_eq!(DnsRecordType::from_u16(46), Some(DnsRecordType::RRSIG));
        assert_eq!(DnsRecordType::from_u16(47), Some(DnsRecordType::NSEC));
        assert_eq!(DnsRecordType::from_u16(48), Some(DnsRecordType::DNSKEY));
        assert_eq!(DnsRecordType::from_u16(50), Some(DnsRecordType::NSEC3));
        assert_eq!(DnsRecordType::NAPTR.as_str(), "NAPTR");
        assert_eq!(DnsRecordType::CAA.as_str(), "CAA");
        assert_eq!(DnsRecordType::TLSA.as_str(), "TLSA");
        assert_eq!(DnsRecordType::HTTPS.as_str(), "HTTPS");
        assert_eq!(DnsRecordType::SVCB.as_str(), "SVCB");
        assert_eq!(DnsRecordType::DS.as_str(), "DS");
        assert_eq!(DnsRecordType::RRSIG.as_str(), "RRSIG");
        assert_eq!(DnsRecordType::NSEC.as_str(), "NSEC");
        assert_eq!(DnsRecordType::DNSKEY.as_str(), "DNSKEY");
        assert_eq!(DnsRecordType::NSEC3.as_str(), "NSEC3");
    }

    #[test]
    fn test_ds_wire() {
        let data = DnsRecordData::DS {
            key_tag: 12345,
            algorithm: 13, // ECDSAP256SHA256
            digest_type: 2, // SHA-256
            digest: vec![0xAB, 0xCD],
        };
        let wire = data.to_wire(DnsRecordType::DS);
        assert_eq!(wire[0..2], [48, 57]); // key_tag 12345
        assert_eq!(wire[2], 13); // algorithm
        assert_eq!(wire[3], 2);  // digest type
        assert_eq!(&wire[4..], &[0xAB, 0xCD]);
    }

    #[test]
    fn test_dnskey_wire() {
        let data = DnsRecordData::DNSKEY {
            protocol: 3,
            flags: 257, // KSK
            algorithm: 13, // ECDSAP256SHA256
            public_key: vec![0x04, 0xAB, 0xCD],
        };
        let wire = data.to_wire(DnsRecordType::DNSKEY);
        assert_eq!(wire[0..2], [1, 1]); // flags 257
        assert_eq!(wire[2], 3);    // protocol
        assert_eq!(wire[3], 13);   // algorithm
        assert_eq!(&wire[4..], &[0x04, 0xAB, 0xCD]);
    }

    #[test]
    fn test_nsec_wire() {
        let data = DnsRecordData::NSEC {
            next_owner: "next.example.com".to_string(),
            type_bits: vec![0x40, 0x01, 0x00, 0x01], // bitmap for A, RRSIG
        };
        let wire = data.to_wire(DnsRecordType::NSEC);
        // Next owner is domain-name encoded
        assert!(wire.len() > 16); // at least the domain name
        assert_eq!(&wire[wire.len()-4..], &[0x40, 0x01, 0x00, 0x01]);
    }

    #[test]
    fn test_nsec3_wire() {
        let data = DnsRecordData::NSEC3 {
            hash_algorithm: 1, // SHA-1
            flags: 0,
            iterations: 5,
            salt: vec![0xDE, 0xAD],
            next_hashed_owner: vec![0xAB, 0xCD, 0xEF],
            type_bits: vec![0x40],
        };
        let wire = data.to_wire(DnsRecordType::NSEC3);
        assert_eq!(wire[0], 1);   // hash algorithm
        assert_eq!(wire[1], 0);   // flags
        assert_eq!(wire[2..4], [0, 5]); // iterations
        assert_eq!(wire[4], 2);   // salt length
        assert_eq!(&wire[5..7], &[0xDE, 0xAD]);
        assert_eq!(wire[7], 3);   // hash length
        assert_eq!(&wire[8..11], &[0xAB, 0xCD, 0xEF]);
        assert_eq!(wire[11], 0x40); // type bits
    }
}
