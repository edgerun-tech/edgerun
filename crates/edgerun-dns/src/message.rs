//! DNS message parser/serializer — RFC 1035 wire format.

use std::io;
use std::net::Ipv4Addr;

use super::record::{DnsRecordType, DnsRecordData, encode_domain_name, decode_domain_name};

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const DNS_PORT: u16 = 53;
const DNS_HEADER_SIZE: usize = 12;

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

/// DNS message header.
#[derive(Debug, Clone)]
pub struct DnsHeader {
    pub id: u16,
    pub is_response: bool,
    pub opcode: DnsOpcode,
    pub authoritative: bool,
    pub truncated: bool,
    pub recursion_desired: bool,
    pub recursion_available: bool,
    pub response_code: DnsResponseCode,
    pub question_count: u16,
    pub answer_count: u16,
    pub authority_count: u16,
    pub additional_count: u16,
}

impl DnsHeader {
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

/// DNS opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DnsOpcode {
    Query = 0,
    IQuery = 1,
    Status = 2,
    Notify = 4,
    Update = 5,
}

impl DnsOpcode {
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

/// DNS response code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DnsResponseCode {
    NoError = 0,
    FormErr = 1,
    ServFail = 2,
    NXDomain = 3,
    NotImp = 4,
    Refused = 5,
    YXDomain = 6,
    YXRRSet = 7,
    NXRRSet = 8,
    NotAuth = 9,
    NotZone = 10,
}

impl DnsResponseCode {
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
#[derive(Debug, Clone)]
pub struct DnsQuestion {
    pub name: String,
    pub qtype: DnsRecordType,
    pub qclass: u16,
}

impl DnsQuestion {
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

/// A DNS resource record (answer, authority, or additional).
#[derive(Debug, Clone)]
pub struct DnsRecord {
    pub name: String,
    pub rtype: DnsRecordType,
    pub rclass: u16,
    pub ttl: u32,
    pub data: DnsRecordData,
}

impl DnsRecord {
    pub fn a(name: String, ip: Ipv4Addr, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::A,
            rclass: 1,
            ttl,
            data: DnsRecordData::A(ip),
        }
    }

    pub fn aaaa(name: String, ip: std::net::Ipv6Addr, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::AAAA,
            rclass: 1,
            ttl,
            data: DnsRecordData::AAAA(ip),
        }
    }

    pub fn cname(name: String, target: String, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::CNAME,
            rclass: 1,
            ttl,
            data: DnsRecordData::CNAME(target),
        }
    }

    pub fn ns(name: String, nameserver: String, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::NS,
            rclass: 1,
            ttl,
            data: DnsRecordData::NS(nameserver),
        }
    }

    pub fn mx(name: String, priority: u16, exchange: String, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::MX,
            rclass: 1,
            ttl,
            data: DnsRecordData::MX { priority, exchange },
        }
    }

    pub fn txt(name: String, text: String, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::TXT,
            rclass: 1,
            ttl,
            data: DnsRecordData::TXT(text),
        }
    }

    pub fn ptr(name: String, ptr_name: String, ttl: u32) -> Self {
        Self {
            name,
            rtype: DnsRecordType::PTR,
            rclass: 1,
            ttl,
            data: DnsRecordData::PTR(ptr_name),
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

/// A complete DNS message.
#[derive(Debug, Clone)]
pub struct DnsMessage {
    pub header: DnsHeader,
    pub questions: Vec<DnsQuestion>,
    pub answers: Vec<DnsRecord>,
    pub authority: Vec<DnsRecord>,
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
