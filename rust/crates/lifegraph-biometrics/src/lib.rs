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

#[derive(Clone, Debug, PartialEq, Eq)]
#[derive(Default)]
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
}
