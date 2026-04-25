//! DSN bounce message generation (RFC 3464).
//!
//! Generates `multipart/report; report-type=delivery-status` messages
//! for delivery failure notifications.

use crate::smtp::types::response::EnhancedStatusCode;

/// DSN action describing what happened to a recipient.
#[derive(Debug, Clone, Copy)]
pub enum DsnAction {
    /// Message was successfully delivered.
    Delivered,
    /// Message was relayed to a non-SMTP system.
    Relayed,
    /// Message was expanded to a mailing list.
    Expanded,
    /// Delivery failed (permanent).
    Failed,
    /// Delivery is delayed (temporary failure).
    Delayed,
}

impl DsnAction {
    fn as_str(&self) -> &'static str {
        match self {
            DsnAction::Delivered => "delivered",
            DsnAction::Relayed => "relayed",
            DsnAction::Expanded => "expanded",
            DsnAction::Failed => "failed",
            DsnAction::Delayed => "delayed",
        }
    }
}

/// Per-recipient delivery status fields (RFC 3464 §2.2).
#[derive(Debug, Clone)]
pub struct DeliveryStatus {
    /// Original recipient from the MAIL FROM/RCPT TO envelope.
    pub original_recipient: Option<String>,
    /// Final recipient (after aliasing/forwarding).
    pub final_recipient: String,
    /// Remote MTA that reported the status.
    pub remote_mta: Option<String>,
    /// The action taken (delivered, failed, etc.).
    pub action: DsnAction,
    /// Enhanced status code (X.Y.Z).
    pub status: Option<EnhancedStatusCode>,
    /// Human-readable diagnostic text from the remote MTA.
    pub diagnostic_code: Option<String>,
    /// Date/time of the status report.
    pub timestamp: Option<String>,
}

impl DeliveryStatus {
    /// Format as a `message/delivery-status` block.
    pub fn format(&self) -> String {
        let mut lines = Vec::new();
        if let Some(ref orig) = self.original_recipient {
            lines.push(format!("Original-Recipient: rfc822; {}", orig));
        }
        lines.push(format!("Final-Recipient: rfc822; {}", self.final_recipient));
        if let Some(ref mta) = self.remote_mta {
            lines.push(format!("Remote-MTA: dns; {}", mta));
        }
        if let Some(ref status) = self.status {
            lines.push(format!("Status: {}", status));
        }
        lines.push(format!("Action: {}", self.action.as_str()));
        if let Some(ref diag) = self.diagnostic_code {
            lines.push(format!("Diagnostic-Code: smtp; {}", diag));
        }
        if let Some(ref ts) = self.timestamp {
            lines.push(format!("Last-Attempt-Date: {}", ts));
        }
        lines.join("\r\n")
    }
}

/// A complete DSN bounce message.
#[derive(Debug, Clone)]
pub struct DsnBounce {
    /// Sender of the bounce (typically postmaster@domain).
    pub sender: String,
    /// Recipient of the bounce (typically the envelope sender / return path).
    pub bounce_recipient: String,
    /// Subject line for the bounce.
    pub subject: String,
    /// Human-readable explanation.
    pub explanation: String,
    /// Per-recipient delivery status blocks.
    pub recipients: Vec<DeliveryStatus>,
    /// Original message headers (for inclusion in the bounce).
    pub original_headers: String,
    /// Original message body (only headers if RET=HDRS, full if RET=FULL).
    pub original_body: Option<String>,
}

impl DsnBounce {
    /// Generate a permanent failure bounce.
    pub fn permanent_failure(
        sender: &str,
        bounce_recipient: &str,
        failed_recipients: Vec<DeliveryStatus>,
        original_headers: &str,
        original_body: Option<&str>,
    ) -> Self {
        let failures: Vec<&DeliveryStatus> = failed_recipients
            .iter()
            .filter(|r| matches!(r.action, DsnAction::Failed))
            .collect();

        let count = failures.len();
        let addr = if count == 1 {
            failures[0].final_recipient.clone()
        } else {
            format!("{} recipients", count)
        };

        Self {
            sender: sender.to_string(),
            bounce_recipient: bounce_recipient.to_string(),
            subject: "Mail delivery failed: returning message to sender".to_string(),
            explanation: format!(
                "This is the mail system at {}\r\n\r\n\
                Your message could not be delivered to {}.\r\n\
                No further attempts will be made.",
                sender, addr
            ),
            recipients: failed_recipients,
            original_headers: original_headers.to_string(),
            original_body: original_body.map(String::from),
        }
    }

