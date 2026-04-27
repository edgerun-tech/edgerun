//! Android Keystore backend for hardware signing.
//!
//! Gate with `feature = "android-keystore"`.

use alloc::vec::Vec;

use edgerun_android_keystore::{
    sign_record_with_keystore_checked, AndroidKeystoreAssuranceLevel,
    AndroidKeystoreSignatureAlgorithm, AndroidKeystoreSigningKey,
};

use crate::{
    BiometricState, HardwareAssuranceLevel, HardwareKeyInfo, HardwareProviderKind,
    HardwareSignatureAlgorithm, HardwareSigningError, HardwareSigningKey,
    HardwareValidationRequirements,
};

// ===========================================================================
// Adapter
// ===========================================================================

/// Wraps any `AndroidKeystoreSigningKey` as a `HardwareSigningKey`.
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

// ===========================================================================
// Provider-level signing
// ===========================================================================

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

// ===========================================================================
// Mapping functions
// ===========================================================================

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

pub fn map_keystore_algorithm(
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

pub fn map_keystore_assurance(level: &AndroidKeystoreAssuranceLevel) -> HardwareAssuranceLevel {
    match level {
        AndroidKeystoreAssuranceLevel::Software => HardwareAssuranceLevel::Software,
        AndroidKeystoreAssuranceLevel::Tee => HardwareAssuranceLevel::IsolatedHardware,
        AndroidKeystoreAssuranceLevel::StrongBox => HardwareAssuranceLevel::StrongBox,
        AndroidKeystoreAssuranceLevel::Attested(v) => HardwareAssuranceLevel::Certified(v.clone()),
        AndroidKeystoreAssuranceLevel::Unknown => HardwareAssuranceLevel::Unknown,
    }
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

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{HardwareAssuranceStrength, MESH_PUBLIC_KEY_LENGTH};
    use alloc::vec;
    use edgerun_android_keystore::{
        AndroidKeystoreAssuranceLevel, AndroidKeystoreError, AndroidKeystoreKeyInfo,
        AndroidKeystoreSignatureAlgorithm,
    };

    // -----------------------------------------------------------------------
    // Fake keystore keys for testing
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[test]
    fn keystore_adapter_maps_key_info() {
        let key = FakeKeystoreKey::new();
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.provider, HardwareProviderKind::AndroidKeystore);
        assert_eq!(info.key_name, "android-key");
        assert_eq!(info.algorithm, HardwareSignatureAlgorithm::Eddsa);
        assert_eq!(info.public_key, vec![4, 5, 6]);
        assert_eq!(info.attestation, vec![vec![7, 8]]);
        assert_eq!(info.assurance_level, HardwareAssuranceLevel::StrongBox);
    }

    #[test]
    fn keystore_adapter_sign_passes_through() {
        let key = FakeKeystoreKey::new();
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(key);
        let sig = adapter.sign_message(b"hello").unwrap();
        assert_eq!(sig, b"hello");
    }

    #[test]
    fn map_keystore_algorithm_variants() {
        assert_eq!(
            map_keystore_algorithm(&AndroidKeystoreSignatureAlgorithm::RsaPkcs1v15Sha256),
            HardwareSignatureAlgorithm::RsaPkcs1v15Sha256
        );
        assert_eq!(
            map_keystore_algorithm(&AndroidKeystoreSignatureAlgorithm::EcdsaP384Sha384),
            HardwareSignatureAlgorithm::EcdsaP384Sha384
        );
    }

    #[test]
    fn map_keystore_assurance_variants() {
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
    }

    #[test]
    #[ignore = "requires Android hardware with Keystore"]
    fn sign_record_with_android_keystore_provider_success() {
        let key =
            FakeKeystoreKey::with_algorithm(AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256);
        let req = HardwareValidationRequirements::default();
        let sig =
            sign_record_with_android_keystore_provider(&key, &req, "test:v0:sig", b"hash").unwrap();
        assert_eq!(sig, b"hash");
    }

    #[test]
    fn keystore_provider_with_custom_pk() {
        let mut pk = [0u8; MESH_PUBLIC_KEY_LENGTH];
        pk[0] = 0xCC;
        let key = FakeKeystoreKeyCustom {
            public_key: pk.to_vec(),
            assurance: AndroidKeystoreAssuranceLevel::Tee,
        };
        let adapter = AndroidKeystoreHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.public_key, pk.to_vec());
        assert_eq!(
            info.assurance_level,
            HardwareAssuranceLevel::IsolatedHardware
        );
        assert!(info.is_mesh_capable());
    }
}
