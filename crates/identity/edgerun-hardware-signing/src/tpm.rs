//! TPM 2.0 backend for hardware signing.
//!
//! Gate with `feature = "tpm"`.

use alloc::vec::Vec;

use edgerun_tpm::{
    TpmAssuranceLevel, TpmSignatureAlgorithm, TpmSigningKey, sign_record_with_tpm_checked,
};

use crate::{
    BiometricState, HardwareAssuranceLevel, HardwareKeyInfo, HardwareProviderKind,
    HardwareSignatureAlgorithm, HardwareSigningError, HardwareSigningKey,
    HardwareValidationRequirements,
};

// ===========================================================================
// Adapter
// ===========================================================================

/// Wraps any `TpmSigningKey` as a `HardwareSigningKey`.
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
        let sig = self.inner.sign_message(message)?;
        // For ECDSA, the TPM returns: scheme(2) + hashAlg(2) + r_size(2) + r + s_size(2) + s
        // The mesh expects just r || s (64 bytes for P-256).
        // Strip the TPM signature wrapper for ECDSA.
        let key_info = self.inner.key_info().map_err(HardwareSigningError::from)?;
        if matches!(
            key_info.algorithm,
            TpmSignatureAlgorithm::EcdsaP256Sha256 | TpmSignatureAlgorithm::EcdsaP384Sha384
        ) {
            if sig.len() < 8 {
                return Err(HardwareSigningError::Provider(
                    "TPM ECDSA signature too short".into(),
                ));
            }
            // Skip scheme(2) + hashAlg(2) + r_size(2)
            let r_start: usize = 6;
            let r_size: usize = u16::from_be_bytes([sig[4], sig[5]]) as usize;
            let r_end: usize = r_start + r_size;
            // Skip s_size(2)
            let s_start: usize = r_end + 2;
            let s_size: usize = u16::from_be_bytes([sig[s_start - 2], sig[s_start - 1]]) as usize;
            let s_end: usize = s_start + s_size;

            if s_end != sig.len() {
                return Err(HardwareSigningError::Provider(
                    "TPM ECDSA signature size mismatch".into(),
                ));
            }

            let mut out = Vec::with_capacity(r_size + s_size);
            out.extend_from_slice(&sig[r_start..r_end]);
            out.extend_from_slice(&sig[s_start..s_end]);
            Ok(out)
        } else {
            Ok(sig)
        }
    }
}

// ===========================================================================
// Provider-level signing
// ===========================================================================

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

// ===========================================================================
// Mapping functions
// ===========================================================================

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

