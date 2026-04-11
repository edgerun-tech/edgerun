//! DNS message parser/serializer — RFC 1035 wire format.

use std::io;
use std::net::Ipv4Addr;

use super::record::{DnsRecordType, DnsRecordData, encode_domain_name, decode_domain_name};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Standard DNS port number.
pub const DNS_PORT: u16 = 53;
const DNS_HEADER_SIZE: usize = 12;

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

/// DNS message header.
///
/// 12 bytes, always present at the start of every DNS message.
#[derive(Debug, Clone)]
pub struct DnsHeader {
    /// Transaction ID — copied into the response to match query/response.
    pub id: u16,
    /// `true` if this message is a response, `false` if a query.
    pub is_response: bool,
    /// Type of query (`Query`, `Notify`, `Update`, …).
    pub opcode: DnsOpcode,
    /// `true` if the server is authoritative for the domain.
    pub authoritative: bool,
    /// `true` if the response was truncated (use TCP for full response).
    pub truncated: bool,
    /// `true` if the client wants recursive resolution.
    pub recursion_desired: bool,
    /// `true` if the server supports recursion.
    pub recursion_available: bool,
    /// Result code (`NoError`, `NXDomain`, `ServFail`, …).
    pub response_code: DnsResponseCode,
    /// Number of entries in the question section.
    pub question_count: u16,
    /// Number of resource records in the answer section.
    pub answer_count: u16,
    /// Number of resource records in the authority section.
    pub authority_count: u16,
    /// Number of resource records in the additional section.
    pub additional_count: u16,
}

impl DnsHeader {
    /// Create a query header.
    pub fn query(id: u16, recursion_desired: bool) -> Self {
        Self {
            id,
            is_response: false,
            opcode: DnsOpcode::Query,
            authoritative: false,
            truncated: false,
            recursion_desired,
            recursion_available: false,
            response_code: DnsResponseCode::NoError,
            question_count: 0,
            answer_count: 0,
            authority_count: 0,
            additional_count: 0,
        }
    }

    /// Create a response header with the given response code.
    pub fn response(id: u16, rcode: DnsResponseCode) -> Self {
        Self {
            id,
            is_response: true,
            opcode: DnsOpcode::Query,
            authoritative: true,
            truncated: false,
            recursion_desired: false,
            recursion_available: true,
            response_code: rcode,
            question_count: 0,
            answer_count: 0,
            authority_count: 0,
            additional_count: 0,
        }
    }

    fn to_wire(&self) -> [u8; DNS_HEADER_SIZE] {
        let mut buf = [0u8; DNS_HEADER_SIZE];
        buf[0..2].copy_from_slice(&self.id.to_be_bytes());

        let mut flags: u16 = 0;
        if self.is_response {
            flags |= 0x8000;
        }
        flags |= ((self.opcode as u16) & 0x0F) << 11;
        if self.authoritative {
            flags |= 0x0400;
        }
        if self.truncated {
            flags |= 0x0200;
        }
        if self.recursion_desired {
            flags |= 0x0100;
        }
        if self.recursion_available {
            flags |= 0x0080;
        }
        flags |= (self.response_code as u16) & 0x000F;

        buf[2..4].copy_from_slice(&flags.to_be_bytes());
        buf[4..6].copy_from_slice(&self.question_count.to_be_bytes());
        buf[6..8].copy_from_slice(&self.answer_count.to_be_bytes());
        buf[8..10].copy_from_slice(&self.authority_count.to_be_bytes());
        buf[10..12].copy_from_slice(&self.additional_count.to_be_bytes());
        buf
    }

