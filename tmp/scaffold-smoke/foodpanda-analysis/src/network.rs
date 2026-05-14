pub const ALLOW_DOMAINS: &[&str] = &[
    "accounts.google.com",
    "aggregator.eu.usercentrics.eu",
    "aggregator.service.usercentrics.eu",
    "aomedia.org",
    "api.avo.app",
    "api.onfido.com",
    "api.shakebugs.com",
    "api.usercentrics.eu",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
