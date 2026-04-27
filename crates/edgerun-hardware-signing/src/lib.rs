//! Hardware signing — universal mesh identity and secure hardware signing.
//!
//! Each hardware backend is an optional feature:
//! - `tpm` — TPM 2.0
//! - `android-keystore` — Android Keystore
//! - `yubikey` — YubiKey PIV
//! - `all-hardware` — enables all backends

#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

use edgerun_biometrics::{BiometricAssuranceStrength, BiometricState};
use edgerun_core::crypto::signature_input;
use edgerun_core::prelude::v1::*;

// ---------------------------------------------------------------------------
// Conditional backend modules
// ---------------------------------------------------------------------------

#[cfg(feature = "tpm")]
pub mod tpm;

#[cfg(feature = "android-keystore")]
pub mod android_keystore;

#[cfg(feature = "yubikey")]
pub mod yubikey;

#[cfg(feature = "tpm")]
pub use tpm::{sign_record_with_tpm_provider, TpmHardwareKeyAdapter};

#[cfg(feature = "android-keystore")]
pub use android_keystore::{
    sign_record_with_android_keystore_provider, AndroidKeystoreHardwareKeyAdapter,
};

#[cfg(feature = "yubikey")]
pub use yubikey::{sign_record_with_yubikey_provider, YubiKeyHardwareKeyAdapter};

// ---------------------------------------------------------------------------
// Mesh identity constants — ECDSA P256 is the universal algorithm
// ---------------------------------------------------------------------------

/// Size of an uncompressed P-256 public key (x || y, no 0x04 prefix).
pub const MESH_PUBLIC_KEY_LENGTH: usize = 64;

/// Size of an ECDSA P-256 signature (r || s, 32 bytes each).
pub const MESH_SIGNATURE_LENGTH: usize = 64;

/// A node's identity in the mesh.
///
/// This is the raw uncompressed ECDSA P-256 public key (64 bytes: x || y).
/// The same format is produced by TPMs, YubiKeys, Android Keystore,
/// and iOS Secure Enclave — making it the universal hardware-backed identity.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeID(pub [u8; MESH_PUBLIC_KEY_LENGTH]);

impl NodeID {
    /// Short hex display for logging/UI (first 8 hex chars).
    pub fn short(&self) -> String {
        edgerun_core::util::bytes_to_hex_prefixed(&self.0[..4])
    }

    /// Full hex representation.
    pub fn to_hex(&self) -> String {
        edgerun_core::util::bytes_to_hex_prefixed(&self.0)
    }
}

impl core::fmt::Debug for NodeID {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "NodeID({})", self.short())
    }
}

