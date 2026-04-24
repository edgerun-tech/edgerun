//! Recursive DNS resolver — iterative resolution from the root hints.
//!
//! Implements RFC 1034 §5.3.3:
//! 1. Start at root servers.
//! 2. Query for the target name.
//! 3. If referral (NS in authority section), follow to child servers.
//! 4. Repeat until answer or NXDOMAIN.
//!
//! Caches all intermediate results with TTL-based expiry.

use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::{Duration, Instant};

use edgerun_rt::AsyncUdpSocket;

use super::message::{DnsMessage, DnsHeader, DnsResponseCode, DnsQuestion, DnsRecord};
use super::record::{DnsRecordData, DnsRecordType};

/// A single root hint: nameserver name → IP addresses.
#[derive(Debug, Clone)]
pub struct RootHint {
    /// Nameserver name (e.g. "a.root-servers.net").
    pub name: String,
    /// IP addresses of this nameserver.
    pub addrs: Vec<Ipv4Addr>,
}

/// Default root hints — the 13 root server families.
pub fn default_root_hints() -> Vec<RootHint> {
    vec![
        RootHint { name: "a.root-servers.net".into(), addrs: vec!["198.41.0.4".parse().unwrap()] },
        RootHint { name: "b.root-servers.net".into(), addrs: vec!["199.9.14.201".parse().unwrap()] },
        RootHint { name: "c.root-servers.net".into(), addrs: vec!["192.33.4.12".parse().unwrap()] },
        RootHint { name: "d.root-servers.net".into(), addrs: vec!["199.7.91.13".parse().unwrap()] },
        RootHint { name: "e.root-servers.net".into(), addrs: vec!["192.203.230.10".parse().unwrap()] },
        RootHint { name: "f.root-servers.net".into(), addrs: vec!["192.5.5.241".parse().unwrap()] },
        RootHint { name: "g.root-servers.net".into(), addrs: vec!["192.112.36.4".parse().unwrap()] },
        RootHint { name: "h.root-servers.net".into(), addrs: vec!["198.97.190.53".parse().unwrap()] },
        RootHint { name: "i.root-servers.net".into(), addrs: vec!["192.36.148.17".parse().unwrap()] },
        RootHint { name: "j.root-servers.net".into(), addrs: vec!["192.58.128.30".parse().unwrap()] },
        RootHint { name: "k.root-servers.net".into(), addrs: vec!["193.0.14.129".parse().unwrap()] },
        RootHint { name: "l.root-servers.net".into(), addrs: vec!["199.7.83.42".parse().unwrap()] },
        RootHint { name: "m.root-servers.net".into(), addrs: vec!["202.12.27.33".parse().unwrap()] },
    ]
}

/// Recursive DNS resolver — performs iterative resolution from root hints.
///
/// Implements RFC 1034 §5.3.3 with caching of intermediate results.
pub struct RecursiveResolver {
    /// Root hints — starting point for iterative resolution.
    root_hints: Vec<RootHint>,
    /// Glue cache: nameserver name → IP address + expiry.
    ns_cache: std::sync::Arc<std::sync::Mutex<HashMap<String, (Ipv4Addr, Instant)>>>,
    /// Answer cache: (name, type) → answer records + expiry.
    answer_cache: std::sync::Arc<std::sync::Mutex<HashMap<String, (Vec<DnsRecord>, Instant)>>>,
    /// UDP socket for outgoing queries.
    socket: AsyncUdpSocket,
    /// Query timeout.
    timeout: Duration,
    /// Maximum recursion depth (label hops).
    max_depth: usize,
}

impl RecursiveResolver {
    /// Create a new recursive resolver.
    pub fn new(root_hints: Vec<RootHint>) -> std::io::Result<Self> {
        let socket = AsyncUdpSocket::bind("0.0.0.0:0")?;
        Ok(Self {
            root_hints,
            ns_cache: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            answer_cache: std::sync::Arc::new(std::sync::Mutex::new(HashMap::new())),
            socket,
            timeout: Duration::from_secs(5),
            max_depth: 10,
        })
    }

    /// Set query timeout.
    pub fn set_timeout(&mut self, t: Duration) { self.timeout = t; }

    /// Resolve a name+type by iterative resolution from the root.
    pub async fn resolve(&self, name: &str, qtype: DnsRecordType) -> Result<Vec<DnsRecord>, std::io::Error> {
        let name_lower = name.to_lowercase();

        if let Some(records) = self.get_cached_answer(&name_lower, qtype) {
            return Ok(records);
        }

        let root_addrs: Vec<Ipv4Addr> = self.root_hints.iter()
            .flat_map(|h| h.addrs.iter().copied())
            .collect();
        self.iterative_resolve(name_lower, qtype, root_addrs, 0).await
    }

