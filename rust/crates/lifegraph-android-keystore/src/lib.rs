use lifegraph_core::crypto::signature_input;

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
                alias: "lifegraph-test-key".into(),
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
        let sig = sign_record_with_keystore(&key, "lifegraph:v0:sig:test", &[5u8; 32]).unwrap();
        let expected_input = signature_input_for_record("lifegraph:v0:sig:test", &[5u8; 32]);
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
            "lifegraph:v0:sig:test",
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
            "lifegraph:v0:sig:test",
            &[7u8; 32],
        )
        .unwrap();
        assert!(!sig.is_empty());
    }
}
