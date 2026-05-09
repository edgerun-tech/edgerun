//! AXFR zone transfer (RFC 5936).
//!
//! Handles full zone transfers over TCP. The client sends an AXFR query,
//! and the server responds with a series of DNS messages containing
//! all records in the zone, bookended by SOA records.
//!
//! Optionally secured with TSIG authentication.

use super::io;
use alloc::collections::BTreeMap as HashMap;
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use super::message::{DnsMessage, DnsOpcode, DnsQuestion, DnsRecord, DnsResponseCode};
use super::record::{DnsRecordData, DnsRecordType};
use super::zone::DnsZone;

/// Runtime-provided TSIG verifier placeholder.
///
/// Protocol AXFR/UPDATE handling stays transport- and crypto-independent; a
/// runtime can authenticate before calling these deterministic helpers.
pub struct TsigVerifier;

/// Handle an AXFR zone transfer request.
///
/// Returns a vector of DNS messages, each containing a portion of the zone.
/// The first and last messages contain SOA records per RFC 5936.
pub fn handle_axfr(query: &DnsMessage, zone: &DnsZone) -> Result<Vec<DnsMessage>, DnsResponseCode> {
    let question = query.questions.first().ok_or(DnsResponseCode::FormErr)?;

    // Verify the requested zone matches
    let qname = question.name.to_lowercase();
    let origin = zone.origin.to_lowercase();
    if qname != origin && qname != format!(".{}", origin) {
        return Err(DnsResponseCode::Refused);
    }
    if question.qtype != DnsRecordType::AXFR {
        return Err(DnsResponseCode::NotImp);
    }

    // Collect all records from the zone
    let mut all_records: Vec<DnsRecord> = Vec::new();
    for name in zone.names() {
        for rr in zone.get_records(name) {
            // Skip TSIG — it's not part of the zone data
            if rr.rtype == DnsRecordType::TSIG {
                continue;
            }
            all_records.push((*rr).clone());
        }
    }

    // Sort records: SOA first, then alphabetically
    all_records.sort_by(|a, b| {
        // SOA first
        let a_is_soa = a.rtype == DnsRecordType::SOA;
        let b_is_soa = b.rtype == DnsRecordType::SOA;
        if a_is_soa && !b_is_soa {
            return core::cmp::Ordering::Less;
        }
        if !a_is_soa && b_is_soa {
            return core::cmp::Ordering::Greater;
        }
        // Then by name, then by type
        a.name
            .cmp(&b.name)
            .then_with(|| a.rtype.as_u16().cmp(&b.rtype.as_u16()))
    });

    if all_records.is_empty() {
        return Err(DnsResponseCode::ServFail);
    }

    let id = query.header.id;

    // First message: SOA
    let first_soa = all_records
        .iter()
        .find(|r| r.rtype == DnsRecordType::SOA)
        .ok_or(DnsResponseCode::ServFail)?
        .clone();

    let mut first_msg = DnsMessage::response(id, DnsResponseCode::NoError, vec![first_soa.clone()]);
    first_msg.questions = query.questions.clone();
    first_msg.header.question_count = 1;

    // Build remaining messages (each fits in a TCP frame — no size limit,
    // but we chunk for sanity).
    const MAX_RECORDS_PER_MSG: usize = 100;
    let mut messages = vec![first_msg];

    // Include SOA as first data record
    let data_records: Vec<DnsRecord> = all_records.clone();
    for chunk in data_records.chunks(MAX_RECORDS_PER_MSG) {
        let mut msg = DnsMessage::response(id, DnsResponseCode::NoError, chunk.to_vec());
        msg.questions = query.questions.clone();
        msg.header.question_count = 1;
        messages.push(msg);
    }

    // Last message: SOA (final envelope)
    let mut last_msg = DnsMessage::response(id, DnsResponseCode::NoError, vec![first_soa]);
    last_msg.questions = query.questions.clone();
    last_msg.header.question_count = 1;
    messages.push(last_msg);

    Ok(messages)
}

/// Handle a NOTIFY message (RFC 1996).
///
/// Validates the NOTIFY and returns an appropriate response.
/// The caller is responsible for actually notifying secondary servers.
pub fn handle_notify(query: &DnsMessage) -> Result<DnsMessage, DnsResponseCode> {
    if query.header.opcode != DnsOpcode::Notify {
        return Err(DnsResponseCode::NotImp);
    }

    let question = query.questions.first().ok_or(DnsResponseCode::FormErr)?;
    if question.qtype != DnsRecordType::SOA {
        return Err(DnsResponseCode::FormErr);
    }

    // Validate the SOA in the authority section
    let has_soa = query
        .authority
        .iter()
        .any(|r| r.rtype == DnsRecordType::SOA);
    if !has_soa {
        return Err(DnsResponseCode::FormErr);
    }

    let mut response = DnsMessage::response(query.header.id, DnsResponseCode::NoError, Vec::new());
    response.questions = query.questions.clone();
    response.header.question_count = query.header.question_count;
    response.header.opcode = DnsOpcode::Notify;

    Ok(response)
}

