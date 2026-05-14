pub const ALLOW_DOMAINS: &[&str] = &[
    "android.com",
    "bugs.chromium.org",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
