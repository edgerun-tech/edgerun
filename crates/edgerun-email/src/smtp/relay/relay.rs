//! Outbound mail relay — DNS MX lookup + SMTP delivery to remote MTAs.

use crate::prelude::*;
use std::io;
use std::time::Duration;

use edgerun_dns::client::DnsClient;
use edgerun_dns::record::DnsRecordData;

use crate::server::read_line;
use crate::smtp::client::SmtpClient;
use crate::smtp::types::MailEnvelope;

#[cfg(feature = "dkim")]
use edgerun_email_auth::sign::DkimSigner;

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
    #[cfg(feature = "dkim")]
    pub dkim_signer: Option<DkimSigner>,
}

impl OutboundRelay {
    pub fn new(ehlo_domain: &str) -> Self {
        Self {
            ehlo_domain: ehlo_domain.to_string(),
            dns_server: "8.8.8.8:53".to_string(),
            max_message_size: 35_882_577, // 34 MB
            connect_timeout: Duration::from_secs(30),
            #[cfg(feature = "dkim")]
            dkim_signer: None,
        }
    }

    #[cfg(feature = "dkim")]
    pub fn with_dkim_signer(mut self, signer: DkimSigner) -> Self {
        self.dkim_signer = Some(signer);
        self
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
        let domain = extract_domain(recipient)
            .ok_or_else(|| format!("invalid recipient address: {}", recipient))?;

        // DNS MX lookup
        let mx_hosts = self.resolve_mx(&domain).await?;
        if mx_hosts.is_empty() {
            // Fall back to A record
            let a_host = format!("{}:25", domain);
            return self
                .deliver_via_smtp(&a_host, envelope, recipient, &domain)
                .await;
        }

        // Try each MX host in priority order
        let mut last_error = String::new();
        for (_priority, host) in &mx_hosts {
            let addr = format!("{}:25", host);
            match self
                .deliver_via_smtp(&addr, envelope, recipient, host)
                .await
            {
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

        let response = dns
            .query(domain, edgerun_dns::record::DnsRecordType::MX)
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
        // Try STARTTLS first. If the local TLS client cannot interoperate with
        // the remote MTA, retry plaintext rather than wedging the outbound
        // queue. This keeps delivery opportunistic while TLS coverage matures.
        let mut client = match SmtpClient::connect(addr).await {
            Ok(client) => client,
            Err(tls_error) => {
                edgerun_log::warn!(
                    "edgerun-smtp-relay: STARTTLS delivery to {} failed: {}; retrying plaintext",
                    addr,
                    tls_error
                );
                SmtpClient::connect_no_tls(addr)
                    .await
                    .map_err(|plain_error| {
                        format!(
                            "connection to {} failed: STARTTLS {}; plaintext {}",
                            addr, tls_error, plain_error
                        )
                    })?
            }
        };

        // EHLO
        client
            .ehlo(&self.ehlo_domain)
            .await
            .map_err(|e| format!("EHLO failed: {}", e))?;

        // Check SIZE extension
        if self.max_message_size > 0 {
            if let Some(max_size_str) = client.capabilities_map.get("SIZE").and_then(|v| v.as_ref())
            {
                if let Ok(max_size) = max_size_str.parse::<usize>() {
                    if envelope.data.len() > max_size {
                        return Err(format!(
                            "message too large for {}: {} > {}",
                            remote_mta,
                            envelope.data.len(),
                            max_size
                        ));
                    }
                }
            }
        }

        // MAIL FROM
        client
            .mail_from(&envelope.from)
            .await
            .map_err(|e| format!("MAIL FROM rejected: {}", e))?;

        // RCPT TO
        client
            .rcpt_to(recipient)
            .await
            .map_err(|e| format!("RCPT TO rejected: {}", e))?;

        // DKIM sign the message if configured
        let message_data = {
            #[cfg(feature = "dkim")]
            {
                if let Some(ref signer) = self.dkim_signer {
                    match sign_message_data(signer, &envelope.data) {
                        Ok(signed) => signed,
                        Err(e) => {
                            edgerun_log::warn!("DKIM signing failed: {}, sending unsigned", e);
                            envelope.data.clone()
                        }
                    }
                } else {
                    envelope.data.clone()
                }
            }
            #[cfg(not(feature = "dkim"))]
            {
                envelope.data.clone()
            }
        };

        // DATA
        client
            .data(&message_data)
            .await
            .map_err(|e| format!("DATA rejected: {}", e))?;

        // QUIT
        let _ = client.quit().await;

        Ok(remote_mta.to_string())
    }
}

#[cfg(feature = "dkim")]
pub(crate) fn sign_message_data(signer: &DkimSigner, data: &[u8]) -> io::Result<Vec<u8>> {
    let (headers, body) = split_header_body(data);
    let signature = signer
        .sign(headers, body)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
    let mut signed = Vec::with_capacity(data.len() + signature.len() + 4);
    signed.extend_from_slice(headers);
    if !headers.ends_with(b"\r\n") {
        signed.extend_from_slice(b"\r\n");
    }
    signed.extend_from_slice(signature.as_bytes());
    signed.extend_from_slice(b"\r\n\r\n");
    signed.extend_from_slice(body);
    Ok(signed)
}

#[cfg(feature = "dkim")]
fn split_header_body(data: &[u8]) -> (&[u8], &[u8]) {
    if let Some(pos) = find_bytes(data, b"\r\n\r\n") {
        return (&data[..pos], &data[pos + 4..]);
    }
    if let Some(pos) = find_bytes(data, b"\n\n") {
        return (&data[..pos], &data[pos + 2..]);
    }
    (data, &[])
}

#[cfg(feature = "dkim")]
fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
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
        assert_eq!(
            extract_domain("user@example.com"),
            Some("example.com".to_string())
        );
        assert_eq!(
            extract_domain("<user@example.com>"),
            Some("example.com".to_string())
        );
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

    #[cfg(feature = "dkim")]
    #[test]
    fn dkim_signature_is_inserted_as_header() {
        let signer = DkimSigner::generate("example.com", "mail").unwrap();
        let signed = sign_message_data(
            &signer,
            b"From: a@example.com\r\nTo: b@example.net\r\nSubject: Test\r\n\r\nHello\r\n",
        )
        .unwrap();
        let signed = String::from_utf8(signed).unwrap();
        let (headers, body) = signed.split_once("\r\n\r\n").unwrap();

        assert!(headers.contains("\r\nDKIM-Signature: "));
        assert_eq!(body, "Hello\r\n");
        assert!(!body.contains("DKIM-Signature"));
    }
}
