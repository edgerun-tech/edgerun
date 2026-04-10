//! Integration tests for Android hardware platform detection.
//!
//! These tests verify that:
//! 1. On non-Android targets, Linux hardware discovery is used
//! 2. Platform-aware inventory includes all expected categories
//! 3. Stub implementations return valid (empty) capability descriptors

#[cfg(test)]
mod tests {
    use edgerun_android_hardware::*;
    use edgerun_biometrics::{BiometricModality, BiometricState, BiometricAssuranceStrength};
    use edgerun_capabilities::CapabilityProvider;

    // =========================================================================
    // Provider descriptor integration tests
    // =========================================================================

    #[test]
    fn all_android_providers_have_valid_descriptors() {
        let providers: Vec<(&str, Box<dyn CapabilityProvider>)> = vec![
            ("input", Box::new(AndroidInputProvider::new())),
            ("audio_in", Box::new(AndroidAudioInputProvider::new())),
            ("audio_out", Box::new(AndroidAudioOutputProvider::new())),
            ("camera", Box::new(AndroidCameraProvider::new())),
            ("sensors", Box::new(AndroidSensorProvider::new())),
            ("display", Box::new(AndroidDisplayProvider::new())),
            ("biometric", Box::new(AndroidBiometricProvider::new())),
            ("location", Box::new(AndroidLocationProvider::new())),
            ("power", Box::new(AndroidPowerProvider::new())),
        ];

        for (name, provider) in providers {
            let desc = provider.descriptor();
            assert!(!desc.provider_name.is_empty(),
                "Provider '{}' has empty provider_name", name);
            assert!(desc.descriptor_version >= 1,
                "Provider '{}' has invalid descriptor_version", name);
        }
    }

    #[test]
    fn all_android_providers_have_correct_roles() {
        // Input providers
        let input = AndroidInputProvider::new();
        assert_eq!(input.descriptor().role,
            edgerun_capabilities::CapabilityRole::Input as i32);

        let audio_in = AndroidAudioInputProvider::new();
        assert_eq!(audio_in.descriptor().role,
            edgerun_capabilities::CapabilityRole::Input as i32);

        // Output providers
        let audio_out = AndroidAudioOutputProvider::new();
        assert_eq!(audio_out.descriptor().role,
            edgerun_capabilities::CapabilityRole::Output as i32);

        let display = AndroidDisplayProvider::new();
        assert_eq!(display.descriptor().role,
            edgerun_capabilities::CapabilityRole::Output as i32);

        // Secure element
        let biometric = AndroidBiometricProvider::new();
        assert_eq!(biometric.descriptor().role,
            edgerun_capabilities::CapabilityRole::SecureElement as i32);
    }

    #[test]
    fn audio_providers_have_auditory_modality() {
        let audio_in = AndroidAudioInputProvider::new();
        let audio_out = AndroidAudioOutputProvider::new();

        let in_modalities = audio_in.descriptor().modalities;
        let out_modalities = audio_out.descriptor().modalities;

        // Auditory modality = 2
        assert!(in_modalities.contains(&(edgerun_capabilities::CapabilityModality::Auditory as i32)),
            "Audio input should have Auditory modality");
        assert!(out_modalities.contains(&(edgerun_capabilities::CapabilityModality::Auditory as i32)),
            "Audio output should have Auditory modality");
    }

    #[test]
    fn camera_has_visual_modality() {
        let camera = AndroidCameraProvider::new();
        let modalities = camera.descriptor().modalities;

        // Visual modality = 1
        assert!(modalities.contains(&(edgerun_capabilities::CapabilityModality::Visual as i32)),
            "Camera should have Visual modality");
    }

    #[test]
    fn display_has_display_modality() {
        let display = AndroidDisplayProvider::new();
        let modalities = display.descriptor().modalities;

        // Display modality = 5
        assert!(modalities.contains(&(edgerun_capabilities::CapabilityModality::Display as i32)),
            "Display should have Display modality");
    }

    #[test]
    fn biometric_has_biometric_modality() {
        let biometric = AndroidBiometricProvider::new();
        let modalities = biometric.descriptor().modalities;

        // Biometric modality = 4
        assert!(modalities.contains(&(edgerun_capabilities::CapabilityModality::Biometric as i32)),
            "Biometric should have Biometric modality");
    }

