//! Unit tests for Android hardware providers.
//!
//! Tests run on any target (using stub implementations when `android-real`
//! is not enabled) to verify the provider contract: descriptor correctness,
//! initialization, and error handling.

#[cfg(test)]
mod tests {
    use edgerun_android_hardware::*;
    use edgerun_capabilities::CapabilityProvider;

    // =========================================================================
    // Input provider tests
    // =========================================================================

    #[test]
    fn android_input_provider_descriptor() {
        let provider = AndroidInputProvider::new();
        let desc = provider.descriptor();
        assert_eq!(desc.provider_name, "android-input-stub");
        assert_eq!(
            desc.role,
            edgerun_capabilities::CapabilityRole::Input as i32
        );
    }

    #[test]
    fn android_input_provider_set_input_queue_no_panic() {
        let mut provider = AndroidInputProvider::new();
        // Null pointer should be handled gracefully
        provider.set_input_queue(std::ptr::null_mut());
    }

    // =========================================================================
    // Audio provider tests
    // =========================================================================

    #[test]
    fn android_audio_input_provider_descriptor() {
        let provider = AndroidAudioInputProvider::new();
        let desc = provider.descriptor();
        assert_eq!(desc.provider_name, "android-microphone-stub");
    }

    #[test]
    fn android_audio_output_provider_descriptor() {
        let provider = AndroidAudioOutputProvider::new();
        let desc = provider.descriptor();
        assert_eq!(desc.provider_name, "android-speaker-stub");
    }

    #[test]
    fn android_audio_input_start_capture_stub_returns_ok() {
        let mut provider = AndroidAudioInputProvider::new();
        // Stub implementation should return Ok
        assert!(provider.start_capture(16000, 1).is_ok());
    }

    #[test]
    fn android_audio_output_start_playback_stub_returns_ok() {
        let mut provider = AndroidAudioOutputProvider::new();
        assert!(provider.start_playback(44100, 2).is_ok());
    }

    // =========================================================================
    // Camera provider tests
    // =========================================================================

    #[test]
    fn android_camera_provider_descriptor() {
        let provider = AndroidCameraProvider::new();
        let desc = provider.descriptor();
        assert_eq!(desc.provider_name, "android-camera-stub");
    }

    #[test]
    fn android_camera_provider_init_stub_returns_ok() {
        let mut provider = AndroidCameraProvider::new();
        assert!(provider.init().is_ok());
    }

    // =========================================================================
    // Sensor provider tests
    // =========================================================================

    #[test]
    fn android_sensor_provider_descriptor() {
        let provider = AndroidSensorProvider::new();
        let desc = provider.descriptor();
        assert_eq!(desc.provider_name, "android-sensors-stub");
    }

    // =========================================================================
    // Display provider tests
    // =========================================================================

    #[test]
    fn android_display_provider_descriptor() {
        let provider = AndroidDisplayProvider::new();
        let desc = provider.descriptor();
        assert_eq!(desc.provider_name, "android-display-stub");
    }

    #[test]
    fn android_display_provider_set_surface_no_panic() {
        let mut provider = AndroidDisplayProvider::new();
        provider.set_surface(std::ptr::null_mut());
        assert!(provider.surface_info().is_none());
    }

    // =========================================================================
    // Biometric provider tests
    // =========================================================================

    #[test]
    fn android_biometric_provider_descriptor() {
        let provider = AndroidBiometricProvider::new();
        let desc = provider.descriptor();
        assert_eq!(desc.provider_name, "android-biometric-stub");
    }

    // =========================================================================
    // Location provider tests
    // =========================================================================

    #[test]
    fn android_location_provider_descriptor() {
        let provider = AndroidLocationProvider::new();
        let desc = provider.descriptor();
        assert_eq!(desc.provider_name, "android-location-stub");
    }

    // =========================================================================
    // Power provider tests
    // =========================================================================

    #[test]
    fn android_power_provider_descriptor() {
        let provider = AndroidPowerProvider::new();
        let desc = provider.descriptor();
        assert_eq!(desc.provider_name, "android-power-stub");
    }

    #[test]
    fn android_power_provider_get_battery_info_stub_returns_ok() {
        let provider = AndroidPowerProvider::new();
        assert!(provider.get_battery_info().is_ok());
    }

    #[test]
    fn android_power_provider_is_on_ac_power_stub_returns_false() {
        let provider = AndroidPowerProvider::new();
        assert!(!provider.is_on_ac_power());
    }

    // =========================================================================
    // Keystore tests (edgerun-android-keystore)
    // =========================================================================

    #[test]
    fn keystore_signature_algorithm_variants() {
        use edgerun_android_keystore::AndroidKeystoreSignatureAlgorithm;
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
    fn keystore_assurance_level_variants() {
        use edgerun_android_keystore::AndroidKeystoreAssuranceLevel;
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
    fn keystore_error_display() {
        use edgerun_android_keystore::{AndroidKeystoreError, AndroidKeystoreSignatureAlgorithm};

        let err = AndroidKeystoreError::Provider("test error".into());
        assert_eq!(format!("{}", err), "test error");

        let err = AndroidKeystoreError::UnsupportedAlgorithm(
            AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256,
        );
        let msg = format!("{}", err);
        assert!(msg.contains("unsupported"));
        assert!(msg.contains("EcdsaP256Sha256"));
    }

    #[test]
    fn keystore_sign_record_with_empty_hash() {
        use edgerun_android_keystore::*;

        struct FakeKey;
        impl AndroidKeystoreSigningKey for FakeKey {
            fn key_info(&self) -> Result<AndroidKeystoreKeyInfo, AndroidKeystoreError> {
                Ok(AndroidKeystoreKeyInfo {
                    alias: "test".into(),
                    algorithm: AndroidKeystoreSignatureAlgorithm::EcdsaP256Sha256,
                    public_key: vec![0x04, 0x03, 0x02, 0x01],
                    attestation_chain: vec![],
                    assurance_level: AndroidKeystoreAssuranceLevel::StrongBox,
                })
            }
            fn sign_message(&self, message: &[u8]) -> Result<Vec<u8>, AndroidKeystoreError> {
                Ok(message.to_vec())
            }
        }

        let key = FakeKey;
        let sig = sign_record_with_keystore(&key, "test:domain", &[]).unwrap();
        assert!(!sig.is_empty());
    }
}
