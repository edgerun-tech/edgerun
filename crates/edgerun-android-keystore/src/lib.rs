use edgerun_core::crypto::signature_input;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AndroidKeystoreSignatureAlgorithm {
    RsaPkcs1v15Sha256,
    RsaPssSha256,
    EcdsaP256Sha256,
    EcdsaP384Sha384,
    Eddsa,
    Opaque(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AndroidKeystoreAssuranceLevel {
    Software,
    Tee,
    StrongBox,
    Attested(String),
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AndroidKeystoreKeyInfo {
    pub alias: String,
    pub algorithm: AndroidKeystoreSignatureAlgorithm,
    pub public_key: Vec<u8>,
    pub attestation_chain: Vec<Vec<u8>>,
    pub assurance_level: AndroidKeystoreAssuranceLevel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AndroidKeystoreError {
    Provider(String),
    UnsupportedAlgorithm(AndroidKeystoreSignatureAlgorithm),
}

impl core::fmt::Display for AndroidKeystoreError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Provider(msg) => f.write_str(msg),
            Self::UnsupportedAlgorithm(algorithm) => {
                write!(
                    f,
                    "unsupported Android Keystore signature algorithm: {algorithm:?}"
                )
            }
        }
    }
}

impl std::error::Error for AndroidKeystoreError {}

pub trait AndroidKeystoreSigningKey {
    fn key_info(&self) -> Result<AndroidKeystoreKeyInfo, AndroidKeystoreError>;

    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, AndroidKeystoreError>;
}

pub fn signature_input_for_record(sig_domain_tag: &str, record_hash: &[u8]) -> Vec<u8> {
    signature_input(sig_domain_tag, record_hash)
}

pub fn sign_record_with_keystore(
    key: &dyn AndroidKeystoreSigningKey,
    sig_domain_tag: &str,
    record_hash: &[u8],
) -> Result<Vec<u8>, AndroidKeystoreError> {
    key.sign_message(&signature_input_for_record(sig_domain_tag, record_hash))
}

pub fn sign_record_with_keystore_checked(
    key: &dyn AndroidKeystoreSigningKey,
    expected_algorithms: &[AndroidKeystoreSignatureAlgorithm],
    sig_domain_tag: &str,
    record_hash: &[u8],
) -> Result<Vec<u8>, AndroidKeystoreError> {
    let key_info = key.key_info()?;
    if !expected_algorithms
        .iter()
        .any(|algorithm| algorithm == &key_info.algorithm)
    {
        return Err(AndroidKeystoreError::UnsupportedAlgorithm(
            key_info.algorithm,
        ));
    }
    key.sign_message(&signature_input_for_record(sig_domain_tag, record_hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FakeKeystoreKey {
        algorithm: AndroidKeystoreSignatureAlgorithm,
    }

    impl FakeKeystoreKey {
        fn new(algorithm: AndroidKeystoreSignatureAlgorithm) -> Self {
            Self { algorithm }
        }
    }

    impl AndroidKeystoreSigningKey for FakeKeystoreKey {
        fn key_info(&self) -> Result<AndroidKeystoreKeyInfo, AndroidKeystoreError> {
            Ok(AndroidKeystoreKeyInfo {
                alias: "edgerun-test-key".into(),
                algorithm: self.algorithm.clone(),
                public_key: vec![4, 3, 2, 1],
                attestation_chain: vec![vec![7, 7, 7]],
                assurance_level: AndroidKeystoreAssuranceLevel::StrongBox,
            })
        }

        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, AndroidKeystoreError> {
            let mut sig = self.key_info()?.public_key;
            sig.extend_from_slice(message);
            Ok(sig)
        }
    }

    #[test]
    fn signs_protocol_record_input_via_generic_keystore_trait() {
        let key = FakeKeystoreKey::new(AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256);
        let sig = sign_record_with_keystore(&key, "edgerun:v0:sig:test", &[5u8; 32]).unwrap();
        let expected_input = signature_input_for_record("edgerun:v0:sig:test", &[5u8; 32]);
        assert!(sig.ends_with(&expected_input));
    }

    #[test]
    fn checked_sign_rejects_unexpected_algorithm() {
        let key = FakeKeystoreKey::new(AndroidKeystoreSignatureAlgorithm::RsaPssSha256);
        let err = sign_record_with_keystore_checked(
            &key,
            &[
                AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256,
                AndroidKeystoreSignatureAlgorithm::Eddsa,
            ],
            "edgerun:v0:sig:test",
            &[5u8; 32],
        )
        .unwrap_err();
        assert_eq!(
            err,
            AndroidKeystoreError::UnsupportedAlgorithm(
                AndroidKeystoreSignatureAlgorithm::RsaPssSha256
            )
        );
    }

    #[test]
    fn checked_sign_accepts_expected_algorithm() {
        let key = FakeKeystoreKey::new(AndroidKeystoreSignatureAlgorithm::Eddsa);
        let sig = sign_record_with_keystore_checked(
            &key,
            &[
                AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256,
                AndroidKeystoreSignatureAlgorithm::Eddsa,
            ],
            "edgerun:v0:sig:test",
            &[7u8; 32],
        )
        .unwrap();
        assert!(!sig.is_empty());
    }

    // --- Enum variant coverage ---

    #[test]
    fn signature_algorithm_variants_are_distinct() {
        let algorithms = vec![
            AndroidKeystoreSignatureAlgorithm::RsaPkcs1v15Sha256,
            AndroidKeystoreSignatureAlgorithm::RsaPssSha256,
            AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256,
            AndroidKeystoreSignatureAlgorithm::EcdsaP384Sha384,
            AndroidKeystoreSignatureAlgorithm::Eddsa,
            AndroidKeystoreSignatureAlgorithm::Opaque("custom".into()),
        ];
        // Verify all variants are distinct via PartialEq
        for (i, a) in algorithms.iter().enumerate() {
            for (j, b) in algorithms.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn assurance_level_variants_are_distinct() {
        let levels = vec![
            AndroidKeystoreAssuranceLevel::Software,
            AndroidKeystoreAssuranceLevel::Tee,
            AndroidKeystoreAssuranceLevel::StrongBox,
            AndroidKeystoreAssuranceLevel::Attested("cert".into()),
            AndroidKeystoreAssuranceLevel::Unknown,
        ];
        for (i, a) in levels.iter().enumerate() {
            for (j, b) in levels.iter().enumerate() {
                if i == j {
                    assert_eq!(a, b);
                } else {
                    assert_ne!(a, b);
                }
            }
        }
    }

    #[test]
    fn opaque_algorithm_equality_works() {
        let a = AndroidKeystoreSignatureAlgorithm::Opaque("foo".into());
        let b = AndroidKeystoreSignatureAlgorithm::Opaque("foo".into());
        let c = AndroidKeystoreSignatureAlgorithm::Opaque("bar".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn attested_assurance_level_equality() {
        let a = AndroidKeystoreAssuranceLevel::Attested("cert1".into());
        let b = AndroidKeystoreAssuranceLevel::Attested("cert1".into());
        let c = AndroidKeystoreAssuranceLevel::Attested("cert2".into());
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // --- Clone, Debug, PartialEq, Eq on structs ---

    #[test]
    fn key_info_clone_and_equality() {
        let info = AndroidKeystoreKeyInfo {
            alias: "test-key".into(),
            algorithm: AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256,
            public_key: vec![1, 2, 3],
            attestation_chain: vec![vec![4, 5], vec![6, 7]],
            assurance_level: AndroidKeystoreAssuranceLevel::StrongBox,
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    #[test]
    fn key_info_debug_output_contains_alias() {
        let info = AndroidKeystoreKeyInfo {
            alias: "debug-key".into(),
            algorithm: AndroidKeystoreSignatureAlgorithm::Eddsa,
            public_key: vec![],
            attestation_chain: vec![],
            assurance_level: AndroidKeystoreAssuranceLevel::Software,
        };
        let debug_str = format!("{:?}", info);
        assert!(debug_str.contains("debug-key"));
    }

    // --- Error type coverage ---

    #[test]
    fn error_display_provider() {
        let err = AndroidKeystoreError::Provider("something broke".into());
        assert_eq!(format!("{}", err), "something broke");
    }

    #[test]
    fn error_display_unsupported_algorithm() {
        let err = AndroidKeystoreError::UnsupportedAlgorithm(
            AndroidKeystoreSignatureAlgorithm::RsaPkcs1v15Sha256,
        );
        let msg = format!("{}", err);
        assert!(msg.contains("unsupported"));
        assert!(msg.contains("RsaPkcs1v15Sha256"));
    }

    #[test]
    fn error_debug_output() {
        let err = AndroidKeystoreError::Provider("oops".into());
        let debug = format!("{:?}", err);
        assert!(debug.contains("Provider"));
    }

    #[test]
    fn error_is_std_error() {
        let err: Box<dyn std::error::Error> =
            Box::new(AndroidKeystoreError::Provider("err".into()));
        assert!(err.to_string().contains("err"));
    }

    #[test]
    fn error_from_source_chain() {
        let err = AndroidKeystoreError::Provider("chain".into());
        // Provider errors have no source
        assert!(std::error::Error::source(&err).is_none());
    }

    // --- Edge cases in signing ---

    #[test]
    fn sign_record_with_empty_hash() {
        let key = FakeKeystoreKey::new(AndroidKeystoreSignatureAlgorithm::Eddsa);
        let sig = sign_record_with_keystore(&key, "domain", &[]).unwrap();
        assert!(!sig.is_empty());
    }

    #[test]
    fn sign_record_with_empty_domain_tag() {
        let key = FakeKeystoreKey::new(AndroidKeystoreSignatureAlgorithm::Eddsa);
        let sig = sign_record_with_keystore(&key, "", &[1u8; 32]).unwrap();
        assert!(!sig.is_empty());
    }

    #[test]
    fn sign_record_with_large_hash() {
        let key = FakeKeystoreKey::new(AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256);
        let large_hash = vec![0xAB; 1024];
        let sig = sign_record_with_keystore(&key, "domain", &large_hash).unwrap();
        assert!(!sig.is_empty());
    }

    #[test]
    fn checked_sign_with_empty_allowed_list() {
        let key = FakeKeystoreKey::new(AndroidKeystoreSignatureAlgorithm::Eddsa);
        let err = sign_record_with_keystore_checked(&key, &[], "domain", &[1u8; 32]).unwrap_err();
        assert!(matches!(
            err,
            AndroidKeystoreError::UnsupportedAlgorithm(_)
        ));
    }

    #[test]
    fn checked_sign_with_single_matching_algorithm() {
        let key = FakeKeystoreKey::new(AndroidKeystoreSignatureAlgorithm::RsaPssSha256);
        let sig = sign_record_with_keystore_checked(
            &key,
            &[AndroidKeystoreSignatureAlgorithm::RsaPssSha256],
            "domain",
            &[1u8; 32],
        )
        .unwrap();
        assert!(!sig.is_empty());
    }

    // --- FakeKeystoreKey behavior ---

    #[test]
    fn fake_key_info_has_expected_fields() {
        let key = FakeKeystoreKey::new(AndroidKeystoreSignatureAlgorithm::EcdsaP384Sha384);
        let info = key.key_info().unwrap();
        assert_eq!(info.alias, "edgerun-test-key");
        assert_eq!(
            info.algorithm,
            AndroidKeystoreSignatureAlgorithm::EcdsaP384Sha384
        );
        assert_eq!(info.public_key, vec![4, 3, 2, 1]);
        assert_eq!(info.attestation_chain, vec![vec![7, 7, 7]]);
        assert_eq!(
            info.assurance_level,
            AndroidKeystoreAssuranceLevel::StrongBox
        );
    }

    #[test]
    fn fake_sign_message_prepends_public_key() {
        let key = FakeKeystoreKey::new(AndroidKeystoreSignatureAlgorithm::Eddsa);
        let sig = key.sign_message(b"hello").unwrap();
        assert_eq!(&sig[..4], &[4, 3, 2, 1]);
        assert_eq!(&sig[4..], b"hello");
    }

    // --- signature_input_for_record delegation ---

    #[test]
    fn signature_input_for_record_produces_nonempty_output() {
        let input = signature_input_for_record("test:domain", &[0xFF; 32]);
        assert!(!input.is_empty());
    }

    #[test]
    fn signature_input_for_record_deterministic() {
        let a = signature_input_for_record("d", &[1, 2, 3]);
        let b = signature_input_for_record("d", &[1, 2, 3]);
        assert_eq!(a, b);
    }

    #[test]
    fn signature_input_for_record_different_domain_different_output() {
        let a = signature_input_for_record("domain-a", &[1u8; 32]);
        let b = signature_input_for_record("domain-b", &[1u8; 32]);
        assert_ne!(a, b);
    }

    #[test]
    fn signature_input_for_record_different_hash_different_output() {
        let a = signature_input_for_record("domain", &[1u8; 32]);
        let b = signature_input_for_record("domain", &[2u8; 32]);
        assert_ne!(a, b);
    }
}
