use std::sync::Arc;

use edgerun_dns::record::DnsRecordType;
use edgerun_dns::zone::DnsZone;
use edgerun_encoding::base64url_nopad_encode;
use sha2::{Digest, Sha256};

use crate::account::AccountKey;

#[derive(Clone)]
pub struct DnsChallenge {
    domain: String,
    token: String,
    thumbprint: String,
    txt_record_name: String,
    txt_record_value: String,
}

impl DnsChallenge {
    pub fn new(domain: &str, token: &str, account_key: &AccountKey) -> Self {
        let thumbprint = account_key.thumbprint_b64();
        let key_authorization = format!("{}.{}", token, thumbprint);

        let mut hasher = Sha256::new();
        hasher.update(key_authorization.as_bytes());
        let digest = hasher.finalize();
        let txt_value = base64url_nopad_encode(&digest);

        let txt_record_name = format!("_acme-challenge.{}", domain);

        Self {
            domain: domain.to_string(),
            token: token.to_string(),
            thumbprint,
            txt_record_name,
            txt_record_value: txt_value,
        }
    }

    pub fn record_name(&self) -> &str {
        &self.txt_record_name
    }

    pub fn record_value(&self) -> &str {
        &self.txt_record_value
    }

    pub fn add_to_zone(&self, zone: &mut DnsZone) {
        zone.add_txt(&self.txt_record_name, &self.txt_record_value, 60);
    }

    pub fn remove_from_zone(&self, zone: &mut DnsZone) {
        zone.remove_record(&self.txt_record_name, DnsRecordType::TXT);
    }
}

pub struct DnsChallengeManager {
    account_key: Arc<AccountKey>,
    active_challenges: std::sync::RwLock<Vec<DnsChallenge>>,
}

impl DnsChallengeManager {
    pub fn new(account_key: Arc<AccountKey>) -> Self {
        Self {
            account_key,
            active_challenges: std::sync::RwLock::new(Vec::new()),
        }
    }

    pub fn create_challenge(&self, domain: &str, token: &str) -> DnsChallenge {
        let challenge = DnsChallenge::new(domain, token, &self.account_key);

        self.active_challenges
            .write()
            .unwrap()
            .push(challenge.clone());

        challenge
    }

    pub fn remove_challenge(&self, domain: &str) {
        let mut challenges = self.active_challenges.write().unwrap();
        challenges.retain(|c| c.domain != domain);
    }

    pub fn clear_all(&self) {
        self.active_challenges.write().unwrap().clear();
    }

    pub fn apply_all_to_zone(&self, zone: &mut DnsZone) {
        let challenges = self.active_challenges.read().unwrap();
        for challenge in challenges.iter() {
            challenge.add_to_zone(zone);
        }
    }

    pub fn remove_all_from_zone(&self, zone: &mut DnsZone) {
        let challenges = self.active_challenges.read().unwrap();
        for challenge in challenges.iter() {
            challenge.remove_from_zone(zone);
        }
    }
}
