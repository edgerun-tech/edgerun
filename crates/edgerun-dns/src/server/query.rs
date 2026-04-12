//! DNS query processing — parsing, resolution, and upstream forwarding.

use std::collections::HashMap;
use std::sync::Arc;

use edgerun_rt::AsyncUdpSocket;

use crate::message::{DnsMessage, DnsOpcode, DnsResponseCode, DnsRecord};
use crate::name::validate_name;
use crate::record::{DnsRecordData, DnsRecordType};
use crate::zone::DnsZone;

/// Maximum UDP response size before truncation is required (RFC 1035).
pub const MAX_UDP_RESPONSE: usize = 512;

/// Shared server state — cloned into spawned tasks.
#[derive(Clone)]
pub struct ServerState {
    /// DNS zones, keyed by origin domain name.
    pub zones: Arc<edgerun_rt::RwLock<HashMap<String, DnsZone>>>,
    /// Default TTL for newly created records.
    pub default_ttl: u32,
    /// Upstream resolver address for recursive forwarding (runtime-configurable).
    pub forward_to: Arc<edgerun_rt::RwLock<Option<String>>>,
}

/// Query parse failure — respond with FORMERR.
pub struct ParseError;

/// Process a DNS query and return `(response_wire, needs_tcp)`.
///
/// If the response exceeds `MAX_UDP_RESPONSE` bytes, `needs_tcp` is true.
pub async fn handle_query(
    wire: &[u8],
    state: &ServerState,
) -> Result<(Vec<u8>, bool), ParseError> {
    let query = DnsMessage::from_wire(wire).map_err(|_| ParseError)?;

    if query.header.is_response || query.header.opcode != DnsOpcode::Query {
        return Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::NotImp, Vec::new()).to_wire(),
            false,
        ));
    }

    let question = match query.questions.first() {
        Some(q) => q,
        None => {
            return Ok((
                DnsMessage::response(query.header.id, DnsResponseCode::FormErr, Vec::new()).to_wire(),
                false,
            ));
        }
    };

    let qname = question.name.to_lowercase();
    let qtype = question.qtype;

    if validate_name(&qname).is_err() {
        edgerun_log::debug!("edgerun-dns: invalid name '{}', FORMERR", qname);
        return Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::FormErr, Vec::new()).to_wire(),
            false,
        ));
    }

    edgerun_log::debug!("edgerun-dns: query {} {} (concurrent)", qtype.as_str(), qname);

    let zones_guard = state.zones.read().await;
    let answers = resolve(&qname, qtype, &zones_guard);

    // Include SOA in authority section per RFC 2308
    let authority = if answers.is_empty() {
        find_matching_zone_soa(&qname, &zones_guard)
    } else {
        Vec::new()
    };
    drop(zones_guard);

    if answers.is_empty() {
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
fn find_matching_zone_soa(
    qname: &str,
    zones: &HashMap<String, DnsZone>,
) -> Vec<DnsRecord> {
    for zone in zones.values() {
        let origin = zone.origin.to_lowercase();
        if qname == origin || qname.ends_with(&format!(".{}", origin)) {
            if let Some(soa) = zone.resolve(&origin, DnsRecordType::SOA) {
                return soa;
            }
        }
    }
    Vec::new()
}

/// Forward a query to an upstream resolver and return the raw response.
pub async fn forward_query(
    upstream_addr: &str,
    query_wire: &[u8],
) -> Result<Vec<u8>, std::io::Error> {
    use std::net::SocketAddr;

    let socket = AsyncUdpSocket::bind("0.0.0.0:0")?;
    let target: SocketAddr = upstream_addr.parse().map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad upstream address")
    })?;

    // Send the query
    socket.send_to(query_wire, target).await?;

    // Receive response with 5s timeout
    let mut buf = [0u8; 4096];
    let (n, _) = edgerun_rt::timeout(
        std::time::Duration::from_secs(5),
        socket.recv_from(&mut buf),
    )
    .await
    .map_err(|_| std::io::Error::new(std::io::ErrorKind::TimedOut, "upstream query timed out"))??;

    Ok(buf[..n].to_vec())
}

/// Resolve a name+type query against the given zones.
pub fn resolve(
    qname: &str,
    qtype: DnsRecordType,
    zones: &HashMap<String, DnsZone>,
) -> Vec<DnsRecord> {
    for zone in zones.values() {
        let origin = zone.origin.to_lowercase();

        if qname == origin || qname.ends_with(&format!(".{}", origin)) {
            let rname = if qname == origin {
                "@".to_string()
            } else {
                qname[..qname.len() - origin.len() - 1].to_string()
            };

            if let Some(records) = zone.resolve(&rname, qtype) {
                return records;
            }

            // Follow CNAME
            if qtype != DnsRecordType::CNAME && qtype != DnsRecordType::ANY {
                if let Some(cname_records) = zone.resolve(&rname, DnsRecordType::CNAME) {
                    for rr in &cname_records {
                        if let DnsRecordData::CNAME(target) = &rr.data {
                            let target_records = resolve(&target, qtype, zones);
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
    }

    Vec::new()
}
