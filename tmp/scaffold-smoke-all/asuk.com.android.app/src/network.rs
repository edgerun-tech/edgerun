pub const ALLOW_DOMAINS: &[&str] = &[
    "7eleven-streamer.trueid-preprod.net",
    "7eleven-streamer.trueid.net",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
