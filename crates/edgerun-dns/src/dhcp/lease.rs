//! DHCP lease tracking.

use std::net::Ipv4Addr;
use std::time::{Duration, Instant};

use edgerun_encoding::ip::{ip_to_u32, u32_to_ip};

/// DHCP lease states per RFC 2131 §3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseState {
    /// Client has received an OFFER but not yet sent REQUEST.
    Offered,
    /// Lease is bound — client has IP and is using it.
    Bound,
    /// Past T1 (50% of lease) — client unicasts REQUEST to current server.
    Renewing,
    /// Past T2 (87.5% of lease) — client broadcasts REQUEST to any server.
    Rebinding,
    /// Client has released the lease.
    Released,
}

/// A DHCP lease record.
#[derive(Debug, Clone)]
pub struct Lease {
    /// Client MAC address.
    pub mac: [u8; 6],
    /// Client identifier (option 61), if provided. RFC 2131 §9 says this
    /// takes precedence over chaddr for client identification.
    pub client_id: Option<Vec<u8>>,
    /// Assigned IP address.
    pub ip: Ipv4Addr,
    /// Lease duration in seconds.
    pub lease_time: u32,
    /// Time when this lease was granted.
    pub granted_at: Instant,
    /// Transaction ID of the exchange.
    pub xid: u32,
    /// Current lease state (RFC 2131 §3).
    pub state: LeaseState,
}

impl Lease {
    /// Create a new lease in BOUND state.
    pub fn new(mac: [u8; 6], ip: Ipv4Addr, lease_time: u32, xid: u32) -> Self {
        Self {
            mac,
            client_id: None,
            ip,
            lease_time,
            granted_at: Instant::now(),
            xid,
            state: LeaseState::Bound,
        }
    }

    /// Create a new offered lease (not yet bound).
    pub fn offered(mac: [u8; 6], ip: Ipv4Addr, lease_time: u32, xid: u32) -> Self {
        Self {
            mac,
            client_id: None,
            ip,
            lease_time,
            granted_at: Instant::now(),
            xid,
            state: LeaseState::Offered,
        }
    }

    /// Create a new lease with client-id.
    pub fn with_client_id(
        mac: [u8; 6],
        client_id: Vec<u8>,
        ip: Ipv4Addr,
        lease_time: u32,
        xid: u32,
    ) -> Self {
        Self {
            mac,
            client_id: Some(client_id),
            ip,
            lease_time,
            granted_at: Instant::now(),
            xid,
            state: LeaseState::Bound,
        }
    }

