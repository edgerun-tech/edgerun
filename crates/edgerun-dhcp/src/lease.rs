//! DHCP lease tracking.

use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use edgerun_encoding::ip::{ip_to_u32, u32_to_ip};

/// A DHCP lease record.
#[derive(Debug, Clone)]
pub struct Lease {
    /// Client MAC address.
    pub mac: [u8; 6],
    /// Assigned IP address.
    pub ip: Ipv4Addr,
    /// Lease duration in seconds.
    pub lease_time: u32,
    /// Time when this lease was granted.
    pub granted_at: Instant,
    /// Transaction ID of the exchange.
    pub xid: u32,
}

impl Lease {
    /// Create a new lease.
    pub fn new(mac: [u8; 6], ip: Ipv4Addr, lease_time: u32, xid: u32) -> Self {
        Self {
            mac,
            ip,
            lease_time,
            granted_at: Instant::now(),
            xid,
        }
    }

    /// Returns true if this lease has expired.
    pub fn is_expired(&self) -> bool {
        self.granted_at.elapsed() > Duration::from_secs(self.lease_time as u64)
    }

    /// Returns true if this lease is past the renewal time (T1).
    pub fn is_renewal_due(&self) -> bool {
        self.granted_at.elapsed() > Duration::from_secs((self.lease_time / 2) as u64)
    }

    /// Returns true if this lease is past the rebinding time (T2).
    pub fn is_rebinding_due(&self) -> bool {
        self.granted_at.elapsed() > Duration::from_secs((self.lease_time * 7 / 8) as u64)
    }

    /// Seconds remaining until lease expiry.
    pub fn remaining_secs(&self) -> u64 {
        let elapsed = self.granted_at.elapsed().as_secs();
        (self.lease_time as u64).saturating_sub(elapsed)
    }
}

/// A pool of available IP addresses and their leases.
#[derive(Debug)]
pub struct LeasePool {
    /// Start of the pool (inclusive).
    pub pool_start: Ipv4Addr,
    /// End of the pool (inclusive).
    pub pool_end: Ipv4Addr,
    /// Active leases, keyed by IP (as u32).
    pub leases: std::collections::HashMap<u32, Lease>,
    /// MAC → IP mapping for fast lookup.
    pub mac_to_ip: std::collections::HashMap<[u8; 6], u32>,
    /// Reserved IPs (not to be handed out).
    pub reserved: std::collections::HashSet<u32>,
}

impl LeasePool {
    /// Create a new lease pool.
    pub fn new(pool_start: Ipv4Addr, pool_end: Ipv4Addr) -> Self {
        Self {
            pool_start,
            pool_end,
            leases: std::collections::HashMap::new(),
            mac_to_ip: std::collections::HashMap::new(),
            reserved: std::collections::HashSet::new(),
        }
    }

    /// Reserve an IP address (won't be handed out by DHCP).
    pub fn reserve(&mut self, ip: Ipv4Addr) {
        self.reserved.insert(ip_to_u32(&ip));
    }

    /// Find an existing lease for this MAC address.
    pub fn find_lease_by_mac(&self, mac: [u8; 6]) -> Option<&Lease> {
        self.mac_to_ip
            .get(&mac)
            .and_then(|ip_u32| self.leases.get(ip_u32))
    }

    /// Find an existing lease for this IP address.
    pub fn find_lease_by_ip(&self, ip: Ipv4Addr) -> Option<&Lease> {
        self.leases.get(&ip_to_u32(&ip))
    }

    /// Allocate the next available IP address for a client.
    /// Returns None if the pool is exhausted.
    pub fn allocate(&mut self, mac: [u8; 6], lease_time: u32, xid: u32) -> Option<Ipv4Addr> {
        // If this MAC already has a lease, return it
        if let Some(ip_u32) = self.mac_to_ip.get(&mac) {
            if let Some(lease) = self.leases.get(ip_u32) {
                if !lease.is_expired() {
                    return Some(lease.ip);
                }
            }
        }

        // Sweep expired leases
        self.sweep_expired();

        // Find a free IP
        let start = ip_to_u32(&self.pool_start);
        let end = ip_to_u32(&self.pool_end);

        for ip_u32 in start..=end {
            if self.reserved.contains(&ip_u32) {
                continue;
            }
            if let std::collections::hash_map::Entry::Vacant(e) = self.leases.entry(ip_u32) {
                let ip = u32_to_ip(ip_u32);
                let lease = Lease::new(mac, ip, lease_time, xid);
                e.insert(lease);
                self.mac_to_ip.insert(mac, ip_u32);
                return Some(ip);
            }
        }

        None
    }

    /// Remove a lease (client released or declined).
    pub fn release(&mut self, mac: [u8; 6]) {
        if let Some(ip_u32) = self.mac_to_ip.remove(&mac) {
            self.leases.remove(&ip_u32);
        }
    }

