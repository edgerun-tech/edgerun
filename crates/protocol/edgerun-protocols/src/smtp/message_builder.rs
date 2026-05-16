//! RFC 5322 / MIME message builder.
//!
//! The builder formats message bytes from explicit inputs. Runtime-generated
//! values such as current date or random message IDs must be supplied by the
//! caller through headers or explicit builder options.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

/// A MIME part within a multipart message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MimePart {
    /// Content-Type of this part.
    pub content_type: String,
    /// Content-Transfer-Encoding.
    pub transfer_encoding: String,
    /// Optional Content-Disposition.
    pub disposition: Option<String>,
    /// Encoded body bytes.
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
    parts: Vec<MimePart>,
    body: String,
    boundary: Option<String>,
}

impl EmailBuilder {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
            parts: Vec::new(),
            body: String::new(),
            boundary: None,
        }
    }

    pub fn from(mut self, address: &str) -> Self {
        self.headers.push(("From".to_string(), address.to_string()));
        self
    }

    pub fn to(mut self, address: &str) -> Self {
        self.headers.push(("To".to_string(), address.to_string()));
        self
    }

    pub fn cc(mut self, address: &str) -> Self {
        self.headers.push(("Cc".to_string(), address.to_string()));
        self
    }

    pub fn bcc(mut self, address: &str) -> Self {
        self.headers.push(("Bcc".to_string(), address.to_string()));
        self
    }

    pub fn subject(mut self, subject: &str) -> Self {
        self.headers
            .push(("Subject".to_string(), subject.to_string()));
        self
    }

    pub fn date(mut self, date: &str) -> Self {
        self.headers.push(("Date".to_string(), date.to_string()));
        self
    }

    pub fn message_id(mut self, id: &str) -> Self {
        self.headers
            .push(("Message-ID".to_string(), id.to_string()));
        self
    }

    pub fn content_type(mut self, mime_type: &str) -> Self {
        self.headers
            .push(("Content-Type".to_string(), mime_type.to_string()));
        self
    }

    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn body(mut self, body: &str) -> Self {
        self.body = body.to_string();
        self
    }

    pub fn part(mut self, part: MimePart) -> Self {
        self.parts.push(part);
        self
    }

    pub fn boundary(mut self, boundary: String) -> Self {
        self.boundary = Some(boundary);
        self
    }

    /// Build the complete RFC 5322 / MIME message.
    pub fn build(self) -> Vec<u8> {
        let mut message = String::new();

        for (name, value) in &self.headers {
            if name == "Content-Type" && !self.parts.is_empty() {
                continue;
            }
            message.push_str(&format!("{}: {}\r\n", name, value));
        }

        message.push_str("\r\n");

        if !self.parts.is_empty() {
            let default_boundary = self.default_boundary();
            let boundary = self.boundary.unwrap_or(default_boundary);

            if self.parts.len() == 2
                && self.parts[0].content_type.starts_with("text/plain")
                && self.parts[1].content_type.starts_with("text/html")
            {
                message.push_str(&format!(
                    "Content-Type: multipart/alternative; boundary=\"{}\"\r\n\r\n",
                    boundary
                ));
            } else {
                message.push_str(&format!(
                    "Content-Type: multipart/mixed; boundary=\"{}\"\r\n\r\n",
                    boundary
                ));
            }

            for part in &self.parts {
                message.push_str(&format!("--{}\r\n", boundary));
                message.push_str(&format!("Content-Type: {}\r\n", part.content_type));
                message.push_str(&format!(
                    "Content-Transfer-Encoding: {}\r\n",
                    part.transfer_encoding
                ));
                if let Some(ref disp) = part.disposition {
                    message.push_str(&format!("Content-Disposition: {}\r\n", disp));
                }
                message.push_str("\r\n");
                message.push_str(&String::from_utf8_lossy(&part.body));
                message.push_str("\r\n");
            }

            message.push_str(&format!("--{}--\r\n", boundary));
        } else {
            message.push_str(&self.body);
            if !message.ends_with("\r\n") {
                message.push_str("\r\n");
            }
        }

        message.into_bytes()
    }

    fn default_boundary(&self) -> String {
        let mut hash = 0xcbf29ce484222325u64;
        fn feed(hash: &mut u64, bytes: &[u8]) {
            for byte in bytes {
                *hash ^= u64::from(*byte);
                *hash = hash.wrapping_mul(0x100000001b3);
            }
        }
        for (name, value) in &self.headers {
            feed(&mut hash, name.as_bytes());
            feed(&mut hash, value.as_bytes());
        }
        feed(&mut hash, self.body.as_bytes());
        for part in &self.parts {
            feed(&mut hash, part.content_type.as_bytes());
            feed(&mut hash, part.transfer_encoding.as_bytes());
            if let Some(disposition) = &part.disposition {
                feed(&mut hash, disposition.as_bytes());
            }
            feed(&mut hash, &part.body);
        }
        format!("----=_EdgerunPart_{:016x}", hash)
    }
}

impl Default for EmailBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Encode bytes as base64 with 76-character line wrapping (RFC 2045).
pub fn encode_base64(data: &[u8]) -> Vec<u8> {
    edgerun_encoding::base64::standard_encode_wrapped(data)
}

/// Encode a string as quoted-printable (RFC 2045).
pub fn encode_quoted_printable(s: &str) -> Vec<u8> {
    edgerun_encoding::quoted_printable::encode_quoted_printable(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_text_message() {
        let msg = EmailBuilder::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test")
            .date("Thu, 07 May 2026 00:00:00 +0000")
            .message_id("<test@edgerun.mail>")
            .body("Hello, World!")
            .build();
        let text = String::from_utf8_lossy(&msg);
        assert!(text.contains("From: sender@example.com"));
        assert!(text.contains("To: recipient@example.com"));
        assert!(text.contains("Subject: Test"));
        assert!(text.contains("Date: Thu, 07 May 2026 00:00:00 +0000"));
        assert!(text.contains("Message-ID: <test@edgerun.mail>"));
        assert!(text.contains("Hello, World!"));
    }

    #[test]
    fn multipart_alternative() {
        let msg = EmailBuilder::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test")
            .part(MimePart::text("Plain text"))
            .part(MimePart::html("<p>HTML</p>"))
            .build();
        let text = String::from_utf8_lossy(&msg);
        assert_eq!(text, String::from_utf8_lossy(&msg));
        assert!(text.contains("multipart/alternative"));
        assert!(text.contains("Plain text"));
        assert!(text.contains("text/html"));
    }

    #[test]
    fn multipart_mixed_with_attachment() {
        let msg = EmailBuilder::new()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("With attachment")
            .part(MimePart::text("See attached."))
            .part(MimePart::attachment(
                "report.pdf",
                "application/pdf",
                b"%PDF-1.4",
            ))
            .build();
        let text = String::from_utf8_lossy(&msg);
        assert!(text.contains("multipart/mixed"));
        assert!(text.contains("report.pdf"));
        assert!(text.contains("Content-Transfer-Encoding: base64"));
        assert!(text.contains("Content-Disposition: attachment"));
    }

    #[test]
    fn encodings() {
        let encoded = encode_base64(b"Hello");
        let text = String::from_utf8_lossy(&encoded);
        assert!(text.contains("SGVsbG8="));

        let encoded = encode_quoted_printable("Hello, =world=");
        let text = String::from_utf8_lossy(&encoded);
        assert!(text.contains("=3D"));
    }
}
