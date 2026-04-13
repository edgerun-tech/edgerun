//! RFC 5322 / MIME email message builder.

/// A MIME part within a multipart message.
#[derive(Debug, Clone)]
pub struct MimePart {
    /// Content-Type of this part.
    pub content_type: String,
    /// Content-Transfer-Encoding.
    pub transfer_encoding: String,
    /// Optional Content-Disposition (e.g., `attachment; filename="..."`).
    pub disposition: Option<String>,
    /// Raw body bytes.
    pub body: Vec<u8>,
}

impl MimePart {
    /// Plain text part (UTF-8, 7bit).
    pub fn text(text: &str) -> Self {
        Self {
            content_type: "text/plain; charset=UTF-8".to_string(),
            transfer_encoding: "7bit".to_string(),
            disposition: None,
            body: text.as_bytes().to_vec(),
        }
    }

    /// HTML part (UTF-8, quoted-printable encoded).
    pub fn html(html: &str) -> Self {
        Self {
            content_type: "text/html; charset=UTF-8".to_string(),
            transfer_encoding: "quoted-printable".to_string(),
            disposition: None,
            body: encode_quoted_printable(html),
        }
    }

    /// Attachment with the given filename and MIME type.
    pub fn attachment(filename: &str, mime_type: &str, data: &[u8]) -> Self {
        Self {
            content_type: format!("{}; name=\"{}\"", mime_type, filename),
            transfer_encoding: "base64".to_string(),
            disposition: Some(format!("attachment; filename=\"{}\"", filename)),
            body: encode_base64(data),
        }
    }
}

/// Helper for building RFC 5322 / MIME email messages.
pub struct EmailBuilder {
    headers: Vec<(String, String)>,
    /// If set, this is a multipart message with these parts.
    parts: Vec<MimePart>,
    /// If no parts set, this is the plain body.
    body: String,
}