impl AsRef<[u8]> for NodeID {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

// ---------------------------------------------------------------------------
// Hardware signature algorithm — existing types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HardwareProviderKind {
    Tpm,
    AndroidKeystore,
    YubiKey,
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HardwareSignatureAlgorithm {
    RsaPkcs1v15Sha256,
    RsaPssSha256,
    EcdsaP256Sha256,
    EcdsaP384Sha384,
    EcSchnorr,
    Eddsa,
    Opaque(String),
}

impl HardwareSignatureAlgorithm {
    /// Returns `true` if this is ECDSA P-256 with SHA-256 — the universal
    /// mesh algorithm supported by every secure enclave.
    #[must_use]
    pub fn is_ecdsa_p256(&self) -> bool {
        matches!(self, Self::EcdsaP256Sha256)
    }
}

/// The canonical mesh signing algorithm — ECDSA P-256 SHA-256.
/// Every secure hardware provider supports this, producing a 64-byte
/// public key (NodeID) and 64-byte signature.
pub const MESH_ALGORITHM: HardwareSignatureAlgorithm = HardwareSignatureAlgorithm::EcdsaP256Sha256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum HardwareAssuranceStrength {
    Unknown,
    Software,
    IsolatedHardware,
    StrongBox,
    Certified,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HardwareAssuranceLevel {
    Software,
    IsolatedHardware,
    StrongBox,
    Certified(String),
    Unknown,
}

impl HardwareAssuranceLevel {
    pub fn strength(&self) -> HardwareAssuranceStrength {
        match self {
            Self::Unknown => HardwareAssuranceStrength::Unknown,
            Self::Software => HardwareAssuranceStrength::Software,
            Self::IsolatedHardware => HardwareAssuranceStrength::IsolatedHardware,
            Self::StrongBox => HardwareAssuranceStrength::StrongBox,
            Self::Certified(_) => HardwareAssuranceStrength::Certified,
        }
    }

    pub fn is_at_least(&self, minimum: HardwareAssuranceStrength) -> bool {
        self.strength() >= minimum
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HardwareKeyInfo {
    pub provider: HardwareProviderKind,
    pub key_name: String,
    pub algorithm: HardwareSignatureAlgorithm,
    pub public_key: Vec<u8>,
    pub attestation: Vec<Vec<u8>>,
    pub assurance_level: HardwareAssuranceLevel,
    pub biometric_state: BiometricState,
}

impl HardwareKeyInfo {
    /// Extracts the `NodeID` from this key's public key.
    ///
    /// Returns `None` if the key is not ECDSA P-256 or the public key
    /// is not exactly 64 bytes (uncompressed x || y, no 0x04 prefix).
    #[must_use]
    pub fn node_id(&self) -> Option<NodeID> {
        if !self.algorithm.is_ecdsa_p256() {
            return None;
        }
        if self.public_key.len() != MESH_PUBLIC_KEY_LENGTH {
            return None;
        }
        let mut bytes = [0u8; MESH_PUBLIC_KEY_LENGTH];
        bytes.copy_from_slice(&self.public_key);
        Some(NodeID(bytes))
    }

    /// Returns `true` if this key is suitable for mesh use
    /// (ECDSA P-256 with a valid 64-byte public key).
    #[must_use]
    pub fn is_mesh_capable(&self) -> bool {
        self.node_id().is_some()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct HardwareValidationRequirements {
    pub allowed_providers: Vec<HardwareProviderKind>,
    pub allowed_algorithms: Vec<HardwareSignatureAlgorithm>,
    pub minimum_assurance: Option<HardwareAssuranceStrength>,
    pub require_attestation: bool,
    pub require_public_key: bool,
    pub minimum_biometric_assurance: Option<BiometricAssuranceStrength>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HardwareValidationIssue {
    ProviderNotAllowed(HardwareProviderKind),
    AlgorithmNotAllowed(HardwareSignatureAlgorithm),
    AssuranceTooWeak {
        actual: HardwareAssuranceLevel,
        required: HardwareAssuranceStrength,
    },
    BiometricAssuranceTooWeak {
        actual: BiometricAssuranceStrength,
        required: BiometricAssuranceStrength,
    },
    MissingAttestation,
    MissingPublicKey,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HardwareSigningError {
    #[cfg(feature = "tpm")]
    Tpm(edgerun_tpm::TpmError),
    #[cfg(feature = "android-keystore")]
    AndroidKeystore(edgerun_android_keystore::AndroidKeystoreError),
    #[cfg(feature = "yubikey")]
    YubiKey(edgerun_yubikey::YubiKeyError),
    UnsupportedAlgorithm(HardwareSignatureAlgorithm),
    Validation(HardwareValidationIssue),
    Provider(String),
}

impl core::fmt::Display for HardwareSigningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            #[cfg(feature = "tpm")]
            Self::Tpm(err) => write!(f, "TPM error: {err}"),
            #[cfg(feature = "android-keystore")]
            Self::AndroidKeystore(err) => write!(f, "Android Keystore error: {err}"),
            #[cfg(feature = "yubikey")]
            Self::YubiKey(err) => write!(f, "YubiKey error: {err}"),
            Self::UnsupportedAlgorithm(algorithm) => {
                write!(f, "unsupported hardware signature algorithm: {algorithm:?}")
            }
            Self::Validation(issue) => write!(f, "hardware validation failed: {issue:?}"),
            Self::Provider(msg) => f.write_str(msg),
        }
    }
}

impl core::error::Error for HardwareSigningError {}

#[cfg(feature = "tpm")]
impl From<edgerun_tpm::TpmError> for HardwareSigningError {
    fn from(value: edgerun_tpm::TpmError) -> Self {
        Self::Tpm(value)
    }
}

#[cfg(feature = "android-keystore")]
impl From<edgerun_android_keystore::AndroidKeystoreError> for HardwareSigningError {
    fn from(value: edgerun_android_keystore::AndroidKeystoreError) -> Self {
        Self::AndroidKeystore(value)
    }
}

#[cfg(feature = "yubikey")]
impl From<edgerun_yubikey::YubiKeyError> for HardwareSigningError {
    fn from(value: edgerun_yubikey::YubiKeyError) -> Self {
        Self::YubiKey(value)
    }
}

pub trait HardwareSigningKey {
    fn key_info(&self) -> Result<HardwareKeyInfo, HardwareSigningError>;
    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, HardwareSigningError>;
}

// ---------------------------------------------------------------------------
// Mesh signer — digest-only interface for the mesh daemon
// ---------------------------------------------------------------------------

/// A signer that the mesh daemon uses to sign outbound frames.
///
/// The private key **never leaves secure hardware**.  The daemon only:
/// 1. Holds the public key (`NodeID`)
/// 2. Hashes the frame preimage with SHA-256
/// 3. Sends the 32-byte digest to hardware → gets back 64-byte signature
pub trait MeshSigner {
    /// This node's identity (ECDSA P-256 public key, 64 bytes: x || y).
    fn node_id(&self) -> NodeID;

    /// Signs a pre-hashed 32-byte SHA-256 digest.
    /// Returns exactly 64 bytes: r (32 bytes) || s (32 bytes).
    ///
    /// **Deprecated:** Use `sign_record` instead to get domain-separated signatures.
    /// This method signs the raw digest without any domain prefix, which means
    /// signatures could potentially be replayed across different record types.
    fn sign_digest(
        &self,
        digest: &[u8; 32],
    ) -> Result<[u8; MESH_SIGNATURE_LENGTH], HardwareSigningError>;

    /// Signs a record with domain separation (spec §17.9, §17.11).
    ///
    /// Computes the family hash-domain-separated record hash, then
    /// signs `sig_domain_tag || 0x00 || record_hash`.
    /// This prevents signature replay across different record types.
    ///
    /// Per spec §17.11: `sig_input = sig_domain_tag || 0x00 || record_hash_bytes`,
    /// then `signature = ECDSA_P256_SHA256_sign(private_key, sig_input)`.
    ///
    /// # Arguments
    /// * `sig_domain_tag` - Domain tag for signing (e.g., `SIG_DOMAIN_COMMAND_ENVELOPE`)
    /// * `canonical_bytes` - The canonical encoding of the record to sign
    fn sign_record(
        &self,
        sig_domain_tag: &str,
        canonical_bytes: &[u8],
    ) -> Result<[u8; MESH_SIGNATURE_LENGTH], HardwareSigningError> {
        let hash_domain = edgerun_core::crypto::hash_domain_for_signature_domain(sig_domain_tag)
            .unwrap_or(sig_domain_tag);
        let record_hash = edgerun_core::crypto::record_hash(hash_domain, canonical_bytes);
        let sig_input = edgerun_core::crypto::signature_input(sig_domain_tag, &record_hash);
        self.sign_message_var(&sig_input)
    }

    /// Signs a message of variable length.
    ///
    /// For software signers, this signs the message directly using ECDSA P-256
    /// with SHA-256 (per spec §17.11).
    ///
    /// For hardware signers that require fixed-size input, this hashes the
    /// message to 32 bytes first (which means hardware signatures differ from
    /// software signatures - this is a hardware limitation).
    ///
    /// Implementations that support variable-length signing SHOULD override this
    /// method for spec compliance.
    fn sign_message_var(
        &self,
        message: &[u8],
    ) -> Result<[u8; MESH_SIGNATURE_LENGTH], HardwareSigningError> {
        let mut digest = [0u8; 32];
        digest.copy_from_slice(&edgerun_core::crypto::sha256(message));
        self.sign_digest(&digest)
    }

    /// Returns self as `Any` for downcasting (used by TCP task cloning).
    fn as_any(&self) -> &dyn core::any::Any {
        &()
    }
}

/// Wraps any `HardwareSigningKey` as a `MeshSigner`.
/// The hardware performs all signing internally — the daemon only sends
/// the SHA-256 digest of the frame preimage.
pub struct HardwareMeshSigner<K: HardwareSigningKey> {
    key: K,
    node_id: NodeID,
}

impl<K: HardwareSigningKey> HardwareMeshSigner<K> {
    pub fn new(key: K) -> Result<Self, HardwareSigningError> {
        let info = key.key_info()?;
        let node_id = info.node_id().ok_or(HardwareSigningError::Provider(
            "hardware key is not an ECDSA P-256 key".into(),
        ))?;
        Ok(Self { key, node_id })
    }

    pub fn key(&self) -> &K {
        &self.key
    }
}

impl<K: HardwareSigningKey> MeshSigner for HardwareMeshSigner<K> {
    fn node_id(&self) -> NodeID {
        self.node_id
    }

    fn sign_digest(
        &self,
        digest: &[u8; 32],
    ) -> Result<[u8; MESH_SIGNATURE_LENGTH], HardwareSigningError> {
        let sig = self.key.sign_message(digest)?;
        if sig.len() != MESH_SIGNATURE_LENGTH {
            return Err(HardwareSigningError::Provider(format!(
                "hardware returned {}-byte signature, expected {}",
                sig.len(),
                MESH_SIGNATURE_LENGTH
            )));
        }
        let mut out = [0u8; MESH_SIGNATURE_LENGTH];
        out.copy_from_slice(&sig);
        Ok(out)
    }

    fn sign_message_var(
        &self,
        message: &[u8],
    ) -> Result<[u8; MESH_SIGNATURE_LENGTH], HardwareSigningError> {
        // Hardware supports variable-length signing via sign_message
        let sig = self.key.sign_message(message)?;
        if sig.len() != MESH_SIGNATURE_LENGTH {
            return Err(HardwareSigningError::Provider(format!(
                "hardware returned {}-byte signature, expected {}",
                sig.len(),
                MESH_SIGNATURE_LENGTH
            )));
        }
        let mut out = [0u8; MESH_SIGNATURE_LENGTH];
        out.copy_from_slice(&sig);
        Ok(out)
    }
}

pub fn signature_input_for_record(sig_domain_tag: &str, record_hash: &[u8]) -> Vec<u8> {
    signature_input(sig_domain_tag, record_hash)
}

pub fn validate_hardware_key_info(
    key_info: &HardwareKeyInfo,
    requirements: &HardwareValidationRequirements,
) -> Result<(), HardwareSigningError> {
    if !requirements.allowed_providers.is_empty()
        && !requirements
            .allowed_providers
            .iter()
            .any(|p| p == &key_info.provider)
    {
        return Err(HardwareSigningError::Validation(
            HardwareValidationIssue::ProviderNotAllowed(key_info.provider.clone()),
        ));
    }

    if !requirements.allowed_algorithms.is_empty()
        && !requirements
            .allowed_algorithms
            .iter()
            .any(|a| a == &key_info.algorithm)
    {
        return Err(HardwareSigningError::Validation(
            HardwareValidationIssue::AlgorithmNotAllowed(key_info.algorithm.clone()),
        ));
    }

    if let Some(minimum) = requirements.minimum_assurance {
        if !key_info.assurance_level.is_at_least(minimum) {
            return Err(HardwareSigningError::Validation(
                HardwareValidationIssue::AssuranceTooWeak {
                    actual: key_info.assurance_level.clone(),
                    required: minimum,
                },
            ));
        }
    }

    if requirements.require_attestation && key_info.attestation.is_empty() {
        return Err(HardwareSigningError::Validation(
            HardwareValidationIssue::MissingAttestation,
        ));
    }

    if requirements.require_public_key && key_info.public_key.is_empty() {
        return Err(HardwareSigningError::Validation(
            HardwareValidationIssue::MissingPublicKey,
        ));
    }

    if let Some(minimum) = requirements.minimum_biometric_assurance {
        let actual = key_info.biometric_state.assurance_strength();
        if actual < minimum {
            return Err(HardwareSigningError::Validation(
                HardwareValidationIssue::BiometricAssuranceTooWeak {
                    actual,
                    required: minimum,
                },
            ));
        }
    }

    Ok(())
}

pub fn sign_record_with_hardware(
    key: &dyn HardwareSigningKey,
    sig_domain_tag: &str,
    record_hash: &[u8],
) -> Result<Vec<u8>, HardwareSigningError> {
    key.sign_message(&signature_input_for_record(sig_domain_tag, record_hash))
}

pub fn sign_record_with_hardware_checked(
    key: &dyn HardwareSigningKey,
    requirements: &HardwareValidationRequirements,
    sig_domain_tag: &str,
    record_hash: &[u8],
) -> Result<Vec<u8>, HardwareSigningError> {
    let key_info = key.key_info()?;
    validate_hardware_key_info(&key_info, requirements)?;
    key.sign_message(&signature_input_for_record(sig_domain_tag, record_hash))
}

// ---------------------------------------------------------------------------
// Tests (core only — backend tests live in their own modules)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use core::hash::{Hash, Hasher};

    use super::*;

    // A generic fake key that implements HardwareSigningKey directly
    struct FakeMeshKey {
        public_key: Vec<u8>,
        algorithm: HardwareSignatureAlgorithm,
        assurance: HardwareAssuranceLevel,
    }

    impl FakeMeshKey {
        fn ecdsa_p256() -> Self {
            Self {
                public_key: {
                    let mut pk = [0u8; 64].to_vec();
                    pk[0] = 0x04;
                    pk[1] = 0xAB;
                    pk
                },
                algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
                assurance: HardwareAssuranceLevel::IsolatedHardware,
            }
        }
    }

    impl HardwareSigningKey for FakeMeshKey {
        fn key_info(&self) -> Result<HardwareKeyInfo, HardwareSigningError> {
            Ok(HardwareKeyInfo {
                provider: HardwareProviderKind::Tpm,
                key_name: "fake-key".into(),
                algorithm: self.algorithm.clone(),
                public_key: self.public_key.clone(),
                attestation: vec![],
                assurance_level: self.assurance.clone(),
                biometric_state: BiometricState::default(),
            })
        }
        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, HardwareSigningError> {
            let mut sig = [0u8; 64];
            let len = message.len().min(32);
            sig[..len].copy_from_slice(&message[..len]);
            let inv: Vec<u8> = message.iter().map(|b| !b).take(32).collect();
            sig[32..32 + len].copy_from_slice(&inv[..len]);
            Ok(sig.to_vec())
        }
    }

    struct FakeMeshKeyWithSig {
        sig_len: usize,
    }

    impl HardwareSigningKey for FakeMeshKeyWithSig {
        fn key_info(&self) -> Result<HardwareKeyInfo, HardwareSigningError> {
            let mut pk = [0u8; 64];
            pk[0] = 0xAA;
            Ok(HardwareKeyInfo {
                provider: HardwareProviderKind::Tpm,
                key_name: "fake-sig-key".into(),
                algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
                public_key: pk.to_vec(),
                attestation: vec![],
                assurance_level: HardwareAssuranceLevel::IsolatedHardware,
                biometric_state: BiometricState::default(),
            })
        }
        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, HardwareSigningError> {
            Ok(vec![0u8; self.sig_len])
        }
    }

    struct FakeFailingKey {
        error_msg: String,
    }

    impl HardwareSigningKey for FakeFailingKey {
        fn key_info(&self) -> Result<HardwareKeyInfo, HardwareSigningError> {
            Ok(HardwareKeyInfo {
                provider: HardwareProviderKind::Tpm,
                key_name: "failing-key".into(),
                algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
                public_key: vec![0xBB; 64],
                attestation: vec![],
                assurance_level: HardwareAssuranceLevel::IsolatedHardware,
                biometric_state: BiometricState::default(),
            })
        }
        fn sign_message(&self, _message: &[u8]) -> Result<Vec<u8>, HardwareSigningError> {
            Err(HardwareSigningError::Provider(self.error_msg.clone()))
        }
    }

    // NodeID tests
    #[test]
    fn node_id_short_display() {
        let mut bytes = [0u8; 64];
        bytes[0] = 0xaa;
        bytes[1] = 0xbb;
        bytes[2] = 0xcc;
        bytes[3] = 0xdd;
        let id = NodeID(bytes);
        assert_eq!(id.short(), "0xaabbccdd");
        assert_eq!(id.to_hex().len(), 130);
    }

    #[test]
    fn node_id_to_hex_produces_full_hex() {
        let mut bytes = [0u8; 64];
        bytes[0] = 0x01;
        bytes[1] = 0x02;
        bytes[63] = 0xFF;
        let id = NodeID(bytes);
        let hex = id.to_hex();
        assert!(hex.starts_with("0x0102"));
        assert!(hex.ends_with("ff"));
    }

    #[test]
    fn node_id_debug_format() {
        let mut bytes = [0u8; 64];
        bytes[0] = 0xDE;
        bytes[1] = 0xAD;
        bytes[2] = 0xBE;
        bytes[3] = 0xEF;
        let id = NodeID(bytes);
        let debug = format!("{:?}", id);
        assert_eq!(debug, "NodeID(0xdeadbeef)");
    }

    #[test]
    fn node_id_as_ref() {
        let mut bytes = [0u8; 64];
        bytes[0] = 0x42;
        let id = NodeID(bytes);
        assert_eq!(id.as_ref()[0], 0x42);
    }

    #[test]
    fn node_id_copy_and_equality() {
        let bytes = [1u8; 64];
        let id1 = NodeID(bytes);
        let id2 = id1;
        assert_eq!(id1, id2);
    }

    #[test]
    fn node_id_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        let bytes = [42u8; 64];
        let id1 = NodeID(bytes);
        let id2 = NodeID(bytes);
        let mut h1 = DefaultHasher::new();
        id1.hash(&mut h1);
        let mut h2 = DefaultHasher::new();
        id2.hash(&mut h2);
        assert_eq!(h1.finish(), h2.finish());
    }

    #[test]
    fn node_id_all_zeros() {
        let bytes = [0u8; 64];
        let id = NodeID(bytes);
        assert_eq!(id.short(), "0x00000000");
        assert_eq!(id.to_hex(), format!("0x{}", "0".repeat(128)));
    }

    #[test]
    fn node_id_all_ones() {
        let bytes = [0xFFu8; 64];
        let id = NodeID(bytes);
        assert_eq!(id.short(), "0xffffffff");
    }

    // HardwareSignatureAlgorithm tests
    #[test]
    fn mesh_algorithm_is_ecdsa_p256() {
        assert!(MESH_ALGORITHM.is_ecdsa_p256());
    }

    #[test]
    fn is_ecdsa_p256_false_for_other_algorithms() {
        assert!(!HardwareSignatureAlgorithm::RsaPkcs1v15Sha256.is_ecdsa_p256());
        assert!(!HardwareSignatureAlgorithm::RsaPssSha256.is_ecdsa_p256());
        assert!(!HardwareSignatureAlgorithm::EcdsaP384Sha384.is_ecdsa_p256());
        assert!(!HardwareSignatureAlgorithm::EcSchnorr.is_ecdsa_p256());
        assert!(!HardwareSignatureAlgorithm::Eddsa.is_ecdsa_p256());
        assert!(!HardwareSignatureAlgorithm::Opaque("custom".into()).is_ecdsa_p256());
    }

    #[test]
    fn algorithm_clone_and_equality() {
        let algo = HardwareSignatureAlgorithm::EcdsaP256Sha256;
        assert_eq!(algo, algo.clone());
    }

    #[test]
    fn opaque_algorithm_variant() {
        let opaque = HardwareSignatureAlgorithm::Opaque("my-custom-algo".into());
        assert!(!opaque.is_ecdsa_p256());
        assert_eq!(
            opaque,
            HardwareSignatureAlgorithm::Opaque("my-custom-algo".into())
        );
        assert_ne!(opaque, HardwareSignatureAlgorithm::Eddsa);
    }

    // HardwareProviderKind tests
    #[test]
    fn provider_kind_clone_and_equality() {
        let tpm = HardwareProviderKind::Tpm;
        assert_eq!(tpm, HardwareProviderKind::Tpm);
        assert_ne!(tpm, HardwareProviderKind::AndroidKeystore);
        let other1 = HardwareProviderKind::Other("custom".into());
        let other2 = HardwareProviderKind::Other("custom".into());
        assert_eq!(other1, other2);
        assert_ne!(other1, HardwareProviderKind::Other("different".into()));
    }

    // HardwareAssuranceLevel and HardwareAssuranceStrength tests
    #[test]
    fn assurance_strength_ordering() {
        use HardwareAssuranceStrength as S;
        assert!(S::Unknown < S::Software);
        assert!(S::Software < S::IsolatedHardware);
        assert!(S::IsolatedHardware < S::StrongBox);
        assert!(S::StrongBox < S::Certified);
    }

    #[test]
    fn assurance_level_strength_mapping() {
        assert_eq!(
            HardwareAssuranceLevel::Unknown.strength(),
            HardwareAssuranceStrength::Unknown
        );
        assert_eq!(
            HardwareAssuranceLevel::Software.strength(),
            HardwareAssuranceStrength::Software
        );
        assert_eq!(
            HardwareAssuranceLevel::IsolatedHardware.strength(),
            HardwareAssuranceStrength::IsolatedHardware
        );
        assert_eq!(
            HardwareAssuranceLevel::StrongBox.strength(),
            HardwareAssuranceStrength::StrongBox
        );
    }

    #[test]
    fn assurance_level_is_at_least() {
        let level = HardwareAssuranceLevel::StrongBox;
        assert!(level.is_at_least(HardwareAssuranceStrength::StrongBox));
        assert!(level.is_at_least(HardwareAssuranceStrength::IsolatedHardware));
        assert!(level.is_at_least(HardwareAssuranceStrength::Software));
        assert!(!level.is_at_least(HardwareAssuranceStrength::Certified));
    }

    #[test]
    fn assurance_level_is_at_least_edge_cases() {
        let unknown = HardwareAssuranceLevel::Unknown;
        assert!(unknown.is_at_least(HardwareAssuranceStrength::Unknown));
        assert!(!unknown.is_at_least(HardwareAssuranceStrength::Software));
    }

    // HardwareKeyInfo tests
    #[test]
    fn node_id_from_ecdsa_p256_key() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "tpm-key".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![0x42u8; 64],
            attestation: vec![vec![9, 9]],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        let node_id = info.node_id().expect("should extract NodeID");
        assert_eq!(node_id.0.len(), 64);
        assert!(info.is_mesh_capable());
    }

    #[test]
    fn node_id_rejects_non_p256_algorithm() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "tpm-key".into(),
            algorithm: HardwareSignatureAlgorithm::Eddsa,
            public_key: vec![0x42u8; 64],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        assert!(info.node_id().is_none());
        assert!(!info.is_mesh_capable());
    }

    #[test]
    fn node_id_rejects_wrong_key_length() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "tpm-key".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![0x42u8; 96],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        assert!(info.node_id().is_none());
    }

