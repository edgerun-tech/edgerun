//! YubiKey PIV backend for hardware signing.
//!
//! Gate with `feature = "yubikey"`.

use alloc::{vec, vec::Vec};

use edgerun_yubikey::{
    YubiKeyAssuranceLevel, YubiKeySignatureAlgorithm, YubiKeySigningKey,
    sign_record_with_yubikey_checked,
};

use crate::{
    BiometricState, HardwareAssuranceLevel, HardwareKeyInfo, HardwareProviderKind,
    HardwareSignatureAlgorithm, HardwareSigningError, HardwareSigningKey,
    HardwareValidationRequirements,
};

// ===========================================================================
// Adapter
// ===========================================================================

/// Wraps any `YubiKeySigningKey` as a `HardwareSigningKey`.
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

// ===========================================================================
// Provider-level signing
// ===========================================================================

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

// ===========================================================================
// Mapping functions
// ===========================================================================

fn provider_requirements_for_yubikey(
    requirements: &HardwareValidationRequirements,
) -> Result<Vec<YubiKeySignatureAlgorithm>, HardwareSigningError> {
    if requirements.allowed_algorithms.is_empty() {
        return Ok(vec![
            YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256,
            YubiKeySignatureAlgorithm::RsaPssSha256,
            YubiKeySignatureAlgorithm::EcdsaP256Sha256,
            YubiKeySignatureAlgorithm::EcdsaP384Sha384,
            YubiKeySignatureAlgorithm::Eddsa,
        ]);
    }
    requirements
        .allowed_algorithms
        .iter()
        .map(map_hardware_to_yubikey_algorithm)
        .collect()
}

pub fn map_yubikey_algorithm(algorithm: &YubiKeySignatureAlgorithm) -> HardwareSignatureAlgorithm {
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

pub fn map_yubikey_assurance(level: &YubiKeyAssuranceLevel) -> HardwareAssuranceLevel {
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
            ));
        }
        HardwareSignatureAlgorithm::Opaque(v) => YubiKeySignatureAlgorithm::Opaque(v.clone()),
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
    use edgerun_yubikey::{
        YubiKeyAssuranceLevel, YubiKeyError, YubiKeyKeyInfo, YubiKeyPinPolicy,
        YubiKeySignatureAlgorithm, YubiKeyTouchPolicy,
    };

    // -----------------------------------------------------------------------
    // Fake YubiKey keys for testing
    // -----------------------------------------------------------------------

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
                pin_policy: Some(YubiKeyPinPolicy::Always),
                touch_policy: Some(YubiKeyTouchPolicy::Always),
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
        public_key: Vec<u8>,
    }

    impl YubiKeySigningKey for FakeYubiKeyCustom {
        fn key_info(&self) -> Result<YubiKeyKeyInfo, YubiKeyError> {
            Ok(YubiKeyKeyInfo {
                slot: "9a".into(),
                algorithm: self.algorithm.clone(),
                public_key: self.public_key.clone(),
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

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[test]
    fn yubikey_adapter_maps_key_info() {
        let key = FakeYubiKey;
        let adapter = YubiKeyHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.provider, HardwareProviderKind::YubiKey);
        assert_eq!(info.key_name, "9c");
        assert_eq!(info.algorithm, HardwareSignatureAlgorithm::EcdsaP256Sha256);
        assert_eq!(info.public_key, vec![6, 5, 4]);
        assert_eq!(info.attestation, vec![vec![3, 2, 1]]);
        assert_eq!(
            info.assurance_level,
            HardwareAssuranceLevel::IsolatedHardware
        );
    }

    #[test]
    fn yubikey_adapter_sign_passes_through() {
        let key = FakeYubiKey;
        let adapter = YubiKeyHardwareKeyAdapter::new(key);
        let sig = adapter.sign_message(b"hello").unwrap();
        assert_eq!(sig, b"hello");
    }

    #[test]
    fn map_yubikey_algorithm_variants() {
        assert_eq!(
            map_yubikey_algorithm(&YubiKeySignatureAlgorithm::RsaPkcs1v15Sha256),
            HardwareSignatureAlgorithm::RsaPkcs1v15Sha256
        );
        assert_eq!(
            map_yubikey_algorithm(&YubiKeySignatureAlgorithm::Eddsa),
            HardwareSignatureAlgorithm::Eddsa
        );
    }

    #[test]
    fn map_yubikey_assurance_variants() {
        assert_eq!(
            map_yubikey_assurance(&YubiKeyAssuranceLevel::SoftwareSimulator),
            HardwareAssuranceLevel::Software
        );
        assert_eq!(
            map_yubikey_assurance(&YubiKeyAssuranceLevel::HardwareBacked),
            HardwareAssuranceLevel::IsolatedHardware
        );
        assert_eq!(
            map_yubikey_assurance(&YubiKeyAssuranceLevel::Fips),
            HardwareAssuranceLevel::Certified("fips".into())
        );
    }

    #[test]
    fn sign_record_with_yubikey_provider_success() {
        let key = FakeYubiKey;
        let req = HardwareValidationRequirements::default();
        let sig = sign_record_with_yubikey_provider(&key, &req, "test:v0:sig", b"hash").unwrap();
        assert_eq!(
            sig,
            crate::signature_input_for_record("test:v0:sig", b"hash")
        );
    }

    #[test]
    fn yubikey_provider_with_custom_pk() {
        let mut pk = [0u8; MESH_PUBLIC_KEY_LENGTH];
        pk[0] = 0xDD;
        let key = FakeYubiKeyCustom {
            algorithm: YubiKeySignatureAlgorithm::EcdsaP256Sha256,
            assurance: YubiKeyAssuranceLevel::HardwareBacked,
            public_key: pk.to_vec(),
        };
        let adapter = YubiKeyHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.public_key, pk.to_vec());
        assert_eq!(
            info.assurance_level,
            HardwareAssuranceLevel::IsolatedHardware
        );
        assert!(info.is_mesh_capable());
    }
}
