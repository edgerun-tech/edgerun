//! External-chain observations supplied by chain adapters or notaries.

extern crate alloc;

use alloc::string::String;

use edgerun_wire::{Archive, Deserialize, Serialize};
use edgerun_work::Hash;

use crate::chain::{WalletAddress, WalletAmount, WalletAssetId, WalletChainId};

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct ExternalTxObservation {
    pub abi_version: u16,
    pub chain: WalletChainId,
    pub tx_id: String,
    pub block_ref: String,
    pub confirmations: u64,
    pub asset: WalletAssetId,
    pub amount: WalletAmount,
    pub from_hint: Option<String>,
    pub to: WalletAddress,
    pub observed_at_ms: u64,
    pub evidence_hash: Hash,
}
