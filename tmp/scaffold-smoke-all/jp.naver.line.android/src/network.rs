pub const ALLOW_DOMAINS: &[&str] = &[
    "1176-ti.cloud.v-key.com",
    "1176-tla.cloud.v-key.com",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
