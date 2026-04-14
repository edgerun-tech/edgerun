//! Outbound mail relay — DNS MX lookup + SMTP delivery to remote MTAs.

use std::io;
use std::time::Duration;

use edgerun_dns::client::DnsClient;
use edgerun_dns::record::DnsRecordData;

use crate::smtp::client::SmtpClient;
use crate::smtp::protocol::read_smtp_line;
use crate::smtp::types::MailEnvelope;

// ===========================================================================
// OutboundRelay
// ===========================================================================

/// Delivers mail to remote MTAs via DNS MX resolution.
pub struct OutboundRelay {
    /// The hostname we announce as in EHLO.
    pub ehlo_domain: String,
    /// DNS server address for MX lookups.
    pub dns_server: String,
    /// Maximum message size we'll attempt to relay.
    pub max_message_size: usize,
    /// Connection timeout for remote SMTP.
    pub connect_timeout: Duration,
}

impl OutboundRelay {
    pub fn new(ehlo_domain: &str) -> Self {
        Self {
            ehlo_domain: ehlo_domain.to_string(),
            dns_server: "8.8.8.8:53".to_string(),
            max_message_size: 35_882_577, // 34 MB
            connect_timeout: Duration::from_secs(30),
        }
    }

    /// Attempt to deliver a message to a single recipient via their domain's MX.
    ///
    /// Returns `Ok(remote_mta)` on success, `Err(reason)` on failure.
    pub async fn deliver_to_recipient(
        &self,
        envelope: &MailEnvelope,
        recipient: &str,
    ) -> Result<String, String> {
        // Extract domain from recipient
        let domain = extract_domain(recipient).ok_or_else(|| {
            format!("invalid recipient address: {}", recipient)
        })?;

        // DNS MX lookup
        let mx_hosts = self.resolve_mx(&domain).await?;
        if mx_hosts.is_empty() {
            // Fall back to A record
            let a_host = format!("{}:25", domain);
            return self.deliver_via_smtp(&a_host, envelope, recipient, &domain).await;
        }

        // Try each MX host in priority order
        let mut last_error = String::new();
        for (_priority, host) in &mx_hosts {
            let addr = format!("{}:25", host);
            match self.deliver_via_smtp(&addr, envelope, recipient, host).await {
                Ok(mta) => return Ok(mta),
                Err(e) => {
                    last_error = e;
                    continue;
                }
            }
        }

        Err(last_error)
    }

    /// Resolve MX records for a domain.
    async fn resolve_mx(&self, domain: &str) -> Result<Vec<(u16, String)>, String> {
        let mut dns = DnsClient::new(&self.dns_server)
            .map_err(|e| format!("failed to create DNS client: {}", e))?;

        let response = dns.query(domain, edgerun_dns::record::DnsRecordType::MX)
            .await
            .map_err(|e| format!("DNS query failed: {}", e))?;

        let mut mx_records = Vec::new();
        for answer in &response.answers {
            if let edgerun_dns::record::DnsRecordData::MX { priority, exchange } = &answer.data {
                mx_records.push((*priority, exchange.clone()));
            }
        }

        // Sort by preference (lower = higher priority)
        mx_records.sort_by_key(|(p, _)| *p);
        Ok(mx_records)
    }

    /// Deliver to a specific SMTP server.
    async fn deliver_via_smtp(
        &self,
        addr: &str,
        envelope: &MailEnvelope,
        recipient: &str,
        remote_mta: &str,
    ) -> Result<String, String> {
        // Connect
        let mut client = SmtpClient::connect(addr).await
            .map_err(|e| format!("connection to {} failed: {}", addr, e))?;

        // EHLO
        client.ehlo(&self.ehlo_domain).await
            .map_err(|e| format!("EHLO failed: {}", e))?;

        // Check SIZE extension
        if self.max_message_size > 0 {
            if let Some(max_size_str) = client.capabilities_map.get("SIZE").and_then(|v| v.as_ref()) {
                if let Ok(max_size) = max_size_str.parse::<usize>() {
                    if envelope.data.len() > max_size {
                        return Err(format!("message too large for {}: {} > {}", remote_mta, envelope.data.len(), max_size));
                    }
                }
            }
        }

        // MAIL FROM
        let from_param = if envelope.data.len() > 0 {
            format!("<{}> SIZE={}", envelope.from, envelope.data.len())
        } else {
            format!("<{}>", envelope.from)
        };
        client.mail_from(&from_param).await
            .map_err(|e| format!("MAIL FROM rejected: {}", e))?;

        // RCPT TO
        client.rcpt_to(&format!("<{}>", recipient)).await
            .map_err(|e| format!("RCPT TO rejected: {}", e))?;

        // DATA
        client.data(&envelope.data).await
            .map_err(|e| format!("DATA rejected: {}", e))?;

        // QUIT
        let _ = client.quit().await;

        Ok(remote_mta.to_string())
    }
}

fn extract_domain(email: &str) -> Option<String> {
    let email = email.trim();
    let email = email.strip_prefix('<').unwrap_or(email);
    let email = email.strip_suffix('>').unwrap_or(email);
    if let Some(at_pos) = email.rfind('@') {
        let domain = &email[at_pos + 1..];
        if !domain.is_empty() {
            return Some(domain.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("user@example.com"), Some("example.com".to_string()));
        assert_eq!(extract_domain("<user@example.com>"), Some("example.com".to_string()));
        assert_eq!(extract_domain("invalid"), None);
        assert_eq!(extract_domain("@"), None);
    }

    #[test]
    fn test_relay_config() {
        let relay = OutboundRelay::new("mail.example.com");
        assert_eq!(relay.ehlo_domain, "mail.example.com");
        assert_eq!(relay.dns_server, "8.8.8.8:53");
        assert_eq!(relay.max_message_size, 35_882_577);
    }
}
