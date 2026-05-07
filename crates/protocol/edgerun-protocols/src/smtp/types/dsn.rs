//! DSN (Delivery Status Notification) types per RFC 3461.

use alloc::string::{String, ToString};

/// DSN RET parameter — what to return on delivery failure.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DsnRet {
    /// Return full message on failure.
    Full,
    /// Return only headers on failure.
    #[default]
    Headers,
}

impl DsnRet {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "FULL" => Some(Self::Full),
            "HDRS" => Some(Self::Headers),
            _ => None,
        }
    }
}

/// DSN NOTIFY parameter — when to send notifications.
#[derive(Debug, Clone, Default)]
pub struct DsnNotify {
    pub never: bool,
    pub success: bool,
    pub failure: bool,
    pub delay: bool,
}

impl DsnNotify {
    pub fn parse(s: &str) -> Self {
        let mut notify = Self::default();
        for token in s.split(',') {
            match token.to_uppercase().as_str() {
                "NEVER" => notify.never = true,
                "SUCCESS" => notify.success = true,
                "FAILURE" => notify.failure = true,
                "DELAY" => notify.delay = true,
                _ => {}
            }
        }
        notify
    }

    /// Returns true if never is set (overrides all others per RFC 3461).
    pub fn is_never(&self) -> bool {
        self.never
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dsn_ret_parse() {
        assert_eq!(DsnRet::parse("FULL"), Some(DsnRet::Full));
        assert_eq!(DsnRet::parse("HDRS"), Some(DsnRet::Headers));
        assert_eq!(DsnRet::parse("full"), Some(DsnRet::Full));
        assert_eq!(DsnRet::parse("INVALID"), None);
    }

    #[test]
    fn test_dsn_notify_parse() {
        let n = DsnNotify::parse("SUCCESS,FAILURE");
        assert!(n.success);
        assert!(n.failure);
        assert!(!n.never);
    }

    #[test]
    fn test_dsn_notify_never_overrides() {
        let n = DsnNotify::parse("NEVER,SUCCESS");
        assert!(n.never);
        assert!(n.is_never());
    }

    #[test]
    fn test_dsn_notify_default() {
        let n = DsnNotify::default();
        assert!(!n.never);
        assert!(!n.success);
        assert!(!n.failure);
    }
}
