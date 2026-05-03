//! DNS wire-message hardening helpers.
//!
//! This module provides a small pre-parse guard for network-facing DNS entry
//! points. It intentionally does not replace the full RFC 1035 parser; it
//! rejects abusive message sizes and impossible section counts before callers
//! allocate offset maps or parse arbitrary record counts.

use crate::message::DnsMessage;
use crate::std::io;
use edgerun_encoding::byteorder::read_u16_be;

/// DNS messages are length-prefixed over TCP with a 16-bit length and are
/// smaller over UDP. Anything larger cannot be a valid DNS wire message.
pub const MAX_DNS_MESSAGE_LEN: usize = 65_535;

/// A normal query has one question. This cap still allows multi-question
/// packets while rejecting count-amplification inputs before allocation.
pub const MAX_DNS_QUESTIONS: u16 = 16;

/// Bound each resource-record section before parsing. Large AXFR/IXFR-style
/// transfers should use dedicated zone-transfer code paths instead of the
/// generic packet parser.
pub const MAX_DNS_SECTION_RECORDS: u16 = 256;

const DNS_HEADER_SIZE: usize = 12;

/// Parsed DNS section counts from the fixed 12-byte header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DnsSectionCounts {
    pub questions: u16,
    pub answers: u16,
    pub authority: u16,
    pub additional: u16,
}

impl DnsSectionCounts {
    pub fn total_records(self) -> u32 {
        self.questions as u32
            + self.answers as u32
            + self.authority as u32
            + self.additional as u32
    }
}

/// Read section counts without fully parsing the message.
pub fn dns_section_counts(data: &[u8]) -> Result<DnsSectionCounts, io::Error> {
    if data.len() < DNS_HEADER_SIZE {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DNS header too short",
        ));
    }

    Ok(DnsSectionCounts {
        questions: read_u16_be(data, 4),
        answers: read_u16_be(data, 6),
        authority: read_u16_be(data, 8),
        additional: read_u16_be(data, 10),
    })
}

/// Validate a DNS wire message before allocating parser data structures.
pub fn validate_dns_wire_bounds(data: &[u8]) -> Result<(), io::Error> {
    if data.len() > MAX_DNS_MESSAGE_LEN {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DNS message exceeds maximum wire length",
        ));
    }

    let counts = dns_section_counts(data)?;

    if counts.questions > MAX_DNS_QUESTIONS {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DNS question count exceeds parser limit",
        ));
    }

    if counts.answers > MAX_DNS_SECTION_RECORDS
        || counts.authority > MAX_DNS_SECTION_RECORDS
        || counts.additional > MAX_DNS_SECTION_RECORDS
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DNS resource-record section count exceeds parser limit",
        ));
    }

    let max_total = MAX_DNS_QUESTIONS as u32 + (MAX_DNS_SECTION_RECORDS as u32 * 3);
    if counts.total_records() > max_total {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "DNS total section count exceeds parser limit",
        ));
    }

    Ok(())
}

/// Bounded wrapper around `DnsMessage::from_wire()` for network-facing callers.
pub fn parse_dns_message_bounded(data: &[u8]) -> Result<DnsMessage, io::Error> {
    validate_dns_wire_bounds(data)?;
    DnsMessage::from_wire(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header_with_counts(qd: u16, an: u16, ns: u16, ar: u16) -> [u8; DNS_HEADER_SIZE] {
        let mut header = [0u8; DNS_HEADER_SIZE];
        header[4..6].copy_from_slice(&qd.to_be_bytes());
        header[6..8].copy_from_slice(&an.to_be_bytes());
        header[8..10].copy_from_slice(&ns.to_be_bytes());
        header[10..12].copy_from_slice(&ar.to_be_bytes());
        header
    }

    #[test]
    fn accepts_empty_header_counts() {
        let header = header_with_counts(0, 0, 0, 0);
        assert!(validate_dns_wire_bounds(&header).is_ok());
    }

    #[test]
    fn rejects_short_header() {
        assert!(validate_dns_wire_bounds(&[0u8; 11]).is_err());
    }

    #[test]
    fn rejects_too_many_questions() {
        let header = header_with_counts(MAX_DNS_QUESTIONS + 1, 0, 0, 0);
        assert!(validate_dns_wire_bounds(&header).is_err());
    }

    #[test]
    fn rejects_too_many_records_in_section() {
        let header = header_with_counts(0, MAX_DNS_SECTION_RECORDS + 1, 0, 0);
        assert!(validate_dns_wire_bounds(&header).is_err());
    }
}
