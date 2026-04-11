//! Async DNS server — handles queries concurrently using edgerun-rt.
//!
//! The server uses a non-blocking UDP socket managed by the edgerun-rt
//! epoll reactor. Each incoming query is spawned as a separate async task
//! via `edgerun_rt::spawn`, so slow lookups never block other queries.

use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use edgerun_rt::AsyncUdpSocket;

use super::message::{DnsMessage, DnsResponseCode};
use super::record::DnsRecordType;
use super::zone::DnsZone;

/// DNS server configuration.
#[derive(Debug, Clone)]
pub struct DnsServerConfig {
    /// Bind address (e.g. "0.0.0.0:53").
    pub bind_addr: String,
    /// Default TTL for records.
    pub default_ttl: u32,
}

impl Default for DnsServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:53".to_string(),
            default_ttl: 3600,
        }
    }
}

/// DNS server — authoritative server for one or more zones.
///
/// Runs on the edgerun-rt async runtime. Each query is handled
/// concurrently via `edgerun_rt::spawn`.
///
/// # Example
/// ```no_run
/// use edgerun_dns::server::{DnsServer, DnsServerConfig};
/// use edgerun_dns::zone::DnsZone;
/// use edgerun_rt::Runtime;
/// use std::net::Ipv4Addr;
///
/// let rt = Runtime::new_multi_thread().enable_all().build().unwrap();
/// rt.block_on(async {
///     let mut zone = DnsZone::new("example.com");
///     zone.add_a("@", Ipv4Addr::new(192, 168, 1, 1), 3600);
///     zone.add_a("www", Ipv4Addr::new(192, 168, 1, 2), 3600);
///
///     let config = DnsServerConfig::default();
///     let server = DnsServer::new(config).unwrap();
///     server.add_zone(zone).await;
///     server.run().await;
/// });
/// ```
pub struct DnsServer {
    socket: AsyncUdpSocket,
    zones: Arc<edgerun_rt::RwLock<HashMap<String, DnsZone>>>,
    default_ttl: u32,
    /// Forward queries to upstream resolver if we don't know the answer.
    pub forward_to: Option<String>,
}

impl DnsServer {
    /// Create a new DNS server.
    pub fn new(config: DnsServerConfig) -> Result<Self, io::Error> {
        let socket = AsyncUdpSocket::bind(&config.bind_addr)?;

        edgerun_log::info!(
            "edgerun-dns: server bound to {}",
            socket.local_addr().unwrap()
        );

        Ok(Self {
            socket,
            zones: Arc::new(edgerun_rt::RwLock::new(HashMap::new())),
            default_ttl: config.default_ttl,
            forward_to: None,
        })
    }

    /// Add a zone to this server.
    pub async fn add_zone(&self, zone: DnsZone) {
        let origin = zone.origin.clone();
        self.zones.write().await.insert(origin, zone);
    }

    /// Remove a zone.
    pub async fn remove_zone(&self, origin: &str) {
        self.zones.write().await.remove(origin);
    }

    /// Run the server event loop (async, runs forever).
    ///
    /// Listens for incoming UDP packets and spawns a task for each query.
    pub async fn run(&self) -> ! {
        let local = self.socket.local_addr().unwrap();
        let zone_names: Vec<_> = self.zones.read().await.keys().cloned().collect();
        edgerun_log::info!("edgerun-dns: server listening on {}", local);
        edgerun_log::info!("  zones: {:?}", zone_names);

        // We need &mut access to the socket for poll_recv_from/poll_send_to.
        // Since AsyncUdpSocket uses interior mutability (raw fd + Arc refs),
        // we can safely use UnsafeCell to get &mut while only mutating internals.
        let socket_cell = std::cell::UnsafeCell::new(self.socket.clone());

        loop {
            let mut buf = [0u8; 4096];

            // Wait for incoming data — registers with epoll reactor.
            let (n, src) = match {
                let socket_mut = unsafe { &mut *socket_cell.get() };
                poll_recv_from(socket_mut, &mut buf).await
            } {
                Ok(v) => v,
                Err(e) => {
                    edgerun_log::warn!("edgerun-dns: recv error: {}", e);
                    continue;
                }
            };

            // Clone what the spawned task needs.
            let zones = Arc::clone(&self.zones);
            let default_ttl = self.default_ttl;
            let forward_to = self.forward_to.clone();
            let socket = self.socket.clone();
            let query_buf = buf[..n].to_vec();

            // Spawn concurrent handler for this query.
            edgerun_rt::spawn(async move {
                match handle_query(&query_buf, &zones, default_ttl, &forward_to).await {
                    Ok(response_wire) => {
                        let mut socket_mut = socket;
                        let _ = poll_send_to(&mut socket_mut, &response_wire, src).await;
                    }
                    Err(ParseError) => {
                        let response =
                            DnsMessage::response(0, DnsResponseCode::FormErr, Vec::new());
                        let mut socket_mut = socket;
                        let _ = poll_send_to(&mut socket_mut, &response.to_wire(), src).await;
                    }
                }
            });
        }
    }

