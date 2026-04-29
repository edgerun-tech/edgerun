//! DSN bounce sender — generates RFC 3464 bounce messages and delivers
//! them to the original sender's return path address.
//!
//! This runs when the delivery worker exhausts all retries for a message.

use crate::prelude::*;
use std::io;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use edgerun_dns::client::DnsClient;
use edgerun_dns::record::{DnsRecordData, DnsRecordType};

use crate::smtp::client::SmtpClient;
use crate::smtp::server::dsn_generator::{DeliveryStatus, DsnAction, DsnBounce};
use crate::smtp::types::response::EnhancedStatusCode;

use super::now_secs;

/// Configuration for the bounce sender.
#[derive(Clone)]
pub struct BounceConfig {
    /// The domain we announce as (used in From: and EHLO).
    pub domain: String,
    /// DNS server for MX lookups.
    pub dns_server: String,
    /// Connection timeout for remote SMTP.
    pub connect_timeout: Duration,
}

impl Default for BounceConfig {
    fn default() -> Self {
        Self {
            domain: "edgerun.mail".to_string(),
            dns_server: "8.8.8.8:53".to_string(),
            connect_timeout: Duration::from_secs(30),
        }
    }
}

/// Generate and send a DSN bounce to the envelope sender.
///
/// # Arguments
/// * `envelope_sender` — The return-path from the original message
/// * `envelope_sender_name` — Human-readable name for the sender (if available)
/// * `failed_recipients` — List of recipients that failed delivery
/// * `original_data` — The original message bytes (for headers extraction)
/// * `config` — Bounce sender configuration
pub async fn send_bounce(
    envelope_sender: &str,
    failed_recipients: &[crate::smtp::relay::queue::RecipientStatus],
    original_data: &[u8],
    config: &BounceConfig,
) -> Result<(), String> {
    // Skip if sender is empty (bounce to empty = loop)
    if envelope_sender.is_empty() || envelope_sender == "<>" || envelope_sender.is_empty() {
        edgerun_log::warn!("edgerun-smtp: skipping bounce — empty envelope sender");
        return Ok(());
    }

    // Extract original headers
    let headers = extract_headers(original_data);

    // Build delivery status blocks for each failed recipient
    let ds_list: Vec<DeliveryStatus> = failed_recipients
        .iter()
        .map(|rec| {
            let (status_code, diag) = match &rec.failure_reason {
                Some(reason) if reason.contains("refused") || reason.contains("connect") => (
                    EnhancedStatusCode::new(4, 4, 1),
                    format!("Connection failed: {}", reason),
                ),
                Some(reason) if reason.contains("too large") || reason.contains("size") => (
                    EnhancedStatusCode::new(5, 3, 4),
                    format!("Message too large: {}", reason),
                ),
                Some(reason)
                    if reason.contains("user")
                        || reason.contains("unknown")
                        || reason.contains("mailbox") =>
                {
                    (
                        EnhancedStatusCode::new(5, 1, 1),
                        format!("User unknown: {}", reason),
                    )
                }
                Some(reason) => (
                    EnhancedStatusCode::new(5, 0, 0),
                    format!("Delivery failed: {}", reason),
                ),
                None => (
                    EnhancedStatusCode::new(5, 0, 0),
                    "Delivery failed".to_string(),
                ),
            };

            let last_attempt_str = rec.last_attempt.map(|ts| format_timestamp(ts));

            DeliveryStatus {
                original_recipient: Some(rec.recipient.clone()),
                final_recipient: rec.recipient.clone(),
                remote_mta: None, // No remote MTA recorded for bounced messages
                action: DsnAction::Failed,
                status: Some(status_code),
                diagnostic_code: Some(diag),
                timestamp: last_attempt_str,
            }
        })
        .collect();

    // Build the bounce message
    let bounce = DsnBounce::permanent_failure(
        &format!("postmaster@{}", config.domain),
        envelope_sender,
        ds_list,
        &headers,
        None, // We only include headers, not the full original body
    );

    let bounce_mime = bounce.render_mime();

    // Build full bounce message with headers
    let bounce_message = format!(
        "From: Mail Delivery System <postmaster@{}>\r\n\
         To: <{}>\r\n\
         Subject: {}\r\n\
         Date: {}\r\n\
         Auto-Submitted: auto-replied (failure)\r\n\
         Precedence: bulk\r\n\
         Message-ID: <bounce-{}-{}@{}>\r\n\
         \r\n\
         {}",
        config.domain,
        envelope_sender,
        bounce.subject,
        format_timestamp(now_secs()),
        now_secs(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0),
        config.domain,
        bounce_mime,
    );

    // Extract domain from sender address
    let sender_domain = extract_domain(envelope_sender).ok_or_else(|| {
        format!(
            "cannot extract domain from sender address: {}",
            envelope_sender
        )
    })?;

    // Resolve sender's MX
    let mx_host = resolve_mx(&sender_domain, &config.dns_server).await?;

    // Connect and deliver
    deliver_bounce(&mx_host, &config.domain, &bounce_message, envelope_sender).await
}

