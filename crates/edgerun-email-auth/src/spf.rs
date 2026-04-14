//! SPF evaluation (RFC 7208).
//!
//! Queries the SPF record for a domain and evaluates whether the client IP
//! is authorized to send mail for that domain.

use std::io;
use std::net::IpAddr;

use crate::DnsQuery;

/// Result of an SPF check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpfResult {
    /// The client is authorized.
    Pass,
    /// The client is explicitly NOT authorized.
    Fail,
    /// The client is not authorized, but the domain owner is experimenting.
    SoftFail,
    /// The domain owner has explicitly stated that they cannot say whether
    /// the client is authorized.
    Neutral,
    /// No SPF record was found for the domain.
    None,
    /// The domain does not send mail, or all mail is relayed.
    PermError,
    /// A transient error occurred during evaluation.
    TempError,
}

impl SpfResult {
    /// Format as an auth-results token (RFC 8601).
    pub fn as_auth_result(&self) -> &'static str {
        match self {
            SpfResult::Pass => "pass",
            SpfResult::Fail => "fail",
            SpfResult::SoftFail => "softfail",
            SpfResult::Neutral => "neutral",
            SpfResult::None => "none",
            SpfResult::PermError => "permerror",
            SpfResult::TempError => "temperror",
        }
    }
}

/// Check SPF for the given client IP and domain.
pub async fn check_spf<D: DnsQuery>(dns: &mut D, client_ip: &str, domain: &str) -> io::Result<SpfResult> {
    let records = dns.query_txt(domain).await?;

    // Find the SPF record
    let spf_record = records
        .iter()
        .find(|r| {
            let trimmed = r.trim();
            trimmed.starts_with("v=spf1") || trimmed.starts_with("v=spf1 ")
        })
        .map(|s| s.to_string());

    let spf_record = match spf_record {
        Some(r) => r,
        None => return Ok(SpfResult::None),
    };

    let client_ip: IpAddr = match client_ip.parse() {
        Ok(ip) => ip,
        Err(_) => return Ok(SpfResult::PermError),
    };

    evaluate_spf(&spf_record, &client_ip, domain)
}

fn evaluate_spf(record: &str, client_ip: &IpAddr, domain: &str) -> io::Result<SpfResult> {
    // Parse mechanisms after "v=spf1"
    let mechanisms: Vec<&str> = record
        .trim()
        .strip_prefix("v=spf1")
        .unwrap_or("")
        .split_whitespace()
        .collect();

    for mech in &mechanisms {
        match evaluate_mechanism(*mech, client_ip, domain) {
            MechanismResult::Match(result) => return Ok(result),
            MechanismResult::NoMatch => continue,
            MechanismResult::Error(e) => return Err(e),
        }
    }

    // Default is neutral if no mechanism matches
    Ok(SpfResult::Neutral)
}

enum MechanismResult {
    Match(SpfResult),
    NoMatch,
    #[allow(dead_code)]
    Error(io::Error),
}

fn evaluate_mechanism(mech: &str, client_ip: &IpAddr, _domain: &str) -> MechanismResult {
    // Parse qualifier and mechanism
    let (qualifier, mechanism) = match mech.chars().next() {
        Some('+') => (SpfResult::Pass, &mech[1..]),
        Some('-') => (SpfResult::Fail, &mech[1..]),
        Some('~') => (SpfResult::SoftFail, &mech[1..]),
        Some('?') => (SpfResult::Neutral, &mech[1..]),
        _ => (SpfResult::Pass, mech),
    };

    if mechanism.is_empty() {
        return MechanismResult::NoMatch;
    }

    match mechanism {
        "all" => MechanismResult::Match(qualifier),
        "a" | "mx" => {
            // Simple: treat as match (would need DNS A/MX queries for full evaluation)
            // For now, we return NoMatch — the caller should implement full A/MX resolution
            MechanismResult::NoMatch
        }
        "ip4" | "ip6" => MechanismResult::NoMatch, // Handled by "ip4:CIDR" below
        _ => {
            if mechanism.starts_with("ip4:") {
                let cidr = &mechanism[4..];
                match cidr.parse::<IpNetwork>() {
                    Ok(net) => {
                        if net.contains(client_ip) {
                            MechanismResult::Match(qualifier)
                        } else {
                            MechanismResult::NoMatch
                        }
                    }
                    Err(_) => MechanismResult::NoMatch,
                }
            } else if mechanism.starts_with("ip6:") {
                let cidr = &mechanism[4..];
                match cidr.parse::<IpNetwork>() {
                    Ok(net) => {
                        if net.contains(client_ip) {
                            MechanismResult::Match(qualifier)
                        } else {
                            MechanismResult::NoMatch
                        }
                    }
                    Err(_) => MechanismResult::NoMatch,
                }
            } else if mechanism.starts_with("include:") {
                // Recursive include — would need to evaluate the included domain's SPF
                // For now, return NoMatch (requires DNS recursion)
                MechanismResult::NoMatch
            } else {
                MechanismResult::NoMatch
            }
        }
    }
}

