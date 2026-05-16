use crate::prelude::*;

use super::challenge_material::{dns_01_record_name, dns_01_record_value};

#[derive(Clone)]
pub struct Dns01Challenge {
    domain: String,
    txt_record_name: String,
    txt_record_value: String,
}

impl Dns01Challenge {
    pub fn new(domain: &str, token: &str, thumbprint: &str) -> Self {
        Self {
            domain: domain.to_string(),
            txt_record_name: dns_01_record_name(domain),
            txt_record_value: dns_01_record_value(token, thumbprint),
        }
    }

    pub fn domain(&self) -> &str {
        &self.domain
    }

    pub fn record_name(&self) -> &str {
        &self.txt_record_name
    }

    pub fn record_value(&self) -> &str {
        &self.txt_record_value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::acme::challenge_material::key_authorization_digest;

    #[test]
    fn dns_01_challenge_has_txt_record_material() {
        let challenge = Dns01Challenge::new("example.com", "token", "thumbprint");

        assert_eq!(challenge.domain(), "example.com");
        assert_eq!(challenge.record_name(), "_acme-challenge.example.com");
        assert_eq!(
            challenge.record_value(),
            key_authorization_digest("token", "thumbprint")
        );
    }
}
