//! Pure event-replay ledger for wallet settlement state.

extern crate alloc;

use alloc::collections::{BTreeMap, BTreeSet};

use edgerun_wire::{Archive, Deserialize, Serialize};
use edgerun_work::{Hash, PublicKey};

use crate::chain::{WalletAccountId, WalletAmount, WalletAssetId};
use crate::emission::EdgeEmissionClaim;
use crate::intent::WalletTransferIntent;
use crate::observation::ExternalTxObservation;
use crate::proof::{edge_emission_claim_hash, external_tx_observation_hash};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WalletSettlementError {
    InvalidAdmission,
    InvalidReceipt,
    InvalidNotaryEvidence,
    ProofMismatch,
    DuplicateClaim,
    DuplicateObservation,
    UnknownAccount,
    InsufficientBalance,
    ZeroAmount,
    Overflow,
}

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub enum WalletSettlementEvent {
    IntentCreated(WalletTransferIntent),
    ExternalTxObserved(ExternalTxObservation),
    EdgeEmissionClaimed(EdgeEmissionClaim),
    EdgeEmissionFinalized(EdgeEmissionClaim),
    BurnClaimed(EdgeEmissionClaim),
    Rejected { claim_hash: Hash, reason_code: u16 },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct BalanceKey {
    account: WalletAccountId,
    asset_hash: Hash,
}

#[derive(Clone, Debug, Default)]
pub struct WalletLedger {
    balances: BTreeMap<BalanceKey, WalletAmount>,
    finalized_claims: BTreeSet<Hash>,
    observed_external_txs: BTreeSet<Hash>,
    intents: BTreeSet<Hash>,
}

impl WalletLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn balance(&self, account: &WalletAccountId, asset: &WalletAssetId) -> WalletAmount {
        *self
            .balances
            .get(&BalanceKey {
                account: account.clone(),
                asset_hash: crate::proof::wallet_asset_id_hash(asset),
            })
            .unwrap_or(&WalletAmount::new(
                0,
                asset.decimals.unwrap_or_default() as u8,
            ))
    }

    pub fn balance_by_asset_hash(
        &self,
        account: &WalletAccountId,
        asset_hash: Hash,
        scale: u8,
    ) -> WalletAmount {
        *self
            .balances
            .get(&BalanceKey {
                account: account.clone(),
                asset_hash,
            })
            .unwrap_or(&WalletAmount::new(0, scale))
    }

    pub fn is_claim_finalized(&self, claim_hash: &Hash) -> bool {
        self.finalized_claims.contains(claim_hash)
    }

    pub fn has_external_observation(&self, observation_hash: &Hash) -> bool {
        self.observed_external_txs.contains(observation_hash)
    }

    pub fn apply(&mut self, event: &WalletSettlementEvent) -> Result<(), WalletSettlementError> {
        match event {
            WalletSettlementEvent::IntentCreated(intent) => {
                self.intents.insert(intent.intent_id);
                Ok(())
            }
            WalletSettlementEvent::ExternalTxObserved(observation) => {
                let hash = external_tx_observation_hash(observation);
                if !self.observed_external_txs.insert(hash) {
                    return Err(WalletSettlementError::DuplicateObservation);
                }
                Ok(())
            }
            WalletSettlementEvent::EdgeEmissionClaimed(_) => Ok(()),
            WalletSettlementEvent::EdgeEmissionFinalized(claim) => self.finalize_emission(claim),
            WalletSettlementEvent::BurnClaimed(claim) => self.apply_burn(claim),
            WalletSettlementEvent::Rejected { .. } => Ok(()),
        }
    }

    pub fn replay(events: &[WalletSettlementEvent]) -> Result<Self, WalletSettlementError> {
        let mut ledger = Self::new();
        for event in events {
            ledger.apply(event)?;
        }
        Ok(ledger)
    }

    fn finalize_emission(
        &mut self,
        claim: &EdgeEmissionClaim,
    ) -> Result<(), WalletSettlementError> {
        if claim.amount.is_zero() {
            return Err(WalletSettlementError::ZeroAmount);
        }
        let claim_hash = edge_emission_claim_hash(claim);
        if !self.finalized_claims.insert(claim_hash) {
            return Err(WalletSettlementError::DuplicateClaim);
        }
        self.add_balance(&claim.beneficiary, claim.policy_hash, claim.amount)
    }

    fn apply_burn(&mut self, claim: &EdgeEmissionClaim) -> Result<(), WalletSettlementError> {
        if claim.amount.is_zero() {
            return Err(WalletSettlementError::ZeroAmount);
        }
        self.sub_balance(&claim.beneficiary, claim.policy_hash, claim.amount)
    }

    fn add_balance(
        &mut self,
        account: &WalletAccountId,
        asset_hash: Hash,
        amount: WalletAmount,
    ) -> Result<(), WalletSettlementError> {
        let key = BalanceKey {
            account: account.clone(),
            asset_hash,
        };
        let current = self
            .balances
            .get(&key)
            .copied()
            .unwrap_or(WalletAmount::new(0, amount.scale));
        let sum = current
            .to_decimal()
            .checked_add(&amount.to_decimal())
            .ok_or(WalletSettlementError::Overflow)?;
        self.balances.insert(key, WalletAmount::from_decimal(sum));
        Ok(())
    }

    pub fn credit_account(
        &mut self,
        account: &WalletAccountId,
        asset_hash: Hash,
        amount: WalletAmount,
        _reason_hash: Hash,
    ) -> Result<(), WalletSettlementError> {
        if amount.is_zero() {
            return Err(WalletSettlementError::ZeroAmount);
        }
        self.add_balance(account, asset_hash, amount)
    }

    fn sub_balance(
        &mut self,
        account: &WalletAccountId,
        asset_hash: Hash,
        amount: WalletAmount,
    ) -> Result<(), WalletSettlementError> {
        let key = BalanceKey {
            account: account.clone(),
            asset_hash,
        };
        let current = self
            .balances
            .get(&key)
            .copied()
            .ok_or(WalletSettlementError::UnknownAccount)?;
        let remaining = current
            .to_decimal()
            .checked_sub(&amount.to_decimal())
            .ok_or(WalletSettlementError::InsufficientBalance)?;
        self.balances
            .insert(key, WalletAmount::from_decimal(remaining));
        Ok(())
    }

    pub fn debit_account(
        &mut self,
        account: &WalletAccountId,
        asset_hash: Hash,
        amount: WalletAmount,
        _reason_hash: Hash,
    ) -> Result<(), WalletSettlementError> {
        if amount.is_zero() {
            return Err(WalletSettlementError::ZeroAmount);
        }
        self.sub_balance(account, asset_hash, amount)
    }
}

pub fn account(owner: PublicKey, subaccount: u64) -> WalletAccountId {
    WalletAccountId {
        abi_version: crate::proof::WALLET_CORE_ABI_VERSION,
        owner,
        subaccount,
    }
}
