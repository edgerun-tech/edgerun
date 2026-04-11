//! DNS zone — in-memory zone file with record management.

use std::collections::HashMap;
use std::net::Ipv4Addr;

use super::record::{DnsRecordType, DnsRecordData};
use super::message::DnsRecord;

/// A single DNS zone (authoritative for one domain).
///
/// # Example
/// ```
/// use edgerun_dns::zone::DnsZone;
/// use std::net::Ipv4Addr;
///
/// let mut zone = DnsZone::new("example.com");
/// zone.add_soa("ns1.example.com", "admin.example.com");
/// zone.add_ns("ns1.example.com");
/// zone.add_a("@", Ipv4Addr::new(192, 168, 1, 1), 3600);
/// zone.add_a("www", Ipv4Addr::new(192, 168, 1, 2), 3600);
/// zone.add_cname("blog", "www.example.com", 3600);
/// ```
#[derive(Debug)]
pub struct DnsZone {
    /// Origin (e.g. "example.com").
    pub origin: String,
    /// Records keyed by name (lowercase).
    /// Each name maps to a list of records (multiple records per name).
    records: HashMap<String, Vec<DnsRecord>>,
    /// Default TTL.
    default_ttl: u32,
}

impl DnsZone {
    /// Create a new zone for the given origin domain.
    pub fn new(origin: &str) -> Self {
        Self {
            origin: origin.trim_end_matches('.').to_lowercase(),
            records: HashMap::new(),
            default_ttl: 3600,
        }
    }

    /// Set the default TTL for new records.
    pub fn set_default_ttl(&mut self, ttl: u32) {
        self.default_ttl = ttl;
    }

    // --- Record addition helpers ---

    /// Add an A record.
    pub fn add_a(&mut self, name: &str, ip: Ipv4Addr, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::a(full_name, ip, ttl);
        self.add_record(rr);
    }

    /// Add an AAAA record.
    pub fn add_aaaa(&mut self, name: &str, ip: std::net::Ipv6Addr, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::aaaa(full_name, ip, ttl);
        self.add_record(rr);
    }

    /// Add a CNAME record.
    pub fn add_cname(&mut self, name: &str, target: &str, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::cname(full_name, target.to_string(), ttl);
        self.add_record(rr);
    }

    /// Add an NS record.
    pub fn add_ns(&mut self, nameserver: &str) {
        let rr = DnsRecord::ns(self.origin.clone(), nameserver.to_string(), self.default_ttl);
        self.add_record(rr);
    }

    /// Add an MX record.
    pub fn add_mx(&mut self, name: &str, priority: u16, exchange: &str, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::mx(full_name, priority, exchange.to_string(), ttl);
        self.add_record(rr);
    }

    /// Add a TXT record.
    pub fn add_txt(&mut self, name: &str, text: &str, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::txt(full_name, text.to_string(), ttl);
        self.add_record(rr);
    }

    /// Add a PTR record.
    pub fn add_ptr(&mut self, name: &str, ptr_name: &str, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::ptr(full_name, ptr_name.to_string(), ttl);
        self.add_record(rr);
    }

    /// Add an SOA record for the zone.
    pub fn add_soa(&mut self, mname: &str, rname: &str) {
        let rr = DnsRecord {
            name: self.origin.clone(),
            rtype: DnsRecordType::SOA,
            rclass: 1,
            ttl: self.default_ttl,
            data: DnsRecordData::SOA {
                mname: mname.to_string(),
                rname: rname.to_string(),
                serial: 2024010101,
                refresh: 3600,
                retry: 900,
                expire: 604800,
                minimum: 86400,
            },
        };
        self.add_record(rr);
    }

    /// Add an SRV record.
    pub fn add_srv(&mut self, name: &str, priority: u16, weight: u16, port: u16, target: &str, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord {
            name: full_name,
            rtype: DnsRecordType::SRV,
            rclass: 1,
            ttl,
            data: DnsRecordData::SRV {
                priority,
                weight,
                port,
                target: target.to_string(),
            },
        };
        self.add_record(rr);
    }

    /// Add a NAPTR record (RFC 3403 — URI/telephone routing).
    pub fn add_naptr(&mut self, name: &str, order: u16, preference: u16,
                     flags: &str, services: &str, regexp: &str, replacement: &str, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::naptr(full_name, order, preference, flags.to_string(),
                                  services.to_string(), regexp.to_string(), replacement.to_string(), ttl);
        self.add_record(rr);
    }

