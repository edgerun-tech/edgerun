//! DNS query processing — parsing, resolution, and upstream forwarding.

use alloc::collections::BTreeMap as HashMap;
use alloc::sync::Arc;
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crate::compat::AsyncUdpSocket;

use crate::message::{DnsMessage, DnsOpcode, DnsRecord, DnsResponseCode};
use crate::name::validate_name;
use crate::record::{DnsRecordData, DnsRecordType};
use crate::zone::DnsZone;

/// Maximum UDP response size before truncation is required (RFC 1035).
pub const MAX_UDP_RESPONSE: usize = 512;

/// Shared server state — cloned into spawned tasks.
#[derive(Clone)]
pub struct ServerState {
    /// DNS zones, keyed by origin domain name.
    pub zones: Arc<crate::compat::RwLock<HashMap<String, DnsZone>>>,
    /// Default TTL for newly created records.
    pub default_ttl: u32,
    /// Upstream resolver address for recursive forwarding (runtime-configurable).
    pub forward_to: Arc<crate::compat::RwLock<Option<String>>>,
}

/// Query parse failure — respond with FORMERR.
pub struct ParseError;

/// Process a DNS query and return `(response_wire, needs_tcp)`.
///
/// If the response exceeds `MAX_UDP_RESPONSE` bytes, `needs_tcp` is true.
pub async fn handle_query(wire: &[u8], state: &ServerState) -> Result<(Vec<u8>, bool), ParseError> {
    let query = DnsMessage::from_wire(wire).map_err(|_| ParseError)?;

    if query.header.is_response {
        return Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::FormErr, Vec::new()).to_wire(),
            false,
        ));
    }

    // Dispatch by opcode
    match query.header.opcode {
        DnsOpcode::Query => handle_standard_query(&query, wire, state).await,
        DnsOpcode::Notify => handle_notify_query(&query).await,
        DnsOpcode::Update => handle_update_query(&query).await,
        DnsOpcode::IQuery | DnsOpcode::Status => Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::NotImp, Vec::new()).to_wire(),
            false,
        )),
    }
}

/// Handle a standard DNS query.
async fn handle_standard_query(
    query: &DnsMessage,
    wire: &[u8],
    state: &ServerState,
) -> Result<(Vec<u8>, bool), ParseError> {
    // Handle AXFR queries (zone transfer — must be over TCP, but we handle it here)
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
        edgerun_log::warn!("edgerun-dns: refused AXFR for {}", question.name);
        return Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::Refused, Vec::new()).to_wire(),
            false,
        ));
    }

    let qname = question.name.to_lowercase();
    let qtype = question.qtype;

    if validate_name(&qname).is_err() {
        edgerun_log::debug!("edgerun-dns: invalid name '{}', FORMERR", qname);
        return Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::FormErr, Vec::new()).to_wire(),
            false,
        ));
    }

    edgerun_log::debug!(
        "edgerun-dns: query {} {} (concurrent)",
        qtype.as_str(),
        qname
    );

    // Resolve while holding the zones lock, then drop it before any
    // other `.await` points. The borrow checker can't prove that
    // `answers`/`authority` outlive the guard unless we scope it.
    let (answers, authority, negative) = {
        let zones_guard = state.zones.read().await;
        let answers = resolve(&qname, qtype, &zones_guard);
        let negative = if answers.is_empty() {
            find_negative_response(&qname, &zones_guard)
        } else {
            None
        };
        let authority = if let Some((_, authority)) = &negative {
            authority.clone()
        } else if answers.is_empty() {
            find_matching_zone_soa(&qname, &zones_guard)
        } else {
            Vec::new()
        };
        (answers, authority, negative.map(|(rcode, _)| rcode))
    };
    // zones_guard dropped here ^

    if answers.is_empty() {
        if let Some(rcode) = negative {
            edgerun_log::debug!("edgerun-dns: local negative response for {}", qname);
            let mut resp = DnsMessage::response(query.header.id, rcode, Vec::new());
            resp.questions = query.questions.clone();
            resp.header.question_count = 1;
            resp.authority = authority;
            return Ok((resp.to_wire(), false));
        }

        // Try forwarding if upstream is configured
        let forward_addr = state.forward_to.read().await.clone();
        if let Some(ref upstream) = forward_addr {
            match forward_query(upstream, wire).await {
                Ok(forwarded) => {
                    edgerun_log::debug!("edgerun-dns: forwarded {} to upstream", qname);
                    return Ok((forwarded, false));
                }
                Err(e) => {
                    edgerun_log::warn!("edgerun-dns: forward failed: {}", e);
                    // Fall through to local NXDOMAIN
                }
            }
        }

        edgerun_log::debug!("edgerun-dns: NXDOMAIN for {}", qname);
        let mut resp = DnsMessage::response(query.header.id, DnsResponseCode::NXDomain, Vec::new());
        resp.questions = query.questions.clone();
        resp.header.question_count = 1;
        resp.authority = authority;
        return Ok((resp.to_wire(), false));
    }

    edgerun_log::debug!("edgerun-dns: {} answer(s) for {}", answers.len(), qname);
    let mut response = DnsMessage::response(query.header.id, DnsResponseCode::NoError, answers);
    response.questions = query.questions.clone();
    response.header.question_count = 1;
    response.authority = authority;
    let wire = response.to_wire();
    let needs_tcp = wire.len() > MAX_UDP_RESPONSE;
    Ok((wire, needs_tcp))
}