    /// Update lease state based on elapsed time.
    pub fn update_state(&self) -> LeaseState {
        if self.is_expired() {
            return LeaseState::Released;
        }
        if self.is_rebinding_due() {
            return LeaseState::Rebinding;
        }
        if self.is_renewal_due() {
            return LeaseState::Renewing;
        }
        LeaseState::Bound
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
    /// Client-ID → IP mapping (RFC 2131 §9: client-id takes precedence).
    pub client_id_to_ip: std::collections::HashMap<Vec<u8>, u32>,
    /// Reserved IPs (not to be handed out).
    pub reserved: std::collections::HashSet<u32>,
    /// IPs that had conflicts (reported via DECLINE). Blacklisted temporarily.
    pub conflicts: std::collections::HashMap<u32, Instant>,
}

impl LeasePool {
    /// Create a new lease pool.
    pub fn new(pool_start: Ipv4Addr, pool_end: Ipv4Addr) -> Self {
        Self {
            pool_start,
            pool_end,
            leases: std::collections::HashMap::new(),
            mac_to_ip: std::collections::HashMap::new(),
            client_id_to_ip: std::collections::HashMap::new(),
            reserved: std::collections::HashSet::new(),
            conflicts: std::collections::HashMap::new(),
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

    /// Allocate (offer) the next available IP address for a client.
    /// Creates a lease in OFFERED state (not yet bound).
    /// If client_id is provided, it takes precedence over MAC for identification (RFC 2131 §9).
    /// Returns None if the pool is exhausted.
    pub fn allocate(
        &mut self,
        mac: [u8; 6],
        client_id: Option<Vec<u8>>,
        lease_time: u32,
        xid: u32,
    ) -> Option<Ipv4Addr> {
        // Check by client-id first (RFC 2131 §9: client-id takes precedence)
        if let Some(ref cid) = client_id {
            if let Some(ip_u32) = self.client_id_to_ip.get(cid) {
                if let Some(lease) = self.leases.get(ip_u32) {
                    if !lease.is_expired() {
                        return Some(lease.ip);
                    }
                }
            }
        }
        // Fall back to MAC
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
        let now = Instant::now();

        for ip_u32 in start..=end {
            if self.reserved.contains(&ip_u32) {
                continue;
            }
            // Skip IPs with recent conflicts (RFC 2131 §2.2 — conflict detection)
            if let Some(conflict_time) = self.conflicts.get(&ip_u32) {
                // Blacklist for 10 minutes after conflict
                if now.duration_since(*conflict_time) < Duration::from_secs(600) {
                    continue;
                }
                // Expired conflict — clean up
                self.conflicts.remove(&ip_u32);
            }
            if let std::collections::hash_map::Entry::Vacant(e) = self.leases.entry(ip_u32) {
                let ip = u32_to_ip(ip_u32);
                let lease = Lease::offered(mac, ip, lease_time, xid);
                e.insert(lease);
                self.mac_to_ip.insert(mac, ip_u32);
                if let Some(ref cid) = client_id {
                    self.client_id_to_ip.insert(cid.clone(), ip_u32);
                }
                return Some(ip);
            }
        }

        None
    }

    /// Remove a lease (client released or declined).
    pub fn release(&mut self, mac: [u8; 6]) {
        if let Some(ip_u32) = self.mac_to_ip.remove(&mac) {
            if let Some(lease) = self.leases.remove(&ip_u32) {
                if let Some(ref cid) = lease.client_id {
                    self.client_id_to_ip.remove(cid);
                }
            }
        }
    }

    /// Acknowledge a lease — transition from OFFERED to BOUND state.
    /// Called when client sends REQUEST and we send ACK.
    pub fn acknowledge(&mut self, ip: Ipv4Addr) {
        if let Some(ip_u32) = self.leases.keys().find(|&&k| u32_to_ip(k) == ip).copied() {
            if let Some(lease) = self.leases.get_mut(&ip_u32) {
                lease.state = LeaseState::Bound;
                lease.granted_at = Instant::now();
            }
        }
    }

    /// Transition an offered lease to bound. Returns the lease if found.
    pub fn bind_offer(&mut self, mac: [u8; 6], ip: Ipv4Addr) -> Option<&Lease> {
        if let Some(ip_u32) = self.mac_to_ip.get(&mac).copied() {
            if let Some(lease) = self.leases.get_mut(&ip_u32) {
                if lease.ip == ip && lease.state == LeaseState::Offered {
                    lease.state = LeaseState::Bound;
                    lease.granted_at = Instant::now();
                    return Some(lease);
                }
            }
        }
        None
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
                if let Some(ref cid) = lease.client_id {
                    self.client_id_to_ip.remove(cid);
                }
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
        let used =
            self.leases.len() as u32 + self.reserved.len() as u32 + self.conflicts.len() as u32;
        self.pool_size().saturating_sub(used)
    }

    /// Record an IP conflict (RFC 2131 §2.2).
    /// Called when a DECLINE is received — the client detected the IP via ARP.
    pub fn record_conflict(&mut self, ip: Ipv4Addr, reporter_mac: [u8; 6]) {
        let ip_u32 = ip_to_u32(&ip);
        edgerun_log::warn!("edgerun-dhcp: IP conflict detected for {} (reported by {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x})",
            ip,
            reporter_mac[0], reporter_mac[1], reporter_mac[2],
            reporter_mac[3], reporter_mac[4], reporter_mac[5]);

        // Release the lease if it exists
        if let Some(lease) = self.leases.remove(&ip_u32) {
            self.mac_to_ip.retain(|_, v| *v != ip_u32);
            if let Some(ref cid) = lease.client_id {
                self.client_id_to_ip.retain(|_, v| *v != ip_u32);
            }
        }

        // Also remove from reserved if it was reserved
        self.reserved.remove(&ip_u32);

        // Blacklist this IP for 10 minutes
        self.conflicts.insert(ip_u32, Instant::now());
    }

    /// Number of active conflicts.
    pub fn active_conflict_count(&self) -> usize {
        let now = Instant::now();
        self.conflicts
            .iter()
            .filter(|(_, t)| now.duration_since(**t) < Duration::from_secs(600))
            .count()
    }

    /// Compact the lease database — remove released leases and rebuild indexes.
    /// Returns the number of entries removed.
    pub fn compact(&mut self) -> usize {
        let before = self.leases.len();
        let released: Vec<u32> = self
            .leases
            .iter()
            .filter(|(_, l)| l.state == LeaseState::Released)
            .map(|(ip, _)| *ip)
            .collect();
        for ip_u32 in &released {
            if let Some(lease) = self.leases.remove(ip_u32) {
                self.mac_to_ip.remove(&lease.mac);
                if let Some(ref cid) = lease.client_id {
                    self.client_id_to_ip.remove(cid);
                }
                self.reserved.remove(ip_u32);
            }
        }
        before - self.leases.len()
    }

    /// Get lease state summary for monitoring.
    pub fn state_summary(&self) -> std::collections::HashMap<&'static str, usize> {
        let mut summary = std::collections::HashMap::new();
        for lease in self.leases.values() {
            let key = match lease.state {
                LeaseState::Offered => "offered",
                LeaseState::Bound => "bound",
                LeaseState::Renewing => "renewing",
                LeaseState::Rebinding => "rebinding",
                LeaseState::Released => "released",
            };
            *summary.entry(key).or_insert(0) += 1;
        }
        summary
    }

    /// Save the lease pool to a file for persistence.
    /// Format: one line per lease as `ip_u32,mac_hex,client_id_hex,lease_time,granted_at_epoch,xid`
    pub fn save_to_file(&self, path: &str) -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::fs::File::create(path)?;
        writeln!(f, "# edgerun-dhcp lease database")?;
        writeln!(
            f,
            "# pool_start={} pool_end={}",
            self.pool_start, self.pool_end
        )?;
        for (ip_u32, lease) in &self.leases {
            let mac_hex = edgerun_encoding::hex::bytes_to_hex_sep(&lease.mac, ':');
            let cid_hex = lease
                .client_id
                .as_ref()
                .map(|c| edgerun_encoding::hex::bytes_to_hex_sep(c, ':'))
                .unwrap_or_else(|| "-".to_string());
            let epoch = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                .saturating_sub(lease.granted_at.elapsed().as_secs());
            writeln!(
                f,
                "{},{},{},{},{},{}",
                ip_u32, mac_hex, cid_hex, lease.lease_time, epoch, lease.xid
            )?;
        }
        Ok(())
    }