    /// Add a CAA record (RFC 8659 — Certificate Authority Authorization).
    pub fn add_caa(&mut self, name: &str, critical: bool, tag: &str, value: &str, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::caa(full_name, critical, tag.to_string(), value.to_string(), ttl);
        self.add_record(rr);
    }

    /// Add a TLSA record (RFC 6698 — DANE certificate binding).
    pub fn add_tlsa(&mut self, name: &str, usage: u8, selector: u8, matching_type: u8,
                    certificate: Vec<u8>, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::tlsa(full_name, usage, selector, matching_type, certificate, ttl);
        self.add_record(rr);
    }

    /// Add an HTTPS/SVCB record (RFC 9460 — Service Binding).
    pub fn add_https(&mut self, name: &str, priority: u16, target: &str, params: Vec<u8>, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::svcb(full_name, priority, target.to_string(), params, ttl);
        self.add_record(rr);
    }

    /// Add a raw record directly.
    pub fn add_record(&mut self, rr: DnsRecord) {
        let name = rr.name.to_lowercase();
        self.records.entry(name).or_default().push(rr);
    }

    /// Remove all records for a name.
    pub fn remove_name(&mut self, name: &str) {
        let full_name = self.full_name(name).to_lowercase();
        self.records.remove(&full_name);
    }

    /// Remove a specific record by name and type.
    pub fn remove_record(&mut self, name: &str, rtype: DnsRecordType) {
        let full_name = self.full_name(name).to_lowercase();
        if let Some(records) = self.records.get_mut(&full_name) {
            records.retain(|r| r.rtype != rtype);
            if records.is_empty() {
                self.records.remove(&full_name);
            }
        }
    }

    // --- Resolution ---

    /// Resolve a name+type query against this zone.
    /// Returns matching records or None if the name doesn't exist in this zone.
    pub fn resolve(&self, name: &str, qtype: DnsRecordType) -> Option<Vec<DnsRecord>> {
        let lookup = self.full_name(name).to_lowercase();

        // Check if name exists in this zone
        if let Some(records) = self.records.get(&lookup) {
            if qtype == DnsRecordType::ANY {
                Some(records.clone())
            } else {
                let matching: Vec<_> = records.iter().filter(|r| r.rtype == qtype).cloned().collect();
                if !matching.is_empty() {
                    Some(matching)
                } else {
                    // Name exists but no records of this type
                    Some(Vec::new())
                }
            }
        } else {
            None
        }
    }

    /// Get all records for a name.
    pub fn get_records(&self, name: &str) -> Vec<&DnsRecord> {
        let full_name = self.full_name(name).to_lowercase();
        self.records.get(&full_name).map(|v| v.iter().collect()).unwrap_or_default()
    }

    /// Get all names in this zone.
    pub fn names(&self) -> Vec<&str> {
        self.records.keys().map(|s| s.as_str()).collect()
    }

    /// Record count for a name.
    pub fn record_count(&self, name: &str) -> usize {
        let full_name = self.full_name(name).to_lowercase();
        self.records.get(&full_name).map(|v| v.len()).unwrap_or(0)
    }

    /// Total record count in the zone.
    pub fn total_records(&self) -> usize {
        self.records.values().map(|v| v.len()).sum()
    }

    // --- Internal ---

