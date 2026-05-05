#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use edgerun_core::crypto;
use edgerun_core::protocol::{
    self, CommandEnvelope, DelegationRecord, EventEnvelope, IdentityRecord, ProtocolRecord,
    RevocationRecord, Signature,
};

pub use edgerun_core::protocol::Signature as ProtocolSignature;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolFamily {
    EventEnvelope,
    CommandEnvelope,
    DelegationRecord,
    RevocationRecord,
    IdentityRecord,
    AssuranceClaim,
    SnapshotDescriptor,
    QueryRequest,
    QueryResultFragment,
    RouteAdvertisement,
    SessionHello,
    SessionAccept,
    RelayEnvelope,
}

impl ProtocolFamily {
    pub fn hash_domain(self) -> &'static str {
        match self {
            Self::EventEnvelope => crypto::HASH_DOMAIN_EVENT_ENVELOPE,
            Self::CommandEnvelope => crypto::HASH_DOMAIN_COMMAND_ENVELOPE,
            Self::DelegationRecord => crypto::HASH_DOMAIN_DELEGATION_RECORD,
            Self::RevocationRecord => crypto::HASH_DOMAIN_REVOCATION_RECORD,
            Self::IdentityRecord => crypto::HASH_DOMAIN_IDENTITY_RECORD,
            Self::AssuranceClaim => crypto::HASH_DOMAIN_ASSURANCE_CLAIM,
            Self::SnapshotDescriptor => crypto::HASH_DOMAIN_SNAPSHOT_DESCRIPTOR,
            Self::QueryRequest => crypto::HASH_DOMAIN_QUERY_REQUEST,
            Self::QueryResultFragment => crypto::HASH_DOMAIN_QUERY_RESULT_FRAGMENT,
            Self::RouteAdvertisement => crypto::HASH_DOMAIN_ROUTE_ADVERTISEMENT,
            Self::SessionHello => crypto::HASH_DOMAIN_SESSION_HELLO,
            Self::SessionAccept => crypto::HASH_DOMAIN_SESSION_ACCEPT,
            Self::RelayEnvelope => crypto::HASH_DOMAIN_RELAY_ENVELOPE,
        }
    }

    pub fn sig_domain(self) -> &'static str {
        match self {
            Self::EventEnvelope => crypto::SIG_DOMAIN_EVENT_ENVELOPE,
            Self::CommandEnvelope => crypto::SIG_DOMAIN_COMMAND_ENVELOPE,
            Self::DelegationRecord => crypto::SIG_DOMAIN_DELEGATION_RECORD,
            Self::RevocationRecord => crypto::SIG_DOMAIN_REVOCATION_RECORD,
            Self::IdentityRecord => crypto::SIG_DOMAIN_IDENTITY_RECORD,
            Self::AssuranceClaim => crypto::SIG_DOMAIN_ASSURANCE_CLAIM,
            Self::SnapshotDescriptor => crypto::SIG_DOMAIN_SNAPSHOT_DESCRIPTOR,
            Self::QueryRequest => crypto::SIG_DOMAIN_QUERY_REQUEST,
            Self::QueryResultFragment => crypto::SIG_DOMAIN_QUERY_RESULT_FRAGMENT,
            Self::RouteAdvertisement => crypto::SIG_DOMAIN_ROUTE_ADVERTISEMENT,
            Self::SessionHello => crypto::SIG_DOMAIN_SESSION_HELLO,
            Self::SessionAccept => crypto::SIG_DOMAIN_SESSION_ACCEPT,
            Self::RelayEnvelope => crypto::SIG_DOMAIN_RELAY_ENVELOPE,
        }
    }

    /// Returns true only for families whose canonical signable bytes are backed
    /// by concrete protocol encoders in edgerun-core today.
    ///
    /// Other families are listed here intentionally, but verification rejects
    /// them until their canonicalization is implemented. This prevents the
    /// placeholder ProtocolRecord fallback from becoming accidental authority.
    pub fn canonicalization_is_implemented(self) -> bool {
        matches!(self, Self::EventEnvelope | Self::CommandEnvelope)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolVerifyError {
    MissingSignature,
    UnsupportedSignatureAlgorithm,
    InvalidPublicKey,
    InvalidSignatureLength,
    InvalidSignature,
    UnsupportedFamily,
    MissingWriterIdentity,
    MissingIssuerIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProtocolVerification {
    pub family: ProtocolFamily,
    pub signature_algorithm: i32,
    pub signer_id: Vec<u8>,
    pub record_hash: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolSignerRef<'a> {
    /// Raw P-256 public key as x || y, 64 bytes.
    P256Raw64(&'a [u8]),
    /// SEC1 P-256 public key. May be compressed or uncompressed if p256 accepts it.
    P256Sec1(&'a [u8]),
}

impl<'a> ProtocolSignerRef<'a> {
    pub fn signer_id(self) -> Vec<u8> {
        match self {
            Self::P256Raw64(bytes) | Self::P256Sec1(bytes) => bytes.to_vec(),
        }
    }

    fn to_p256_verifying_key(self) -> Result<crypto::VerifyingKey, ProtocolVerifyError> {
        match self {
            Self::P256Raw64(bytes) => {
                let raw: &[u8; 64] = bytes
                    .try_into()
                    .map_err(|_| ProtocolVerifyError::InvalidPublicKey)?;
                crypto::node_id_to_verifying_key(raw).ok_or(ProtocolVerifyError::InvalidPublicKey)
            }
            Self::P256Sec1(bytes) => crypto::VerifyingKey::from_sec1_bytes(bytes)
                .map_err(|_| ProtocolVerifyError::InvalidPublicKey),
        }
    }
}

pub fn protocol_record_hash(
    record: &ProtocolRecord,
    family: ProtocolFamily,
) -> Result<Vec<u8>, ProtocolVerifyError> {
    let canonical = protocol_signable_bytes(record, family)?;
    Ok(crypto::record_hash(family.hash_domain(), &canonical))
}

pub fn protocol_signature_input(
    record: &ProtocolRecord,
    family: ProtocolFamily,
) -> Result<Vec<u8>, ProtocolVerifyError> {
    let hash = protocol_record_hash(record, family)?;
    Ok(crypto::signature_input(family.sig_domain(), &hash))
}

pub fn protocol_signable_bytes(
    record: &ProtocolRecord,
    family: ProtocolFamily,
) -> Result<Vec<u8>, ProtocolVerifyError> {
    match (family, record) {
        (ProtocolFamily::EventEnvelope, ProtocolRecord::EventEnvelope(_))
        | (ProtocolFamily::CommandEnvelope, ProtocolRecord::CommandEnvelope(_)) => {
            Ok(protocol::canonical_bytes(record, true))
        }
        _ => Err(ProtocolVerifyError::UnsupportedFamily),
    }
}

pub fn verify_protocol_record(
    record: &ProtocolRecord,
    family: ProtocolFamily,
    signature: &Signature,
    signer: ProtocolSignerRef<'_>,
) -> Result<ProtocolVerification, ProtocolVerifyError> {
    if signature.algorithm as u8 != crypto::SIGNATURE_ALGORITHM_ECDSA_P256 {
        return Err(ProtocolVerifyError::UnsupportedSignatureAlgorithm);
    }
    if signature.value.len() != crypto::ECDSA_P256_SIGNATURE_LEN {
        return Err(ProtocolVerifyError::InvalidSignatureLength);
    }

    let canonical = protocol_signable_bytes(record, family)?;
    let record_hash = crypto::record_hash(family.hash_domain(), &canonical);
    let verifying_key = signer.to_p256_verifying_key()?;

    if !crypto::verify_record(
        &verifying_key,
        family.sig_domain(),
        &record_hash,
        &signature.value,
    ) {
        return Err(ProtocolVerifyError::InvalidSignature);
    }

    Ok(ProtocolVerification {
        family,
        signature_algorithm: signature.algorithm,
        signer_id: signer.signer_id(),
        record_hash,
    })
}

pub fn verify_protocol_record_hw(
    record: &ProtocolRecord,
    family: ProtocolFamily,
    signature: &Signature,
    signer: ProtocolSignerRef<'_>,
) -> Result<ProtocolVerification, ProtocolVerifyError> {
    if signature.algorithm as u8 != crypto::SIGNATURE_ALGORITHM_ECDSA_P256 {
        return Err(ProtocolVerifyError::UnsupportedSignatureAlgorithm);
    }
    if signature.value.len() != crypto::ECDSA_P256_SIGNATURE_LEN {
        return Err(ProtocolVerifyError::InvalidSignatureLength);
    }

    let canonical = protocol_signable_bytes(record, family)?;
    let record_hash = crypto::record_hash(family.hash_domain(), &canonical);
    let verifying_key = signer.to_p256_verifying_key()?;

    if !crypto::verify_canonical_record_hw(
        &verifying_key,
        family.sig_domain(),
        &canonical,
        &signature.value,
    ) {
        return Err(ProtocolVerifyError::InvalidSignature);
    }

    Ok(ProtocolVerification {
        family,
        signature_algorithm: signature.algorithm,
        signer_id: signer.signer_id(),
        record_hash,
    })
}

pub trait VerifiableProtocolRecord {
    fn protocol_family(&self) -> ProtocolFamily;
    fn protocol_record(&self) -> ProtocolRecord;
    fn protocol_signature(&self) -> Option<&Signature>;
}

impl VerifiableProtocolRecord for EventEnvelope {
    fn protocol_family(&self) -> ProtocolFamily {
        ProtocolFamily::EventEnvelope
    }

    fn protocol_record(&self) -> ProtocolRecord {
        ProtocolRecord::EventEnvelope(self.clone())
    }

    fn protocol_signature(&self) -> Option<&Signature> {
        self.signature.as_ref()
    }
}

impl VerifiableProtocolRecord for CommandEnvelope {
    fn protocol_family(&self) -> ProtocolFamily {
        ProtocolFamily::CommandEnvelope
    }

    fn protocol_record(&self) -> ProtocolRecord {
        ProtocolRecord::CommandEnvelope(self.clone())
    }

    fn protocol_signature(&self) -> Option<&Signature> {
        self.signature.as_ref()
    }
}

impl VerifiableProtocolRecord for DelegationRecord {
    fn protocol_family(&self) -> ProtocolFamily {
        ProtocolFamily::DelegationRecord
    }

    fn protocol_record(&self) -> ProtocolRecord {
        ProtocolRecord::DelegationRecord(self.clone())
    }

    fn protocol_signature(&self) -> Option<&Signature> {
        self.signature.as_ref()
    }
}

impl VerifiableProtocolRecord for RevocationRecord {
    fn protocol_family(&self) -> ProtocolFamily {
        ProtocolFamily::RevocationRecord
    }

    fn protocol_record(&self) -> ProtocolRecord {
        ProtocolRecord::RevocationRecord(self.clone())
    }

    fn protocol_signature(&self) -> Option<&Signature> {
        self.signature.as_ref()
    }
}

pub fn verify_signed_record<R: VerifiableProtocolRecord>(
    record: &R,
    signer: ProtocolSignerRef<'_>,
) -> Result<ProtocolVerification, ProtocolVerifyError> {
    let signature = record
        .protocol_signature()
        .ok_or(ProtocolVerifyError::MissingSignature)?;
    verify_protocol_record(
        &record.protocol_record(),
        record.protocol_family(),
        signature,
        signer,
    )
}

pub fn verify_signed_record_hw<R: VerifiableProtocolRecord>(
    record: &R,
    signer: ProtocolSignerRef<'_>,
) -> Result<ProtocolVerification, ProtocolVerifyError> {
    let signature = record
        .protocol_signature()
        .ok_or(ProtocolVerifyError::MissingSignature)?;
    verify_protocol_record_hw(
        &record.protocol_record(),
        record.protocol_family(),
        signature,
        signer,
    )
}

/// Verify an event envelope when the caller already knows the stream writer key.
pub fn verify_event_envelope(
    event: &EventEnvelope,
    writer: ProtocolSignerRef<'_>,
) -> Result<ProtocolVerification, ProtocolVerifyError> {
    verify_signed_record(event, writer)
}

/// Verify a command envelope when the caller already resolved the issuer key.
pub fn verify_command_envelope(
    command: &CommandEnvelope,
    issuer: ProtocolSignerRef<'_>,
) -> Result<ProtocolVerification, ProtocolVerifyError> {
    verify_signed_record(command, issuer)
}

/// Verification entry point for delegation records.
///
/// Currently returns UnsupportedFamily until DelegationRecord has concrete
/// protocol canonicalization in edgerun-core.
pub fn verify_delegation_record(
    delegation: &DelegationRecord,
    issuer: ProtocolSignerRef<'_>,
) -> Result<ProtocolVerification, ProtocolVerifyError> {
    verify_signed_record(delegation, issuer)
}

/// Verification entry point for revocation records.
///
/// Currently returns UnsupportedFamily until RevocationRecord has concrete
/// protocol canonicalization in edgerun-core.
pub fn verify_revocation_record(
    revocation: &RevocationRecord,
    issuer: ProtocolSignerRef<'_>,
) -> Result<ProtocolVerification, ProtocolVerifyError> {
    verify_signed_record(revocation, issuer)
}

/// Verification entry point for identity records.
///
/// Currently returns UnsupportedFamily until IdentityRecord has concrete
/// protocol canonicalization in edgerun-core.
pub fn verify_identity_record(
    identity: &IdentityRecord,
    signature: &Signature,
    signer: ProtocolSignerRef<'_>,
) -> Result<ProtocolVerification, ProtocolVerifyError> {
    verify_protocol_record(
        &ProtocolRecord::IdentityRecord(identity.clone()),
        ProtocolFamily::IdentityRecord,
        signature,
        signer,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_core::crypto::{sign_canonical_record, SigningKey};
    use edgerun_core::protocol::{EventEnvelope, ProtocolRecord, Signature};

    fn test_signing_key() -> SigningKey {
        let bytes: [u8; 32] = [7u8; 32];
        SigningKey::from_bytes(&bytes.into()).unwrap()
    }

    #[test]
    fn verifies_event_envelope_signature_using_protocol_domains() {
        let signing = test_signing_key();
        let writer = edgerun_core::crypto::verifying_key_to_node_id(signing.verifying_key());
        let mut event = EventEnvelope {
            envelope_version: 1,
            stream_id: writer.to_vec(),
            seq: 0,
            event_version: 1,
            ..EventEnvelope::default()
        };
        let canonical = protocol::canonical_bytes(&ProtocolRecord::EventEnvelope(event.clone()), true);
        let sig = sign_canonical_record(&signing, crypto::SIG_DOMAIN_EVENT_ENVELOPE, &canonical)
            .expect("signing works");
        event.signature = Some(Signature {
            algorithm: crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32,
            value: sig,
        });

        let verified = verify_event_envelope(&event, ProtocolSignerRef::P256Raw64(&writer)).unwrap();
        assert_eq!(verified.family, ProtocolFamily::EventEnvelope);
        assert_eq!(verified.record_hash.len(), 32);
    }

    #[test]
    fn rejects_wrong_signature_domain() {
        let signing = test_signing_key();
        let writer = edgerun_core::crypto::verifying_key_to_node_id(signing.verifying_key());
        let mut event = EventEnvelope {
            envelope_version: 1,
            stream_id: writer.to_vec(),
            seq: 0,
            event_version: 1,
            ..EventEnvelope::default()
        };
        let canonical = protocol::canonical_bytes(&ProtocolRecord::EventEnvelope(event.clone()), true);
        let sig = sign_canonical_record(&signing, crypto::SIG_DOMAIN_COMMAND_ENVELOPE, &canonical)
            .expect("signing works");
        event.signature = Some(Signature {
            algorithm: crypto::SIGNATURE_ALGORITHM_ECDSA_P256 as i32,
            value: sig,
        });

        let err = verify_event_envelope(&event, ProtocolSignerRef::P256Raw64(&writer)).unwrap_err();
        assert_eq!(err, ProtocolVerifyError::InvalidSignature);
    }

    #[test]
    fn unsupported_placeholder_families_are_rejected() {
        assert!(!ProtocolFamily::DelegationRecord.canonicalization_is_implemented());
        let record = ProtocolRecord::DelegationRecord(DelegationRecord::default());
        let err = protocol_record_hash(&record, ProtocolFamily::DelegationRecord).unwrap_err();
        assert_eq!(err, ProtocolVerifyError::UnsupportedFamily);
    }
}