    fn from_wire(data: &[u8]) -> Result<Self, io::Error> {
        if data.len() < DNS_HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DNS header too short",
            ));
        }

        let id = u16::from_be_bytes([data[0], data[1]]);
        let flags = u16::from_be_bytes([data[2], data[3]]);

        let is_response = (flags & 0x8000) != 0;
        let opcode_val = ((flags >> 11) & 0x0F) as u8;
        let opcode = DnsOpcode::from_u8(opcode_val).unwrap_or(DnsOpcode::Query);
        let authoritative = (flags & 0x0400) != 0;
        let truncated = (flags & 0x0200) != 0;
        let recursion_desired = (flags & 0x0100) != 0;
        let recursion_available = (flags & 0x0080) != 0;
        let response_code = DnsResponseCode::from_u8((flags & 0x0F) as u8)
            .unwrap_or(DnsResponseCode::ServFail);

        let question_count = u16::from_be_bytes([data[4], data[5]]);
        let answer_count = u16::from_be_bytes([data[6], data[7]]);
        let authority_count = u16::from_be_bytes([data[8], data[9]]);
        let additional_count = u16::from_be_bytes([data[10], data[11]]);

        Ok(Self {
            id,
            is_response,
            opcode,
            authoritative,
            truncated,
            recursion_desired,
            recursion_available,
            response_code,
            question_count,
            answer_count,
            authority_count,
            additional_count,
        })
    }
}

// ---------------------------------------------------------------------------
// Opcode & RCODE
// ---------------------------------------------------------------------------

/// DNS opcode — the type of DNS operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DnsOpcode {
    /// Standard query.
    Query = 0,
    /// Inverse query (deprecated, RFC 3425).
    IQuery = 1,
    /// Server status request.
    Status = 2,
    /// Zone change notification (RFC 1996).
    Notify = 4,
    /// Dynamic update (RFC 2136).
    Update = 5,
}

impl DnsOpcode {
    /// Parse an opcode from a raw u8 value. Returns `None` for unknown values.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Query),
            1 => Some(Self::IQuery),
            2 => Some(Self::Status),
            4 => Some(Self::Notify),
            5 => Some(Self::Update),
            _ => None,
        }
    }
}

/// DNS response code (RCODE) — indicates the result of a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DnsResponseCode {
    /// No error — the query was successful.
    NoError = 0,
    /// Format error — the server could not interpret the query.
    FormErr = 1,
    /// Server failure — the server encountered an internal problem.
    ServFail = 2,
    /// Non-existent domain — the domain name does not exist.
    NXDomain = 3,
    /// Not implemented — the server does not support this operation.
    NotImp = 4,
    /// Refused — the server refused to perform the operation.
    Refused = 5,
    /// Name should not exist but does (dynamic update).
    YXDomain = 6,
    /// RR set should not exist but does (dynamic update).
    YXRRSet = 7,
    /// RR set should exist but does not (dynamic update).
    NXRRSet = 8,
    /// Server not authoritative for the zone.
    NotAuth = 9,
    /// Server not a member of the zone (TSIG).
    NotZone = 10,
}

impl DnsResponseCode {
    /// Parse a response code from a raw u8 value. Returns `None` for unknown values.
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::NoError),
            1 => Some(Self::FormErr),
            2 => Some(Self::ServFail),
            3 => Some(Self::NXDomain),
            4 => Some(Self::NotImp),
            5 => Some(Self::Refused),
            6 => Some(Self::YXDomain),
            7 => Some(Self::YXRRSet),
            8 => Some(Self::NXRRSet),
            9 => Some(Self::NotAuth),
            10 => Some(Self::NotZone),
            _ => None,
        }
    }

    /// Human-readable string representation of the response code.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NoError => "NOERROR",
            Self::FormErr => "FORMERR",
            Self::ServFail => "SERVFAIL",
            Self::NXDomain => "NXDOMAIN",
            Self::NotImp => "NOTIMP",
            Self::Refused => "REFUSED",
            Self::YXDomain => "YXDOMAIN",
            Self::YXRRSet => "YXRRSET",
            Self::NXRRSet => "NXRRSET",
            Self::NotAuth => "NOTAUTH",
            Self::NotZone => "NOTZONE",
        }
    }
}

// ---------------------------------------------------------------------------
// Question
// ---------------------------------------------------------------------------