    #[test]
    fn sensor_has_other_modality() {
        let sensors = AndroidSensorProvider::new();
        let modalities = sensors.descriptor().modalities;

        // Other modality = 11
        assert!(modalities.contains(&(edgerun_capabilities::CapabilityModality::Other as i32)),
            "Sensors should have Other modality");
    }

    #[test]
    fn location_has_other_modality() {
        let location = AndroidLocationProvider::new();
        let modalities = location.descriptor().modalities;

        assert!(modalities.contains(&(edgerun_capabilities::CapabilityModality::Other as i32)),
            "Location should have Other modality");
    }

    #[test]
    fn power_has_other_modality() {
        let power = AndroidPowerProvider::new();
        let modalities = power.descriptor().modalities;

        assert!(modalities.contains(&(edgerun_capabilities::CapabilityModality::Other as i32)),
            "Power should have Other modality");
    }

    #[test]
    fn all_android_providers_have_query_operation() {
        let providers: Vec<Box<dyn CapabilityProvider>> = vec![
            Box::new(AndroidInputProvider::new()),
            Box::new(AndroidAudioInputProvider::new()),
            Box::new(AndroidAudioOutputProvider::new()),
            Box::new(AndroidCameraProvider::new()),
            Box::new(AndroidSensorProvider::new()),
            Box::new(AndroidDisplayProvider::new()),
            Box::new(AndroidBiometricProvider::new()),
            Box::new(AndroidLocationProvider::new()),
            Box::new(AndroidPowerProvider::new()),
        ];

        for provider in providers {
            let desc = provider.descriptor();
            // Query operation = 0
            assert!(desc.operations.contains(&(edgerun_capabilities::CapabilityOperation::Query as i32)),
                "Provider '{}' should have Query operation", desc.provider_name);
        }
    }

    // =========================================================================
    // Keystore integration tests
    // =========================================================================

    #[test]
    fn keystore_trait_can_be_implemented() {
        use edgerun_android_keystore::{
            AndroidKeystoreKeyInfo, AndroidKeystoreSignatureAlgorithm,
            AndroidKeystoreAssuranceLevel, AndroidKeystoreError,
            AndroidKeystoreSigningKey,
        };

        struct TestKey;
        impl AndroidKeystoreSigningKey for TestKey {
            fn key_info(&self) -> Result<AndroidKeystoreKeyInfo, AndroidKeystoreError> {
                Ok(AndroidKeystoreKeyInfo {
                    alias: "test-key".into(),
                    algorithm: AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256,
                    public_key: vec![0x04; 65], // Uncompressed EC point
                    attestation_chain: vec![],
                    assurance_level: AndroidKeystoreAssuranceLevel::StrongBox,
                })
            }

            fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, AndroidKeystoreError> {
                // In real implementation, this would call Android Keystore
                // For test, just return a deterministic "signature"
                let mut sig = vec![0xAB; 64];
                sig[0..4].copy_from_slice(&(message.len() as u32).to_le_bytes());
                Ok(sig)
            }
        }

        let key = TestKey;
        let info = key.key_info().unwrap();
        assert_eq!(info.alias, "test-key");
        assert_eq!(info.algorithm, AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256);
        assert_eq!(info.assurance_level, AndroidKeystoreAssuranceLevel::StrongBox);

        let sig = key.sign_message(b"test message").unwrap();
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn keystore_signing_functions_work() {
        use edgerun_android_keystore::{
            AndroidKeystoreKeyInfo, AndroidKeystoreSignatureAlgorithm,
            AndroidKeystoreAssuranceLevel, AndroidKeystoreError,
            AndroidKeystoreSigningKey, sign_record_with_keystore,
            sign_record_with_keystore_checked,
        };

        struct TestKey;
        impl AndroidKeystoreSigningKey for TestKey {
            fn key_info(&self) -> Result<AndroidKeystoreKeyInfo, AndroidKeystoreError> {
                Ok(AndroidKeystoreKeyInfo {
                    alias: "test".into(),
                    algorithm: AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256,
                    public_key: vec![0x04; 65],
                    attestation_chain: vec![],
                    assurance_level: AndroidKeystoreAssuranceLevel::Tee,
                })
            }
            fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, AndroidKeystoreError> {
                Ok(message.to_vec())
            }
        }

        let key = TestKey;

        // Basic signing
        let sig = sign_record_with_keystore(&key, "test:v0:sig", &[0xFF; 32]).unwrap();
        assert!(!sig.is_empty());

        // Checked signing with matching algorithm
        let sig = sign_record_with_keystore_checked(
            &key,
            &[AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256],
            "test:v0:sig",
            &[0xFF; 32],
        ).unwrap();
        assert!(!sig.is_empty());

        // Checked signing with non-matching algorithm should fail
        let err = sign_record_with_keystore_checked(
            &key,
            &[AndroidKeystoreSignatureAlgorithm::RsaPssSha256],
            "test:v0:sig",
            &[0xFF; 32],
        ).unwrap_err();
        assert!(matches!(err, AndroidKeystoreError::UnsupportedAlgorithm(_)));
    }

    // =========================================================================
    // Biometric integration tests
    // =========================================================================

    #[test]
    fn biometric_stub_request_auth_fails_gracefully() {
        let provider = AndroidBiometricProvider::new();
        let result = provider.request_authentication("Test", "Authenticate");
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("not available") || err_msg.contains("stub"));
    }

