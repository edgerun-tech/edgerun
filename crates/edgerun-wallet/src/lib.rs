//! EdgeRun Wallet — core types and logic.
//!
//! Uses existing edgerun-storage::stream and edgerun-storage for durable event records.
//! Implements DecimalAmount (no floats, no external decimal crates),
//! asset model, status machine, and exchange types.

#![no_std]

extern crate alloc;

pub mod adapter;
pub mod amount;
pub mod asset;
pub mod chain;
pub mod emission;
pub mod errors;
pub mod intent;
pub mod ledger;
pub mod observation;
pub mod proof;
pub mod status;

#[cfg(test)]
mod settlement_tests;

// Re-exports
pub use adapter::ChainAdapter;
pub use amount::DecimalAmount;
pub use asset::{Asset, AssetRef, canonical_asset_id};
pub use chain::{
    WalletAccountId, WalletAddress, WalletAmount, WalletAssetId, WalletChainFamily, WalletChainId,
};
pub use emission::{
    EdgeEmissionClaim, EdgeEmissionEvidence, notary_work_report_hash, verify_edge_emission_evidence,
};
pub use errors::WalletError;
pub use intent::WalletTransferIntent;
pub use ledger::{WalletLedger, WalletSettlementError, WalletSettlementEvent, account};
pub use observation::ExternalTxObservation;
pub use proof::{
    WALLET_CORE_ABI_VERSION, edge_emission_claim_hash, edge_emission_claim_preimage,
    external_tx_observation_hash, external_tx_observation_preimage, is_edgerun_chain,
    wallet_account_id_hash, wallet_account_id_preimage, wallet_address_hash,
    wallet_address_preimage, wallet_asset_id_hash, wallet_asset_id_preimage, wallet_chain_id_hash,
    wallet_chain_id_preimage, wallet_transfer_intent_hash, wallet_transfer_intent_preimage,
};
pub use status::{CanonicalOrderStatus, can_transition, is_terminal};
