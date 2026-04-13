//! Email authentication: SPF, DKIM, and DMARC evaluation.
//!
//! # Overview
//! - **SPF** (RFC 7208): Validates that the sending IP is authorized for the
//!   envelope sender domain. Queries TXT records at the domain.
//! - **DKIM** (RFC 6376): Cryptographic signature verification. Queries TXT
//!   records at `selector._domainkey.domain` for the public key, then verifies
//!   the signature over canonicalized headers and body.
//! - **DMARC** (RFC 7489): Policy evaluation based on SPF and DKIM results.
//!   Queries TXT at `_dmarc.domain` for policy, checks identifier alignment.
//!
//! # Usage
//! ```ignore
//! use edgerun_email_auth::EmailAuthEvaluator;
//! use edgerun_dns::client::DnsClient;
//!
//! let mut dns = DnsClient::new("8.8.8.8:53")?;
//! let mut evaluator = EmailAuthEvaluator::new(&mut dns);
//! let results = evaluator
//!     .evaluate("192.168.1.1", &envelope_from, &headers, &body)
//!     .await?;
//! let auth_results = results.to_header_value("myserver.example.com");
//! // Add `Authentication-Results: auth_results` to the message
//! ```

pub mod spf;
pub mod dkim;
pub mod dmarc;

pub use dkim::{DkimResult, DkimSignature, DkimStatus};
pub use dmarc::{DmarcPolicy, DmarcResult, DmarcStatus};
pub use spf::SpfResult;

use std::io;

/// Authentication results for a single message.
#[derive(Debug, Clone)]
pub struct AuthenticationResults {
    pub spf: SpfResult,
    pub dkim: Vec<DkimResult>,
    pub dmarc: Option<DmarcResult>,
}

impl AuthenticationResults {
    pub fn new(spf: SpfResult, dkim: Vec<DkimResult>, dmarc: Option<DmarcResult>) -> Self {
        Self { spf, dkim, dmarc }
    }

    /// Format as an `Authentication-Results` header value (RFC 8601).
    pub fn to_header_value(&self, authserv_id: &str) -> String {
        let mut parts = Vec::new();

        // SPF
        let spf_str = self.spf.as_auth_result();
        parts.push(format!("spf={}", spf_str));

        // DKIM (one entry per signature)
        for (i, dkim) in self.dkim.iter().enumerate() {
            let dkim_str = dkim.as_auth_result();
            let domain = dkim.domain.as_deref().unwrap_or("unknown");
            parts.push(format!("dkim={}; header.d={}", dkim_str, domain));
            if i == 0 {
                // Only show the selector for the first one to keep it readable
                if let Some(ref sel) = dkim.selector {
                    parts.last_mut().unwrap().push_str(&format!("; header.s={}", sel));
                }
            }
        }

        // DMARC
        if let Some(ref dmarc) = self.dmarc {
            let dmarc_str = dmarc.as_auth_result();
            let domain = dmarc.domain.as_deref().unwrap_or("unknown");
            parts.push(format!("dmarc={}; header.from={}", dmarc_str, domain));
        }

        format!("{}; {}", authserv_id, parts.join("; "))
    }

    /// Whether the message passed all applicable authentication checks.
    pub fn is_pass(&self) -> bool {
        let spf_pass = matches!(self.spf, SpfResult::Pass);
        let dmarc_pass = self
            .dmarc
            .as_ref()
            .map(|d| matches!(d.status, DmarcStatus::Pass))
            .unwrap_or(false);
        spf_pass || dmarc_pass
    }
}

/// Evaluates SPF, DKIM, and DMARC for a message.
pub struct EmailAuthEvaluator<'a, D> {
    dns: &'a mut D,
}

impl<'a, D: DnsQuery> EmailAuthEvaluator<'a, D> {
    pub fn new(dns: &'a mut D) -> Self {
        Self { dns }
    }

    /// Evaluate all authentication mechanisms for a message.
    ///
    /// - `client_ip`: The IP address of the connecting client.
    /// - `envelope_from`: The MAIL FROM address (for SPF).
    /// - `header_from`: The From: header address (for DMARC alignment).
    /// - `headers`: Raw message headers (for DKIM).
    /// - `body`: Raw message body (for DKIM).
    pub async fn evaluate(
        &mut self,
        client_ip: &str,
        envelope_from: &str,
        header_from: &str,
        headers: &[u8],
        body: &[u8],
    ) -> io::Result<AuthenticationResults> {
        // Evaluate SPF
        let spf = self.eval_spf(client_ip, envelope_from).await;

        // Evaluate DKIM
        let dkim = self.eval_dkim(headers, body).await;

        // Evaluate DMARC (depends on SPF + DKIM + header_from)
        let dmarc = self.eval_dmarc(header_from, &spf, &dkim).await;

        Ok(AuthenticationResults::new(spf, dkim, dmarc))
    }

    async fn eval_spf(&mut self, client_ip: &str, envelope_from: &str) -> SpfResult {
        // Extract domain from envelope_from
        let domain = match Self::extract_domain(envelope_from) {
            Some(d) => d,
            None => return SpfResult::None,
        };

        match spf::check_spf(self.dns, client_ip, &domain).await {
            Ok(result) => result,
            Err(e) => {
                edgerun_log::warn!("edgerun-email-auth: SPF check failed: {}", e);
                SpfResult::TempError
            }
        }
    }

    async fn eval_dkim(&mut self, headers: &[u8], body: &[u8]) -> Vec<DkimResult> {
        // Find all DKIM-Signature headers
        let signatures = match dkim::parse_dkim_signatures(headers) {
            Ok(sigs) => sigs,
            Err(e) => {
                edgerun_log::warn!("edgerun-email-auth: DKIM parse error: {}", e);
                return vec![DkimResult::perm_error("parse error")];
            }
        };

        if signatures.is_empty() {
            return vec![];
        }

        let mut results = Vec::with_capacity(signatures.len());
        for sig in signatures {
            match dkim::verify_signature(self.dns, &sig, headers, body).await {
                Ok(()) => results.push(DkimResult::pass(&sig)),
                Err(e) => results.push(DkimResult::fail(sig, &e.to_string())),
            }
        }

        results
    }

    async fn eval_dmarc(
        &mut self,
        header_from: &str,
        spf: &SpfResult,
        dkim: &[DkimResult],
    ) -> Option<DmarcResult> {
        let domain = match Self::extract_domain(header_from) {
            Some(d) => d,
            None => return None,
        };

        match dmarc::evaluate_dmarc(self.dns, &domain, header_from, spf, dkim).await {
            Ok(result) => Some(result),
            Err(e) => {
                edgerun_log::warn!("edgerun-email-auth: DMARC evaluation failed: {}", e);
                Some(DmarcResult::none("evaluation error"))
            }
        }
    }

    fn extract_domain(email: &str) -> Option<String> {
        let email = email.trim();
        // Handle "<user@domain>" or "user@domain"
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
}

/// Trait for DNS queries needed by the evaluator.
pub trait DnsQuery {
    fn query_txt(&mut self, name: &str) -> impl std::future::Future<Output = io::Result<Vec<String>>> + Send;
}

impl DnsQuery for edgerun_dns::client::DnsClient {
    async fn query_txt(&mut self, name: &str) -> io::Result<Vec<String>> {
        self.query_txt(name).await
    }
}
