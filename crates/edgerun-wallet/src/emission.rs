//! EDGE emission claims and verification against admitted work evidence.

use edgerun_wire::{Archive, Deserialize, Serialize};
use edgerun_work::notary_work_report_preimage;
use edgerun_work::{
    Hash, NotaryWorkReport, WorkAdmission, WorkReceipt, blake3_hash, packet_hash,
    verify_notary_work_report, verify_work_admission, verify_work_receipt,
};

use crate::chain::{WalletAccountId, WalletAmount};
use crate::ledger::WalletSettlementError;
use crate::proof::edge_emission_claim_hash;

#[derive(Clone, Debug, PartialEq, Eq, Archive, Serialize, Deserialize)]
#[rkyv(crate = edgerun_wire)]
pub struct EdgeEmissionClaim {
    pub abi_version: u16,
    pub claim_id: Hash,
    pub beneficiary: WalletAccountId,
    pub amount: WalletAmount,
    pub admission_hash: Hash,
    pub receipt_hash: Hash,
    pub notary_report_hash: Hash,
    pub policy_hash: Hash,
}

pub struct EdgeEmissionEvidence<'a> {
    pub claim: &'a EdgeEmissionClaim,
    pub admission: &'a WorkAdmission,
    pub receipt: &'a WorkReceipt,
    pub notary_report: &'a NotaryWorkReport,
}

pub fn notary_work_report_hash(report: &NotaryWorkReport) -> Hash {
    blake3_hash(&notary_work_report_preimage(report))
}

pub fn verify_edge_emission_evidence(
    evidence: &EdgeEmissionEvidence<'_>,
) -> Result<Hash, WalletSettlementError> {
    if evidence.claim.amount.is_zero() {
        return Err(WalletSettlementError::ZeroAmount);
    }
    if !verify_work_admission(evidence.admission) {
        return Err(WalletSettlementError::InvalidAdmission);
    }
    if !verify_work_receipt(evidence.receipt) {
        return Err(WalletSettlementError::InvalidReceipt);
    }
    if !verify_notary_work_report(evidence.notary_report) {
        return Err(WalletSettlementError::InvalidNotaryEvidence);
    }

    let admission_hash = packet_hash(&edgerun_work::WorkPacket::WorkAdmission(
        evidence.admission.clone(),
    ))
    .map_err(|_| WalletSettlementError::InvalidAdmission)?;
    let receipt_hash = packet_hash(&edgerun_work::WorkPacket::WorkReceipt(
        evidence.receipt.clone(),
    ))
    .map_err(|_| WalletSettlementError::InvalidReceipt)?;
    let notary_report_hash = notary_work_report_hash(evidence.notary_report);

    if evidence.claim.admission_hash != admission_hash
        || evidence.claim.receipt_hash != receipt_hash
        || evidence.claim.notary_report_hash != notary_report_hash
        || evidence.claim.policy_hash != evidence.admission.policy_hash
    {
        return Err(WalletSettlementError::ProofMismatch);
    }

    if evidence.receipt.admission_hash != admission_hash
        || evidence.receipt.request_hash != evidence.admission.request_hash
        || evidence.notary_report.admission_hash != admission_hash
        || evidence.notary_report.request_hash != evidence.admission.request_hash
    {
        return Err(WalletSettlementError::ProofMismatch);
    }

    Ok(edge_emission_claim_hash(evidence.claim))
}
