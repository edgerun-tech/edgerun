//! DMARC evaluation (RFC 7489).

use std::io;

use crate::dkim::{DkimResult, DkimStatus};
use crate::spf::SpfResult;
use crate::DnsQuery;

/// Result of a DMARC evaluation.
#[derive(Debug, Clone)]
pub struct DmarcResult {
    pub status: DmarcStatus,
    pub domain: Option<String>,
    pub policy: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DmarcStatus {
    Pass,
    Fail,
    None,
}

impl DmarcResult {
    pub fn pass(domain: &str) -> Self {
        Self {
            status: DmarcStatus::Pass,
            domain: Some(domain.to_string()),
            policy: None,
            error: None,
        }
    }

    pub fn fail(domain: &str) -> Self {
        Self {
            status: DmarcStatus::Fail,
            domain: Some(domain.to_string()),
            policy: None,
            error: None,
        }
    }

    pub fn none(reason: &str) -> Self {
        Self {
            status: DmarcStatus::None,
            domain: None,
            policy: None,
            error: Some(reason.to_string()),
        }
    }

    pub fn as_auth_result(&self) -> &'static str {
        match self.status {
            DmarcStatus::Pass => "pass",
            DmarcStatus::Fail => "fail",
            DmarcStatus::None => "none",
        }
    }
}

/// Parsed DMARC policy record.
#[derive(Debug, Clone)]
pub struct DmarcPolicy {
    pub policy: String,       // p= (none, quarantine, reject)
    pub sp_policy: String,    // sp= (subdomain policy)
    pub pct: u8,              // pct= (percentage)
    pub rua: Vec<String>,     // rua= (aggregate report URIs)
    pub ruf: Vec<String>,     // ruf= (forensic report URIs)
    pub adkim: String,        // adkim= (r=relaxed, s=strict)
    pub aspf: String,         // aspf= (r=relaxed, s=strict)
}

impl Default for DmarcPolicy {
    fn default() -> Self {
        Self {
            policy: "none".to_string(),
            sp_policy: "none".to_string(),
            pct: 100,
            rua: Vec::new(),
            ruf: Vec::new(),
            adkim: "r".to_string(),
            aspf: "r".to_string(),
        }
    }
}

/// Evaluate DMARC for a domain.
pub async fn evaluate_dmarc<D: DnsQuery>(
    dns: &mut D,
    domain: &str,
    header_from: &str,
    spf: &SpfResult,
    dkim: &[DkimResult],
) -> io::Result<DmarcResult> {
    // Fetch DMARC record
    let dmarc_domain = format!("_dmarc.{}", domain);
    let records = dns.query_txt(&dmarc_domain).await?;

    let dmarc_record = records
        .iter()
        .find(|r| {
            let trimmed = r.trim();
            trimmed.starts_with("v=DMARC1") || trimmed.starts_with("v=dmarc1 ")
        })
        .map(|s| s.to_string());

    let dmarc_record = match dmarc_record {
        Some(r) => r,
        None => return Ok(DmarcResult::none("no DMARC record found")),
    };

    let policy = parse_dmarc_record(&dmarc_record)?;

    // Check identifier alignment
    let spf_aligned = is_spf_aligned(spf, header_from, &policy);
    let dkim_aligned = is_dkim_aligned(dkim, header_from, &policy);

    if spf_aligned || dkim_aligned {
        Ok(DmarcResult::pass(domain))
    } else {
        Ok(DmarcResult::fail(domain))
    }
}

