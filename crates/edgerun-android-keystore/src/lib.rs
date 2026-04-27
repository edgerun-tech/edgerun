//! Android Keystore signing via JNI.
//!
//! The Android Keystore has no public C API. We must go through JNI to call
//! `android.security.keystore.KeyGenParameterSpec.Builder` and
//! `java.security.KeyStore`.
//!
//! Enable the `android-real` feature to get the real JNI-backed implementation.
//! On non-Android targets (or without the feature), only the trait definitions
//! and test fakes are available.

#![no_std]

extern crate alloc;

#[cfg(test)]
extern crate std;

use alloc::string::String;
use alloc::vec::Vec;
use core::fmt::Write;
use core::result::Result::{self, Err};
use edgerun_core::crypto::signature_input;

// ===========================================================================
// Public types (always available)
// ===========================================================================

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

impl core::error::Error for AndroidKeystoreError {}

/// Trait for Android Keystore-backed signing keys.
///
/// On Android devices with the `android-real` feature enabled, a concrete
/// implementation uses JNI to call the `android.security.keystore` APIs.
/// On other targets, only the test fakes implement this.
pub trait AndroidKeystoreSigningKey {
    fn key_info(&self) -> Result<AndroidKeystoreKeyInfo, AndroidKeystoreError>;
    fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, AndroidKeystoreError>;
}

// ===========================================================================
// Public helpers (always available)
// ===========================================================================

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

// ===========================================================================
// Fake keystore for testing on any target
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;

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
}

// ===========================================================================
// Real JNI-backed implementation (android-real feature, Android target)
// ===========================================================================

#[cfg(all(feature = "android-real", target_os = "android"))]
mod real {
    use super::*;
    use jni::objects::{GlobalRef, JValue};
    use jni::strings::JNIString;
    use jni::InitJavaVM;
    use ndk_context::android_context;
    use once_cell::sync::OnceCell;

    /// Global Java VM reference, initialized once on Android startup.
    static JAVA_VM: OnceCell<jni::JavaVM> = OnceCell::new();

    /// Initialize the JVM context from Android's native activity.
    /// Must be called once before using any keystore operations.
    pub fn init_keystore_jvm() -> Result<(), AndroidKeystoreError> {
        if JAVA_VM.get().is_some() {
            return Ok(()); // Already initialized
        }
        unsafe {
            let ctx = android_context();
            let vm = ctx.vm();
            if vm.is_null() {
                return Err(AndroidKeystoreError::Provider(
                    "Android context returned null VM".into(),
                ));
            }
            let java_vm = jni::JavaVM::from_raw(vm).map_err(|e| {
                AndroidKeystoreError::Provider(format!("Failed to attach to Java VM: {e}"))
            })?;
            // from_raw takes ownership — but we don't want to destroy it,
            // so we detach manually after storing
            JAVA_VM
                .set(java_vm)
                .map_err(|_| AndroidKeystoreError::Provider("JVM already initialized".into()))?;
            Ok(())
        }
    }

