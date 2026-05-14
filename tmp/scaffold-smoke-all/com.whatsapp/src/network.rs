pub const ALLOW_DOMAINS: &[&str] = &[
    "accounts.google.com",
    "acs.whatsapp.com",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
