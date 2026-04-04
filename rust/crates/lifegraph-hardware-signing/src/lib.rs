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
}