    /// Load leases from a file. Restores leases that haven't expired.
    /// Returns the number of leases restored.
    pub fn load_from_file(&mut self, path: &str) -> std::io::Result<usize> {
        let content = std::fs::read_to_string(path)?;
        let mut restored = 0;
        let now_epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() != 6 {
                continue;
            }
            let ip_u32: u32 = parts[0].parse().unwrap_or(0);
            let ip = u32_to_ip(ip_u32);
            let Some(mac) = edgerun_encoding::hex::parse_mac(parts[1]) else {
                continue;
            };
            let client_id = if parts[2] != "-" {
                Some(
                    parts[2]
                        .split(':')
                        .filter_map(|s| u8::from_str_radix(s, 16).ok())
                        .collect::<Vec<_>>(),
                )
            } else {
                None
            };
            let lease_time: u32 = parts[3].parse().unwrap_or(0);
            let granted_at_epoch: u64 = parts[4].parse().unwrap_or(0);
            let xid: u32 = parts[5].parse().unwrap_or(0);
            if now_epoch >= granted_at_epoch + lease_time as u64 {
                continue;
            }
            let remaining = lease_time.saturating_sub((now_epoch - granted_at_epoch) as u32);
            if remaining == 0 {
                continue;
            }
            let lease = if let Some(ref cid) = client_id {
                Lease::with_client_id(mac, cid.clone(), ip, remaining, xid)
            } else {
                Lease::new(mac, ip, remaining, xid)
            };
            self.leases.insert(ip_u32, lease);
            self.mac_to_ip.insert(mac, ip_u32);
            if let Some(ref cid) = client_id {
                self.client_id_to_ip.insert(cid.clone(), ip_u32);
            }
            restored += 1;
        }
        Ok(restored)
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

