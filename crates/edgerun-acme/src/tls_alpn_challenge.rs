use crate::prelude::v1::*;

use edgerun_rt::RwLock;

use crate::challenge_material::tls_alpn_01_challenge_value;

#[derive(Clone)]
pub struct TlsAlpnChallenge {
    domain: String,
    challenge_value: String,
}

impl TlsAlpnChallenge {
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

pub struct TlsAlpnManager {
    thumbprint: String,
    active_challenge: RwLock<Option<TlsAlpnChallenge>>,
}

impl TlsAlpnManager {
    pub fn new(thumbprint: String) -> Self {
        Self {
            thumbprint,
            active_challenge: RwLock::new(None),
        }
    }

    pub fn set_challenge(&self, domain: &str, token: &str) -> TlsAlpnChallenge {
        let challenge = TlsAlpnChallenge::new(domain, token, &self.thumbprint);
        *self.active_challenge.write() = Some(challenge.clone());
        challenge
    }

    pub fn clear_challenge(&self) {
        *self.active_challenge.write() = None;
    }

    pub fn get_challenge_value(&self) -> Option<String> {
        self.active_challenge
            .read()
            .as_ref()
            .map(|c| c.challenge_value().to_string())
    }
}
