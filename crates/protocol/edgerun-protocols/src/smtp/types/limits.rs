/// Configurable limits for the SMTP server.
#[derive(Debug, Clone)]
pub struct ServerLimits {
    /// Maximum message size in bytes (0 = unlimited).
    pub max_message_size: usize,
    /// Maximum number of recipients per mail transaction.
    pub max_recipients: usize,
    /// Maximum line length (RFC 5322 §2.1.1: 998 recommended).
    pub max_line_length: usize,
    /// Idle timeout (seconds). Connection is closed if no data received.
    pub idle_timeout_secs: u64,
    /// Maximum commands per connection before forced close.
    pub max_commands: usize,
}

impl Default for ServerLimits {
    fn default() -> Self {
        Self {
            max_message_size: 35_882_577,
            max_recipients: 100,
            max_line_length: 998,
            idle_timeout_secs: 300,
            max_commands: 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_defaults() {
        let limits = ServerLimits::default();
        assert_eq!(limits.max_line_length, 998);
        assert_eq!(limits.max_recipients, 100);
        assert_eq!(limits.idle_timeout_secs, 300);
        assert_eq!(limits.max_commands, 1000);
    }
}