/// A DNS question section entry.
///
/// Contains the domain name, record type, and class being queried.
#[derive(Debug, Clone)]
pub struct DnsQuestion {
    /// Domain name being queried (e.g. "www.example.com").
    pub name: String,
    /// Record type requested (A, AAAA, MX, …).
    pub qtype: DnsRecordType,
    /// Network class — always `1` for IN (Internet).
    pub qclass: u16,
}

impl DnsQuestion {
    /// Create a new question for the given name and record type.
    pub fn new(name: String, qtype: DnsRecordType) -> Self {
        Self {
            name,
            qtype,
            qclass: 1, // IN (Internet)
        }
    }

    fn to_wire(&self) -> Vec<u8> {
        let mut buf = encode_domain_name(&self.name);
        buf.extend_from_slice(&self.qtype.as_u16().to_be_bytes());
        buf.extend_from_slice(&self.qclass.to_be_bytes());
        buf
    }

    fn from_wire(
        data: &[u8],
        offset: usize,
        offset_map: &[(usize, usize)],
    ) -> Result<(Self, usize), io::Error> {
        let name = decode_domain_name(data, offset, offset_map)?;
        let name_len = domain_name_wire_len(data, offset);
        let rr_start = offset + name_len;

        if rr_start + 4 > data.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DNS question truncated",
            ));
        }

        let qtype_val = u16::from_be_bytes([data[rr_start], data[rr_start + 1]]);
        let qclass = u16::from_be_bytes([data[rr_start + 2], data[rr_start + 3]]);

        let qtype = DnsRecordType::from_u16(qtype_val).unwrap_or(DnsRecordType::A);

        Ok((
            Self {
                name,
                qtype,
                qclass,
            },
            rr_start + 4,
        ))
    }
}

// ---------------------------------------------------------------------------
// Resource Record
// ---------------------------------------------------------------------------

/// A DNS resource record (answer, authority, or additional section).
///
/// Holds one piece of DNS data — an IP address, a mail server, a text string, etc.
#[derive(Debug, Clone)]
pub struct DnsRecord {
    /// Domain name this record belongs to.
    pub name: String,
    /// Record type (A, AAAA, CNAME, …).
    pub rtype: DnsRecordType,
    /// Network class — always `1` for IN (Internet).
    pub rclass: u16,
    /// Time-to-live in seconds — how long resolvers may cache this record.
    pub ttl: u32,
    /// The actual record data (IP, hostname, text, …).
    pub data: DnsRecordData,
}

