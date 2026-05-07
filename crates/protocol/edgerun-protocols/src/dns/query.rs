//! DNS authoritative query processing.
//!
//! This module owns transport-independent DNS query response construction. It
//! does not open sockets, forward to recursive resolvers, rate limit peers, or
//! own listener state.

use alloc::collections::BTreeMap as HashMap;
use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use super::axfr::handle_notify;
use super::limits::parse_dns_message_bounded;
use super::message::{DnsMessage, DnsOpcode, DnsRecord, DnsResponseCode};
use super::name::validate_name;
use super::record::{DnsRecordData, DnsRecordType};
use super::zone::DnsZone;

/// Maximum UDP response size before truncation is required (RFC 1035).
pub const MAX_UDP_RESPONSE: usize = 512;

/// Query parse failure; callers should respond with FORMERR.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParseError;

/// Process a DNS query without opening upstream sockets.
///
/// This is the protocol-only authoritative path for runtimes that provide
/// their own forwarding adapter or intentionally disable recursion.
pub fn handle_query_without_forwarding(
    wire: &[u8],
    zones: &HashMap<String, DnsZone>,
) -> Result<(Vec<u8>, bool), ParseError> {
    let query = parse_dns_message_bounded(wire).map_err(|_| ParseError)?;

    if query.header.is_response {
        return Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::FormErr, Vec::new()).to_wire(),
            false,
        ));
    }

    match query.header.opcode {
        DnsOpcode::Query => handle_standard_query(&query, zones),
        DnsOpcode::Notify => handle_notify_query(&query),
        DnsOpcode::Update => handle_update_query(&query),
        DnsOpcode::IQuery | DnsOpcode::Status => Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::NotImp, Vec::new()).to_wire(),
            false,
        )),
    }
}

/// Convert a query result into the UDP response bytes that should be sent.
///
/// If the protocol response is too large for UDP, this builds the RFC 1035
/// truncated response from the original query. Socket adapters should not need
/// to parse DNS messages just to handle UDP truncation.
pub fn udp_response_wire(query_wire: &[u8], response_wire: Vec<u8>, needs_tcp: bool) -> Vec<u8> {
    if !needs_tcp {
        return response_wire;
    }

    match parse_dns_message_bounded(query_wire) {
        Ok(query) => {
            let mut response =
                DnsMessage::response(query.header.id, DnsResponseCode::NoError, Vec::new());
            response.header.truncated = true;
            response.questions = query.questions;
            response.header.question_count = response.questions.len() as u16;
            response.to_wire()
        }
        Err(_) => response_wire,
    }
}

fn handle_standard_query(
    query: &DnsMessage,
    zones: &HashMap<String, DnsZone>,
) -> Result<(Vec<u8>, bool), ParseError> {
    let question = match query.questions.first() {
        Some(q) => q,
        None => {
            return Ok((
                DnsMessage::response(query.header.id, DnsResponseCode::FormErr, Vec::new())
                    .to_wire(),
                false,
            ));
        }
    };

    if question.qtype == DnsRecordType::AXFR {
        return Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::Refused, Vec::new()).to_wire(),
            false,
        ));
    }

    let qname = question.name.to_lowercase();
    let qtype = question.qtype;

    if validate_name(&qname).is_err() {
        return Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::FormErr, Vec::new()).to_wire(),
            false,
        ));
    }

    let answers = resolve(&qname, qtype, zones);
    let negative = if answers.is_empty() {
        find_negative_response(&qname, zones)
    } else {
        None
    };
    let authority = if let Some((_, authority)) = &negative {
        authority.clone()
    } else if answers.is_empty() {
        find_matching_zone_soa(&qname, zones)
    } else {
        Vec::new()
    };

    if answers.is_empty() {
        let rcode = negative
            .map(|(rcode, _)| rcode)
            .unwrap_or(DnsResponseCode::NXDomain);
        let mut resp = DnsMessage::response(query.header.id, rcode, Vec::new());
        resp.questions = query.questions.clone();
        resp.header.question_count = 1;
        resp.authority = authority;
        return Ok((resp.to_wire(), false));
    }

    let mut response = DnsMessage::response(query.header.id, DnsResponseCode::NoError, answers);
    response.questions = query.questions.clone();
    response.header.question_count = 1;
    response.authority = authority;
    let wire = response.to_wire();
    let needs_tcp = wire.len() > MAX_UDP_RESPONSE;
    Ok((wire, needs_tcp))
}

/// Resolve a name+type query against the given zones.
pub fn resolve(
    qname: &str,
    qtype: DnsRecordType,
    zones: &HashMap<String, DnsZone>,
) -> Vec<DnsRecord> {
    let qname = qname.trim_end_matches('.').to_lowercase();
    if let Some(zone) = best_matching_zone(&qname, zones) {
        let origin = zone.origin.to_lowercase();
        let rname = relative_zone_name(&qname, &origin);

        if let Some(records) = zone.resolve(&rname, qtype) {
            return records;
        }

        if qtype != DnsRecordType::CNAME && qtype != DnsRecordType::ANY {
            if let Some(cname_records) = zone.resolve(&rname, DnsRecordType::CNAME) {
                for rr in &cname_records {
                    if let DnsRecordData::CNAME(target) = &rr.data {
                        let target_records = resolve(target, qtype, zones);
                        if !target_records.is_empty() {
                            let mut all = cname_records.clone();
                            all.extend(target_records);
                            return all;
                        }
                    }
                }
            }
        }
    }

    Vec::new()
}

