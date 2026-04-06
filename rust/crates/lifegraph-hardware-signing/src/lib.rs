use lifegraph_android_keystore::{
    sign_record_with_keystore_checked, AndroidKeystoreAssuranceLevel, AndroidKeystoreError,
    AndroidKeystoreSignatureAlgorithm, AndroidKeystoreSigningKey,
};
use lifegraph_biometrics::{BiometricAssuranceStrength, BiometricState};
use lifegraph_core::crypto::signature_input;
use lifegraph_tpm::{
    sign_record_with_tpm_checked, TpmAssuranceLevel, TpmError, TpmSignatureAlgorithm, TpmSigningKey,
};
use lifegraph_yubikey::{
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
        lifegraph_core::util::bytes_to_hex(&self.0[..4])
    }

    /// Full hex representation.
    pub fn to_hex(&self) -> String {
        lifegraph_core::util::bytes_to_hex(&self.0)
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
        if !key_info.assurance_level.is_at_least(minimum.clone()) {
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
    use lifegraph_android_keystore::{
        AndroidKeystoreAssuranceLevel, AndroidKeystoreKeyInfo, AndroidKeystoreSignatureAlgorithm,
    };
    use lifegraph_tpm::{TpmAssuranceLevel, TpmKeyInfo, TpmSignatureAlgorithm};
    use lifegraph_yubikey::{YubiKeyAssuranceLevel, YubiKeyKeyInfo, YubiKeySignatureAlgorithm};

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
    fn mesh_algorithm_is_ecdsa_p256() {
        assert!(MESH_ALGORITHM.is_ecdsa_p256());
    }

    struct FakeTpmKey;
    impl TpmSigningKey for FakeTpmKey {
        fn key_info(&self) -> Result<TpmKeyInfo, TpmError> {
            Ok(TpmKeyInfo {
                key_name: "tpm-key".into(),
                algorithm: TpmSignatureAlgorithm::EcdsaP256Sha256,
                public_key: vec![1, 2, 3],
                attestation_blob: Some(vec![9, 9]),
                assurance_level: TpmAssuranceLevel::DiscreteTpm,
            })
        }
        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, TpmError> {
            Ok(message.to_vec())
        }
    }

    struct FakeKeystoreKey;
    impl AndroidKeystoreSigningKey for FakeKeystoreKey {
        fn key_info(&self) -> Result<AndroidKeystoreKeyInfo, AndroidKeystoreError> {
            Ok(AndroidKeystoreKeyInfo {
                alias: "android-key".into(),
                algorithm: AndroidKeystoreSignatureAlgorithm::Eddsa,
                public_key: vec![4, 5, 6],
                attestation_chain: vec![vec![7, 8]],
                assurance_level: AndroidKeystoreAssuranceLevel::StrongBox,
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
                pin_policy: Some(lifegraph_yubikey::YubiKeyPinPolicy::Always),
                touch_policy: Some(lifegraph_yubikey::YubiKeyTouchPolicy::Always),
                generated_on_device: Some(true),
            })
        }
        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, YubiKeyError> {
            Ok(message.to_vec())
        }
    }

    #[test]
    fn tpm_adapter_maps_key_info() {
        let adapter = TpmHardwareKeyAdapter::new(FakeTpmKey);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.provider, HardwareProviderKind::Tpm);
        assert_eq!(info.algorithm, HardwareSignatureAlgorithm::EcdsaP256Sha256);
        assert_eq!(
            info.assurance_level,
            HardwareAssuranceLevel::IsolatedHardware
        );
    }

    #[test]
    fn android_adapter_maps_key_info() {
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(FakeKeystoreKey);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.provider, HardwareProviderKind::AndroidKeystore);
        assert_eq!(info.algorithm, HardwareSignatureAlgorithm::Eddsa);
        assert_eq!(info.assurance_level, HardwareAssuranceLevel::StrongBox);
    }

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
    fn hardware_checked_sign_rejects_unexpected_algorithm() {
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(FakeKeystoreKey);
        let err = sign_record_with_hardware_checked(
            &adapter,
            &HardwareValidationRequirements {
                allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
                ..Default::default()
            },
            "lifegraph:v0:sig:test",
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
    fn provider_checked_sign_routes_to_tpm_helper() {
        let sig = sign_record_with_tpm_provider(
            &FakeTpmKey,
            &HardwareValidationRequirements {
                allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
                ..Default::default()
            },
            "lifegraph:v0:sig:test",
            &[2u8; 32],
        )
        .unwrap();
        assert_eq!(
            sig,
            signature_input_for_record("lifegraph:v0:sig:test", &[2u8; 32])
        );
    }

    #[test]
    fn provider_checked_sign_rejects_unsupported_keystore_mapping() {
        let err = sign_record_with_android_keystore_provider(
            &FakeKeystoreKey,
            &HardwareValidationRequirements {
                allowed_algorithms: vec![HardwareSignatureAlgorithm::EcSchnorr],
                ..Default::default()
            },
            "lifegraph:v0:sig:test",
            &[3u8; 32],
        )
        .unwrap_err();
        assert!(matches!(err, HardwareSigningError::Provider(_)));
    }

    #[test]
    fn provider_checked_sign_routes_to_yubikey_helper() {
        let sig = sign_record_with_yubikey_provider(
            &FakeYubiKey,
            &HardwareValidationRequirements {
                allowed_algorithms: vec![HardwareSignatureAlgorithm::EcdsaP256Sha256],
                ..Default::default()
            },
            "lifegraph:v0:sig:test",
            &[8u8; 32],
        )
        .unwrap();
        assert_eq!(
            sig,
            signature_input_for_record("lifegraph:v0:sig:test", &[8u8; 32])
        );
    }

    #[test]
    fn hardware_mesh_signer_extracts_node_id_and_signs_digest() {
        // Use the FakeTpmKey from existing tests
        struct FakeMeshKey;
        impl HardwareSigningKey for FakeMeshKey {
            fn key_info(&self) -> Result<HardwareKeyInfo, HardwareSigningError> {
                // ECDSA P-256 public key (64 bytes)
                let mut pk = [0u8; 64];
                pk[0] = 0x04; // marker byte, not used
                pk[1] = 0xAB;
                // Rest zeros
                Ok(HardwareKeyInfo {
                    provider: HardwareProviderKind::Tpm,
                    key_name: "fake-key".into(),
                    algorithm: HardwareSignatureAlgorithm::EcdsaP256Sha256,
                    public_key: pk.to_vec(),
                    attestation: vec![],
                    assurance_level: HardwareAssuranceLevel::IsolatedHardware,
                    biometric_state: BiometricState::default(),
                })
            }
            fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, HardwareSigningError> {
                // Simulate hardware: return 64-byte "signature" based on digest
                let mut sig = [0u8; 64];
                sig[..32].copy_from_slice(message);
                sig[32..].copy_from_slice(&message.iter().map(|b| !b).collect::<Vec<_>>()[..32]);
                Ok(sig.to_vec())
            }
        }

        let key = FakeMeshKey;
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
}
