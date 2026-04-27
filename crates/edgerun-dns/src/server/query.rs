//! DNS query processing — parsing, resolution, and upstream forwarding.

use alloc::{boxed::Box, format, string::{String, ToString}, vec, vec::Vec};
use alloc::collections::BTreeMap as HashMap;
use alloc::sync::Arc;

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
        DnsOpcode::Update => handle_update_query(&query, state).await,
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
        let zones_guard = state.zones.read().await;
        let qname = question.name.to_lowercase();
        let zone = zones_guard.get(&qname).cloned();
        drop(zones_guard);

        if let Some(zone) = zone {
            match crate::axfr::handle_axfr(query, &zone) {
                Ok(messages) => {
                    // Return the first message (SOA envelope); remaining messages
                    // are handled by the TCP handler for multi-message responses.
                    let wire = messages[0].to_wire();
                    return Ok((wire, true)); // needs TCP for full transfer
                }
                Err(rcode) => {
                    return Ok((
                        DnsMessage::response(query.header.id, rcode, Vec::new()).to_wire(),
                        false,
                    ));
                }
            }
        } else {
            return Ok((
                DnsMessage::response(query.header.id, DnsResponseCode::NotAuth, Vec::new())
                    .to_wire(),
                false,
            ));
        }
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
    let (answers, authority) = {
        let zones_guard = state.zones.read().await;
        let answers = resolve(&qname, qtype, &zones_guard);
        let authority = if answers.is_empty() {
            find_matching_zone_soa(&qname, &zones_guard)
        } else {
            Vec::new()
        };
        (answers, authority)
    };
    // zones_guard dropped here ^

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
fn find_matching_zone_soa(qname: &str, zones: &HashMap<String, DnsZone>) -> Vec<DnsRecord> {
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
) -> Result<Vec<u8>, crate::std::io::Error> {
    use crate::std::net::SocketAddr;

    let socket = AsyncUdpSocket::bind("0.0.0.0:0")?;
    let target: SocketAddr = upstream_addr.parse().map_err(|_| {
        crate::std::io::Error::new(crate::std::io::ErrorKind::InvalidInput, "bad upstream address")
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
    .map_err(|_| crate::std::io::Error::new(crate::std::io::ErrorKind::TimedOut, "upstream query timed out"))??;

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
    }

    Vec::new()
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
async fn handle_update_query(
    query: &DnsMessage,
    state: &ServerState,
) -> Result<(Vec<u8>, bool), ParseError> {
    let zone_question = match query.questions.first() {
        Some(q) => q,
        None => {
            return Ok((
                DnsMessage::response(query.header.id, DnsResponseCode::FormErr, Vec::new())
                    .to_wire(),
                false,
            ))
        }
    };

    let zone_name = zone_question.name.to_lowercase();

    // We need to hold the write lock to modify the zone
    // Since we can't hold it across the axfr::handle_update call easily,
    // we do the update inline here.
    let mut zones_guard = state.zones.write().await;
    let zone = zones_guard.get_mut(&zone_name);

    match zone {
        Some(zone) => match crate::axfr::handle_update(query, zone, None) {
            Ok(response) => Ok((response.to_wire(), false)),
            Err(rcode) => Ok((
                DnsMessage::response(query.header.id, rcode, Vec::new()).to_wire(),
                false,
            )),
        },
        None => Ok((
            DnsMessage::response(query.header.id, DnsResponseCode::NotAuth, Vec::new()).to_wire(),
            false,
        )),
    }
}
