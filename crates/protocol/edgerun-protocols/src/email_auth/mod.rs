//! Email authentication protocol logic.
//!
//! This module owns SPF, DKIM, and DMARC parsing, canonicalization, signing,
//! and verification helpers. It does not own DNS transports, sockets, policy
//! decisions, queues, or message storage.

pub mod dkim;
pub mod dmarc;
pub mod sign;
pub mod spf;
pub mod std;

pub use dkim::{DkimResult, DkimSignature, DkimStatus};
pub use dmarc::{DmarcPolicy, DmarcResult, DmarcStatus};
pub use sign::{DkimKeyStore, DkimSigner};
pub use spf::SpfResult;

use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec;
use alloc::vec::Vec;
use std::io;

/// DNS queries needed by SPF, DKIM, and DMARC evaluation.
///
/// The protocol crate defines the required API but does not decide how those
/// TXT lookups are transported or cached.
pub trait DnsQuery {
    fn query_txt(
        &mut self,
        name: &str,
    ) -> impl std::future::Future<Output = io::Result<Vec<String>>> + Send;
}

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

        parts.push(format!("spf={}", self.spf.as_auth_result()));

        for (i, dkim) in self.dkim.iter().enumerate() {
            let dkim_str = dkim.as_auth_result();
            let domain = dkim.domain.as_deref().unwrap_or("unknown");
            parts.push(format!("dkim={}; header.d={}", dkim_str, domain));
            if i == 0 {
                if let Some(ref sel) = dkim.selector {
                    parts
                        .last_mut()
                        .expect("dkim result was just pushed")
                        .push_str(&format!("; header.s={}", sel));
                }
            }
        }

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

/// Evaluates SPF, DKIM, and DMARC for a message using caller-provided DNS.
pub struct EmailAuthEvaluator<'a, D> {
    dns: &'a mut D,
}

impl<'a, D: DnsQuery> EmailAuthEvaluator<'a, D> {
    pub fn new(dns: &'a mut D) -> Self {
        Self { dns }
    }

    /// Evaluate all authentication mechanisms for a message.
    pub async fn evaluate(
        &mut self,
        client_ip: &str,
        envelope_from: &str,
        header_from: &str,
        headers: &[u8],
        body: &[u8],
    ) -> io::Result<AuthenticationResults> {
        let spf = self.eval_spf(client_ip, envelope_from).await;
        let dkim = self.eval_dkim(headers, body).await;
        let dmarc = self.eval_dmarc(header_from, &spf, &dkim).await;

        Ok(AuthenticationResults::new(spf, dkim, dmarc))
    }

    async fn eval_spf(&mut self, client_ip: &str, envelope_from: &str) -> SpfResult {
        let domain = match Self::extract_domain(envelope_from) {
            Some(d) => d,
            None => return SpfResult::None,
        };

        match spf::check_spf(self.dns, client_ip, &domain).await {
            Ok(result) => result,
            Err(_) => SpfResult::TempError,
        }
    }

    async fn eval_dkim(&mut self, headers: &[u8], body: &[u8]) -> Vec<DkimResult> {
        let signatures = match dkim::parse_dkim_signatures(headers) {
            Ok(sigs) => sigs,
            Err(_) => return vec![DkimResult::perm_error("parse error")],
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
            Err(_) => Some(DmarcResult::none("evaluation error")),
        }
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
}
