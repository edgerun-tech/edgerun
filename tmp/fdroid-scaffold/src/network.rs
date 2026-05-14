pub const ALLOW_DOMAINS: &[&str] = &[
    "4everland.io",
    "apt.izzysoft.de",
    "archive.newpipe.net",
    "briarproject.org",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
