//! RFC 5322 email message builder.

/// Helper for building RFC 5322 email messages.
pub struct EmailBuilder {
    headers: Vec<(String, String)>,
    body: String,
}

impl EmailBuilder {
    pub fn new() -> Self {
        Self {
            headers: Vec::new(),
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

    /// Set the Bcc header (not sent to server, but embedded in message).
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

    /// Set the Message-ID header.
    pub fn message_id(mut self, id: &str) -> Self {
        self.headers.push(("Message-ID".to_string(), id.to_string()));
        self
    }

    /// Set Content-Type header.
    pub fn content_type(mut self, mime_type: &str) -> Self {
        self.headers.push(("Content-Type".to_string(), mime_type.to_string()));
        self
    }

    /// Add a custom header.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    /// Set the message body.
    pub fn body(mut self, body: &str) -> Self {
        self.body = body.to_string();
        self
    }

    /// Build the complete RFC 5322 message.
    pub fn build(self) -> Vec<u8> {
        let mut message = String::new();

        for (name, value) in &self.headers {
            message.push_str(&format!("{}: {}\r\n", name, value));
        }

        // Auto-generate Date if absent
        if !self.headers.iter().any(|(n, _)| n == "Date") {
            message.push_str(&format!("Date: {}\r\n", generate_rfc2822_date()));
        }

        // Blank line separates headers from body
        message.push_str("\r\n");
        message.push_str(&self.body);

        // Ensure trailing CRLF
        if !message.ends_with("\r\n") {
            message.push_str("\r\n");
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
// Date generation
// ===========================================================================

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
