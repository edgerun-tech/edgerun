pub const ALLOW_DOMAINS: &[&str] = &[
    "accountlinking-pa.clients6.google.com",
    "accountlinking-pa.googleapis.com",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
