//! Deterministic wallet intents.

use edgerun_wire::{Archive, Deserialize, Serialize};
use edgerun_work::Hash;

use crate::chain::{WalletAccountId, WalletAddress, WalletAmount, WalletAssetId};

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct WalletTransferIntent {
    pub abi_version: u16,
    pub intent_id: Hash,
    pub from: WalletAccountId,
    pub to: WalletAddress,
    pub asset: WalletAssetId,
    pub amount: WalletAmount,
    pub max_fee_hash: Hash,
    pub sequence: u64,
    pub expires_at_ms: u64,
    pub memo_hash: Hash,
}