impl DnsRecord {
    /// Create an A record (IPv4 address).
    pub fn a(name: String, ip: Ipv4Addr, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::A,
            rclass: 1,
            ttl,
            data: DnsRecordData::A(ip),
        }
    }

    /// Create an AAAA record (IPv6 address).
    pub fn aaaa(name: String, ip: std::net::Ipv6Addr, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::AAAA,
            rclass: 1,
            ttl,
            data: DnsRecordData::AAAA(ip),
        }
    }

    /// Create a CNAME record (canonical name / alias).
    pub fn cname(name: String, target: String, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::CNAME,
            rclass: 1,
            ttl,
            data: DnsRecordData::CNAME(target),
        }
    }

    /// Create an NS record (name server).
    pub fn ns(name: String, nameserver: String, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::NS,
            rclass: 1,
            ttl,
            data: DnsRecordData::NS(nameserver),
        }
    }

    /// Create an MX record (mail exchange).
    pub fn mx(name: String, priority: u16, exchange: String, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::MX,
            rclass: 1,
            ttl,
            data: DnsRecordData::MX { priority, exchange },
        }
    }

    /// Create a TXT record (arbitrary text, e.g. SPF records).
    pub fn txt(name: String, text: String, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::TXT,
            rclass: 1,
            ttl,
            data: DnsRecordData::TXT(text),
        }
    }

    /// Create a PTR record (reverse DNS pointer).
    pub fn ptr(name: String, ptr_name: String, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::PTR,
            rclass: 1,
            ttl,
            data: DnsRecordData::PTR(ptr_name),
        }
    }

    /// Create a NAPTR record (RFC 3403 — URI/telephone routing).
    pub fn naptr(name: String, order: u16, preference: u16, flags: String,
                 services: String, regexp: String, replacement: String, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::NAPTR,
            rclass: 1,
            ttl,
            data: DnsRecordData::NAPTR { order, preference, flags, services, regexp, replacement },
        }
    }

    /// Create a CAA record (RFC 8659 — Certificate Authority Authorization).
    pub fn caa(name: String, critical: bool, tag: String, value: String, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::CAA,
            rclass: 1,
            ttl,
            data: DnsRecordData::CAA { critical, tag, value },
        }
    }

    /// Create a TLSA record (RFC 6698 — DANE certificate binding).
    pub fn tlsa(name: String, usage: u8, selector: u8, matching_type: u8,
                certificate: Vec<u8>, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::TLSA,
            rclass: 1,
            ttl,
            data: DnsRecordData::TLSA { usage, selector, matching_type, certificate },
        }
    }

    /// Create an SVCB/HTTPS record (RFC 9460 — Service Binding).
    pub fn svcb(name: String, priority: u16, target: String, params: Vec<u8>, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::HTTPS,
            rclass: 1,
            ttl,
            data: DnsRecordData::SVCB { priority, target, params },
        }
    }

    /// Create a DS record (RFC 4034 — Delegation Signer).
    pub fn ds(name: String, key_tag: u16, algorithm: u8, digest_type: u8,
              digest: Vec<u8>, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::DS,
            rclass: 1,
            ttl,
            data: DnsRecordData::DS { key_tag, algorithm, digest_type, digest },
        }
    }

    /// Create a DNSKEY record (RFC 4034 — DNS Public Key).
    pub fn dnskey(name: String, flags: u16, protocol: u8, algorithm: u8,
                  public_key: Vec<u8>, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::DNSKEY,
            rclass: 1,
            ttl,
            data: DnsRecordData::DNSKEY { protocol, flags, algorithm, public_key },
        }
    }

    /// Create an RRSIG record (RFC 4034 — RRset Signature).
    pub fn rrsig(name: String, type_covered: u16, algorithm: u8, labels: u8,
                 original_ttl: u32, expiration: u32, inception: u32, key_tag: u16,
                 signer_name: String, signature: Vec<u8>, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::RRSIG,
            rclass: 1,
            ttl,
            data: DnsRecordData::RRSIG {
                type_covered, algorithm, labels, original_ttl,
                expiration, inception, key_tag, signer_name, signature,
            },
        }
    }

    /// Create an NSEC record (RFC 4034 — Next Secure).
    pub fn nsec(name: String, next_owner: String, type_bits: Vec<u8>, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::NSEC,
            rclass: 1,
            ttl,
            data: DnsRecordData::NSEC { next_owner, type_bits },
        }
    }

    /// Create an NSEC3 record (RFC 5155 — Next SECure v3).
    pub fn nsec3(name: String, hash_algorithm: u8, flags: u8, iterations: u16,
                 salt: Vec<u8>, next_hashed_owner: Vec<u8>, type_bits: Vec<u8>, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::NSEC3,
            rclass: 1,
            ttl,
            data: DnsRecordData::NSEC3 {
                hash_algorithm, flags, iterations, salt,
                next_hashed_owner, type_bits,
            },
        }
    }

    /// Create an EDNS0 OPT pseudo-record (RFC 6891).
    ///
    /// The `rclass` field holds the UDP payload size, and `ttl` encodes
    /// extended RCODE, version, and flags per the RFC.
    pub fn opt(udp_payload_size: u16, ext_rcode: u8, version: u8, flags: u16, options: Vec<u8>) -> Self {
        Self {
            name: ".".to_string(),
            rtype: DnsRecordType::OPT,
            rclass: udp_payload_size,
            ttl: ((ext_rcode as u32) << 24) | ((version as u32) << 16) | (flags as u32),
            data: DnsRecordData::OPT {
                ext_rcode,
                version,
                flags,
                options,
            },
        }
    }

    fn to_wire(&self, _full_message_offset: usize) -> Vec<u8> {
        let mut buf = encode_domain_name(&self.name);
        buf.extend_from_slice(&self.rtype.as_u16().to_be_bytes());
        buf.extend_from_slice(&self.rclass.to_be_bytes());
        buf.extend_from_slice(&self.ttl.to_be_bytes());

        let rdata = self.data.to_wire(self.rtype);
        buf.extend_from_slice(&(rdata.len() as u16).to_be_bytes());
        buf.extend_from_slice(&rdata);
        buf
    }

    fn from_wire(
        data: &[u8],
        offset: usize,
        offset_map: &[(usize, usize)],
    ) -> Result<(Self, usize), io::Error> {
        let name = decode_domain_name(data, offset, offset_map)?;
        let name_len = domain_name_wire_len(data, offset);
        let rr_start = offset + name_len;

        if rr_start + 10 > data.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DNS RR too short",
            ));
        }

        let rtype_val = u16::from_be_bytes([data[rr_start], data[rr_start + 1]]);
        let rclass = u16::from_be_bytes([data[rr_start + 2], data[rr_start + 3]]);
        let ttl = u32::from_be_bytes([
            data[rr_start + 4],
            data[rr_start + 5],
            data[rr_start + 6],
            data[rr_start + 7],
        ]);
        let rdlength = u16::from_be_bytes([data[rr_start + 8], data[rr_start + 9]]) as usize;
        let rdata_start = rr_start + 10;

        if rdata_start + rdlength > data.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DNS RR data truncated",
            ));
        }

        let rdata = &data[rdata_start..rdata_start + rdlength];
        let rtype = DnsRecordType::from_u16(rtype_val).unwrap_or(DnsRecordType::A);
        let data_parsed = DnsRecordData::from_wire(rtype, rdata, offset_map)?;

        Ok((
            Self {
                name,
                rtype,
                rclass,
                ttl,
                data: data_parsed,
            },
            rdata_start + rdlength,
        ))
    }
}

