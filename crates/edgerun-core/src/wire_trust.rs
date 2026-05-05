//! Rkyv-only trust wire boundary.
//!
//! Trust record byte helpers are explicit breakpoints until the signing and
//! verification paths archive the concrete rkyv record directly.

use alloc::vec::Vec;

use crate::protocol::{AssuranceClaim, DelegationRecord, IdentityRecord, RevocationRecord};

#[cold]
fn removed_trust_wire_path() -> ! {
    panic!("trust wire helpers were removed; use the rkyv wire boundary")
}

#[must_use]
pub fn identity_record_signable_bytes(_value: &IdentityRecord) -> Vec<u8> {
    removed_trust_wire_path()
}

#[must_use]
pub fn identity_record_full_bytes(_value: &IdentityRecord) -> Vec<u8> {
    removed_trust_wire_path()
}

#[must_use]
pub fn delegation_record_signable_bytes(_value: &DelegationRecord) -> Vec<u8> {
    removed_trust_wire_path()
}

#[must_use]
pub fn delegation_record_full_bytes(_value: &DelegationRecord) -> Vec<u8> {
    removed_trust_wire_path()
}

#[must_use]
pub fn revocation_record_signable_bytes(_value: &RevocationRecord) -> Vec<u8> {
    removed_trust_wire_path()
}

#[must_use]
pub fn revocation_record_full_bytes(_value: &RevocationRecord) -> Vec<u8> {
    removed_trust_wire_path()
}

#[must_use]
pub fn assurance_claim_signable_bytes(_value: &AssuranceClaim) -> Vec<u8> {
    removed_trust_wire_path()
}

#[must_use]
pub fn assurance_claim_full_bytes(_value: &AssuranceClaim) -> Vec<u8> {
    removed_trust_wire_path()
}
