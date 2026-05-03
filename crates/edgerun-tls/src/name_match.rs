use alloc::string::{String, ToString};

pub fn normalize_tls_dns_name(name: &str) -> Option<String> {
    let name = name.trim().trim_end_matches('.');
    if name.is_empty() || name.len() > 253 {
        return None;
    }
    if name.bytes().any(|b| b == 0 || b.is_ascii_whitespace()) {
        return None;
    }
    if is_ip_literal_like(name) {
        return None;
    }
    let lower = name.to_ascii_lowercase();
    let mut labels = 0usize;
    for label in lower.split('.') {
        labels += 1;
        if !is_dns_label(label) {
            return None;
        }
    }
    if labels < 2 {
        return None;
    }
    Some(lower)
}

pub fn tls_dns_name_matches(pattern: &str, host: &str) -> bool {
    let Some(host) = normalize_tls_dns_name(host) else {
        return false;
    };
    let pattern = pattern.trim().trim_end_matches('.').to_ascii_lowercase();
    if pattern == host {
        return true;
    }
    let bytes = pattern.as_bytes();
    if bytes.len() < 3 || bytes[0] != 42 || bytes[1] != b'.' {
        return false;
    }
    let suffix = &pattern[2..];
    if suffix.as_bytes().contains(&42) || suffix == host || !suffix.contains('.') {
        return false;
    }
    let Some(prefix) = host.strip_suffix(suffix).and_then(|s| s.strip_suffix('.')) else {
        return false;
    };
    !prefix.is_empty() && !prefix.contains('.') && is_dns_label(prefix)
}

fn is_dns_label(label: &str) -> bool {
    if label.is_empty() || label.len() > 63 {
        return false;
    }
    if label.starts_with('-') || label.ends_with('-') {
        return false;
    }
    label.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn is_ip_literal_like(name: &str) -> bool {
    name.bytes().all(|b| b.is_ascii_digit() || b == b'.') || name.contains(':')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_dns_names_for_tls() {
        assert_eq!(normalize_tls_dns_name("Example.COM."), Some("example.com".to_string()));
        assert_eq!(normalize_tls_dns_name("localhost"), None);
        assert_eq!(normalize_tls_dns_name("127.0.0.1"), None);
        assert_eq!(normalize_tls_dns_name("bad label.example"), None);
        assert_eq!(normalize_tls_dns_name("-bad.example"), None);
        assert_eq!(normalize_tls_dns_name("bad-.example"), None);
    }
}