// ---------------------------------------------------------------------------
// Full message
// ---------------------------------------------------------------------------

/// A complete DNS message (query or response).
///
/// Contains a header, one question, and zero or more answer/authority/additional records.
#[derive(Debug, Clone)]
pub struct DnsMessage {
    /// Message header — transaction ID, flags, and record counts.
    pub header: DnsHeader,
    /// Question section — typically one entry per query.
    pub questions: Vec<DnsQuestion>,
    /// Answer section — records matching the queried name and type.
    pub answers: Vec<DnsRecord>,
    /// Authority section — name servers responsible for the domain.
    pub authority: Vec<DnsRecord>,
    /// Additional section — extra helpful records (glue records, etc.).
    pub additional: Vec<DnsRecord>,
}

impl DnsMessage {
    /// Serialize to DNS wire format.
    pub fn to_wire(&self) -> Vec<u8> {
        let mut header = self.header.clone();
        header.question_count = self.questions.len() as u16;
        header.answer_count = self.answers.len() as u16;
        header.authority_count = self.authority.len() as u16;
        header.additional_count = self.additional.len() as u16;

        let mut buf = header.to_wire().to_vec();

        for q in &self.questions {
            buf.extend_from_slice(&q.to_wire());
        }
        for rr in &self.answers {
            buf.extend_from_slice(&rr.to_wire(buf.len()));
        }
        for rr in &self.authority {
            buf.extend_from_slice(&rr.to_wire(buf.len()));
        }
        for rr in &self.additional {
            buf.extend_from_slice(&rr.to_wire(buf.len()));
        }

        buf
    }

