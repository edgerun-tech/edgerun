use alloc::string::String;
use alloc::vec::Vec;

use crate::smtp::types::dsn::{DsnNotify, DsnRet};

/// SMTP mail envelope (sender + recipients + data).
#[derive(Debug, Clone)]
pub struct MailEnvelope {
    /// Reverse-path (sender).
    pub from: String,
    /// Forward-paths (recipients).
    pub recipients: Vec<String>,
    /// Raw message data (headers + body).
    pub data: Vec<u8>,
    /// ESMTP parameters from MAIL FROM.
    pub from_parameters: Vec<(String, Option<String>)>,
    /// ESMTP parameters from RCPT TO (per recipient).
    pub recipient_parameters: Vec<Vec<(String, Option<String>)>>,

    // ── DSN fields ────────────────────────────────────────────────────
    /// DSN RET parameter — what to return on failure (from MAIL FROM).
    pub dsn_ret: DsnRet,
    /// DSN ENVID — envelope identifier (from MAIL FROM).
    pub dsn_envid: Option<String>,
    /// DSN NOTIFY per recipient (from RCPT TO).
    pub dsn_notify: Vec<DsnNotify>,
    /// DSN ORCPT per recipient (from RCPT TO).
    pub dsn_orcpt: Vec<Option<String>>,

    // ── Auth fields ───────────────────────────────────────────────────
    /// Authenticated identity (if AUTH was used).
    pub authenticated_identity: Option<String>,
}

impl MailEnvelope {
    pub fn new(from: String) -> Self {
        Self {
            from,
            recipients: Vec::new(),
            data: Vec::new(),
            from_parameters: Vec::new(),
            recipient_parameters: Vec::new(),
            dsn_ret: DsnRet::default(),
            dsn_envid: None,
            dsn_notify: Vec::new(),
            dsn_orcpt: Vec::new(),
            authenticated_identity: None,
        }
    }

    pub fn add_recipient(
        &mut self,
        address: String,
        parameters: Vec<(String, Option<String>)>,
        notify: DsnNotify,
        orcpt: Option<String>,
    ) {
        self.recipients.push(address);
        self.recipient_parameters.push(parameters);
        self.dsn_notify.push(notify);
        self.dsn_orcpt.push(orcpt);
    }

    pub fn recipient_count(&self) -> usize {
        self.recipients.len()
    }

    pub fn reset(&mut self) {
        self.recipients.clear();
        self.recipient_parameters.clear();
        self.data.clear();
        self.dsn_ret = DsnRet::default();
        self.dsn_envid = None;
        self.dsn_notify.clear();
        self.dsn_orcpt.clear();
        // authenticated_identity survives reset — applies to the connection
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;

    #[test]
    fn test_envelope_lifecycle() {
        let mut env = MailEnvelope::new("sender@example.com".to_string());
        assert_eq!(env.recipient_count(), 0);

        env.add_recipient(
            "a@b.com".to_string(),
            vec![("NOTIFY".to_string(), Some("FAILURE".to_string()))],
            DsnNotify {
                failure: true,
                ..Default::default()
            },
            None,
        );
        assert_eq!(env.recipient_count(), 1);
        assert!(env.dsn_notify[0].failure);
    }

    #[test]
    fn test_envelope_reset_preserves_auth() {
        let mut env = MailEnvelope::new("sender@example.com".to_string());
        env.authenticated_identity = Some("user@example.com".to_string());
        env.add_recipient("a@b.com".to_string(), vec![], DsnNotify::default(), None);
        env.reset();
        assert_eq!(env.recipient_count(), 0);
        assert_eq!(
            env.authenticated_identity,
            Some("user@example.com".to_string())
        );
    }
}
