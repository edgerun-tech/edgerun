//! DNS zone — in-memory zone file with record management.

use crate::std::net::Ipv4Addr;
use alloc::collections::BTreeMap as HashMap;
use alloc::{
    boxed::Box,
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};

use super::message::DnsRecord;
use super::record::{DnsRecordData, DnsRecordType};

/// A single DNS zone (authoritative for one domain).
///
/// # Example
/// ```
/// use edgerun_dns::zone::DnsZone;
/// use crate::std::net::Ipv4Addr;
///
/// let mut zone = DnsZone::new("example.com");
/// zone.add_soa("ns1.example.com", "admin.example.com");
/// zone.add_ns("ns1.example.com");
/// zone.add_a("@", Ipv4Addr::new(192, 168, 1, 1), 3600);
/// zone.add_a("www", Ipv4Addr::new(192, 168, 1, 2), 3600);
/// zone.add_cname("blog", "www.example.com", 3600);
/// ```
#[derive(Debug, Clone)]
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
    pub fn add_aaaa(&mut self, name: &str, ip: crate::std::net::Ipv6Addr, ttl: u32) {
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
        let rr = DnsRecord::ns(
            self.origin.clone(),
            nameserver.to_string(),
            self.default_ttl,
        );
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
    pub fn add_srv(
        &mut self,
        name: &str,
        priority: u16,
        weight: u16,
        port: u16,
        target: &str,
        ttl: u32,
    ) {
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
    pub fn add_naptr(
        &mut self,
        name: &str,
        order: u16,
        preference: u16,
        flags: &str,
        services: &str,
        regexp: &str,
        replacement: &str,
        ttl: u32,
    ) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::naptr(
            full_name,
            order,
            preference,
            flags.to_string(),
            services.to_string(),
            regexp.to_string(),
            replacement.to_string(),
            ttl,
        );
        self.add_record(rr);
    }

    /// Add a CAA record (RFC 8659 — Certificate Authority Authorization).
    pub fn add_caa(&mut self, name: &str, critical: bool, tag: &str, value: &str, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::caa(full_name, critical, tag.to_string(), value.to_string(), ttl);
        self.add_record(rr);
    }

    /// Add a TLSA record (RFC 6698 — DANE certificate binding).
    pub fn add_tlsa(
        &mut self,
        name: &str,
        usage: u8,
        selector: u8,
        matching_type: u8,
        certificate: Vec<u8>,
        ttl: u32,
    ) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::tlsa(full_name, usage, selector, matching_type, certificate, ttl);
        self.add_record(rr);
    }

    /// Add an HTTPS/SVCB record (RFC 9460 — Service Binding).
    pub fn add_https(
        &mut self,
        name: &str,
        priority: u16,
        target: &str,
        params: Vec<u8>,
        ttl: u32,
    ) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::svcb(full_name, priority, target.to_string(), params, ttl);
        self.add_record(rr);
    }

    /// Add a DS record (RFC 4034 — Delegation Signer).
    pub fn add_ds(
        &mut self,
        name: &str,
        key_tag: u16,
        algorithm: u8,
        digest_type: u8,
        digest: Vec<u8>,
        ttl: u32,
    ) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::ds(full_name, key_tag, algorithm, digest_type, digest, ttl);
        self.add_record(rr);
    }

    /// Add a DNSKEY record (RFC 4034 — DNS Public Key).
    pub fn add_dnskey(
        &mut self,
        name: &str,
        flags: u16,
        protocol: u8,
        algorithm: u8,
        public_key: Vec<u8>,
        ttl: u32,
    ) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::dnskey(full_name, flags, protocol, algorithm, public_key, ttl);
        self.add_record(rr);
    }

    /// Add an RRSIG record (RFC 4034 — RRset Signature).
    pub fn add_rrsig(
        &mut self,
        name: &str,
        type_covered: u16,
        algorithm: u8,
        labels: u8,
        original_ttl: u32,
        expiration: u32,
        inception: u32,
        key_tag: u16,
        signer_name: &str,
        signature: Vec<u8>,
        ttl: u32,
    ) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::rrsig(
            full_name,
            type_covered,
            algorithm,
            labels,
            original_ttl,
            expiration,
            inception,
            key_tag,
            signer_name.to_string(),
            signature,
            ttl,
        );
        self.add_record(rr);
    }

    /// Add an NSEC record (RFC 4034 — Next Secure).
    pub fn add_nsec(&mut self, name: &str, next_owner: &str, type_bits: Vec<u8>, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::nsec(full_name, next_owner.to_string(), type_bits, ttl);
        self.add_record(rr);
    }

    /// Add an NSEC3 record (RFC 5155 — Next SECure v3).
    pub fn add_nsec3(
        &mut self,
        name: &str,
        hash_algorithm: u8,
        flags: u8,
        iterations: u16,
        salt: Vec<u8>,
        next_hashed_owner: Vec<u8>,
        type_bits: Vec<u8>,
        ttl: u32,
    ) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::nsec3(
            full_name,
            hash_algorithm,
            flags,
            iterations,
            salt,
            next_hashed_owner,
            type_bits,
            ttl,
        );
        self.add_record(rr);
    }

    /// Add an HINFO record (RFC 1035 — Host Info).
    pub fn add_hinfo(&mut self, name: &str, cpu: &str, os: &str, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::hinfo(full_name, cpu.to_string(), os.to_string(), ttl);
        self.add_record(rr);
    }

    /// Add an RP record (RFC 1183 — Responsible Person).
    pub fn add_rp(&mut self, name: &str, mbox: &str, txt: &str, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::rp(full_name, mbox.to_string(), txt.to_string(), ttl);
        self.add_record(rr);
    }

    /// Add a LOC record (RFC 1876 — Location).
    pub fn add_loc(
        &mut self,
        name: &str,
        size: u32,
        latitude: u32,
        longitude: u32,
        altitude: u32,
        ttl: u32,
    ) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::loc(
            full_name, 0, size, 18582825, 18582825, latitude, longitude, altitude, ttl,
        );
        self.add_record(rr);
    }

    /// Add an AFSDB record (RFC 1183 — AFS Database).
    pub fn add_afsdb(&mut self, name: &str, subtype: u16, hostname: &str, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::afsdb(full_name, subtype, hostname.to_string(), ttl);
        self.add_record(rr);
    }

    /// Add a URI record (RFC 7553 — Uniform Resource Identifier).
    pub fn add_uri(&mut self, name: &str, priority: u16, weight: u16, target: &str, ttl: u32) {
        let full_name = self.full_name(name);
        let rr = DnsRecord::uri(full_name, priority, weight, target.to_string(), ttl);
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
    ///
    /// Supports RFC 1034 §4.3.3 wildcards: `*.example.com` matches any
    /// single-label name like `foo.example.com` when the queried name
    /// does not exist exactly in the zone.
    pub fn resolve(&self, name: &str, qtype: DnsRecordType) -> Option<Vec<DnsRecord>> {
        let lookup = self.full_name(name).to_lowercase();

        // Check if name exists in this zone
        if let Some(records) = self.records.get(&lookup) {
            if qtype == DnsRecordType::ANY {
                Some(records.clone())
            } else {
                let matching: Vec<_> = records
                    .iter()
                    .filter(|r| r.rtype == qtype)
                    .cloned()
                    .collect();
                if !matching.is_empty() {
                    Some(matching)
                } else {
                    // Name exists but no records of this type
                    Some(Vec::new())
                }
            }
        } else {
            // Name does not exist — try wildcard matching (RFC 1034 §4.3.3).
            // For `foo.example.com`, try `*.example.com`.
            // For `bar.foo.example.com`, try `*.foo.example.com` then `*.example.com`.
            self.resolve_wildcard(&lookup, qtype)
        }
    }

    /// Try wildcard matching when an exact name lookup fails.
    /// Returns the most specific wildcard match, or None.
    fn resolve_wildcard(&self, qname: &str, qtype: DnsRecordType) -> Option<Vec<DnsRecord>> {
        let origin = self.origin.to_lowercase();

        let qname_without_origin = qname.strip_suffix(&format!(".{}", origin));
        let prefix = match qname_without_origin {
            Some(p) if !p.is_empty() => p,
            _ => return None,
        };

        let labels: Vec<&str> = prefix.split('.').collect();
        // Try wildcard at each sub-level.
        // When i=0 → zone-level wildcard: `*.example.com`
        // When i>0 → sub-level wildcard: `*.foo.example.com`
        for i in (0..=labels.len()).rev() {
            let wildcard = if i == 0 {
                format!("*.{}", origin)
            } else {
                format!("*.{}.{}", labels[i..].join("."), origin)
            };

            if let Some(records) = self.records.get(&wildcard) {
                if qtype == DnsRecordType::ANY {
                    return Some(records.clone());
                }
                let matching: Vec<_> = records
                    .iter()
                    .filter(|r| r.rtype == qtype)
                    .cloned()
                    .collect();
                return Some(matching);
            }
        }

        None
    }

    /// Get all records for a name.
    pub fn get_records(&self, name: &str) -> Vec<&DnsRecord> {
        let full_name = self.full_name(name).to_lowercase();
        self.records
            .get(&full_name)
            .map(|v| v.iter().collect())
            .unwrap_or_default()
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

        let cname_records = zone
            .resolve("blog.example.com", DnsRecordType::CNAME)
            .unwrap();
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

    #[test]
    fn test_wildcard_single_label() {
        let mut zone = DnsZone::new("example.com");
        zone.add_a("www", Ipv4Addr::new(192, 168, 1, 1), 3600);
        zone.add_a("*", Ipv4Addr::new(10, 0, 0, 1), 300);

        let records = zone.resolve("www.example.com", DnsRecordType::A).unwrap();
        assert_eq!(records.len(), 1); // exact match wins
        if let DnsRecordData::A(ip) = &records[0].data {
            assert_eq!(*ip, Ipv4Addr::new(192, 168, 1, 1));
        }

        let records = zone.resolve("foo.example.com", DnsRecordType::A).unwrap();
        assert_eq!(records.len(), 1); // wildcard match
        if let DnsRecordData::A(ip) = &records[0].data {
            assert_eq!(*ip, Ipv4Addr::new(10, 0, 0, 1));
        }
    }

    #[test]
    fn test_wildcard_no_match_origin() {
        let mut zone = DnsZone::new("example.com");
        zone.add_a("*", Ipv4Addr::new(10, 0, 0, 1), 300);
        assert!(zone.resolve("example.com", DnsRecordType::A).is_none());
    }

    #[test]
    fn test_wildcard_multi_level_specific() {
        let mut zone = DnsZone::new("example.com");
        zone.add_a("*", Ipv4Addr::new(10, 0, 0, 1), 300);
        zone.add_a("*.sub", Ipv4Addr::new(10, 0, 1, 1), 300);

        let records = zone
            .resolve("foo.sub.example.com", DnsRecordType::A)
            .unwrap();
        assert_eq!(records.len(), 1);
        if let DnsRecordData::A(ip) = &records[0].data {
            assert_eq!(*ip, Ipv4Addr::new(10, 0, 1, 1)); // more specific wildcard
        }
    }

    #[test]
    fn test_wildcard_any_type() {
        let mut zone = DnsZone::new("example.com");
        zone.add_a("*", Ipv4Addr::new(10, 0, 0, 1), 300);
        zone.add_txt("*", "wildcard text", 300);

        let records = zone.resolve("foo.example.com", DnsRecordType::ANY).unwrap();
        assert_eq!(records.len(), 2);
    }
}