/// Find the SOA record for the zone that would match this query name.
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

/// Forward a query to an upstream resolver and return the raw response.
pub async fn forward_query(
    upstream_addr: &str,
    query_wire: &[u8],
) -> Result<Vec<u8>, crate::std::io::Error> {
    use crate::std::net::SocketAddr;

    let socket = AsyncUdpSocket::bind("0.0.0.0:0")?;
    let target: SocketAddr = upstream_addr.parse().map_err(|_| {
        crate::std::io::Error::new(
            crate::std::io::ErrorKind::InvalidInput,
            "bad upstream address",
        )
    })?;

    // Send the query
    socket.send_to(query_wire, target).await?;

    // Receive response with 5s timeout
    let mut buf = [0u8; 4096];
    let (n, _) = crate::compat::timeout(
        crate::std::time::Duration::from_secs(5),
        socket.recv_from(&mut buf),
    )
    .await
    .map_err(|_| {
        crate::std::io::Error::new(
            crate::std::io::ErrorKind::TimedOut,
            "upstream query timed out",
        )
    })??;

    Ok(buf[..n].to_vec())
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

        // Follow CNAME
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

        return Vec::new();
    }

    Vec::new()
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

/// Handle a NOTIFY query (RFC 1996).
async fn handle_notify_query(query: &DnsMessage) -> Result<(Vec<u8>, bool), ParseError> {
    match crate::axfr::handle_notify(query) {
        Ok(response) => Ok((response.to_wire(), false)),
        Err(rcode) => Ok((
            DnsMessage::response(query.header.id, rcode, Vec::new()).to_wire(),
            false,
        )),
    }
}

/// Handle a DNS Update query (RFC 2136).
async fn handle_update_query(query: &DnsMessage) -> Result<(Vec<u8>, bool), ParseError> {
    edgerun_log::warn!("edgerun-dns: refused unauthenticated DNS UPDATE");
    Ok((
        DnsMessage::response(query.header.id, DnsResponseCode::Refused, Vec::new()).to_wire(),
        false,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> ServerState {
        ServerState {
            zones: Arc::new(crate::compat::RwLock::new(HashMap::new())),
            default_ttl: 3600,
            forward_to: Arc::new(crate::compat::RwLock::new(None)),
        }
    }

    #[test]
    fn axfr_is_refused_by_default() {
        let rt = edgerun_rt::Runtime::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let state = test_state();
            let mut zone = DnsZone::new("example.com");
            zone.add_soa("ns1.example.com", "admin.example.com");
            state
                .zones
                .write()
                .await
                .insert("example.com".to_string(), zone);

            let query = DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::AXFR);
            let Ok((wire, needs_tcp)) = handle_query(&query.to_wire(), &state).await else {
                panic!("AXFR query should parse");
            };
            let response = DnsMessage::from_wire(&wire).unwrap();

            assert!(!needs_tcp);
            assert_eq!(response.header.response_code, DnsResponseCode::Refused);
        });
    }

    #[test]
    fn dns_update_is_refused_by_default() {
        let rt = edgerun_rt::Runtime::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let state = test_state();
            let mut query =
                DnsMessage::query(0x1234, "example.com".to_string(), DnsRecordType::SOA);
            query.header.opcode = DnsOpcode::Update;

            let Ok((wire, needs_tcp)) = handle_query(&query.to_wire(), &state).await else {
                panic!("UPDATE query should parse");
            };
            let response = DnsMessage::from_wire(&wire).unwrap();

            assert!(!needs_tcp);
            assert_eq!(response.header.response_code, DnsResponseCode::Refused);
        });
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
        let rt = edgerun_rt::Runtime::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();
        rt.block_on(async {
            let state = test_state();
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
            state
                .zones
                .write()
                .await
                .insert("example.com".to_string(), zone);

            let query =
                DnsMessage::query(0x1234, "nodes.example.com".to_string(), DnsRecordType::DS);
            let Ok((wire, needs_tcp)) = handle_query(&query.to_wire(), &state).await else {
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
        });
    }
}
