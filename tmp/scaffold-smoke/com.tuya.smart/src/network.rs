pub const ALLOW_DOMAINS: &[&str] = &[
    "accounts.google.com",
    "admob-gmats.uc.r.appspot.com",
    "android.asset",
];

pub fn is_allowed_domain(domain: &str) -> bool {
    ALLOW_DOMAINS.iter().any(|allowed| *allowed == domain)
}