fn parse_dmarc_record(record: &str) -> io::Result<DmarcPolicy> {
    let mut policy = DmarcPolicy::default();

    for (tag, value) in edgerun_encoding::kv::parse_tag_list_semicolon(record) {
        let tag = tag.to_lowercase();
        let value = value.as_str();

        match tag.as_str() {
            "p" => policy.policy = value.to_lowercase(),
            "sp" => policy.sp_policy = value.to_lowercase(),
            "pct" => policy.pct = value.parse().unwrap_or(100),
            "adkim" => policy.adkim = value.to_lowercase(),
            "aspf" => policy.aspf = value.to_lowercase(),
            "rua" => {
                policy.rua = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
            }
            "ruf" => {
                policy.ruf = value
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .collect();
            }
            _ => {}
        }
    }

    // Validate policy
    if !["none", "quarantine", "reject"].contains(&policy.policy.as_str()) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid DMARC policy: {}", policy.policy),
        ));
    }

    Ok(policy)
}

fn is_spf_aligned(spf: &SpfResult, header_from: &str, policy: &DmarcPolicy) -> bool {
    if !matches!(spf, SpfResult::Pass) {
        return false;
    }

    // SPF alignment: envelope_from domain must align with header_from domain
    // For relaxed alignment, organizational domain must match
    // For strict alignment, domains must be identical
    let _header_domain = extract_organizational_domain(header_from);

    // SPF doesn't directly provide the aligned domain — we check if the
    // envelope sender domain matches the header from domain
    // This is a simplified check; a full implementation would extract the
    // envelope sender domain from the MAIL FROM and compare
    // For now, we consider SPF pass as potentially aligned (the caller
    // should verify this with the actual envelope_from domain)
    let header_org = extract_organizational_domain(header_from);

    // Simplified: if SPF passed, we consider it aligned for now
    // A full implementation would need the envelope_from domain
    matches!(policy.aspf.as_str(), "r") && !header_org.is_empty()
}

fn is_dkim_aligned(dkim: &[DkimResult], header_from: &str, policy: &DmarcPolicy) -> bool {
    let header_domain = extract_organizational_domain(header_from);

    for result in dkim {
        if result.status != DkimStatus::Pass {
            continue;
        }

        if let Some(ref sig_domain) = result.domain {
            let sig_org = extract_organizational_domain(sig_domain);

            let aligned = match policy.adkim.as_str() {
                "s" => sig_domain == header_from || sig_domain == &header_domain,
                _ => sig_org == header_domain,
            };

            if aligned {
                return true;
            }
        }
    }

    false
}

/// Extract the organizational domain (e.g., "example.com" from "mail.example.com").
/// This is a simplified implementation — a full implementation would use the
/// Public Suffix List.
fn extract_organizational_domain(domain: &str) -> String {
    let domain = domain.trim().to_lowercase();
    let parts: Vec<&str> = domain.split('.').collect();

    if parts.len() >= 2 {
        // Take the last two parts as the organizational domain
        // This is a simplification; the real implementation needs the PSL
        let start = parts.len() - 2;
        parts[start..].join(".")
    } else {
        domain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dmarc_record() {
        let record = "v=DMARC1; p=reject; sp=quarantine; pct=50; adkim=s; aspf=s; rua=mailto:dmarc@example.com";
        let policy = parse_dmarc_record(record).unwrap();
        assert_eq!(policy.policy, "reject");
        assert_eq!(policy.sp_policy, "quarantine");
        assert_eq!(policy.pct, 50);
        assert_eq!(policy.adkim, "s");
        assert_eq!(policy.aspf, "s");
        assert_eq!(policy.rua, vec!["mailto:dmarc@example.com"]);
    }

    #[test]
    fn test_extract_organizational_domain() {
        assert_eq!(extract_organizational_domain("mail.example.com"), "example.com");
        assert_eq!(extract_organizational_domain("example.com"), "example.com");
        assert_eq!(extract_organizational_domain("sub.mail.example.com"), "example.com");
    }

    #[test]
    fn test_dmarc_result_as_auth() {
        assert_eq!(DmarcResult::pass("example.com").as_auth_result(), "pass");
        assert_eq!(DmarcResult::fail("example.com").as_auth_result(), "fail");
        assert_eq!(DmarcResult::none("no record").as_auth_result(), "none");
    }
}
