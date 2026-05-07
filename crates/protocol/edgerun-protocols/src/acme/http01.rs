use crate::prelude::*;

use super::challenge_material::{
    http_01_challenge_path, key_authorization, key_authorization_digest,
};

#[derive(Clone)]
pub struct Http01Challenge {
    pub token: String,
    pub key_authorization: String,
}

impl Http01Challenge {
    pub fn new(token: &str, key_authorization: &str) -> Self {
        Self {
            token: token.to_string(),
            key_authorization: key_authorization.to_string(),
        }
    }

    pub fn from_thumbprint(token: &str, thumbprint: &str) -> Self {
        Self::new(token, &key_authorization(token, thumbprint))
    }

    pub fn compute_key_authorization(token: &str, thumbprint: &str) -> String {
        key_authorization(token, thumbprint)
    }

    pub fn compute_digest(token: &str, thumbprint: &str) -> String {
        key_authorization_digest(token, thumbprint)
    }

    pub fn path(&self) -> String {
        http_01_challenge_path(&self.token)
    }

    pub fn body_for_path(&self, path: &str) -> Option<&str> {
        (path == self.path()).then_some(self.key_authorization.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_01_challenge_matches_only_its_well_known_path() {
        let challenge = Http01Challenge::new("token-123", "token-123.thumbprint");

        assert_eq!(challenge.path(), "/.well-known/acme-challenge/token-123");
        assert_eq!(
            challenge.body_for_path("/.well-known/acme-challenge/token-123"),
            Some("token-123.thumbprint")
        );
        assert_eq!(challenge.body_for_path("/other"), None);
    }
}
