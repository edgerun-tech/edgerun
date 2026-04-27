use url::Url;

use crate::types::{ChallengeStatus, ChallengeType};

#[derive(Debug, Clone)]
pub struct Challenge {
    inner: crate::types::Challenge,
}

impl Challenge {
    pub fn from_acme(challenge: crate::types::Challenge) -> Self {
        Self { inner: challenge }
    }

    pub fn challenge_type(&self) -> ChallengeType {
        self.inner.challenge_type
    }

    pub fn status(&self) -> ChallengeStatus {
        self.inner.status
    }

    pub fn token(&self) -> Option<&str> {
        self.inner.token.as_deref()
    }

    pub fn url(&self) -> &Url {
        &self.inner.url
    }

    pub fn is_pending(&self) -> bool {
        self.inner.status == ChallengeStatus::Pending
    }

    pub fn is_valid(&self) -> bool {
        self.inner.status == ChallengeStatus::Valid
    }

    pub fn is_processing(&self) -> bool {
        self.inner.status == ChallengeStatus::Processing
    }

    pub fn key_authorization(&self, thumbprint: &str) -> String {
        if let Some(token) = &self.inner.token {
            format!("{}.{}", token, thumbprint)
        } else {
            thumbprint.to_string()
        }
    }
}