impl EmailBuilder {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            parts: Vec::new(),
            body: String::new(),
        }
    }

    /// Set the From header.
    pub fn from(mut self, address: &str) -> Self {
        self.headers.push(("From".to_string(), address.to_string()));
        self
    }

    /// Set the To header (call multiple times for multiple recipients).
    pub fn to(mut self, address: &str) -> Self {
        self.headers.push(("To".to_string(), address.to_string()));
        self
    }

    /// Set the Cc header.
    pub fn cc(mut self, address: &str) -> Self {
        self.headers.push(("Cc".to_string(), address.to_string()));
        self
    }

    /// Set the Bcc header.
    pub fn bcc(mut self, address: &str) -> Self {
        self.headers.push(("Bcc".to_string(), address.to_string()));
        self
    }

    /// Set the Subject header.
    pub fn subject(mut self, subject: &str) -> Self {
        self.headers.push(("Subject".to_string(), subject.to_string()));
        self
    }

    /// Set the Date header (auto-generated if not set).
    pub fn date(mut self, date: &str) -> Self {
        self.headers.push(("Date".to_string(), date.to_string()));
        self
    }

    /// Set the Message-ID header (auto-generated if not set).
    pub fn message_id(mut self, id: &str) -> Self {
        self.headers.push(("Message-ID".to_string(), id.to_string()));
        self
    }

    /// Set Content-Type header manually (overridden by `build()` if multipart).
    pub fn content_type(mut self, mime_type: &str) -> Self {
        self.headers.push(("Content-Type".to_string(), mime_type.to_string()));
        self
    }

    /// Add a custom header.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    /// Set the plain text body (non-multipart).
    pub fn body(mut self, body: &str) -> Self {
        self.body = body.to_string();
        self
    }

    /// Add a MIME part (makes the message multipart).
    pub fn part(mut self, part: MimePart) -> Self {
        self.parts.push(part);
        self
    }

    /// Set the multipart boundary string (auto-generated if not set).
    pub fn boundary(mut self, boundary: String) -> Self {
        self.headers.push(("X-Boundary".to_string(), boundary));
        self
    }

    /// Build the complete RFC 5322 / MIME message.
    pub fn build(self) -> Vec<u8> {
        let mut message = String::new();

        // Write headers
        for (name, value) in &self.headers {
            if name == "X-Boundary" {
                continue; // handled below
            }
            // Skip Content-Type — we set it based on structure
            if name == "Content-Type" && !self.parts.is_empty() {
                continue;
            }
            message.push_str(&format!("{}: {}\r\n", name, value));
        }

        // Auto-generate Date if absent
        if !self.headers.iter().any(|(n, _)| n == "Date") {
            message.push_str(&format!("Date: {}\r\n", generate_rfc2822_date()));
        }

        // Auto-generate Message-ID if absent
        if !self.headers.iter().any(|(n, _)| n == "Message-ID") {
            message.push_str(&format!("Message-ID: <{}@edgerun.mail>\r\n", generate_message_id()));
        }

        // Blank line separates headers from body
        message.push_str("\r\n");

        if !self.parts.is_empty() {
            // ── Multipart message ─────────────────────────────────────────
            let boundary = self.headers
                .iter()
                .find(|(n, _)| n == "X-Boundary")
                .map(|(_, v)| v.clone())
                .unwrap_or_else(generate_boundary);

            if self.parts.len() == 2
                && self.parts[0].content_type.starts_with("text/plain")
                && self.parts[1].content_type.starts_with("text/html")
            {
                // multipart/alternative (plain + HTML alternatives)
                message.push_str(&format!("Content-Type: multipart/alternative; boundary=\"{}\"\r\n\r\n", boundary));
            } else {
                // multipart/mixed (body + attachments)
                message.push_str(&format!("Content-Type: multipart/mixed; boundary=\"{}\"\r\n\r\n", boundary));
            }

            for part in &self.parts {
                message.push_str(&format!("--{}\r\n", boundary));
                message.push_str(&format!("Content-Type: {}\r\n", part.content_type));
                message.push_str(&format!("Content-Transfer-Encoding: {}\r\n", part.transfer_encoding));
                if let Some(ref disp) = part.disposition {
                    message.push_str(&format!("Content-Disposition: {}\r\n", disp));
                }
                message.push_str("\r\n");
                message.push_str(&String::from_utf8_lossy(&part.body));
                message.push_str("\r\n");
            }

            // Closing boundary
            message.push_str(&format!("--{}--\r\n", boundary));
        } else {
            // ── Simple single-part message ────────────────────────────────
            message.push_str(&self.body);
            if !message.ends_with("\r\n") {
                message.push_str("\r\n");
            }
        }

        message.into_bytes()
    }
}

impl Default for EmailBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// MIME Encodings
// ===========================================================================

/// Encode bytes as base64 with 76-char line wrapping (RFC 2045).
fn encode_base64(data: &[u8]) -> Vec<u8> {
    const TABLE: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = Vec::with_capacity((data.len() * 4 + 2) / 3);
    let mut col = 0;

    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };

        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(TABLE[((triple >> 18) & 0x3F) as usize]);
        result.push(TABLE[((triple >> 12) & 0x3F) as usize]);
        if chunk.len() > 1 {
            result.push(TABLE[((triple >> 6) & 0x3F) as usize]);
        } else {
            result.push(b'=');
        }
        if chunk.len() > 2 {
            result.push(TABLE[(triple & 0x3F) as usize]);
        } else {
            result.push(b'=');
        }

        col += 4;
        if col >= 76 {
            result.extend_from_slice(b"\r\n");
            col = 0;
        }
    }

    if col > 0 {
        result.extend_from_slice(b"\r\n");
    }

    result
}