/// Handle a DNS Update message (RFC 2136).
///
/// Parses the update and applies it to the zone if authenticated.
/// Returns the response message.
pub fn handle_update(
    query: &DnsMessage,
    zone: &mut DnsZone,
    _tsig_verifier: Option<&TsigVerifier>,
) -> Result<DnsMessage, DnsResponseCode> {
    if query.header.opcode != DnsOpcode::Update {
        return Err(DnsResponseCode::NotImp);
    }

    // Zone section must have exactly one question specifying the zone
    let zone_question = query.questions.first().ok_or(DnsResponseCode::FormErr)?;
    if zone_question.qclass != 1 {
        // IN
        return Err(DnsResponseCode::NotAuth);
    }
    if zone_question.qtype != DnsRecordType::SOA {
        return Err(DnsResponseCode::FormErr);
    }

    // Verify zone name matches
    if zone_question.name.to_lowercase() != zone.origin.to_lowercase() {
        return Err(DnsResponseCode::NotAuth);
    }

    // Process authority section as updates (simplified RFC 2136):
    // - If rclass == 0 → delete all records for this name
    // - If rtype == ANY → delete all records for this name
    // - Otherwise → add/update record
    for rr in &query.authority {
        if rr.rclass == 0 || rr.rtype == DnsRecordType::ANY {
            zone.remove_name(&rr.name);
        } else {
            let mut new_rr = rr.clone();
            new_rr.rclass = 1; // IN
            zone.add_record(new_rr);
        }
    }

    let mut response = DnsMessage::response(query.header.id, DnsResponseCode::NoError, Vec::new());
    response.header.opcode = DnsOpcode::Update;
    response.authority = query.authority.clone();
    response.header.authority_count = query.header.authority_count;

    Ok(response)
}

/// Format AXFR zone transfer output as a human-readable string (dig-like).
pub fn format_axfr(messages: &[DnsMessage]) -> String {
    let mut out = String::new();
    for (i, msg) in messages.iter().enumerate() {
        out.push_str(&format!(
            ";; Message {} ({} records)\n",
            i + 1,
            msg.answers.len()
        ));
        for rr in &msg.answers {
            out.push_str(&format_rr(rr));
        }
    }
    out
}

fn format_rr(rr: &DnsRecord) -> String {
    format!(
        "{}  {}  {}  {}  {}\n",
        rr.name,
        rr.ttl,
        rr.rclass,
        rr.rtype,
        format_rdata(&rr.data)
    )
}

