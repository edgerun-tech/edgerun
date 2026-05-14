pub const ALLOW_DOMAINS: &[&str] = &[
    "default.url",
    "developer.android.com",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
