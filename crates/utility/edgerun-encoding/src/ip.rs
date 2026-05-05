//! IPv4 address encoding/decoding.
//!
//! Provides big-endian `u32` ↔ `Ipv4Addr` conversion used by
//! DHCP lease files and config storage.
//!
//! All operations are `no_std` compatible.

use core::net::Ipv4Addr;

/// Convert an `Ipv4Addr` to a big-endian `u32`.
///
/// # Examples
/// ```
/// use edgerun_encoding::ip::ip_to_u32;
/// use core::net::Ipv4Addr;
/// assert_eq!(ip_to_u32(&Ipv4Addr::new(192, 168, 1, 1)), 0xc0a80101);
/// ```
pub fn ip_to_u32(ip: &Ipv4Addr) -> u32 {
    let octets = ip.octets();
    u32::from_be_bytes(octets)
}

/// Convert a big-endian `u32` to an `Ipv4Addr`.
///
/// # Examples
/// ```
/// use edgerun_encoding::ip::u32_to_ip;
/// use core::net::Ipv4Addr;
/// assert_eq!(u32_to_ip(0xc0a80101), Ipv4Addr::new(192, 168, 1, 1));
/// ```
pub fn u32_to_ip(n: u32) -> Ipv4Addr {
    Ipv4Addr::from(n.to_be_bytes())
}

/// Compute the broadcast address for a given IP and subnet mask.
///
/// # Examples
/// ```
/// use edgerun_encoding::ip::{ip_to_u32, u32_to_ip, broadcast_address};
/// use core::net::Ipv4Addr;
/// let ip = Ipv4Addr::new(192, 168, 1, 10);
/// let mask = Ipv4Addr::new(255, 255, 255, 0);
/// assert_eq!(broadcast_address(&ip, &mask), Ipv4Addr::new(192, 168, 1, 255));
/// ```
pub fn broadcast_address(ip: &Ipv4Addr, mask: &Ipv4Addr) -> Ipv4Addr {
    let ip_bits = ip_to_u32(ip);
    let mask_bits = ip_to_u32(mask);
    let broadcast_bits = ip_bits | !mask_bits;
    u32_to_ip(broadcast_bits)
}

/// Compute the network address for a given IP and subnet mask.
///
/// # Examples
/// ```
/// use edgerun_encoding::ip::{network_address};
/// use core::net::Ipv4Addr;
/// let ip = Ipv4Addr::new(192, 168, 1, 10);
/// let mask = Ipv4Addr::new(255, 255, 255, 0);
/// assert_eq!(network_address(&ip, &mask), Ipv4Addr::new(192, 168, 1, 0));
/// ```
pub fn network_address(ip: &Ipv4Addr, mask: &Ipv4Addr) -> Ipv4Addr {
    let ip_bits = ip_to_u32(ip);
    let mask_bits = ip_to_u32(mask);
    u32_to_ip(ip_bits & mask_bits)
}

/// Check if an IP address is within the given network.
///
/// # Examples
/// ```
/// use edgerun_encoding::ip::ip_in_network;
/// use core::net::Ipv4Addr;
/// let ip = Ipv4Addr::new(192, 168, 1, 50);
/// let network = Ipv4Addr::new(192, 168, 1, 0);
/// let mask = Ipv4Addr::new(255, 255, 255, 0);
/// assert!(ip_in_network(&ip, &network, &mask));
/// ```
pub fn ip_in_network(ip: &Ipv4Addr, network: &Ipv4Addr, mask: &Ipv4Addr) -> bool {
    network_address(ip, mask) == *network
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_to_u32_basic() {
        let ip = Ipv4Addr::new(192, 168, 1, 1);
        assert_eq!(ip_to_u32(&ip), 0xc0a80101);
    }

    #[test]
    fn test_ip_to_u32_zero() {
        let ip = Ipv4Addr::new(0, 0, 0, 0);
        assert_eq!(ip_to_u32(&ip), 0);
    }

    #[test]
    fn test_ip_to_u32_broadcast() {
        let ip = Ipv4Addr::new(255, 255, 255, 255);
        assert_eq!(ip_to_u32(&ip), 0xffffffff);
    }

    #[test]
    fn test_u32_to_ip_basic() {
        assert_eq!(u32_to_ip(0xc0a80101), Ipv4Addr::new(192, 168, 1, 1));
    }

    #[test]
    fn test_u32_to_ip_zero() {
        assert_eq!(u32_to_ip(0), Ipv4Addr::new(0, 0, 0, 0));
    }

    #[test]
    fn test_u32_to_ip_broadcast() {
        assert_eq!(u32_to_ip(0xffffffff), Ipv4Addr::new(255, 255, 255, 255));
    }

    #[test]
    fn test_roundtrip() {
        let ip = Ipv4Addr::new(10, 0, 0, 1);
        assert_eq!(u32_to_ip(ip_to_u32(&ip)), ip);
    }

    #[test]
    fn test_broadcast_address() {
        let ip = Ipv4Addr::new(192, 168, 1, 10);
        let mask = Ipv4Addr::new(255, 255, 255, 0);
        assert_eq!(
            broadcast_address(&ip, &mask),
            Ipv4Addr::new(192, 168, 1, 255)
        );
    }

    #[test]
    fn test_broadcast_address_class_a() {
        let ip = Ipv4Addr::new(10, 0, 0, 1);
        let mask = Ipv4Addr::new(255, 0, 0, 0);
        assert_eq!(
            broadcast_address(&ip, &mask),
            Ipv4Addr::new(10, 255, 255, 255)
        );
    }

    #[test]
    fn test_network_address() {
        let ip = Ipv4Addr::new(192, 168, 1, 10);
        let mask = Ipv4Addr::new(255, 255, 255, 0);
        assert_eq!(network_address(&ip, &mask), Ipv4Addr::new(192, 168, 1, 0));
    }

    #[test]
    fn test_ip_in_network_true() {
        let ip = Ipv4Addr::new(192, 168, 1, 50);
        let network = Ipv4Addr::new(192, 168, 1, 0);
        let mask = Ipv4Addr::new(255, 255, 255, 0);
        assert!(ip_in_network(&ip, &network, &mask));
    }

    #[test]
    fn test_ip_in_network_false() {
        let ip = Ipv4Addr::new(192, 168, 2, 50);
        let network = Ipv4Addr::new(192, 168, 1, 0);
        let mask = Ipv4Addr::new(255, 255, 255, 0);
        assert!(!ip_in_network(&ip, &network, &mask));
    }
}
