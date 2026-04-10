//! DNS server — handles queries from a zone file.

use std::collections::HashMap;
use std::io;
use std::net::UdpSocket;
use std::time::Duration;

use super::message::{DnsMessage, DnsRecord, DnsResponseCode};
use super::record::{DnsRecordType, DnsRecordData};
use super::zone::DnsZone;

/// DNS server configuration.
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
/// # Example
/// ```no_run
/// use edgerun_dns::server::{DnsServer, DnsServerConfig};
/// use edgerun_dns::zone::DnsZone;
/// use std::net::Ipv4Addr;
///
/// let mut zone = DnsZone::new("example.com");
/// zone.add_a("@", Ipv4Addr::new(192, 168, 1, 1), 3600);
/// zone.add_a("www", Ipv4Addr::new(192, 168, 1, 2), 3600);
///
/// let config = DnsServerConfig::default();
/// let mut server = DnsServer::new(config).unwrap();
/// server.add_zone(zone);
/// server.run();
/// ```
pub struct DnsServer {
    socket: UdpSocket,
    zones: HashMap<String, DnsZone>,
    default_ttl: u32,
    /// Forward queries to upstream resolver if we don't know the answer.
    pub forward_to: Option<String>,
}

impl DnsServer {
    /// Create a new DNS server.
    pub fn new(config: DnsServerConfig) -> Result<Self, io::Error> {
        let socket = UdpSocket::bind(&config.bind_addr)?;
        socket.set_read_timeout(Some(Duration::from_millis(200)))?;

        Ok(Self {
            socket,
            zones: HashMap::new(),
            default_ttl: config.default_ttl,
            forward_to: None,
        })
    }

    /// Add a zone to this server.
    pub fn add_zone(&mut self, zone: DnsZone) {
        let origin = zone.origin.clone();
        self.zones.insert(origin, zone);
    }

    /// Remove a zone.
    pub fn remove_zone(&mut self, origin: &str) {
        self.zones.remove(origin);
    }

    /// Run the server event loop (blocking).
    pub fn run(&mut self) -> Result<(), io::Error> {
        eprintln!(
            "edgerun-dns: server listening on {}",
            self.socket.local_addr().unwrap()
        );
        eprintln!("  zones: {:?}", self.zones.keys().collect::<Vec<_>>());

        loop {
            match self.tick() {
                Ok(()) => {}
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
                Err(e) if e.kind() == io::ErrorKind::TimedOut => continue,
                Err(e) => {
                    eprintln!("edgerun-dns: server error: {}", e);
                    return Err(e);
                }
            }
        }
    }

    /// Process one incoming query. Call from your own event loop.
    pub fn tick(&mut self) -> Result<(), io::Error> {
        let mut buf = [0u8; 4096];
        let (n, src) = match self.socket.recv_from(&mut buf) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        let query = match DnsMessage::from_wire(&buf[..n]) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("edgerun-dns: parse error from {}: {}", src, e);
                // FORMERR
                let response = DnsMessage::response(0, DnsResponseCode::FormErr, Vec::new());
                let _ = self.socket.send_to(&response.to_wire(), src);
                return Ok(());
            }
        };

        // Must be a query
        if !query.header.is_response && query.header.opcode == super::message::DnsOpcode::Query {
            let response = self.handle_query(&query);
            let wire = response.to_wire();
            let _ = self.socket.send_to(&wire, src);
        }

        Ok(())
    }

    fn handle_query(&self, query: &DnsMessage) -> DnsMessage {
        let id = query.header.id;
        let question = match query.questions.first() {
            Some(q) => q,
            None => {
                return DnsMessage::response(id, DnsResponseCode::FormErr, Vec::new());
            }
        };

        let qname = question.name.to_lowercase();
        let qtype = question.qtype;

        eprintln!(
            "edgerun-dns: query {} {} from {}",
            qtype.as_str(),
            qname,
            self.socket.local_addr().unwrap()
        );

        // Try to find the answer in our zones
        let answers = self.resolve(&qname, qtype);

        if answers.is_empty() {
            // NXDOMAIN or REFUSED
            eprintln!("edgerun-dns: NXDOMAIN for {}", qname);
            DnsMessage::response(id, DnsResponseCode::NXDomain, Vec::new())
        } else {
            eprintln!("edgerun-dns: {} answer(s) for {}", answers.len(), qname);
            let mut response = DnsMessage::response(id, DnsResponseCode::NoError, answers);
            response.questions = query.questions.clone();
            response.header.question_count = 1;
            response
        }
    }

    fn resolve(&self, qname: &str, qtype: DnsRecordType) -> Vec<DnsRecord> {
        // Try exact match first, then strip labels to find zone
        for zone in self.zones.values() {
            // Check if qname is within this zone
            let zone_origin = zone.origin.to_lowercase();

            if qname == zone_origin || qname.ends_with(&format!(".{}", zone_origin)) {
                // This query is for our zone — compute the relative name
                let rname = if qname == zone_origin {
                    "@".to_string()
                } else {
                    qname[..qname.len() - zone_origin.len() - 1].to_string()
                };

                if let Some(records) = zone.resolve(&rname, qtype) {
                    return records;
                }

                // If it's a CNAME, follow it
                if qtype != DnsRecordType::CNAME && qtype != DnsRecordType::ANY {
                    if let Some(cname_records) = zone.resolve(&rname, DnsRecordType::CNAME) {
                        for rr in &cname_records {
                            if let DnsRecordData::CNAME(target) = &rr.data {
                                // Follow the CNAME chain
                                let target_records = self.resolve(&target, qtype);
                                if !target_records.is_empty() {
                                    let mut all = cname_records.clone();
                                    all.extend(target_records);
                                    return all;
                                }
                            }
                        }
                    }
                }

                // If we're here and the zone exists but has no matching record,
                // it's a valid zone but the specific record doesn't exist
                return Vec::new();
            }
        }

        // No zone matches
        Vec::new()
    }

    /// Get zone count.
    pub fn zone_count(&self) -> usize {
        self.zones.len()
    }

    /// Get the names of all loaded zones.
    pub fn zone_names(&self) -> Vec<&str> {
        self.zones.keys().map(|s| s.as_str()).collect()
    }
}