    /// Parse from DNS wire format.
    pub fn from_wire(data: &[u8]) -> Result<Self, io::Error> {
        let header = DnsHeader::from_wire(data)?;

        // Build offset map: message_offset -> data_offset
        // Since we're parsing from a slice starting at the header,
        // message offsets = data offsets
        let offset_map: Vec<(usize, usize)> = (0..data.len()).map(|i| (i, i)).collect();

        let mut pos = DNS_HEADER_SIZE;
        let mut questions = Vec::new();
        for _ in 0..header.question_count {
            let (q, next) = DnsQuestion::from_wire(data, pos, &offset_map)?;
            questions.push(q);
            pos = next;
        }

        let mut answers = Vec::new();
        for _ in 0..header.answer_count {
            let (rr, next) = DnsRecord::from_wire(data, pos, &offset_map)?;
            answers.push(rr);
            pos = next;
        }

        let mut authority = Vec::new();
        for _ in 0..header.authority_count {
            let (rr, next) = DnsRecord::from_wire(data, pos, &offset_map)?;
            authority.push(rr);
            pos = next;
        }

        let mut additional = Vec::new();
        for _ in 0..header.additional_count {
            let (rr, next) = DnsRecord::from_wire(data, pos, &offset_map)?;
            additional.push(rr);
            pos = next;
        }

        Ok(Self {
            header,
            questions,
            answers,
            authority,
            additional,
        })
    }

    /// Create a standard query message.
    pub fn query(id: u16, name: String, qtype: DnsRecordType) -> Self {
        let mut header = DnsHeader::query(id, true);
        header.question_count = 1;
        Self {
            header,
            questions: vec![DnsQuestion::new(name, qtype)],
            answers: Vec::new(),
            authority: Vec::new(),
            additional: Vec::new(),
        }
    }

    /// Create a response message.
    pub fn response(id: u16, rcode: DnsResponseCode, answers: Vec<DnsRecord>) -> Self {
        let mut header = DnsHeader::response(id, rcode);
        header.question_count = 0; // Set by caller
        header.answer_count = answers.len() as u16;
        Self {
            header,
            questions: Vec::new(),
            answers,
            authority: Vec::new(),
            additional: Vec::new(),
        }
    }

    // -----------------------------------------------------------------------
    // EDNS0 helpers (RFC 6891)
    // -----------------------------------------------------------------------

    /// Get the EDNS0 OPT record from the additional section, if present.
    pub fn opt_record(&self) -> Option<&DnsRecord> {
        self.additional.iter().find(|r| r.rtype == DnsRecordType::OPT)
    }

    /// Check if the client sent an EDNS0 query.
    pub fn has_edns0(&self) -> bool {
        self.opt_record().is_some()
    }