pub fn map_tpm_algorithm(algorithm: &TpmSignatureAlgorithm) -> HardwareSignatureAlgorithm {
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

pub fn map_tpm_assurance(level: &TpmAssuranceLevel) -> HardwareAssuranceLevel {
    match level {
        TpmAssuranceLevel::SoftwareSimulated => HardwareAssuranceLevel::Software,
        TpmAssuranceLevel::DiscreteTpm | TpmAssuranceLevel::IntegratedTpm => {
            HardwareAssuranceLevel::IsolatedHardware
        }
        TpmAssuranceLevel::Certified(v) => HardwareAssuranceLevel::Certified(v.clone()),
        TpmAssuranceLevel::Unknown => HardwareAssuranceLevel::Unknown,
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

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validate_hardware_key_info;
    use crate::{HardwareAssuranceStrength, MESH_SIGNATURE_LENGTH};
    use alloc::vec;
    use edgerun_tpm::{TpmAssuranceLevel, TpmError, TpmKeyInfo, TpmSignatureAlgorithm};

    // -----------------------------------------------------------------------
    // Fake TPM keys for testing
    // -----------------------------------------------------------------------

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
            Self { pkcs1v15_key: true }
        }
    }

    /// Builds a fake TPM ECDSA signature buffer:
    ///   scheme(2) + hashAlg(2) + r_size(2) + r(32) + s_size(2) + s(32)
    fn fake_tpm_ecdsa_signature(message: &[u8]) -> Vec<u8> {
        let mut sig = Vec::with_capacity(72);
        sig.extend_from_slice(&0x0018u16.to_be_bytes()); // TPM_ALG_ECDSA
        sig.extend_from_slice(&0x000Bu16.to_be_bytes()); // TPM_ALG_SHA256
        let mut r = [0u8; 32];
        let len = message.len().min(32);
        r[..len].copy_from_slice(&message[..len]);
        sig.extend_from_slice(&(r.len() as u16).to_be_bytes());
        sig.extend_from_slice(&r);
        let mut s = [0u8; 32];
        for (i, b) in s.iter_mut().enumerate() {
            *b = message.get(i).copied().unwrap_or(0).wrapping_add(0x55);
        }
        sig.extend_from_slice(&(s.len() as u16).to_be_bytes());
        sig.extend_from_slice(&s);
        sig
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

    /// Fake TPM key that returns properly formatted ECDSA signatures for adapter tests
    struct TpmAdapterFakeKey;

    impl TpmSigningKey for TpmAdapterFakeKey {
        fn key_info(&self) -> Result<TpmKeyInfo, TpmError> {
            Ok(TpmKeyInfo {
                key_name: "tpm-adapter-fake".into(),
                algorithm: TpmSignatureAlgorithm::EcdsaP256Sha256,
                public_key: vec![1, 2, 3],
                attestation_blob: None,
                assurance_level: TpmAssuranceLevel::DiscreteTpm,
            })
        }
        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, TpmError> {
            Ok(fake_tpm_ecdsa_signature(message))
        }
    }

    // -----------------------------------------------------------------------
    // Tests
    // -----------------------------------------------------------------------

    #[test]
    fn tpm_adapter_maps_key_info() {
        let key = FakeTpmKey::new();
        let adapter = TpmHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.provider, HardwareProviderKind::Tpm);
        assert_eq!(info.key_name, "tpm-key");
        assert_eq!(info.algorithm, HardwareSignatureAlgorithm::EcdsaP256Sha256);
        assert_eq!(info.public_key, vec![1, 2, 3]);
        assert_eq!(info.attestation, vec![vec![9, 9]]);
        assert_eq!(
            info.assurance_level,
            HardwareAssuranceLevel::IsolatedHardware
        );
    }

    #[test]
    fn tpm_adapter_sign_strips_ecdsa_wrapper() {
        let key = TpmAdapterFakeKey;
        let adapter = TpmHardwareKeyAdapter::new(key);
        let sig = adapter.sign_message(b"hello").unwrap();
        // After stripping: 32 bytes r + 32 bytes s = 64 bytes
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn tpm_adapter_passes_through_non_ecdsa() {
        let key = FakeTpmKey::with_pkcs1v15();
        let adapter = TpmHardwareKeyAdapter::new(key);
        let sig = adapter.sign_message(b"hello").unwrap();
        // Non-ECDSA: raw message passes through
        assert_eq!(sig, b"hello");
    }

    #[test]
    fn map_tpm_algorithm_variants() {
        assert_eq!(
            map_tpm_algorithm(&TpmSignatureAlgorithm::RsaPkcs1v15Sha256),
            HardwareSignatureAlgorithm::RsaPkcs1v15Sha256
        );
        assert_eq!(
            map_tpm_algorithm(&TpmSignatureAlgorithm::EcdsaP384Sha384),
            HardwareSignatureAlgorithm::EcdsaP384Sha384
        );
        assert_eq!(
            map_tpm_algorithm(&TpmSignatureAlgorithm::Opaque("x".into())),
            HardwareSignatureAlgorithm::Opaque("x".into())
        );
    }

    #[test]
    fn map_tpm_assurance_variants() {
        assert_eq!(
            map_tpm_assurance(&TpmAssuranceLevel::SoftwareSimulated),
            HardwareAssuranceLevel::Software
        );
        assert_eq!(
            map_tpm_assurance(&TpmAssuranceLevel::DiscreteTpm),
            HardwareAssuranceLevel::IsolatedHardware
        );
        assert_eq!(
            map_tpm_assurance(&TpmAssuranceLevel::Certified("cert".into())),
            HardwareAssuranceLevel::Certified("cert".into())
        );
    }

    #[test]
    fn sign_record_with_tpm_provider_success() {
        let key = FakeTpmKey::new();
        let req = HardwareValidationRequirements::default();
        let sig = sign_record_with_tpm_provider(&key, &req, "test:v0:sig", b"hash").unwrap();
        // FakeTpmKey returns TPM-format signature (not raw hash)
        assert!(!sig.is_empty());
        assert!(sig.len() >= 4); // At least header bytes
    }

    #[test]
    fn tpm_provider_with_custom_pk() {
        let key = FakeTpmKeyWithCustomPk {
            public_key: vec![0xABu8; 64],
            assurance: TpmAssuranceLevel::IntegratedTpm,
        };
        let adapter = TpmHardwareKeyAdapter::new(key);
        let info = adapter.key_info().unwrap();
        assert_eq!(info.public_key, vec![0xABu8; 64]);
        assert_eq!(
            info.assurance_level,
            HardwareAssuranceLevel::IsolatedHardware
        );
        assert!(info.is_mesh_capable());
    }
}
