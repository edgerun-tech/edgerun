pub const ALLOW_DOMAINS: &[&str] = &[
    "account.firstvet.com",
    "accounts.google.com",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