fn format_rdata(data: &DnsRecordData) -> String {
    match data {
        DnsRecordData::A(ip) => ip.to_string(),
        DnsRecordData::AAAA(ip) => ip.to_string(),
        DnsRecordData::CNAME(name) => name.clone(),
        DnsRecordData::NS(name) => name.clone(),
        DnsRecordData::MX { priority, exchange } => format!("{} {}", priority, exchange),
        DnsRecordData::TXT(text) => format!("\"{}\"", text),
        DnsRecordData::SOA {
            mname,
            rname,
            serial,
            refresh,
            retry,
            expire,
            minimum,
        } => format!(
            "{} {} {} {} {} {} {}",
            mname, rname, serial, refresh, retry, expire, minimum
        ),
        DnsRecordData::SRV {
            priority,
            weight,
            port,
            target,
        } => format!("{} {} {} {}", priority, weight, port, target),
        DnsRecordData::PTR(name) => name.clone(),
        DnsRecordData::NAPTR {
            order,
            preference,
            flags,
            services,
            regexp,
            replacement,
        } => format!(
            "{} {} \"{}\" \"{}\" \"{}\" {}",
            order, preference, flags, services, regexp, replacement
        ),
        DnsRecordData::CAA {
            critical,
            tag,
            value,
        } => format!("{} {} \"{}\"", if *critical { 128 } else { 0 }, tag, value),
        DnsRecordData::HINFO { cpu, os } => format!("\"{}\" \"{}\"", cpu, os),
        DnsRecordData::RP { mbox, txt } => format!("{} {}", mbox, txt),
        DnsRecordData::LOC {
            version,
            size,
            horiz_pre,
            vert_pre,
            latitude,
            longitude,
            altitude,
        } => format!(
            "{} {} {} {} {} {} {} {}",
            version, size, horiz_pre, vert_pre, latitude, longitude, altitude, altitude
        ),
        DnsRecordData::AFSDB { subtype, hostname } => format!("{} {}", subtype, hostname),
        DnsRecordData::URI {
            priority,
            weight,
            target,
        } => format!("{} {} \"{}\"", priority, weight, target),
        _ => format!("{:?}", data),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::net::Ipv4Addr;

    fn make_test_zone() -> DnsZone {
        let mut zone = DnsZone::new("example.com");
        zone.add_soa("ns1.example.com", "admin.example.com");
        zone.add_ns("ns1.example.com");
        zone.add_a("@", Ipv4Addr::new(1, 2, 3, 4), 3600);
        zone.add_a("www", Ipv4Addr::new(1, 2, 3, 5), 3600);
        zone.add_mx("@", 10, "mail.example.com", 3600);
        zone
    }

    #[test]
    fn test_axfr_basic() {
        let zone = make_test_zone();
        let query = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::AXFR);
        let msgs = handle_axfr(&query, &zone).unwrap();

        // At least 3 messages: SOA, data, SOA
        assert!(msgs.len() >= 3);

        // First and last should have SOA
        assert!(
            msgs.first()
                .unwrap()
                .answers
                .iter()
                .any(|r| r.rtype == DnsRecordType::SOA)
        );
        assert!(
            msgs.last()
                .unwrap()
                .answers
                .iter()
                .any(|r| r.rtype == DnsRecordType::SOA)
        );

        // All should have same ID
        for msg in &msgs {
            assert_eq!(msg.header.id, 0x1234);
            assert_eq!(msg.header.response_code, DnsResponseCode::NoError);
        }
    }

    #[test]
    fn test_axfr_wrong_zone() {
        let zone = make_test_zone();
        let query = DnsMessage::query(0x1234, "other.com".to_string(), DnsRecordType::AXFR);
        assert!(handle_axfr(&query, &zone).is_err());
    }

    #[test]
    fn test_axfr_not_axfr_type() {
        let zone = make_test_zone();
        let query = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::A);
        assert!(handle_axfr(&query, &zone).is_err());
    }

    #[test]
    fn test_notify_valid() {
        let mut query = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::SOA);
        query.header.opcode = DnsOpcode::Notify;
        query.authority.push(DnsRecord::soa(
            "example.com".into(),
            "ns1.example.com".into(),
            "admin.example.com".into(),
            1,
            3600,
            900,
            604800,
            86400,
            3600,
        ));

        let resp = handle_notify(&query).unwrap();
        assert_eq!(resp.header.response_code, DnsResponseCode::NoError);
        assert_eq!(resp.header.opcode, DnsOpcode::Notify);
    }

    #[test]
    fn test_notify_no_soa() {
        let mut query = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::SOA);
        query.header.opcode = DnsOpcode::Notify;

        assert!(handle_notify(&query).is_err());
    }

    #[test]
    fn test_update_add_record() {
        let mut zone = make_test_zone();
        let mut query = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::SOA);
        query.header.opcode = DnsOpcode::Update;
        // Update section: add a TXT record
        query
            .authority
            .push(DnsRecord::txt("example.com".into(), "v=spf1".into(), 3600));

        let resp = handle_update(&query, &mut zone, None).unwrap();
        assert_eq!(resp.header.response_code, DnsResponseCode::NoError);

        // Verify record was added
        let txt = zone.resolve("example.com", DnsRecordType::TXT);
        assert!(txt.is_some());
    }

    #[test]
    fn test_update_delete_record() {
        let mut zone = make_test_zone();
        let mut query = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::SOA);
        query.header.opcode = DnsOpcode::Update;
        // Delete all records for "www"
        let mut del = DnsRecord::a("www.example.com".into(), Ipv4Addr::new(0, 0, 0, 0), 0);
        del.rclass = 0; // NONE = delete all
        query.authority.push(del);

        let resp = handle_update(&query, &mut zone, None).unwrap();
        assert_eq!(resp.header.response_code, DnsResponseCode::NoError);

        // Verify www was deleted
        let a = zone.resolve("www.example.com", DnsRecordType::A);
        assert!(a.is_none());
    }

    #[test]
    fn test_format_axfr() {
        let zone = make_test_zone();
        let query = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::AXFR);
        let msgs = handle_axfr(&query, &zone).unwrap();
        let output = format_axfr(&msgs);
        assert!(output.contains("example.com"));
        assert!(output.contains("SOA"));
    }
}