    #[test]
    fn node_id_rejects_empty_public_key() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "tpm-key".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        assert!(info.node_id().is_none());
        assert!(!info.is_mesh_capable());
    }

    #[test]
    fn node_id_rejects_key_too_short() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "tpm-key".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![0x42u8; 32],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        assert!(info.node_id().is_none());
    }

    #[test]
    fn hardware_key_info_clone() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "key".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![1, 2],
            attestation: vec![vec![3]],
            assurance_level: HardwareAssuranceLevel::Unknown,
            biometric_state: BiometricState::default(),
        };
        let cloned = info.clone();
        assert_eq!(cloned.key_name, "key");
        assert_eq!(cloned.public_key, vec![1, 2]);
    }

    // HardwareValidationRequirements tests
    #[test]
    fn default_requirements_allow_everything() {
        let req = HardwareValidationRequirements::default();
        assert!(req.allowed_providers.is_empty());
        assert!(req.allowed_algorithms.is_empty());
        assert!(req.minimum_assurance.is_none());
        assert!(!req.require_attestation);
        assert!(!req.require_public_key);
    }

    // validate_hardware_key_info tests
    #[test]
    fn validate_accepts_default_requirements() {
        let key = FakeMeshKey::ecdsa_p256();
        let info = key.key_info().unwrap();
        validate_hardware_key_info(&info, &HardwareValidationRequirements::default()).unwrap();
    }

    #[test]
    fn validate_rejects_disallowed_provider() {
        let key = FakeMeshKey::ecdsa_p256();
        let info = key.key_info().unwrap();
        let req = HardwareValidationRequirements {
            allowed_providers: vec![HardwareProviderKind::YubiKey],
            ..Default::default()
        };
        assert!(validate_hardware_key_info(&info, &req).is_err());
    }

    #[test]
    fn validate_rejects_disallowed_algorithm() {
        let key = FakeMeshKey::ecdsa_p256();
        let info = key.key_info().unwrap();
        let req = HardwareValidationRequirements {
            allowed_algorithms: vec![HardwareSignatureAlgorithm::Eddsa],
            ..Default::default()
        };
        assert!(validate_hardware_key_info(&info, &req).is_err());
    }

    #[test]
    fn validate_rejects_weak_assurance() {
        let key = FakeMeshKey::ecdsa_p256();
        let info = key.key_info().unwrap();
        let req = HardwareValidationRequirements {
            minimum_assurance: Some(HardwareAssuranceStrength::Certified),
            ..Default::default()
        };
        assert!(validate_hardware_key_info(&info, &req).is_err());
    }

    #[test]
    fn validate_rejects_missing_attestation() {
        let key = FakeMeshKey::ecdsa_p256();
        let info = key.key_info().unwrap();
        let req = HardwareValidationRequirements {
            require_attestation: true,
            ..Default::default()
        };
        assert!(validate_hardware_key_info(&info, &req).is_err());
    }

    #[test]
    fn validate_rejects_missing_public_key() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "key".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        let req = HardwareValidationRequirements {
            require_public_key: true,
            ..Default::default()
        };
        assert!(validate_hardware_key_info(&info, &req).is_err());
    }

    // HardwareMeshSigner tests
    #[test]
    fn hardware_mesh_signer_creates() {
        let key = FakeMeshKey::ecdsa_p256();
        let signer = HardwareMeshSigner::new(key).unwrap();
        assert_eq!(signer.node_id().0.len(), 64);
    }

    #[test]
    fn hardware_mesh_signer_signs() {
        let key = FakeMeshKey::ecdsa_p256();
        let signer = HardwareMeshSigner::new(key).unwrap();
        let digest = [0xABu8; 32];
        let sig = signer.sign_digest(&digest).unwrap();
        assert_eq!(sig.len(), MESH_SIGNATURE_LENGTH);
        // First half should match the digest (our fake copies message)
        assert_eq!(&sig[..32], &digest[..]);
    }

    #[test]
    fn hardware_mesh_signer_rejects_wrong_sig_len() {
        let key = FakeMeshKeyWithSig { sig_len: 32 };
        let signer = HardwareMeshSigner::new(key).unwrap();
        let digest = [0u8; 32];
        let err = signer.sign_digest(&digest).unwrap_err();
        assert!(matches!(err, HardwareSigningError::Provider(_)));
    }

    #[test]
    fn hardware_mesh_signer_key_method() {
        let key = FakeMeshKey::ecdsa_p256();
        let signer = HardwareMeshSigner::new(key).unwrap();
        let _: &FakeMeshKey = signer.key();
    }

    #[test]
    fn mesh_signer_trait_methods() {
        let key = FakeMeshKey::ecdsa_p256();
        let signer = HardwareMeshSigner::new(key).unwrap();
        let node_id = signer.node_id();
        assert_eq!(node_id.0.len(), 64);
        let _ = signer.as_any();
    }

    // sign_record_with_hardware tests
    #[test]
    fn sign_record_with_hardware_success() {
        let key = FakeMeshKey::ecdsa_p256();
        let sig = sign_record_with_hardware(&key, "test:v0:sig", b"hash").unwrap();
        assert_eq!(sig.len(), MESH_SIGNATURE_LENGTH);
    }

    #[test]
    fn sign_record_with_hardware_checked_success() {
        let key = FakeMeshKey::ecdsa_p256();
        let req = HardwareValidationRequirements::default();
        let sig = sign_record_with_hardware_checked(&key, &req, "test:v0:sig", b"hash").unwrap();
        assert_eq!(sig.len(), MESH_SIGNATURE_LENGTH);
    }

    #[test]
    fn sign_record_with_hardware_checked_fails_validation() {
        let key = FakeMeshKey::ecdsa_p256();
        let req = HardwareValidationRequirements {
            allowed_algorithms: vec![HardwareSignatureAlgorithm::Eddsa],
            ..Default::default()
        };
        assert!(sign_record_with_hardware_checked(&key, &req, "test:v0:sig", b"hash").is_err());
    }

    // signature_input_for_record tests
    #[test]
    fn signature_input_for_record_produces_tagged_input() {
        let input = signature_input_for_record("edgerun:v0:sig:test", b"hash");
        assert!(input.starts_with(b"edgerun:v0:sig:test"));
        assert_eq!(input[b"edgerun:v0:sig:test".len()], 0);
    }

    // Failing key tests
    #[test]
    fn failing_key_propagates_error() {
        let key = FakeFailingKey {
            error_msg: "broken".into(),
        };
        let err = sign_record_with_hardware(&key, "test:v0:sig", b"hash").unwrap_err();
        assert!(matches!(err, HardwareSigningError::Provider(ref m) if m == "broken"));
    }

    #[test]
    fn hardware_signing_error_display() {
        let err = HardwareSigningError::Provider("test error".into());
        let msg = format!("{}", err);
        assert!(msg.contains("test error"));

        let err2 = HardwareSigningError::UnsupportedAlgorithm(HardwareSignatureAlgorithm::Eddsa);
        let msg2 = format!("{}", err2);
        assert!(msg2.contains("Eddsa"));

        let err3 = HardwareSigningError::Validation(HardwareValidationIssue::MissingAttestation);
        let msg3 = format!("{}", err3);
        assert!(msg3.contains("validation failed"));
    }

    #[test]
    fn hardware_signing_error_is_std_error() {
        let err = HardwareSigningError::Provider("x".into());
        let _: &dyn std::error::Error = &err;
    }
}