    /// Convert a relative name to a fully qualified name for this zone.
    fn full_name(&self, name: &str) -> String {
        let name = name.trim_end_matches('.');
        let lower = name.to_lowercase();
        let origin_lower = self.origin.to_lowercase();

        if lower == "@" || lower.is_empty() {
            self.origin.clone()
        } else if lower == origin_lower || lower.ends_with(&format!(".{}", origin_lower)) {
            // Already fully qualified — just return lowercase
            name.to_lowercase()
        } else {
            format!("{}.{}", lower, origin_lower)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_name() {
        let zone = DnsZone::new("example.com");
        assert_eq!(zone.full_name("www"), "www.example.com");
        assert_eq!(zone.full_name("@"), "example.com");
        assert_eq!(zone.full_name(""), "example.com");
        assert_eq!(zone.full_name("www.example.com"), "www.example.com");
        assert_eq!(zone.full_name("sub.www"), "sub.www.example.com");
    }

    #[test]
    fn test_add_and_resolve_a() {
        let mut zone = DnsZone::new("example.com");
        zone.add_a("@", Ipv4Addr::new(192, 168, 1, 1), 3600);
        zone.add_a("www", Ipv4Addr::new(192, 168, 1, 2), 3600);

        let records = zone.resolve("@", DnsRecordType::A).unwrap();
        assert_eq!(records.len(), 1);
        if let DnsRecordData::A(ip) = &records[0].data {
            assert_eq!(*ip, Ipv4Addr::new(192, 168, 1, 1));
        }

        let records = zone.resolve("www.example.com", DnsRecordType::A).unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn test_resolve_nonexistent() {
        let zone = DnsZone::new("example.com");
        assert!(zone.resolve("nonexistent", DnsRecordType::A).is_none());
    }

    #[test]
    fn test_multiple_a_records() {
        let mut zone = DnsZone::new("example.com");
        zone.add_a("mail", Ipv4Addr::new(10, 0, 0, 1), 3600);
        zone.add_a("mail", Ipv4Addr::new(10, 0, 0, 2), 3600);

        let records = zone.resolve("mail.example.com", DnsRecordType::A).unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_any_query() {
        let mut zone = DnsZone::new("example.com");
        zone.add_a("@", Ipv4Addr::new(1, 1, 1, 1), 3600);
        zone.add_txt("@", "v=spf1", 3600);

        let records = zone.resolve("@", DnsRecordType::ANY).unwrap();
        assert_eq!(records.len(), 2);
    }

    #[test]
    fn test_cname() {
        let mut zone = DnsZone::new("example.com");
        zone.add_a("www", Ipv4Addr::new(192, 168, 1, 1), 3600);
        zone.add_cname("blog", "www.example.com", 3600);

        let cname_records = zone.resolve("blog.example.com", DnsRecordType::CNAME).unwrap();
        assert_eq!(cname_records.len(), 1);
        if let DnsRecordData::CNAME(target) = &cname_records[0].data {
            assert_eq!(target, "www.example.com");
        }
    }

    #[test]
    fn test_soa() {
        let mut zone = DnsZone::new("example.com");
        zone.add_soa("ns1.example.com", "admin.example.com");

        let records = zone.resolve("example.com", DnsRecordType::SOA).unwrap();
        assert_eq!(records.len(), 1);
        if let DnsRecordData::SOA { mname, rname, .. } = &records[0].data {
            assert_eq!(mname, "ns1.example.com");
            assert_eq!(rname, "admin.example.com");
        }
    }

    #[test]
    fn test_mx() {
        let mut zone = DnsZone::new("example.com");
        zone.add_mx("@", 10, "mail.example.com", 3600);

        let records = zone.resolve("example.com", DnsRecordType::MX).unwrap();
        assert_eq!(records.len(), 1);
        if let DnsRecordData::MX { priority, exchange } = &records[0].data {
            assert_eq!(*priority, 10);
            assert_eq!(exchange, "mail.example.com");
        }
    }

    #[test]
    fn test_remove_name() {
        let mut zone = DnsZone::new("example.com");
        zone.add_a("temp", Ipv4Addr::new(10, 0, 0, 1), 3600);
        assert!(zone.resolve("temp.example.com", DnsRecordType::A).is_some());

        zone.remove_name("temp");
        assert!(zone.resolve("temp.example.com", DnsRecordType::A).is_none());
    }

    #[test]
    fn test_total_records() {
        let mut zone = DnsZone::new("example.com");
        zone.add_soa("ns1.example.com", "admin.example.com");
        zone.add_ns("ns1.example.com");
        zone.add_a("@", Ipv4Addr::new(1, 1, 1, 1), 3600);
        zone.add_a("www", Ipv4Addr::new(1, 1, 1, 2), 3600);

        assert_eq!(zone.total_records(), 4);
    }

    #[test]
    fn test_case_insensitive() {
        let mut zone = DnsZone::new("example.com");
        zone.add_a("WWW", Ipv4Addr::new(192, 168, 1, 1), 3600);

        // Should resolve regardless of case
        let records = zone.resolve("www.example.com", DnsRecordType::A).unwrap();
        assert_eq!(records.len(), 1);

        let records = zone.resolve("WWW.EXAMPLE.COM", DnsRecordType::A).unwrap();
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn test_remove_record_by_type() {
        let mut zone = DnsZone::new("example.com");
        zone.add_a("mail", Ipv4Addr::new(10, 0, 0, 1), 3600);
        zone.add_txt("mail", "some text", 3600);

        assert_eq!(zone.record_count("mail"), 2);
        zone.remove_record("mail", DnsRecordType::TXT);
        assert_eq!(zone.record_count("mail"), 1);
    }
}
