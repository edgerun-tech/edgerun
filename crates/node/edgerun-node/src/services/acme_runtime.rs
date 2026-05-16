//! Node-owned ACME orchestration surface.
//!
//! ACME is not just a protocol parser: it mutates DNS/HTTP/TLS challenge
//! surfaces, persists account and certificate material, schedules renewals, and
//! uses node-owned outbound transport. Protocol helpers live in
//! `edgerun-protocols`; this module is the node boundary that decides how those
//! helpers affect runtime resources.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use edgerun_protocols::acme::dns01::Dns01Challenge;
use edgerun_protocols::acme::http01::Http01Challenge;
use edgerun_protocols::acme::tls_alpn01::TlsAlpn01Challenge;
use edgerun_protocols::tls::ACME_TLS_ALPN_PROTOCOL;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcmeDns01Plan {
    pub domain: String,
    pub record_name: String,
    pub record_value: String,
    pub ttl_secs: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcmeHttp01Plan {
    pub domain: String,
    pub route_path: String,
    pub response_body: String,
    pub content_type: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AcmeTlsAlpn01Plan {
    pub domain: String,
    pub alpn_protocol: Vec<u8>,
    pub challenge_value: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AcmeChallengePlan {
    Dns01(AcmeDns01Plan),
    Http01(AcmeHttp01Plan),
    TlsAlpn01(AcmeTlsAlpn01Plan),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeAcmeOrchestrator {
    ttl_secs: u32,
}

impl Default for NodeAcmeOrchestrator {
    fn default() -> Self {
        Self { ttl_secs: 60 }
    }
}

impl NodeAcmeOrchestrator {
    pub const fn new(ttl_secs: u32) -> Self {
        Self { ttl_secs }
    }

    pub fn dns01_plan(&self, domain: &str, token: &str, account_thumbprint: &str) -> AcmeDns01Plan {
        let challenge = Dns01Challenge::new(domain, token, account_thumbprint);
        AcmeDns01Plan {
            domain: challenge.domain().to_string(),
            record_name: challenge.record_name().to_string(),
            record_value: challenge.record_value().to_string(),
            ttl_secs: self.ttl_secs,
        }
    }

    pub fn dns01_plans(
        &self,
        domains: &[String],
        token: &str,
        account_thumbprint: &str,
    ) -> Vec<AcmeDns01Plan> {
        domains
            .iter()
            .map(|domain| self.dns01_plan(domain, token, account_thumbprint))
            .collect()
    }

    pub fn http01_plan(
        &self,
        domain: &str,
        token: &str,
        account_thumbprint: &str,
    ) -> AcmeHttp01Plan {
        let challenge = Http01Challenge::from_thumbprint(token, account_thumbprint);
        AcmeHttp01Plan {
            domain: domain.to_string(),
            route_path: challenge.path(),
            response_body: challenge.key_authorization,
            content_type: "text/plain",
        }
    }

    pub fn http01_plans(
        &self,
        domains: &[String],
        token: &str,
        account_thumbprint: &str,
    ) -> Vec<AcmeHttp01Plan> {
        domains
            .iter()
            .map(|domain| self.http01_plan(domain, token, account_thumbprint))
            .collect()
    }

    pub fn tls_alpn01_plan(
        &self,
        domain: &str,
        token: &str,
        account_thumbprint: &str,
    ) -> AcmeTlsAlpn01Plan {
        let challenge = TlsAlpn01Challenge::new(domain, token, account_thumbprint);
        AcmeTlsAlpn01Plan {
            domain: challenge.domain().to_string(),
            alpn_protocol: ACME_TLS_ALPN_PROTOCOL.to_vec(),
            challenge_value: challenge.challenge_value().to_string(),
        }
    }

    pub fn tls_alpn01_plans(
        &self,
        domains: &[String],
        token: &str,
        account_thumbprint: &str,
    ) -> Vec<AcmeTlsAlpn01Plan> {
        domains
            .iter()
            .map(|domain| self.tls_alpn01_plan(domain, token, account_thumbprint))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_plans_all_acme_challenge_surfaces() {
        let orchestrator = NodeAcmeOrchestrator::new(30);

        let dns = orchestrator.dns01_plan("example.com", "token", "thumbprint");
        assert_eq!(dns.domain, "example.com");
        assert_eq!(dns.record_name, "_acme-challenge.example.com");
        assert!(!dns.record_value.is_empty());
        assert_eq!(dns.ttl_secs, 30);

        let http = orchestrator.http01_plan("example.com", "token", "thumbprint");
        assert_eq!(http.domain, "example.com");
        assert_eq!(http.route_path, "/.well-known/acme-challenge/token");
        assert_eq!(http.response_body, "token.thumbprint");
        assert_eq!(http.content_type, "text/plain");

        let tls = orchestrator.tls_alpn01_plan("example.com", "token", "thumbprint");
        assert_eq!(tls.domain, "example.com");
        assert_eq!(tls.alpn_protocol, b"acme-tls/1");
        assert!(!tls.challenge_value.is_empty());
    }
}
