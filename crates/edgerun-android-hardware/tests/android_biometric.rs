//! Unit tests for Android biometric provider.

#[cfg(test)]
mod tests {
    use edgerun_android_hardware::AndroidBiometricProvider;
    use edgerun_capabilities::CapabilityProvider;
    use edgerun_biometrics::{BiometricAssuranceStrength, BiometricModality, BiometricState};

    // =========================================================================
    // Stub implementation tests (non-Android targets)
    // =========================================================================

    #[test]
    fn biometric_provider_descriptor() {
        let provider = AndroidBiometricProvider::new();
        let desc = provider.descriptor();
        assert_eq!(desc.provider_name, "android-biometric-stub");
        assert_eq!(desc.role, edgerun_capabilities::CapabilityRole::SecureElement as i32);
    }

    #[test]
    fn biometric_provider_has_biometric_modality() {
        let provider = AndroidBiometricProvider::new();
        let desc = provider.descriptor();
        assert!(desc.modalities.contains(
            &(edgerun_capabilities::CapabilityModality::Biometric as i32)
        ));
    }

    #[test]
    fn biometric_provider_has_query_operation() {
        let provider = AndroidBiometricProvider::new();
        let desc = provider.descriptor();
        assert!(desc.operations.contains(
            &(edgerun_capabilities::CapabilityOperation::Query as i32)
        ));
    }

    #[test]
    fn biometric_provider_init_stub_returns_ok() {
        let mut provider = AndroidBiometricProvider::new();
        assert!(provider.init().is_ok());
    }

    #[test]
    fn biometric_is_available_stub_checks_env() {
        // On non-Android, this checks for ANDROID_ROOT env var or sysfs paths
        let available = AndroidBiometricProvider::is_available();
        // On a Linux host without Android env vars, this should be false
        #[cfg(not(target_os = "android"))]
        if std::env::var("ANDROID_ROOT").is_err() {
            assert!(!available);
        }
    }

    #[test]
    fn biometric_get_state_stub_returns_default() {
        let state = AndroidBiometricProvider::get_state();
        assert_eq!(state, BiometricState::default());
        assert!(!state.verified);
        assert!(!state.hardware_protected);
        assert!(!state.user_present);
        assert_eq!(state.modality, None);
    }

    #[test]
    fn biometric_get_assurance_strength_stub_returns_none() {
        let strength = AndroidBiometricProvider::get_assurance_strength();
        assert_eq!(strength, BiometricAssuranceStrength::None);
    }

    #[test]
    fn biometric_request_authentication_stub_fails() {
        let provider = AndroidBiometricProvider::new();
        let result = provider.request_authentication("Test", "Please authenticate");
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_str = format!("{}", err);
        assert!(err_str.contains("not available") || err_str.contains("stub"));
    }

    // =========================================================================
    // BiometricState tests (from edgerun-biometrics)
    // =========================================================================

    #[test]
    fn biometric_state_fingerprint_verified_is_strong() {
        let state = BiometricState {
            modality: Some(BiometricModality::Fingerprint),
            verified: true,
            hardware_protected: true,
            user_present: true,
        };
        assert_eq!(
            state.assurance_strength(),
            BiometricAssuranceStrength::HardwareProtectedBiometric
        );
    }

    #[test]
    fn biometric_state_face_verified_not_hardware() {
        let state = BiometricState {
            modality: Some(BiometricModality::Face),
            verified: true,
            hardware_protected: false,
            user_present: true,
        };
        assert_eq!(
            state.assurance_strength(),
            BiometricAssuranceStrength::BiometricMatch
        );
    }

    #[test]
    fn biometric_state_user_present_not_verified() {
        let state = BiometricState {
            modality: Some(BiometricModality::Fingerprint),
            verified: false,
            hardware_protected: false,
            user_present: true,
        };
        assert_eq!(
            state.assurance_strength(),
            BiometricAssuranceStrength::UserPresence
        );
    }

    #[test]
    fn biometric_strength_ordering() {
        assert!(BiometricAssuranceStrength::None < BiometricAssuranceStrength::UserPresence);
        assert!(BiometricAssuranceStrength::UserPresence < BiometricAssuranceStrength::BiometricMatch);
        assert!(BiometricAssuranceStrength::BiometricMatch < BiometricAssuranceStrength::HardwareProtectedBiometric);
    }

    #[test]
    fn biometric_state_satisfies_minimum() {
        let state = BiometricState {
            modality: Some(BiometricModality::Fingerprint),
            verified: true,
            hardware_protected: true,
            user_present: true,
        };
        // Strongest state satisfies all levels
        assert!(state.satisfies(BiometricAssuranceStrength::HardwareProtectedBiometric));
        assert!(state.satisfies(BiometricAssuranceStrength::BiometricMatch));
        assert!(state.satisfies(BiometricAssuranceStrength::UserPresence));
        assert!(state.satisfies(BiometricAssuranceStrength::None));
    }

    #[test]
    fn biometric_state_satisfies_partial() {
        let state = BiometricState {
            modality: Some(BiometricModality::Fingerprint),
            verified: false,
            hardware_protected: false,
            user_present: true,
        };
        // User presence only
        assert!(state.satisfies(BiometricAssuranceStrength::UserPresence));
        assert!(state.satisfies(BiometricAssuranceStrength::None));
        assert!(!state.satisfies(BiometricAssuranceStrength::BiometricMatch));
        assert!(!state.satisfies(BiometricAssuranceStrength::HardwareProtectedBiometric));
    }

    #[test]
    fn biometric_modality_variants() {
        assert_eq!(BiometricModality::Fingerprint, BiometricModality::Fingerprint);
        assert_ne!(BiometricModality::Fingerprint, BiometricModality::Face);
        assert_ne!(BiometricModality::Face, BiometricModality::Iris);
        assert_ne!(BiometricModality::Iris, BiometricModality::Voice);
        assert_ne!(BiometricModality::Voice, BiometricModality::Palm);
    }
}
