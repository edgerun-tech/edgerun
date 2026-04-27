//! DNS ↔ DHCP integration layer.
//!
//! When DHCP allocates an IP, automatically:
//! - Create forward (A) record: hostname → IP
//! - Create reverse (PTR) record: IP → hostname
//!
//! When DHCP releases/declines:
//! - Remove both records from the DNS zone

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::net::Ipv4Addr;

pub type HashMap<K, V> = BTreeMap<K, V>;

#[cfg(target_os = "none")]
type Mutex<T> = edgerun_rt::Mutex<T>;
#[cfg(not(target_os = "none"))]
type Mutex<T> = std::sync::Mutex<T>;

/// Tracks DNS records created by DHCP allocations.
pub struct DnsDhcpIntegration {
    /// Active DHCP-created A records: hostname → IP
    a_records: Arc<Mutex<HashMap<String, Ipv4Addr>>>,
    /// Active DHCP-created PTR records: IP → hostname
    ptr_records: Arc<Mutex<HashMap<Ipv4Addr, String>>>,
}

impl DnsDhcpIntegration {
    pub fn new() -> Self {
        Self {
            a_records: Arc::new(Mutex::new(HashMap::new())),
            ptr_records: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Called when DHCP allocates an IP to a client.
    /// Creates A and PTR records in DNS.
    pub fn allocate(&self, hostname: &str, ip: Ipv4Addr) {
        let mut a = lock(&self.a_records);
        let mut ptr = lock(&self.ptr_records);

        // Remove old PTR if hostname was previously assigned different IP
        if let Some(old_ip) = a.get(hostname) {
            ptr.remove(old_ip);
        }

        // Remove old A record if IP was previously assigned to different hostname
        if let Some(old_hostname) = ptr.get(&ip) {
            a.remove(old_hostname);
        }

        a.insert(hostname.to_string(), ip);
        ptr.insert(ip, hostname.to_string());

        edgerun_log::info!("edgerun-net: DNS A record created: {} → {}", hostname, ip);
        edgerun_log::info!("edgerun-net: DNS PTR record created: {} → {}", ip, hostname);
    }

    /// Called when DHCP releases or declines an IP.
    /// Removes A and PTR records from DNS.
    pub fn release(&self, hostname: &str, ip: Ipv4Addr) {
        let mut a = lock(&self.a_records);
        let mut ptr = lock(&self.ptr_records);

        a.remove(hostname);
        ptr.remove(&ip);

        edgerun_log::info!("edgerun-net: DNS records removed: {} ↔ {}", hostname, ip);
    }

    /// Get all DHCP-created A records for DNS zone population.
    pub fn get_a_records(&self) -> HashMap<String, Ipv4Addr> {
        lock(&self.a_records).clone()
    }

    /// Get all DHCP-created PTR records for reverse zone population.
    pub fn get_ptr_records(&self) -> HashMap<Ipv4Addr, String> {
        lock(&self.ptr_records).clone()
    }

    /// Count active DHCP-DNS records.
    pub fn active_count(&self) -> usize {
        lock(&self.a_records).len()
    }
}

#[cfg(not(target_os = "none"))]
fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap()
}

#[cfg(target_os = "none")]
fn lock<T>(mutex: &Mutex<T>) -> edgerun_rt::MutexGuard<'_, T> {
    mutex.lock()
}

impl Default for DnsDhcpIntegration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocate_creates_both_records() {
        let integration = DnsDhcpIntegration::new();
        integration.allocate("laptop.local", Ipv4Addr::new(192, 168, 1, 100));

        let a = integration.get_a_records();
        let ptr = integration.get_ptr_records();

        assert_eq!(
            a.get("laptop.local"),
            Some(&Ipv4Addr::new(192, 168, 1, 100))
        );
        assert_eq!(
            ptr.get(&Ipv4Addr::new(192, 168, 1, 100)),
            Some(&"laptop.local".to_string())
        );
    }

    #[test]
    fn test_release_removes_both_records() {
        let integration = DnsDhcpIntegration::new();
        integration.allocate("phone.local", Ipv4Addr::new(192, 168, 1, 101));
        integration.release("phone.local", Ipv4Addr::new(192, 168, 1, 101));

        assert_eq!(integration.active_count(), 0);
        assert!(integration.get_ptr_records().is_empty());
    }

    #[test]
    fn test_reassign_updates_records() {
        let integration = DnsDhcpIntegration::new();
        integration.allocate("pc.local", Ipv4Addr::new(10, 0, 0, 1));
        integration.allocate("pc.local", Ipv4Addr::new(10, 0, 0, 2));

        let a = integration.get_a_records();
        let ptr = integration.get_ptr_records();

        assert_eq!(a.get("pc.local"), Some(&Ipv4Addr::new(10, 0, 0, 2)));
        assert_eq!(
            ptr.get(&Ipv4Addr::new(10, 0, 0, 2)),
            Some(&"pc.local".to_string())
        );
        assert!(!ptr.contains_key(&Ipv4Addr::new(10, 0, 0, 1)));
    }

    #[test]
    fn test_different_host_same_ip_updates() {
        let integration = DnsDhcpIntegration::new();
        integration.allocate("host-a.local", Ipv4Addr::new(10, 0, 0, 50));
        integration.allocate("host-b.local", Ipv4Addr::new(10, 0, 0, 50));

        let a = integration.get_a_records();
        assert_eq!(a.len(), 1);
        assert_eq!(a.get("host-b.local"), Some(&Ipv4Addr::new(10, 0, 0, 50)));
        assert!(!a.contains_key("host-a.local"));
    }
}
