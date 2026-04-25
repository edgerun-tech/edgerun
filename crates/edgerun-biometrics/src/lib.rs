#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BiometricAssuranceStrength {
    None,
    UserPresence,
    BiometricMatch,
    HardwareProtectedBiometric,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BiometricModality {
    Fingerprint,
    Face,
    Iris,
    Voice,
    Palm,
    Other(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UserVerificationPolicy {
    None,
    OptionalPresence,
    RequiredPresence,
    PreferredBiometric,
    RequiredBiometric,
}

#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct BiometricState {
    pub modality: Option<BiometricModality>,
    pub verified: bool,
    pub hardware_protected: bool,
    pub user_present: bool,
}

impl BiometricState {
    pub fn assurance_strength(&self) -> BiometricAssuranceStrength {
        match (self.user_present, self.verified, self.hardware_protected) {
            (_, true, true) => BiometricAssuranceStrength::HardwareProtectedBiometric,
            (_, true, false) => BiometricAssuranceStrength::BiometricMatch,
            (true, false, _) => BiometricAssuranceStrength::UserPresence,
            _ => BiometricAssuranceStrength::None,
        }
    }

    pub fn satisfies(&self, minimum: BiometricAssuranceStrength) -> bool {
        self.assurance_strength() >= minimum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_protected_biometric_is_strongest() {
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
    fn presence_only_is_weaker_than_biometric() {
        let state = BiometricState {
            user_present: true,
            ..Default::default()
        };
        assert!(state.satisfies(BiometricAssuranceStrength::UserPresence));
        assert!(!state.satisfies(BiometricAssuranceStrength::BiometricMatch));
    }

    #[test]
    fn assurance_strength_none_when_not_verified_or_present() {
        let state = BiometricState {
            modality: None,
            verified: false,
            hardware_protected: false,
            user_present: false,
        };
        assert_eq!(state.assurance_strength(), BiometricAssuranceStrength::None);
    }

    #[test]
    fn assurance_strength_biometric_match_when_verified_not_hardware() {
        let state = BiometricState {
            modality: Some(BiometricModality::Face),
            verified: true,
            hardware_protected: false,
            user_present: false,
        };
        assert_eq!(
            state.assurance_strength(),
            BiometricAssuranceStrength::BiometricMatch
        );
    }

    #[test]
    fn assurance_strength_user_presence_when_not_verified_but_present() {
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
    fn satisfies_is_transitive() {
        let state = BiometricState {
            verified: true,
            hardware_protected: true,
            user_present: true,
            ..Default::default()
        };
        assert!(state.satisfies(BiometricAssuranceStrength::HardwareProtectedBiometric));
        assert!(state.satisfies(BiometricAssuranceStrength::BiometricMatch));
        assert!(state.satisfies(BiometricAssuranceStrength::UserPresence));
        assert!(state.satisfies(BiometricAssuranceStrength::None));
    }

    #[test]
    fn satisfies_none_is_always_true() {
        let state = BiometricState::default();
        assert!(state.satisfies(BiometricAssuranceStrength::None));
    }

    #[test]
    fn assurance_strength_ordering() {
        assert!(BiometricAssuranceStrength::None < BiometricAssuranceStrength::UserPresence);
        assert!(
            BiometricAssuranceStrength::UserPresence < BiometricAssuranceStrength::BiometricMatch
        );
        assert!(
            BiometricAssuranceStrength::BiometricMatch
                < BiometricAssuranceStrength::HardwareProtectedBiometric
        );
    }

    #[test]
    fn modality_variants() {
        assert_eq!(
            BiometricModality::Fingerprint,
            BiometricModality::Fingerprint
        );
        assert_ne!(BiometricModality::Fingerprint, BiometricModality::Face);
        assert_ne!(BiometricModality::Face, BiometricModality::Iris);
        assert_ne!(BiometricModality::Voice, BiometricModality::Palm);
        assert_eq!(
            BiometricModality::Other("custom".into()),
            BiometricModality::Other("custom".into())
        );
        assert_ne!(
            BiometricModality::Other("a".into()),
            BiometricModality::Other("b".into())
        );
    }

    #[test]
    fn user_verification_policy_variants() {
        assert_ne!(
            UserVerificationPolicy::None,
            UserVerificationPolicy::OptionalPresence
        );
        assert_ne!(
            UserVerificationPolicy::OptionalPresence,
            UserVerificationPolicy::RequiredPresence
        );
        assert_ne!(
            UserVerificationPolicy::RequiredPresence,
            UserVerificationPolicy::PreferredBiometric
        );
        assert_ne!(
            UserVerificationPolicy::PreferredBiometric,
            UserVerificationPolicy::RequiredBiometric
        );
    }

    #[test]
    fn biometric_state_clone_and_debug() {
        let state = BiometricState {
            modality: Some(BiometricModality::Fingerprint),
            verified: true,
            hardware_protected: false,
            user_present: true,
        };
        let cloned = state.clone();
        assert_eq!(state.modality, cloned.modality);
        assert_eq!(state.verified, cloned.verified);
        assert_eq!(state.hardware_protected, cloned.hardware_protected);
        assert_eq!(state.user_present, cloned.user_present);
        // Debug impl
        let debug_str = format!("{state:?}");
        assert!(debug_str.contains("BiometricState"));
    }

    #[test]
    fn biometric_state_default() {
        let state = BiometricState::default();
        assert!(!state.verified);
        assert!(!state.hardware_protected);
        assert!(!state.user_present);
        assert_eq!(state.modality, None);
    }

    #[test]
    fn hardware_protected_overrides_user_present_for_strength() {
        // Even without user_present, verified + hardware_protected = HardwareProtectedBiometric
        let state = BiometricState {
            verified: true,
            hardware_protected: true,
            user_present: false,
            ..Default::default()
        };
        assert_eq!(
            state.assurance_strength(),
            BiometricAssuranceStrength::HardwareProtectedBiometric
        );
    }
}