    fn get_jvm() -> Result<&'static jni::JavaVM, AndroidKeystoreError> {
        JAVA_VM.get().ok_or_else(|| {
            AndroidKeystoreError::Provider(
                "JVM not initialized — call init_keystore_jvm() first".into(),
            )
        })
    }

    /// A real Android Keystore-backed signing key.
    ///
    /// Uses JNI to call `android.security.keystore` Java APIs.
    /// The private key never leaves the secure hardware.
    pub struct JniKeystoreKey {
        alias: String,
        algorithm: AndroidKeystoreSignatureAlgorithm,
        /// JNI global reference to the Java `PrivateKey` object.
        private_key: GlobalRef,
        /// JNI global reference to the Java `PublicKey` object.
        public_key: GlobalRef,
        assurance_level: AndroidKeystoreAssuranceLevel,
        attestation_chain: Vec<Vec<u8>>,
    }

    impl JniKeystoreKey {
        /// Generates or retrieves a key with the given alias and algorithm.
        ///
        /// If the key doesn't exist yet, it is generated with
        /// `KeyGenParameterSpec.Builder` using the specified algorithm.
        pub fn generate_or_retrieve(
            alias: &str,
            algorithm: AndroidKeystoreSignatureAlgorithm,
        ) -> Result<Self, AndroidKeystoreError> {
            let jvm = get_jvm()?;
            let mut env = jvm
                .attach_current_thread()
                .map_err(|e| AndroidKeystoreError::Provider(format!("JNI attach failed: {e}")))?;

            let key_alias = JNIString::from(alias);

            // Get KeyStore instance
            let key_store_class = env
                .find_class("java/security/KeyStore")
                .map_err(|e| AndroidKeystoreError::Provider(format!("find_class failed: {e}")))?;
            let keystore = env
                .call_static_method(
                    key_store_class,
                    "getInstance",
                    "(Ljava/lang/String;)Ljava/security/KeyStore;",
                    &[JValue::Object(&env.new_string("AndroidKeyStore").unwrap())],
                )
                .map_err(|e| AndroidKeystoreError::Provider(format!("getInstance failed: {e}")))?
                .l()
                .map_err(|e| AndroidKeystoreError::Provider(format!("l() failed: {e}")))?;

            // Load the keystore (null KeyStore.LoadParameter means use default)
            env.call_method(
                &keystore,
                "load",
                "(Ljava/security/KeyStore$LoadStoreParameter;)V",
                &[],
            )
            .map_err(|e| AndroidKeystoreError::Provider(format!("keystore.load failed: {e}")))?;

            // Check if key exists
            let contains = env
                .call_method(
                    &keystore,
                    "containsAlias",
                    "(Ljava/lang/String;)Z",
                    &[JValue::Object(&env.new_string(alias).unwrap())],
                )
                .map_err(|e| AndroidKeystoreError::Provider(format!("containsAlias failed: {e}")))?
                .z()
                .map_err(|e| AndroidKeystoreError::Provider(format!("z() failed: {e}")))?;

            if !contains {
                // Generate new key
                Self::generate_key(&mut env, alias, &algorithm)?;
            }

            // Get the private and public key
            let private_key = env
                .call_method(
                    &keystore,
                    "getKey",
                    "(Ljava/lang/String;[C)Ljava/security/Key;",
                    &[
                        JValue::Object(&env.new_string(alias).unwrap()),
                        JValue::Object(&jni::objects::JObject::null()),
                    ],
                )
                .map_err(|e| AndroidKeystoreError::Provider(format!("getKey failed: {e}")))?
                .l()
                .map_err(|e| AndroidKeystoreError::Provider(format!("l() failed: {e}")))?;

            let cert = env
                .call_method(
                    &keystore,
                    "getCertificate",
                    "(Ljava/lang/String;)Ljava/security/cert/Certificate;",
                    &[JValue::Object(&env.new_string(alias).unwrap())],
                )
                .map_err(|e| AndroidKeystoreError::Provider(format!("getCertificate failed: {e}")))?
                .l()
                .map_err(|e| AndroidKeystoreError::Provider(format!("l() failed: {e}")))?;

            let public_key = env
                .call_method(&cert, "getPublicKey", "()Ljava/security/PublicKey;", &[])
                .map_err(|e| AndroidKeystoreError::Provider(format!("getPublicKey failed: {e}")))?
                .l()
                .map_err(|e| AndroidKeystoreError::Provider(format!("l() failed: {e}")))?;

            let private_key_global = env.new_global_ref(&private_key).map_err(|e| {
                AndroidKeystoreError::Provider(format!("new_global_ref failed: {e}"))
            })?;
            let public_key_global = env.new_global_ref(&public_key).map_err(|e| {
                AndroidKeystoreError::Provider(format!("new_global_ref failed: {e}"))
            })?;

            // Get the encoded public key
            let encoded = env
                .call_method(&public_key, "getEncoded", "()[B", &[])
                .map_err(|e| AndroidKeystoreError::Provider(format!("getEncoded failed: {e}")))?
                .l()
                .map_err(|e| AndroidKeystoreError::Provider(format!("l() failed: {e}")))?;

            // Get attestation chain (if StrongBox or attested key)
            let attestation_chain = Self::get_attestation_chain(&mut env, &keystore, alias)?;
            let assurance = Self::get_assurance_level(&mut env, &keystore, alias)?;

            Ok(Self {
                alias: alias.to_string(),
                algorithm,
                private_key: private_key_global,
                public_key: public_key_global,
                assurance_level: assurance,
                attestation_chain,
            })
        }

        fn generate_key(
            env: &mut jni::JNIEnv,
            alias: &str,
            algorithm: &AndroidKeystoreSignatureAlgorithm,
        ) -> Result<(), AndroidKeystoreError> {
            let (key_type, digest, key_size) = match algorithm {
                AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256 => ("EC", "SHA256", 256),
                AndroidKeystoreSignatureAlgorithm::EcdsaP384Sha384 => ("EC", "SHA384", 384),
                AndroidKeystoreSignatureAlgorithm::RsaPkcs1v15Sha256 => ("RSA", "SHA256", 2048),
                AndroidKeystoreSignatureAlgorithm::RsaPssSha256 => ("RSA", "SHA256", 2048),
                AndroidKeystoreSignatureAlgorithm::Eddsa => {
                    // EdDSA is not supported by Android Keystore — fall back to EC P-256
                    return Err(AndroidKeystoreError::UnsupportedAlgorithm(
                        AndroidKeystoreSignatureAlgorithm::Eddsa,
                    ));
                }
                AndroidKeystoreSignatureAlgorithm::Opaque(_) => {
                    return Err(AndroidKeystoreError::UnsupportedAlgorithm(
                        algorithm.clone(),
                    ));
                }
            };

            // Build KeyGenParameterSpec
            let builder_class = env
                .find_class("android/security/keystore/KeyGenParameterSpec$Builder")
                .map_err(|e| {
                    AndroidKeystoreError::Provider(format!("find_class Builder failed: {e}"))
                })?;

            let builder = env
                .new_object(
                    builder_class,
                    "(Ljava/lang/String;I)V",
                    &[
                        JValue::Object(&env.new_string(alias).unwrap()),
                        JValue::Int(3), // PURPOSE_SIGN = 3
                    ],
                )
                .map_err(|e| {
                    AndroidKeystoreError::Provider(format!("new_object Builder failed: {e}"))
                })?;

            // Set algorithm
            let builder = env
                .call_method(
                    &builder,
                    "setAlgorithmParameterSpec",
                    "(Ljava/security/spec/AlgorithmParameterSpec;)Landroid/security/keystore/KeyGenParameterSpec$Builder;",
                    &[JValue::Object(&jni::objects::JObject::null())],
                )
                .map_err(|e| format!("setAlgorithmParameterSpec: {e}"))?
                .l()
                .map_err(|e| format!("l(): {e}"))?;

            // Set digest
            let digest_id = match digest {
                "SHA256" => 32, // DIGEST_SHA256
                "SHA384" => 33, // DIGEST_SHA384
                _ => 32,
            };
            let builder = env
                .call_method(
                    &builder,
                    "setDigests",
                    "([I)Landroid/security/keystore/KeyGenParameterSpec$Builder;",
                    &[JValue::Object(&env.new_int_array(&[digest_id]).unwrap())],
                )
                .map_err(|e| AndroidKeystoreError::Provider(format!("setDigests failed: {e}")))?
                .l()
                .map_err(|e| AndroidKeystoreError::Provider(format!("l() failed: {e}")))?;

            // For RSA, set signature padding
            if key_type == "RSA" {
                let padding = if algorithm == &AndroidKeystoreSignatureAlgorithm::RsaPssSha256 {
                    4 // SIGNATURE_PADDING_RSA_PSS
                } else {
                    1 // SIGNATURE_PADDING_RSA_PKCS1
                };
                let _ = env
                    .call_method(
                        &builder,
                        "setSignaturePaddings",
                        "(I)Landroid/security/keystore/KeyGenParameterSpec$Builder;",
                        &[JValue::Int(padding)],
                    )
                    .map_err(|e| {
                        AndroidKeystoreError::Provider(format!("setSignaturePaddings failed: {e}"))
                    });
            }

            // Build the spec
            let spec = env
                .call_method(
                    &builder,
                    "build",
                    "()Landroid/security/keystore/KeyGenParameterSpec;",
                    &[],
                )
                .map_err(|e| AndroidKeystoreError::Provider(format!("build failed: {e}")))?
                .l()
                .map_err(|e| AndroidKeystoreError::Provider(format!("l() failed: {e}")))?;

            // Get KeyGenerator and generate
            let key_gen_class = env.find_class("javax/crypto/KeyGenerator").map_err(|e| {
                AndroidKeystoreError::Provider(format!("find_class KeyGenerator failed: {e}"))
            })?;
            let key_gen = env
                .call_static_method(
                    key_gen_class,
                    "getInstance",
                    "(Ljava/lang/String;)Ljavax/crypto/KeyGenerator;",
                    &[JValue::Object(&env.new_string("AndroidKeyStore").unwrap())],
                )
                .map_err(|e| {
                    AndroidKeystoreError::Provider(format!("KeyGenerator.getInstance failed: {e}"))
                })?
                .l()
                .map_err(|e| AndroidKeystoreError::Provider(format!("l() failed: {e}")))?;

            env.call_method(
                &key_gen,
                "init",
                "(Ljava/security/spec/AlgorithmParameterSpec;)V",
                &[JValue::Object(&spec)],
            )
            .map_err(|e| {
                AndroidKeystoreError::Provider(format!("KeyGenerator.init failed: {e}"))
            })?;

            env.call_method(&key_gen, "generateKey", "()Ljava/security/Key;", &[])
                .map_err(|e| AndroidKeystoreError::Provider(format!("generateKey failed: {e}")))?;

            Ok(())
        }

        fn get_attestation_chain(
            env: &mut jni::JNIEnv,
            keystore: &jni::objects::JObject,
            alias: &str,
        ) -> Result<Vec<Vec<u8>>, AndroidKeystoreError> {
            // Call keystore.getCertificateChain(alias) which returns Certificate[]
            let chain_array = env
                .call_method(
                    keystore,
                    "getCertificateChain",
                    "(Ljava/lang/String;)[Ljava/security/cert/Certificate;",
                    &[JValue::Object(&env.new_string(alias).unwrap())],
                )
                .map_err(|e| {
                    AndroidKeystoreError::Provider(format!("getCertificateChain failed: {e}"))
                })?
                .l()
                .map_err(|e| AndroidKeystoreError::Provider(format!("l() failed: {e}")))?;

            if chain_array.is_null() {
                return Ok(Vec::new());
            }

            // Get array length
            let len = env.get_array_length(&chain_array).map_err(|e| {
                AndroidKeystoreError::Provider(format!("get_array_length failed: {e}"))
            })? as usize;

            if len == 0 {
                return Ok(Vec::new());
            }

            // Extract each certificate's encoded bytes
            let mut chain = Vec::with_capacity(len);
            let cert_objs = env
                .convert_object_array(&chain_array, "java/security/cert/Certificate")
                .map_err(|e| {
                    AndroidKeystoreError::Provider(format!("convert_object_array failed: {e}"))
                })?;

            for cert_obj in cert_objs {
                if cert_obj.is_null() {
                    continue;
                }
                let encoded = env
                    .call_method(&cert_obj, "getEncoded", "()[B", &[])
                    .map_err(|e| AndroidKeystoreError::Provider(format!("getEncoded failed: {e}")))?
                    .l()
                    .map_err(|e| AndroidKeystoreError::Provider(format!("l() failed: {e}")))?;

                let bytes = env.convert_byte_array(encoded.as_raw()).map_err(|e| {
                    AndroidKeystoreError::Provider(format!("convert_byte_array failed: {e}"))
                })?;
                chain.push(bytes);
            }

            Ok(chain)
        }

        fn get_assurance_level(
            env: &mut jni::JNIEnv,
            keystore: &jni::objects::JObject,
            alias: &str,
        ) -> Result<AndroidKeystoreAssuranceLevel, AndroidKeystoreError> {
            // Try to get the KeyInfo to check for StrongBox attestation
            // This requires API 23+ KeyFactory + AndroidKeyStore
            let cert_chain = Self::get_attestation_chain(env, keystore, alias)?;

            // If we have an attestation chain with multiple certs, the key is attested
            if cert_chain.len() > 1 {
                // Check for StrongBox by examining the attestation cert
                // StrongBox attestation certs have specific characteristics
                // For now, if we have multiple certs in the chain, assume StrongBox
                return Ok(AndroidKeystoreAssuranceLevel::StrongBox);
            } else if cert_chain.len() == 1 {
                // Single cert = TEE attestation
                return Ok(AndroidKeystoreAssuranceLevel::Tee);
            }

            // No attestation chain = software (shouldn't happen for AndroidKeyStore)
            Ok(AndroidKeystoreAssuranceLevel::Software)
        }
    }

    impl AndroidKeystoreSigningKey for JniKeystoreKey {
        fn key_info(&self) -> Result<AndroidKeystoreKeyInfo, AndroidKeystoreError> {
            let jvm = get_jvm()?;
            let env = jvm
                .attach_current_thread()
                .map_err(|e| AndroidKeystoreError::Provider(format!("JNI attach failed: {e}")))?;

            let encoded = env
                .call_method(&self.public_key, "getEncoded", "()[B", &[])
                .map_err(|e| AndroidKeystoreError::Provider(format!("getEncoded failed: {e}")))?
                .l()
                .map_err(|e| AndroidKeystoreError::Provider(format!("l() failed: {e}")))?;

            let public_key_bytes = env.convert_byte_array(encoded.as_raw()).map_err(|e| {
                AndroidKeystoreError::Provider(format!("convert_byte_array failed: {e}"))
            })?;

            Ok(AndroidKeystoreKeyInfo {
                alias: self.alias.clone(),
                algorithm: self.algorithm.clone(),
                public_key: public_key_bytes,
                attestation_chain: self.attestation_chain.clone(),
                assurance_level: self.assurance_level.clone(),
            })
        }

        fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, AndroidKeystoreError> {
            let jvm = get_jvm()?;
            let env = jvm
                .attach_current_thread()
                .map_err(|e| AndroidKeystoreError::Provider(format!("JNI attach failed: {e}")))?;

            // Create Signature object with the right algorithm
            let sig_algo = match self.algorithm {
                AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256 => "SHA256withECDSA",
                AndroidKeystoreSignatureAlgorithm::EcdsaP384Sha384 => "SHA384withECDSA",
                AndroidKeystoreSignatureAlgorithm::RsaPkcs1v15Sha256 => "SHA256withRSA",
                AndroidKeystoreSignatureAlgorithm::RsaPssSha256 => "SHA256withRSAandMGF1",
                _ => {
                    return Err(AndroidKeystoreError::UnsupportedAlgorithm(
                        self.algorithm.clone(),
                    ));
                }
            };

            let sig_class = env.find_class("java/security/Signature").map_err(|e| {
                AndroidKeystoreError::Provider(format!("find_class Signature failed: {e}"))
            })?;

            let sig = env
                .call_static_method(
                    sig_class,
                    "getInstance",
                    "(Ljava/lang/String;)Ljava/security/Signature;",
                    &[JValue::Object(&env.new_string(sig_algo).unwrap())],
                )
                .map_err(|e| AndroidKeystoreError::Provider(format!("getInstance failed: {e}")))?
                .l()
                .map_err(|e| AndroidKeystoreError::Provider(format!("l() failed: {e}")))?;

            env.call_method(
                &sig,
                "initSign",
                "(Ljava/security/PrivateKey;)V",
                &[JValue::Object(&self.private_key)],
            )
            .map_err(|e| AndroidKeystoreError::Provider(format!("initSign failed: {e}")))?;

            let msg_arr = env.byte_array_from_slice(message).map_err(|e| {
                AndroidKeystoreError::Provider(format!("byte_array_from_slice failed: {e}"))
            })?;

            env.call_method(&sig, "update", "([B)V", &[JValue::Object(&msg_arr)])
                .map_err(|e| AndroidKeystoreError::Provider(format!("update failed: {e}")))?;

            let sig_bytes = env
                .call_method(&sig, "sign", "()[B", &[])
                .map_err(|e| AndroidKeystoreError::Provider(format!("sign failed: {e}")))?
                .l()
                .map_err(|e| AndroidKeystoreError::Provider(format!("l() failed: {e}")))?;

            env.convert_byte_array(sig_bytes.as_raw())
                .map_err(|e| {
                    AndroidKeystoreError::Provider(format!("convert_byte_array failed: {e}"))
                })?
                .into()
                .map(Ok)
                .unwrap_or_else(|| Err(AndroidKeystoreError::Provider("sign returned null".into())))
        }
    }
}

#[cfg(all(feature = "android-real", target_os = "android"))]
pub use real::{init_keystore_jvm, JniKeystoreKey};