fn find_matching_zone_soa(qname: &str, zones: &HashMap<String, DnsZone>) -> Vec<DnsRecord> {
    if let Some(zone) = best_matching_zone(qname, zones) {
        let origin = zone.origin.to_lowercase();
        if let Some(soa) = zone.resolve(&origin, DnsRecordType::SOA) {
            return soa;
        }
    }
    Vec::new()
}

fn find_negative_response(
    qname: &str,
    zones: &HashMap<String, DnsZone>,
) -> Option<(DnsResponseCode, Vec<DnsRecord>)> {
    let zone = best_matching_zone(qname, zones)?;
    let origin = zone.origin.to_lowercase();
    let rname = relative_zone_name(qname, &origin);
    let name_exists = zone.resolve(&rname, DnsRecordType::ANY).is_some();
    let mut authority = zone
        .resolve(&origin, DnsRecordType::SOA)
        .unwrap_or_default();
    if name_exists {
        if let Some(mut proof) = zone.resolve(&rname, DnsRecordType::NSEC) {
            authority.append(&mut proof);
        }
        return Some((DnsResponseCode::NoError, authority));
    }
    if let Some(mut proof) = all_nsec_proofs(zone) {
        authority.append(&mut proof);
    } else if let Some(mut proof) = find_nsec_covering(zone, qname) {
        authority.append(&mut proof);
    }
    Some((DnsResponseCode::NXDomain, authority))
}

fn find_nsec_covering(zone: &DnsZone, qname: &str) -> Option<Vec<DnsRecord>> {
    let qname = qname.trim_end_matches('.').to_ascii_lowercase();
    let mut names: Vec<String> = zone
        .names()
        .into_iter()
        .filter(|name| zone.resolve(name, DnsRecordType::NSEC).is_some())
        .map(str::to_string)
        .collect();
    names.sort_by(|left, right| dnssec_canonical_name_cmp(left, right));
    names.dedup();
    if names.is_empty() {
        return None;
    }
    for (index, owner) in names.iter().enumerate() {
        let next = &names[(index + 1) % names.len()];
        let owner_to_next = dnssec_canonical_name_cmp(owner, next);
        let owner_to_qname = dnssec_canonical_name_cmp(owner, &qname);
        let qname_to_next = dnssec_canonical_name_cmp(&qname, next);
        let covers = if owner_to_next == core::cmp::Ordering::Less {
            owner_to_qname == core::cmp::Ordering::Less
                && qname_to_next == core::cmp::Ordering::Less
        } else {
            owner_to_qname == core::cmp::Ordering::Less
                || qname_to_next == core::cmp::Ordering::Less
        };
        if covers || owner == &qname {
            return zone.resolve(owner, DnsRecordType::NSEC);
        }
    }
    zone.resolve(names.last()?, DnsRecordType::NSEC)
}

fn all_nsec_proofs(zone: &DnsZone) -> Option<Vec<DnsRecord>> {
    let mut proofs = Vec::new();
    for name in zone.names() {
        if let Some(mut records) = zone.resolve(name, DnsRecordType::NSEC) {
            proofs.append(&mut records);
        }
    }
    if proofs.is_empty() {
        None
    } else {
        Some(proofs)
    }
}

fn dnssec_canonical_name_cmp(left: &str, right: &str) -> core::cmp::Ordering {
    let left_lower = left.trim_end_matches('.').to_ascii_lowercase();
    let right_lower = right.trim_end_matches('.').to_ascii_lowercase();
    let left_labels: Vec<&str> = left_lower.split('.').collect();
    let right_labels: Vec<&str> = right_lower.split('.').collect();
    let mut left_iter = left_labels.iter().rev();
    let mut right_iter = right_labels.iter().rev();
    loop {
        match (left_iter.next(), right_iter.next()) {
            (Some(left), Some(right)) => match left.as_bytes().cmp(right.as_bytes()) {
                core::cmp::Ordering::Equal => {}
                order => return order,
            },
            (None, Some(_)) => return core::cmp::Ordering::Less,
            (Some(_), None) => return core::cmp::Ordering::Greater,
            (None, None) => return core::cmp::Ordering::Equal,
        }
    }
}

fn best_matching_zone<'a>(qname: &str, zones: &'a HashMap<String, DnsZone>) -> Option<&'a DnsZone> {
    let qname = qname.trim_end_matches('.').to_lowercase();
    zones
        .values()
        .filter(|zone| {
            let origin = zone.origin.to_lowercase();
            qname == origin || qname.ends_with(&format!(".{origin}"))
        })
        .max_by_key(|zone| zone.origin.len())
}

