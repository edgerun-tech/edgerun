use crate::prelude::*;

use super::challenge_material::tls_alpn_01_challenge_value;

#[derive(Clone)]
pub struct TlsAlpn01Challenge {
    domain: String,
    challenge_value: String,
}

impl TlsAlpn01Challenge {
    pub fn new(domain: &str, token: &str, thumbprint: &str) -> Self {
        Self {
            domain: domain.to_string(),
            challenge_value: tls_alpn_01_challenge_value(token, thumbprint),
        }
    }

    pub fn challenge_value(&self) -> &str {
        &self.challenge_value
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acme::challenge_material::key_authorization_digest;

    #[test]
    fn tls_alpn_01_value_is_key_authorization_digest() {
        let challenge = TlsAlpn01Challenge::new("example.com", "token", "thumbprint");

        assert_eq!(challenge.domain(), "example.com");
        assert_eq!(
            challenge.challenge_value(),
            key_authorization_digest("token", "thumbprint")
        );
    }
}