        let ip1 = pool.allocate(mac1, None, 3600, 0x1111).unwrap();
        let ip2 = pool.allocate(mac2, None, 3600, 0x2222).unwrap();

        assert_ne!(ip1, ip2);
        assert_eq!(pool.active_count(), 2);
        assert_eq!(pool.available_count(), 9); // 11 total - 2 leased

        // Same MAC gets same IP
        let ip1_again = pool.allocate(mac1, None, 3600, 0x3333).unwrap();
        assert_eq!(ip1, ip1_again);

        // Release
        pool.release(mac1);
        assert_eq!(pool.active_count(), 1);
    }

    #[test]
    fn test_pool_reserve() {
        let mut pool = LeasePool::new(Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(10, 0, 0, 10));

        pool.reserve(Ipv4Addr::new(10, 0, 0, 1)); // Gateway

        let mac = [0xaa; 6];
        let ip = pool.allocate(mac, None, 3600, 0x1234).unwrap();
        assert_ne!(ip, Ipv4Addr::new(10, 0, 0, 1));
    }

    #[test]
    fn test_pool_exhaustion() {
        let mut pool = LeasePool::new(Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(10, 0, 0, 2));

        let mac1 = [1, 1, 1, 1, 1, 1];
        let mac2 = [2, 2, 2, 2, 2, 2];
        let mac3 = [3, 3, 3, 3, 3, 3];

        assert!(pool.allocate(mac1, None, 3600, 0x1).is_some());
        assert!(pool.allocate(mac2, None, 3600, 0x2).is_some());
        assert!(pool.allocate(mac3, None, 3600, 0x3).is_none());
    }

    #[test]
    fn test_sweep_expired() {
        let mut pool = LeasePool::new(Ipv4Addr::new(10, 0, 0, 1), Ipv4Addr::new(10, 0, 0, 10));

        let mac = [0xff; 6];
        pool.allocate(mac, None, 3600, 0x1234);
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
        let ip = pool.allocate(mac, None, 3600, 0x1234).unwrap();

        let lease_by_mac = pool.find_lease_by_mac(mac).unwrap();
        assert_eq!(lease_by_mac.ip, ip);

        let lease_by_ip = pool.find_lease_by_ip(ip).unwrap();
        assert_eq!(lease_by_ip.mac, mac);
    }

    #[test]
    fn test_conflict_detection() {
        let mut pool = LeasePool::new(
            Ipv4Addr::new(192, 168, 1, 100),
            Ipv4Addr::new(192, 168, 1, 110),
        );

        let mac1 = [1, 2, 3, 4, 5, 6];
        let mac2 = [6, 5, 4, 3, 2, 1];
        let ip = Ipv4Addr::new(192, 168, 1, 100);

        // Allocate to mac1
        let assigned = pool.allocate(mac1, None, 3600, 1).unwrap();
        assert_eq!(assigned, ip);
        assert_eq!(pool.active_conflict_count(), 0);

        // mac2 reports conflict (ARP detected ip already in use)
        pool.record_conflict(ip, mac2);

        // The IP should be blacklisted and not re-allocated
        assert_eq!(pool.active_conflict_count(), 1);
        assert!(pool.find_lease_by_ip(ip).is_none());

        // Next allocation should skip the conflicted IP
        let assigned2 = pool.allocate(mac2, None, 3600, 2).unwrap();
        assert_ne!(assigned2, ip); // Should get the next available IP
    }
}
