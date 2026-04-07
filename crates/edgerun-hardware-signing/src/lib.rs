use edgerun_android_keystore::{
    sign_record_with_keystore_checked, AndroidKeystoreAssuranceLevel, AndroidKeystoreError,
    AndroidKeystoreSignatureAlgorithm, AndroidKeystoreSigningKey,
};
use edgerun_biometrics::{BiometricAssuranceStrength, BiometricState};
use edgerun_core::crypto::signature_input;
use edgerun_tpm::{
    sign_record_with_tpm_checked, TpmAssuranceLevel, TpmError, TpmSignatureAlgorithm, TpmSigningKey,
};
use edgerun_yubikey::{
    sign_record_with_yubikey_checked, YubiKeyAssuranceLevel, YubiKeyError,
    YubiKeySignatureAlgorithm, YubiKeySigningKey,
};

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
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
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
    Tpm(TpmError),
    AndroidKeystore(AndroidKeystoreError),
    YubiKey(YubiKeyError),
    UnsupportedAlgorithm(HardwareSignatureAlgorithm),
    Validation(HardwareValidationIssue),
    Provider(String),
}

impl core::fmt::Display for HardwareSigningError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Tpm(err) => write!(f, "TPM error: {err}"),
            Self::AndroidKeystore(err) => write!(f, "Android Keystore error: {err}"),
            Self::YubiKey(err) => write!(f, "YubiKey error: {err}"),
            Self::UnsupportedAlgorithm(algorithm) => {
                write!(f, "unsupported hardware signature algorithm: {algorithm:?}")
            }
            Self::Validation(issue) => write!(f, "hardware validation failed: {issue:?}"),
            Self::Provider(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for HardwareSigningError {}

impl From<TpmError> for HardwareSigningError {
    fn from(value: TpmError) -> Self {
        Self::Tpm(value)
    }
}

impl From<AndroidKeystoreError> for HardwareSigningError {
    fn from(value: AndroidKeystoreError) -> Self {
        Self::AndroidKeystore(value)
    }
}

impl From<YubiKeyError> for HardwareSigningError {
    fn from(value: YubiKeyError) -> Self {
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
    fn sign_digest(&self, digest: &[u8; 32]) -> Result<[u8; MESH_SIGNATURE_LENGTH], HardwareSigningError>;

    /// Returns self as `Any` for downcasting (used by TCP task cloning).
    fn as_any(&self) -> &dyn std::any::Any {
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

    fn sign_digest(&self, digest: &[u8; 32]) -> Result<[u8; MESH_SIGNATURE_LENGTH], HardwareSigningError> {
        // The hardware key signs the 32-byte digest directly.
        // For ECDSA P-256, the signature is exactly 64 bytes (r || s).
        let sig = self.key.sign_message(digest)?;
        if sig.len() != MESH_SIGNATURE_LENGTH {
            return Err(HardwareSigningError::Provider(format!(
                "hardware returned {}-byte signature, expected {}",
                sig.len(), MESH_SIGNATURE_LENGTH
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
            .any(|provider| provider == &key_info.provider)
    {
        return Err(HardwareSigningError::Validation(
            HardwareValidationIssue::ProviderNotAllowed(key_info.provider.clone()),
        ));
    }

    if !requirements.allowed_algorithms.is_empty()
        && !requirements
            .allowed_algorithms
            .iter()
            .any(|algorithm| algorithm == &key_info.algorithm)
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

pub struct TpmHardwareKeyAdapter<K> {
    inner: K,
}

impl<K> TpmHardwareKeyAdapter<K> {
    pub fn new(inner: K) -> Self {
        Self { inner }
    }
}

impl<K: TpmSigningKey> HardwareSigningKey for TpmHardwareKeyAdapter<K> {
    fn key_info(&self) -> Result<HardwareKeyInfo, HardwareSigningError> {
        let info = self.inner.key_info()?;
        Ok(HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: info.key_name,
            algorithm: map_tpm_algorithm(&info.algorithm),
            public_key: info.public_key,
            attestation: info.attestation_blob.into_iter().collect(),
            assurance_level: map_tpm_assurance(&info.assurance_level),
            biometric_state: BiometricState::default(),
        })
    }

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, HardwareSigningError> {
        self.inner.sign_message(message).map_err(Into::into)
    }
}

pub struct AndroidKeystoreHardwareKeyAdapter<K> {
    inner: K,
}

impl<K> AndroidKeystoreHardwareKeyAdapter<K> {
    pub fn new(inner: K) -> Self {
        Self { inner }
    }
}

impl<K: AndroidKeystoreSigningKey> HardwareSigningKey for AndroidKeystoreHardwareKeyAdapter<K> {
    fn key_info(&self) -> Result<HardwareKeyInfo, HardwareSigningError> {
        let info = self.inner.key_info()?;
        Ok(HardwareKeyInfo {
            provider: HardwareProviderKind::AndroidKeystore,
            key_name: info.alias,
            algorithm: map_keystore_algorithm(&info.algorithm),
            public_key: info.public_key,
            attestation: info.attestation_chain,
            assurance_level: map_keystore_assurance(&info.assurance_level),
            biometric_state: BiometricState::default(),
        })
    }

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, HardwareSigningError> {
        self.inner.sign_message(message).map_err(Into::into)
    }
}

pub struct YubiKeyHardwareKeyAdapter<K> {
    inner: K,
}

impl<K> YubiKeyHardwareKeyAdapter<K> {
    pub fn new(inner: K) -> Self {
        Self { inner }
    }
}

impl<K: YubiKeySigningKey> HardwareSigningKey for YubiKeyHardwareKeyAdapter<K> {
    fn key_info(&self) -> Result<HardwareKeyInfo, HardwareSigningError> {
        let info = self.inner.key_info()?;
        Ok(HardwareKeyInfo {
            provider: HardwareProviderKind::YubiKey,
            key_name: info.slot,
            algorithm: map_yubikey_algorithm(&info.algorithm),
            public_key: info.public_key,
            attestation: info.attestation_chain,
            assurance_level: map_yubikey_assurance(&info.assurance_level),
            biometric_state: BiometricState::default(),
        })
    }

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, HardwareSigningError> {
        self.inner.sign_message(message).map_err(Into::into)
    }
}

pub fn sign_record_with_tpm_provider<K: TpmSigningKey>(
    key: &K,
    requirements: &HardwareValidationRequirements,
    sig_domain_tag: &str,
    record_hash: &[u8],
) -> Result<Vec<u8>, HardwareSigningError> {
    let provider_requirements = provider_requirements_for_tpm(requirements)?;
    sign_record_with_tpm_checked(key, &provider_requirements, sig_domain_tag, record_hash)
        .map_err(Into::into)
}

pub fn sign_record_with_android_keystore_provider<K: AndroidKeystoreSigningKey>(
    key: &K,
    requirements: &HardwareValidationRequirements,
    sig_domain_tag: &str,
    record_hash: &[u8],
) -> Result<Vec<u8>, HardwareSigningError> {
    let provider_requirements = provider_requirements_for_keystore(requirements)?;
    sign_record_with_keystore_checked(key, &provider_requirements, sig_domain_tag, record_hash)
        .map_err(Into::into)
}

pub fn sign_record_with_yubikey_provider<K: YubiKeySigningKey>(
    key: &K,
    requirements: &HardwareValidationRequirements,
    sig_domain_tag: &str,
    record_hash: &[u8],
) -> Result<Vec<u8>, HardwareSigningError> {
    let provider_requirements = provider_requirements_for_yubikey(requirements)?;
    sign_record_with_yubikey_checked(key, &provider_requirements, sig_domain_tag, record_hash)
        .map_err(Into::into)
}

fn provider_requirements_for_tpm(
    requirements: &HardwareValidationRequirements,
) -> Result<Vec<TpmSignatureAlgorithm>, HardwareSigningError> {
    if requirements.allowed_algorithms.is_empty() {
        return Ok(Vec::new());
    }
    requirements
        .allowed_algorithms
        .iter()
        .map(map_hardware_to_tpm_algorithm)
        .collect()
}

fn provider_requirements_for_keystore(
    requirements: &HardwareValidationRequirements,
) -> Result<Vec<AndroidKeystoreSignatureAlgorithm>, HardwareSigningError> {
    if requirements.allowed_algorithms.is_empty() {
        return Ok(Vec::new());
    }
    requirements
        .allowed_algorithms
        .iter()
        .map(map_hardware_to_keystore_algorithm)
        .collect()
}

fn provider_requirements_for_yubikey(
    requirements: &HardwareValidationRequirements,
) -> Result<Vec<YubiKeySignatureAlgorithm>, HardwareSigningError> {
    if requirements.allowed_algorithms.is_empty() {
        return Ok(Vec::new());
    }
    requirements
        .allowed_algorithms
        .iter()
        .map(map_hardware_to_yubikey_algorithm)
        .collect()
}

fn map_tpm_algorithm(algorithm: &TpmSignatureAlgorithm) -> HardwareSignatureAlgorithm {
    match algorithm {
        TpmSignatureAlgorithm::RsaPkcs1v15Sha256 => HardwareSignatureAlgorithm::RsaPkcs1v15Sha256,
        TpmSignatureAlgorithm::RsaPssSha256 => HardwareSignatureAlgorithm::RsaPssSha256,
        TpmSignatureAlgorithm::EcdsaP256Sha256 => HardwareSignatureAlgorithm::EcdsaP256Sha256,
        TpmSignatureAlgorithm::EcdsaP384Sha384 => HardwareSignatureAlgorithm::EcdsaP384Sha384,
        TpmSignatureAlgorithm::EcSchnorr => HardwareSignatureAlgorithm::EcSchnorr,
        TpmSignatureAlgorithm::Eddsa => HardwareSignatureAlgorithm::Eddsa,
        TpmSignatureAlgorithm::Opaque(v) => HardwareSignatureAlgorithm::Opaque(v.clone()),
    }
}

fn map_keystore_algorithm(
    algorithm: &AndroidKeystoreSignatureAlgorithm,
) -> HardwareSignatureAlgorithm {
    match algorithm {
        AndroidKeystoreSignatureAlgorithm::RsaPkcs1v15Sha256 => {
            HardwareSignatureAlgorithm::RsaPkcs1v15Sha256
        }
        AndroidKeystoreSignatureAlgorithm::RsaPssSha256 => HardwareSignatureAlgorithm::RsaPssSha256,
        AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256 => {
            HardwareSignatureAlgorithm::EcdsaP256Sha256
        }
        AndroidKeystoreSignatureAlgorithm::EcdsaP384Sha384 => {
            HardwareSignatureAlgorithm::EcdsaP384Sha384
        }
        AndroidKeystoreSignatureAlgorithm::Eddsa => HardwareSignatureAlgorithm::Eddsa,
        AndroidKeystoreSignatureAlgorithm::Opaque(v) => {
            HardwareSignatureAlgorithm::Opaque(v.clone())
        }
    }
}

fn map_yubikey_algorithm(algorithm: &YubiKeySignatureAlgorithm) -> HardwareSignatureAlgorithm {
    match algorithm {
        YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256 => {
            HardwareSignatureAlgorithm::RsaPkcs1v15Sha256
        }
        YubiKeySignatureAlgorithm::RsaPssSha256 => HardwareSignatureAlgorithm::RsaPssSha256,
        YubiKeySignatureAlgorithm::EcdsaP256Sha256 => HardwareSignatureAlgorithm::EcdsaP256Sha256,
        YubiKeySignatureAlgorithm::EcdsaP384Sha384 => HardwareSignatureAlgorithm::EcdsaP384Sha384,
        YubiKeySignatureAlgorithm::Eddsa => HardwareSignatureAlgorithm::Eddsa,
        YubiKeySignatureAlgorithm::Opaque(v) => HardwareSignatureAlgorithm::Opaque(v.clone()),
    }
}

fn map_tpm_assurance(level: &TpmAssuranceLevel) -> HardwareAssuranceLevel {
    match level {
        TpmAssuranceLevel::SoftwareSimulated => HardwareAssuranceLevel::Software,
        TpmAssuranceLevel::DiscreteTpm | TpmAssuranceLevel::IntegratedTpm => {
            HardwareAssuranceLevel::IsolatedHardware
        }
        TpmAssuranceLevel::Certified(v) => HardwareAssuranceLevel::Certified(v.clone()),
        TpmAssuranceLevel::Unknown => HardwareAssuranceLevel::Unknown,
    }
}

fn map_keystore_assurance(level: &AndroidKeystoreAssuranceLevel) -> HardwareAssuranceLevel {
    match level {
        AndroidKeystoreAssuranceLevel::Software => HardwareAssuranceLevel::Software,
        AndroidKeystoreAssuranceLevel::Tee => HardwareAssuranceLevel::IsolatedHardware,
        AndroidKeystoreAssuranceLevel::StrongBox => HardwareAssuranceLevel::StrongBox,
        AndroidKeystoreAssuranceLevel::Attested(v) => HardwareAssuranceLevel::Certified(v.clone()),
        AndroidKeystoreAssuranceLevel::Unknown => HardwareAssuranceLevel::Unknown,
    }
}

fn map_yubikey_assurance(level: &YubiKeyAssuranceLevel) -> HardwareAssuranceLevel {
    match level {
        YubiKeyAssuranceLevel::SoftwareSimulator => HardwareAssuranceLevel::Software,
        YubiKeyAssuranceLevel::HardwareBacked | YubiKeyAssuranceLevel::PivAttested => {
            HardwareAssuranceLevel::IsolatedHardware
        }
        YubiKeyAssuranceLevel::Fips => HardwareAssuranceLevel::Certified("fips".into()),
        YubiKeyAssuranceLevel::Certified(v) => HardwareAssuranceLevel::Certified(v.clone()),
        YubiKeyAssuranceLevel::Unknown => HardwareAssuranceLevel::Unknown,
    }
}

fn map_hardware_to_tpm_algorithm(
    algorithm: &HardwareSignatureAlgorithm,
) -> Result<TpmSignatureAlgorithm, HardwareSigningError> {
    Ok(match algorithm {
        HardwareSignatureAlgorithm::RsaPkcs1v15Sha256 => TpmSignatureAlgorithm::RsaPkcs1v15Sha256,
        HardwareSignatureAlgorithm::RsaPssSha256 => TpmSignatureAlgorithm::RsaPssSha256,
        HardwareSignatureAlgorithm::EcdsaP256Sha256 => TpmSignatureAlgorithm::EcdsaP256Sha256,
        HardwareSignatureAlgorithm::EcdsaP384Sha384 => TpmSignatureAlgorithm::EcdsaP384Sha384,
        HardwareSignatureAlgorithm::EcSchnorr => TpmSignatureAlgorithm::EcSchnorr,
        HardwareSignatureAlgorithm::Eddsa => TpmSignatureAlgorithm::Eddsa,
        HardwareSignatureAlgorithm::Opaque(v) => TpmSignatureAlgorithm::Opaque(v.clone()),
    })
}

fn map_hardware_to_keystore_algorithm(
    algorithm: &HardwareSignatureAlgorithm,
) -> Result<AndroidKeystoreSignatureAlgorithm, HardwareSigningError> {
    Ok(match algorithm {
        HardwareSignatureAlgorithm::RsaPkcs1v15Sha256 => {
            AndroidKeystoreSignatureAlgorithm::RsaPkcs1v15Sha256
        }
        HardwareSignatureAlgorithm::RsaPssSha256 => AndroidKeystoreSignatureAlgorithm::RsaPssSha256,
        HardwareSignatureAlgorithm::EcdsaP256Sha256 => {
            AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256
        }
        HardwareSignatureAlgorithm::EcdsaP384Sha384 => {
            AndroidKeystoreSignatureAlgorithm::EcdsaP384Sha384
        }
        HardwareSignatureAlgorithm::Eddsa => AndroidKeystoreSignatureAlgorithm::Eddsa,
        HardwareSignatureAlgorithm::EcSchnorr => {
            return Err(HardwareSigningError::Provider(
                "Android Keystore adapter does not expose EcSchnorr".into(),
            ))
        }
        HardwareSignatureAlgorithm::Opaque(v) => {
            AndroidKeystoreSignatureAlgorithm::Opaque(v.clone())
        }
    })
}

fn map_hardware_to_yubikey_algorithm(
    algorithm: &HardwareSignatureAlgorithm,
) -> Result<YubiKeySignatureAlgorithm, HardwareSigningError> {
    Ok(match algorithm {
        HardwareSignatureAlgorithm::RsaPkcs1v15Sha256 => {
            YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256
        }
        HardwareSignatureAlgorithm::RsaPssSha256 => YubiKeySignatureAlgorithm::RsaPssSha256,
        HardwareSignatureAlgorithm::EcdsaP256Sha256 => YubiKeySignatureAlgorithm::EcdsaP256Sha256,
        HardwareSignatureAlgorithm::EcdsaP384Sha384 => YubiKeySignatureAlgorithm::EcdsaP384Sha384,
        HardwareSignatureAlgorithm::Eddsa => YubiKeySignatureAlgorithm::Eddsa,
        HardwareSignatureAlgorithm::EcSchnorr => {
            return Err(HardwareSigningError::Provider(
                "YubiKey adapter does not expose EcSchnorr".into(),
            ))
        }
        HardwareSignatureAlgorithm::Opaque(v) => YubiKeySignatureAlgorithm::Opaque(v.clone()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use edgerun_android_keystore::{
        AndroidKeystoreAssuranceLevel, AndroidKeystoreKeyInfo, AndroidKeystoreSignatureAlgorithm,
    };
    use edgerun_biometrics::BiometricModality;
    use edgerun_tpm::{TpmAssuranceLevel, TpmKeyInfo, TpmSignatureAlgorithm};
    use edgerun_yubikey::{YubiKeyAssuranceLevel, YubiKeyKeyInfo, YubiKeySignatureAlgorithm};

    // =========================================================================
    // Fake key implementations for testing
    // =========================================================================

    struct FakeTpmKey {
        pkcs1v15_key: bool,
    }

    impl FakeTpmKey {
        fn new() -> Self {
            Self {
                pkcs1v15_key: false,
            }
        }
        fn with_pkcs1v15() -> Self {
            Self {
                pkcs1v15_key: true,
            }
        }
    }

    impl TpmSigningKey for FakeTpmKey {
        fn key_info(&self) -> Result<TpmKeyInfo, TpmError> {
            let algorithm = if self.pkcs1v15_key {
                TpmSignatureAlgorithm::RsaPkcs1v15Sha256
            } else {
                TpmSignatureAlgorithm::EcdsaP256Sha256
            };
            Ok(TpmKeyInfo {
                key_name: "tpm-key".into(),
                algorithm,
                public_key: vec![1, 2, 3],
                attestation_blob: Some(vec![9, 9]),
                assurance_level: TpmAssuranceLevel::DiscreteTpm,
            })
        }
        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, TpmError> {
            Ok(message.to_vec())
        }
    }

    struct FakeTpmKeyWithCustomPk {
        public_key: Vec<u8>,
        assurance: TpmAssuranceLevel,
    }

    impl TpmSigningKey for FakeTpmKeyWithCustomPk {
        fn key_info(&self) -> Result<TpmKeyInfo, TpmError> {
            Ok(TpmKeyInfo {
                key_name: "tpm-custom".into(),
                algorithm: TpmSignatureAlgorithm::EcdsaP256Sha256,
                public_key: self.public_key.clone(),
                attestation_blob: Some(vec![10, 10]),
                assurance_level: self.assurance.clone(),
            })
        }
        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, TpmError> {
            Ok(message.to_vec())
        }
    }

    struct FakeKeystoreKey {
        algorithm: AndroidKeystoreSignatureAlgorithm,
    }

    impl FakeKeystoreKey {
        fn new() -> Self {
            Self {
                algorithm: AndroidKeystoreSignatureAlgorithm::Eddsa,
            }
        }
        fn with_algorithm(algo: AndroidKeystoreSignatureAlgorithm) -> Self {
            Self { algorithm: algo }
        }
    }

    impl AndroidKeystoreSigningKey for FakeKeystoreKey {
        fn key_info(&self) -> Result<AndroidKeystoreKeyInfo, AndroidKeystoreError> {
            Ok(AndroidKeystoreKeyInfo {
                alias: "android-key".into(),
                algorithm: self.algorithm.clone(),
                public_key: vec![4, 5, 6],
                attestation_chain: vec![vec![7, 8]],
                assurance_level: AndroidKeystoreAssuranceLevel::StrongBox,
            })
        }
        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, AndroidKeystoreError> {
            Ok(message.to_vec())
        }
    }

    struct FakeKeystoreKeyCustom {
        public_key: Vec<u8>,
        assurance: AndroidKeystoreAssuranceLevel,
    }

    impl AndroidKeystoreSigningKey for FakeKeystoreKeyCustom {
        fn key_info(&self) -> Result<AndroidKeystoreKeyInfo, AndroidKeystoreError> {
            Ok(AndroidKeystoreKeyInfo {
                alias: "android-custom".into(),
                algorithm: AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256,
                public_key: self.public_key.clone(),
                attestation_chain: vec![],
                assurance_level: self.assurance.clone(),
            })
        }
        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, AndroidKeystoreError> {
            Ok(message.to_vec())
        }
    }

    struct FakeYubiKey;

    impl YubiKeySigningKey for FakeYubiKey {
        fn key_info(&self) -> Result<YubiKeyKeyInfo, YubiKeyError> {
            Ok(YubiKeyKeyInfo {
                slot: "9c".into(),
                algorithm: YubiKeySignatureAlgorithm::EcdsaP256Sha256,
                public_key: vec![6, 5, 4],
                attestation_chain: vec![vec![3, 2, 1]],
                serial_number: Some("123456".into()),
                assurance_level: YubiKeyAssuranceLevel::PivAttested,
                pin_policy: Some(edgerun_yubikey::YubiKeyPinPolicy::Always),
                touch_policy: Some(edgerun_yubikey::YubiKeyTouchPolicy::Always),
                generated_on_device: Some(true),
            })
        }
        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, YubiKeyError> {
            Ok(message.to_vec())
        }
    }

    struct FakeYubiKeyCustom {
        algorithm: YubiKeySignatureAlgorithm,
        assurance: YubiKeyAssuranceLevel,
    }

    impl YubiKeySigningKey for FakeYubiKeyCustom {
        fn key_info(&self) -> Result<YubiKeyKeyInfo, YubiKeyError> {
            Ok(YubiKeyKeyInfo {
                slot: "9a".into(),
                algorithm: self.algorithm.clone(),
                public_key: vec![10, 11, 12],
                attestation_chain: vec![],
                serial_number: None,
                assurance_level: self.assurance.clone(),
                pin_policy: None,
                touch_policy: None,
                generated_on_device: None,
            })
        }
        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, YubiKeyError> {
            Ok(message.to_vec())
        }
    }

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
            // Simulate hardware: return 64-byte "signature" based on digest
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

    // =========================================================================
    // NodeID tests
    // =========================================================================

    #[test]
    fn node_id_short_display() {
        let mut bytes = [0u8; 64];
        bytes[0] = 0xaa;
        bytes[1] = 0xbb;
        bytes[2] = 0xcc;
        bytes[3] = 0xdd;
        let id = NodeID(bytes);
        assert_eq!(id.short(), "0xaabbccdd");
        assert_eq!(id.to_hex().len(), 130); // 64 bytes * 2 + "0x"
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
        let slice: &[u8] = id.as_ref();
        assert_eq!(slice.len(), 64);
        assert_eq!(slice[0], 0x42);
    }

    #[test]
    fn node_id_copy_and_equality() {
        let bytes = [1u8; 64];
        let id1 = NodeID(bytes);
        let id2 = id1; // Copy
        assert_eq!(id1, id2);
    }

    #[test]
    fn node_id_hash_consistency() {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

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

    // =========================================================================
    // HardwareSignatureAlgorithm tests
    // =========================================================================

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
        let cloned = algo.clone();
        assert_eq!(algo, cloned);
    }

    #[test]
    fn opaque_algorithm_variant() {
        let opaque = HardwareSignatureAlgorithm::Opaque("my-custom-algo".into());
        assert!(!opaque.is_ecdsa_p256());
        assert_eq!(opaque, HardwareSignatureAlgorithm::Opaque("my-custom-algo".into()));
        assert_ne!(opaque, HardwareSignatureAlgorithm::Eddsa);
    }

    // =========================================================================
    // HardwareProviderKind tests
    // =========================================================================

    #[test]
    fn provider_kind_clone_and_equality() {
        let tpm = HardwareProviderKind::Tpm;
        assert_eq!(tpm, HardwareProviderKind::Tpm);
        assert_ne!(tpm, HardwareProviderKind::AndroidKeystore);
        assert_ne!(tpm, HardwareProviderKind::YubiKey);

        let other1 = HardwareProviderKind::Other("custom".into());
        let other2 = HardwareProviderKind::Other("custom".into());
        assert_eq!(other1, other2);
        assert_ne!(other1, HardwareProviderKind::Other("different".into()));
    }

    // =========================================================================
    // HardwareAssuranceLevel and HardwareAssuranceStrength tests
    // =========================================================================

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
        assert_eq!(
            HardwareAssuranceLevel::Certified("cert".into()).strength(),
            HardwareAssuranceStrength::Certified
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

        let software = HardwareAssuranceLevel::Software;
        assert!(software.is_at_least(HardwareAssuranceStrength::Software));
        assert!(software.is_at_least(HardwareAssuranceStrength::Unknown));
        assert!(!software.is_at_least(HardwareAssuranceStrength::IsolatedHardware));
    }

    // =========================================================================
    // HardwareKeyInfo tests
    // =========================================================================

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
            public_key: vec![0x42u8; 96], // P384 size
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
    fn is_mesh_capable_with_valid_ecdsa_key() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::YubiKey,
            key_name: "yubi-9c".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![0xABu8; 64],
            attestation: vec![vec![1, 2, 3]],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        assert!(info.is_mesh_capable());
    }

    #[test]
    fn is_mesh_capable_false_for_ecdsa_p384() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "tpm-p384".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP384Sha384,
            public_key: vec![0xABu8; 64],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        assert!(!info.is_mesh_capable());
    }

    #[test]
    fn hardware_key_info_clone_and_debug() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::AndroidKeystore,
            key_name: "alias".into(),
            algorithm: HardwareSignatureAlgorithm::RsaPkcs1v15Sha256,
            public_key: vec![1, 2, 3],
            attestation: vec![vec![4, 5]],
            assurance_level: HardwareAssuranceLevel::StrongBox,
            biometric_state: BiometricState::default(),
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
        // Debug should not panic
        let _ = format!("{:?}", info);
    }

    // =========================================================================
    // HardwareValidationRequirements tests
    // =========================================================================

    #[test]
    fn validation_requirements_default() {
        let reqs = HardwareValidationRequirements::default();
        assert!(reqs.allowed_providers.is_empty());
        assert!(reqs.allowed_algorithms.is_empty());
        assert!(reqs.minimum_assurance.is_none());
        assert!(!reqs.require_attestation);
        assert!(!reqs.require_public_key);
        assert!(reqs.minimum_biometric_assurance.is_none());
    }

    #[test]
    fn validation_requirements_clone_and_equality() {
        let reqs = HardwareValidationRequirements {
            allowed_providers: vec![HardwareProviderKind::Tpm],
            allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
            minimum_assurance: Some(HardwareAssuranceStrength::IsolatedHardware),
            require_attestation: true,
            require_public_key: true,
            minimum_biometric_assurance: Some(BiometricAssuranceStrength::BiometricMatch),
        };
        let cloned = reqs.clone();
        assert_eq!(reqs, cloned);
    }

    // =========================================================================
    // HardwareValidationIssue tests
    // =========================================================================

    #[test]
    fn validation_issue_clone_and_debug() {
        let issue = HardwareValidationIssue::ProviderNotAllowed(HardwareProviderKind::Tpm);
        let cloned = issue.clone();
        assert_eq!(issue, cloned);
        let _ = format!("{:?}", issue);

        let issue2 = HardwareValidationIssue::MissingAttestation;
        let _ = format!("{:?}", issue2);
    }

    #[test]
    fn validation_issue_assurance_too_weak() {
        let issue = HardwareValidationIssue::AssuranceTooWeak {
            actual: HardwareAssuranceLevel::Software,
            required: HardwareAssuranceStrength::IsolatedHardware,
        };
        let cloned = issue.clone();
        assert_eq!(issue, cloned);
    }

    #[test]
    fn validation_issue_biometric_too_weak() {
        let issue = HardwareValidationIssue::BiometricAssuranceTooWeak {
            actual: BiometricAssuranceStrength::None,
            required: BiometricAssuranceStrength::BiometricMatch,
        };
        let cloned = issue.clone();
        assert_eq!(issue, cloned);
    }

    // =========================================================================
    // HardwareSigningError tests
    // =========================================================================

    #[test]
    fn error_display_tpm() {
        let err = HardwareSigningError::Tpm(TpmError::Provider("tpm unavailable".into()));
        let msg = format!("{}", err);
        assert!(msg.starts_with("TPM error:"));
        assert!(msg.contains("tpm unavailable"));
    }

    #[test]
    fn error_display_android_keystore() {
        let err = HardwareSigningError::AndroidKeystore(AndroidKeystoreError::Provider("keystore unavailable".into()));
        let msg = format!("{}", err);
        assert!(msg.starts_with("Android Keystore error:"));
        assert!(msg.contains("keystore unavailable"));
    }

    #[test]
    fn error_display_yubikey() {
        let err = HardwareSigningError::YubiKey(YubiKeyError::Provider("yubikey not found".into()));
        let msg = format!("{}", err);
        assert!(msg.starts_with("YubiKey error:"));
        assert!(msg.contains("yubikey not found"));
    }

    #[test]
    fn error_display_unsupported_algorithm() {
        let err = HardwareSigningError::UnsupportedAlgorithm(HardwareSignatureAlgorithm::EcSchnorr);
        let msg = format!("{}", err);
        assert!(msg.contains("unsupported hardware signature algorithm"));
        assert!(msg.contains("EcSchnorr"));
    }

    #[test]
    fn error_display_validation() {
        let err = HardwareSigningError::Validation(HardwareValidationIssue::MissingAttestation);
        let msg = format!("{}", err);
        assert!(msg.contains("hardware validation failed"));
        assert!(msg.contains("MissingAttestation"));
    }

    #[test]
    fn error_display_provider() {
        let err = HardwareSigningError::Provider("custom error message".into());
        let msg = format!("{}", err);
        assert_eq!(msg, "custom error message");
    }

    #[test]
    fn error_is_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(HardwareSigningError::Provider("test".into()));
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn error_from_tpm_error() {
        let tpm_err = TpmError::Provider("test".into());
        let hw_err: HardwareSigningError = tpm_err.into();
        assert!(matches!(hw_err, HardwareSigningError::Tpm(_)));
    }

    #[test]
    fn error_from_android_keystore_error() {
        let android_err = AndroidKeystoreError::Provider("test".into());
        let hw_err: HardwareSigningError = android_err.into();
        assert!(matches!(hw_err, HardwareSigningError::AndroidKeystore(_)));
    }

    #[test]
    fn error_from_yubikey_error() {
        let yubi_err = YubiKeyError::Provider("test".into());
        let hw_err: HardwareSigningError = yubi_err.into();
        assert!(matches!(hw_err, HardwareSigningError::YubiKey(_)));
    }

    #[test]
    fn error_clone_and_equality() {
        let err1 = HardwareSigningError::UnsupportedAlgorithm(HardwareSignatureAlgorithm::Eddsa);
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    // =========================================================================
    // validate_hardware_key_info tests
    // =========================================================================

    #[test]
    fn validation_passes_with_empty_requirements() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![1, 2, 3],
            attestation: vec![vec![1]],
            assurance_level: HardwareAssuranceLevel::Unknown,
            biometric_state: BiometricState::default(),
        };
        let reqs = HardwareValidationRequirements::default();
        validate_hardware_key_info(&info, &reqs).unwrap();
    }

    #[test]
    fn validation_rejects_disallowed_provider() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::AndroidKeystore,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![1],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        let reqs = HardwareValidationRequirements {
            allowed_providers: vec![HardwareProviderKind::Tpm],
            ..Default::default()
        };
        let err = validate_hardware_key_info(&info, &reqs).unwrap_err();
        assert_eq!(
            err,
            HardwareSigningError::Validation(HardwareValidationIssue::ProviderNotAllowed(
                HardwareProviderKind::AndroidKeystore
            ))
        );
    }

    #[test]
    fn validation_accepts_allowed_provider() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![1],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        let reqs = HardwareValidationRequirements {
            allowed_providers: vec![HardwareProviderKind::Tpm],
            ..Default::default()
        };
        validate_hardware_key_info(&info, &reqs).unwrap();
    }

    #[test]
    fn validation_rejects_disallowed_algorithm() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::Eddsa,
            public_key: vec![1],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        let reqs = HardwareValidationRequirements {
            allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
            ..Default::default()
        };
        let err = validate_hardware_key_info(&info, &reqs).unwrap_err();
        assert_eq!(
            err,
            HardwareSigningError::Validation(HardwareValidationIssue::AlgorithmNotAllowed(
                HardwareSignatureAlgorithm::Eddsa
            ))
        );
    }

    #[test]
    fn validation_accepts_allowed_algorithm() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![1],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        let reqs = HardwareValidationRequirements {
            allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
            ..Default::default()
        };
        validate_hardware_key_info(&info, &reqs).unwrap();
    }

    #[test]
    fn validation_rejects_weak_assurance() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::AndroidKeystore,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::Eddsa,
            public_key: vec![1],
            attestation: vec![vec![1]],
            assurance_level: HardwareAssuranceLevel::Software,
            biometric_state: BiometricState::default(),
        };
        let err = validate_hardware_key_info(
            &info,
            &HardwareValidationRequirements {
                minimum_assurance: Some(HardwareAssuranceStrength::IsolatedHardware),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(matches!(
            err,
            HardwareSigningError::Validation(HardwareValidationIssue::AssuranceTooWeak { .. })
        ));
    }

    #[test]
    fn validation_accepts_sufficient_assurance() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![1],
            attestation: vec![vec![1]],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        validate_hardware_key_info(
            &info,
            &HardwareValidationRequirements {
                minimum_assurance: Some(HardwareAssuranceStrength::IsolatedHardware),
                ..Default::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn validation_rejects_missing_attestation() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![1],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        let err = validate_hardware_key_info(
            &info,
            &HardwareValidationRequirements {
                require_attestation: true,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            err,
            HardwareSigningError::Validation(HardwareValidationIssue::MissingAttestation)
        );
    }

    #[test]
    fn validation_passes_with_attestation() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![1],
            attestation: vec![vec![1, 2, 3]],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        validate_hardware_key_info(
            &info,
            &HardwareValidationRequirements {
                require_attestation: true,
                ..Default::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn validation_rejects_missing_public_key() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        let err = validate_hardware_key_info(
            &info,
            &HardwareValidationRequirements {
                require_public_key: true,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            err,
            HardwareSigningError::Validation(HardwareValidationIssue::MissingPublicKey)
        );
    }

    #[test]
    fn validation_passes_with_public_key() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![0xAB; 64],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::IsolatedHardware,
            biometric_state: BiometricState::default(),
        };
        validate_hardware_key_info(
            &info,
            &HardwareValidationRequirements {
                require_public_key: true,
                ..Default::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn validation_rejects_missing_biometric_requirement() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::AndroidKeystore,
            key_name: "bio".into(),
            algorithm: HardwareSignatureAlgorithm::Eddsa,
            public_key: vec![1],
            attestation: vec![vec![1]],
            assurance_level: HardwareAssuranceLevel::StrongBox,
            biometric_state: BiometricState::default(),
        };
        let err = validate_hardware_key_info(
            &info,
            &HardwareValidationRequirements {
                minimum_biometric_assurance: Some(BiometricAssuranceStrength::BiometricMatch),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            err,
            HardwareSigningError::Validation(HardwareValidationIssue::BiometricAssuranceTooWeak {
                actual: BiometricAssuranceStrength::None,
                required: BiometricAssuranceStrength::BiometricMatch,
            })
        );
    }

    #[test]
    fn validation_accepts_hardware_protected_biometric() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::AndroidKeystore,
            key_name: "bio".into(),
            algorithm: HardwareSignatureAlgorithm::Eddsa,
            public_key: vec![1],
            attestation: vec![vec![1]],
            assurance_level: HardwareAssuranceLevel::StrongBox,
            biometric_state: BiometricState {
                modality: None,
                verified: true,
                hardware_protected: true,
                user_present: true,
            },
        };
        validate_hardware_key_info(
            &info,
            &HardwareValidationRequirements {
                minimum_biometric_assurance: Some(BiometricAssuranceStrength::BiometricMatch),
                ..Default::default()
            },
        )
        .unwrap();
    }

    #[test]
    fn validation_rejects_user_presence_when_biometric_match_required() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::AndroidKeystore,
            key_name: "bio".into(),
            algorithm: HardwareSignatureAlgorithm::Eddsa,
            public_key: vec![1],
            attestation: vec![vec![1]],
            assurance_level: HardwareAssuranceLevel::StrongBox,
            biometric_state: BiometricState {
                modality: Some(BiometricModality::Fingerprint),
                verified: false,
                hardware_protected: false,
                user_present: true,
            },
        };
        let err = validate_hardware_key_info(
            &info,
            &HardwareValidationRequirements {
                minimum_biometric_assurance: Some(BiometricAssuranceStrength::BiometricMatch),
                ..Default::default()
            },
        )
        .unwrap_err();
        assert_eq!(
            err,
            HardwareSigningError::Validation(HardwareValidationIssue::BiometricAssuranceTooWeak {
                actual: BiometricAssuranceStrength::UserPresence,
                required: BiometricAssuranceStrength::BiometricMatch,
            })
        );
    }

    #[test]
    fn validation_combined_multiple_requirements() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![0xCC; 64],
            attestation: vec![vec![1, 2]],
            assurance_level: HardwareAssuranceLevel::Certified("tpm-cert".into()),
            biometric_state: BiometricState::default(),
        };
        let reqs = HardwareValidationRequirements {
            allowed_providers: vec![HardwareProviderKind::Tpm],
            allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
            minimum_assurance: Some(HardwareAssuranceStrength::Certified),
            require_attestation: true,
            require_public_key: true,
            ..Default::default()
        };
        validate_hardware_key_info(&info, &reqs).unwrap();
    }

    #[test]
    fn validation_fails_on_first_check_only() {
        // When multiple requirements fail, only the first one is reported
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::AndroidKeystore,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::Eddsa,
            public_key: vec![],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::Software,
            biometric_state: BiometricState::default(),
        };
        let reqs = HardwareValidationRequirements {
            allowed_providers: vec![HardwareProviderKind::Tpm],
            allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
            minimum_assurance: Some(HardwareAssuranceStrength::IsolatedHardware),
            require_attestation: true,
            require_public_key: true,
            ..Default::default()
        };
        let err = validate_hardware_key_info(&info, &reqs).unwrap_err();
        // First check that fails is provider
        assert_eq!(
            err,
            HardwareSigningError::Validation(HardwareValidationIssue::ProviderNotAllowed(
                HardwareProviderKind::AndroidKeystore
            ))
        );
    }

    // =========================================================================
    // sign_record_with_hardware tests
    // =========================================================================

    #[test]
    fn sign_record_with_hardware_calls_sign_message() {
        let key = FakeMeshKey::ecdsa_p256();
        let record_hash = [0xAA; 32];
        let sig = sign_record_with_hardware(&key, "edgerun:v0:sig:test", &record_hash).unwrap();
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn sign_record_with_hardware_uses_correct_sig_input() {
        let key = FakeMeshKey::ecdsa_p256();
        let record_hash = [0xBB; 32];
        let sig_input = signature_input_for_record("edgerun:v0:sig:test", &record_hash);
        let sig = sign_record_with_hardware(&key, "edgerun:v0:sig:test", &record_hash).unwrap();
        // FakeMeshKey echoes first 32 bytes of message as first half of signature
        assert_eq!(sig[..32], sig_input[..32]);
    }

    #[test]
    fn sign_record_with_hardware_failing_key() {
        let key = FakeFailingKey {
            error_msg: "signing failed".into(),
        };
        let err = sign_record_with_hardware(&key, "tag", &[0u8; 32]).unwrap_err();
        assert_eq!(err, HardwareSigningError::Provider("signing failed".into()));
    }

    // =========================================================================
    // sign_record_with_hardware_checked tests
    // =========================================================================

    #[test]
    fn hardware_checked_sign_rejects_unexpected_algorithm() {
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(FakeKeystoreKey::new());
        let err = sign_record_with_hardware_checked(
            &adapter,
            &HardwareValidationRequirements {
                allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
                ..Default::default()
            },
            "edgerun:v0:sig:test",
            &[1u8; 32],
        )
        .unwrap_err();
        assert_eq!(
            err,
            HardwareSigningError::Validation(HardwareValidationIssue::AlgorithmNotAllowed(
                HardwareSignatureAlgorithm::Eddsa
            ))
        );
    }

    #[test]
    fn hardware_checked_sign_passes_when_validated() {
        let key = FakeMeshKey::ecdsa_p256();
        let sig = sign_record_with_hardware_checked(
            &key,
            &HardwareValidationRequirements {
                allowed_providers: vec![HardwareProviderKind::Tpm],
                allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
                ..Default::default()
            },
            "edgerun:v0:sig:test",
            &[0xCC; 32],
        )
        .unwrap();
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn hardware_checked_sign_rejects_disallowed_provider() {
        let key = FakeMeshKey::ecdsa_p256();
        let err = sign_record_with_hardware_checked(
            &key,
            &HardwareValidationRequirements {
                allowed_providers: vec![HardwareProviderKind::YubiKey],
                ..Default::default()
            },
            "tag",
            &[0u8; 32],
        )
        .unwrap_err();
        assert!(matches!(
            err,
            HardwareSigningError::Validation(HardwareValidationIssue::ProviderNotAllowed(
                HardwareProviderKind::Tpm
            ))
        ));
    }

    // =========================================================================
    // HardwareMeshSigner tests
    // =========================================================================

    #[test]
    fn hardware_mesh_signer_extracts_node_id_and_signs_digest() {
        let key = FakeMeshKey::ecdsa_p256();
        let signer = HardwareMeshSigner::new(key).expect("should create signer");

        // NodeID extracted from public key
        let node_id = signer.node_id();
        assert_eq!(node_id.0[0], 0x04);
        assert_eq!(node_id.0[1], 0xAB);

        // Sign a digest
        let digest = [0x42u8; 32];
        let sig = signer.sign_digest(&digest).expect("should sign digest");
        assert_eq!(sig.len(), 64);
        assert_eq!(sig[..32], digest); // first half is the digest (as expected from fake key)
    }

    #[test]
    fn hardware_mesh_signer_rejects_non_ecdsa_key() {
        let key = FakeMeshKey {
            public_key: vec![0x01; 64],
            algorithm: HardwareSignatureAlgorithm::Eddsa,
            assurance: HardwareAssuranceLevel::IsolatedHardware,
        };
        match HardwareMeshSigner::new(key) {
            Ok(_) => panic!("should have failed"),
            Err(HardwareSigningError::Provider(msg)) => assert!(msg.contains("ECDSA P-256")),
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn hardware_mesh_signer_rejects_wrong_key_length() {
        let key = FakeMeshKey {
            public_key: vec![0x01; 32],
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            assurance: HardwareAssuranceLevel::IsolatedHardware,
        };
        match HardwareMeshSigner::new(key) {
            Ok(_) => panic!("should have failed"),
            Err(HardwareSigningError::Provider(_)) => {} // expected
            Err(e) => panic!("unexpected error: {:?}", e),
        }
    }

    #[test]
    fn hardware_mesh_signer_key_accessor() {
        let key = FakeMeshKey::ecdsa_p256();
        let signer = HardwareMeshSigner::new(key).unwrap();
        let _inner: &FakeMeshKey = signer.key();
    }

    #[test]
    fn hardware_mesh_signer_returns_wrong_signature_length() {
        let key = FakeMeshKeyWithSig { sig_len: 32 };
        let signer = HardwareMeshSigner::new(key).unwrap();
        let digest = [0u8; 32];
        let err = signer.sign_digest(&digest).unwrap_err();
        assert!(matches!(err, HardwareSigningError::Provider(_)));
        let msg = format!("{}", err);
        assert!(msg.contains("32-byte signature"));
        assert!(msg.contains("expected 64"));
    }

    #[test]
    fn hardware_mesh_signer_returns_zero_length_signature() {
        let key = FakeMeshKeyWithSig { sig_len: 0 };
        let signer = HardwareMeshSigner::new(key).unwrap();
        let digest = [0u8; 32];
        let err = signer.sign_digest(&digest).unwrap_err();
        assert!(matches!(err, HardwareSigningError::Provider(_)));
    }

    #[test]
    fn hardware_mesh_signer_signing_failure_propagates() {
        let key = FakeFailingKey {
            error_msg: "hardware signing failed".into(),
        };
        let signer = HardwareMeshSigner::new(key).unwrap();
        let digest = [0u8; 32];
        let err = signer.sign_digest(&digest).unwrap_err();
        assert_eq!(
            err,
            HardwareSigningError::Provider("hardware signing failed".into())
        );
    }

    #[test]
    fn hardware_mesh_signer_produces_exactly_64_bytes() {
        let key = FakeMeshKey::ecdsa_p256();
        let signer = HardwareMeshSigner::new(key).unwrap();
        let digest = [0xFF; 32];
        let sig = signer.sign_digest(&digest).unwrap();
        assert_eq!(sig.len(), MESH_SIGNATURE_LENGTH);
    }

    #[test]
    fn hardware_mesh_signer_as_any() {
        let key = FakeMeshKey::ecdsa_p256();
        let signer = HardwareMeshSigner::new(key).unwrap();
        let _any: &dyn std::any::Any = signer.as_any();
    }

    // =========================================================================
    // Adapter tests: TpmHardwareKeyAdapter
    // =========================================================================

    #[test]
    fn tpm_adapter_maps_key_info() {
        let adapter = TpmHardwareKeyAdapter::new(FakeTpmKey::new());
        let info = adapter.key_info().unwrap();
        assert_eq!(info.provider, HardwareProviderKind::Tpm);
        assert_eq!(info.algorithm, HardwareSignatureAlgorithm::EcdsaP256Sha256);
        assert_eq!(
            info.assurance_level,
            HardwareAssuranceLevel::IsolatedHardware
        );
    }

    #[test]
    fn tpm_adapter_maps_pkcs1v15_algorithm() {
        let adapter = TpmHardwareKeyAdapter::new(FakeTpmKey::with_pkcs1v15());
        let info = adapter.key_info().unwrap();
        assert_eq!(info.algorithm, HardwareSignatureAlgorithm::RsaPkcs1v15Sha256);
    }

    #[test]
    fn tpm_adapter_sign_message() {
        let adapter = TpmHardwareKeyAdapter::new(FakeTpmKey::new());
        let message = b"test message";
        let sig = adapter.sign_message(message).unwrap();
        assert_eq!(sig, message.to_vec());
    }

    #[test]
    fn tpm_adapter_certified_assurance() {
        let key = FakeTpmKeyWithCustomPk {
            public_key: vec![1, 2, 3],
            assurance: TpmAssuranceLevel::Certified("tpm-cert".into()),
        };
        let adapter = TpmHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(
            info.assurance_level,
            HardwareAssuranceLevel::Certified("tpm-cert".into())
        );
    }

    #[test]
    fn tpm_adapter_software_simulated() {
        let key = FakeTpmKeyWithCustomPk {
            public_key: vec![1, 2, 3],
            assurance: TpmAssuranceLevel::SoftwareSimulated,
        };
        let adapter = TpmHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.assurance_level, HardwareAssuranceLevel::Software);
    }

    #[test]
    fn tpm_adapter_unknown_assurance() {
        let key = FakeTpmKeyWithCustomPk {
            public_key: vec![1, 2, 3],
            assurance: TpmAssuranceLevel::Unknown,
        };
        let adapter = TpmHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.assurance_level, HardwareAssuranceLevel::Unknown);
    }

    // =========================================================================
    // Adapter tests: AndroidKeystoreHardwareKeyAdapter
    // =========================================================================

    #[test]
    fn android_adapter_maps_key_info() {
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(FakeKeystoreKey::new());
        let info = adapter.key_info().unwrap();
        assert_eq!(info.provider, HardwareProviderKind::AndroidKeystore);
        assert_eq!(info.algorithm, HardwareSignatureAlgorithm::Eddsa);
        assert_eq!(info.assurance_level, HardwareAssuranceLevel::StrongBox);
    }

    #[test]
    fn android_adapter_sign_message() {
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(FakeKeystoreKey::new());
        let message = b"android test";
        let sig = adapter.sign_message(message).unwrap();
        assert_eq!(sig, message.to_vec());
    }

    #[test]
    fn android_adapter_maps_ecdsa_p256() {
        let key = FakeKeystoreKey::with_algorithm(
            AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256,
        );
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.algorithm, HardwareSignatureAlgorithm::EcdsaP256Sha256);
    }

    #[test]
    fn android_adapter_maps_rsa_pss() {
        let key = FakeKeystoreKey::with_algorithm(
            AndroidKeystoreSignatureAlgorithm::RsaPssSha256,
        );
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.algorithm, HardwareSignatureAlgorithm::RsaPssSha256);
    }

    #[test]
    fn android_adapter_maps_rsa_pkcs1v15() {
        let key = FakeKeystoreKey::with_algorithm(
            AndroidKeystoreSignatureAlgorithm::RsaPkcs1v15Sha256,
        );
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(
            info.algorithm,
            HardwareSignatureAlgorithm::RsaPkcs1v15Sha256
        );
    }

    #[test]
    fn android_adapter_maps_ecdsa_p384() {
        let key = FakeKeystoreKey::with_algorithm(
            AndroidKeystoreSignatureAlgorithm::EcdsaP384Sha384,
        );
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.algorithm, HardwareSignatureAlgorithm::EcdsaP384Sha384);
    }

    #[test]
    fn android_adapter_maps_opaque() {
        let key = FakeKeystoreKey::with_algorithm(
            AndroidKeystoreSignatureAlgorithm::Opaque("custom".into()),
        );
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(
            info.algorithm,
            HardwareSignatureAlgorithm::Opaque("custom".into())
        );
    }

    #[test]
    fn android_adapter_software_assurance() {
        let key = FakeKeystoreKeyCustom {
            public_key: vec![1, 2, 3],
            assurance: AndroidKeystoreAssuranceLevel::Software,
        };
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.assurance_level, HardwareAssuranceLevel::Software);
    }

    #[test]
    fn android_adapter_tee_assurance() {
        let key = FakeKeystoreKeyCustom {
            public_key: vec![1, 2, 3],
            assurance: AndroidKeystoreAssuranceLevel::Tee,
        };
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.assurance_level, HardwareAssuranceLevel::IsolatedHardware);
    }

    #[test]
    fn android_adapter_attested_assurance() {
        let key = FakeKeystoreKeyCustom {
            public_key: vec![1, 2, 3],
            assurance: AndroidKeystoreAssuranceLevel::Attested("attest".into()),
        };
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(
            info.assurance_level,
            HardwareAssuranceLevel::Certified("attest".into())
        );
    }

    #[test]
    fn android_adapter_unknown_assurance() {
        let key = FakeKeystoreKeyCustom {
            public_key: vec![1, 2, 3],
            assurance: AndroidKeystoreAssuranceLevel::Unknown,
        };
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.assurance_level, HardwareAssuranceLevel::Unknown);
    }

    // =========================================================================
    // Adapter tests: YubiKeyHardwareKeyAdapter
    // =========================================================================

    #[test]
    fn yubikey_adapter_maps_key_info() {
        let adapter = YubiKeyHardwareKeyAdapter::new(FakeYubiKey);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.provider, HardwareProviderKind::YubiKey);
        assert_eq!(info.algorithm, HardwareSignatureAlgorithm::EcdsaP256Sha256);
        assert_eq!(
            info.assurance_level,
            HardwareAssuranceLevel::IsolatedHardware
        );
    }

    #[test]
    fn yubikey_adapter_sign_message() {
        let adapter = YubiKeyHardwareKeyAdapter::new(FakeYubiKey);
        let message = b"yubikey test";
        let sig = adapter.sign_message(message).unwrap();
        assert_eq!(sig, message.to_vec());
    }

    #[test]
    fn yubikey_adapter_fips_assurance() {
        let key = FakeYubiKeyCustom {
            algorithm: YubiKeySignatureAlgorithm::EcdsaP256Sha256,
            assurance: YubiKeyAssuranceLevel::Fips,
        };
        let adapter = YubiKeyHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(
            info.assurance_level,
            HardwareAssuranceLevel::Certified("fips".into())
        );
    }

    #[test]
    fn yubikey_adapter_software_simulator() {
        let key = FakeYubiKeyCustom {
            algorithm: YubiKeySignatureAlgorithm::EcdsaP256Sha256,
            assurance: YubiKeyAssuranceLevel::SoftwareSimulator,
        };
        let adapter = YubiKeyHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.assurance_level, HardwareAssuranceLevel::Software);
    }

    #[test]
    fn yubikey_adapter_hardware_backed() {
        let key = FakeYubiKeyCustom {
            algorithm: YubiKeySignatureAlgorithm::EcdsaP256Sha256,
            assurance: YubiKeyAssuranceLevel::HardwareBacked,
        };
        let adapter = YubiKeyHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(
            info.assurance_level,
            HardwareAssuranceLevel::IsolatedHardware
        );
    }

    #[test]
    fn yubikey_adapter_certified_assurance() {
        let key = FakeYubiKeyCustom {
            algorithm: YubiKeySignatureAlgorithm::EcdsaP256Sha256,
            assurance: YubiKeyAssuranceLevel::Certified("yubi-cert".into()),
        };
        let adapter = YubiKeyHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(
            info.assurance_level,
            HardwareAssuranceLevel::Certified("yubi-cert".into())
        );
    }

    // =========================================================================
    // Provider-checked signing: TPM routing
    // =========================================================================

    #[test]
    fn provider_checked_sign_routes_to_tpm_helper() {
        let sig = sign_record_with_tpm_provider(
            &FakeTpmKey::new(),
            &HardwareValidationRequirements {
                allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
                ..Default::default()
            },
            "edgerun:v0:sig:test",
            &[2u8; 32],
        )
        .unwrap();
        assert_eq!(
            sig,
            signature_input_for_record("edgerun:v0:sig:test", &[2u8; 32])
        );
    }

    #[test]
    fn provider_checked_sign_tpm_with_multiple_algorithms() {
        let sig = sign_record_with_tpm_provider(
            &FakeTpmKey::with_pkcs1v15(),
            &HardwareValidationRequirements {
                allowed_algorithms: vec![HardwareSignatureAlgorithm::RsaPkcs1v15Sha256],
                ..Default::default()
            },
            "tag",
            &[3u8; 32],
        )
        .unwrap();
        assert_eq!(sig, signature_input_for_record("tag", &[3u8; 32]));
    }

    #[test]
    fn provider_checked_sign_tpm_empty_algorithm_requirements() {
        // When allowed_algorithms is empty, no algorithm filtering is done
        let sig = sign_record_with_tpm_provider(
            &FakeTpmKey::new(),
            &HardwareValidationRequirements {
                allowed_algorithms: vec![],
                ..Default::default()
            },
            "tag",
            &[4u8; 32],
        )
        .unwrap();
        assert_eq!(sig, signature_input_for_record("tag", &[4u8; 32]));
    }

    // =========================================================================
    // Provider-checked signing: Android Keystore routing
    // =========================================================================

    #[test]
    fn provider_checked_sign_routes_to_yubikey_helper() {
        let sig = sign_record_with_yubikey_provider(
            &FakeYubiKey,
            &HardwareValidationRequirements {
                allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
                ..Default::default()
            },
            "edgerun:v0:sig:test",
            &[8u8; 32],
        )
        .unwrap();
        assert_eq!(
            sig,
            signature_input_for_record("edgerun:v0:sig:test", &[8u8; 32])
        );
    }

    #[test]
    fn provider_checked_sign_rejects_unsupported_keystore_mapping() {
        let err = sign_record_with_android_keystore_provider(
            &FakeKeystoreKey::new(),
            &HardwareValidationRequirements {
                allowed_algorithms: vec![HardwareSignatureAlgorithm::EcSchnorr],
                ..Default::default()
            },
            "edgerun:v0:sig:test",
            &[3u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, HardwareSigningError::Provider(_)));
    }

    #[test]
    fn provider_checked_sign_android_ecdsa_p256() {
        let key = FakeKeystoreKey::with_algorithm(
            AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256,
        );
        let sig = sign_record_with_android_keystore_provider(
            &key,
            &HardwareValidationRequirements {
                allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
                ..Default::default()
            },
            "tag",
            &[5u8; 32],
        )
        .unwrap();
        assert_eq!(sig, signature_input_for_record("tag", &[5u8; 32]));
    }

    #[test]
    fn provider_checked_sign_android_empty_algorithm_requirements() {
        // When allowed_algorithms is empty, the underlying check fails because
        // the key's algorithm is not in the empty list.
        let err = sign_record_with_android_keystore_provider(
            &FakeKeystoreKey::new(),
            &HardwareValidationRequirements {
                allowed_algorithms: vec![],
                ..Default::default()
            },
            "tag",
            &[6u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, HardwareSigningError::AndroidKeystore(_)));
    }

    // =========================================================================
    // Provider-checked signing: YubiKey routing
    // =========================================================================

    #[test]
    fn provider_checked_sign_yubikey_rejects_ecschnorr() {
        let err = sign_record_with_yubikey_provider(
            &FakeYubiKeyCustom {
                algorithm: YubiKeySignatureAlgorithm::EcdsaP256Sha256,
                assurance: YubiKeyAssuranceLevel::HardwareBacked,
            },
            &HardwareValidationRequirements {
                allowed_algorithms: vec![HardwareSignatureAlgorithm::EcSchnorr],
                ..Default::default()
            },
            "tag",
            &[7u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, HardwareSigningError::Provider(_)));
        let msg = format!("{}", err);
        assert!(msg.contains("YubiKey adapter does not expose EcSchnorr"));
    }

    #[test]
    fn provider_checked_sign_yubikey_empty_algorithm_requirements() {
        // When allowed_algorithms is empty, the underlying check fails because
        // the key's algorithm is not in the empty list.
        let err = sign_record_with_yubikey_provider(
            &FakeYubiKey,
            &HardwareValidationRequirements {
                allowed_algorithms: vec![],
                ..Default::default()
            },
            "tag",
            &[9u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, HardwareSigningError::YubiKey(_)));
    }

    // =========================================================================
    // Algorithm mapping tests: TPM
    // =========================================================================

    #[test]
    fn map_tpm_algorithm_all_variants() {
        assert_eq!(
            map_tpm_algorithm(&TpmSignatureAlgorithm::RsaPkcs1v15Sha256),
            HardwareSignatureAlgorithm::RsaPkcs1v15Sha256
        );
        assert_eq!(
            map_tpm_algorithm(&TpmSignatureAlgorithm::RsaPssSha256),
            HardwareSignatureAlgorithm::RsaPssSha256
        );
        assert_eq!(
            map_tpm_algorithm(&TpmSignatureAlgorithm::EcdsaP256Sha256),
            HardwareSignatureAlgorithm::EcdsaP256Sha256
        );
        assert_eq!(
            map_tpm_algorithm(&TpmSignatureAlgorithm::EcdsaP384Sha384),
            HardwareSignatureAlgorithm::EcdsaP384Sha384
        );
        assert_eq!(
            map_tpm_algorithm(&TpmSignatureAlgorithm::EcSchnorr),
            HardwareSignatureAlgorithm::EcSchnorr
        );
        assert_eq!(
            map_tpm_algorithm(&TpmSignatureAlgorithm::Eddsa),
            HardwareSignatureAlgorithm::Eddsa
        );
        assert_eq!(
            map_tpm_algorithm(&TpmSignatureAlgorithm::Opaque("custom".into())),
            HardwareSignatureAlgorithm::Opaque("custom".into())
        );
    }

    #[test]
    fn map_hardware_to_tpm_algorithm_all_variants() {
        assert_eq!(
            map_hardware_to_tpm_algorithm(&HardwareSignatureAlgorithm::RsaPkcs1v15Sha256),
            Ok(TpmSignatureAlgorithm::RsaPkcs1v15Sha256)
        );
        assert_eq!(
            map_hardware_to_tpm_algorithm(&HardwareSignatureAlgorithm::RsaPssSha256),
            Ok(TpmSignatureAlgorithm::RsaPssSha256)
        );
        assert_eq!(
            map_hardware_to_tpm_algorithm(&HardwareSignatureAlgorithm::EcdsaP256Sha256),
            Ok(TpmSignatureAlgorithm::EcdsaP256Sha256)
        );
        assert_eq!(
            map_hardware_to_tpm_algorithm(&HardwareSignatureAlgorithm::EcdsaP384Sha384),
            Ok(TpmSignatureAlgorithm::EcdsaP384Sha384)
        );
        assert_eq!(
            map_hardware_to_tpm_algorithm(&HardwareSignatureAlgorithm::EcSchnorr),
            Ok(TpmSignatureAlgorithm::EcSchnorr)
        );
        assert_eq!(
            map_hardware_to_tpm_algorithm(&HardwareSignatureAlgorithm::Eddsa),
            Ok(TpmSignatureAlgorithm::Eddsa)
        );
        assert_eq!(
            map_hardware_to_tpm_algorithm(&HardwareSignatureAlgorithm::Opaque("x".into())),
            Ok(TpmSignatureAlgorithm::Opaque("x".into()))
        );
    }

    // =========================================================================
    // Algorithm mapping tests: Android Keystore
    // =========================================================================

    #[test]
    fn map_keystore_algorithm_all_variants() {
        assert_eq!(
            map_keystore_algorithm(&AndroidKeystoreSignatureAlgorithm::RsaPkcs1v15Sha256),
            HardwareSignatureAlgorithm::RsaPkcs1v15Sha256
        );
        assert_eq!(
            map_keystore_algorithm(&AndroidKeystoreSignatureAlgorithm::RsaPssSha256),
            HardwareSignatureAlgorithm::RsaPssSha256
        );
        assert_eq!(
            map_keystore_algorithm(&AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256),
            HardwareSignatureAlgorithm::EcdsaP256Sha256
        );
        assert_eq!(
            map_keystore_algorithm(&AndroidKeystoreSignatureAlgorithm::EcdsaP384Sha384),
            HardwareSignatureAlgorithm::EcdsaP384Sha384
        );
        assert_eq!(
            map_keystore_algorithm(&AndroidKeystoreSignatureAlgorithm::Eddsa),
            HardwareSignatureAlgorithm::Eddsa
        );
        assert_eq!(
            map_keystore_algorithm(&AndroidKeystoreSignatureAlgorithm::Opaque("k".into())),
            HardwareSignatureAlgorithm::Opaque("k".into())
        );
    }

    #[test]
    fn map_hardware_to_keystore_algorithm_all_variants() {
        assert_eq!(
            map_hardware_to_keystore_algorithm(&HardwareSignatureAlgorithm::RsaPkcs1v15Sha256),
            Ok(AndroidKeystoreSignatureAlgorithm::RsaPkcs1v15Sha256)
        );
        assert_eq!(
            map_hardware_to_keystore_algorithm(&HardwareSignatureAlgorithm::RsaPssSha256),
            Ok(AndroidKeystoreSignatureAlgorithm::RsaPssSha256)
        );
        assert_eq!(
            map_hardware_to_keystore_algorithm(&HardwareSignatureAlgorithm::EcdsaP256Sha256),
            Ok(AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256)
        );
        assert_eq!(
            map_hardware_to_keystore_algorithm(&HardwareSignatureAlgorithm::EcdsaP384Sha384),
            Ok(AndroidKeystoreSignatureAlgorithm::EcdsaP384Sha384)
        );
        assert_eq!(
            map_hardware_to_keystore_algorithm(&HardwareSignatureAlgorithm::Eddsa),
            Ok(AndroidKeystoreSignatureAlgorithm::Eddsa)
        );
        assert_eq!(
            map_hardware_to_keystore_algorithm(&HardwareSignatureAlgorithm::Opaque("k".into())),
            Ok(AndroidKeystoreSignatureAlgorithm::Opaque("k".into()))
        );
    }

    #[test]
    fn map_hardware_to_keystore_algorithm_rejects_ecschnorr() {
        let err =
            map_hardware_to_keystore_algorithm(&HardwareSignatureAlgorithm::EcSchnorr).unwrap_err();
        assert!(matches!(err, HardwareSigningError::Provider(_)));
        let msg = format!("{}", err);
        assert!(msg.contains("Android Keystore adapter does not expose EcSchnorr"));
    }

    // =========================================================================
    // Algorithm mapping tests: YubiKey
    // =========================================================================

    #[test]
    fn map_yubikey_algorithm_all_variants() {
        assert_eq!(
            map_yubikey_algorithm(&YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256),
            HardwareSignatureAlgorithm::RsaPkcs1v15Sha256
        );
        assert_eq!(
            map_yubikey_algorithm(&YubiKeySignatureAlgorithm::RsaPssSha256),
            HardwareSignatureAlgorithm::RsaPssSha256
        );
        assert_eq!(
            map_yubikey_algorithm(&YubiKeySignatureAlgorithm::EcdsaP256Sha256),
            HardwareSignatureAlgorithm::EcdsaP256Sha256
        );
        assert_eq!(
            map_yubikey_algorithm(&YubiKeySignatureAlgorithm::EcdsaP384Sha384),
            HardwareSignatureAlgorithm::EcdsaP384Sha384
        );
        assert_eq!(
            map_yubikey_algorithm(&YubiKeySignatureAlgorithm::Eddsa),
            HardwareSignatureAlgorithm::Eddsa
        );
        assert_eq!(
            map_yubikey_algorithm(&YubiKeySignatureAlgorithm::Opaque("y".into())),
            HardwareSignatureAlgorithm::Opaque("y".into())
        );
    }

    #[test]
    fn map_hardware_to_yubikey_algorithm_all_variants() {
        assert_eq!(
            map_hardware_to_yubikey_algorithm(&HardwareSignatureAlgorithm::RsaPkcs1v15Sha256),
            Ok(YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256)
        );
        assert_eq!(
            map_hardware_to_yubikey_algorithm(&HardwareSignatureAlgorithm::RsaPssSha256),
            Ok(YubiKeySignatureAlgorithm::RsaPssSha256)
        );
        assert_eq!(
            map_hardware_to_yubikey_algorithm(&HardwareSignatureAlgorithm::EcdsaP256Sha256),
            Ok(YubiKeySignatureAlgorithm::EcdsaP256Sha256)
        );
        assert_eq!(
            map_hardware_to_yubikey_algorithm(&HardwareSignatureAlgorithm::EcdsaP384Sha384),
            Ok(YubiKeySignatureAlgorithm::EcdsaP384Sha384)
        );
        assert_eq!(
            map_hardware_to_yubikey_algorithm(&HardwareSignatureAlgorithm::Eddsa),
            Ok(YubiKeySignatureAlgorithm::Eddsa)
        );
        assert_eq!(
            map_hardware_to_yubikey_algorithm(&HardwareSignatureAlgorithm::Opaque("y".into())),
            Ok(YubiKeySignatureAlgorithm::Opaque("y".into()))
        );
    }

    #[test]
    fn map_hardware_to_yubikey_algorithm_rejects_ecschnorr() {
        let err =
            map_hardware_to_yubikey_algorithm(&HardwareSignatureAlgorithm::EcSchnorr).unwrap_err();
        assert!(matches!(err, HardwareSigningError::Provider(_)));
        let msg = format!("{}", err);
        assert!(msg.contains("YubiKey adapter does not expose EcSchnorr"));
    }

    // =========================================================================
    // Assurance mapping tests
    // =========================================================================

    #[test]
    fn map_tpm_assurance_all_variants() {
        assert_eq!(
            map_tpm_assurance(&TpmAssuranceLevel::SoftwareSimulated),
            HardwareAssuranceLevel::Software
        );
        assert_eq!(
            map_tpm_assurance(&TpmAssuranceLevel::DiscreteTpm),
            HardwareAssuranceLevel::IsolatedHardware
        );
        assert_eq!(
            map_tpm_assurance(&TpmAssuranceLevel::IntegratedTpm),
            HardwareAssuranceLevel::IsolatedHardware
        );
        assert_eq!(
            map_tpm_assurance(&TpmAssuranceLevel::Certified("c".into())),
            HardwareAssuranceLevel::Certified("c".into())
        );
        assert_eq!(
            map_tpm_assurance(&TpmAssuranceLevel::Unknown),
            HardwareAssuranceLevel::Unknown
        );
    }

    #[test]
    fn map_keystore_assurance_all_variants() {
        assert_eq!(
            map_keystore_assurance(&AndroidKeystoreAssuranceLevel::Software),
            HardwareAssuranceLevel::Software
        );
        assert_eq!(
            map_keystore_assurance(&AndroidKeystoreAssuranceLevel::Tee),
            HardwareAssuranceLevel::IsolatedHardware
        );
        assert_eq!(
            map_keystore_assurance(&AndroidKeystoreAssuranceLevel::StrongBox),
            HardwareAssuranceLevel::StrongBox
        );
        assert_eq!(
            map_keystore_assurance(&AndroidKeystoreAssuranceLevel::Attested("a".into())),
            HardwareAssuranceLevel::Certified("a".into())
        );
        assert_eq!(
            map_keystore_assurance(&AndroidKeystoreAssuranceLevel::Unknown),
            HardwareAssuranceLevel::Unknown
        );
    }

    #[test]
    fn map_yubikey_assurance_all_variants() {
        assert_eq!(
            map_yubikey_assurance(&YubiKeyAssuranceLevel::SoftwareSimulator),
            HardwareAssuranceLevel::Software
        );
        assert_eq!(
            map_yubikey_assurance(&YubiKeyAssuranceLevel::HardwareBacked),
            HardwareAssuranceLevel::IsolatedHardware
        );
        assert_eq!(
            map_yubikey_assurance(&YubiKeyAssuranceLevel::PivAttested),
            HardwareAssuranceLevel::IsolatedHardware
        );
        assert_eq!(
            map_yubikey_assurance(&YubiKeyAssuranceLevel::Fips),
            HardwareAssuranceLevel::Certified("fips".into())
        );
        assert_eq!(
            map_yubikey_assurance(&YubiKeyAssuranceLevel::Certified("c".into())),
            HardwareAssuranceLevel::Certified("c".into())
        );
        assert_eq!(
            map_yubikey_assurance(&YubiKeyAssuranceLevel::Unknown),
            HardwareAssuranceLevel::Unknown
        );
    }

    // =========================================================================
    // signature_input_for_record tests
    // =========================================================================

    #[test]
    fn signature_input_for_record_produces_nonempty_output() {
        let input = signature_input_for_record("edgerun:v0:sig:test", &[0xAA; 32]);
        assert!(!input.is_empty());
    }

    #[test]
    fn signature_input_for_record_deterministic() {
        let input1 = signature_input_for_record("tag", &[1, 2, 3]);
        let input2 = signature_input_for_record("tag", &[1, 2, 3]);
        assert_eq!(input1, input2);
    }

    #[test]
    fn signature_input_for_record_different_tags() {
        let input1 = signature_input_for_record("tag1", &[1, 2, 3]);
        let input2 = signature_input_for_record("tag2", &[1, 2, 3]);
        assert_ne!(input1, input2);
    }

    #[test]
    fn signature_input_for_record_different_hashes() {
        let input1 = signature_input_for_record("tag", &[1, 2, 3]);
        let input2 = signature_input_for_record("tag", &[4, 5, 6]);
        assert_ne!(input1, input2);
    }

    // =========================================================================
    // Constants tests
    // =========================================================================

    #[test]
    fn mesh_public_key_length_is_64() {
        assert_eq!(MESH_PUBLIC_KEY_LENGTH, 64);
    }

    #[test]
    fn mesh_signature_length_is_64() {
        assert_eq!(MESH_SIGNATURE_LENGTH, 64);
    }

    // =========================================================================
    // Edge cases: invalid keys, wrong algorithms, missing fields
    // =========================================================================

    #[test]
    fn hardware_key_info_with_all_fields_populated() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "full-key".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![0xFF; 64],
            attestation: vec![vec![1], vec![2], vec![3]],
            assurance_level: HardwareAssuranceLevel::Certified("cert".into()),
            biometric_state: BiometricState {
                modality: Some(BiometricModality::Fingerprint),
                verified: true,
                hardware_protected: true,
                user_present: true,
            },
        };
        assert!(info.is_mesh_capable());
        let node_id = info.node_id().unwrap();
        assert_eq!(node_id.0, [0xFF; 64]);
    }

    #[test]
    fn validation_with_all_requirements_fails_on_provider() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::YubiKey,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::Eddsa,
            public_key: vec![],
            attestation: vec![],
            assurance_level: HardwareAssuranceLevel::Software,
            biometric_state: BiometricState::default(),
        };
        let reqs = HardwareValidationRequirements {
            allowed_providers: vec![HardwareProviderKind::Tpm],
            allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
            minimum_assurance: Some(HardwareAssuranceStrength::IsolatedHardware),
            require_attestation: true,
            require_public_key: true,
            minimum_biometric_assurance: Some(BiometricAssuranceStrength::BiometricMatch),
        };
        // First failing check is provider
        let err = validate_hardware_key_info(&info, &reqs).unwrap_err();
        assert!(matches!(
            err,
            HardwareSigningError::Validation(HardwareValidationIssue::ProviderNotAllowed(
                HardwareProviderKind::YubiKey
            ))
        ));
    }

    #[test]
    fn validation_with_all_requirements_passes() {
        let info = HardwareKeyInfo {
            provider: HardwareProviderKind::Tpm,
            key_name: "k".into(),
            algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![0xAA; 64],
            attestation: vec![vec![1, 2, 3]],
            assurance_level: HardwareAssuranceLevel::Certified("cert".into()),
            biometric_state: BiometricState {
                modality: Some(BiometricModality::Face),
                verified: true,
                hardware_protected: true,
                user_present: true,
            },
        };
        let reqs = HardwareValidationRequirements {
            allowed_providers: vec![HardwareProviderKind::Tpm],
            allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
            minimum_assurance: Some(HardwareAssuranceStrength::Certified),
            require_attestation: true,
            require_public_key: true,
            minimum_biometric_assurance: Some(BiometricAssuranceStrength::HardwareProtectedBiometric),
        };
        validate_hardware_key_info(&info, &reqs).unwrap();
    }

    #[test]
    fn hardware_mesh_signer_key_info_failure_propagates() {
        struct FailingKeyInfo;
        impl HardwareSigningKey for FailingKeyInfo {
            fn key_info(&self) -> Result<HardwareKeyInfo, HardwareSigningError> {
                Err(HardwareSigningError::Provider("key info failed".into()))
            }
            fn sign_message(&self, _message: &[u8]) -> Result<Vec<u8>, HardwareSigningError> {
                Ok(vec![0u8; 64])
            }
        }
        match HardwareMeshSigner::new(FailingKeyInfo) {
            Ok(_) => panic!("should have failed"),
            Err(e) => assert_eq!(e, HardwareSigningError::Provider("key info failed".into())),
        }
    }
}