/// Encode a string as quoted-printable (RFC 2045).
fn encode_quoted_printable(s: &str) -> Vec<u8> {
    let mut result = Vec::with_capacity(s.len() * 3);
    let mut col = 0;
    const HEX: &[u8] = b"0123456789ABCDEF";

    for &byte in s.as_bytes() {
        match byte {
            b'\r' | b'\n' => {
                result.push(byte);
                col = 0;
            }
            33..=60 | 62..=126 => {
                // Printable ASCII except `=` (61)
                result.push(byte);
                col += 1;
            }
            b' ' | b'\t' => {
                // Space/tab — encode if at end of line
                result.push(byte);
                col += 1;
            }
            _ => {
                result.push(b'=');
                result.push(HEX[((byte >> 4) & 0xF) as usize]);
                result.push(HEX[(byte & 0xF) as usize]);
                col += 3;
            }
        }

        if col >= 73 {
            result.extend_from_slice(b"=\r\n");
            col = 0;
        }
    }

    result
}

// ===========================================================================
// Boundary / ID generation
// ===========================================================================

fn generate_boundary() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("----=_NextPart_{:x}_{:x}", ts, ts.wrapping_mul(0x5DEECE66D) & 0xFFFF_FFFF)
}

fn generate_message_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}.edgerun", ts)
}

/// Generate an RFC 2822 date string.
fn generate_rfc2822_date() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    let secs = dur.as_secs();

    let days = secs / 86400;
    let year = 1970 + days / 365;
    let day_of_year = days % 365;

    let month_days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 0;
    let mut day = day_of_year as i64;
    for (i, &md) in month_days.iter().enumerate() {
        if day < md as i64 {
            month = i;
            break;
        }
        day -= md as i64;
    }
    day += 1;

    let time_of_day = secs % 86400;
    let hours = time_of_day / 3600;
    let mins = (time_of_day % 3600) / 60;
    let secs = time_of_day % 60;
    let dow = ((days + 3) % 7) as usize;

    const MONTHS: &[&str] = &["Jan", "Feb", "Mar", "Apr", "May", "Jun",
                               "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    const DAYS: &[&str] = &["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

    format!("{}, {:02} {} {:04} {:02}:{:02}:{:02} +0000",
        DAYS[dow], day, MONTHS[month], year, hours, mins, secs)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_text_message() {
        let msg = EmailBuilder::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test")
            .body("Hello, World!")
            .build();
        let text = String::from_utf8_lossy(&msg);
        assert!(text.contains("From: sender@example.com"));
        assert!(text.contains("To: recipient@example.com"));
        assert!(text.contains("Subject: Test"));
        assert!(text.contains("Hello, World!"));
    }

    #[test]
    fn test_multipart_alternative() {
        let msg = EmailBuilder::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test")
            .part(MimePart::text("Plain text"))
            .part(MimePart::html("<p>HTML</p>"))
            .build();
        let text = String::from_utf8_lossy(&msg);
        assert!(text.contains("multipart/alternative"));
        assert!(text.contains("Plain text"));
        assert!(text.contains("text/html"));
    }

    #[test]
    fn test_multipart_mixed_with_attachment() {
        let msg = EmailBuilder::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("With attachment")
            .part(MimePart::text("See attached."))
            .part(MimePart::attachment("report.pdf", "application/pdf", b"%PDF-1.4"))
            .build();
        let text = String::from_utf8_lossy(&msg);
        assert!(text.contains("multipart/mixed"));
        assert!(text.contains("report.pdf"));
        assert!(text.contains("Content-Transfer-Encoding: base64"));
        assert!(text.contains("Content-Disposition: attachment"));
    }

    #[test]
    fn test_base64_encode() {
        let encoded = encode_base64(b"Hello");
        let text = String::from_utf8_lossy(&encoded);
        assert!(text.contains("SGVsbG8="));
    }

    #[test]
    fn test_quoted_printable_encode() {
        let encoded = encode_quoted_printable("Hello, =world=");
        let text = String::from_utf8_lossy(&encoded);
        assert!(text.contains("Hello, =3Dworld=3D"));
    }
}
