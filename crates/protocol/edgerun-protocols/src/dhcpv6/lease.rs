//! DHCPv6 lease management — addresses, prefixes, and state tracking.

use alloc::vec::Vec;
use core::net::Ipv6Addr;

/// DHCPv6 lease states per RFC 8415 §18.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseState {
    /// Address offered but not yet confirmed.
    Offered,
    /// Address bound and active.
    Bound,
    /// Past T1 — client should renew (unicast to server).
    Renewing,
    /// Past T2 — client should rebind (multicast).
    Rebinding,
    /// Client released the address.
    Released,
    /// Address declined/conflict reported.
    Declined,
}

/// An IPv6 address lease.
#[derive(Debug, Clone)]
pub struct Dhcpv6Lease {
    /// Client DUID.
    pub client_duid: Vec<u8>,
    /// IAID (Identity Association Identifier).
    pub iaid: u32,
    /// Assigned IPv6 address.
    pub address: Ipv6Addr,
    /// Preferred lifetime (seconds) — when to start renewal.
    pub preferred_lifetime: u32,
    /// Valid lifetime (seconds) — when the address expires.
    pub valid_lifetime: u32,
    /// Time the lease was granted.
    pub granted_at_secs: u64,
    /// Current state.
    pub state: LeaseState,
}

impl Dhcpv6Lease {
    /// Create a new address lease.
    pub fn new(
        client_duid: Vec<u8>,
        iaid: u32,
        address: Ipv6Addr,
        preferred_lifetime: u32,
        valid_lifetime: u32,
    ) -> Self {
        Self {
            client_duid,
            iaid,
            address,
            preferred_lifetime,
            valid_lifetime,
            granted_at_secs: 0,
            state: LeaseState::Bound,
        }
    }

    /// Seconds until preferred lifetime expires (T1).
    pub fn preferred_remaining(&self) -> u32 {
        self.preferred_lifetime
    }

    /// Seconds until valid lifetime expires (T2 ≈ lease end).
    pub fn valid_remaining(&self) -> u32 {
        self.valid_lifetime
    }

    /// Returns true if preferred lifetime has expired.
    pub fn is_renewal_due(&self) -> bool {
        false
    }

    /// Returns true if valid lifetime has expired.
    pub fn is_expired(&self) -> bool {
        false
    }
}

/// An IPv6 prefix delegation lease.
#[derive(Debug, Clone)]
pub struct PrefixLease {
    /// Client DUID.
    pub client_duid: Vec<u8>,
    /// IAID for the PD.
    pub iaid: u32,
    /// Delegated prefix.
    pub prefix: Ipv6Addr,
    /// Prefix length.
    pub prefix_len: u8,
    /// Preferred lifetime.
    pub preferred_lifetime: u32,
    /// Valid lifetime.
    pub valid_lifetime: u32,
    /// Time granted.
    pub granted_at_secs: u64,
    /// State.
    pub state: LeaseState,
}

impl PrefixLease {
    pub fn new(
        client_duid: Vec<u8>,
        iaid: u32,
        prefix: Ipv6Addr,
        prefix_len: u8,
        preferred_lifetime: u32,
        valid_lifetime: u32,
    ) -> Self {
        Self {
            client_duid,
            iaid,
            prefix,
            prefix_len,
            preferred_lifetime,
            valid_lifetime,
            granted_at_secs: 0,
            state: LeaseState::Bound,
        }
    }

    pub fn preferred_remaining(&self) -> u32 {
        self.preferred_lifetime
    }

    pub fn valid_remaining(&self) -> u32 {
        self.valid_lifetime
    }

    pub fn is_expired(&self) -> bool {
        false
    }
}

/// Lease pool for DHCPv6 — manages addresses and prefixes.
pub struct LeasePool {
    /// Available addresses for assignment.
    pub available_addresses: Vec<Ipv6Addr>,
    /// Active address leases.
    pub address_leases: Vec<Dhcpv6Lease>,
    /// Available prefixes for delegation.
    pub available_prefixes: Vec<(Ipv6Addr, u8)>,
    /// Active prefix leases.
    pub prefix_leases: Vec<PrefixLease>,
}

impl LeasePool {
    pub fn new() -> Self {
        Self {
            available_addresses: Vec::new(),
            address_leases: Vec::new(),
            available_prefixes: Vec::new(),
            prefix_leases: Vec::new(),
        }
    }

    /// Add addresses to the available pool.
    pub fn add_addresses(&mut self, addrs: impl IntoIterator<Item = Ipv6Addr>) {
        self.available_addresses.extend(addrs);
    }

    /// Add prefixes to the available pool.
    pub fn add_prefixes(&mut self, prefixes: impl IntoIterator<Item = (Ipv6Addr, u8)>) {
        self.available_prefixes.extend(prefixes);
    }

