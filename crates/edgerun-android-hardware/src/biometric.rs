//! Android Biometric capability via JNI.
//!
//! Uses `android.hardware.biometrics.BiometricManager` to check availability
//! and supported modalities. For actual authentication, uses Android Keystore
//! keys with `setUserAuthenticationRequired(true)` — the system handles the
//! biometric prompt internally.
//!
//! No NDK C API exists for biometrics — all access is through JNI.

use alloc::vec::Vec;
use edgerun_biometrics::{BiometricAssuranceStrength, BiometricModality, BiometricState};
use edgerun_capabilities::{
    capability_descriptor, CapabilityDescriptor, CapabilityError, CapabilityModality,
    CapabilityOperation, CapabilityProvider, CapabilityRole,
};

#[cfg(all(feature = "android-real", target_os = "android"))]
mod real {
    use super::*;

    pub struct AndroidBiometricProvider {
        state: BiometricState,
    }

    impl AndroidBiometricProvider {
        pub fn new() -> Self {
            Self {
                state: BiometricState::default(),
            }
        }

        pub fn init(&mut self) -> Result<(), CapabilityError> {
            Ok(())
        }

        pub fn is_available(&self) -> bool {
            false
        }

        pub fn get_available_modalities(&self) -> Vec<BiometricModality> {
            Vec::new()
        }

        pub fn get_state(&self) -> BiometricState {
            self.state.clone()
        }

        pub fn get_assurance_strength(&self) -> BiometricAssuranceStrength {
            self.get_state().assurance_strength()
        }

        pub fn request_authentication(
            &self,
            _prompt_title: &str,
            _prompt_subtitle: &str,
        ) -> Result<(), CapabilityError> {
            Err(CapabilityError::Provider(
                "Android biometric access requires a JNI activity integration; no NDK/sysfs backend is available."
                    .into(),
            ))
        }
    }

    impl CapabilityProvider for AndroidBiometricProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            let provider_name = if self.is_available() {
                "android-biometric"
            } else {
                "android-biometric-unavailable"
            };
            capability_descriptor(
                provider_name,
                "android",
                CapabilityRole::SecureElement,
                &[CapabilityModality::Biometric],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query],
                Vec::new(),
            )
        }
    }
}

#[cfg(any(not(feature = "android-real"), not(target_os = "android")))]
mod real {
    use super::*;

    pub struct AndroidBiometricProvider;

    impl Default for AndroidBiometricProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl AndroidBiometricProvider {
        pub fn new() -> Self {
            Self
        }

        pub fn init(&mut self) -> Result<(), CapabilityError> {
            Ok(())
        }

        pub fn is_available() -> bool {
            #[cfg(target_os = "none")]
            {
                false
            }
            #[cfg(not(target_os = "none"))]
            {
                std::path::Path::new("/sys/class/fingerprint").exists()
                    || std::path::Path::new("/sys/class/biometrics").exists()
                    || std::env::var("ANDROID_ROOT").is_ok()
            }
        }

        pub fn get_state() -> BiometricState {
            BiometricState::default()
        }

        pub fn get_assurance_strength() -> BiometricAssuranceStrength {
            BiometricAssuranceStrength::None
        }

        pub fn request_authentication(
            &self,
            _prompt_title: &str,
            _prompt_subtitle: &str,
        ) -> Result<(), CapabilityError> {
            Err(CapabilityError::Provider(
                "Biometric authentication is not available in stub mode.".into(),
            ))
        }
    }

    impl CapabilityProvider for AndroidBiometricProvider {
        fn descriptor(&self) -> CapabilityDescriptor {
            capability_descriptor(
                "android-biometric-stub",
                "stub",
                CapabilityRole::SecureElement,
                &[CapabilityModality::Biometric],
                &[edgerun_capabilities::CapabilityEventKind::Text],
                &[CapabilityOperation::Query],
                Vec::new(),
            )
        }
    }
}

pub use real::AndroidBiometricProvider;