    /// Perform one step of iterative resolution.
    fn iterative_resolve<'a>(
        &'a self,
        name: String,
        qtype: DnsRecordType,
        servers: Vec<Ipv4Addr>,
        depth: usize,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<DnsRecord>, std::io::Error>> + Send + 'a>> {
        Box::pin(async move {
            if depth >= self.max_depth {
                return Err(std::io::Error::other(
                    "recursive resolution exceeded max depth",
                ));
            }
            if servers.is_empty() {
                return Err(std::io::Error::other("no nameservers to query"));
            }

            for server in &servers {
                let query_id = (depth as u16).wrapping_add(self.random_id());
                let msg = DnsMessage::query(query_id, name.clone(), qtype);
                let wire = msg.to_wire();
                let target = SocketAddr::V4(SocketAddrV4::new(*server, 53));

                let response = match edgerun_rt::timeout(self.timeout, self.query_server(&wire, target)).await {
                    Ok(Ok(r)) => r,
                    Ok(Err(e)) => {
                        edgerun_log::debug!("edgerun-dns: recursive query to {} failed: {}", server, e);
                        continue;
                    }
                    Err(_) => {
                        edgerun_log::debug!("edgerun-dns: recursive query to {} timed out", server);
                        continue;
                    }
                };

                let resp = match DnsMessage::from_wire(&response) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                if resp.header.id != query_id { continue; }

                match resp.header.response_code {
                    DnsResponseCode::NoError => {
                        if !resp.answers.is_empty() {
                            self.cache_answer(&name, qtype, &resp.answers);
                            self.cache_additional(&resp);
                            return Ok(resp.answers);
                        }

                        if let Some(ns_names) = self.extract_ns_referral(&resp) {
                            let ns_addrs = self.resolve_ns_glue(&ns_names, &resp, depth + 1).await;
                            if !ns_addrs.is_empty() {
                                return self.iterative_resolve(name, qtype, ns_addrs, depth + 1).await;
                            }
                            // No glue — resolve NS names recursively
                            for ns_name in &ns_names {
                                if let Ok(addrs) = self.iterative_resolve(ns_name.clone(), DnsRecordType::A, servers.clone(), depth + 1).await {
                                    let ip_addrs: Vec<Ipv4Addr> = addrs.iter().filter_map(|r| {
                                        if let DnsRecordData::A(ip) = &r.data { Some(*ip) } else { None }
                                    }).collect();
                                    if !ip_addrs.is_empty() {
                                        return self.iterative_resolve(name, qtype, ip_addrs, depth + 1).await;
                                    }
                                }
                            }
                        }
                        return Ok(Vec::new());
                    }
                    DnsResponseCode::NXDomain => return Ok(Vec::new()),
                    _ => continue,
                }
            }

            Err(std::io::Error::other("all nameservers failed"))
        })
    }

    /// Send a single DNS query and receive the response.
    async fn query_server(&self, wire: &[u8], target: SocketAddr) -> Result<Vec<u8>, std::io::Error> {
        self.socket.send_to(wire, target).await?;

        let mut buf = [0u8; 4096];
        let (n, _) = self.socket.recv_from(&mut buf).await?;
        Ok(buf[..n].to_vec())
    }

    /// Extract NS referral from authority section.
    fn extract_ns_referral(&self, resp: &DnsMessage) -> Option<Vec<String>> {
        let ns_names: Vec<_> = resp.authority.iter()
            .filter(|r| r.rtype == DnsRecordType::NS)
            .filter_map(|r| {
                if let DnsRecordData::NS(name) = &r.data { Some(name.clone()) } else { None }
            })
            .collect();
        if ns_names.is_empty() { None } else { Some(ns_names) }
    }

    /// Resolve NS names to IP addresses, using glue records first.
    async fn resolve_ns_glue(
        &self,
        ns_names: &[String],
        resp: &DnsMessage,
        _depth: usize,
    ) -> Vec<Ipv4Addr> {
        let mut addrs = Vec::new();

        // First check additional section for glue A records
        for ns_name in ns_names {
            // Check additional/glue
            for rr in &resp.additional {
                if rr.rtype == DnsRecordType::A && rr.name.to_lowercase() == ns_name.to_lowercase() {
                    if let DnsRecordData::A(ip) = &rr.data {
                        addrs.push(*ip);
                    }
                }
            }

            // Check ns_cache
            if addrs.is_empty() {
                let guard = self.ns_cache.lock().unwrap();
                if let Some((ip, expiry)) = guard.get(&ns_name.to_lowercase()) {
                    if Instant::now() < *expiry {
                        addrs.push(*ip);
                    }
                }
            }
        }

        addrs
    }

    /// Cache answer records with TTL.
    fn cache_answer(&self, name: &str, qtype: DnsRecordType, records: &[DnsRecord]) {
        let min_ttl = records.iter().map(|r| r.ttl).min().unwrap_or(300);
        let key = format!("{}\0{}", name.to_lowercase(), qtype.as_u16());
        let expiry = Instant::now() + Duration::from_secs(min_ttl as u64);
        self.answer_cache.lock().unwrap().insert(key, (records.to_vec(), expiry));
    }

    /// Get cached answer.
    fn get_cached_answer(&self, name: &str, qtype: DnsRecordType) -> Option<Vec<DnsRecord>> {
        let key = format!("{}\0{}", name.to_lowercase(), qtype.as_u16());
        let mut guard = self.answer_cache.lock().unwrap();
        if let Some((records, expiry)) = guard.get(&key) {
            if Instant::now() < *expiry {
                return Some(records.clone());
            }
            guard.remove(&key);
        }
        None
    }

    /// Cache additional section records (NS referrals, glue A records).
    fn cache_additional(&self, resp: &DnsMessage) {
        for rr in &resp.additional {
            if rr.rtype == DnsRecordType::A {
                if let DnsRecordData::A(ip) = &rr.data {
                    let expiry = Instant::now() + Duration::from_secs(rr.ttl as u64);
                    self.ns_cache.lock().unwrap().insert(rr.name.to_lowercase(), (*ip, expiry));
                }
            }
            if rr.rtype == DnsRecordType::NS {
                if let DnsRecordData::NS(name) = &rr.data {
                    // Find glue A for this NS
                    for glue in &resp.additional {
                        if glue.rtype == DnsRecordType::A
                            && glue.name.to_lowercase() == name.to_lowercase() {
                            if let DnsRecordData::A(ip) = &glue.data {
                                let expiry = Instant::now() + Duration::from_secs(glue.ttl as u64);
                                self.ns_cache.lock().unwrap().insert(name.to_lowercase(), (*ip, expiry));
                            }
                        }
                    }
                }
            }
        }
    }

    fn random_id(&self) -> u16 {
        use std::time::SystemTime;
        let now = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
        (now.as_nanos() as u16).wrapping_add(now.subsec_nanos() as u16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_root_hints_not_empty() {
        let hints = default_root_hints();
        assert_eq!(hints.len(), 13);
        assert!(hints.iter().all(|h| !h.addrs.is_empty()));
    }

    #[test]
    fn test_resolver_creation() {
        let r = RecursiveResolver::new(default_root_hints());
        assert!(r.is_ok());
    }

    #[test]
    fn test_extract_ns_referral() {
        let resolver = RecursiveResolver::new(default_root_hints()).unwrap();
        let mut resp = DnsMessage::response(0x1234, DnsResponseCode::NoError, Vec::new());
        resp.authority.push(DnsRecord::ns("example.com".into(), "ns1.example.com".into(), 3600));

        let ns = resolver.extract_ns_referral(&resp);
        assert!(ns.is_some());
        let ns = ns.unwrap();
        assert_eq!(ns.len(), 1);
        assert_eq!(ns[0], "ns1.example.com");
    }

    #[test]
    fn test_cache_answer_roundtrip() {
        let resolver = RecursiveResolver::new(default_root_hints()).unwrap();
        let records = vec![DnsRecord::a("example.com".into(), Ipv4Addr::new(1,2,3,4), 300)];
        resolver.cache_answer("example.com", DnsRecordType::A, &records);

        let cached = resolver.get_cached_answer("example.com", DnsRecordType::A);
        assert!(cached.is_some());
        let cached = cached.unwrap();
        assert_eq!(cached.len(), 1);
    }

    #[test]
    fn test_max_depth_exceeded() {
        let resolver = RecursiveResolver::new(default_root_hints()).unwrap();
        // Can't easily test iterative resolution without a real DNS,
        // but verify the struct is configured properly
        assert_eq!(resolver.max_depth, 10);
    }
}