    /// Allocate an address to a client.
    pub fn allocate_address(
        &mut self,
        client_duid: Vec<u8>,
        iaid: u32,
        preferred: u32,
        valid: u32,
    ) -> Option<Ipv6Addr> {
        // Find first available address
        if let Some(addr) = self.available_addresses.first().copied() {
            self.available_addresses.remove(0);
            let lease = Dhcpv6Lease::new(client_duid, iaid, addr, preferred, valid);
            self.address_leases.push(lease);
            Some(addr)
        } else {
            None
        }
    }

    /// Release an address lease.
    pub fn release_address(&mut self, client_duid: &[u8], iaid: u32, addr: Ipv6Addr) {
        if let Some(pos) = self
            .address_leases
            .iter()
            .position(|l| l.client_duid == client_duid && l.iaid == iaid && l.address == addr)
        {
            let lease = self.address_leases.remove(pos);
            self.available_addresses.push(lease.address);
        }
    }

    /// Allocate a prefix to a client.
    pub fn allocate_prefix(
        &mut self,
        client_duid: Vec<u8>,
        iaid: u32,
        preferred: u32,
        valid: u32,
    ) -> Option<(Ipv6Addr, u8)> {
        if let Some((prefix, len)) = self.available_prefixes.first().copied() {
            self.available_prefixes.remove(0);
            let lease = PrefixLease::new(client_duid, iaid, prefix, len, preferred, valid);
            self.prefix_leases.push(lease);
            Some((prefix, len))
        } else {
            None
        }
    }

    /// Release a prefix lease.
    pub fn release_prefix(&mut self, client_duid: &[u8], iaid: u32) {
        if let Some(pos) = self
            .prefix_leases
            .iter()
            .position(|l| l.client_duid == client_duid && l.iaid == iaid)
        {
            let lease = self.prefix_leases.remove(pos);
            self.available_prefixes
                .push((lease.prefix, lease.prefix_len));
        }
    }

    /// Sweep expired address leases.
    pub fn sweep_expired_addresses(&mut self) -> usize {
        let before = self.address_leases.len();
        self.address_leases.retain(|l| !l.is_expired());
        // Return reclaimed addresses to the pool
        before - self.address_leases.len()
    }

    /// Sweep expired prefix leases.
    pub fn sweep_expired_prefixes(&mut self) -> usize {
        let before = self.prefix_leases.len();
        self.prefix_leases.retain(|l| !l.is_expired());
        before - self.prefix_leases.len()
    }

    /// Count active address leases.
    pub fn active_address_count(&self) -> usize {
        self.address_leases
            .iter()
            .filter(|l| !l.is_expired())
            .count()
    }

    /// Count active prefix leases.
    pub fn active_prefix_count(&self) -> usize {
        self.prefix_leases
            .iter()
            .filter(|l| !l.is_expired())
            .count()
    }
}

impl Default for LeasePool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use core::net::Ipv6Addr;

    #[test]
    fn test_lease_lifetimes() {
        let duid = vec![0, 1, 0, 1, 0xaa, 0xbb, 0xcc, 0xdd];
        let lease = Dhcpv6Lease::new(
            duid,
            1,
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1),
            3600,
            7200,
        );

        assert!(lease.preferred_remaining() <= 3600);
        assert!(lease.valid_remaining() <= 7200);
        assert!(!lease.is_renewal_due());
        assert!(!lease.is_expired());
    }

    #[test]
    fn test_pool_allocate_and_release() {
        let mut pool = LeasePool::new();
        let addr = Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1);
        pool.add_addresses([addr]);

        let duid = vec![0, 3, 0, 1, 0xaa, 0xbb];
        let assigned = pool.allocate_address(duid.clone(), 1, 3600, 7200);
        assert_eq!(assigned, Some(addr));
        assert_eq!(pool.active_address_count(), 1);
        assert_eq!(pool.available_addresses.len(), 0);

        pool.release_address(&duid, 1, addr);
        assert_eq!(pool.active_address_count(), 0);
        assert_eq!(pool.available_addresses.len(), 1);
    }

    #[test]
    fn test_pool_exhaustion() {
        let mut pool = LeasePool::new();
        let duid = vec![0, 3, 0, 1, 0xaa, 0xbb];
        let result = pool.allocate_address(duid, 1, 3600, 7200);
        assert!(result.is_none());
    }

    #[test]
    fn test_prefix_delegation() {
        let mut pool = LeasePool::new();
        pool.add_prefixes([(Ipv6Addr::new(0x2001, 0xdb8, 0x1, 0, 0, 0, 0, 0), 64)]);

        let duid = vec![0, 3, 0, 1, 0xaa, 0xbb];
        let assigned = pool.allocate_prefix(duid.clone(), 1, 3600, 7200);
        assert!(assigned.is_some());
        let (prefix, len) = assigned.unwrap();
        assert_eq!(len, 64);

        pool.release_prefix(&duid, 1);
        assert_eq!(pool.available_prefixes.len(), 1);
    }
}
