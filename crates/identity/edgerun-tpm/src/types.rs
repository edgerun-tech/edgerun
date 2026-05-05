use crate::prelude::v1::*;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// Signature algorithms supported by the TPM signing path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TpmSignatureAlgorithm {
    RsaPkcs1v15Sha256,
    RsaPssSha256,
    EcdsaP256Sha256,
    EcdsaP384Sha384,
    EcSchnorr,
    Eddsa,
    Opaque(String),
}

/// Assurance level reported for a TPM key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TpmAssuranceLevel {
    SoftwareSimulated,
    DiscreteTpm,
    IntegratedTpm,
    Certified(String),
    Unknown,
}

/// A TPM handle newtype.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TpmHandle(pub u32);

/// Metadata about a TPM key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmKeyInfo {
    pub key_name: String,
    pub algorithm: TpmSignatureAlgorithm,
    pub public_key: Vec<u8>,
    pub attestation_blob: Option<Vec<u8>>,
    pub assurance_level: TpmAssuranceLevel,
}

/// TPM2 command header (wire format).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TpmCommandHeader {
    pub tag: u16,
    pub size: u32,
    pub command_code: u32,
}

/// TPM2 response header (wire format).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TpmResponseHeader {
    pub tag: u16,
    pub size: u32,
    pub response_code: u32,
}

/// Object type decoded from a TPMT_PUBLIC area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpmPublicObjectType {
    Rsa,
    Ecc,
    KeyedHash,
    SymCipher,
    Unknown(u16),
}

/// Name (digest) algorithm identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpmNameAlgorithm {
    Sha1,
    Sha256,
    Sha384,
    Sha512,
    Null,
    Unknown(u16),
}

/// ECC curve identifier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpmEccCurve {
    NistP256,
    NistP384,
    Curve25519,
    Unknown(u16),
}

/// Decoded TPMT_PUBLIC area contents.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmPublicAreaInfo {
    pub object_type: TpmPublicObjectType,
    pub name_algorithm: TpmNameAlgorithm,
    pub object_attributes: u32,
    pub auth_policy: Vec<u8>,
    pub parameters: Vec<u8>,
    pub unique: Vec<u8>,
    pub curve: Option<TpmEccCurve>,
    pub key_bits: Option<u16>,
}

/// TPM2_ReadPublic response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmReadPublicInfo {
    pub public_area: TpmPublicAreaInfo,
    pub public_area_raw: Vec<u8>,
    pub name: Vec<u8>,
    pub qualified_name: Vec<u8>,
}

/// TPM signature scheme descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmSignatureScheme {
    pub scheme: u16,
    pub hash_algorithm: Option<TpmNameAlgorithm>,
}

/// Parameters for TPM2_Sign command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmSignCommandParams {
    pub key_handle: TpmHandle,
    pub digest: Vec<u8>,
    pub scheme: TpmSignatureScheme,
    pub validation_ticket_tag: u16,
    pub validation_ticket_hierarchy: u32,
    pub validation_digest: Vec<u8>,
}

/// Parameters for TPM2_Hash command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmHashParams {
    pub data: Vec<u8>,
    pub hash_algorithm: TpmNameAlgorithm,
    pub hierarchy: u32,
}

/// TPM2_Hash response validation ticket.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmHashCheckTicket {
    pub tag: u16,
    pub hierarchy: u32,
    pub digest: Vec<u8>,
}

/// TPM2_Hash response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmHashResponse {
    pub digest: Vec<u8>,
    pub validation: TpmHashCheckTicket,
}

/// Auth-value authorization session data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmAuthValueSession {
    pub auth_value: Vec<u8>,
    pub session_attributes: u8,
}

/// Session type for TPM2_StartAuthSession.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpmSessionType {
    Hmac = 0x00,
    Policy = 0x01,
    Trial = 0x03,
}

/// Symmetric definition (always Null for policy sessions).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TpmSymmetricDefinition {
    Null,
}