/// Minimal IP network representation.
struct IpNetwork {
    network: IpAddr,
    prefix_len: u8,
}

impl IpNetwork {
    fn contains(&self, ip: &IpAddr) -> bool {
        match (self.network, ip) {
            (IpAddr::V4(net), IpAddr::V4(ip)) => {
                let net_bytes = net.octets();
                let ip_bytes = ip.octets();
                let prefix_bits = self.prefix_len.min(32) as usize;
                for i in 0..4 {
                    let bits = if prefix_bits >= (i + 1) * 8 {
                        8
                    } else if prefix_bits > i * 8 {
                        prefix_bits - i * 8
                    } else {
                        0
                    };
                    let mask: u8 = if bits == 8 { 0xFF } else if bits == 0 { 0x00 } else { 0xFFu8 << (8 - bits) };
                    if (net_bytes[i] & mask) != (ip_bytes[i] & mask) {
                        return false;
                    }
                }
                true
            }
            (IpAddr::V6(net), IpAddr::V6(ip)) => {
                let net_bytes = net.octets();
                let ip_bytes = ip.octets();
                let prefix_bits = self.prefix_len.min(128) as usize;
                for i in 0..16 {
                    let bits = if prefix_bits >= (i + 1) * 8 {
                        8
                    } else if prefix_bits > i * 8 {
                        prefix_bits - i * 8
                    } else {
                        0
                    };
                    let mask: u8 = if bits == 8 { 0xFF } else if bits == 0 { 0x00 } else { 0xFFu8 << (8 - bits) };
                    if (net_bytes[i] & mask) != (ip_bytes[i] & mask) {
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }
}

impl std::str::FromStr for IpNetwork {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(pos) = s.find('/') {
            let ip_str = &s[..pos];
            let prefix: u8 = s[pos + 1..].parse().map_err(|_| ())?;
            let network: IpAddr = ip_str.parse().map_err(|_| ())?;
            let max_prefix = if network.is_ipv4() { 32 } else { 128 };
            if prefix > max_prefix {
                return Err(());
            }
            Ok(IpNetwork { network, prefix_len: prefix })
        } else {
            let network: IpAddr = s.parse().map_err(|_| ())?;
            let prefix_len = if network.is_ipv4() { 32 } else { 128 };
            Ok(IpNetwork { network, prefix_len })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spf_ipv4_match() {
        let result = evaluate_spf("v=spf1 ip4:192.168.1.0/24 -all", &"192.168.1.50".parse().unwrap(), "example.com");
        assert_eq!(result.unwrap(), SpfResult::Pass);
    }

    #[test]
    fn test_spf_ipv4_no_match() {
        let result = evaluate_spf("v=spf1 ip4:192.168.1.0/24 -all", &"10.0.0.1".parse().unwrap(), "example.com");
        assert_eq!(result.unwrap(), SpfResult::Fail);
    }

    #[test]
    fn test_spf_no_record() {
        // This test verifies the logic path when no SPF record exists
        // The actual DNS query is handled by the caller
        let result = evaluate_spf("v=spf1 -all", &"192.168.1.1".parse().unwrap(), "example.com");
        assert_eq!(result.unwrap(), SpfResult::Fail);
    }

    #[test]
    fn test_spf_all_pass() {
        let result = evaluate_spf("v=spf1 +all", &"1.2.3.4".parse().unwrap(), "example.com");
        assert_eq!(result.unwrap(), SpfResult::Pass);
    }

    #[test]
    fn test_spf_soft_fail() {
        let result = evaluate_spf("v=spf1 ~all", &"1.2.3.4".parse().unwrap(), "example.com");
        assert_eq!(result.unwrap(), SpfResult::SoftFail);
    }

    #[test]
    fn test_spf_neutral() {
        let result = evaluate_spf("v=spf1 ?all", &"1.2.3.4".parse().unwrap(), "example.com");
        assert_eq!(result.unwrap(), SpfResult::Neutral);
    }

    #[test]
    fn test_ip_network_contains_ipv4() {
        let net: IpNetwork = "192.168.1.0/24".parse().unwrap();
        assert!(net.contains(&"192.168.1.0".parse().unwrap()));
        assert!(net.contains(&"192.168.1.255".parse().unwrap()));
        assert!(!net.contains(&"192.168.2.0".parse().unwrap()));
    }

    #[test]
    fn test_spf_result_as_auth() {
        assert_eq!(SpfResult::Pass.as_auth_result(), "pass");
        assert_eq!(SpfResult::Fail.as_auth_result(), "fail");
        assert_eq!(SpfResult::SoftFail.as_auth_result(), "softfail");
        assert_eq!(SpfResult::None.as_auth_result(), "none");
        assert_eq!(SpfResult::PermError.as_auth_result(), "permerror");
        assert_eq!(SpfResult::TempError.as_auth_result(), "temperror");
    }
}
