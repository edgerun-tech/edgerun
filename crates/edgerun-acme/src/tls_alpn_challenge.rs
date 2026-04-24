use edgerun_crypto::{Sha256, Digest};

use edgerun_encoding::base64url_nopad_encode;

#[derive(Clone)]
#[allow(dead_code)]
pub struct TlsAlpnChallenge {
    domain: String,
    token: String,
    thumbprint: String,
    challenge_value: String,
}

impl TlsAlpnChallenge {
    pub fn new(domain: &str, token: &str, thumbprint: &str) -> Self {
        let key_authorization = format!("{}.{}", token, thumbprint);
        
        let mut hasher = Sha256::new();
        hasher.update(key_authorization.as_bytes());
        let digest = hasher.finalize();
        
        Self {
            domain: domain.to_string(),
            token: token.to_string(),
            thumbprint: thumbprint.to_string(),
            challenge_value: base64url_nopad_encode(&digest),
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
    active_challenge: std::sync::RwLock<Option<TlsAlpnChallenge>>,
}

impl TlsAlpnManager {
    pub fn new(thumbprint: String) -> Self {
        Self {
            thumbprint,
            active_challenge: std::sync::RwLock::new(None),
        }
    }

    pub fn set_challenge(&self, domain: &str, token: &str) -> TlsAlpnChallenge {
        let challenge = TlsAlpnChallenge::new(domain, token, &self.thumbprint);
        *self.active_challenge.write().unwrap() = Some(challenge.clone());
        challenge
    }

    pub fn clear_challenge(&self) {
        *self.active_challenge.write().unwrap() = None;
    }

    pub fn get_challenge_value(&self) -> Option<String> {
        self.active_challenge.read().unwrap().as_ref().map(|c| c.challenge_value().to_string())
    }
}