/// Parameters for TPM2_StartAuthSession.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmStartAuthSessionParams {
    pub tpm_key: TpmHandle,
    pub bind: TpmHandle,
    pub nonce_caller: Vec<u8>,
    pub session_type: TpmSessionType,
    pub symmetric: TpmSymmetricDefinition,
    pub auth_hash: TpmNameAlgorithm,
}

/// Active policy session handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TpmPolicySession {
    pub session_handle: u32,
    pub session_attributes: u8,
}

/// PCR selection descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmPcrSelection {
    pub hash_algorithm: TpmNameAlgorithm,
    pub pcrs: Vec<u8>,
}

/// TPM2_PolicyPCR parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmPolicyPcrParams {
    pub session: TpmPolicySession,
    pub pcr_digest: Vec<u8>,
    pub selection: TpmPcrSelection,
}

/// TPM2_PolicyAuthorize parameters.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmPolicyAuthorizeParams {
    pub session: TpmPolicySession,
    pub approved_policy: Vec<u8>,
    pub policy_ref: Vec<u8>,
    pub key_sign_name: Vec<u8>,
    pub check_ticket_hierarchy: u32,
    pub check_ticket_digest: Vec<u8>,
}

/// Generic TPM auth command area.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmAuthCommand {
    pub session_handle: u32,
    pub nonce: Vec<u8>,
    pub session_attributes: u8,
    pub hmac: Vec<u8>,
}

/// TPM2_StartAuthSession response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmStartAuthSessionResponse {
    pub session_handle: u32,
    pub nonce_tpm: Vec<u8>,
}

/// Parsed TPM2_Sign response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TpmParsedSignature {
    Rsa {
        scheme: u16,
        hash_algorithm: TpmNameAlgorithm,
        signature: Vec<u8>,
    },
    Ecc {
        scheme: u16,
        hash_algorithm: TpmNameAlgorithm,
        r: Vec<u8>,
        s: Vec<u8>,
    },
    Opaque {
        scheme: u16,
        bytes: Vec<u8>,
    },
}

/// Error type for TPM operations.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TpmError {
    Provider(String),
    UnsupportedAlgorithm(TpmSignatureAlgorithm),
    UnsupportedPublicArea,
    Io(String),
    Protocol(String),
    TpmResponseCode(u32),
}

impl core::fmt::Display for TpmError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Provider(msg) => f.write_str(msg),
            Self::UnsupportedAlgorithm(algorithm) => {
                write!(f, "unsupported TPM signature algorithm: {algorithm:?}")
            }
            Self::UnsupportedPublicArea => f.write_str("unsupported TPM public area"),
            Self::Io(msg) => write!(f, "TPM I/O error: {msg}"),
            Self::Protocol(msg) => write!(f, "TPM protocol error: {msg}"),
            Self::TpmResponseCode(code) => write!(f, "TPM returned response code 0x{code:08x}"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for TpmError {}

/// Authorization mode for signing operations.
#[derive(Clone, Debug)]
pub enum TpmAuthorizationMode {
    None,
    AuthValue(TpmAuthValueSession),
    Policy(TpmPolicySessionRunner),
}

/// Orchestrates policy session setup and authorized signing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TpmPolicySessionRunner {
    pub nonce_caller: Vec<u8>,
    pub auth_hash: TpmNameAlgorithm,
    pub session_attributes: u8,
    pub pcr_policy: Option<TpmPcrSelection>,
    pub pcr_digest: Vec<u8>,
    pub authorize_policy: Option<TpmPolicyAuthorizeParams>,
}

impl Default for TpmPolicySessionRunner {
    fn default() -> Self {
        Self {
            nonce_caller: vec![0u8; 16],
            auth_hash: TpmNameAlgorithm::Sha256,
            session_attributes: 0,
            pcr_policy: None,
            pcr_digest: Vec::new(),
            authorize_policy: None,
        }
    }
}

/// Result of creating and persisting a TPM key.
#[derive(Clone, Debug)]
pub struct TpmProvisionedKey {
    pub persistent_handle: u32,
    pub public_key_bytes: Vec<u8>,
    pub name: Vec<u8>,
}
