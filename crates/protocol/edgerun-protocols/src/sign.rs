//! Signing adapters for legacy `edgerun-core` protocol records.
//!
//! These helpers keep older stream, command, identity, and snapshot records
//! verifiable while the active cross-node authority model converges on
//! `edgerun-work`.

extern crate alloc;

use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::verify::{
    ProtocolFamily, ProtocolVerifyError, protocol_record_hash, protocol_signable_wire_bytes,
};
use edgerun_core::protocol::{
    AssuranceClaim, CommandEnvelope, DelegationRecord, EventEnvelope, IdentityRecord,
    ProtocolRecord, RevocationRecord, RouteAdvertisement, Signature, SnapshotDescriptor,
};

pub use crate::verify::ProtocolFamily as SignableProtocolFamily;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolSignError {
    UnsupportedFamily,
    SignerFailed,
    InvalidKey,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageSignAlgorithm {
    Ed25519,
    EcdsaP256Sha256,
}

pub trait MessageSigner {
    fn message_sign_algorithm(&self) -> MessageSignAlgorithm;
    fn public_key_bytes(&self) -> Vec<u8>;
    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, ProtocolSignError>;
}

#[cfg(feature = "ed25519")]
#[derive(Clone)]
pub struct Ed25519MessageSigner {
    signing_key: edgerun_crypto::Ed25519SigningKey,
}

#[cfg(feature = "ed25519")]
impl Ed25519MessageSigner {
    pub const fn new(signing_key: edgerun_crypto::Ed25519SigningKey) -> Self {
        Self { signing_key }
    }

    pub fn signing_key(&self) -> &edgerun_crypto::Ed25519SigningKey {
        &self.signing_key
    }

    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self::new(edgerun_crypto::Ed25519SigningKey::from_bytes(&seed))
    }

    pub fn verifying_key(&self) -> edgerun_crypto::Ed25519VerifyingKey {
        self.signing_key.verifying_key()
    }
}

#[cfg(feature = "ed25519")]
impl MessageSigner for Ed25519MessageSigner {
    fn message_sign_algorithm(&self) -> MessageSignAlgorithm {
        MessageSignAlgorithm::Ed25519
    }

    fn public_key_bytes(&self) -> Vec<u8> {
        self.verifying_key().to_bytes().to_vec()
    }

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, ProtocolSignError> {
        Ok(self.signing_key.sign_bytes(message).to_vec())
    }
}

#[cfg(feature = "sign-p256")]
#[derive(Clone)]
pub struct P256MessageSigner {
    signing_key: edgerun_core::crypto::SigningKey,
}

#[cfg(feature = "sign-p256")]
impl P256MessageSigner {
    pub const fn new(signing_key: edgerun_core::crypto::SigningKey) -> Self {
        Self { signing_key }
    }

    pub fn signing_key(&self) -> &edgerun_core::crypto::SigningKey {
        &self.signing_key
    }

    pub fn verifying_key(&self) -> edgerun_core::crypto::VerifyingKey {
        self.signing_key.verifying_key()
    }
}

#[cfg(feature = "sign-p256")]
impl MessageSigner for P256MessageSigner {
    fn message_sign_algorithm(&self) -> MessageSignAlgorithm {
        MessageSignAlgorithm::EcdsaP256Sha256
    }

    fn public_key_bytes(&self) -> Vec<u8> {
        edgerun_core::crypto::verifying_key_to_node_id(&self.signing_key.verifying_key()).to_vec()
    }

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, ProtocolSignError> {
        let digest = edgerun_core::crypto::sha256(message);
        self.signing_key
            .sign_prehash_fixed(&digest)
            .map(|signature| signature.to_vec())
            .map_err(|_| ProtocolSignError::SignerFailed)
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
