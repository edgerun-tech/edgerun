pub const ALLOW_DOMAINS: &[&str] = &[
    "b-www.facebook.com",
    "fburl.com",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
