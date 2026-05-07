//! HTTP authentication header helpers.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Parse a `WWW-Authenticate: Bearer ...` challenge.
///
/// Returns `(realm, service, scope)` for compatibility with OCI registry
/// token exchange callers. Unknown auth parameters are ignored.
pub fn parse_bearer_auth(header: &str) -> Option<(String, String, Option<String>)> {
    if !header.starts_with("Bearer ") && !header.starts_with("bearer ") {
        return None;
    }

    let params = &header[7..];
    let mut realm = None;
    let mut service = None;
    let mut scope = None;

    for part in split_auth_params(params) {
        let part = part.trim();
        if let Some((key, val)) = part.split_once('=') {
            let key = key.trim();
            let val = unquote_auth_value(val.trim());
            match key {
                "realm" => realm = Some(val),
                "service" => service = Some(val),
                "scope" => scope = Some(val),
                _ => {}
            }
        }
    }

    Some((realm?, service.unwrap_or_else(|| "registry".into()), scope))
}

fn split_auth_params(params: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut in_quote = false;
    let mut escaped = false;

    for (index, ch) in params.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match ch {
            '\\' if in_quote => escaped = true,
            '"' => in_quote = !in_quote,
            ',' if !in_quote => {
                parts.push(&params[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    parts.push(&params[start..]);
    parts
}

fn unquote_auth_value(value: &str) -> String {
    let Some(quoted) = value.strip_prefix('"').and_then(|s| s.strip_suffix('"')) else {
        return value.to_string();
    };

    let mut unquoted = String::new();
    let mut chars = quoted.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            if let Some(escaped) = chars.next() {
                unquoted.push(escaped);
            }
        } else {
            unquoted.push(ch);
        }
    }
    unquoted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bearer_auth_full() {
        let header = "Bearer realm=\"https://auth.docker.io/token\",service=\"registry.docker.io\",scope=\"repository:library/alpine:pull\"";
        let (realm, service, scope) = parse_bearer_auth(header).unwrap();
        assert_eq!(realm, "https://auth.docker.io/token");
        assert_eq!(service, "registry.docker.io");
        assert_eq!(scope, Some("repository:library/alpine:pull".into()));
    }

    #[test]
    fn parse_bearer_auth_no_scope() {
        let header = "Bearer realm=\"https://auth.example.com/token\",service=\"registry\"";
        let (realm, service, scope) = parse_bearer_auth(header).unwrap();
        assert_eq!(realm, "https://auth.example.com/token");
        assert_eq!(service, "registry");
        assert!(scope.is_none());
    }

    #[test]
    fn parse_bearer_auth_handles_quoted_commas_and_spaced_equals() {
        let header = "Bearer realm = \"https://auth.example.com/token?a=b,c=d\", service = \"registry.example\", scope = \"repository:owner/repo:pull,push\"";

        let (realm, service, scope) = parse_bearer_auth(header).unwrap();

        assert_eq!(realm, "https://auth.example.com/token?a=b,c=d");
        assert_eq!(service, "registry.example");
        assert_eq!(scope, Some("repository:owner/repo:pull,push".into()));
    }

    #[test]
    fn parse_bearer_auth_unescapes_quoted_values() {
        let header =
            "Bearer realm=\"https://auth.example.com/token\",service=\"registry\\\"quoted\"";

        let (_realm, service, _scope) = parse_bearer_auth(header).unwrap();

        assert_eq!(service, "registry\"quoted");
    }

    #[test]
    fn parse_bearer_auth_invalid_prefix() {
        assert!(parse_bearer_auth("Basic abc").is_none());
        assert!(parse_bearer_auth("").is_none());
    }
}
