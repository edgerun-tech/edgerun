#![no_std]

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;

use edgerun_core::protocol::{
    AssuranceClaim, CommandEnvelope, DelegationRecord, EventEnvelope, IdentityRecord,
    ProtocolRecord, RevocationRecord, RouteAdvertisement, Signature, SnapshotDescriptor,
};
use edgerun_verify::{
    protocol_record_hash, protocol_signable_wire_bytes, ProtocolFamily, ProtocolVerifyError,
};

pub use edgerun_verify::ProtocolFamily as SignableProtocolFamily;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolSignError {
    UnsupportedFamily,
    SignerFailed,
}

impl From<ProtocolVerifyError> for ProtocolSignError {
    fn from(value: ProtocolVerifyError) -> Self {
        match value {
            ProtocolVerifyError::UnsupportedFamily => Self::UnsupportedFamily,
            _ => Self::SignerFailed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolSigningInput {
    pub family: ProtocolFamily,
    pub wire_bytes: Vec<u8>,
    pub record_hash: Vec<u8>,
    pub signature_input: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtocolSigningOutput {
    pub family: ProtocolFamily,
    pub signature: Signature,
    pub record_hash: Vec<u8>,
}

pub trait ProtocolSigner {
    fn signature_algorithm(&self) -> i32;

    fn sign_signature_input(
        &self,
        family: ProtocolFamily,
        signature_input: &[u8],
    ) -> Result<Vec<u8>, ProtocolSignError>;

    fn sign_protocol_record(
        &self,
        record: &ProtocolRecord,
        family: ProtocolFamily,
    ) -> Result<ProtocolSigningOutput, ProtocolSignError> {
        let input = protocol_signing_input(record, family)?;
        let value = self.sign_signature_input(family, &input.signature_input)?;
        Ok(ProtocolSigningOutput {
            family,
            signature: Signature {
                algorithm: self.signature_algorithm(),
                value,
            },
            record_hash: input.record_hash,
        })
    }
}

pub fn protocol_signing_input(
    record: &ProtocolRecord,
    family: ProtocolFamily,
) -> Result<ProtocolSigningInput, ProtocolSignError> {
    let wire_bytes = protocol_signable_wire_bytes(record, family)?;
    let record_hash = protocol_record_hash(record, family)?;
    let signature_input = edgerun_core::crypto::signature_input(family.sig_domain(), &record_hash);
    Ok(ProtocolSigningInput {
        family,
        wire_bytes,
        record_hash,
        signature_input,
    })
}

impl<T: ProtocolSigner + ?Sized> ProtocolSigner for Arc<T> {
    fn signature_algorithm(&self) -> i32 {
        self.as_ref().signature_algorithm()
    }

    fn sign_signature_input(
        &self,
        family: ProtocolFamily,
        signature_input: &[u8],
    ) -> Result<Vec<u8>, ProtocolSignError> {
        self.as_ref().sign_signature_input(family, signature_input)
    }
}

pub fn sign_event_envelope<S: ProtocolSigner + ?Sized>(
    signer: &S,
    event: &EventEnvelope,
) -> Result<ProtocolSigningOutput, ProtocolSignError> {
    signer.sign_protocol_record(
        &ProtocolRecord::EventEnvelope(event.clone()),
        ProtocolFamily::EventEnvelope,
    )
}

pub fn sign_command_envelope<S: ProtocolSigner + ?Sized>(
    signer: &S,
    command: &CommandEnvelope,
) -> Result<ProtocolSigningOutput, ProtocolSignError> {
    signer.sign_protocol_record(
        &ProtocolRecord::CommandEnvelope(command.clone()),
        ProtocolFamily::CommandEnvelope,
    )
}

pub fn sign_delegation_record<S: ProtocolSigner + ?Sized>(
    signer: &S,
    delegation: &DelegationRecord,
) -> Result<ProtocolSigningOutput, ProtocolSignError> {
    signer.sign_protocol_record(
        &ProtocolRecord::DelegationRecord(delegation.clone()),
        ProtocolFamily::DelegationRecord,
    )
}

pub fn sign_revocation_record<S: ProtocolSigner + ?Sized>(
    signer: &S,
    revocation: &RevocationRecord,
) -> Result<ProtocolSigningOutput, ProtocolSignError> {
    signer.sign_protocol_record(
        &ProtocolRecord::RevocationRecord(revocation.clone()),
        ProtocolFamily::RevocationRecord,
    )
}

pub fn sign_identity_record<S: ProtocolSigner + ?Sized>(
    signer: &S,
    identity: &IdentityRecord,
) -> Result<ProtocolSigningOutput, ProtocolSignError> {
    signer.sign_protocol_record(
        &ProtocolRecord::IdentityRecord(identity.clone()),
        ProtocolFamily::IdentityRecord,
    )
}

pub fn sign_assurance_claim<S: ProtocolSigner + ?Sized>(
    signer: &S,
    claim: &AssuranceClaim,
) -> Result<ProtocolSigningOutput, ProtocolSignError> {
    signer.sign_protocol_record(
        &ProtocolRecord::AssuranceClaim(claim.clone()),
        ProtocolFamily::AssuranceClaim,
    )
}

pub fn sign_snapshot_descriptor<S: ProtocolSigner + ?Sized>(
    signer: &S,
    descriptor: &SnapshotDescriptor,
) -> Result<ProtocolSigningOutput, ProtocolSignError> {
    signer.sign_protocol_record(
        &ProtocolRecord::SnapshotDescriptor(descriptor.clone()),
        ProtocolFamily::SnapshotDescriptor,
    )
}

pub fn sign_route_advertisement<S: ProtocolSigner + ?Sized>(
    signer: &S,
    route: &RouteAdvertisement,
) -> Result<ProtocolSigningOutput, ProtocolSignError> {
    signer.sign_protocol_record(
        &ProtocolRecord::RouteAdvertisement(route.clone()),
        ProtocolFamily::RouteAdvertisement,
    )
}