    /// Remove all expired leases.
    pub fn sweep_expired(&mut self) {
        let expired: Vec<u32> = self
            .leases
            .iter()
            .filter(|(_, lease)| lease.is_expired())
            .map(|(ip, _)| *ip)
            .collect();

        for ip_u32 in expired {
            if let Some(lease) = self.leases.remove(&ip_u32) {
                self.mac_to_ip.remove(&lease.mac);
            }
        }
    }

    /// Number of active (non-expired) leases.
    pub fn active_count(&self) -> usize {
        self.leases.iter().filter(|(_, l)| !l.is_expired()).count()
    }

    /// Total addresses in the pool (including reserved and leased).
    pub fn pool_size(&self) -> u32 {
        ip_to_u32(&self.pool_end) - ip_to_u32(&self.pool_start) + 1
    }

    /// Number of available addresses.
    pub fn available_count(&self) -> u32 {
        let used = self.leases.len() as u32 + self.reserved.len() as u32;
        self.pool_size().saturating_sub(used)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_lease_expiry() {
        let mac = [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff];
        let lease = Lease::new(mac, Ipv4Addr::new(192, 168, 1, 100), 3600, 0x1234);
        assert!(!lease.is_expired());
        assert!(!lease.is_renewal_due());

        // Verify the logic path
        assert!(lease.remaining_secs() <= 3600);
    }

    #[test]
    fn test_pool_allocate_and_release() {
        let mut pool = LeasePool::new(
            Ipv4Addr::new(192, 168, 1, 100),
            Ipv4Addr::new(192, 168, 1, 110),
        );

        let mac1 = [1, 2, 3, 4, 5, 6];
        let mac2 = [6, 5, 4, 3, 2, 1];

        let ip1 = pool.allocate(mac1, 3600, 0x1111).unwrap();
        let ip2 = pool.allocate(mac2, 3600, 0x2222).unwrap();

        assert_ne!(ip1, ip2);
        assert_eq!(pool.active_count(), 2);
        assert_eq!(pool.available_count(), 9); // 11 total - 2 leased

        // Same MAC gets same IP
        let ip1_again = pool.allocate(mac1, 3600, 0x3333).unwrap();
        assert_eq!(ip1, ip1_again);

        // Release
        pool.release(mac1);
        assert_eq!(pool.active_count(), 1);
    }

    #[test]
    fn test_pool_reserve() {
        let mut pool = LeasePool::new(
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv4Addr::new(10, 0, 0, 10),
        );

        pool.reserve(Ipv4Addr::new(10, 0, 0, 1)); // Gateway

        let mac = [0xaa; 6];
        let ip = pool.allocate(mac, 3600, 0x1234).unwrap();
        assert_ne!(ip, Ipv4Addr::new(10, 0, 0, 1));
    }

    #[test]
    fn test_pool_exhaustion() {
        let mut pool = LeasePool::new(
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv4Addr::new(10, 0, 0, 2),
        );

        let mac1 = [1, 1, 1, 1, 1, 1];
        let mac2 = [2, 2, 2, 2, 2, 2];
        let mac3 = [3, 3, 3, 3, 3, 3];

        assert!(pool.allocate(mac1, 3600, 0x1).is_some());
        assert!(pool.allocate(mac2, 3600, 0x2).is_some());
        assert!(pool.allocate(mac3, 3600, 0x3).is_none());
    }

    #[test]
    fn test_sweep_expired() {
        let mut pool = LeasePool::new(
            Ipv4Addr::new(10, 0, 0, 1),
            Ipv4Addr::new(10, 0, 0, 10),
        );

        let mac = [0xff; 6];
        pool.allocate(mac, 3600, 0x1234);
        assert_eq!(pool.active_count(), 1);

        pool.sweep_expired();
        assert_eq!(pool.active_count(), 1); // Not actually expired yet
    }

    #[test]
    fn test_ip_u32_conversion() {
        let ip = Ipv4Addr::new(192, 168, 1, 100);
        let n = ip_to_u32(&ip);
        assert_eq!(u32_to_ip(n), ip);

        assert_eq!(ip_to_u32(&Ipv4Addr::new(0, 0, 0, 0)), 0);
        assert_eq!(ip_to_u32(&Ipv4Addr::new(255, 255, 255, 255)), 0xFFFFFFFF);
    }

    #[test]
    fn test_find_lease_by_mac_and_ip() {
        let mut pool = LeasePool::new(
            Ipv4Addr::new(192, 168, 1, 100),
            Ipv4Addr::new(192, 168, 1, 110),
        );

        let mac = [0x11, 0x22, 0x33, 0x44, 0x55, 0x66];
        let ip = pool.allocate(mac, 3600, 0x1234).unwrap();

        let lease_by_mac = pool.find_lease_by_mac(mac).unwrap();
        assert_eq!(lease_by_mac.ip, ip);

        let lease_by_ip = pool.find_lease_by_ip(ip).unwrap();
        assert_eq!(lease_by_ip.mac, mac);
    }
}
