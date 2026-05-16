//! Deterministic wallet preimages, hashes, and proof helpers.

extern crate alloc;

use alloc::vec::Vec;

use edgerun_work::preimage::PreimageBuilder;
use edgerun_work::{Hash, blake3_hash};

use crate::chain::{
    WalletAccountId, WalletAddress, WalletAmount, WalletAssetId, WalletChainFamily, WalletChainId,
};
use crate::emission::EdgeEmissionClaim;
use crate::intent::WalletTransferIntent;
use crate::observation::ExternalTxObservation;

pub const WALLET_CORE_ABI_VERSION: u16 = 1;

const CHAIN_ID_DOMAIN: &[u8] = b"edgerun:v1:wallet:chain-id";
const ASSET_ID_DOMAIN: &[u8] = b"edgerun:v1:wallet:asset-id";
const ACCOUNT_ID_DOMAIN: &[u8] = b"edgerun:v1:wallet:account-id";
const ADDRESS_DOMAIN: &[u8] = b"edgerun:v1:wallet:address";
const TRANSFER_INTENT_DOMAIN: &[u8] = b"edgerun:v1:wallet:transfer-intent";
const EXTERNAL_TX_OBSERVATION_DOMAIN: &[u8] = b"edgerun:v1:wallet:external-tx-observation";
const EDGE_EMISSION_CLAIM_DOMAIN: &[u8] = b"edgerun:v1:wallet:edge-emission-claim";

pub fn wallet_chain_id_preimage(value: &WalletChainId) -> Vec<u8> {
    let mut builder = PreimageBuilder::domain(CHAIN_ID_DOMAIN)
        .u16(value.abi_version)
        .u16(value.family as u16)
        .bytes(value.network.as_bytes())
        .u16(value.namespace.is_some() as u16);
    if let Some(namespace) = &value.namespace {
        builder = builder.bytes(namespace.as_bytes());
    }
    builder.finish()
}

pub fn wallet_chain_id_hash(value: &WalletChainId) -> Hash {
    blake3_hash(&wallet_chain_id_preimage(value))
}

pub fn wallet_asset_id_preimage(value: &WalletAssetId) -> Vec<u8> {
    let mut builder = PreimageBuilder::domain(ASSET_ID_DOMAIN)
        .u16(value.abi_version)
        .bytes(value.symbol.as_bytes())
        .hash(&wallet_chain_id_hash(&value.chain))
        .u16(value.contract.is_some() as u16);
    if let Some(contract) = &value.contract {
        builder = builder.bytes(contract.as_bytes());
    }
    builder
        .u16(value.decimals.is_some() as u16)
        .u64(value.decimals.unwrap_or_default() as u64)
        .finish()
}

pub fn wallet_asset_id_hash(value: &WalletAssetId) -> Hash {
    blake3_hash(&wallet_asset_id_preimage(value))
}

pub fn wallet_account_id_preimage(value: &WalletAccountId) -> Vec<u8> {
    PreimageBuilder::domain(ACCOUNT_ID_DOMAIN)
        .u16(value.abi_version)
        .hash(&value.owner)
        .u64(value.subaccount)
        .finish()
}

pub fn wallet_account_id_hash(value: &WalletAccountId) -> Hash {
    blake3_hash(&wallet_account_id_preimage(value))
}

pub fn wallet_address_preimage(value: &WalletAddress) -> Vec<u8> {
    PreimageBuilder::domain(ADDRESS_DOMAIN)
        .u16(value.abi_version)
        .hash(&wallet_chain_id_hash(&value.chain))
        .bytes(value.address.as_bytes())
        .finish()
}

pub fn wallet_address_hash(value: &WalletAddress) -> Hash {
    blake3_hash(&wallet_address_preimage(value))
}

pub fn wallet_amount_to_preimage(builder: PreimageBuilder, value: WalletAmount) -> PreimageBuilder {
    builder
        .u64((value.mantissa >> 64) as u64)
        .u64(value.mantissa as u64)
        .u16(value.scale as u16)
}

pub fn wallet_transfer_intent_preimage(value: &WalletTransferIntent) -> Vec<u8> {
    wallet_amount_to_preimage(
        PreimageBuilder::domain(TRANSFER_INTENT_DOMAIN)
            .u16(value.abi_version)
            .hash(&value.intent_id)
            .hash(&wallet_account_id_hash(&value.from))
            .hash(&wallet_address_hash(&value.to))
            .hash(&wallet_asset_id_hash(&value.asset)),
        value.amount,
    )
    .hash(&value.max_fee_hash)
    .u64(value.sequence)
    .u64(value.expires_at_ms)
    .hash(&value.memo_hash)
    .finish()
}

pub fn wallet_transfer_intent_hash(value: &WalletTransferIntent) -> Hash {
    blake3_hash(&wallet_transfer_intent_preimage(value))
}

pub fn external_tx_observation_preimage(value: &ExternalTxObservation) -> Vec<u8> {
    let mut builder = wallet_amount_to_preimage(
        PreimageBuilder::domain(EXTERNAL_TX_OBSERVATION_DOMAIN)
            .u16(value.abi_version)
            .hash(&wallet_chain_id_hash(&value.chain))
            .bytes(value.tx_id.as_bytes())
            .bytes(value.block_ref.as_bytes())
            .u64(value.confirmations)
            .hash(&wallet_asset_id_hash(&value.asset)),
        value.amount,
    )
    .u16(value.from_hint.is_some() as u16);
    if let Some(from_hint) = &value.from_hint {
        builder = builder.bytes(from_hint.as_bytes());
    }
    builder
        .hash(&wallet_address_hash(&value.to))
        .u64(value.observed_at_ms)
        .hash(&value.evidence_hash)
        .finish()
}

pub fn external_tx_observation_hash(value: &ExternalTxObservation) -> Hash {
    blake3_hash(&external_tx_observation_preimage(value))
}

pub fn edge_emission_claim_preimage(value: &EdgeEmissionClaim) -> Vec<u8> {
    wallet_amount_to_preimage(
        PreimageBuilder::domain(EDGE_EMISSION_CLAIM_DOMAIN)
            .u16(value.abi_version)
            .hash(&value.claim_id)
            .hash(&wallet_account_id_hash(&value.beneficiary)),
        value.amount,
    )
    .hash(&value.admission_hash)
    .hash(&value.receipt_hash)
    .hash(&value.notary_report_hash)
    .hash(&value.policy_hash)
    .finish()
}

pub fn edge_emission_claim_hash(value: &EdgeEmissionClaim) -> Hash {
    blake3_hash(&edge_emission_claim_preimage(value))
}

pub fn is_edgerun_chain(value: &WalletChainId) -> bool {
    value.family == WalletChainFamily::EdgeRun
}