    /// Generate a delay notification (temporary failure).
    pub fn delivery_delay(
        sender: &str,
        bounce_recipient: &str,
        delayed_recipients: Vec<DeliveryStatus>,
        original_headers: &str,
    ) -> Self {
        let count = delayed_recipients.len();
        let addr = if count == 1 {
            delayed_recipients[0].final_recipient.clone()
        } else {
            format!("{} recipients", count)
        };

        Self {
            sender: sender.to_string(),
            bounce_recipient: bounce_recipient.to_string(),
            subject: format!("Warning: mail delivery delayed to {}", addr),
            explanation: format!(
                "This is the mail system at {}.\r\n\r\n\
                Your message to {} has not yet been delivered.\r\n\
                Delivery will be retried.",
                sender, addr
            ),
            recipients: delayed_recipients,
            original_headers: original_headers.to_string(),
            original_body: None,
        }
    }

    /// Render the full MIME multipart/report message (RFC 3464).
    pub fn render_mime(&self) -> String {
        let boundary = generate_boundary();
        let mut parts = Vec::new();

        // MIME header
        parts.push(format!(
            "Content-Type: multipart/report; report-type=delivery-status; boundary=\"{}\"\r\n\
             MIME-Version: 1.0\r\n",
            boundary
        ));

        // Part 1: Human-readable explanation
        parts.push(format!(
            "--{}\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\
             Content-Transfer-Encoding: 7bit\r\n\r\n\
             {}\r\n",
            boundary, self.explanation
        ));

        // Part 2: Delivery status fields
        let mut ds_block = String::new();
        for recip in &self.recipients {
            if !ds_block.is_empty() {
                ds_block.push_str("\r\n");
            }
            ds_block.push_str(&recip.format());
        }
        parts.push(format!(
            "--{}\r\n\
             Content-Type: message/delivery-status\r\n\r\n\
             {}\r\n",
            boundary, ds_block
        ));

        // Part 3: Original message (headers + optional body)
        let original_text = if let Some(ref body) = self.original_body {
            format!("{}\r\n\r\n{}", self.original_headers, body)
        } else {
            self.original_headers.clone()
        };
        parts.push(format!(
            "--{}\r\n\
             Content-Type: message/rfc822\r\n\r\n\
             {}\r\n",
            boundary, original_text
        ));

        // Close boundary
        parts.push(format!("--{}--\r\n", boundary));

        parts.join("\r\n")
    }
}

/// Generate a unique boundary string.
fn generate_boundary() -> String {
    // Simple unique boundary using a timestamp-like approach
    format!(
        "----=_EdgerunDSN_{}_7f8a9b",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delivery_status_format() {
        let ds = DeliveryStatus {
            original_recipient: Some("user@example.com".to_string()),
            final_recipient: "user@example.com".to_string(),
            remote_mta: Some("mail.example.com".to_string()),
            action: DsnAction::Failed,
            status: Some(EnhancedStatusCode::new(5, 1, 1)),
            diagnostic_code: Some("550 User unknown".to_string()),
            timestamp: None,
        };
        let formatted = ds.format();
        assert!(formatted.contains("Original-Recipient: rfc822; user@example.com"));
        assert!(formatted.contains("Final-Recipient: rfc822; user@example.com"));
        assert!(formatted.contains("Action: failed"));
        assert!(formatted.contains("Status: 5.1.1"));
        assert!(formatted.contains("Diagnostic-Code: smtp; 550 User unknown"));
    }

    #[test]
    fn test_bounce_render_mime() {
        let ds = DeliveryStatus {
            original_recipient: Some("nouser@example.com".to_string()),
            final_recipient: "nouser@example.com".to_string(),
            remote_mta: Some("mx.example.com".to_string()),
            action: DsnAction::Failed,
            status: Some(EnhancedStatusCode::new(5, 1, 1)),
            diagnostic_code: Some("550 No such user".to_string()),
            timestamp: None,
        };

        let bounce = DsnBounce::permanent_failure(
            "postmaster@mail.local",
            "sender@gmail.com",
            vec![ds],
            "From: sender@gmail.com\r\nTo: nouser@example.com\r\nSubject: Hello\r\n",
            Some("This is the body"),
        );

        let mime = bounce.render_mime();
        assert!(mime.contains("multipart/report; report-type=delivery-status"));
        assert!(mime.contains("Content-Type: text/plain"));
        assert!(mime.contains("Content-Type: message/delivery-status"));
        assert!(mime.contains("Content-Type: message/rfc822"));
        assert!(mime.contains("Action: failed"));
        assert!(mime.contains("550 No such user"));
        assert!(mime.contains("This is the body"));
    }

    #[test]
    fn test_dsn_action_strings() {
        assert_eq!(DsnAction::Delivered.as_str(), "delivered");
        assert_eq!(DsnAction::Relayed.as_str(), "relayed");
        assert_eq!(DsnAction::Expanded.as_str(), "expanded");
        assert_eq!(DsnAction::Failed.as_str(), "failed");
        assert_eq!(DsnAction::Delayed.as_str(), "delayed");
    }
}
