pub const ALLOW_DOMAINS: &[&str] = &[
    "account-inn-test.tcljd.com",
    "account-rus-test.tcljd.com",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