fn relative_zone_name(qname: &str, origin: &str) -> String {
    if qname == origin {
        "@".to_string()
    } else {
        qname[..qname.len() - origin.len() - 1].to_string()
    }
}

fn handle_notify_query(query: &DnsMessage) -> Result<(Vec<u8>, bool), ParseError> {
    match handle_notify(query) {
        Ok(response) => Ok((response.to_wire(), false)),
        Err(rcode) => Ok((
            DnsMessage::response(query.header.id, rcode, Vec::new()).to_wire(),
            false,
        )),
    }
}

fn handle_update_query(query: &DnsMessage) -> Result<(Vec<u8>, bool), ParseError> {
    Ok((
        DnsMessage::response(query.header.id, DnsResponseCode::Refused, Vec::new()).to_wire(),
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn axfr_is_refused_by_default() {
        let mut zones = HashMap::new();
        let mut zone = DnsZone::new("example.com");
        zone.add_soa("ns1.example.com", "admin.example.com");
        zones.insert("example.com".to_string(), zone);

        let query = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::AXFR);
        let Ok((wire, needs_tcp)) = handle_query_without_forwarding(&query.to_wire(), &zones)
        else {
            panic!("AXFR query should parse");
        };
        let response = DnsMessage::from_wire(&wire).unwrap();

        assert!(!needs_tcp);
        assert_eq!(response.header.response_code, DnsResponseCode::Refused);
    }

    #[test]
    fn dns_update_is_refused_by_default() {
        let zones = HashMap::new();
        let mut query = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::SOA);
        query.header.opcode = DnsOpcode::Update;

        let Ok((wire, needs_tcp)) = handle_query_without_forwarding(&query.to_wire(), &zones)
        else {
            panic!("UPDATE query should parse");
        };
        let response = DnsMessage::from_wire(&wire).unwrap();

        assert!(!needs_tcp);
        assert_eq!(response.header.response_code, DnsResponseCode::Refused);
    }

    #[test]
    fn udp_response_wire_synthesizes_truncated_response() {
        let query = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::A);
        let fallback = vec![1, 2, 3];
        let wire = udp_response_wire(&query.to_wire(), fallback, true);
        let response = DnsMessage::from_wire(&wire).unwrap();

        assert_eq!(response.header.id, 0x1234);
        assert!(response.header.truncated);
        assert_eq!(response.questions.len(), 1);
        assert_eq!(response.questions[0].name, "example.com");
    }

    #[test]
    fn udp_response_wire_keeps_non_truncated_response() {
        let response = vec![1, 2, 3];
        assert_eq!(udp_response_wire(&[], response.clone(), false), response);
    }

    #[test]
    fn resolve_prefers_longest_matching_zone() {
        let mut zones = HashMap::new();
        let mut parent = DnsZone::new("example.com");
        parent.add_soa("ns1.example.com", "admin.example.com");
        parent.add_record(DnsRecord::txt(
            "child.example.com".to_string(),
            "parent".to_string(),
            3600,
        ));
        zones.insert("example.com".to_string(), parent);

        let mut child = DnsZone::new("child.example.com");
        child.add_soa("ns1.example.com", "admin.example.com");
        child.add_record(DnsRecord::txt(
            "child.example.com".to_string(),
            "child".to_string(),
            3600,
        ));
        zones.insert("child.example.com".to_string(), child);

        let answers = resolve("child.example.com", DnsRecordType::TXT, &zones);
        assert_eq!(answers.len(), 1);
        assert!(matches!(&answers[0].data, DnsRecordData::TXT(value) if value == "child"));
    }

    #[test]
    fn ds_denial_for_existing_unsigned_name_includes_nsec() {
        let mut zones = HashMap::new();
        let mut zone = DnsZone::new("example.com");
        zone.add_soa("ns1.example.com", "admin.example.com");
        zone.add_record(DnsRecord::mx(
            "nodes.example.com".to_string(),
            0,
            "mail.example.com".to_string(),
            3600,
        ));
        zone.add_record(DnsRecord::nsec(
            "nodes.example.com".to_string(),
            "ns1.example.com".to_string(),
            vec![0, 3, 0, 0, 0x40],
            86400,
        ));
        zones.insert("example.com".to_string(), zone);

        let query = DnsMessage::query(0x1234, "nodes.example.com".to_string(), DnsRecordType::DS);
        let Ok((wire, needs_tcp)) = handle_query_without_forwarding(&query.to_wire(), &zones)
        else {
            panic!("DS query should parse");
        };
        let response = DnsMessage::from_wire(&wire).unwrap();

        assert!(!needs_tcp);
        assert_eq!(response.header.response_code, DnsResponseCode::NoError);
        assert!(response.answers.is_empty());
        assert!(response
            .authority
            .iter()
            .any(|record| record.rtype == DnsRecordType::NSEC));
    }
}
