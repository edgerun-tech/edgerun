use crate::prelude::v1::*;
use alloc::sync::Arc;

use edgerun_dns::record::DnsRecordType;
use edgerun_dns::zone::DnsZone;
use edgerun_rt::RwLock;

use crate::account::AccountKey;
use crate::challenge_material::{dns_01_record_name, dns_01_record_value};

#[derive(Clone)]
pub struct DnsChallenge {
    domain: String,
    txt_record_name: String,
    txt_record_value: String,
}

impl DnsChallenge {
    pub fn new(domain: &str, token: &str, account_key: &AccountKey) -> Self {
        let thumbprint = account_key.thumbprint_b64();
        let txt_value = dns_01_record_value(token, &thumbprint);
        let txt_record_name = dns_01_record_name(domain);

        Self {
            domain: domain.to_string(),
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
    active_challenges: RwLock<Vec<DnsChallenge>>,
}

impl DnsChallengeManager {
    pub fn new(account_key: Arc<AccountKey>) -> Self {
        Self {
            account_key,
            active_challenges: RwLock::new(Vec::new()),
        }
    }

    pub fn create_challenge(&self, domain: &str, token: &str) -> DnsChallenge {
        let challenge = DnsChallenge::new(domain, token, &self.account_key);

        self.active_challenges.write().push(challenge.clone());

        challenge
    }

    pub fn remove_challenge(&self, domain: &str) {
        let mut challenges = self.active_challenges.write();
        challenges.retain(|c| c.domain != domain);
    }

    pub fn clear_all(&self) {
        self.active_challenges.write().clear();
    }

    pub fn apply_all_to_zone(&self, zone: &mut DnsZone) {
        let challenges = self.active_challenges.read();
        for challenge in challenges.iter() {
            challenge.add_to_zone(zone);
        }
    }

    pub fn remove_all_from_zone(&self, zone: &mut DnsZone) {
        let challenges = self.active_challenges.read();
        for challenge in challenges.iter() {
            challenge.remove_from_zone(zone);
        }
    }
}
