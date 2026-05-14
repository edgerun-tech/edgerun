pub const ALLOW_DOMAINS: &[&str] = &[
    "android.com",
    "android.googleapis.com",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