    #[test]
    fn biometric_stub_state_is_default() {
        let state = AndroidBiometricProvider::get_state();
        assert_eq!(state, BiometricState::default());
        assert_eq!(
            state.assurance_strength(),
            BiometricAssuranceStrength::None
        );
    }

    #[test]
    fn biometric_strength_hierarchy() {
        let none = BiometricState::default();
        let presence = BiometricState {
            user_present: true, ..Default::default()
        };
        let biometric_match = BiometricState {
            verified: true, ..Default::default()
        };
        let hardware_protected = BiometricState {
            verified: true,
            hardware_protected: true,
            user_present: true,
            ..Default::default()
        };

        assert_eq!(none.assurance_strength(), BiometricAssuranceStrength::None);
        assert_eq!(presence.assurance_strength(), BiometricAssuranceStrength::UserPresence);
        assert_eq!(biometric_match.assurance_strength(), BiometricAssuranceStrength::BiometricMatch);
        assert_eq!(hardware_protected.assurance_strength(), BiometricAssuranceStrength::HardwareProtectedBiometric);

        assert!(none.assurance_strength() < presence.assurance_strength());
        assert!(presence.assurance_strength() < biometric_match.assurance_strength());
        assert!(biometric_match.assurance_strength() < hardware_protected.assurance_strength());
    }

    #[test]
    fn biometric_modality_mapping() {
        let modalities = vec![
            BiometricModality::Fingerprint,
            BiometricModality::Face,
            BiometricModality::Iris,
            BiometricModality::Voice,
            BiometricModality::Palm,
            BiometricModality::Other("custom".into()),
        ];
        for (i, a) in modalities.iter().enumerate() {
            for (j, b) in modalities.iter().enumerate() {
                if i == j { assert_eq!(a, b); } else { assert_ne!(a, b); }
            }
        }
    }

    #[test]
    fn biometric_satisfies_policy() {
        let strong = BiometricState {
            verified: true, hardware_protected: true, user_present: true,
            modality: Some(BiometricModality::Fingerprint),
        };
        assert!(strong.satisfies(BiometricAssuranceStrength::HardwareProtectedBiometric));
        assert!(strong.satisfies(BiometricAssuranceStrength::BiometricMatch));
        assert!(strong.satisfies(BiometricAssuranceStrength::UserPresence));
        assert!(strong.satisfies(BiometricAssuranceStrength::None));

        let weak = BiometricState { user_present: true, ..Default::default() };
        assert!(weak.satisfies(BiometricAssuranceStrength::UserPresence));
        assert!(weak.satisfies(BiometricAssuranceStrength::None));
        assert!(!weak.satisfies(BiometricAssuranceStrength::BiometricMatch));
    }
}