    /// Get zone count.
    pub async fn zone_count(&self) -> usize {
        self.zones.read().await.len()
    }

    /// Get the names of all loaded zones.
    pub async fn zone_names(&self) -> Vec<String> {
        self.zones.read().await.keys().cloned().collect()
    }
}

// ---------------------------------------------------------------------------
// Async wrappers around AsyncUdpSocket poll methods
// ---------------------------------------------------------------------------

async fn poll_recv_from(
    socket: &mut AsyncUdpSocket,
    buf: &mut [u8],
) -> io::Result<(usize, SocketAddr)> {
    use std::future::poll_fn;
    poll_fn(|cx| Pin::new(&mut *socket).poll_recv_from(cx, buf)).await
}

async fn poll_send_to(
    socket: &mut AsyncUdpSocket,
    buf: &[u8],
    target: SocketAddr,
) -> io::Result<usize> {
    use std::future::poll_fn;
    poll_fn(|cx| Pin::new(&mut *socket).poll_send_to(cx, buf, target)).await
}

// ---------------------------------------------------------------------------
// Query handling
// ---------------------------------------------------------------------------

struct ParseError;

async fn handle_query(
    wire: &[u8],
    zones: &Arc<edgerun_rt::RwLock<HashMap<String, DnsZone>>>,
    _default_ttl: u32,
    _forward_to: &Option<String>,
) -> Result<Vec<u8>, ParseError> {
    let query = DnsMessage::from_wire(wire).map_err(|_| ParseError)?;

    if query.header.is_response || query.header.opcode != super::message::DnsOpcode::Query {
        return Ok(DnsMessage::response(query.header.id, DnsResponseCode::NotImp, Vec::new()).to_wire());
    }

    let question = match query.questions.first() {
        Some(q) => q,
        None => {
            return Ok(DnsMessage::response(query.header.id, DnsResponseCode::FormErr, Vec::new()).to_wire());
        }
    };

    let qname = question.name.to_lowercase();
    let qtype = question.qtype;

    edgerun_log::debug!("edgerun-dns: query {} {} (concurrent)", qtype.as_str(), qname);

    let zones_guard = zones.read().await;
    let answers = resolve(&qname, qtype, &zones_guard);
    drop(zones_guard);

    if answers.is_empty() {
        edgerun_log::debug!("edgerun-dns: NXDOMAIN for {}", qname);
        Ok(DnsMessage::response(query.header.id, DnsResponseCode::NXDomain, Vec::new()).to_wire())
    } else {
        edgerun_log::debug!("edgerun-dns: {} answer(s) for {}", answers.len(), qname);
        let mut response = DnsMessage::response(query.header.id, DnsResponseCode::NoError, answers);
        response.questions = query.questions.clone();
        response.header.question_count = 1;
        Ok(response.to_wire())
    }
}

fn resolve(
    qname: &str,
    qtype: DnsRecordType,
    zones: &HashMap<String, DnsZone>,
) -> Vec<super::message::DnsRecord> {
    for zone in zones.values() {
        let zone_origin = zone.origin.to_lowercase();

        if qname == zone_origin || qname.ends_with(&format!(".{}", zone_origin)) {
            let rname = if qname == zone_origin {
                "@".to_string()
            } else {
                qname[..qname.len() - zone_origin.len() - 1].to_string()
            };

            if let Some(records) = zone.resolve(&rname, qtype) {
                return records;
            }

            // Follow CNAME
            if qtype != DnsRecordType::CNAME && qtype != DnsRecordType::ANY {
                if let Some(cname_records) = zone.resolve(&rname, DnsRecordType::CNAME) {
                    for rr in &cname_records {
                        if let super::record::DnsRecordData::CNAME(target) = &rr.data {
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