    /// Get the client's advertised UDP payload size (from EDNS0), defaulting to 512.
    pub fn udp_payload_size(&self) -> u16 {
        self.opt_record().map(|r| r.rclass).unwrap_or(512)
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip() {
        let h = DnsHeader::query(0x1234, true);
        let wire = h.to_wire();
        let parsed = DnsHeader::from_wire(&wire).unwrap();
        assert_eq!(parsed.id, 0x1234);
        assert!(!parsed.is_response);
        assert!(parsed.recursion_desired);
    }

    #[test]
    fn test_response_header_roundtrip() {
        let h = DnsHeader::response(0xabcd, DnsResponseCode::NoError);
        let wire = h.to_wire();
        let parsed = DnsHeader::from_wire(&wire).unwrap();
        assert!(parsed.is_response);
        assert!(parsed.recursion_available);
        assert!(parsed.authoritative);
        assert_eq!(parsed.response_code, DnsResponseCode::NoError);
    }

    #[test]
    fn test_query_message_roundtrip() {
        let msg = DnsMessage::query(0x1234, "www.example.com".to_string(), DnsRecordType::A);
        let wire = msg.to_wire();
        let parsed = DnsMessage::from_wire(&wire).unwrap();

        assert_eq!(parsed.header.id, 0x1234);
        assert_eq!(parsed.questions.len(), 1);
        assert_eq!(parsed.questions[0].name, "www.example.com");
        assert_eq!(parsed.questions[0].qtype, DnsRecordType::A);
    }

    #[test]
    fn test_a_response_roundtrip() {
        let answers = vec![DnsRecord::a(
            "www.example.com".to_string(),
            Ipv4Addr::new(93, 184, 216, 34),
            3600,
        )];
        let mut msg = DnsMessage::response(0x5678, DnsResponseCode::NoError, answers);
        msg.questions.push(DnsQuestion::new(
            "www.example.com".to_string(),
            DnsRecordType::A,
        ));
        msg.header.question_count = 1;

        let wire = msg.to_wire();
        let parsed = DnsMessage::from_wire(&wire).unwrap();

        assert!(parsed.header.is_response);
        assert_eq!(parsed.answers.len(), 1);
        assert_eq!(parsed.answers[0].name, "www.example.com");
        assert_eq!(parsed.answers[0].rtype, DnsRecordType::A);
        if let DnsRecordData::A(ip) = &parsed.answers[0].data {
            assert_eq!(*ip, Ipv4Addr::new(93, 184, 216, 34));
        } else {
            panic!("Expected A record data");
        }
    }

    #[test]
    fn test_nxdomain_response() {
        let msg = DnsMessage::response(0x9999, DnsResponseCode::NXDomain, Vec::new());
        assert_eq!(msg.header.response_code, DnsResponseCode::NXDomain);
    }

    #[test]
    fn test_question_encoding() {
        let q = DnsQuestion::new("test.example.com".to_string(), DnsRecordType::AAAA);
        let wire = q.to_wire();
        let offset_map: Vec<(usize, usize)> = wire.iter().enumerate().map(|(i, _)| (i, i)).collect();
        let (parsed, _) =
            DnsQuestion::from_wire(&wire, 0, &offset_map).unwrap();
        assert_eq!(parsed.name, "test.example.com");
        assert_eq!(parsed.qtype, DnsRecordType::AAAA);
    }

    #[test]
    fn test_a_record_wire() {
        let rr = DnsRecord::a(
            "example.com".to_string(),
            Ipv4Addr::new(1, 2, 3, 4),
            300,
        );
        let wire = rr.to_wire(0);
        // Should contain: name + type(2) + class(2) + ttl(4) + rdlength(2) + rdata(4)
        assert!(wire.len() > 14);
    }

    #[test]
    fn test_multiple_answers() {
        let answers = vec![
            DnsRecord::a("example.com".to_string(), Ipv4Addr::new(1, 1, 1, 1), 60),
            DnsRecord::a("example.com".to_string(), Ipv4Addr::new(2, 2, 2, 2), 60),
        ];
        let mut msg = DnsMessage::response(0x1111, DnsResponseCode::NoError, answers);
        msg.questions.push(DnsQuestion::new("example.com".to_string(), DnsRecordType::A));
        msg.header.question_count = 1;

        let wire = msg.to_wire();
        let parsed = DnsMessage::from_wire(&wire).unwrap();
        assert_eq!(parsed.answers.len(), 2);
    }

    #[test]
    fn test_dns_opcode_and_rcode() {
        assert_eq!(DnsOpcode::from_u8(0), Some(DnsOpcode::Query));
        assert_eq!(DnsOpcode::from_u8(5), Some(DnsOpcode::Update));
        assert_eq!(DnsOpcode::from_u8(99), None);

        assert_eq!(DnsResponseCode::from_u8(3), Some(DnsResponseCode::NXDomain));
        assert_eq!(DnsResponseCode::NoError.as_str(), "NOERROR");
        assert_eq!(DnsResponseCode::NXDomain.as_str(), "NXDOMAIN");
    }

    #[test]
    fn test_ptr_record() {
        let rr = DnsRecord::ptr(
            "1.1.168.192.in-addr.arpa".to_string(),
            "host.example.com".to_string(),
            3600,
        );
        assert_eq!(rr.rtype, DnsRecordType::PTR);
        if let DnsRecordData::PTR(name) = &rr.data {
            assert_eq!(name, "host.example.com");
        }
    }
}
