use crate::prelude::v1::*;

use edgerun_encoding::base64url_nopad_encode;

pub const HTTP_01_CHALLENGE_PREFIX: &str = "/.well-known/acme-challenge/";
pub const DNS_01_RECORD_PREFIX: &str = "_acme-challenge.";

pub fn key_authorization(token: &str, thumbprint: &str) -> String {
    format!("{}.{}", token, thumbprint)
}

pub fn key_authorization_digest(token: &str, thumbprint: &str) -> String {
    let key_authorization = key_authorization(token, thumbprint);
    base64url_nopad_encode(&edgerun_crypto::sha256(key_authorization.as_bytes()))
}

pub fn http_01_challenge_path(token: &str) -> String {
    format!("{}{}", HTTP_01_CHALLENGE_PREFIX, token)
}

pub fn dns_01_record_name(domain: &str) -> String {
    format!("{}{}", DNS_01_RECORD_PREFIX, domain)
}

pub fn dns_01_record_value(token: &str, thumbprint: &str) -> String {
    key_authorization_digest(token, thumbprint)
}

pub fn tls_alpn_01_challenge_value(token: &str, thumbprint: &str) -> String {
    key_authorization_digest(token, thumbprint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_material_is_shared_across_challenge_types() {
        let digest = key_authorization_digest("token", "thumbprint");
        assert_eq!(dns_01_record_value("token", "thumbprint"), digest);
        assert_eq!(tls_alpn_01_challenge_value("token", "thumbprint"), digest);
        assert_eq!(key_authorization("token", "thumbprint"), "token.thumbprint");
        assert_eq!(
            http_01_challenge_path("token"),
            "/.well-known/acme-challenge/token"
        );
        assert_eq!(
            dns_01_record_name("example.com"),
            "_acme-challenge.example.com"
        );
    }
}