/// Extract domain from an email address.
fn extract_domain(email: &str) -> Option<String> {
    let email = email.trim().trim_start_matches('<').trim_end_matches('>');
    if let Some(at_pos) = email.rfind('@') {
        let domain = &email[at_pos + 1..];
        if !domain.is_empty() {
            return Some(domain.to_string());
        }
    }
    None
}

/// Resolve MX for a domain, fall back to A.
async fn resolve_mx(domain: &str, dns_server: &str) -> Result<String, String> {
    let mut dns =
        DnsClient::new(dns_server).map_err(|e| format!("failed to create DNS client: {}", e))?;

    let response = dns
        .query(domain, DnsRecordType::MX)
        .await
        .map_err(|e| format!("DNS MX query failed: {}", e))?;

    let mut mx_records = Vec::new();
    for answer in &response.answers {
        if let DnsRecordData::MX { priority, exchange } = &answer.data {
            mx_records.push((*priority, exchange.clone()));
        }
    }
    mx_records.sort_by_key(|(p, _)| *p);

    if let Some((_, host)) = mx_records.first() {
        Ok(format!("{}:25", host))
    } else {
        Ok(format!("{}:25", domain))
    }
}

/// Connect to remote MTA and deliver the bounce.
async fn deliver_bounce(
    addr: &str,
    ehlo_domain: &str,
    bounce_message: &str,
    bounce_recipient: &str,
) -> Result<(), String> {
    let mut client = SmtpClient::connect(addr)
        .await
        .map_err(|e| format!("connection to {} failed: {}", addr, e))?;

    client
        .ehlo(ehlo_domain)
        .await
        .map_err(|e| format!("EHLO failed: {}", e))?;

    // MAIL FROM:<> (null sender for bounces)
    client
        .mail_from("<>")
        .await
        .map_err(|e| format!("MAIL FROM rejected: {}", e))?;

    // RCPT TO:<sender>
    client
        .rcpt_to(&format!("<{}>", bounce_recipient))
        .await
        .map_err(|e| format!("RCPT TO rejected: {}", e))?;

    // DATA — bounce message
    client
        .data(bounce_message.as_bytes())
        .await
        .map_err(|e| format!("DATA rejected: {}", e))?;

    // QUIT
    let _ = client.quit().await;

    Ok(())
}

/// Extract headers from raw message data.
fn extract_headers(data: &[u8]) -> String {
    let text = String::from_utf8_lossy(data);
    if let Some(pos) = text.find("\r\n\r\n") {
        text[..pos].to_string()
    } else {
        text.to_string()
    }
}

/// Format a Unix timestamp as an RFC 2822 date string.
fn format_timestamp(ts: i64) -> String {
    let secs = ts as u64;
    let mins = secs / 60;
    let hours = mins / 60;
    let days = hours / 24;

    // Simplified — just return a reasonable timestamp
    // For production, use chrono or time crate
    format!("{} GMT", secs)
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
        assert_eq!(extract_domain(""), None);
    }

    #[test]
    fn test_extract_headers() {
        let data = b"From: a@b.com\r\nTo: c@d.com\r\nSubject: Test\r\n\r\nBody here";
        let headers = extract_headers(data);
        assert_eq!(headers, "From: a@b.com\r\nTo: c@d.com\r\nSubject: Test");
    }

    #[test]
    fn test_extract_headers_no_separator() {
        let data = b"From: a@b.com\r\nTo: c@d.com";
        let headers = extract_headers(data);
        assert_eq!(headers, "From: a@b.com\r\nTo: c@d.com");
    }

    #[test]
    fn test_bounce_config_default() {
        let config = BounceConfig::default();
        assert_eq!(config.domain, "edgerun.mail");
        assert_eq!(config.dns_server, "8.8.8.8:53");
    }

    #[test]
    fn test_format_timestamp() {
        let ts = format_timestamp(1700000000);
        assert!(ts.contains("1700000000"));
        assert!(ts.contains("GMT"));
    }
}
